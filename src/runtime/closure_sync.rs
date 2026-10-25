//! Sync captured bindings between closure snapshots and a live root environment.

use crate::bytecode::BytecodeFnDef;
use crate::value::{BytecodeFunction, Environment, ScopeStamp, Value};
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

thread_local! {
    /// Scope stamps of (closure, root) right after the closure's last pull.
    /// A pull is idempotent, so only names written since then can change.
    static PULL_STAMPS: std::cell::RefCell<
        std::collections::HashMap<(u64, Vec<u64>), (Vec<ScopeStamp>, Vec<ScopeStamp>)>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub fn pull_root_into_closure(closure: &mut Environment, root: &Environment) {
    // Callers' frames are fresh per call; key on the scopes they hang off.
    let mut root_ids = root.scope_ids();
    root_ids.remove(0);
    let key = (closure.scope_id(), root_ids);
    let changed = PULL_STAMPS.with(|m| {
        let m = m.borrow();
        let (pc, pr) = m.get(&key)?;
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        (closure.names_written_since(pc, &mut seen, &mut names)
            && root.names_written_since(pr, &mut seen, &mut names))
        .then_some(names)
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
    let stamps = (closure.scope_stamps(), root.scope_stamps());
    PULL_STAMPS.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() >= 4096 {
            m.clear();
        }
        m.insert(key, stamps);
    });
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
        let mut exact: Option<(String, Value)> = None;
        let mut oid_fallback: Option<(String, Value)> = None;
        env.visit_visible_bindings(|name, live| {
            let Value::Object(live_map) = live else {
                return true;
            };
            if Rc::ptr_eq(live_map, arg_map) {
                exact = Some((name.to_string(), live.clone()));
                return false;
            }
            if oid_fallback.is_none() && object_oid_of(live_map) == Some(oid) {
                oid_fallback = Some((name.to_string(), live.clone()));
            }
            true
        });
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

pub fn pull_bytecode_globals(f: &mut BytecodeFunction, root: &Environment) {
    for name in &f.def.globals {
        if let Some(v) = root.get(name) {
            if let Some(cur) = f.closure.get(name) {
                if same_binding_value(&cur, &v) {
                    continue;
                }
                let _ = f.closure.assign(name, v);
            } else {
                f.closure.set(name.clone(), v);
            }
        }
    }
}

pub fn sync_bytecode_globals_to_root(
    f: &BytecodeFunction,
    call_env: &Environment,
    root: &mut Environment,
) {
    for name in &f.def.globals {
        let Some(v) = call_env.get(name).or_else(|| f.closure.get(name)) else {
            continue;
        };
        if let Some(cur) = root.get(name) {
            if same_binding_value(&cur, &v) {
                continue;
            }
            let _ = root.assign(name, v);
        } else {
            root.set(name.clone(), v);
        }
    }
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
