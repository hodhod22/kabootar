//! Sync captured bindings between closure snapshots and a live root environment.

use crate::bytecode::BytecodeFnDef;
use crate::value::{BytecodeFunction, Environment, Value};
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

pub fn pull_root_into_closure(closure: &mut Environment, root: &Environment) {
    for name in closure.all_binding_names() {
        let Some(cur) = closure.get(&name) else {
            continue;
        };
        if matches!(cur, Value::BytecodeFn(_)) {
            continue;
        }
        if let Some(v) = root.get(&name) {
            let _ = closure.assign(&name, v);
        }
    }
}

pub fn merge_object_fields(from: &Value, into: &mut Value) {
    let Value::Object(src) = from else {
        return;
    };
    let Value::Object(dst) = into else {
        return;
    };
    for (k, v) in src.iter() {
        if !k.starts_with("__kab_") {
            Rc::make_mut(dst).insert(k.clone(), v.clone());
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
        for name in env.all_binding_names() {
            let Some(live) = env.get(&name) else {
                continue;
            };
            let Value::Object(live_map) = &live else {
                continue;
            };
            if object_oid_of(live_map) == Some(oid) {
                let mut merged = live.clone();
                merge_object_fields(updated, &mut merged);
                let _ = env.assign(&name, merged);
                break;
            }
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
        if matches!(updated, Value::Object(_)) {
            out.push((ptr, updated.clone()));
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
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(map) = &live else {
            continue;
        };
        let ptr = Rc::as_ptr(map) as usize;
        if let Some((_, updated)) = wbs.iter().find(|(p, _)| *p == ptr) {
            let _ = env.assign(&name, updated.clone());
        }
    }
}

pub fn pull_bytecode_globals(f: &mut BytecodeFunction, root: &Environment) {
    for name in &f.def.globals {
        if let Some(v) = root.get(name) {
            if f.closure.get(name).is_some() {
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
        if root.get(name).is_some() {
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
