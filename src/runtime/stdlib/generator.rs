//! ECMAScript-style generator functions (`fn*`) with `yield`.

use crate::bytecode::{find_try_region_for_ip, run_generator_step, ChunkCursor, GeneratorResume};
use crate::runtime::stdlib::async_iterator::{
    attach_async_iterator_instance_methods, ASYNC_ITERATOR_MARKER,
};
use crate::runtime::stdlib::iterator::{
    attach_next_to_map, iterator_result, ITERATOR_MARKER,
};
use crate::runtime::stdlib::object::writeback_generator_by_oid;
use crate::value::{BytecodeFunction, Environment, PromiseValue, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const GEN_MARKER: &str = "__kab_generator";
const GEN_ASYNC: &str = "__kab_async_generator";
const GEN_SUSPENDED: &str = "__kab_gen_suspended";
const GEN_DELEGATE: &str = "__kab_gen_delegate";

fn resolved_promise(value: Value) -> Value {
    Value::Promise(Rc::new(RefCell::new(PromiseValue::Resolved(value))))
}

pub fn is_generator_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(GEN_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_async_generator_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => {
            matches!(m.get(GEN_MARKER), Some(Value::Bool(true)))
                && matches!(m.get(GEN_ASYNC), Some(Value::Bool(true)))
        }
        _ => false,
    }
}

fn generator_suspended(map: &HashMap<String, Value>) -> bool {
    matches!(map.get(GEN_SUSPENDED), Some(Value::Bool(true)))
}

pub fn create_generator(
    mut func: BytecodeFunction,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    if func.def.params.len() != args.len() {
        return Err(format!(
            "Argument count mismatch: expected {}, got {}",
            func.def.params.len(),
            args.len()
        ));
    }
    crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
    let def = func.def.as_ref();
    let is_async = func.def.async_fn;
    let mut local_vals = vec![Value::Undefined; def.locals.len().max(1)];
    for (i, param) in def.params.iter().enumerate() {
        if let Some(idx) = def.locals.iter().position(|l| l == param) {
            local_vals[idx] = args.get(i).cloned().unwrap_or(Value::Undefined);
        }
    }
    let mut map = HashMap::new();
    crate::runtime::stdlib::object::object_oid(&mut map);
    map.insert(GEN_MARKER.into(), Value::Bool(true));
    map.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    map.insert("__kab_gen_fn".into(), Value::BytecodeFn(func));
    map.insert("__kab_gen_locals".into(), Value::from_array(local_vals));
    map.insert("__kab_gen_ip".into(), Value::Number(0));
    map.insert("__kab_gen_stack".into(), Value::from_array(Vec::new()));
    map.insert("__kab_gen_done".into(), Value::Bool(false));
    map.insert(GEN_SUSPENDED.into(), Value::Bool(false));
    if is_async {
        map.insert(GEN_ASYNC.into(), Value::Bool(true));
        map.insert(ASYNC_ITERATOR_MARKER.into(), Value::Bool(true));
        attach_next_to_map(&mut map, async_generator_next_native);
        attach_async_iterator_instance_methods(&mut map);
    } else {
        attach_next_to_map(&mut map, generator_next_native);
        crate::runtime::stdlib::iterator::attach_iterator_return(&mut map);
        crate::runtime::stdlib::iterator::attach_iterator_throw(&mut map);
    }
    Ok(Value::from_object(map))
}

fn generator_next_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("generator.next()")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut it, env);
    let result = advance_generator_with_optional_arg(&mut it, args.get(1), env)?;
    writeback_generator_by_oid(&it, env);
    Ok(result)
}

fn async_generator_next_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("asyncGenerator.next()")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut it, env);
    advance_async_generator_next(&mut it, args.get(1), env)
}

fn advance_generator_with_optional_arg(
    it: &mut Value,
    resume_arg: Option<&Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let resume = resume_arg
        .cloned()
        .map(GeneratorResume::Next)
        .or_else(|| {
            if generator_suspended_from_value(it) {
                Some(GeneratorResume::Next(Value::Undefined))
            } else {
                None
            }
        });
    advance_generator(it, resume, env)
}

/// Advance an async generator in place; returns a resolved Promise of `{ value, done }`.
pub fn advance_async_generator_next(
    it: &mut Value,
    resume_arg: Option<&Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let result = advance_generator_with_optional_arg(it, resume_arg, env)?;
    writeback_generator_by_oid(it, env);
    Ok(resolved_promise(result))
}

fn generator_suspended_from_value(it: &Value) -> bool {
    match it {
        Value::Object(map) => generator_suspended(map),
        _ => false,
    }
}

fn reattach_generator_protocol(map: &mut HashMap<String, Value>) {
    if matches!(map.get(GEN_ASYNC), Some(Value::Bool(true))) {
        attach_next_to_map(map, async_generator_next_native);
    } else {
        attach_next_to_map(map, generator_next_native);
    }
}

pub fn advance_generator(
    it: &mut Value,
    resume: Option<GeneratorResume>,
    env: &mut Environment,
) -> Result<Value, String> {
    let Value::Object(ref mut map) = it else {
        return Err("generator.next() expects generator object".into());
    };
    if matches!(map.get("__kab_gen_done"), Some(Value::Bool(true))) {
        return Ok(iterator_result(Value::Null, true));
    }
    let mut func = match map.get("__kab_gen_fn") {
        Some(Value::BytecodeFn(f)) => f.clone(),
        _ => return Err("internal generator missing function".into()),
    };
    crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
    let def = func.def.as_ref();
    let mut local_vals = match map.get("__kab_gen_locals") {
        Some(Value::Array(items)) => items.as_ref().clone(),
        _ => vec![Value::Undefined; def.locals.len().max(1)],
    };
    let mut cursor = ChunkCursor {
        ip: match map.get("__kab_gen_ip") {
            Some(Value::Number(n)) if *n >= 0 => *n as usize,
            _ => 0,
        },
        stack: match map.get("__kab_gen_stack") {
            Some(Value::Array(items)) => items.as_ref().clone(),
            _ => Vec::new(),
        },
        delegate: map.get(GEN_DELEGATE).cloned(),
        generator_async: matches!(map.get(GEN_ASYNC), Some(Value::Bool(true))),
    };
    let suspended = generator_suspended(map);
    let effective_resume = match resume {
        Some(GeneratorResume::Throw(_)) | Some(GeneratorResume::Return(_)) => resume,
        other if suspended => other,
        _ => None,
    };
    let mut call_env = Environment::child_from(&func.closure);
    let (yield_or_return, done) =
        run_generator_step(def, &[], &mut local_vals, &mut cursor, &mut call_env, effective_resume)?;
    crate::runtime::closure_sync::sync_closure_writes(&func.closure, &call_env, env);
    crate::runtime::closure_sync::sync_bytecode_globals_to_root(&func, &call_env, env);
    Rc::make_mut(map).insert("__kab_gen_fn".into(), Value::BytecodeFn(func));
    Rc::make_mut(map).insert("__kab_gen_locals".into(), Value::from_array(local_vals));
    Rc::make_mut(map).insert("__kab_gen_ip".into(), Value::Number(cursor.ip as i64));
    Rc::make_mut(map).insert("__kab_gen_stack".into(), Value::from_array(cursor.stack));
    Rc::make_mut(map).insert("__kab_gen_done".into(), Value::Bool(done));
    Rc::make_mut(map).insert(GEN_SUSPENDED.into(), Value::Bool(!done));
    if let Some(delegate) = cursor.delegate.clone() {
        Rc::make_mut(map).insert(GEN_DELEGATE.into(), delegate);
    } else {
        Rc::make_mut(map).remove(GEN_DELEGATE);
    }
    reattach_generator_protocol(Value::object_make_mut(map));
    Ok(iterator_result(yield_or_return, done))
}

pub fn throw_generator(
    it: &mut Value,
    reason: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let Value::Object(src) = it else {
        return Err("generator.throw() expects generator object".into());
    };
    if matches!(src.get("__kab_gen_done"), Some(Value::Bool(true))) {
        return Ok(iterator_result(Value::Null, true));
    }
    if src.contains_key(GEN_DELEGATE) {
        let result = advance_generator(it, Some(GeneratorResume::Throw(reason)), env)?;
        writeback_generator_by_oid(it, env);
        return Ok(result);
    }
    let ip = match src.get("__kab_gen_ip") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let has_catch = match src.get("__kab_gen_fn") {
        Some(Value::BytecodeFn(f)) => {
            let def = f.def.as_ref();
            find_try_region_for_ip(def, ip).is_some()
                || (generator_suspended(src)
                    && (1..=3).any(|d| {
                        ip >= d && find_try_region_for_ip(def, ip - d).is_some()
                    }))
        }
        _ => false,
    };
    let suspended = generator_suspended(src);
    if !suspended || !has_catch {
        return close_generator(it, reason, env);
    }
    let result = advance_generator(it, Some(GeneratorResume::Throw(reason)), env)?;
    writeback_generator_by_oid(it, env);
    Ok(result)
}

pub fn return_generator(
    it: &mut Value,
    value: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let Value::Object(src) = it else {
        return Err("generator.return() expects generator object".into());
    };
    if matches!(src.get("__kab_gen_done"), Some(Value::Bool(true))) {
        return Ok(iterator_result(Value::Null, true));
    }
    if src.contains_key(GEN_DELEGATE) {
        let result = advance_generator(it, Some(GeneratorResume::Return(value)), env)?;
        writeback_generator_by_oid(it, env);
        return Ok(result);
    }
    close_generator(it, value, env)
}

pub fn close_generator(
    it: &mut Value,
    value: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("generator.return() expects generator object".into());
    };
    let map = Value::object_make_mut(map_rc);
    map.remove("__kab_gen_done");
    map.insert(GEN_SUSPENDED.into(), Value::Bool(false));
    map.remove(GEN_DELEGATE);
    reattach_generator_protocol(map);
    if !matches!(map.get(GEN_ASYNC), Some(Value::Bool(true))) {
        crate::runtime::stdlib::iterator::attach_iterator_return(map);
        crate::runtime::stdlib::iterator::attach_iterator_throw(map);
    }
    *it = Value::Object(map_rc.clone());
    writeback_generator_by_oid(it, env);
    Ok(iterator_result(value, true))
}

pub fn generator_iterator(v: &Value) -> Option<Value> {
    if is_generator_object(v) {
        Some(v.clone())
    } else {
        None
    }
}
