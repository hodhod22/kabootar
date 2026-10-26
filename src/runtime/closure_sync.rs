//! Sync captured bindings between closure snapshots and a live root environment.

use crate::bytecode::BytecodeFnDef;
use crate::value::{BytecodeFunction, Environment, ScopeStamp, StampDelta, Value};
use std::rc::Rc;

pub fn sync_closure_writes(closure: &Environment, call_env: &Environment, root: &mut Environment) {
    sync_closure_writes_filtered(closure, call_env, root, None);
}

/// Like `sync_closure_writes`, but when `captures` is set only sync capture slots
/// (plus any name that exists on `closure` and is listed as a capture).
pub fn sync_closure_writes_filtered(
    closure: &Environment,
    call_env: &Environment,
    root: &mut Environment,
    capture_names: Option<&[String]>,
) {
    let names: Vec<String> = match capture_names {
        Some(caps) => caps.to_vec(),
        None => call_env.local_names(),
    };
    for name in names {
        let Some(v) = call_env.get(&name) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if closure.get(&name).is_none() {
            continue;
        }
        if root.get(&name).is_some() {
            let _ = root.assign(&name, v);
        } else {
            root.set(name, v);
        }
    }
}

/// Stamps of two scope chains after a sync, plus per-side innermost names
/// already filtered by the sync's predicate (valid while the predicate's
/// inputs are unchanged).
pub struct SyncStamps {
    a: Vec<ScopeStamp>,
    b: Vec<ScopeStamp>,
    filtered: [Option<(Rc<[String]>, Vec<String>)>; 2],
}

type SyncCache<K> = std::cell::RefCell<std::collections::HashMap<K, SyncStamps>>;

fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|n| n == name) {
        out.push(name.to_string());
    }
}

/// Names of `a`/`b` accepted by `keep` that may have changed since `prev`.
/// `None` when that cannot be bounded (callers then rescan everything).
/// `keep_ctx_side` names the chain `keep` reads; any change there drops the
/// cached filters.
fn names_changed_since(
    prev: &mut SyncStamps,
    a: &Environment,
    b: &Environment,
    keep_ctx_side: Option<usize>,
    keep: impl Fn(&str) -> bool,
) -> Option<Vec<String>> {
    let deltas = [a.stamp_delta(&prev.a), b.stamp_delta(&prev.b)];
    if deltas.iter().any(|d| matches!(d, StampDelta::Other)) {
        prev.filtered = [None, None];
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        let ok = a.names_written_since(&prev.a, &mut seen, &mut names)
            && b.names_written_since(&prev.b, &mut seen, &mut names);
        return ok.then(|| names.into_iter().filter(|n| keep(n)).collect());
    }
    if let Some(side) = keep_ctx_side {
        if !matches!(deltas[side], StampDelta::Same) {
            prev.filtered = [None, None];
        }
    }
    let mut out = Vec::new();
    for (side, delta) in deltas.iter().enumerate() {
        let StampDelta::Inner(keys) = delta else {
            continue;
        };
        let hit = matches!(&prev.filtered[side], Some((k, _)) if Rc::ptr_eq(k, keys) || **k == **keys);
        if !hit {
            let names = keys.iter().filter(|n| keep(n)).cloned().collect();
            prev.filtered[side] = Some((Rc::clone(keys), names));
        }
        if let Some((_, names)) = &prev.filtered[side] {
            for n in names {
                push_unique(&mut out, n);
            }
        }
    }
    Some(out)
}

fn take_stamps<K: std::hash::Hash + Eq>(
    cache: &'static std::thread::LocalKey<SyncCache<K>>,
    key: &K,
) -> Option<SyncStamps> {
    cache.with(|m| m.borrow_mut().remove(key))
}

fn store_stamps<K: std::hash::Hash + Eq>(
    cache: &'static std::thread::LocalKey<SyncCache<K>>,
    key: K,
    prev: Option<SyncStamps>,
    a: &Environment,
    b: &Environment,
) {
    let filtered = prev.map(|p| p.filtered).unwrap_or([None, None]);
    let stamps = SyncStamps {
        a: a.scope_stamps(),
        b: b.scope_stamps(),
        filtered,
    };
    cache.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() >= 4096 {
            m.clear();
        }
        m.insert(key, stamps);
    });
}

type GlobalSyncKey = (usize, Vec<u64>, Vec<u64>);

thread_local! {
    /// Stamps of (closure, root) right after the closure's last pull.
    /// A pull is idempotent, so only names written since then can change.
    static PULL_STAMPS: SyncCache<(u64, Vec<u64>)> = std::cell::RefCell::new(std::collections::HashMap::new());
    /// Stamps of (closure, root) right after `pull_bytecode_globals`.
    static PULL_GLOBAL_STAMPS: SyncCache<GlobalSyncKey> = std::cell::RefCell::new(std::collections::HashMap::new());
    /// Stamps of (call env, root) right after `sync_bytecode_globals_to_root`.
    static PUSH_GLOBAL_STAMPS: SyncCache<GlobalSyncKey> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn parent_scope_ids(env: &Environment) -> Vec<u64> {
    let mut ids = env.scope_ids();
    ids.remove(0);
    ids
}

pub fn pull_root_into_closure(closure: &mut Environment, root: &Environment) {
    // Callers' frames are fresh per call; key on the scopes they hang off.
    let key = (closure.scope_id(), parent_scope_ids(root));
    let mut prev = take_stamps(&PULL_STAMPS, &key);
    let changed = prev.as_mut().and_then(|p| {
        let c = &*closure;
        names_changed_since(p, c, root, Some(0), |n| c.get(n).is_some())
    });
    let mut updates: Vec<(String, Value)> = Vec::new();
    let mut check = |name: &str, cur: &Value| {
        if matches!(cur, Value::BytecodeFn(_)) {
            return;
        }
        if let Some(v) = root.get(name) {
            if !same_binding_value(cur, &v) {
                updates.push((name.to_string(), v));
            }
        }
    };
    match changed.or_else(|| closure.binding_names_unshared_with(root)) {
        Some(names) => {
            for name in names {
                if let Some(cur) = closure.get(&name) {
                    check(&name, &cur);
                }
            }
        }
        None => closure.visit_visible_bindings(|name, cur| {
            check(name, cur);
            true
        }),
    }
    for (name, v) in updates {
        let _ = closure.assign(&name, v);
    }
    store_stamps(&PULL_STAMPS, key, prev, closure, root);
}

/// Re-assigning an identical value is a no-op, so the pull can skip it.
fn same_binding_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::NativeFunction(x), Value::NativeFunction(y)) => *x as usize == *y as usize,
        (Value::KabootarDom(x), Value::KabootarDom(y)) => x.same_tree(y),
        (Value::OsHandle(x), Value::OsHandle(y)) => x.same_handle(y),
        (Value::DbConnection(x), Value::DbConnection(y)) => x.same_handle(y),
        (Value::HttpRouter(x), Value::HttpRouter(y)) => x.same_handle(y),
        (Value::SecurityHandle(x), Value::SecurityHandle(y)) => x.same_handle(y),
        (Value::KabootarBrowser(x), Value::KabootarBrowser(y)) => x.same_handle(y),
        (Value::BytecodeFn(x), Value::BytecodeFn(y)) => {
            Rc::ptr_eq(&x.def, &y.def) && x.closure.scope_id() == y.closure.scope_id()
        }
        _ => crate::ops::values_identical(a, b),
    }
}

pub fn merge_object_fields(from: &Value, into: &mut Value) {
    let Value::Object(src) = from else {
        return;
    };
    let Value::Object(dst) = into else {
        return;
    };
    if std::rc::Rc::ptr_eq(src, dst) {
        return;
    }
    let dst = Value::object_make_mut(dst);
    for (k, v) in src.iter() {
        if !k.starts_with("__kab_") {
            dst.insert(k.clone(), v.clone());
        }
    }
}

fn local_vals_from_params(f: &BytecodeFunction, local_vals: &[Value]) -> Vec<Value> {
    f.def
        .params
        .iter()
        .filter_map(|param| {
            f.def
                .locals
                .iter()
                .position(|name| name == param)
                .and_then(|idx| local_vals.get(idx))
                .cloned()
        })
        .collect()
}

pub fn writeback_object_args(
    func: &BytecodeFnDef,
    args: &[Value],
    local_vals: &[Value],
    env: &mut Environment,
) {
    use crate::runtime::stdlib::object::object_oid_of;
    for (param, arg) in func.params.iter().zip(args) {
        let Value::Object(arg_map) = arg else {
            continue;
        };
        let Some(oid) = object_oid_of(arg_map) else {
            continue;
        };
        let Some(idx) = func.locals.iter().position(|l| l == param) else {
            continue;
        };
        let Some(updated) = local_vals.get(idx) else {
            continue;
        };
        // Prefer the binding that still holds the exact passed map: a bare
        // oid match can alias a distinct same-oid object (e.g. a CoW snapshot
        // clone that kept `__kab_oid`) and merge stale fields into it.
        // The caller's own frame usually holds it and shadows outer scopes.
        let mut exact = env.find_own_binding(|v| matches!(v, Value::Object(m) if Rc::ptr_eq(m, arg_map)));
        if exact.is_none() {
            exact = env.find_visible_binding(|v| matches!(v, Value::Object(m) if Rc::ptr_eq(m, arg_map)));
        }
        let oid_fallback = if exact.is_none() {
            env.find_visible_binding(|v| matches!(v, Value::Object(m) if object_oid_of(m) == Some(oid)))
        } else {
            None
        };
        if let Some((name, live)) = exact.or(oid_fallback) {
            let mut merged = live;
            merge_object_fields(updated, &mut merged);
            let _ = env.assign(&name, merged);
        }
    }
}

/// Pointer identity of the object Rc at call time → callee's mutated object.
pub fn object_arg_writebacks(
    func: &BytecodeFnDef,
    args: &[Value],
    local_vals: &[Value],
) -> Vec<(usize, Value)> {
    let mut out = Vec::new();
    for (param, arg) in func.params.iter().zip(args) {
        let Value::Object(arg_map) = arg else {
            continue;
        };
        let ptr = Rc::as_ptr(arg_map) as usize;
        let Some(idx) = func.locals.iter().position(|l| l == param) else {
            continue;
        };
        let Some(updated) = local_vals.get(idx) else {
            continue;
        };
        match updated {
            // Mutated in place: every slot holding this Rc already sees it.
            Value::Object(m) if Rc::ptr_eq(m, arg_map) => {}
            Value::Object(_) => out.push((ptr, updated.clone())),
            _ => {}
        }
    }
    out
}

pub fn apply_object_arg_writebacks(local_vals: &mut [Value], wbs: &[(usize, Value)]) {
    if wbs.is_empty() {
        return;
    }
    for slot in local_vals.iter_mut() {
        let Value::Object(map) = slot else {
            continue;
        };
        let ptr = Rc::as_ptr(map) as usize;
        if let Some((_, updated)) = wbs.iter().find(|(p, _)| *p == ptr) {
            *slot = updated.clone();
        }
    }
}

pub fn apply_object_arg_writebacks_env(env: &mut Environment, wbs: &[(usize, Value)]) {
    if wbs.is_empty() {
        return;
    }
    let mut hits: Vec<(String, Value)> = Vec::new();
    env.visit_visible_bindings(|name, live| {
        if let Value::Object(map) = live {
            let ptr = Rc::as_ptr(map) as usize;
            if let Some((_, updated)) = wbs.iter().find(|(p, _)| *p == ptr) {
                hits.push((name.to_string(), updated.clone()));
            }
        }
        true
    });
    for (name, updated) in hits {
        let _ = env.assign(&name, updated);
    }
}

fn is_fn_decl(v: &Value, name: &str) -> bool {
    matches!(v, Value::BytecodeFn(b) if b.def.name == name)
}

fn pull_bytecode_global(f: &mut BytecodeFunction, root: &Environment, name: &str) {
    if let Some(v) = root.get(name) {
        if let Some(cur) = f.closure.get(name) {
            // A module's own `fn name` must not be replaced by a same-named
            // function from another module (e.g. the self-host compiler).
            if same_binding_value(&cur, &v) || is_fn_decl(&cur, name) {
                return;
            }
            let _ = f.closure.assign(name, v);
        } else {
            f.closure.set(name.to_string(), v);
        }
    }
}

pub fn pull_bytecode_globals(f: &mut BytecodeFunction, root: &Environment) {
    if f.def.globals.is_empty() {
        return;
    }
    let key = (
        Rc::as_ptr(&f.def) as usize,
        f.closure.scope_ids(),
        parent_scope_ids(root),
    );
    let mut prev = take_stamps(&PULL_GLOBAL_STAMPS, &key);
    let def = f.def.clone();
    let changed = prev.as_mut().and_then(|p| {
        names_changed_since(p, &f.closure, root, None, |n| def.globals.iter().any(|g| g == n))
    });
    match changed {
        Some(names) => {
            for name in &names {
                pull_bytecode_global(f, root, name);
            }
        }
        None => {
            for name in &def.globals {
                pull_bytecode_global(f, root, name);
            }
        }
    }
    store_stamps(&PULL_GLOBAL_STAMPS, key, prev, &f.closure, root);
}

fn push_bytecode_global(
    f: &BytecodeFunction,
    call_env: &Environment,
    root: &mut Environment,
    name: &str,
) {
    let Some(v) = call_env.get(name).or_else(|| f.closure.get(name)) else {
        return;
    };
    if let Some(cur) = root.get(name) {
        if same_binding_value(&cur, &v) {
            return;
        }
        let _ = root.assign(name, v);
    } else {
        root.set(name.to_string(), v);
    }
}

pub fn sync_bytecode_globals_to_root(
    f: &BytecodeFunction,
    call_env: &Environment,
    root: &mut Environment,
) {
    if f.def.globals.is_empty() {
        return;
    }
    // `call_env` is a fresh frame over `f.closure`; its parents carry identity.
    let key = (
        Rc::as_ptr(&f.def) as usize,
        parent_scope_ids(call_env),
        parent_scope_ids(root),
    );
    let mut prev = take_stamps(&PUSH_GLOBAL_STAMPS, &key);
    let globals = &f.def.globals;
    let changed = prev.as_mut().and_then(|p| {
        names_changed_since(p, call_env, root, None, |n| globals.iter().any(|g| g == n))
    });
    match changed {
        Some(names) => {
            for name in &names {
                push_bytecode_global(f, call_env, root, name);
            }
        }
        None => {
            for name in globals {
                push_bytecode_global(f, call_env, root, name);
            }
        }
    }
    store_stamps(&PUSH_GLOBAL_STAMPS, key, prev, call_env, root);
}

pub fn call_with_closure_sync(
    callee: &mut Value,
    args: Vec<Value>,
    env: &mut Environment,
    merge_target: Option<&mut Value>,
) -> Result<Value, String> {
    if let Value::BytecodeFn(f) = callee {
        pull_bytecode_globals(f, env);
        pull_root_into_closure(&mut f.closure, env);
        let mut call_env = Environment::child_from(&f.closure);
        let def = f.def.clone();
        let (result, local_vals) = match crate::bytecode::run_bytecode_fn_with_locals(
            def.as_ref(),
            args.clone(),
            &mut call_env,
        ) {
            Ok(v) => v,
            Err(_) if args.len() == 1 && def.params.is_empty() => {
                crate::bytecode::run_bytecode_fn_with_locals(def.as_ref(), vec![], &mut call_env)?
            }
            Err(e) => return Err(e),
        };
        sync_closure_writes(&f.closure, &call_env, env);
        sync_bytecode_globals_to_root(f, &call_env, env);
        if let Some(target) = merge_target {
            for val in local_vals_from_params(f, &local_vals) {
                if matches!(val, Value::Object(_)) && matches!(target, Value::Object(_)) {
                    merge_object_fields(&val, target);
                    break;
                }
            }
            crate::runtime::stdlib::object::writeback_bytecode_fn_closure_on_receiver(target, f);
        }
        return Ok(result);
    }
    crate::bytecode::call_value(
        callee.clone(),
        args,
        &[],
        &[],
        &[],
        &[],
        env,
    )
}
