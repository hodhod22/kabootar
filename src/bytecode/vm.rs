//! Stack bytecode VM (v2.18).

use super::classes::{
    instantiate_class, register_module_classes, register_module_enums, register_module_interfaces,
};
use super::types::{BytecodeClassDef, BytecodeFnDef, BytecodeModule, Constant, GeneratorTryRegion, Opcode};
use crate::lang_preprocess::MemoryMode;
use crate::ops::{eval_binary_op, get_length, read_index, read_member, write_index, write_member};
use crate::runtime::ownership;
use crate::value::{AsyncBody, BytecodeFunction, Environment, Microtask, PromiseValue, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_BYTECODE_STACK: usize = 8192;

/// P10: monomorphic GetMember IC — object identity + shape hash + cached slot value.
struct MemberIc {
    ptr: u64,
    shape: u64,
    /// Interned member name — **not** `GetMember` const-pool index (`key_idx` is
    /// per-function; `sess["pCur"]` and `sess["pLeft"]` can both be slot 0).
    key: u32,
    slot: u16,
    value: Value,
}

impl Default for MemberIc {
    fn default() -> Self {
        Self {
            ptr: 0,
            shape: 0,
            key: u32::MAX,
            slot: 0,
            value: Value::Undefined,
        }
    }
}

thread_local! {
    static MEMBER_IC: RefCell<MemberIc> = RefCell::new(MemberIc::default());
}

/// P1: monomorphic LoadGlobal inline cache (global idx + env frame).
struct GlobalIc {
    idx: Option<u16>,
    env_id: usize,
    value: Value,
}

impl Default for GlobalIc {
    fn default() -> Self {
        Self {
            idx: None,
            env_id: 0,
            value: Value::Undefined,
        }
    }
}

thread_local! {
    static GLOBAL_IC: RefCell<GlobalIc> = RefCell::new(GlobalIc::default());
}

static MEMBER_IC_HITS: AtomicU64 = AtomicU64::new(0);
static MEMBER_IC_MISSES: AtomicU64 = AtomicU64::new(0);
static GLOBAL_IC_HITS: AtomicU64 = AtomicU64::new(0);
static GLOBAL_IC_MISSES: AtomicU64 = AtomicU64::new(0);
static CALL_IC_HITS: AtomicU64 = AtomicU64::new(0);
static CALL_IC_MISSES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static CALL_ARGS_BUF: RefCell<Vec<Value>> = RefCell::new(Vec::new());
}

struct CallIc {
    native: Option<fn(&[Value], &mut Environment) -> Result<Value, String>>,
    bc_ptr: usize,
    method: Option<fn(&[Value], &mut Environment) -> Result<Value, String>>,
}

thread_local! {
    static CALL_IC: RefCell<CallIc> = RefCell::new(CallIc {
        native: None,
        bc_ptr: 0,
        method: None,
    });
}

struct Intern {
    ids: HashMap<String, u32>,
}

thread_local! {
    static KEY_INTERN: RefCell<Intern> = RefCell::new(Intern {
        ids: HashMap::new(),
    });
}

struct ObjSlots {
    shape: u64,
    ids: Vec<u32>,
    slots: Vec<Value>,
}

thread_local! {
    static OBJ_SLOTS: RefCell<HashMap<u64, ObjSlots>> = RefCell::new(HashMap::new());
}

fn env_frame_id(env: &Environment) -> usize {
    env.frame_id()
}

fn global_ic_invalidate(idx: u16) {
    GLOBAL_IC.with(|ic| {
        let mut ic = ic.borrow_mut();
        if ic.idx == Some(idx) {
            ic.idx = None;
        }
    });
}

fn global_ic_load(idx: u16, env: &Environment, name: &str) -> Result<Value, String> {
    let frame = env_frame_id(env);
    if let Some(v) = GLOBAL_IC.with(|ic| {
        let ic = ic.borrow();
        if ic.idx == Some(idx) && ic.env_id == frame {
            GLOBAL_IC_HITS.fetch_add(1, Ordering::Relaxed);
            Some(ic.value.clone())
        } else {
            None
        }
    }) {
        return Ok(v);
    }
    GLOBAL_IC_MISSES.fetch_add(1, Ordering::Relaxed);
    let v = env
        .get(name)
        .ok_or_else(|| crate::evaluator::undefined_var_message(name, env))?;
    GLOBAL_IC.with(|ic| {
        let mut ic = ic.borrow_mut();
        ic.idx = Some(idx);
        ic.env_id = frame;
        ic.value = v.clone();
    });
    Ok(v)
}

/// Diagnostic counters for GetMember IC (tests / `gc_frame_stats`-style probes).
pub fn member_ic_stats() -> (u64, u64) {
    (
        MEMBER_IC_HITS.load(Ordering::Relaxed),
        MEMBER_IC_MISSES.load(Ordering::Relaxed),
    )
}

pub fn member_ic_reset_for_tests() {
    MEMBER_IC_HITS.store(0, Ordering::Relaxed);
    MEMBER_IC_MISSES.store(0, Ordering::Relaxed);
    MEMBER_IC.with(|ic| *ic.borrow_mut() = MemberIc::default());
    OBJ_SLOTS.with(|t| t.borrow_mut().clear());
}

/// Diagnostic counters for LoadGlobal IC (P1).
pub fn global_ic_stats() -> (u64, u64) {
    (
        GLOBAL_IC_HITS.load(Ordering::Relaxed),
        GLOBAL_IC_MISSES.load(Ordering::Relaxed),
    )
}

pub fn global_ic_reset_for_tests() {
    GLOBAL_IC_HITS.store(0, Ordering::Relaxed);
    GLOBAL_IC_MISSES.store(0, Ordering::Relaxed);
    GLOBAL_IC.with(|ic| *ic.borrow_mut() = GlobalIc::default());
}

/// Diagnostic counters for Call native IC (P1).
pub fn call_ic_stats() -> (u64, u64) {
    (
        CALL_IC_HITS.load(Ordering::Relaxed),
        CALL_IC_MISSES.load(Ordering::Relaxed),
    )
}

pub fn call_ic_reset_for_tests() {
    CALL_IC_HITS.store(0, Ordering::Relaxed);
    CALL_IC_MISSES.store(0, Ordering::Relaxed);
    CALL_IC.with(|ic| {
        let mut ic = ic.borrow_mut();
        ic.native = None;
        ic.bc_ptr = 0;
        ic.method = None;
    });
}

fn take_call_args(stack: &mut Vec<Value>, n: usize) -> Result<Vec<Value>, String> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut args = CALL_ARGS_BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    args.clear();
    args.reserve(n);
    match n {
        1 => args.push(stack.pop().ok_or("Bytecode stack underflow")?),
        2 => {
            let b = stack.pop().ok_or("Bytecode stack underflow")?;
            let a = stack.pop().ok_or("Bytecode stack underflow")?;
            args.push(a);
            args.push(b);
        }
        3 => {
            let c = stack.pop().ok_or("Bytecode stack underflow")?;
            let b = stack.pop().ok_or("Bytecode stack underflow")?;
            let a = stack.pop().ok_or("Bytecode stack underflow")?;
            args.push(a);
            args.push(b);
            args.push(c);
        }
        _ => {
            for _ in 0..n {
                args.push(stack.pop().ok_or("Bytecode stack underflow")?);
            }
            args.reverse();
        }
    }
    Ok(args)
}

fn recycle_call_args(mut args: Vec<Value>) {
    args.clear();
    CALL_ARGS_BUF.with(|b| *b.borrow_mut() = args);
}

fn invalidate_member_ic() {
    MEMBER_IC.with(|ic| *ic.borrow_mut() = MemberIc::default());
    OBJ_SLOTS.with(|t| t.borrow_mut().clear());
}

fn object_shape_hash(map: &HashMap<String, Value>) -> u64 {
    let mut acc = 0u64;
    let mut n = 0u64;
    for k in map.keys() {
        if k.starts_with("__kab_") {
            continue;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        k.hash(&mut h);
        acc ^= h.finish();
        n += 1;
    }
    acc ^ n.wrapping_mul(0x9E3779B97F4A7C15)
}

fn object_ic_ptr(map: &Rc<HashMap<String, Value>>) -> u64 {
    Rc::as_ptr(map) as u64
}

fn member_is_callable(v: &Value) -> bool {
    matches!(
        v,
        Value::NativeFunction(_)
            | Value::BytecodeFn(_)
            | Value::Function { .. }
            | Value::BoundNative(_, _)
    )
}

fn intern_key(s: &str) -> u32 {
    KEY_INTERN.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(id) = t.ids.get(s) {
            return *id;
        }
        let id = t.ids.len() as u32;
        t.ids.insert(s.to_string(), id);
        id
    })
}

fn ensure_obj_slots(ptr: u64, shape: u64, map: &HashMap<String, Value>) {
    OBJ_SLOTS.with(|t| {
        let mut t = t.borrow_mut();
        let rebuild = t.get(&ptr).map(|o| o.shape != shape).unwrap_or(true);
        if rebuild {
            let mut pairs: Vec<(u32, Value)> = map
                .iter()
                .filter(|(k, _)| !k.starts_with("__kab_"))
                .map(|(k, v)| (intern_key(k), v.clone()))
                .collect();
            pairs.sort_by_key(|(id, _)| *id);
            let ids: Vec<u32> = pairs.iter().map(|(id, _)| *id).collect();
            let slots: Vec<Value> = pairs.into_iter().map(|(_, v)| v).collect();
            t.insert(ptr, ObjSlots { shape, ids, slots });
        }
    });
}

fn slot_load(ptr: u64, intern: u32) -> Option<Value> {
    OBJ_SLOTS.with(|t| {
        let t = t.borrow();
        let o = t.get(&ptr)?;
        let i = o.ids.iter().position(|id| *id == intern)?;
        o.slots.get(i).cloned()
    })
}

fn slot_index(ptr: u64, intern: u32) -> Option<u16> {
    OBJ_SLOTS.with(|t| {
        let t = t.borrow();
        let o = t.get(&ptr)?;
        o.ids.iter().position(|id| *id == intern).map(|i| i as u16)
    })
}

fn slot_load_i(ptr: u64, slot: u16) -> Option<Value> {
    OBJ_SLOTS.with(|t| {
        t.borrow()
            .get(&ptr)
            .and_then(|o| o.slots.get(slot as usize).cloned())
    })
}

fn chunk_is_manual(module: Option<&BytecodeModule>, env: &Environment) -> bool {
    if let Some(m) = module {
        return m.memory_mode == MemoryMode::Manual;
    }
    ownership::is_manual(env)
}

fn load_local_value(
    local_vals: &mut [Value],
    locals: &[String],
    i: usize,
    args: &Option<(&BytecodeFnDef, Vec<Value>)>,
    env: &Environment,
    _manual: bool,
) -> Result<Value, String> {
    // Owned buffers: peek (shared handle). Move/invalidate via `drop` / overwrite / scope end.
    // (Auto-move on every LoadLocal breaks `owned_write(b, …); owned_read(b, …)`.)
    if args.is_some() {
        return Ok(local_vals.get(i).cloned().unwrap_or(Value::Undefined));
    }
    // P10d: after StoreLocal/AccAddLocal the slot is live — skip env HashMap.
    if let Some(v) = local_vals.get(i) {
        if !matches!(v, Value::Undefined) {
            return Ok(v.clone());
        }
    }
    let v = if let Some(name) = locals.get(i) {
        if name.starts_with("__kab_") {
            local_vals.get(i).cloned().unwrap_or(Value::Undefined)
        } else {
            env.get(name).unwrap_or_else(|| {
                local_vals.get(i).cloned().unwrap_or(Value::Undefined)
            })
        }
    } else {
        local_vals.get(i).cloned().unwrap_or(Value::Undefined)
    };
    Ok(v)
}

fn drop_owned_locals(local_vals: &[Value], env: &mut Environment) -> Result<(), String> {
    for v in local_vals {
        ownership::drop_owned_value(v, env)?;
    }
    Ok(())
}

pub struct ChunkCursor {
    pub ip: usize,
    pub stack: Vec<Value>,
    pub delegate: Option<Value>,
    pub generator_async: bool,
}

enum ChunkExit {
    Done(Value),
    Yield(Value),
}

/// Resume kind for a suspended generator (`.next(value)` / `.throw(reason)` / `.return(value)`).
#[derive(Debug, Clone)]
pub enum GeneratorResume {
    Next(Value),
    Throw(Value),
    Return(Value),
}

pub fn find_try_region_for_ip(func: &BytecodeFnDef, ip: usize) -> Option<&GeneratorTryRegion> {
    func.try_regions
        .iter()
        .filter(|r| ip >= r.body_start && ip <= r.body_end)
        .max_by_key(|r| r.body_start)
        .or_else(|| {
            if ip > 0 {
                func.try_regions
                    .iter()
                    .filter(|r| {
                        let site = ip - 1;
                        site >= r.body_start && site <= r.body_end
                    })
                    .max_by_key(|r| r.body_start)
            } else {
                None
            }
        })
}

/// If `err` is a `throw` marker and the current IP is inside a try region, bind the
/// value and jump to the catch handler. Nested calls previously leaked throws past
/// the caller's `try` because only same-frame `Opcode::Throw` consulted regions.
fn try_catch_propagated_throw(
    err: &str,
    current_fn: Option<&BytecodeFnDef>,
    module: Option<&BytecodeModule>,
    locals: &[String],
    immutable_locals: &[bool],
    local_captures: &[bool],
    local_vals: &mut Vec<Value>,
    env: &mut Environment,
    ip: &mut usize,
    stack: &mut Vec<Value>,
) -> Result<bool, String> {
    let Some(v) = crate::runtime::stdlib::error::take_throw_value(err) else {
        return Ok(false);
    };
    let ip_now = *ip;
    let region = current_fn
        .and_then(|func| find_try_region_for_ip(func, ip_now))
        .or_else(|| {
            module.and_then(|m| {
                m.main_try_regions
                    .iter()
                    .filter(|r| ip_now >= r.body_start && ip_now <= r.body_end)
                    .max_by_key(|r| r.body_start)
            })
        });
    let Some(region) = region else {
        return Err(crate::runtime::stdlib::error::throw_value(v));
    };
    let li = region.err_local as usize;
    if li >= local_vals.len() {
        local_vals.resize(li + 1, Value::Undefined);
    }
    let caught = crate::runtime::stdlib::error::enrich_error_value_for_catch(v);
    local_vals[li] = caught.clone();
    store_local_to_env(locals, immutable_locals, local_captures, li, &caught, env)?;
    push_stack(stack, caught)?;
    *ip = region.catch_start;
    Ok(true)
}

#[inline]
fn push_stack(stack: &mut Vec<Value>, v: Value) -> Result<(), String> {
    if stack.len() >= MAX_BYTECODE_STACK {
        return Err("Bytecode stack overflow (memory safety limit)".into());
    }
    stack.push(v);
    Ok(())
}

fn const_to_value(c: &Constant) -> Value {
    match c {
        Constant::Number(n) => Value::Number(*n),
        Constant::BigInt(s) => Value::BigInt(
            crate::runtime::stdlib::bigint::parse_decimal(s)
                .unwrap_or_else(|_| num_bigint::BigInt::from(0)),
        ),
        Constant::Float(f) => Value::Float(*f),
        Constant::String(s) => Value::String(s.clone()),
        Constant::Bool(b) => Value::Bool(*b),
        Constant::Null => Value::Null,
        Constant::Undefined => Value::Undefined,
        Constant::Nan => Value::Float(f64::NAN),
    }
}

fn module_main_is_trivial(code: &[Opcode]) -> bool {
    match code {
        [] | [Opcode::Halt] | [Opcode::Return] => true,
        [Opcode::Const(_), Opcode::Halt] | [Opcode::Const(_), Opcode::Return] => true,
        _ => false,
    }
}

pub fn run_module(module: &BytecodeModule, env: &mut Environment) -> Result<Value, String> {
    crate::runtime::ownership::set_memory_mode(env, module.memory_mode);
    for name in &module.imports {
        let imported = crate::modules::import_module_exported(name, env)?;
        if module.pub_imports.iter().any(|m| m == name) {
            for export_name in imported {
                env.mark_exported(export_name);
            }
        }
    }
    register_module_interfaces(module, env);
    register_module_enums(module, env);
    register_module_classes(module, env)?;
    register_functions(module, env)?;
    let result = if module_main_is_trivial(&module.main_code) {
        Value::Null
    } else {
        let mut cursor = ChunkCursor {
            ip: 0,
            stack: Vec::new(),
            delegate: None,
            generator_async: false,
        };
        let (exit, local_vals) = run_chunk(
            &module.main_code,
            &module.constants,
            &module.globals,
            &module.main_locals,
            &module.main_immutable_locals,
            &module.arrow_functions,
            &module.classes,
            None,
            Some(module),
            None,
            &mut cursor,
            false,
            env,
        )?;
        let result = match exit {
            ChunkExit::Done(v) => v,
            ChunkExit::Yield(_) => return Err("yield outside generator".into()),
        };
        sync_main_locals(module, &local_vals, env)?;
        result
    };
    refresh_function_closures(module, env);
    for name in &module.exports {
        env.mark_exported(name);
    }
    Ok(result)
}

fn sync_main_locals(
    module: &BytecodeModule,
    local_vals: &[Value],
    env: &mut Environment,
) -> Result<(), String> {
    for (i, name) in module.main_locals.iter().enumerate() {
        let v = env
            .get(name)
            .or_else(|| local_vals.get(i).cloned())
            .unwrap_or(Value::Undefined);
        if matches!(v, Value::Undefined) {
            continue;
        }
        if module.main_immutable_locals.get(i) == Some(&true) {
            env.set_const(name.clone(), v.clone());
        } else {
            env.set(name.clone(), v.clone());
        }
    }
    Ok(())
}

fn register_functions(module: &BytecodeModule, env: &mut Environment) -> Result<(), String> {
    for func in &module.functions {
        let mut f = func.clone();
        if f.globals.is_empty() {
            f.globals = module.globals.clone();
        }
        if f.constants.is_empty() {
            f.constants = module.constants.clone();
        }
        env.set(
            f.name.clone(),
            Value::BytecodeFn(BytecodeFunction {
                def: Rc::new(f),
                // Share the module frame — `env.clone()` per fn recurses through prior
                // BytecodeFn closures and OOMs (~7–14 top-level fns). Mutual visibility
                // is rebuilt in `refresh_function_closures` after main runs.
                closure: env.share_bindings(),
            }),
        );
    }
    Ok(())
}

/// Copy live local slots into *this* activation frame so `MakeArrowFn` can capture them.
/// Always `set` on the current frame — never `assign` into a parent/module env (that breaks
/// reentrant calls that share the same local names).
fn sync_locals_into_env(locals: &[String], local_vals: &[Value], env: &mut Environment) {
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        let Some(v) = local_vals.get(i) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        env.set(name.clone(), v.clone());
    }
}

/// After nested calls mutate captured bindings via `env`, refresh `local_vals`.
fn pull_env_into_local_vals(locals: &[String], local_vals: &mut Vec<Value>, env: &Environment) {
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        let Some(v) = env.get(name) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if i >= local_vals.len() {
            local_vals.resize(i + 1, Value::Undefined);
        }
        local_vals[i] = v.clone();
    }
}

/// Like `pull_env_into_local_vals`, but only for captured enclosing slots.
/// Ordinary fn-locals stay authoritative in `local_vals` (needed for in-place
/// ops like `ArrayPushLocal` that must not be wiped by a stale env copy).
fn pull_captured_locals_from_env(
    locals: &[String],
    local_captures: &[bool],
    local_vals: &mut Vec<Value>,
    env: &Environment,
) {
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        if local_captures.get(i).copied() != Some(true) {
            continue;
        }
        let Some(v) = env.get(name) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if i >= local_vals.len() {
            local_vals.resize(i + 1, Value::Undefined);
        }
        local_vals[i] = v.clone();
    }
}

/// Refresh caller object locals after `Call` writeback (by `__oid`), without pulling
/// scalar locals that nested frames may have clobbered in the shared env.
fn pull_object_locals_from_env(
    locals: &[String],
    local_vals: &mut Vec<Value>,
    env: &Environment,
) {
    use crate::runtime::stdlib::object::object_oid_of;
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        let Some(oid) = local_vals.get(i).and_then(|v| match v {
            Value::Object(map) => object_oid_of(map),
            _ => None,
        }) else {
            continue;
        };
        let Some(live) = env.get(name) else {
            continue;
        };
        let Value::Object(ref live_map) = live else {
            continue;
        };
        if object_oid_of(live_map) != Some(oid) {
            continue;
        }
        local_vals[i] = live;
    }
}

fn store_local_to_env(
    locals: &[String],
    immutable_locals: &[bool],
    local_captures: &[bool],
    i: usize,
    v: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    let Some(name) = locals.get(i) else {
        return Ok(());
    };
    if name.starts_with("__kab_") {
        return Ok(());
    }
    if matches!(v, Value::Undefined) {
        return Ok(());
    }
    if immutable_locals.get(i) == Some(&true) {
        if !env.has_own_binding(name) && env.get(name).is_none() {
            env.set_const(name.clone(), v.clone());
        }
        return Ok(());
    }
    let is_capture = local_captures.get(i).copied().unwrap_or(false);
    if is_capture {
        // Update the shared enclosing activation.
        if env.has_own_binding(name) {
            env.set(name.clone(), v.clone());
        } else if env.get(name).is_some() {
            env.assign(name, v.clone())?;
        } else {
            env.set(name.clone(), v.clone());
        }
    } else {
        // Fresh fn-local: always shadow on this frame (never clobber parent same-name).
        env.set(name.clone(), v.clone());
    }
    Ok(())
}

fn refresh_function_closures(module: &BytecodeModule, env: &mut Environment) {
    let fn_names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    let data_only = env.clone_excluding(&fn_names);
    let temp_data = data_only.share_bindings();

    let mut fn_registry = Environment::new();
    for func in &module.functions {
        if let Some(Value::BytecodeFn(existing)) = env.get(&func.name) {
            fn_registry.set(
                func.name.clone(),
                Value::BytecodeFn(BytecodeFunction {
                    def: existing.def.clone(),
                    closure: temp_data.share_bindings(),
                }),
            );
        }
    }

    let mut data_env = Environment::child_from(&fn_registry);
    for name in data_only.all_binding_names() {
        if let Some(v) = data_only.get(&name) {
            data_env.set(name, v);
        }
    }
    let data_handle = data_env.share_bindings();

    for func in &module.functions {
        if let Some(Value::BytecodeFn(existing)) = fn_registry.get(&func.name) {
            fn_registry.set(
                func.name.clone(),
                Value::BytecodeFn(BytecodeFunction {
                    def: existing.def.clone(),
                    closure: data_handle.share_bindings(),
                }),
            );
        }
    }

    for func in &module.functions {
        if let Some(v) = fn_registry.get(&func.name) {
            env.set(func.name.clone(), v.clone());
        }
    }
}

pub fn prepare_exported_bytecode_fn(
    name: &str,
    func: BytecodeFunction,
    module_env: &Environment,
) -> BytecodeFunction {
    let Some(Value::BytecodeFn(refreshed)) = module_env.get(name) else {
        return func;
    };
    BytecodeFunction {
        def: refreshed.def.clone(),
        closure: refreshed.closure.share_bindings(),
    }
}

fn run_chunk(
    code: &[Opcode],
    constants: &[Constant],
    globals: &[String],
    locals: &[String],
    immutable_locals: &[bool],
    arrow_functions: &[BytecodeFnDef],
    classes: &[BytecodeClassDef],
    args: Option<(&BytecodeFnDef, Vec<Value>)>,
    module: Option<&BytecodeModule>,
    resume_local_vals: Option<Vec<Value>>,
    cursor: &mut ChunkCursor,
    generator_mode: bool,
    env: &mut Environment,
) -> Result<(ChunkExit, Vec<Value>), String> {
    let is_fresh = resume_local_vals.is_none();
    let mut local_vals =
        resume_local_vals.unwrap_or_else(|| vec![Value::Undefined; locals.len().max(1)]);
    let local_captures: &[bool] = args
        .as_ref()
        .map(|(f, _)| f.local_captures.as_slice())
        .unwrap_or(&[]);
    if is_fresh {
        if let Some((func, arg_vals)) = &args {
            for (i, param) in func.params.iter().enumerate() {
                if let Some(idx) = locals.iter().position(|l| l == param) {
                    local_vals[idx] = arg_vals.get(i).cloned().unwrap_or(Value::Undefined);
                }
            }
            // Captured enclosing locals are LoadLocal slots: seed from the closure env
            // (MakeArrowFn shares the enclosing activation via share_bindings).
            for (idx, name) in locals.iter().enumerate() {
                if name.starts_with("__kab_") {
                    continue;
                }
                if !matches!(local_vals.get(idx), Some(Value::Undefined) | None) {
                    continue;
                }
                if let Some(v) = env.get(name) {
                    if !matches!(v, Value::Undefined) {
                        local_vals[idx] = v;
                    }
                }
            }
        }
    }
    while cursor.ip < code.len() {
        let ip = &mut cursor.ip;
        let stack = &mut cursor.stack;
        match &code[*ip] {
            Opcode::Const(idx) => {
                let c = constants
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid const index {idx}"))?;
                push_stack(stack, const_to_value(c))?;
            }
            Opcode::LoadLocal(idx) => {
                let i = *idx as usize;
                let manual = chunk_is_manual(module, env);
                let v = load_local_value(&mut local_vals, locals, i, &args, env, manual)?;
                push_stack(stack, v)?;
            }
            Opcode::StoreLocal(idx) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true)
                    && !matches!(local_vals.get(i), Some(Value::Undefined))
                {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                if chunk_is_manual(module, env) {
                    if let Some(old) = local_vals.get(i) {
                        ownership::drop_owned_value(old, env)?;
                    }
                }
                local_vals[i] = v.clone();
                let must_mirror = args.is_none()
                    || immutable_locals.get(i) == Some(&true)
                    || local_captures.get(i).copied() == Some(true);
                if must_mirror {
                    store_local_to_env(
                        locals,
                        immutable_locals,
                        local_captures,
                        i,
                        &v,
                        env,
                    )?;
                }
            }
            Opcode::LoadGlobal(idx) => {
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                let v = global_ic_load(*idx, env, name)?;
                push_stack(stack, v)?;
            }
            Opcode::StoreGlobal(idx) => {
                global_ic_invalidate(*idx);
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?
                    .clone();
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                ownership::store_binding(env, &name, v)?;
            }
            Opcode::Pop => {
                let _ = stack.pop().ok_or("Bytecode stack underflow")?;
            }
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::And
            | Opcode::Or
            | Opcode::In
            | Opcode::BitAnd
            | Opcode::BitOr
            | Opcode::BitXor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Ushr => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                let op = code[*ip];
                push_stack(stack, eval_binary_op(&left, &opcode_to_binop(op), &right, env)?)?;
            }
            Opcode::JumpIfNotNullish(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !v.is_null() && !v.is_undefined() {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::Not => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Bool(!v.is_truthy()))?;
            }
            Opcode::Neg => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let negated = match v {
                    Value::Number(n) => Value::Number(-n),
                    Value::Float(f) => Value::Float(-f),
                    other => {
                        if let Some(r) = crate::runtime::stdlib::bigint::try_neg(&other) {
                            r?
                        } else {
                            return Err(format!("Cannot negate {other:?}"));
                        }
                    }
                };
                push_stack(stack, negated)?;
            }
            Opcode::BitNot => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, crate::ops::eval_unary_bitnot(&v)?)?;
            }
            Opcode::Jump(off) => {
                *ip = ((*ip as i32 + 1) + off) as usize;
                continue;
            }
            Opcode::JumpIfFalse(off) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                if !v.is_truthy() {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::Call(argc) => {
                let n = *argc as usize;
                if stack.len() < n + 1 {
                    return Err("Bytecode stack underflow on call".into());
                }
                let callee = stack.pop().ok_or("Bytecode stack underflow")?;
                // P10c: CALL_0 skips arg-buf; CALL_1/2/3/N share the recycled buf.
                let used_buf = n > 0;
                let call_args = if n == 0 {
                    Vec::new()
                } else {
                    take_call_args(stack, n)?
                };
                if let Value::NativeFunction(f) = callee {
                    let hit = CALL_IC.with(|ic| {
                        ic.borrow()
                            .native
                            .map(|p| std::ptr::fn_addr_eq(p, f))
                            .unwrap_or(false)
                    });
                    if hit {
                        CALL_IC_HITS.fetch_add(1, Ordering::Relaxed);
                    } else {
                        CALL_IC_MISSES.fetch_add(1, Ordering::Relaxed);
                        CALL_IC.with(|ic| {
                            let mut ic = ic.borrow_mut();
                            ic.native = Some(f);
                            ic.bc_ptr = 0;
                            ic.method = None;
                        });
                    }
                    let result = f(&call_args, env);
                    if used_buf {
                        recycle_call_args(call_args);
                    }
                    let result = result?;
                    if args.is_none() {
                        pull_env_into_local_vals(locals, &mut local_vals, env);
                    } else {
                        pull_captured_locals_from_env(
                            locals,
                            local_captures,
                            &mut local_vals,
                            env,
                        );
                        pull_object_locals_from_env(locals, &mut local_vals, env);
                    }
                    push_stack(stack, result)?;
                } else if let Value::BytecodeFn(func) = callee {
                    if !func.def.generator_fn && !func.def.async_fn {
                        let p = Rc::as_ptr(&func.def) as usize;
                        let hit = CALL_IC.with(|ic| ic.borrow().bc_ptr == p);
                        if hit {
                            CALL_IC_HITS.fetch_add(1, Ordering::Relaxed);
                        } else {
                            CALL_IC_MISSES.fetch_add(1, Ordering::Relaxed);
                            CALL_IC.with(|ic| {
                                let mut ic = ic.borrow_mut();
                                ic.bc_ptr = p;
                                ic.native = None;
                                ic.method = None;
                            });
                        }
                        let (result, obj_wb) = match call_bytecode_sync(func, call_args, env) {
                            Ok(v) => v,
                            Err(e) => {
                                if try_catch_propagated_throw(
                                    &e,
                                    args.as_ref().map(|(f, _)| *f),
                                    module,
                                    locals,
                                    immutable_locals,
                                    local_captures,
                                    &mut local_vals,
                                    env,
                                    ip,
                                    stack,
                                )? {
                                    continue;
                                }
                                return Err(e);
                            }
                        };
                        crate::runtime::closure_sync::apply_object_arg_writebacks(
                            &mut local_vals,
                            &obj_wb,
                        );
                        crate::runtime::closure_sync::apply_object_arg_writebacks_env(env, &obj_wb);
                        if args.is_none() {
                            pull_env_into_local_vals(locals, &mut local_vals, env);
                        } else {
                            pull_captured_locals_from_env(
                                locals,
                                local_captures,
                                &mut local_vals,
                                env,
                            );
                            pull_object_locals_from_env(locals, &mut local_vals, env);
                        }
                        push_stack(stack, result)?;
                    } else {
                        let result = match call_value(
                            Value::BytecodeFn(func),
                            call_args,
                            constants,
                            globals,
                            arrow_functions,
                            classes,
                            env,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                if try_catch_propagated_throw(
                                    &e,
                                    args.as_ref().map(|(f, _)| *f),
                                    module,
                                    locals,
                                    immutable_locals,
                                    local_captures,
                                    &mut local_vals,
                                    env,
                                    ip,
                                    stack,
                                )? {
                                    continue;
                                }
                                return Err(e);
                            }
                        };
                        if args.is_none() {
                            pull_env_into_local_vals(locals, &mut local_vals, env);
                        } else {
                            pull_captured_locals_from_env(
                                locals,
                                local_captures,
                                &mut local_vals,
                                env,
                            );
                            pull_object_locals_from_env(locals, &mut local_vals, env);
                        }
                        push_stack(stack, result)?;
                    }
                } else if let Value::BoundNative(receiver, f) = callee {
                    let hit = CALL_IC.with(|ic| {
                        ic.borrow()
                            .method
                            .map(|p| std::ptr::fn_addr_eq(p, f))
                            .unwrap_or(false)
                    });
                    if hit {
                        CALL_IC_HITS.fetch_add(1, Ordering::Relaxed);
                    } else {
                        CALL_IC_MISSES.fetch_add(1, Ordering::Relaxed);
                        CALL_IC.with(|ic| {
                            let mut ic = ic.borrow_mut();
                            ic.method = Some(f);
                            ic.native = None;
                            ic.bc_ptr = 0;
                        });
                    }
                    let mut recv = (*receiver).clone();
                    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut recv, env);
                    let mut method_args = Vec::with_capacity(call_args.len() + 1);
                    method_args.push(recv);
                    method_args.extend(call_args.iter().cloned());
                    let result = f(&method_args, env);
                    if used_buf {
                        recycle_call_args(call_args);
                    }
                    let result = result?;
                    if args.is_none() {
                        pull_env_into_local_vals(locals, &mut local_vals, env);
                    } else {
                        pull_captured_locals_from_env(
                            locals,
                            local_captures,
                            &mut local_vals,
                            env,
                        );
                        pull_object_locals_from_env(locals, &mut local_vals, env);
                    }
                    push_stack(stack, result)?;
                } else {
                    let result = match call_value(
                        callee,
                        call_args,
                        constants,
                        globals,
                        arrow_functions,
                        classes,
                        env,
                    ) {
                        Ok(v) => v,
                        Err(e) => {
                            if try_catch_propagated_throw(
                                &e,
                                args.as_ref().map(|(f, _)| *f),
                                module,
                                locals,
                                immutable_locals,
                                local_captures,
                                &mut local_vals,
                                env,
                                ip,
                                stack,
                            )? {
                                continue;
                            }
                            return Err(e);
                        }
                    };
                    if args.is_none() {
                        pull_env_into_local_vals(locals, &mut local_vals, env);
                    } else {
                        // Nested bytecode fn: refresh captures mutated via shared env, and
                        // object params written back by oid. Do not pull ordinary fn-locals
                        // (would wipe in-place ArrayPushLocal / similar).
                        pull_captured_locals_from_env(
                            locals,
                            local_captures,
                            &mut local_vals,
                            env,
                        );
                        pull_object_locals_from_env(locals, &mut local_vals, env);
                    }
                    push_stack(stack, result)?;
                }
            }
            Opcode::Dup => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow on dup")?
                    .clone();
                push_stack(stack, v)?;
            }
            Opcode::MakeArray(n) => {
                let count = *n as usize;
                if stack.len() < count {
                    return Err("Bytecode stack underflow on make_array".into());
                }
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(stack.pop().ok_or("Bytecode stack underflow")?);
                }
                items.reverse();
                crate::runtime::stdlib::weak::note_heap_alloc(1);
                push_stack(stack, Value::from_array(items))?;
            }
            Opcode::MakeObject(n) => {
                let count = *n as usize;
                if stack.len() < count * 2 {
                    return Err("Bytecode stack underflow on make_object".into());
                }
                let mut map = std::collections::HashMap::new();
                for _ in 0..count {
                    let key_val = stack.pop().ok_or("Bytecode stack underflow")?;
                    let Value::String(key) = key_val else {
                        return Err("Object key must be a string".into());
                    };
                    let val = stack.pop().ok_or("Bytecode stack underflow")?;
                    map.insert(key, val);
                }
                crate::runtime::stdlib::object::object_oid(&mut map);
                crate::runtime::stdlib::weak::note_heap_alloc(1);
                push_stack(stack, Value::from_object(map))?;
            }
            Opcode::IndexGet => {
                let idx = stack.pop().ok_or("Bytecode stack underflow")?;
                let container = stack.pop().ok_or("Bytecode stack underflow")?;
                // P1/P10: array[number] and object[string] without full read_index.
                let fast = match (&container, &idx) {
                    (Value::Array(items), Value::Number(n)) if *n >= 0 => {
                        let i = *n as usize;
                        if i < items.len() {
                            Some(items[i].clone())
                        } else {
                            None
                        }
                    }
                    (Value::Array(items), Value::Float(f))
                        if *f >= 0.0 && f.fract() == 0.0 =>
                    {
                        let i = *f as usize;
                        if i < items.len() {
                            Some(items[i].clone())
                        } else {
                            None
                        }
                    }
                    (Value::Object(map), Value::String(k)) => map.get(k).cloned(),
                    _ => None,
                };
                if let Some(v) = fast {
                    push_stack(stack, v)?;
                } else {
                    push_stack(stack, read_index(&container, &idx, env)?)?;
                }
            }
            Opcode::IndexSet => {
                let val = stack.pop().ok_or("Bytecode stack underflow")?;
                let idx = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut container = stack.pop().ok_or("Bytecode stack underflow")?;
                invalidate_member_ic();
                write_index(&mut container, &idx, val.clone(), env)?;
                push_stack(stack, container)?;
                push_stack(stack, val)?;
            }
            Opcode::GetLength => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, get_length(&v)?)?;
            }
            Opcode::ArrayPush => {
                let item = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                Value::reject_direct_container_cycle(&arr, &item)?;
                let Value::Array(ref mut items) = arr else {
                    return Err("array_push requires an array receiver".into());
                };
                Value::array_make_mut(items).push(item);
                let len = items.len() as i64;
                push_stack(stack, arr)?;
                push_stack(stack, Value::Number(len))?;
            }
            Opcode::TakeLocal(idx) => {
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true) {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                let taken = if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    if name.starts_with("__kab_") {
                        std::mem::replace(
                            local_vals.get_mut(i).ok_or("Invalid local index")?,
                            Value::Undefined,
                        )
                    } else {
                        env.take_binding(name)?
                    }
                } else {
                    let v = std::mem::replace(
                        local_vals.get_mut(i).ok_or("Invalid local index")?,
                        Value::Undefined,
                    );
                    if local_captures.get(i).copied() == Some(true) {
                        store_local_to_env(
                            locals,
                            immutable_locals,
                            local_captures,
                            i,
                            &Value::Undefined,
                            env,
                        )?;
                    }
                    v
                };
                push_stack(stack, taken)?;
            }
            Opcode::TakeGlobal(idx) => {
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                let taken = env.take_binding(name)?;
                push_stack(stack, taken)?;
            }
            Opcode::ArrayPushLocal(idx) => {
                let item = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true) {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                let Value::Array(ref mut items) = arr else {
                    return Err(format!(
                        "array_push_local requires an array (got {})",
                        crate::value::format_value(&arr)
                    ));
                };
                Rc::make_mut(items).push(item);
                let len = items.len() as i64;
                let stored = arr;
                if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    if name.starts_with("__kab_") {
                        local_vals[i] = stored;
                    } else {
                        env.set(name.to_string(), stored);
                    }
                } else {
                    local_vals[i] = stored.clone();
                    store_local_to_env(
                        locals,
                        immutable_locals,
                        local_captures,
                        i,
                        &stored,
                        env,
                    )?;
                }
                push_stack(stack, Value::Number(len))?;
            }
            Opcode::ArrayPushGlobal(idx) => {
                global_ic_invalidate(*idx);
                let item = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                let Value::Array(ref mut items) = arr else {
                    return Err(format!(
                        "array_push_global requires an array (got {})",
                        crate::value::format_value(&arr)
                    ));
                };
                Rc::make_mut(items).push(item);
                let len = items.len() as i64;
                ownership::store_binding(env, name, arr)?;
                push_stack(stack, Value::Number(len))?;
            }
            Opcode::ArrayPopLocal(idx) => {
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true) {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    env.array_pop_inplace(name)?;
                } else {
                    match local_vals.get_mut(i) {
                        Some(Value::Array(ref mut items)) => {
                            let _ = Rc::make_mut(items).pop();
                        }
                        other => {
                            return Err(format!(
                                "array_pop_local requires an array local (got {})",
                                crate::value::format_value(
                                    other.map_or(&Value::Undefined, |v| &*v)
                                )
                            ));
                        }
                    }
                    if local_captures.get(i).copied() == Some(true) {
                        let synced = local_vals.get(i).cloned().unwrap_or(Value::Undefined);
                        store_local_to_env(
                            locals,
                            immutable_locals,
                            local_captures,
                            i,
                            &synced,
                            env,
                        )?;
                    }
                }
                push_stack(stack, Value::Null)?;
            }
            Opcode::ArrayPopGlobal(idx) => {
                global_ic_invalidate(*idx);
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                env.array_pop_inplace(name)?;
                push_stack(stack, Value::Null)?;
            }
            Opcode::AccAddLocal(idx) => {
                let rhs = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true) {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    env.acc_add_inplace(name, rhs)?;
                    if let Some(v) = env.get(name) {
                        local_vals[i] = v;
                    }
                } else if matches!(
                    (local_vals.get(i), &rhs),
                    (Some(Value::Number(_)), Value::Number(_))
                ) {
                    if let (Some(Value::Number(n)), Value::Number(m)) =
                        (local_vals.get_mut(i), rhs)
                    {
                        *n += m;
                    }
                    if local_captures.get(i).copied() == Some(true) {
                        store_local_to_env(
                            locals,
                            immutable_locals,
                            local_captures,
                            i,
                            &local_vals[i],
                            env,
                        )?;
                    }
                } else {
                    crate::value::acc_add_value(
                        local_vals.get_mut(i).ok_or("Invalid local index")?,
                        rhs,
                    )?;
                    if local_captures.get(i).copied() == Some(true) {
                        let synced = local_vals.get(i).cloned().unwrap_or(Value::Undefined);
                        store_local_to_env(
                            locals,
                            immutable_locals,
                            local_captures,
                            i,
                            &synced,
                            env,
                        )?;
                    }
                }
            }
            Opcode::AccAddGlobal(idx) => {
                global_ic_invalidate(*idx);
                let rhs = stack.pop().ok_or("Bytecode stack underflow")?;
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                env.acc_add_inplace(name, rhs)?;
            }
            Opcode::LenLocal(idx) => {
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                let n = if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    env.len_of(name)?
                } else {
                    crate::value::container_len(
                        local_vals.get(i).unwrap_or(&Value::Undefined),
                    )?
                };
                push_stack(stack, Value::Number(n))?;
            }
            Opcode::LenGlobal(idx) => {
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                push_stack(stack, Value::Number(env.len_of(name)?))?;
            }
            Opcode::IndexGetLocal(idx) => {
                let index = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                let v = if args.is_none() {
                    let name = locals
                        .get(i)
                        .ok_or_else(|| format!("Invalid local index {i}"))?;
                    env.index_get_clone(name, &index)?
                } else {
                    crate::value::index_get_element(
                        local_vals.get(i).unwrap_or(&Value::Undefined),
                        &index,
                    )?
                };
                push_stack(stack, v)?;
            }
            Opcode::IndexGetGlobal(idx) => {
                let index = stack.pop().ok_or("Bytecode stack underflow")?;
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                push_stack(stack, env.index_get_clone(name, &index)?)?;
            }
            Opcode::GetMember(key_idx) => {
                let key = member_name(constants, *key_idx)?;
                let container = stack.pop().ok_or("Bytecode stack underflow")?;
                let val = if let Value::EnumNamespace(type_name) = &container {
                    crate::class::resolve_enum_member(type_name, key, env)?
                } else if let Value::Object(map) = &container {
                    if let Some(v) = map.get(key) {
                        MEMBER_IC_HITS.fetch_add(1, Ordering::Relaxed);
                        crate::runtime::ptak::note_shape_hit();
                        v.clone()
                    } else {
                        MEMBER_IC_MISSES.fetch_add(1, Ordering::Relaxed);
                        read_member(&container, key, env)?
                    }
                } else {
                    read_member(&container, key, env)?
                };
                push_stack(
                    stack,
                    crate::runtime::stdlib::object::bind_object_method(container, val),
                )?;
            }
            Opcode::MemberSet(key_idx) => {
                let key = member_name(constants, *key_idx)?;
                let val = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut container = stack.pop().ok_or("Bytecode stack underflow")?;
                if let Value::ClassInstance(inst) = &container {
                    let guard = inst
                        .try_borrow()
                        .map_err(|e| format!("class instance borrow: {e}"))?;
                    crate::class::type_check::validate_class_field_write(
                        &guard,
                        key,
                        &val,
                        env.classes(),
                    )?;
                }
                // Invalidate IC for this object on write.
                invalidate_member_ic();
                crate::runtime::ptak::note_shape_transition();
                write_member(&mut container, key, val.clone(), env)?;
                push_stack(stack, container)?;
                push_stack(stack, val)?;
            }
            Opcode::Swap => {
                let top = stack.pop().ok_or("Bytecode stack underflow")?;
                let second = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, top)?;
                push_stack(stack, second)?;
            }
            Opcode::ConcatArray => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut left = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(ref mut a) = left else {
                    return Err("ConcatArray requires two arrays".into());
                };
                let Value::Array(b) = right else {
                    return Err("ConcatArray requires two arrays".into());
                };
                Rc::make_mut(a).extend(b.iter().cloned());
                push_stack(stack, left)?;
            }
            Opcode::MergeObject => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut left = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Object(ref mut a) = left else {
                    return Err("MergeObject requires two objects".into());
                };
                let Value::Object(b) = right else {
                    return Err("MergeObject requires two objects".into());
                };
                for (k, v) in b.iter() {
                    Rc::make_mut(a).insert(k.clone(), v.clone());
                }
                invalidate_member_ic();
                push_stack(stack, left)?;
            }
            Opcode::CallFromArray => {
                let args_arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let callee = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = args_arr else {
                    return Err("CallFromArray requires an array of arguments".into());
                };
                let result = match call_value(
                    callee,
                    items.to_vec(),
                    constants,
                    globals,
                    arrow_functions,
                    classes,
                    env,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        if try_catch_propagated_throw(
                            &e,
                            args.as_ref().map(|(f, _)| *f),
                            module,
                            locals,
                            immutable_locals,
                            local_captures,
                            &mut local_vals,
                            env,
                            ip,
                            stack,
                        )? {
                            continue;
                        }
                        return Err(e);
                    }
                };
                push_stack(stack, result)?;
            }
            Opcode::MakeOk => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Result(Ok(Box::new(v))))?;
            }
            Opcode::MakeErr => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Result(Err(Box::new(v))))?;
            }
            Opcode::MakeSome => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Option(Some(Box::new(v))))?;
            }
            Opcode::MakeNone => {
                push_stack(stack, Value::Option(None))?;
            }
            Opcode::JumpIfResultErr(off) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                match v {
                    Value::Result(Err(e)) => {
                        push_stack(stack, *e)?;
                        *ip = ((*ip as i32 + 1) + off) as usize;
                        continue;
                    }
                    Value::Result(Ok(o)) => push_stack(stack, *o)?,
                    other => push_stack(stack, other)?,
                }
            }
            Opcode::ArraySliceFrom(start) => {
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = arr else {
                    return Err("ArraySliceFrom requires an array".into());
                };
                let start = *start as usize;
                push_stack(stack, Value::from_array(items.get(start..).unwrap_or(&[]).to_vec()))?;
            }
            Opcode::MakeArrowFn(idx) => {
                sync_locals_into_env(locals, &local_vals, env);
                let arrow = arrow_functions
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid arrow function index {idx}"))?
                    .clone();
                let mut f = arrow;
                if f.globals.is_empty() {
                    f.globals = globals.to_vec();
                }
                if f.constants.is_empty() {
                    f.constants = constants.to_vec();
                }
                // Share this activation frame so later StoreLocal updates are visible,
                // without aliasing recursive sibling frames (each call has its own env).
                push_stack(stack, Value::BytecodeFn(BytecodeFunction {
                    def: Rc::new(f),
                    closure: env.share_bindings(),
                }))?;
            }
            Opcode::JumpUnlessResultOk(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Result(Ok(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapResultOk => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Result(Ok(inner)) = v else {
                    return Err("UnwrapResultOk requires Result Ok".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessResultErr(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Result(Err(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapResultErr => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Result(Err(inner)) = v else {
                    return Err("UnwrapResultErr requires Result Err".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessOptionSome(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Option(Some(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapOptionSome => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Option(Some(inner)) = v else {
                    return Err("UnwrapOptionSome requires Option Some".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessOptionNone(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Option(None)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessEnumVariant(type_idx, variant_idx, off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                let type_name = member_name(constants, *type_idx)?;
                let variant = member_name(constants, *variant_idx)?;
                let matches = match v {
                    Value::EnumValue {
                        type_name: tn,
                        variant: vr,
                        ..
                    } => {
                        vr == variant
                            && (tn == type_name
                                || tn.starts_with(&format!("{type_name}$")))
                    }
                    _ => false,
                };
                if !matches {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnpackEnumFields(n) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::EnumValue { fields, .. } = v else {
                    return Err("UnpackEnumFields requires EnumValue".into());
                };
                if fields.len() != *n as usize {
                    return Err(format!(
                        "UnpackEnumFields expected {n} fields, got {}",
                        fields.len()
                    ));
                }
                for f in fields.into_iter().rev() {
                    push_stack(stack, f)?;
                }
            }
            Opcode::JumpUnlessConstEq(const_idx, off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                let c = constants
                    .get(*const_idx as usize)
                    .ok_or_else(|| format!("Invalid const index {const_idx}"))?;
                let matches = if matches!(c, Constant::Nan) {
                    Value::Bool(v.is_nan())
                } else {
                    let expected = const_to_value(c);
                    eval_binary_op(v, &crate::ast::BinaryOp::Eq, &expected, env)?
                };
                if let Value::Bool(false) = matches {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessArray(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Array(_)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessObject(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Object(_)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessObjectEmpty(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                let empty = matches!(v, Value::Object(map) if crate::runtime::stdlib::object::object_is_pattern_empty(map));
                if !empty {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessHasMember(key_idx, off) => {
                let key = member_name(constants, *key_idx)?;
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                let has = matches!(v, Value::Object(map) if map.contains_key(key));
                if !has {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::IndexPeekFromEnd(from_end) => {
                let from_end = *from_end as usize;
                let peek = {
                    let arr = stack
                        .last()
                        .ok_or("Bytecode stack underflow")?;
                    let Value::Array(items) = arr else {
                        return Err("IndexPeekFromEnd requires an array".into());
                    };
                    if from_end == 0 || from_end > items.len() {
                        return Err("IndexPeekFromEnd index out of range".into());
                    }
                    items[items.len() - from_end].clone()
                };
                push_stack(stack, peek)?;
            }
            Opcode::ArraySliceRest(start_trim, end_trim) => {
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = arr else {
                    return Err("ArraySliceRest requires an array".into());
                };
                let start = *start_trim as usize;
                let end_trim = *end_trim as usize;
                if start + end_trim > items.len() {
                    push_stack(stack, Value::from_array(Vec::new()))?;
                } else {
                    push_stack(stack, Value::from_array(items[start..items.len() - end_trim].to_vec()))?;
                }
            }
            Opcode::ObjectRest(key_count) => {
                let count = *key_count as usize;
                if stack.len() < count + 1 {
                    return Err("Bytecode stack underflow on object_rest".into());
                }
                let mut exclude = std::collections::HashSet::new();
                for _ in 0..count {
                    let key_val = stack.pop().ok_or("Bytecode stack underflow")?;
                    let Value::String(key) = key_val else {
                        return Err("ObjectRest keys must be strings".into());
                    };
                    exclude.insert(key);
                }
                let obj = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Object(map) = obj else {
                    return Err("ObjectRest requires an object".into());
                };
                let rest: std::collections::HashMap<String, Value> = map
                    .iter()
                    .filter(|(k, _)| !exclude.contains(k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                push_stack(stack, Value::from_object(rest))?;
            }
            Opcode::MatchFail => return Err("No matching pattern".into()),
            Opcode::Await => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                // Expose this frame's locals before nested microtask drain (async tasks /
                // module lets may mutate shared bindings). Always refresh after — including
                // module main (`args.is_none()`), matching `Call` (L4).
                sync_locals_into_env(locals, &local_vals, env);
                let resolved = crate::evaluator::resolve_await_value(v, env)?;
                pull_env_into_local_vals(locals, &mut local_vals, env);
                push_stack(stack, resolved)?;
            }
            Opcode::Yield => {
                if !generator_mode {
                    return Err("yield outside generator function".into());
                }
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                *ip += 1;
                return Ok((ChunkExit::Yield(v), local_vals));
            }
            Opcode::YieldStar => {
                if !generator_mode {
                    return Err("yield* outside generator function".into());
                }
                let iterable = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut it = if cursor.generator_async {
                    crate::runtime::stdlib::async_iterator::get_async_iterator(&iterable, env)?
                } else {
                    crate::runtime::stdlib::iterator::get_sync_iterator(&iterable, env)?
                };
                let (value, done) =
                    delegate_next_step(&mut it, None, cursor.generator_async, env)?;
                if done {
                    push_stack(stack, value)?;
                } else {
                    cursor.delegate = Some(it);
                    return Ok((ChunkExit::Yield(value), local_vals));
                }
            }
            Opcode::IteratorStepInPlace => {
                let mut it = stack.pop().ok_or("Bytecode stack underflow")?;
                let (value, done) =
                    crate::runtime::stdlib::iterator::iterator_next(&mut it, env)?;
                push_stack(stack, it)?;
                push_stack(
                    stack,
                    crate::runtime::stdlib::iterator::iterator_result(value, done),
                )?;
            }
            Opcode::AsyncIteratorStepInPlace => {
                let mut it = stack.pop().ok_or("Bytecode stack underflow")?;
                let next_p =
                    crate::runtime::stdlib::async_iterator::async_iterator_next(&mut it, env)?;
                push_stack(stack, it)?;
                push_stack(stack, next_p)?;
            }
            Opcode::NewInstance(class_idx, argc) => {
                let n = *argc as usize;
                if stack.len() < n {
                    return Err("Bytecode stack underflow on new_instance".into());
                }
                let mut call_args = Vec::with_capacity(n);
                for _ in 0..n {
                    call_args.push(stack.pop().ok_or("Bytecode stack underflow")?);
                }
                call_args.reverse();
                let class = classes
                    .get(*class_idx as usize)
                    .ok_or_else(|| format!("Invalid class index {class_idx}"))?;
                let instance = instantiate_class(class, classes, call_args.to_vec(), env)?;
                push_stack(stack, instance)?;
            }
            Opcode::NewInstanceFromArray(class_idx) => {
                let mut arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(call_args) = arr else {
                    return Err("Spread constructor requires an array of arguments".into());
                };
                let class = classes
                    .get(*class_idx as usize)
                    .ok_or_else(|| format!("Invalid class index {class_idx}"))?;
                let instance = instantiate_class(class, classes, call_args.to_vec(), env)?;
                push_stack(stack, instance)?;
            }
            Opcode::GetSuperMethod(key_idx) => {
                let member = member_name(constants, *key_idx)?;
                let this_val = env
                    .get("this")
                    .ok_or_else(|| "`super` used outside of method".to_string())?;
                let Value::ClassInstance(inst) = this_val else {
                    return Err("`super` requires class instance `this`".into());
                };
                let v = crate::evaluator::resolve_super_member(&inst, member, env)?;
                push_stack(stack, v)?;
            }
            Opcode::ResultQuestion => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                match v {
                    Value::Result(Ok(inner)) => push_stack(stack, *inner)?,
                    Value::Result(Err(e)) => push_stack(stack, Value::Result(Err(e)))?,
                    other => {
                        return Err(format!(
                            "? operator requires Result (Ok/Err), got {}",
                            crate::value::format_value(&other)
                        ))?;
                    }
                }
            }
            Opcode::Throw => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let ip_now = *ip;
                let region = args
                    .as_ref()
                    .and_then(|(func, _)| find_try_region_for_ip(func, ip_now))
                    .or_else(|| {
                        module.and_then(|m| {
                            m.main_try_regions
                                .iter()
                                .filter(|r| ip_now >= r.body_start && ip_now <= r.body_end)
                                .max_by_key(|r| r.body_start)
                        })
                    });
                if let Some(region) = region {
                    let li = region.err_local as usize;
                    if li >= local_vals.len() {
                        local_vals.resize(li + 1, Value::Undefined);
                    }
                    let caught = crate::runtime::stdlib::error::enrich_error_value_for_catch(v.clone());
                    local_vals[li] = caught.clone();
                    store_local_to_env(locals, immutable_locals, local_captures, li, &caught, env)?;
                    push_stack(stack, caught)?;
                    *ip = region.catch_start;
                    continue;
                }
                return Err(crate::runtime::stdlib::error::throw_value(v));
            }
            Opcode::Return => {
                let v = stack.pop().unwrap_or(Value::Undefined);
                if chunk_is_manual(module, env) {
                    drop_owned_locals(&local_vals, env)?;
                }
                return Ok((ChunkExit::Done(v), local_vals));
            }
            Opcode::Halt => {
                if chunk_is_manual(module, env) {
                    drop_owned_locals(&local_vals, env)?;
                }
                return Ok((
                    ChunkExit::Done(stack.pop().unwrap_or(Value::Null)),
                    local_vals,
                ));
            }
        }
        *ip += 1;
    }
    Ok((
        ChunkExit::Done(cursor.stack.pop().unwrap_or(Value::Null)),
        local_vals,
    ))
}

/// After a bytecode call, copy mutated locals (e.g. object params) back into `env` bindings.
fn sync_fn_locals_to_env(func: &BytecodeFnDef, local_vals: &[Value], env: &mut Environment) {
    for (i, name) in func.locals.iter().enumerate() {
        let Some(v) = local_vals.get(i) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        env.set(name.clone(), v.clone());
    }
}

pub fn run_bytecode_fn(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    run_bytecode_fn_with_locals(func, args, env).map(|(v, _)| v)
}

pub fn run_bytecode_fn_with_locals(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<(Value, Vec<Value>), String> {
    if let Some(typed) = super::typed::try_run_typed_i64(func, &args) {
        let (v, local_vals) = typed?;
        sync_fn_locals_to_env(func, &local_vals, env);
        return Ok((v, local_vals));
    }
    bind_bytecode_params(func, &args, env);
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, local_vals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        &[],
        Some((func, args)),
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    sync_fn_locals_to_env(func, &local_vals, env);
    let v = match exit {
        ChunkExit::Done(v) => v,
        ChunkExit::Yield(_) => return Err("yield outside generator".into()),
    };
    Ok((v, local_vals))
}

pub fn run_expr_snippet(
    code: &[Opcode],
    constants: &[Constant],
    globals: &[String],
    env: &mut Environment,
) -> Result<Value, String> {
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, _) = run_chunk(
        code,
        constants,
        globals,
        &[],
        &[],
        &[],
        &[],
        None,
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    match exit {
        ChunkExit::Done(v) => Ok(v),
        ChunkExit::Yield(_) => Err("yield outside generator".into()),
    }
}

pub fn run_generator_step(
    func: &BytecodeFnDef,
    arg_vals: &[Value],
    local_vals: &mut Vec<Value>,
    cursor: &mut ChunkCursor,
    env: &mut Environment,
    resume: Option<GeneratorResume>,
) -> Result<(Value, bool), String> {
    if let Some(GeneratorResume::Return(value)) = resume {
        if cursor.delegate.is_some() {
            let mut del = cursor
                .delegate
                .take()
                .ok_or("internal generator delegate missing")?;
            let (completion, done) =
                delegate_return_step(&mut del, value.clone(), cursor.generator_async, env)?;
            if !done {
                cursor.delegate = Some(del);
                return Ok((completion, false));
            }
            return Ok((value, true));
        }
        return Ok((value, true));
    }
    let mut resume = resume;
    if cursor.delegate.is_some() {
        if let Some(yielded) = continue_generator_delegate(cursor, resume.take(), env)? {
            return Ok((yielded, false));
        }
    } else if let Some(res) = resume.take() {
        match res {
            GeneratorResume::Next(v) => {
                cursor.stack.push(v);
            }
            GeneratorResume::Throw(reason) => {
                let ip = cursor.ip;
                let Some(region) = find_try_region_for_ip(func, ip) else {
                    return Err(format!(
                        "uncaught generator throw: {}",
                        crate::value::format_value(&reason)
                    ));
                };
                let li = region.err_local as usize;
                if li >= local_vals.len() {
                    local_vals.resize(li + 1, Value::Undefined);
                }
                local_vals[li] = reason.clone();
                cursor.stack.push(reason);
                cursor.ip = region.catch_start;
            }
            GeneratorResume::Return(_) => unreachable!("return handled above"),
        }
    }
    let (exit, updated_locals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        &[],
        Some((func, arg_vals.to_vec())),
        None,
        Some(std::mem::take(local_vals)),
        cursor,
        true,
        env,
    )?;
    *local_vals = updated_locals;
    match exit {
        ChunkExit::Yield(v) => Ok((v, false)),
        ChunkExit::Done(v) => Ok((v, true)),
    }
}

/// Advance an active `yield*` delegate; returns `Some(value)` when another value should be yielded.
fn continue_generator_delegate(
    cursor: &mut ChunkCursor,
    resume: Option<GeneratorResume>,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    let mut del = cursor
        .delegate
        .take()
        .ok_or("internal generator delegate missing")?;
    let (value, done) = match resume {
        Some(GeneratorResume::Throw(reason)) => {
            let result = delegate_throw_step(&mut del, reason, cursor.generator_async, env)?;
            crate::runtime::stdlib::iterator::parse_iterator_result(&result)?
        }
        Some(GeneratorResume::Return(_)) => {
            return Err("internal: return resume in delegate continue".into());
        }
        other => delegate_next_step(&mut del, other, cursor.generator_async, env)?,
    };
    if !done {
        cursor.delegate = Some(del);
        return Ok(Some(value));
    }
    cursor.stack.push(value);
    cursor.delegate = None;
    cursor.ip += 1;
    Ok(None)
}

fn delegate_throw_step(
    del: &mut Value,
    reason: Value,
    host_async: bool,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        return crate::runtime::stdlib::generator::throw_generator(del, reason, env);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        if let Value::Object(ref mut map) = del {
            if let Some(mut sync) = map.get(crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE).cloned() {
                let result =
                    crate::runtime::stdlib::iterator::iterator_throw(&mut sync, reason, env)?;
                Rc::make_mut(map).insert(
                    crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE.into(),
                    sync,
                );
                return Ok(result);
            }
        }
    }
    crate::runtime::stdlib::iterator::iterator_throw(del, reason, env)
}

fn delegate_return_step(
    del: &mut Value,
    value: Value,
    host_async: bool,
    env: &mut Environment,
) -> Result<(Value, bool), String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        let result = crate::runtime::stdlib::generator::return_generator(del, value, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        if let Value::Object(ref mut map) = del {
            if let Some(mut sync) = map.get(crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE).cloned() {
                let result =
                    crate::runtime::stdlib::iterator::iterator_return(&mut sync, value, env)?;
                Rc::make_mut(map).insert(
                    crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE.into(),
                    sync,
                );
                return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
            }
        }
    }
    let result = crate::runtime::stdlib::iterator::iterator_return(del, value, env)?;
    crate::runtime::stdlib::iterator::parse_iterator_result(&result)
}

fn delegate_next_step(
    del: &mut Value,
    resume: Option<GeneratorResume>,
    host_async: bool,
    env: &mut Environment,
) -> Result<(Value, bool), String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        let gen_resume = match resume {
            Some(GeneratorResume::Next(v)) => Some(GeneratorResume::Next(v)),
            Some(GeneratorResume::Throw(reason)) => Some(GeneratorResume::Throw(reason)),
            Some(GeneratorResume::Return(value)) => {
                return delegate_return_step(del, value, host_async, env);
            }
            None => None,
        };
        let result =
            crate::runtime::stdlib::generator::advance_generator(del, gen_resume, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        let _ = resume;
        let next_p = crate::runtime::stdlib::async_iterator::async_iterator_next(del, env)?;
        let result = crate::evaluator::resolve_await_value(next_p, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    let _ = resume;
    crate::runtime::stdlib::iterator::iterator_next(del, env)
}

pub fn run_bytecode_fn_with_classes(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    classes: &[BytecodeClassDef],
    env: &mut Environment,
) -> Result<Value, String> {
    bind_bytecode_params(func, &args, env);
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, local_vals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        classes,
        Some((func, args)),
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    sync_fn_locals_to_env(func, &local_vals, env);
    match exit {
        ChunkExit::Done(v) => Ok(v),
        ChunkExit::Yield(_) => Err("yield outside generator".into()),
    }
}

/// Bind bytecode parameter names in `env` so `LoadGlobal` can resolve them from natives.
pub fn bind_bytecode_params(func: &BytecodeFnDef, args: &[Value], env: &mut Environment) {
    for (p, a) in func.params.iter().zip(args) {
        env.set(p.clone(), a.clone());
    }
}

pub fn schedule_bytecode_async(
    func: Rc<BytecodeFnDef>,
    args: Vec<Value>,
    closure: Environment,
    env: &Environment,
) -> Result<Value, String> {
    if func.params.len() != args.len() {
        return Err(format!(
            "Argument count mismatch: expected {}, got {}",
            func.params.len(),
            args.len()
        ));
    }
    let promise: crate::value::SharedPromise =
        Rc::new(RefCell::new(PromiseValue::Pending));
    env.schedule_microtask(Microtask {
        promise: promise.clone(),
        params: func.params.clone(),
        body: AsyncBody::Bytecode(func),
        env: closure,
        args,
        bindings: Vec::new(),
        writeback: Some(env.share_bindings()),
    });
    Ok(Value::Promise(promise))
}

fn call_bytecode_sync(
    mut func: BytecodeFunction,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<(Value, Vec<(usize, Value)>), String> {
    crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
    crate::runtime::closure_sync::pull_root_into_closure(&mut func.closure, env);
    let mut call_env = Environment::child_from(&func.closure);
    let needs_obj_writeback = args.iter().any(|a| matches!(a, Value::Object(_)));
    let orig_args = if needs_obj_writeback {
        args.clone()
    } else {
        Vec::new()
    };
    let (result, local_vals) =
        run_bytecode_fn_with_locals(func.def.as_ref(), args, &mut call_env)?;
    let capture_names: Vec<String> = func
        .def
        .locals
        .iter()
        .enumerate()
        .filter(|(i, _)| func.def.local_captures.get(*i).copied().unwrap_or(false))
        .map(|(_, n)| n.clone())
        .collect();
    crate::runtime::closure_sync::sync_closure_writes_filtered(
        &func.closure,
        &call_env,
        env,
        Some(&capture_names),
    );
    crate::runtime::closure_sync::sync_bytecode_globals_to_root(&func, &call_env, env);
    let mut wbs = Vec::new();
    if needs_obj_writeback {
        crate::runtime::closure_sync::writeback_object_args(
            func.def.as_ref(),
            &orig_args,
            &local_vals,
            env,
        );
        wbs = crate::runtime::closure_sync::object_arg_writebacks(
            func.def.as_ref(),
            &orig_args,
            &local_vals,
        );
    }
    Ok((result, wbs))
}

pub fn call_value(
    callee: Value,
    args: Vec<Value>,
    _constants: &[Constant],
    _globals: &[String],
    _arrow_functions: &[BytecodeFnDef],
    classes: &[BytecodeClassDef],
    env: &mut Environment,
) -> Result<Value, String> {
    match callee {
        Value::BytecodeFn(func) => {
            if func.def.generator_fn {
                return crate::runtime::stdlib::generator::create_generator(func, args, env);
            }
            if func.def.async_fn {
                let mut func = func;
                crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
                crate::runtime::closure_sync::pull_root_into_closure(&mut func.closure, env);
                return schedule_bytecode_async(
                    func.def.clone(),
                    args,
                    func.closure.share_bindings(),
                    env,
                );
            }
            let (v, _wb) = call_bytecode_sync(func, args, env)?;
            Ok(v)
        }
        Value::NativeFunction(f) => f(&args, env),
        callee if crate::runtime::stdlib::symbol::is_symbol_ctor_object(&callee) => {
            crate::runtime::stdlib::symbol::try_symbol_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::proxy::is_proxy_ctor_object(&callee) => {
            crate::runtime::stdlib::proxy::try_proxy_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::weak::is_weakref_ctor_object(&callee) => {
            crate::runtime::stdlib::weak::try_weakref_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::weak::is_finreg_ctor_object(&callee) => {
            crate::runtime::stdlib::weak::try_finreg_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::intl::is_number_format_ctor(&callee) => {
            crate::runtime::stdlib::intl::try_number_format_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::intl::is_date_time_format_ctor(&callee) => {
            crate::runtime::stdlib::intl::try_date_time_format_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::temporal::is_plain_date_ctor(&callee) => {
            crate::runtime::stdlib::temporal::try_plain_date_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::temporal::is_instant_ctor(&callee) => {
            crate::runtime::stdlib::temporal::try_instant_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::proxy::is_proxy(&callee) => {
            crate::runtime::stdlib::proxy::trap_apply(
                &callee,
                Value::Undefined,
                args,
                env,
            )
        }
        Value::BoundNative(receiver, f) => {
            let mut recv = (*receiver).clone();
            crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut recv, env);
            let mut call_args = vec![recv];
            call_args.extend(args);
            f(&call_args, env)
        }
        Value::PromiseSettler { ctrl_id, reject } => {
            crate::runtime::stdlib::promise::call_settler(ctrl_id, reject, &args, env)
        }
        Value::Function {
            params,
            body,
            env: closure_env,
            async_fn: false,
            ..
        } => {
            if params.len() != args.len() {
                return Err(format!(
                    "Argument count mismatch: expected {}, got {}",
                    params.len(),
                    args.len()
                ));
            }
            let mut call_env = Environment::child(closure_env.clone());
            for (p, a) in params.iter().zip(args) {
                call_env.set(p.clone(), a);
            }
            let result = crate::evaluator::eval_expr(&body, &mut call_env)?;
            crate::runtime::closure_sync::sync_closure_writes(&closure_env, &call_env, env);
            Ok(result)
        }
        Value::BoundMethod(instance, method) => {
            if method.params.len() != args.len() {
                return Err(format!(
                    "Argument count mismatch: expected {}, got {}",
                    method.params.len(),
                    args.len()
                ));
            }
            let (owner, recv) = {
                let inst_ref = instance
                    .try_borrow()
                    .map_err(|e| format!("class instance borrow: {e}"))?;
                let owner = crate::class::method_owner_class(&method, &inst_ref);
                let recv = crate::class::receiver_binding(inst_ref.is_struct);
                (owner, recv)
            };
            if let Some(bc) = &method.bytecode {
                let mut call_env = crate::evaluator::create_global_env();
                *call_env.classes_mut() = env.classes().clone();
                call_env.set_private_scope(Some(&owner));
                call_env.set(
                    recv.to_string(),
                    Value::ClassInstance(instance.clone()),
                );
                let result = run_bytecode_fn_with_classes(bc, args, classes, &mut call_env)?;
                if env.get(recv).is_some() {
                    if let Some(Value::ClassInstance(updated)) = call_env.get(recv) {
                        env.assign(recv, Value::ClassInstance(updated.clone()))?;
                    }
                }
                return Ok(result);
            }
            let mut call_env = crate::evaluator::create_global_env();
            *call_env.classes_mut() = env.classes().clone();
            call_env.set_private_scope(Some(&owner));
            call_env.set(
                recv.to_string(),
                Value::ClassInstance(instance.clone()),
            );
            for (p, a) in method.params.iter().zip(args) {
                call_env.set(p.clone(), a);
            }
            let result = crate::evaluator::eval_expr(&method.body, &mut call_env)?;
            if env.get(recv).is_some() {
                if let Some(Value::ClassInstance(updated)) = call_env.get(recv) {
                    env.assign(recv, Value::ClassInstance(updated.clone()))?;
                }
            }
            Ok(result)
        }
        Value::EnumCtor {
            type_name,
            variant,
            arity,
        } => crate::class::invoke_enum_ctor(&type_name, &variant, arity, args),
        other => Err(format!("Not a function: {other:?}")),
    }
}

fn member_name(constants: &[Constant], idx: u16) -> Result<&str, String> {
    match constants.get(idx as usize) {
        Some(Constant::String(s)) => Ok(s.as_str()),
        _ => Err(format!("Invalid member name const index {idx}")),
    }
}

fn opcode_to_binop(op: Opcode) -> crate::ast::BinaryOp {
    use crate::ast::BinaryOp;
    match op {
        Opcode::Add => BinaryOp::Add,
        Opcode::Sub => BinaryOp::Sub,
        Opcode::Mul => BinaryOp::Mul,
        Opcode::Div => BinaryOp::Div,
        Opcode::Mod => BinaryOp::Mod,
        Opcode::Pow => BinaryOp::Pow,
        Opcode::Eq => BinaryOp::Eq,
        Opcode::Ne => BinaryOp::Ne,
        Opcode::Lt => BinaryOp::Lt,
        Opcode::Le => BinaryOp::Le,
        Opcode::Gt => BinaryOp::Gt,
        Opcode::Ge => BinaryOp::Ge,
        Opcode::And => BinaryOp::And,
        Opcode::Or => BinaryOp::Or,
        Opcode::In => BinaryOp::In,
        Opcode::BitAnd => BinaryOp::BitAnd,
        Opcode::BitOr => BinaryOp::BitOr,
        Opcode::BitXor => BinaryOp::BitXor,
        Opcode::Shl => BinaryOp::Shl,
        Opcode::Shr => BinaryOp::Shr,
        Opcode::Ushr => BinaryOp::Ushr,
        _ => BinaryOp::Add,
    }
}

#[cfg(test)]
mod tests {
    use super::run_module;
    use crate::bytecode::compiler::try_compile;
    use crate::bytecode::compile_source;
    use crate::evaluator::create_global_env;
    use crate::Value;

    #[test]
    fn vm_if_return_done_branch() {
        let p = compile_source(
            r#"
            fn stepNext(it) {
              if (it.n < 3) {
                it.n = it.n + 1
                return { value: it.n, done: false }
              }
              return { value: null, done: true }
            }
            stepNext({ n: 3 })
            "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).unwrap();
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).unwrap();
        assert!(matches!(v, Value::Object(_)), "got {v:?}");
    }

    #[test]
    fn vm_logical_or_short_circuit() {
        use crate::bytecode::run_module;
        let p = compile_source(
            r#"
            false || true
            "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).unwrap();
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).unwrap();
        assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    }

    #[test]
    fn vm_return_undefined_object() {
        let p = compile_source(
            r#"
            fn f() {
              return { value: null, done: true }
            }
            f()
            "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).unwrap();
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).unwrap();
        assert!(matches!(v, Value::Object(_)), "got {v:?}");
    }

    #[test]
    fn rejects_direct_array_self_cycle() {
        use std::rc::Rc;
        let arr = Value::from_array(vec![Value::Number(1)]);
        let Value::Array(rc) = &arr else {
            panic!("expected array");
        };
        let same = Value::Array(Rc::clone(rc));
        let err = Value::reject_direct_container_cycle(&arr, &same).unwrap_err();
        assert!(err.contains("cycle"), "{err}");
        // Distinct allocation is fine.
        let other = Value::from_array(vec![Value::Number(2)]);
        Value::reject_direct_container_cycle(&arr, &other).unwrap();
    }
}
