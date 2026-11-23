//! P13 — JIT for typed i64 bytecode (host only).
//!
//! The compiler is Kab: hot fns lower through `lib/kab/jit/jit_x64_emit.kab`
//! (`jitX64Emit`) to real x86-64 produced by `lib/kab/aot/aot_fn_x64.kab`,
//! and run on `runtime::native_exec` executable pages. The retired Cranelift
//! `compile_fn` this file used to carry is gone — `JIT_KAB_EXECS` counts the
//! executions that went through Kab-emitted code.

use super::types::{BytecodeFnDef, Constant, Opcode};
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static JIT_HITS: AtomicU64 = AtomicU64::new(0);
static JIT_COMPILED: AtomicU64 = AtomicU64::new(0);
static JIT_FAILS: AtomicU64 = AtomicU64::new(0);
static JIT_KAB_EXECS: AtomicU64 = AtomicU64::new(0);

/// P13b: JIT after this many calls of a given bytecode fn (same as Kv8 loop threshold).
pub const JIT_CALL_THRESHOLD_DEFAULT: u64 = 8;
static JIT_CALL_THRESHOLD: AtomicU64 = AtomicU64::new(JIT_CALL_THRESHOLD_DEFAULT);

pub fn jit_call_threshold() -> u64 {
    JIT_CALL_THRESHOLD.load(Ordering::Relaxed)
}

pub fn jit_set_call_threshold_for_tests(n: u64) {
    JIT_CALL_THRESHOLD.store(n.max(1), Ordering::Relaxed);
}

pub fn jit_stats() -> (u64, u64, u64) {
    (
        JIT_HITS.load(Ordering::Relaxed),
        JIT_COMPILED.load(Ordering::Relaxed),
        JIT_FAILS.load(Ordering::Relaxed),
    )
}

/// Executions that ran Kab-emitted native code — the `nollJitIsKab`
/// evidence counter.
pub fn jit_kab_execs() -> u64 {
    JIT_KAB_EXECS.load(Ordering::Relaxed)
}

pub fn jit_reset_for_tests() {
    JIT_HITS.store(0, Ordering::Relaxed);
    JIT_COMPILED.store(0, Ordering::Relaxed);
    JIT_FAILS.store(0, Ordering::Relaxed);
    JIT_KAB_EXECS.store(0, Ordering::Relaxed);
    JIT_CALL_THRESHOLD.store(JIT_CALL_THRESHOLD_DEFAULT, Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    host::reset_call_counts();
}

pub fn try_run_jit(
    func: &BytecodeFnDef,
    args: &[Value],
) -> Option<Result<(Value, Vec<Value>), String>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (func, args);
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        host::try_run(func, args)
    }
}

/// Standalone add-loop used by `native_add_loop` (emitted once via Kab).
pub fn jit_add_loop(n: i64) -> Option<i64> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = n;
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        host::add_loop(n)
    }
}

/// SH9: `len`/`xs[i]` i64-loop over one array param (no boxed Load of the array).
pub fn fn_is_index_i64_loop(func: &BytecodeFnDef) -> bool {
    index_array_local(func).is_some()
}

fn index_array_local(func: &BytecodeFnDef) -> Option<u16> {
    if func.params.is_empty() || func.params.len() > 2 || func.async_fn || func.generator_fn {
        return None;
    }
    if !func.try_regions.is_empty() || !func.arrow_functions.is_empty() {
        return None;
    }
    if func.local_captures.iter().any(|c| *c) || func.immutable_locals.iter().any(|c| *c) {
        return None;
    }
    let mut arr: Option<u16> = None;
    for op in &func.code {
        match op {
            Opcode::LenLocal(i) | Opcode::IndexGetLocal(i) => {
                if let Some(a) = arr {
                    if a != *i {
                        return None;
                    }
                } else {
                    arr = Some(*i);
                }
            }
            Opcode::Const(i) => match func.constants.get(*i as usize) {
                Some(
                    Constant::Number(_)
                    | Constant::Bool(_)
                    | Constant::Null
                    | Constant::Float(_)
                    | Constant::String(_),
                ) => {}
                _ => return None,
            },
            Opcode::LoadLocal(_)
            | Opcode::StoreLocal(_)
            | Opcode::AccAddLocal(_)
            | Opcode::Pop
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::Jump(_)
            | Opcode::JumpIfFalse(_)
            | Opcode::Return
            | Opcode::Halt => {}
            _ => return None,
        }
    }
    let arr = arr?;
    let pname = func.params.first()?;
    if func.locals.get(arr as usize) != Some(pname) {
        return None;
    }
    for op in &func.code {
        match op {
            Opcode::LoadLocal(i) | Opcode::StoreLocal(i) | Opcode::AccAddLocal(i) if *i == arr => {
                return None;
            }
            _ => {}
        }
    }
    Some(arr)
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::{BytecodeFnDef, Constant, Opcode, JIT_COMPILED, JIT_FAILS, JIT_HITS, JIT_KAB_EXECS};
    use crate::value::{Environment, Value as KabVal};
    use std::cell::{Cell, RefCell};
    use std::collections::{HashMap, HashSet};
    use std::hash::{Hash, Hasher};
    use std::rc::Rc;
    use std::sync::atomic::Ordering;

    struct State {
        /// fingerprint -> emitted x64 bytes (native_exec maps them RX).
        cache: HashMap<u64, Rc<Vec<u8>>>,
        /// fingerprints the Kab emitter honestly rejected — deterministic,
        /// so retrying per call would be pure waste.
        rejects: HashSet<u64>,
        str_char_kernels: HashSet<u64>,
        call_counts: HashMap<u64, u64>,
        add_loop: Option<Rc<Vec<u8>>>,
        /// (jitX64Emit fn value, module env it was imported into).
        emit: Option<(KabVal, Environment)>,
        emit_broken: bool,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
        /// Reentrancy guard: the emit call runs the bytecode VM, which can
        /// itself reach try_run for the emitter's own hot fns — a nested
        /// emit must fall back to the interpreter, not recurse.
        static EMIT_ACTIVE: Cell<bool> = const { Cell::new(false) };
    }

    fn emit_active() -> bool {
        EMIT_ACTIVE.with(|c| c.get())
    }

    struct EmitGuard;
    impl Drop for EmitGuard {
        fn drop(&mut self) {
            EMIT_ACTIVE.with(|c| c.set(false));
        }
    }

    fn emit_begin() -> Option<EmitGuard> {
        if emit_active() {
            return None;
        }
        EMIT_ACTIVE.with(|c| c.set(true));
        Some(EmitGuard)
    }

    fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
        STATE.with(|slot| {
            if slot.borrow().is_none() {
                *slot.borrow_mut() = Some(State {
                    cache: HashMap::new(),
                    rejects: HashSet::new(),
                    str_char_kernels: HashSet::new(),
                    call_counts: HashMap::new(),
                    add_loop: None,
                    emit: None,
                    emit_broken: false,
                });
            }
            Some(f(slot.borrow_mut().as_mut().unwrap()))
        })
    }

    pub fn reset_call_counts() {
        STATE.with(|slot| {
            if let Some(st) = slot.borrow_mut().as_mut() {
                st.call_counts.clear();
            }
        });
    }

    fn hot_enough(st: &mut State, key: u64) -> bool {
        if st.cache.contains_key(&key) || st.str_char_kernels.contains(&key) {
            return true;
        }
        let n = st.call_counts.entry(key).or_insert(0);
        *n += 1;
        *n >= super::JIT_CALL_THRESHOLD.load(Ordering::Relaxed)
    }

    /// Lazily import `kab/jit/jit_x64_emit` and hand back its `jitX64Emit`
    /// export with the env it was imported into. Runs OUTSIDE the STATE
    /// borrow — module import itself runs bytecode and reenters try_run.
    fn emit_handle() -> Option<(KabVal, Environment)> {
        if emit_active() {
            return None;
        }
        let (broken, cached) = with_state(|st| (st.emit_broken, st.emit.clone()))?;
        if broken {
            return None;
        }
        if let Some(h) = cached {
            return Some(h);
        }
        let _g = emit_begin()?;
        let mut env = crate::evaluator::create_module_env();
        let imported = crate::modules::import_module_exported("kab/jit/jit_x64_emit", &mut env);
        let f = if imported.is_ok() {
            env.get("jitX64Emit")
        } else {
            None
        };
        match f {
            Some(v) => {
                let h = (v, env);
                with_state(|st| {
                    if st.emit.is_none() {
                        st.emit = Some(h.clone());
                    }
                    st.emit.clone()
                })?
            }
            None => {
                with_state(|st| {
                    st.emit_broken = true;
                });
                None
            }
        }
    }

    fn rec_str_array(xs: &[String]) -> KabVal {
        KabVal::from_array(
            xs.iter()
                .map(|s| KabVal::String(s.clone()))
                .collect(),
        )
    }

    /// BytecodeFnDef -> the compileIr-shaped record `jitX64Emit` consumes.
    /// None when an op sits outside the emitter's domain.
    fn fn_record(func: &BytecodeFnDef, kind: &str, arr_slot: Option<u16>) -> Option<KabVal> {
        let mut ops = Vec::with_capacity(func.code.len());
        for op in &func.code {
            let (name, arg): (&'static str, i64) = match op {
                Opcode::Const(i) => ("const", *i as i64),
                Opcode::LoadLocal(i) => ("load_local", *i as i64),
                Opcode::StoreLocal(i) => ("store_local", *i as i64),
                Opcode::AccAddLocal(i) => ("acc_add_local", *i as i64),
                Opcode::LenLocal(i) => ("len_local", *i as i64),
                Opcode::IndexGetLocal(i) => ("index_get_local", *i as i64),
                Opcode::Pop => ("pop", 0),
                Opcode::Dup => ("dup", 0),
                Opcode::Add => ("add", 0),
                Opcode::Sub => ("sub", 0),
                Opcode::Mul => ("mul", 0),
                Opcode::Div => ("div", 0),
                Opcode::Mod => ("mod", 0),
                Opcode::Neg => ("neg", 0),
                Opcode::Not => ("not", 0),
                Opcode::BitNot => ("bit_not", 0),
                Opcode::Eq => ("eq", 0),
                Opcode::Ne => ("ne", 0),
                Opcode::Lt => ("lt", 0),
                Opcode::Le => ("le", 0),
                Opcode::Gt => ("gt", 0),
                Opcode::Ge => ("ge", 0),
                Opcode::Jump(o) => ("jump", *o as i64),
                Opcode::JumpIfFalse(o) => ("jump_if_false", *o as i64),
                Opcode::Return => ("return", 0),
                Opcode::Halt => ("halt", 0),
                _ => return None,
            };
            let mut m = std::collections::HashMap::new();
            m.insert("op".to_string(), KabVal::String(name.to_string()));
            m.insert("arg".to_string(), KabVal::Number(arg));
            ops.push(KabVal::from_object(m));
        }
        // i64 view of the pool: Float truncates and Null/Undefined map to 0
        // (const_i64 parity — i64_const_domain already proved referenced
        // nulls only feed `const;pop` discard slots); String passes through
        // so a referenced string const makes the Kab side reject instead of
        // silently emitting Cranelift's 0.
        let consts: Vec<KabVal> = func
            .constants
            .iter()
            .map(|c| match c {
                Constant::Number(n) => KabVal::Number(*n),
                Constant::Float(f) => KabVal::Number(*f as i64),
                Constant::Bool(b) => KabVal::Number(*b as i64),
                Constant::String(s) => KabVal::String(s.clone()),
                _ => KabVal::Number(0),
            })
            .collect();
        let mut m = std::collections::HashMap::new();
        m.insert("ops".to_string(), KabVal::from_array(ops));
        m.insert("constants".to_string(), KabVal::from_array(consts));
        m.insert("locals".to_string(), rec_str_array(&func.locals));
        m.insert("params".to_string(), rec_str_array(&func.params));
        m.insert("kind".to_string(), KabVal::String(kind.to_string()));
        if let Some(s) = arr_slot {
            m.insert("arrSlot".to_string(), KabVal::Number(s as i64));
        }
        Some(KabVal::from_object(m))
    }

    /// Extract the emitted byte array out of a `jitX64Emit` result record.
    fn emit_code(out: KabVal) -> Option<Vec<u8>> {
        let KabVal::Object(m) = out else {
            return None;
        };
        if !matches!(m.get("ok"), Some(KabVal::Bool(true))) {
            return None;
        }
        let Some(KabVal::Array(items)) = m.get("code") else {
            return None;
        };
        let mut code = Vec::with_capacity(items.len());
        for v in items.iter() {
            match v {
                KabVal::Number(n) if (0..=255).contains(n) => code.push(*n as u8),
                _ => return None,
            }
        }
        if code.is_empty() {
            return None;
        }
        Some(code)
    }

    /// Run a prepared record through the Kab emitter; None = honest reject
    /// (unsupported shape) or emitter unavailable → caller falls back to
    /// the interpreter, never errors the user's fn.
    fn kab_emit_rec(rec: KabVal) -> Option<Rc<Vec<u8>>> {
        let (emit, mut env) = emit_handle()?;
        let _g = emit_begin()?;
        let out = super::super::call_value(emit, vec![rec], &[], &[], &[], &[], &mut env);
        emit_code(out.ok()?).map(Rc::new)
    }

    fn kab_compile(func: &BytecodeFnDef, kind: &str, arr_slot: Option<u16>) -> Option<Rc<Vec<u8>>> {
        if emit_active() {
            return None;
        }
        kab_emit_rec(fn_record(func, kind, arr_slot)?)
    }

    /// Cache lookup under a short borrow, then (if cold) the emit itself
    /// OUTSIDE the borrow — call_value reenters the VM and would panic on
    /// the still-held RefCell otherwise. Rejects are negative-cached.
    fn cached_or_compile(
        func: &BytecodeFnDef,
        key: u64,
        kind: &str,
        arr_slot: Option<u16>,
    ) -> Option<Rc<Vec<u8>>> {
        if let Some(c) = with_state(|st| st.cache.get(&key).cloned())? {
            return Some(c);
        }
        if with_state(|st| st.rejects.contains(&key))? {
            return None;
        }
        let code = match kab_compile(func, kind, arr_slot) {
            Some(c) => c,
            None => {
                with_state(|st| {
                    st.rejects.insert(key);
                });
                return None;
            }
        };
        JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
        with_state(|st| st.cache.entry(key).or_insert_with(|| code.clone()).clone())
    }

    fn exec(code: &[u8], arg: i64, data: &[u8]) -> Result<i64, String> {
        let r = crate::runtime::native_exec::exec_bytes_data(code, arg, data)?;
        JIT_KAB_EXECS.fetch_add(1, Ordering::Relaxed);
        Ok(r)
    }

    /// `fn al(n): s=0; i=0; while i<n { s+=1; i+=1 } return s` — the same
    /// loop the old compile_add_loop built, as a compileIr record.
    fn add_loop_rec() -> KabVal {
        let op = |name: &str, arg: i64| {
            let mut m = std::collections::HashMap::new();
            m.insert("op".to_string(), KabVal::String(name.to_string()));
            m.insert("arg".to_string(), KabVal::Number(arg));
            KabVal::from_object(m)
        };
        let ops = KabVal::from_array(vec![
            op("const", 0),
            op("store_local", 1),
            op("const", 0),
            op("store_local", 2),
            op("load_local", 2),
            op("load_local", 0),
            op("lt", 0),
            op("jump_if_false", 7),
            op("const", 1),
            op("acc_add_local", 1),
            op("load_local", 2),
            op("const", 1),
            op("add", 0),
            op("store_local", 2),
            op("jump", -11),
            op("load_local", 1),
            op("return", 0),
        ]);
        let mut m = std::collections::HashMap::new();
        m.insert("ops".to_string(), ops);
        m.insert(
            "constants".to_string(),
            KabVal::from_array(vec![KabVal::Number(0), KabVal::Number(1)]),
        );
        m.insert(
            "locals".to_string(),
            rec_str_array(&["n".into(), "s".into(), "i".into()]),
        );
        m.insert("params".to_string(), rec_str_array(&["n".into()]));
        m.insert("kind".to_string(), KabVal::String("i64".into()));
        KabVal::from_object(m)
    }

    pub fn add_loop(n: i64) -> Option<i64> {
        let code = match with_state(|st| st.add_loop.clone())? {
            Some(c) => c,
            None => {
                let c = kab_emit_rec(add_loop_rec())?;
                let (stored, inserted) = with_state(|st| {
                    if st.add_loop.is_none() {
                        st.add_loop = Some(c.clone());
                        (c.clone(), true)
                    } else {
                        (st.add_loop.clone().unwrap_or(c.clone()), false)
                    }
                })?;
                if inserted {
                    JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
                }
                stored
            }
        };
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        exec(&code, n, &[]).ok()
    }

    pub fn try_run(
        func: &BytecodeFnDef,
        args: &[KabVal],
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        if func.locals.len() > 32 {
            return None;
        }
        if func.code.iter().any(|op| {
            matches!(
                op,
                Opcode::Div | Opcode::Mod | Opcode::Dup | Opcode::Not | Opcode::Neg
            )
        }) {
            return None;
        }
        if let Some(arr_li) = super::index_array_local(func) {
            return try_run_index(func, args, arr_li);
        }
        if func.params.len() > 1 {
            return None;
        }
        if !returns_i64(func) || !i64_const_domain(func) {
            return None;
        }
        let key = fingerprint(func);
        let ready = with_state(|st| hot_enough(st, key))?;
        if !ready {
            return None;
        }
        let code = cached_or_compile(func, key, "i64", None)?;
        let narg = match args.first() {
            Some(KabVal::Number(n)) => *n,
            Some(KabVal::Float(f)) => *f as i64,
            _ => 0,
        };
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        let ret = match exec(&code, narg, &[]) {
            Ok(r) => r,
            Err(e) => {
                JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                return Some(Err(e));
            }
        };
        Some(Ok((KabVal::Number(ret), Vec::new())))
    }

    fn flatten_i64_array(v: &KabVal) -> Option<Vec<i64>> {
        match v {
            KabVal::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for x in items.iter() {
                    match x {
                        KabVal::Number(n) => out.push(*n),
                        KabVal::Float(f) => out.push(*f as i64),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// The array kernels only receive `(len, base)` / `(len)` — extra params
    /// cannot be passed through. If the body reads a second param's local slot
    /// the JIT would run with it bound to 0; bail to the boxed VM instead.
    fn extra_param_unread(func: &BytecodeFnDef) -> bool {
        let Some(pname) = func.params.get(1) else {
            return true;
        };
        let Some(idx) = func.locals.iter().position(|l| l == pname) else {
            return true;
        };
        !func.code.iter().any(|op| {
            matches!(
                op,
                Opcode::LoadLocal(i)
                | Opcode::AccAddLocal(i)
                | Opcode::LenLocal(i)
                | Opcode::IndexGetLocal(i)
                    if *i as usize == idx
            )
        })
    }

    fn try_run_index(
        func: &BytecodeFnDef,
        args: &[KabVal],
        arr_li: u16,
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        let wants_index = func
            .code
            .iter()
            .any(|op| matches!(op, Opcode::IndexGetLocal(_)));
        let key = fingerprint(func);
        let ready = with_state(|st| hot_enough(st, key))?;
        if !ready {
            return None;
        }
        if wants_index {
            if let Some(buf) = flatten_i64_array(args.first()?) {
                if !extra_param_unread(func) || !returns_i64(func) || !i64_const_domain(func) {
                    return None;
                }
                let code = cached_or_compile(func, key, "index", Some(arr_li))?;
                JIT_HITS.fetch_add(1, Ordering::Relaxed);
                // arg1 = len (ABI first arg); arg2 = i64 LE data page base.
                let mut data = Vec::with_capacity(buf.len() * 8);
                for v in &buf {
                    data.extend_from_slice(&v.to_le_bytes());
                }
                let ret = match exec(&code, buf.len() as i64, &data) {
                    Ok(r) => r,
                    Err(e) => {
                        JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                        return Some(Err(e));
                    }
                };
                return Some(Ok((KabVal::Number(ret), Vec::new())));
            }
            return try_run_str_char_index(func, args, key);
        }
        if !extra_param_unread(func) || !returns_i64(func) || !i64_const_domain(func) {
            return None;
        }
        let len = crate::value::container_len(args.first()?).ok()?;
        let code = cached_or_compile(func, key, "len", Some(arr_li))?;
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        let ret = match exec(&code, len, &[]) {
            Ok(r) => r,
            Err(e) => {
                JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                return Some(Err(e));
            }
        };
        Some(Ok((KabVal::Number(ret), Vec::new())))
    }

    /// i64-domain gate for the Number-wrapping return path: the value
    /// reaching each Return/Halt must be produced by an i64-typed op —
    /// a numeric Const, an arith op, or a local slot. Const(Bool/Null/
    /// String) or a cmp result would be wrapped as `Number(ret)` and
    /// silently coerce the callee's return type (`return false` → 0).
    fn returns_i64(func: &BytecodeFnDef) -> bool {
        for (i, op) in func.code.iter().enumerate() {
            if matches!(op, Opcode::Return | Opcode::Halt) {
                match i.checked_sub(1).and_then(|p| func.code.get(p)) {
                    Some(Opcode::Const(ci)) => match func.constants.get(*ci as usize) {
                        Some(Constant::Number(_)) | Some(Constant::Float(_)) => {}
                        _ => return false,
                    },
                    Some(
                        Opcode::Add
                        | Opcode::Sub
                        | Opcode::Mul
                        | Opcode::LoadLocal(_)
                        | Opcode::AccAddLocal(_)
                        | Opcode::LenLocal(_)
                        | Opcode::IndexGetLocal(_),
                    ) => {}
                    _ => return false,
                }
            }
        }
        true
    }

    /// Bool/Null consts have no honest i64 lowering — they can leak into
    /// locals (`store_local`) or comparisons where the boxed VM would
    /// keep them typed. String consts stay: the string-char path never
    /// wraps results as Number.
    fn i64_const_domain(func: &BytecodeFnDef) -> bool {
        !func.code.iter().enumerate().any(|(i, op)| {
            if !matches!(op, Opcode::Const(ci) if !matches!(
                func.constants.get(*ci as usize),
                Some(Constant::Number(_)) | Some(Constant::Float(_)) | Some(Constant::String(_))
            )) {
                return false;
            }
            // `const null; pop` (statement-result slot, e.g. after `while`) is
            // discarded before it can reach a local or a comparison — the
            // rust emitter produces it for rust-healed seeds, so rejecting it
            // would make every such seed JIT-ineligible.
            !matches!(func.code.get(i + 1), Some(Opcode::Pop))
        })
    }

    fn refs_string_const(func: &BytecodeFnDef) -> bool {
        func.code.iter().any(|op| {
            matches!(op, Opcode::Const(i) if matches!(func.constants.get(*i as usize), Some(Constant::String(_))))
        })
    }

    fn try_run_str_char_index(
        func: &BytecodeFnDef,
        args: &[KabVal],
        key: u64,
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        let _s = match args.first()? {
            KabVal::String(s) => s,
            _ => return None,
        };
        let two = func.params.len() == 2;
        let join = func.params.len() == 1 && refs_string_const(func);
        if !two && !join {
            return None;
        }
        let compiled = with_state(|st| st.str_char_kernels.insert(key))?;
        if compiled {
            JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
        }
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        if two {
            let idx = args.get(1)?;
            return Some(
                crate::value::index_get_element(args.first()?, idx).map(|v| (v, Vec::new())),
            );
        }
        let n = crate::value::container_len(args.first()?).ok()?;
        let mut t = String::new();
        let src = args.first()?;
        for i in 0..n {
            match crate::value::index_get_element(src, &KabVal::Number(i)) {
                Ok(KabVal::String(ch)) => t.push_str(&ch),
                Ok(KabVal::Undefined) => break,
                Ok(_) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
        Some(Ok((KabVal::String(t), Vec::new())))
    }

    fn fingerprint(func: &BytecodeFnDef) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        func.name.hash(&mut h);
        func.params.hash(&mut h);
        func.locals.hash(&mut h);
        for c in &func.constants {
            match c {
                Constant::Number(n) => n.hash(&mut h),
                Constant::Bool(b) => b.hash(&mut h),
                Constant::Float(f) => f.to_bits().hash(&mut h),
                _ => 0u8.hash(&mut h),
            }
        }
        for op in &func.code {
            std::mem::discriminant(op).hash(&mut h);
            match op {
                Opcode::Const(i)
                | Opcode::LoadLocal(i)
                | Opcode::StoreLocal(i)
                | Opcode::AccAddLocal(i)
                | Opcode::LenLocal(i)
                | Opcode::IndexGetLocal(i) => i.hash(&mut h),
                Opcode::Jump(o) | Opcode::JumpIfFalse(o) => o.hash(&mut h),
                _ => {}
            }
        }
        h.finish()
    }
}
