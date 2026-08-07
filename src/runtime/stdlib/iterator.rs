//! ECMAScript iterator protocol — `{ next() }` with `{ value, done }` results.

use crate::runtime::stdlib::descriptor::is_callable_value;
use crate::runtime::stdlib::map::{
    is_map_value, is_set_value, map_get_at_id, map_id, map_key_list, set_id,
    set_values_for_iteration,
};
use crate::runtime::stdlib::object::object_oid_of;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::rc::Rc;

pub const SYMBOL_ITERATOR: u64 = 1;

pub const ITERATOR_MARKER: &str = "__kab_iterator";
const ITER_ARRAY: &str = "__kab_iter_items";
const ITER_INDEX: &str = "__kab_iter_index";
const ITER_RANGE: &str = "__kab_iter_range";

pub fn is_iterator_value(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(ITERATOR_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

fn has_next_method(v: &Value) -> bool {
    match v {
        Value::Object(m) => m
            .get("next")
            .is_some_and(|next| is_callable_value(next)),
        _ => false,
    }
}

pub fn iterator_result(value: Value, done: bool) -> Value {
    let mut m = HashMap::new();
    m.insert("value".into(), value);
    m.insert("done".into(), Value::Bool(done));
    Value::from_object(m)
}

pub fn iterator_is_returned(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(ITER_RETURNED), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn iterator_mark_returned(it: &mut Value) {
    if let Value::Object(ref mut map_rc) = it {
        Value::object_make_mut(map_rc).insert(ITER_RETURNED.into(), Value::Bool(true));
    }
}

pub fn parse_iterator_result(v: &Value) -> Result<(Value, bool), String> {
    let Value::Object(m) = v else {
        return Err("iterator.next() must return an object with done and value".into());
    };
    let done = matches!(m.get("done"), Some(Value::Bool(true)));
    let value = m.get("value").cloned().unwrap_or(Value::Null);
    Ok((value, done))
}

pub fn create_array_iterator(items: Vec<Value>) -> Value {
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ITER_ARRAY.into(), Value::from_array(items));
    m.insert(ITER_INDEX.into(), Value::Number(0));
    crate::runtime::stdlib::object::object_oid(&mut m);
    attach_iterator_instance_methods(&mut m);
    attach_next_native(&mut m)
}

fn attach_next_native(map: &mut HashMap<String, Value>) -> Value {
    attach_next_to_map(map, array_iterator_next_native);
    Value::from_object(map.clone())
}

const ITER_METHODS: &str = "__kab_iter_methods";

const ITER_BOUND_METHODS: &[&str] = &[
    "next",
    "map",
    "filter",
    "take",
    "skip",
    "flatMap",
    "dropWhile",
    "toArray",
    "reduce",
    "some",
    "every",
    "return",
    "throw",
    "forEach",
    "find",
    "findIndex",
    "includes",
];

/// Drop eager-bound instance methods from a sync iterator used as an async delegate.
pub(crate) fn strip_iterator_bound_methods(mut iter: Value) -> Value {
    let Value::Object(ref mut map_rc) = iter else {
        return iter;
    };
    let map = Value::object_make_mut(map_rc);
    for name in ITER_BOUND_METHODS {
        if *name != "next" {
            map.remove(*name);
        }
    }
    map.remove(ITER_METHODS);
    Value::Object(map_rc.clone())
}

fn iterator_bound_receiver(map: &HashMap<String, Value>) -> Value {
    let mut slim = HashMap::new();
    for (k, v) in map.iter() {
        if ITER_BOUND_METHODS.contains(&k.as_str())
            || k == ITER_METHODS
            || k == "__kab_async_iter_methods"
        {
            continue;
        }
        slim.insert(k.clone(), v.clone());
    }
    Value::from_object(slim)
}

pub(crate) fn attach_bound_method(
    map: &mut HashMap<String, Value>,
    name: &str,
    native: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    map.insert(
        name.into(),
        Value::BoundNative(Box::new(iterator_bound_receiver(map)), native),
    );
}

pub fn attach_next_to_map(
    map: &mut HashMap<String, Value>,
    native: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    attach_bound_method(map, "next", native);
}

fn array_iterator_next_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let it = args.first().ok_or("iterator.next()")?;
    array_iterator_next(it)
}

fn array_iterator_next(it: &Value) -> Result<Value, String> {
    let Value::Object(map) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let items = match map.get(ITER_ARRAY) {
        Some(Value::Array(items)) => items,
        _ => return Err("internal array iterator missing items".into()),
    };
    let idx = match map.get(ITER_INDEX) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    if idx >= items.len() {
        return Ok(iterator_result(Value::Null, true));
    }
    let value = items[idx].clone();
    // Index is advanced by the caller when mutating the iterator object in-place.
    Ok(iterator_result(value, false))
}

fn advance_array_iterator(it: &mut Value) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let map = Value::object_make_mut(map_rc);
    let items = match map.get(ITER_ARRAY) {
        Some(Value::Array(items)) => items.as_ref().clone(),
        _ => return Err("internal array iterator missing items".into()),
    };
    let idx = match map.get(ITER_INDEX) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    if idx >= items.len() {
        return Ok(iterator_result(Value::Null, true));
    }
    let value = items[idx].clone();
    map.insert(ITER_INDEX.into(), Value::Number((idx + 1) as i64));
    Ok(iterator_result(value, false))
}

fn create_range_iterator(start: i64, end: i64, step: i64) -> Result<Value, String> {
    if step == 0 {
        return Err("range step cannot be 0".into());
    }
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ITER_RANGE.into(), Value::Bool(true));
    m.insert("__kab_range_start".into(), Value::Number(start));
    m.insert("__kab_range_end".into(), Value::Number(end));
    m.insert("__kab_range_step".into(), Value::Number(step));
    m.insert("__kab_range_cur".into(), Value::Number(start));
    crate::runtime::stdlib::object::object_oid(&mut m);
    let it = Value::from_object(m.clone());
    m.insert(
        "next".into(),
        Value::BoundNative(Box::new(it), range_iterator_next_native),
    );
    attach_iterator_instance_methods(&mut m);
    Ok(Value::from_object(m))
}

fn range_i64(map: &HashMap<String, Value>, key: &str) -> Result<i64, String> {
    match map.get(key) {
        Some(Value::Number(n)) => Ok(*n),
        _ => Err(format!("internal range iterator missing {key}")),
    }
}

fn range_iterator_next_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator.next()")?.clone();
    advance_range_iterator(&mut it)
}

fn advance_range_iterator(it: &mut Value) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let end = range_i64(map_rc.as_ref(), "__kab_range_end")?;
    let step = range_i64(map_rc.as_ref(), "__kab_range_step")?;
    let cur = range_i64(map_rc.as_ref(), "__kab_range_cur")?;
    let done = if step > 0 { cur >= end } else { cur <= end };
    if done {
        return Ok(iterator_result(Value::Null, true));
    }
    let value = Value::Number(cur);
    Value::object_make_mut(map_rc).insert("__kab_range_cur".into(), Value::Number(cur + step));
    Ok(iterator_result(value, false))
}

fn wrap_delegate_iterator(target: Value) -> Value {
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert("__kab_iter_delegate".into(), target);
    crate::runtime::stdlib::object::object_oid(&mut m);
    let it = Value::from_object(m.clone());
    m.insert(
        "next".into(),
        Value::BoundNative(Box::new(it), delegate_iterator_next_native),
    );
    attach_iterator_instance_methods(&mut m);
    Value::from_object(m)
}

pub(crate) fn next_fn_uses_param(next_fn: &Value) -> bool {
    match next_fn {
        Value::BytecodeFn(f) => !f.def.params.is_empty(),
        Value::Function { params, .. } => !params.is_empty(),
        _ => false,
    }
}

pub fn call_delegate_next(
    next_fn: Value,
    mut target: Value,
    env: &mut Environment,
) -> Result<(Value, Value), String> {
    if next_fn_uses_param(&next_fn) {
        let mut callee = next_fn;
        let result = crate::runtime::closure_sync::call_with_closure_sync(
            &mut callee,
            vec![target.clone()],
            env,
            Some(&mut target),
        )?;
        return Ok((result, target));
    }
    crate::runtime::stdlib::object::call_object_method(next_fn, vec![], target, env)
}

fn delegate_iterator_next(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let Some(mut target) = map_rc.get("__kab_iter_delegate").cloned() else {
        return Err("internal delegate iterator missing target".into());
    };
    if crate::runtime::stdlib::generator::is_generator_object(&target) {
        let result = crate::runtime::stdlib::generator::advance_generator(&mut target, None, env)?;
        Value::object_make_mut(map_rc).insert("__kab_iter_delegate".into(), target);
        return Ok(result);
    }
    let next_fn = match &target {
        Value::Object(inner) => inner
            .get("next")
            .cloned()
            .ok_or("iterator object missing next()")?,
        _ => return Err("iterator object missing next()".into()),
    };
    let (result, updated_target) = call_delegate_next(next_fn.clone(), target, env)?;
    target = updated_target;
    if !matches!(&result, Value::Object(_)) {
        return Err(format!(
            "iterator.next() must return an object with done and value, got {}",
            crate::value::format_value(&result)
        ));
    }
    Value::object_make_mut(map_rc).insert("__kab_iter_delegate".into(), target);
    parse_iterator_result(&result).map(|(value, done)| iterator_result(value, done))
}

fn delegate_iterator_next_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let it = args.first().ok_or("iterator.next()")?;
    let mut it = it.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut it, env);
    delegate_iterator_next(&mut it, env)
}

fn delegate_iterator_return(
    target: &mut Value,
    value: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(target) {
        return crate::runtime::stdlib::generator::return_generator(target, value, env);
    }
    if let Value::Object(inner) = target {
        if let Some(ret_fn) = inner.get("return").cloned() {
            let args = vec![value.clone()];
            let result = if next_fn_uses_param(&ret_fn) {
                let mut callee = ret_fn;
                crate::runtime::closure_sync::call_with_closure_sync(
                    &mut callee,
                    args,
                    env,
                    Some(target),
                )?
            } else {
                let (result, _) = crate::runtime::stdlib::object::call_object_method(
                    ret_fn,
                    args,
                    target.clone(),
                    env,
                )?;
                result
            };
            if let Value::Object(_) = &result {
                return parse_iterator_result(&result)
                    .map(|(v, done)| iterator_result(v, done));
            }
        }
    }
    Ok(iterator_result(value, true))
}

fn delegate_iterator_throw(
    target: &mut Value,
    reason: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(target) {
        return crate::runtime::stdlib::generator::throw_generator(target, reason, env);
    }
    if let Value::Object(inner) = target {
        if let Some(throw_fn) = inner.get("throw").cloned() {
            let args = vec![reason.clone()];
            let result = if next_fn_uses_param(&throw_fn) {
                let mut callee = throw_fn;
                crate::runtime::closure_sync::call_with_closure_sync(
                    &mut callee,
                    args,
                    env,
                    Some(target),
                )?
            } else {
                let (result, _) = crate::runtime::stdlib::object::call_object_method(
                    throw_fn,
                    args,
                    target.clone(),
                    env,
                )?;
                result
            };
            if let Value::Object(_) = &result {
                return parse_iterator_result(&result)
                    .map(|(v, done)| iterator_result(v, done));
            }
        }
    }
    Ok(iterator_result(reason, true))
}

pub fn iterator_collect(iter: &mut Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for _ in 0..4096 {
        let (value, done) = iterator_next(iter, env)?;
        if done {
            break;
        }
        out.push(value);
    }
    if out.len() >= 4096 {
        return Err("iterator exceeded maximum steps".into());
    }
    Ok(out)
}

const ITER_LAZY: &str = "__kab_iter_lazy";
const ITER_SRC: &str = "__kab_iter_src";
const ITER_FN: &str = "__kab_iter_fn";
const ITER_TAKE: &str = "__kab_iter_take";
const ITER_CHAIN: &str = "__kab_iter_chain";
const ITER_CHAIN_IDX: &str = "__kab_iter_chain_idx";
const ITER_ZIP_RIGHT: &str = "__kab_iter_zip_right";
const ITER_ENUM_IDX: &str = "__kab_iter_enum_idx";
const ITER_MAP: &str = "__kab_iter_map";
const ITER_MAP_ID: &str = "__kab_iter_map_id";
const ITER_MAP_KEYS: &str = "__kab_iter_map_keys";
const ITER_SET: &str = "__kab_iter_set";
const ITER_FLAT_INNER: &str = "__kab_iter_flat_inner";
const ITER_FLAT_DEPTH: &str = "__kab_iter_flat_depth";
const ITER_DROP_DONE: &str = "__kab_iter_drop_done";
const ITER_TAKE_DONE: &str = "__kab_iter_take_done";
const ITER_PAIR_PREV: &str = "__kab_iter_pair_prev";
const ITER_ACC: &str = "__kab_iter_acc";
const ITER_ACC_INITIAL: &str = "__kab_iter_acc_initial";
const ITER_ACC_SEEDED: &str = "__kab_iter_acc_seeded";
const ITER_RETURNED: &str = "__kab_iter_returned";

fn call_fn(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    let mut callee = func.clone();
    crate::runtime::closure_sync::call_with_closure_sync(&mut callee, args, env, None)
}

/// Obtain a sync iterator for any iterable (without materializing to array).
pub fn get_sync_iterator(v: &Value, env: &mut Environment) -> Result<Value, String> {
    let owned_target;
    let effective: &Value = if let Some(target) =
        crate::runtime::stdlib::proxy::proxy_target_for_iteration(v)
    {
        owned_target = target;
        &owned_target
    } else {
        v
    };
    if is_iterator_value(effective) {
        return Ok(effective.clone());
    }
    if let Value::Object(map) = effective {
        if let Ok(Some(iter_fn)) = crate::runtime::stdlib::descriptor::get_own_symbol(
            map,
            SYMBOL_ITERATOR,
            effective,
            env,
        ) {
            let result = match call_fn(&iter_fn, vec![], env) {
                Ok(val) => val,
                Err(_) => call_fn(&iter_fn, vec![effective.clone()], env)?,
            };
            if let Value::Array(items) = result {
                return Ok(create_array_iterator(items.as_ref().clone()));
            }
            return normalize_iterator(result);
        }
    }
    if let Some(it) = builtin_iterator(effective) {
        return Ok(it);
    }
    if let Some(it) = object_with_next_iterator(effective) {
        return Ok(it);
    }
    Ok(create_array_iterator(crate::runtime::stdlib::map::for_of_items(effective)?))
}

/// Persist mutated iterator object back into the environment (by `__kab_oid`).
pub fn writeback_iterator_by_oid(updated: &Value, env: &mut Environment) {
    let Value::Object(updated_map) = updated else {
        return;
    };
    let Some(oid) = object_oid_of(updated_map) else {
        return;
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
            if let Value::Object(dst) = &mut merged {
                if let Value::Object(src) = updated {
                    for (k, v) in src.iter() {
                        if k.starts_with("__kab_") || k == "next" {
                            Rc::make_mut(dst).insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            let _ = env.assign(&name, merged);
        }
    }
}

fn persist_lazy_src(it: &mut Value, src: &Value) {
    let Value::Object(ref mut map_rc) = it else {
        return;
    };
    Value::object_make_mut(map_rc).insert(ITER_SRC.into(), src.clone());
}

/// Advance `it` and return `{ value, done }`; mutates iterator state in place.
pub fn iterator_step(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    let (value, done) = iterator_next(it, env)?;
    Ok(iterator_result(value, done))
}

fn attach_lazy_next(map: &mut HashMap<String, Value>) {
    attach_next_to_map(map, lazy_iterator_next_native);
}

fn lazy_iterator_next_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator.next()")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut it, env);
    lazy_iterator_next(&mut it, env)
}

fn lazy_iterator_next(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let kind = map_rc
        .get(ITER_LAZY)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .ok_or("internal lazy iterator missing kind")?
        .to_string();
    let mut src = map_rc
        .get(ITER_SRC)
        .cloned()
        .ok_or("internal lazy iterator missing source")?;
    let map = Value::object_make_mut(map_rc);
    let result = match kind.as_str() {
        "map" => {
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal map iterator missing fn")?;
            loop {
                let (value, done) = iterator_next(&mut src, env)?;
                if done {
                    break Ok(iterator_result(Value::Null, true));
                }
                let mapped = call_fn(&func, vec![value], env)?;
                break Ok(iterator_result(mapped, false));
            }
        }
        "filter" => {
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal filter iterator missing fn")?;
            let mut result = None;
            for _ in 0..4096 {
                let (value, done) = iterator_next(&mut src, env)?;
                if done {
                    result = Some(iterator_result(Value::Null, true));
                    break;
                }
                if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                    result = Some(iterator_result(value, false));
                    break;
                }
            }
            result.ok_or_else(|| "filter iterator exceeded maximum steps".into())
        }
        "take" => match map.get(ITER_TAKE) {
            Some(Value::Number(n)) if *n > 0 => {
                let (value, done) = iterator_next(&mut src, env)?;
                map.insert(ITER_TAKE.into(), Value::Number(n - 1));
                if done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    Ok(iterator_result(value, false))
                }
            }
            _ => Ok(iterator_result(Value::Null, true)),
        },
        "pass" => {
            let (value, done) = iterator_next(&mut src, env)?;
            Ok(iterator_result(value, done))
        },
        "chain" => {
            let chain = match map.get(ITER_CHAIN) {
                Some(Value::Array(items)) => items.clone(),
                _ => return Err("internal chain iterator missing iterables".into()),
            };
            let mut idx = match map.get(ITER_CHAIN_IDX) {
                Some(Value::Number(n)) if *n >= 0 => *n as usize,
                _ => 0,
            };
            let mut result = None;
            for _ in 0..4096 {
                let (value, done) = iterator_next(&mut src, env)?;
                if !done {
                    map.insert(ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
                    result = Some(iterator_result(value, false));
                    break;
                }
                if idx + 1 >= chain.len() {
                    map.insert(ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
                    result = Some(iterator_result(Value::Null, true));
                    break;
                }
                idx += 1;
                src = get_sync_iterator(&chain[idx], env)?;
                map.insert(ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
            }
            result.ok_or_else(|| "chain iterator exceeded maximum steps".into())
        }
        "zip" => {
            let mut right = map
                .get(ITER_ZIP_RIGHT)
                .cloned()
                .ok_or("internal zip iterator missing right source")?;
            let (left_val, left_done) = iterator_next(&mut src, env)?;
            if left_done {
                Ok(iterator_result(Value::Null, true))
            } else {
                let (right_val, right_done) = iterator_next(&mut right, env)?;
                if right_done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    map.insert(ITER_ZIP_RIGHT.into(), right);
                    Ok(iterator_result(
                        Value::from_array(vec![left_val, right_val]),
                        false,
                    ))
                }
            }
        }
        "enumerate" => {
            let idx = match map.get(ITER_ENUM_IDX) {
                Some(Value::Number(n)) if *n >= 0 => *n,
                _ => 0,
            };
            let (value, done) = iterator_next(&mut src, env)?;
            if done {
                Ok(iterator_result(Value::Null, true))
            } else {
                let pair = Value::from_array(vec![Value::Number(idx), value]);
                map.insert(ITER_ENUM_IDX.into(), Value::Number(idx + 1));
                Ok(iterator_result(pair, false))
            }
        }
        "dropwhile" => {
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal dropWhile iterator missing fn")?;
            if matches!(map.get(ITER_DROP_DONE), Some(Value::Bool(true))) {
                let (value, done) = iterator_next(&mut src, env)?;
                Ok(iterator_result(value, done))
            } else {
                let mut result = None;
                for _ in 0..4096 {
                    let (value, done) = iterator_next(&mut src, env)?;
                    if done {
                        result = Some(iterator_result(Value::Null, true));
                        break;
                    }
                    if !call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                        map.insert(ITER_DROP_DONE.into(), Value::Bool(true));
                        result = Some(iterator_result(value, false));
                        break;
                    }
                }
                result.ok_or_else(|| "dropWhile iterator exceeded maximum steps".into())
            }
        }
        "takewhile" => {
            if matches!(map.get(ITER_TAKE_DONE), Some(Value::Bool(true))) {
                return Ok(iterator_result(Value::Null, true));
            }
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal takeWhile iterator missing fn")?;
            let (value, done) = iterator_next(&mut src, env)?;
            if done {
                Ok(iterator_result(Value::Null, true))
            } else if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                Ok(iterator_result(value, false))
            } else {
                map.insert(ITER_TAKE_DONE.into(), Value::Bool(true));
                Ok(iterator_result(Value::Null, true))
            }
        }
        "pairwise" => {
            let mut result = None;
            for _ in 0..4096 {
                if let Some(prev) = map.get(ITER_PAIR_PREV).cloned() {
                    let (value, done) = iterator_next(&mut src, env)?;
                    if done {
                        map.remove(ITER_PAIR_PREV);
                        result = Some(iterator_result(Value::Null, true));
                        break;
                    }
                    map.insert(ITER_PAIR_PREV.into(), value.clone());
                    result = Some(iterator_result(
                        Value::from_array(vec![prev, value]),
                        false,
                    ));
                    break;
                }
                let (value, done) = iterator_next(&mut src, env)?;
                if done {
                    result = Some(iterator_result(Value::Null, true));
                    break;
                }
                map.insert(ITER_PAIR_PREV.into(), value);
            }
            result.ok_or_else(|| "pairwise iterator exceeded maximum steps".into())
        }
        "accumulate" => {
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal accumulate iterator missing fn")?;
            let seeded = matches!(map.get(ITER_ACC_SEEDED), Some(Value::Bool(true)));
            if !seeded {
                let first_acc = if let Some(init) = map.get(ITER_ACC_INITIAL) {
                    init.clone()
                } else {
                    let (value, done) = iterator_next(&mut src, env)?;
                    if done {
                        return Ok(iterator_result(Value::Null, true));
                    }
                    value
                };
                map.insert(ITER_ACC.into(), first_acc.clone());
                map.insert(ITER_ACC_SEEDED.into(), Value::Bool(true));
                Ok(iterator_result(first_acc, false))
            } else {
                let acc = map
                    .get(ITER_ACC)
                    .cloned()
                    .ok_or("internal accumulate iterator missing acc")?;
                let (value, done) = iterator_next(&mut src, env)?;
                if done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    let new_acc = call_fn(&func, vec![acc, value], env)?;
                    map.insert(ITER_ACC.into(), new_acc.clone());
                    Ok(iterator_result(new_acc, false))
                }
            }
        }
        "flatmap" => {
            let func = map
                .get(ITER_FN)
                .cloned()
                .ok_or("internal flatmap iterator missing fn")?;
            let mut result = None;
            for _ in 0..4096 {
                if let Some(mut inner) = map.get(ITER_FLAT_INNER).cloned() {
                    let (value, done) = iterator_next(&mut inner, env)?;
                    if !done {
                        map.insert(ITER_FLAT_INNER.into(), inner);
                        result = Some(iterator_result(value, false));
                        break;
                    }
                    map.remove(ITER_FLAT_INNER);
                }
                let (item, done) = iterator_next(&mut src, env)?;
                if done {
                    result = Some(iterator_result(Value::Null, true));
                    break;
                }
                let mapped = call_fn(&func, vec![item], env)?;
                let depth = flatmap_depth(map);
                match flatmap_expand_mapped(mapped, depth, env)? {
                    FlatmapExpand::Iterable(mut inner) => {
                        let (value, inner_done) = iterator_next(&mut inner, env)?;
                        if inner_done {
                            continue;
                        }
                        map.insert(ITER_FLAT_INNER.into(), inner);
                        result = Some(iterator_result(value, false));
                        break;
                    }
                    FlatmapExpand::Scalar(scalar) => {
                        result = Some(iterator_result(scalar, false));
                        break;
                    }
                }
            }
            result.ok_or_else(|| "flatmap iterator exceeded maximum steps".into())
        }
        other => Err(format!("unknown lazy iterator kind: {other}")),
    }?;
    persist_lazy_src(it, &src);
    Ok(result)
}

fn flatmap_depth(map: &HashMap<String, Value>) -> i64 {
    match map.get(ITER_FLAT_DEPTH) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    }
}

pub(crate) fn flatmap_expand_mapped(
    mapped: Value,
    depth: i64,
    env: &mut Environment,
) -> Result<FlatmapExpand, String> {
    if depth > 1 {
        if let Value::Array(items) = mapped {
            let flat = crate::runtime::stdlib::array::flatten_values(&items, depth as usize);
            return Ok(FlatmapExpand::Iterable(create_array_iterator(flat)));
        }
    }
    if is_iterator_value(&mapped)
        || crate::runtime::stdlib::generator::is_generator_object(&mapped)
    {
        return Ok(FlatmapExpand::Iterable(mapped));
    }
    if let Some(it) = builtin_iterator(&mapped) {
        return Ok(FlatmapExpand::Iterable(it));
    }
    if object_with_next_iterator(&mapped).is_some() {
        return Ok(FlatmapExpand::Iterable(get_sync_iterator(&mapped, env)?));
    }
    Ok(FlatmapExpand::Scalar(mapped))
}

pub(crate) enum FlatmapExpand {
    Scalar(Value),
    Iterable(Value),
}

fn slim_iterator_for_lazy_src(v: Value) -> Value {
    strip_iterator_bound_methods(v)
}

pub fn needs_sync_iterator_instance_methods(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            is_iterator_value(v) && !map.contains_key(ITER_METHODS)
        }
        _ => false,
    }
}

pub fn is_sync_instance_method(field: &str) -> bool {
    matches!(
        field,
        "next"
            | "return"
            | "throw"
            | "map"
            | "filter"
            | "take"
            | "skip"
            | "flatMap"
            | "dropWhile"
            | "takeWhile"
            | "pairwise"
            | "accumulate"
            | "zip"
            | "enumerate"
            | "chain"
            | "toArray"
            | "reduce"
            | "some"
            | "every"
            | "forEach"
            | "find"
            | "findIndex"
            | "includes"
    )
}

pub fn ensure_sync_iterator_instance_methods(it: &mut Value) {
    let Value::Object(ref mut map_rc) = it else {
        return;
    };
    let map = Value::object_make_mut(map_rc);
    if map.contains_key(ITER_LAZY) && !map.contains_key("next") {
        attach_lazy_next(map);
    }
    attach_iterator_instance_methods(map);
}

fn create_lazy_iterator(kind: &str, src: Value, extra: HashMap<String, Value>) -> Result<Value, String> {
    if !is_iterator_value(&src) {
        return Err("lazy iterator requires iterator source".into());
    }
    let src = slim_iterator_for_lazy_src(src);
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ITER_LAZY.into(), Value::String(kind.into()));
    m.insert(ITER_SRC.into(), src);
    for (k, v) in extra {
        m.insert(k, v);
    }
    crate::runtime::stdlib::object::object_oid(&mut m);
    Ok(Value::from_object(m))
}

pub fn create_map_iterator_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("map", src, HashMap::from([(ITER_FN.into(), func)]))
}

pub fn create_filter_iterator_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("filter", src, HashMap::from([(ITER_FN.into(), func)]))
}

pub fn create_take_iterator_from_iterable(
    v: &Value,
    n: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("take", src, HashMap::from([(ITER_TAKE.into(), Value::Number(n))]))
}

pub fn create_skip_iterator_from_iterable(
    v: &Value,
    n: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let mut src = get_sync_iterator(v, env)?;
    let mut remaining = n;
    while remaining > 0 {
        let (_, done) = iterator_next(&mut src, env)?;
        if done {
            break;
        }
        remaining -= 1;
    }
    create_lazy_iterator("pass", src, HashMap::new())
}

pub fn create_chain_iterator_from_iterables(
    iterables: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    if iterables.is_empty() {
        return Err("iterator_chain requires at least one iterable".into());
    }
    let first = get_sync_iterator(&iterables[0], env)?;
    create_lazy_iterator(
        "chain",
        first,
        HashMap::from([
            (ITER_CHAIN.into(), Value::from_array(iterables.to_vec())),
            (ITER_CHAIN_IDX.into(), Value::Number(0)),
        ]),
    )
}

pub fn create_zip_iterator_from_iterables(
    a: &Value,
    b: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let left = get_sync_iterator(a, env)?;
    let right = get_sync_iterator(b, env)?;
    create_lazy_iterator(
        "zip",
        left,
        HashMap::from([(ITER_ZIP_RIGHT.into(), right)]),
    )
}

pub fn create_enumerate_iterator_from_iterable(
    v: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator(
        "enumerate",
        src,
        HashMap::from([(ITER_ENUM_IDX.into(), Value::Number(0))]),
    )
}

pub fn create_flat_map_iterator_from_iterable(
    v: &Value,
    func: Value,
    depth: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_flat_map_on_iterator(&src, func, depth)
}

pub fn create_flat_map_on_iterator(
    iter: &Value,
    func: Value,
    depth: i64,
) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "flatmap",
        iter.clone(),
        HashMap::from([
            (ITER_FN.into(), func),
            (ITER_FLAT_DEPTH.into(), Value::Number(depth.max(1))),
        ]),
    )
}

pub fn create_drop_while_iterator_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("dropwhile", src, HashMap::from([(ITER_FN.into(), func)]))
}

pub fn create_take_while_iterator_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("takewhile", src, HashMap::from([(ITER_FN.into(), func)]))
}

pub fn create_pairwise_iterator_from_iterable(
    v: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    create_lazy_iterator("pairwise", src, HashMap::new())
}

pub fn create_accumulate_iterator_from_iterable(
    v: &Value,
    func: Value,
    initial: Option<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_sync_iterator(v, env)?;
    let mut extra = HashMap::from([(ITER_FN.into(), func)]);
    if let Some(init) = initial {
        extra.insert(ITER_ACC_INITIAL.into(), init);
    }
    create_lazy_iterator("accumulate", src, extra)
}

pub fn iterator_from_iterable(v: &Value, env: &mut Environment) -> Result<Value, String> {
    get_sync_iterator(v, env)
}

pub fn iterator_next(it: &mut Value, env: &mut Environment) -> Result<(Value, bool), String> {
    if iterator_is_returned(it) {
        return Ok((Value::Null, true));
    }
    if let Value::Object(map) = it {
        if map.contains_key(ITER_LAZY) {
            let result = lazy_iterator_next(it, env)?;
            return parse_iterator_result(&result);
        }
    }
    if crate::runtime::stdlib::generator::is_generator_object(it) {
        let result = crate::runtime::stdlib::generator::advance_generator(it, None, env)?;
        return parse_iterator_result(&result);
    }
    if let Value::Object(map) = it {
        if map.contains_key(ITER_MAP) {
            let result = advance_map_iterator(it)?;
            return parse_iterator_result(&result);
        }
        if map.contains_key(ITER_SET) && map.contains_key(ITER_ARRAY) {
            let result = advance_array_iterator(it)?;
            return parse_iterator_result(&result);
        }
        if map.contains_key(ITER_ARRAY) {
            let result = advance_array_iterator(it)?;
            return parse_iterator_result(&result);
        }
        if map.contains_key(ITER_RANGE) {
            let result = advance_range_iterator(it)?;
            return parse_iterator_result(&result);
        }
        if map.contains_key("__kab_iter_delegate") {
            let result = delegate_iterator_next(it, env)?;
            return parse_iterator_result(&result);
        }
    }
    if has_next_method(it) {
        let mut owned = wrap_delegate_iterator(it.clone());
        let result = delegate_iterator_next(&mut owned, env)?;
        return parse_iterator_result(&result);
    }
    Err("value is not an iterator".into())
}

/// Plain `{ next() { ... } }` objects (no `Symbol.iterator`).
pub fn object_with_next_iterator(v: &Value) -> Option<Value> {
    if crate::runtime::stdlib::generator::is_generator_object(v) {
        return Some(v.clone());
    }
    if has_next_method(v) {
        Some(wrap_delegate_iterator(v.clone()))
    } else {
        None
    }
}

pub fn normalize_iterator(v: Value) -> Result<Value, String> {
    if is_iterator_value(&v) {
        return Ok(v);
    }
    if let Some(it) = builtin_iterator(&v) {
        return Ok(it);
    }
    if has_next_method(&v) {
        return Ok(wrap_delegate_iterator(v));
    }
    Err("Symbol.iterator must return an iterator object".into())
}

/// Iterator object for built-in iterables (`Array`, `String`, `Map`, `Set`).
pub fn builtin_iterator(v: &Value) -> Option<Value> {
    match v {
        Value::Array(items) => Some(create_array_iterator(items.as_ref().clone())),
        Value::String(s) => Some(create_string_iterator(s)),
        _ if is_map_value(v) => create_map_iterator(v).ok(),
        _ if is_set_value(v) => create_set_iterator(v).ok(),
        Value::Range { start, end, step } => create_range_iterator(*start, *end, *step).ok(),
        _ => None,
    }
}

/// Remaining element count for iterator objects (for `len()`).
pub fn iterator_len(v: &Value) -> Option<usize> {
    if !is_iterator_value(v) {
        return None;
    }
    let Value::Object(map) = v else {
        return None;
    };
    if let Some(Value::String(kind)) = map.get(ITER_LAZY) {
        if kind == "take" {
            if let Some(Value::Number(n)) = map.get(ITER_TAKE) {
                let take = (*n).max(0) as usize;
                if let Some(src) = map.get(ITER_SRC) {
                    if let Some(src_len) = iterator_len(src) {
                        return Some(take.min(src_len));
                    }
                }
                return Some(take);
            }
        }
    }
    if let Some(Value::Array(items)) = map.get(ITER_ARRAY) {
        let idx = match map.get(ITER_INDEX) {
            Some(Value::Number(n)) if *n >= 0 => *n as usize,
            _ => 0,
        };
        return Some(items.len().saturating_sub(idx));
    }
    if let Some(Value::Array(keys)) = map.get(ITER_MAP_KEYS) {
        let idx = match map.get(ITER_INDEX) {
            Some(Value::Number(n)) if *n >= 0 => *n as usize,
            _ => 0,
        };
        return Some(keys.len().saturating_sub(idx));
    }
    if map.contains_key(ITER_RANGE) {
        let cur = range_i64(map, "__kab_range_cur").ok()?;
        let end = range_i64(map, "__kab_range_end").ok()?;
        let step = range_i64(map, "__kab_range_step").ok()?;
        return count_range_remaining(cur, end, step).ok();
    }
    None
}

fn count_range_remaining(cur: i64, end: i64, step: i64) -> Result<usize, String> {
    if step == 0 {
        return Err("range step cannot be 0".into());
    }
    let mut n = 0usize;
    let mut i = cur;
    if step > 0 {
        while i < end {
            n += 1;
            i += step;
        }
    } else {
        while i > end {
            n += 1;
            i += step;
        }
    }
    Ok(n)
}

fn create_string_iterator(s: &str) -> Value {
    let items: Vec<Value> = s
        .chars()
        .map(|c| Value::String(c.to_string()))
        .collect();
    create_array_iterator(items)
}

fn advance_map_iterator(it: &mut Value) -> Result<Value, String> {
    let Value::Object(ref mut map_rc) = it else {
        return Err("iterator.next() expects iterator receiver".into());
    };
    let id = match map_rc.get(ITER_MAP_ID) {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => return Err("internal map iterator missing id".into()),
    };
    let keys = match map_rc.get(ITER_MAP_KEYS) {
        Some(Value::Array(items)) => items.as_ref().clone(),
        _ => return Err("internal map iterator missing keys".into()),
    };
    let idx = match map_rc.get(ITER_INDEX) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    if idx >= keys.len() {
        return Ok(iterator_result(Value::Null, true));
    }
    let Value::String(key) = &keys[idx] else {
        return Err("internal map iterator key must be string".into());
    };
    let value = map_get_at_id(id, key).ok_or_else(|| format!("map key missing: {key}"))?;
    let entry = Value::from_array(vec![Value::String(key.clone()), value]);
    Value::object_make_mut(map_rc).insert(ITER_INDEX.into(), Value::Number((idx + 1) as i64));
    Ok(iterator_result(entry, false))
}

fn map_iterator_next_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator.next()")?.clone();
    advance_map_iterator(&mut it)
}

fn create_map_iterator(v: &Value) -> Result<Value, String> {
    let id = map_id(v)?;
    let keys: Vec<Value> = map_key_list(id)
        .into_iter()
        .map(Value::String)
        .collect();
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ITER_MAP.into(), Value::Bool(true));
    m.insert(ITER_MAP_ID.into(), Value::Number(id as i64));
    m.insert(ITER_MAP_KEYS.into(), Value::from_array(keys));
    m.insert(ITER_INDEX.into(), Value::Number(0));
    crate::runtime::stdlib::object::object_oid(&mut m);
    attach_next_to_map(&mut m, map_iterator_next_native);
    attach_iterator_instance_methods(&mut m);
    Ok(Value::from_object(m))
}

fn create_set_iterator(v: &Value) -> Result<Value, String> {
    let id = set_id(v)?;
    let items = set_values_for_iteration(id);
    let mut m = HashMap::new();
    m.insert(ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ITER_SET.into(), Value::Bool(true));
    m.insert(ITER_ARRAY.into(), Value::from_array(items));
    m.insert(ITER_INDEX.into(), Value::Number(0));
    crate::runtime::stdlib::object::object_oid(&mut m);
    Ok(attach_next_native(&mut m))
}

/// `value[Symbol.iterator]` — bound factory returning a fresh iterator.
pub fn symbol_iterator_method(v: &Value) -> Option<Value> {
    if builtin_iterator(v).is_some()
        || is_iterator_value(v)
        || crate::runtime::stdlib::generator::is_generator_object(v)
    {
        Some(Value::BoundNative(
            Box::new(v.clone()),
            symbol_iterator_call_native,
        ))
    } else {
        None
    }
}

fn symbol_iterator_call_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let recv = args.first().ok_or("Symbol.iterator()")?;
    if crate::runtime::stdlib::generator::is_generator_object(recv) || is_iterator_value(recv) {
        return Ok(recv.clone());
    }
    builtin_iterator(recv).ok_or_else(|| "value is not iterable".into())
}

pub fn create_map_on_iterator(iter: &Value, func: Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "map",
        iter.clone(),
        HashMap::from([(ITER_FN.into(), func)]),
    )
}

pub fn create_filter_on_iterator(iter: &Value, func: Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "filter",
        iter.clone(),
        HashMap::from([(ITER_FN.into(), func)]),
    )
}

pub fn create_take_on_iterator(iter: &Value, n: i64) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "take",
        iter.clone(),
        HashMap::from([(ITER_TAKE.into(), Value::Number(n))]),
    )
}

pub fn create_skip_on_iterator(
    iter: &Value,
    n: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    let mut src = iter.clone();
    let mut remaining = n;
    while remaining > 0 {
        let (_, done) = iterator_next(&mut src, env)?;
        if done {
            break;
        }
        remaining -= 1;
    }
    create_lazy_iterator("pass", src, HashMap::new())
}

pub fn create_drop_while_on_iterator(iter: &Value, func: Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "dropwhile",
        iter.clone(),
        HashMap::from([(ITER_FN.into(), func)]),
    )
}

pub fn create_take_while_on_iterator(iter: &Value, func: Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "takewhile",
        iter.clone(),
        HashMap::from([(ITER_FN.into(), func)]),
    )
}

pub fn create_pairwise_on_iterator(iter: &Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator("pairwise", iter.clone(), HashMap::new())
}

pub fn create_accumulate_on_iterator(
    iter: &Value,
    func: Value,
    initial: Option<Value>,
) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    let mut extra = HashMap::from([(ITER_FN.into(), func)]);
    if let Some(init) = initial {
        extra.insert(ITER_ACC_INITIAL.into(), init);
    }
    create_lazy_iterator("accumulate", iter.clone(), extra)
}

pub fn create_zip_on_iterator(
    iter: &Value,
    other: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    let right = get_sync_iterator(other, env)?;
    create_lazy_iterator(
        "zip",
        iter.clone(),
        HashMap::from([(ITER_ZIP_RIGHT.into(), right)]),
    )
}

pub fn create_enumerate_on_iterator(iter: &Value) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    create_lazy_iterator(
        "enumerate",
        iter.clone(),
        HashMap::from([(ITER_ENUM_IDX.into(), Value::Number(0))]),
    )
}

pub fn create_chain_on_iterator(iter: &Value, rest: &[Value]) -> Result<Value, String> {
    if !is_iterator_value(iter) {
        return Err("Iterator method expects an iterator receiver".into());
    }
    if rest.is_empty() {
        return Err("Iterator.chain(iterable, ...) expects at least one iterable".into());
    }
    let mut chain_items = vec![Value::Null];
    chain_items.extend(rest.iter().cloned());
    create_lazy_iterator(
        "chain",
        iter.clone(),
        HashMap::from([
            (ITER_CHAIN.into(), Value::from_array(chain_items)),
            (ITER_CHAIN_IDX.into(), Value::Number(0)),
        ]),
    )
}

pub fn iterator_reduce(
    it: &mut Value,
    func: Value,
    initial: Option<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let mut acc = match initial {
        Some(v) => v,
        None => {
            let (first, done) = iterator_next(it, env)?;
            if done {
                return Err("reduce of empty iterator with no initial value".into());
            }
            first
        }
    };
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        acc = call_fn(&func, vec![acc, value], env)?;
    }
    Ok(acc)
}

pub fn iterator_some(it: &mut Value, func: Value, env: &mut Environment) -> Result<Value, String> {
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        if call_fn(&func, vec![value], env)?.is_truthy() {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

pub fn iterator_every(it: &mut Value, func: Value, env: &mut Environment) -> Result<Value, String> {
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        if !call_fn(&func, vec![value], env)?.is_truthy() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

pub fn iterator_return(
    it: &mut Value,
    value: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(it) {
        return crate::runtime::stdlib::generator::return_generator(it, value.clone(), env);
    }
    if let Value::Object(ref mut map_rc) = it {
        if map_rc.contains_key("__kab_iter_delegate") {
            let mut target = map_rc
                .get("__kab_iter_delegate")
                .cloned()
                .ok_or("internal delegate iterator missing target")?;
            let result = delegate_iterator_return(&mut target, value, env)?;
            Value::object_make_mut(map_rc).insert("__kab_iter_delegate".into(), target);
            iterator_mark_returned(it);
            writeback_iterator_by_oid(it, env);
            return Ok(result);
        }
    }
    iterator_mark_returned(it);
    writeback_iterator_by_oid(it, env);
    Ok(iterator_result(value, true))
}

pub fn iterator_throw(
    it: &mut Value,
    reason: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(it) {
        return crate::runtime::stdlib::generator::throw_generator(it, reason, env);
    }
    if let Value::Object(ref mut map_rc) = it {
        if map_rc.contains_key("__kab_iter_delegate") {
            let mut target = map_rc
                .get("__kab_iter_delegate")
                .cloned()
                .ok_or("internal delegate iterator missing target")?;
            let result = delegate_iterator_throw(&mut target, reason, env)?;
            Value::object_make_mut(map_rc).insert("__kab_iter_delegate".into(), target);
            iterator_mark_returned(it);
            writeback_iterator_by_oid(it, env);
            return Ok(result);
        }
    }
    iterator_mark_returned(it);
    writeback_iterator_by_oid(it, env);
    Ok(iterator_result(reason, true))
}

pub fn iterator_for_each(
    it: &mut Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        call_fn(&func, vec![value], env)?;
    }
    Ok(Value::Null)
}

pub fn iterator_find(
    it: &mut Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
            return Ok(value);
        }
    }
    Ok(Value::Undefined)
}

pub fn iterator_find_index(
    it: &mut Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let mut index = 0i64;
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        if call_fn(&func, vec![value], env)?.is_truthy() {
            return Ok(Value::Number(index));
        }
        index += 1;
    }
    Ok(Value::Number(-1))
}

pub fn iterator_includes(
    it: &mut Value,
    needle: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    loop {
        let (value, done) = iterator_next(it, env)?;
        if done {
            break;
        }
        if crate::ops::values_equal(&value, &needle) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn iterator_inst_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.map(fn)")?;
    let func = args.get(1).ok_or("Iterator.map(fn)")?;
    create_map_on_iterator(iter, func.clone())
}

fn iterator_inst_filter_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.filter(fn)")?;
    let func = args.get(1).ok_or("Iterator.filter(fn)")?;
    create_filter_on_iterator(iter, func.clone())
}

fn iterator_inst_take_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.take(n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("Iterator.take(n) expects non-negative number".into()),
    };
    create_take_on_iterator(iter, n)
}

fn iterator_inst_skip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.skip(n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("Iterator.skip(n) expects non-negative number".into()),
    };
    create_skip_on_iterator(iter, n, env)
}

fn flatmap_depth_from_args(args: &[Value]) -> i64 {
    match args.get(2) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    }
}

fn iterator_inst_flat_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.flatMap(fn, depth?)")?;
    let func = args.get(1).ok_or("Iterator.flatMap(fn, depth?)")?;
    create_flat_map_on_iterator(iter, func.clone(), flatmap_depth_from_args(args))
}

fn iterator_inst_drop_while_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.dropWhile(fn)")?;
    let func = args.get(1).ok_or("Iterator.dropWhile(fn)")?;
    create_drop_while_on_iterator(iter, func.clone())
}

fn iterator_inst_take_while_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.takeWhile(fn)")?;
    let func = args.get(1).ok_or("Iterator.takeWhile(fn)")?;
    create_take_while_on_iterator(iter, func.clone())
}

fn iterator_inst_pairwise_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.pairwise()")?;
    create_pairwise_on_iterator(iter)
}

fn iterator_inst_accumulate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.accumulate(fn, initial?)")?;
    let func = args.get(1).ok_or("Iterator.accumulate(fn, initial?)")?;
    let initial = args.get(2).cloned();
    create_accumulate_on_iterator(iter, func.clone(), initial)
}

fn iterator_inst_zip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.zip(iterable)")?;
    let other = args.get(1).ok_or("Iterator.zip(iterable)")?;
    create_zip_on_iterator(iter, other, env)
}

fn iterator_inst_enumerate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.enumerate()")?;
    create_enumerate_on_iterator(iter)
}

fn iterator_inst_chain_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("Iterator.chain(iterable, ...)")?;
    if args.len() < 2 {
        return Err("Iterator.chain(iterable, ...) expects at least one iterable".into());
    }
    create_chain_on_iterator(iter, &args[1..])
}

fn iterator_inst_to_array_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.toArray()")?.clone();
    Ok(Value::from_array(iterator_collect(&mut iter, env)?))
}

fn iterator_inst_reduce_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.reduce(fn, initial?)")?.clone();
    let func = args.get(1).ok_or("Iterator.reduce(fn, initial?)")?.clone();
    let initial = args.get(2).cloned();
    iterator_reduce(&mut iter, func, initial, env)
}

fn iterator_inst_some_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.some(fn)")?.clone();
    let func = args.get(1).ok_or("Iterator.some(fn)")?.clone();
    iterator_some(&mut iter, func, env)
}

fn iterator_inst_every_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.every(fn)")?.clone();
    let func = args.get(1).ok_or("Iterator.every(fn)")?.clone();
    iterator_every(&mut iter, func, env)
}

fn iterator_inst_return_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.return(value?)")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut iter, env);
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    iterator_return(&mut iter, value, env)
}

fn iterator_inst_throw_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.throw(reason?)")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut iter, env);
    let reason = args.get(1).cloned().unwrap_or(Value::Null);
    let result = iterator_throw(&mut iter, reason, env)?;
    writeback_iterator_by_oid(&iter, env);
    if crate::runtime::stdlib::generator::is_generator_object(&iter) {
        crate::runtime::stdlib::object::writeback_generator_by_oid(&iter, env);
    }
    Ok(result)
}

fn iterator_inst_for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.forEach(fn)")?.clone();
    let func = args.get(1).ok_or("Iterator.forEach(fn)")?.clone();
    iterator_for_each(&mut iter, func, env)
}

fn iterator_inst_find_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.find(fn)")?.clone();
    let func = args.get(1).ok_or("Iterator.find(fn)")?.clone();
    iterator_find(&mut iter, func, env)
}

fn iterator_inst_find_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.findIndex(fn)")?.clone();
    let func = args.get(1).ok_or("Iterator.findIndex(fn)")?.clone();
    iterator_find_index(&mut iter, func, env)
}

fn iterator_inst_includes_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("Iterator.includes(value)")?.clone();
    let needle = args.get(1).ok_or("Iterator.includes(value)")?.clone();
    iterator_includes(&mut iter, needle, env)
}

pub fn attach_iterator_return(map: &mut HashMap<String, Value>) {
    if map.contains_key("return") {
        return;
    }
    attach_bound_method(map, "return", iterator_inst_return_native);
}

pub fn attach_iterator_throw(map: &mut HashMap<String, Value>) {
    if map.contains_key("throw") {
        return;
    }
    attach_bound_method(map, "throw", iterator_inst_throw_native);
}

pub fn attach_iterator_instance_methods(map: &mut HashMap<String, Value>) {
    if map.contains_key(ITER_METHODS) {
        return;
    }
    map.insert(ITER_METHODS.into(), Value::Bool(true));
    attach_bound_method(map, "map", iterator_inst_map_native);
    attach_bound_method(map, "filter", iterator_inst_filter_native);
    attach_bound_method(map, "take", iterator_inst_take_native);
    attach_bound_method(map, "skip", iterator_inst_skip_native);
    attach_bound_method(map, "flatMap", iterator_inst_flat_map_native);
    attach_bound_method(map, "dropWhile", iterator_inst_drop_while_native);
    attach_bound_method(map, "takeWhile", iterator_inst_take_while_native);
    attach_bound_method(map, "pairwise", iterator_inst_pairwise_native);
    attach_bound_method(map, "accumulate", iterator_inst_accumulate_native);
    attach_bound_method(map, "zip", iterator_inst_zip_native);
    attach_bound_method(map, "enumerate", iterator_inst_enumerate_native);
    attach_bound_method(map, "chain", iterator_inst_chain_native);
    attach_bound_method(map, "toArray", iterator_inst_to_array_native);
    attach_bound_method(map, "reduce", iterator_inst_reduce_native);
    attach_bound_method(map, "some", iterator_inst_some_native);
    attach_bound_method(map, "every", iterator_inst_every_native);
    attach_bound_method(map, "forEach", iterator_inst_for_each_native);
    attach_bound_method(map, "find", iterator_inst_find_native);
    attach_bound_method(map, "findIndex", iterator_inst_find_index_native);
    attach_bound_method(map, "includes", iterator_inst_includes_native);
    attach_iterator_return(map);
    attach_iterator_throw(map);
}

#[cfg(test)]
mod iterator_leak_tests {
    use super::*;
    use crate::evaluator::{create_global_env, eval_source};
    use crate::value::Value;

    #[test]
    fn array_iterator_three_values() {
        let mut env = create_global_env();
        let mut it = create_array_iterator(vec![
            Value::Number(1),
            Value::Number(2),
            Value::Number(3),
        ]);
        let mut sum = 0i64;
        for _ in 0..5 {
            let (val, done) = iterator_next(&mut it, &mut env).unwrap();
            if done {
                break;
            }
            if let Value::Number(n) = val {
                sum += n;
            }
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn generator_three_yields_via_next() {
        let mut env = create_global_env();
        eval_source(
            r#"fn* g() {
              yield 1
              yield 2
              yield 3
            }"#,
            &mut env,
        )
        .unwrap();
        let mut it = eval_source("g()", &mut env).unwrap();
        let mut sum = 0i64;
        for _ in 0..5 {
            let (val, done) = iterator_next(&mut it, &mut env).unwrap();
            if done {
                break;
            }
            if let Value::Number(n) = val {
                sum += n;
            }
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn lazy_take_over_range_three() {
        let mut env = create_global_env();
        let out = eval_source(
            r#"
            let sum = 0
            for x of iterator_take(range(1, 100), 3) { sum = sum + x }
            sum
            "#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(out, Value::Number(6)));
    }
}
