//! ECMAScript `WeakRef` and `FinalizationRegistry` (best-effort reachability sweep).

use crate::ops::values_identical;
use crate::runtime::stdlib::map::{is_map_value, is_set_value};
use crate::runtime::stdlib::object::{object_oid, object_oid_of};
use crate::runtime::stdlib::proxy;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

const WEAKREF_MARKER: &str = "__kab_weakref";
const WEAKREF_OID: &str = "__kab_weak_target_oid";
const FINREG_MARKER: &str = "__kab_finreg";
const FINREG_ID: &str = "__kab_finreg_id";
const WEAKREF_CTOR: &str = "__kab_weakref_ctor";
const FINREG_CTOR: &str = "__kab_finreg_ctor";

static NEXT_FINREG_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REG_TOKEN: AtomicU64 = AtomicU64::new(1);

/// P3: heap allocs since last `gc_frame_begin` (MakeObject / MakeArray / …).
static FRAME_ALLOCS: AtomicU64 = AtomicU64::new(0);
/// Soft budget: when exceeded on frame tick, run a GC sweep.
static FRAME_ALLOC_BUDGET: AtomicU64 = AtomicU64::new(2_048);
static FRAME_SWEEPS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct Registration {
    registry_id: u64,
    target_oid: u64,
    held_value: Value,
    /// When true, `unregister(target)` matches by target oid (no strong ref kept).
    token_is_target: bool,
    unregister_token: Option<Value>,
}

#[derive(Default)]
struct WeakState {
    tracked_oids: HashSet<u64>,
    collected_oids: HashSet<u64>,
    finregs: HashMap<u64, Value>,
    registrations: HashMap<u64, Registration>,
    regs_by_target: HashMap<u64, Vec<u64>>,
    /// Last held value passed to a finalization callback (test/diagnostics).
    last_finalized: Option<Value>,
}

thread_local! {
    static WEAK_STATE: RefCell<WeakState> = RefCell::new(WeakState::default());
}

fn with_state<F, T>(f: F) -> T
where
    F: FnOnce(&mut WeakState) -> T,
{
    WEAK_STATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn is_weakref_ctor_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(WEAKREF_CTOR), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_finreg_ctor_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(FINREG_CTOR), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_weakref_value(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(WEAKREF_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_finreg_value(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(FINREG_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn value_oid(v: &Value) -> Option<u64> {
    match v {
        Value::Object(map) => object_oid_of(map),
        _ => None,
    }
}

fn is_weakref_target(v: &Value) -> bool {
    match v {
        Value::Object(_) => {
            !is_map_value(v)
                && !is_set_value(v)
                && !proxy::is_proxy(v)
                && !is_weakref_value(v)
                && !is_finreg_value(v)
                && !crate::runtime::stdlib::symbol::is_symbol_ctor_object(v)
                && !proxy::is_proxy_ctor_object(v)
        }
        _ => false,
    }
}

fn weakref_target_oid(v: &Value) -> Option<u64> {
    let Value::Object(m) = v else {
        return None;
    };
    match m.get(WEAKREF_OID) {
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    }
}

fn finreg_id(v: &Value) -> Option<u64> {
    let Value::Object(m) = v else {
        return None;
    };
    match m.get(FINREG_ID) {
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    }
}

fn track_oid(oid: u64) {
    with_state(|s| {
        s.tracked_oids.insert(oid);
        s.collected_oids.remove(&oid);
    });
}

pub fn create_weakref(target: Value) -> Result<Value, String> {
    if !is_weakref_target(&target) {
        return Err("WeakRef target must be a plain object".into());
    }
    let mut target = target;
    let Value::Object(map) = &mut target else {
        return Err("WeakRef target must be a plain object".into());
    };
    let oid = object_oid(Value::object_make_mut(map));
    track_oid(oid);
    let mut m = HashMap::new();
    m.insert(WEAKREF_MARKER.into(), Value::Bool(true));
    m.insert(WEAKREF_OID.into(), Value::Number(oid as i64));
    let wr = Value::from_object(m.clone());
    m.insert(
        "deref".into(),
        Value::BoundNative(Box::new(wr), weakref_deref_native),
    );
    Ok(Value::from_object(m))
}

fn weakref_deref_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let wr = args.first().ok_or("WeakRef.deref()")?;
    let Some(oid) = weakref_target_oid(wr) else {
        return Err("WeakRef.deref() expects WeakRef receiver".into());
    };
    if with_state(|s| s.collected_oids.contains(&oid)) {
        return Ok(Value::Undefined);
    }
    if let Some(found) = find_value_by_oid(env, oid) {
        return Ok(found);
    }
    Ok(Value::Undefined)
}

pub fn create_finalization_registry(cleanup: Value) -> Result<Value, String> {
    if !is_callable(&cleanup) {
        return Err("FinalizationRegistry cleanup must be callable".into());
    }
    let id = NEXT_FINREG_ID.fetch_add(1, Ordering::Relaxed);
    with_state(|s| {
        s.finregs.insert(id, cleanup);
    });
    let mut m = HashMap::new();
    m.insert(FINREG_MARKER.into(), Value::Bool(true));
    m.insert(FINREG_ID.into(), Value::Number(id as i64));
    let fr = Value::from_object(m.clone());
    m.insert(
        "register".into(),
        Value::BoundNative(Box::new(fr.clone()), finreg_register_native),
    );
    m.insert(
        "unregister".into(),
        Value::BoundNative(Box::new(fr), finreg_unregister_native),
    );
    Ok(Value::from_object(m))
}

fn finreg_register_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let registry = args.first().ok_or("FinalizationRegistry.register(...)")?;
    let target = args.get(1).ok_or("FinalizationRegistry.register(...)")?;
    let held = args
        .get(2)
        .cloned()
        .unwrap_or(Value::Undefined);
    let token_is_target = args.get(3).is_none();
    let unregister_token = args.get(3).cloned();
    let registry_id = finreg_id(registry).ok_or("register() requires FinalizationRegistry receiver")?;
    if !is_weakref_target(target) {
        return Err("FinalizationRegistry.register target must be a plain object".into());
    }
    let mut target = target.clone();
    let Value::Object(map) = &mut target else {
        return Err("FinalizationRegistry.register target must be a plain object".into());
    };
    let target_oid = object_oid(Value::object_make_mut(map));
    track_oid(target_oid);
    let token_id = NEXT_REG_TOKEN.fetch_add(1, Ordering::Relaxed);
    with_state(|s| {
        s.registrations.insert(
            token_id,
            Registration {
                registry_id,
                target_oid,
                held_value: held,
                token_is_target,
                unregister_token,
            },
        );
        s.regs_by_target
            .entry(target_oid)
            .or_default()
            .push(token_id);
    });
    Ok(Value::Undefined)
}

fn finreg_unregister_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let registry = args.first().ok_or("FinalizationRegistry.unregister(token)")?;
    let token = args.get(1).ok_or("FinalizationRegistry.unregister(token)")?;
    let registry_id = finreg_id(registry).ok_or("unregister() requires FinalizationRegistry receiver")?;
    let removed = with_state(|s| {
        let mut removed = false;
        let mut drop_tokens = Vec::new();
        for (token_id, reg) in &s.registrations {
            if reg.registry_id == registry_id && registration_matches_token(reg, token) {
                drop_tokens.push(*token_id);
            }
        }
        for token_id in drop_tokens {
            if let Some(reg) = s.registrations.remove(&token_id) {
                removed = true;
                if let Some(list) = s.regs_by_target.get_mut(&reg.target_oid) {
                    list.retain(|id| *id != token_id);
                }
            }
        }
        removed
    });
    Ok(Value::Bool(removed))
}

fn registration_matches_token(reg: &Registration, token: &Value) -> bool {
    if reg.token_is_target {
        return value_oid(token) == Some(reg.target_oid);
    }
    reg.unregister_token
        .as_ref()
        .is_some_and(|stored| values_identical(stored, token))
}

fn is_callable(v: &Value) -> bool {
    crate::runtime::stdlib::descriptor::is_callable_value(v)
}

fn collect_roots(env: &Environment) -> Vec<Value> {
    let mut roots = Vec::new();
    for name in env.all_binding_names() {
        if let Some(v) = env.get(&name) {
            roots.push(v);
        }
    }
    roots.extend(env.gc_scheduler_roots());
    roots
}

fn find_value_by_oid(env: &Environment, oid: u64) -> Option<Value> {
    let mut stack = collect_roots(env);
    let mut visited_oids = HashSet::new();
    while let Some(v) = stack.pop() {
        if let Some(found_oid) = value_oid(&v) {
            if found_oid == oid {
                return Some(v);
            }
            if !visited_oids.insert(found_oid) {
                continue;
            }
        }
        push_children(&v, &mut stack);
    }
    None
}

pub fn is_oid_reachable(env: &Environment, oid: u64) -> bool {
    find_value_by_oid(env, oid).is_some()
}

fn push_children(v: &Value, stack: &mut Vec<Value>) {
    match v {
        Value::Array(items) => stack.extend(items.iter().cloned()), Value::Object(map) => {
            for (k, val) in map.iter() {
                if k == proxy::PROXY_TARGET
                    || k == crate::runtime::stdlib::object::OBJECT_PARENT_KEY
                    || k == "__kab_proto"
                    || !k.starts_with("__kab_")
                {
                    stack.push(val.clone());
                }
            }
        }
        Value::Function { env: closure, .. } => {
            for name in closure.all_binding_names() {
                if let Some(v) = closure.get(&name) {
                    stack.push(v);
                }
            }
        }
        Value::BytecodeFn(f) => {
            for name in &f.def.globals {
                if let Some(v) = f.closure.get(name) {
                    stack.push(v.clone());
                }
            }
        }
        Value::ClassInstance(inst) => {
            if let Ok(i) = inst.try_borrow() {
                stack.extend(i.fields.values().cloned());
            }
        }
        Value::BoundMethod(inst, _) => stack.push(Value::ClassInstance(inst.clone())),
        Value::BoundNative(receiver, _) => stack.push((**receiver).clone()),
        Value::Option(Some(inner)) => stack.push((**inner).clone()),
        Value::Result(Ok(inner)) => stack.push((**inner).clone()),
        Value::Result(Err(inner)) => stack.push((**inner).clone()),
        _ => {}
    }
}

fn collect_reachable_oids(env: &Environment) -> HashSet<u64> {
    let mut oids = HashSet::new();
    let mut stack = collect_roots(env);
    let mut visited_oids = HashSet::new();
    while let Some(v) = stack.pop() {
        if let Some(oid) = value_oid(&v) {
            if !visited_oids.insert(oid) {
                continue;
            }
            oids.insert(oid);
        }
        push_children(&v, &mut stack);
    }
    oids
}

pub fn run_gc_sweep(env: &mut Environment) -> Result<(), String> {
    let newly_dead: Vec<u64> = with_state(|s| {
        if s.tracked_oids.is_empty() {
            return Vec::new();
        }
        let reachable = collect_reachable_oids(env);
        let mut dead = Vec::new();
        for oid in s.tracked_oids.clone() {
            if !reachable.contains(&oid) && !s.collected_oids.contains(&oid) {
                s.collected_oids.insert(oid);
                dead.push(oid);
            }
        }
        dead
    });

    if newly_dead.is_empty() {
        return Ok(());
    }

    FRAME_SWEEPS.fetch_add(1, Ordering::Relaxed);

    let callbacks: Vec<(Value, Value)> = with_state(|s| {
        let mut out = Vec::new();
        for oid in &newly_dead {
            let Some(token_ids) = s.regs_by_target.remove(oid) else {
                continue;
            };
            for token_id in token_ids {
                let Some(reg) = s.registrations.remove(&token_id) else {
                    continue;
                };
                if let Some(cleanup) = s.finregs.get(&reg.registry_id).cloned() {
                    out.push((cleanup, reg.held_value));
                }
            }
        }
        out
    });

    for (cleanup, held) in callbacks {
        invoke_finreg_cleanup(&cleanup, held, env)?;
    }
    let _ = newly_dead;
    Ok(())
}

/// Count a heap allocation toward the current frame budget (P3).
pub fn note_heap_alloc(n: u64) {
    FRAME_ALLOCS.fetch_add(n, Ordering::Relaxed);
}

/// Reset per-frame alloc counter (call at start of `game_tick`).
pub fn gc_frame_begin() {
    FRAME_ALLOCS.store(0, Ordering::Relaxed);
}

/// If allocs since `gc_frame_begin` exceed soft budget, run a sweep.
pub fn gc_frame_maybe_sweep(env: &mut Environment) -> Result<bool, String> {
    let allocs = FRAME_ALLOCS.load(Ordering::Relaxed);
    let budget = FRAME_ALLOC_BUDGET.load(Ordering::Relaxed);
    if budget > 0 && allocs >= budget {
        run_gc_sweep(env)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn set_frame_alloc_budget(budget: u64) {
    FRAME_ALLOC_BUDGET.store(budget, Ordering::Relaxed);
}

/// `{ allocs, budget, sweeps, over_budget }` for the current/last frame window.
pub fn gc_frame_stats_value() -> Value {
    let allocs = FRAME_ALLOCS.load(Ordering::Relaxed);
    let budget = FRAME_ALLOC_BUDGET.load(Ordering::Relaxed);
    let sweeps = FRAME_SWEEPS.load(Ordering::Relaxed);
    let mut m = HashMap::new();
    m.insert("allocs".into(), Value::Number(allocs as i64));
    m.insert("budget".into(), Value::Number(budget as i64));
    m.insert("sweeps".into(), Value::Number(sweeps as i64));
    m.insert(
        "over_budget".into(),
        Value::Bool(budget > 0 && allocs >= budget),
    );
    Value::from_object(m)
}

pub fn gc_frame_reset_for_tests() {
    FRAME_ALLOCS.store(0, Ordering::Relaxed);
    FRAME_SWEEPS.store(0, Ordering::Relaxed);
    FRAME_ALLOC_BUDGET.store(2_048, Ordering::Relaxed);
}

fn finreg_cleanup_microtask_native(
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    let cleanup = args
        .first()
        .ok_or("internal FinalizationRegistry microtask")?;
    let held = args.get(1).cloned().unwrap_or(Value::Undefined);
    with_state(|s| s.last_finalized = Some(held.clone()));
    invoke_finreg_cleanup(cleanup, held, env)?;
    Ok(Value::Undefined)
}

fn sync_finreg_cleanup_writes(
    closure: &Environment,
    call_env: &Environment,
    root: &mut Environment,
) {
    crate::runtime::closure_sync::sync_closure_writes(closure, call_env, root);
}

fn invoke_finreg_cleanup(
    cleanup: &Value,
    held: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    match cleanup {
        Value::BytecodeFn(f) => {
            let mut f = f.clone();
            crate::runtime::closure_sync::pull_bytecode_globals(&mut f, env);
            crate::runtime::closure_sync::pull_root_into_closure(&mut f.closure, env);
            let result = crate::bytecode::run_bytecode_fn(f.def.as_ref(), vec![held], env)?;
            Ok(result)
        }
        Value::Function {
            params,
            defaults,
            rest,
            body,
            env: closure_env,
            ..
        } => {
            let mut call_env = Environment::child(closure_env.clone());
            crate::evaluator::bind_call_params(params, defaults, rest, &[held], &mut call_env)?;
            let result = crate::evaluator::eval_expr(body, &mut call_env)?;
            sync_finreg_cleanup_writes(closure_env, &call_env, env);
            Ok(result)
        }
        _ => crate::bytecode::call_value(
            cleanup.clone(),
            vec![held],
            &[],
            &[],
            &[],
            &[],
            env,
        ),
    }
}

fn weakref_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("WeakRef(target)")?;
    create_weakref(target.clone())
}

fn finreg_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let cleanup = args.first().ok_or("FinalizationRegistry(cleanup)")?;
    create_finalization_registry(cleanup.clone())
}

pub fn try_weakref_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_weakref_ctor_object(callee) {
        Some(weakref_ctor_native(args, env))
    } else {
        None
    }
}

pub fn try_finreg_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_finreg_ctor_object(callee) {
        Some(finreg_ctor_native(args, env))
    } else {
        None
    }
}

fn build_weakref_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert(WEAKREF_CTOR.into(), Value::Bool(true));
    Value::from_object(m)
}

fn build_finreg_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert(FINREG_CTOR.into(), Value::Bool(true));
    Value::from_object(m)
}

pub fn register_weak(env: &mut Environment) {
    env.set("WeakRef".to_string(), build_weakref_namespace());
    env.set(
        "FinalizationRegistry".to_string(),
        build_finreg_namespace(),
    );
    env.set(
        "gc_frame_stats".to_string(),
        Value::NativeFunction(|_args, _env| Ok(gc_frame_stats_value())),
    );
    env.set(
        "gc_set_frame_budget".to_string(),
        Value::NativeFunction(|args, _env| {
            let n = match args.first() {
                Some(Value::Number(n)) if *n >= 0 => *n as u64,
                Some(Value::Float(f)) if *f >= 0.0 => *f as u64,
                _ => return Err("gc_set_frame_budget(n) expects non-negative number".into()),
            };
            set_frame_alloc_budget(n);
            Ok(Value::Null)
        }),
    );
}

/// Test/diagnostic: last `held` value delivered to a finalization callback.
pub fn last_finalized_held() -> Option<Value> {
    with_state(|s| s.last_finalized.clone())
}
