//! Sync captured bindings between closure snapshots and a live root environment.

use crate::bytecode::BytecodeFnDef;
use crate::value::{BytecodeFunction, Environment, Value};

pub fn sync_closure_writes(closure: &Environment, call_env: &Environment, root: &mut Environment) {
    for name in call_env.all_binding_names() {
        let Some(v) = call_env.get(&name) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if root.get(&name).is_some() {
            let _ = root.assign(&name, v);
        } else if closure.get(&name).is_some() {
            root.set(name, v);
        }
    }
}

pub fn pull_root_into_closure(closure: &mut Environment, root: &Environment) {
    for name in closure.all_binding_names() {
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
    for (k, v) in src {
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
