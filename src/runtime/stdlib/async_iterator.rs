//! ECMAScript async iteration — `Symbol.asyncIterator` and `for await...of`.

use crate::evaluator::resolve_await_value;
use crate::runtime::stdlib::descriptor::is_callable_value;
use crate::runtime::stdlib::iterator::{attach_next_to_map, iterator_next, iterator_result, normalize_iterator,
    parse_iterator_result};
use crate::value::{Environment, PromiseValue, Value};

fn call_fn(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    let mut callee = func.clone();
    crate::runtime::closure_sync::call_with_closure_sync(&mut callee, args, env, None)
}
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const SYMBOL_ASYNC_ITERATOR: u64 = 2;

pub const ASYNC_ITERATOR_MARKER: &str = "__kab_async_iterator";
pub(crate) const ASYNC_SYNC_DELEGATE: &str = "__kab_async_sync_delegate";
const ASYNC_ITER_LAZY: &str = "__kab_async_iter_lazy";
const ASYNC_ITER_SRC: &str = "__kab_async_iter_src";
const ASYNC_ITER_FN: &str = "__kab_async_iter_fn";
const ASYNC_ITER_TAKE: &str = "__kab_async_iter_take";
const ASYNC_ITER_FLAT_INNER: &str = "__kab_async_iter_flat_inner";
const ASYNC_ITER_FLAT_DEPTH: &str = "__kab_async_iter_flat_depth";
const ASYNC_ITER_DROP_DONE: &str = "__kab_async_iter_drop_done";
const ASYNC_ITER_TAKE_DONE: &str = "__kab_async_iter_take_done";
const ASYNC_ITER_RETURNED: &str = "__kab_async_iter_returned";
const ASYNC_ITER_CHAIN: &str = "__kab_async_iter_chain";
const ASYNC_ITER_CHAIN_IDX: &str = "__kab_async_iter_chain_idx";
const ASYNC_ITER_ZIP_RIGHT: &str = "__kab_async_iter_zip_right";
const ASYNC_ITER_ENUM_IDX: &str = "__kab_async_iter_enum_idx";
const ASYNC_ITER_PAIR_PREV: &str = "__kab_async_iter_pair_prev";
const ASYNC_ITER_ACC: &str = "__kab_async_iter_acc";
const ASYNC_ITER_ACC_INITIAL: &str = "__kab_async_iter_acc_initial";
const ASYNC_ITER_ACC_SEEDED: &str = "__kab_async_iter_acc_seeded";

pub fn is_async_iterator_value(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(ASYNC_ITERATOR_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

fn resolved_promise(value: Value) -> Value {
    Value::Promise(Rc::new(RefCell::new(PromiseValue::Resolved(value))))
}

fn attach_async_next(map: &mut HashMap<String, Value>) {
    attach_next_to_map(map, async_iterator_next_native);
}

fn create_async_lazy_iterator(
    kind: &str,
    src: Value,
    extra: HashMap<String, Value>,
) -> Value {
    let mut m = HashMap::new();
    m.insert(ASYNC_ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(ASYNC_ITER_LAZY.into(), Value::String(kind.into()));
    m.insert(ASYNC_ITER_SRC.into(), src);
    for (k, v) in extra {
        m.insert(k, v);
    }
    crate::runtime::stdlib::object::object_oid(&mut m);
    attach_async_next(&mut m);
    attach_async_iterator_instance_methods(&mut m);
    Value::Object(m)
}

fn async_flatmap_depth(map: &HashMap<String, Value>) -> i64 {
    match map.get(ASYNC_ITER_FLAT_DEPTH) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    }
}

fn async_step_source(src: &mut Value, env: &mut Environment) -> Result<(Value, bool), String> {
    let next_p = async_iterator_next(src, env)?;
    let result = resolve_await_value(next_p, env)?;
    parse_iterator_result(&result)
}

fn async_step_inner(inner: &mut Value, env: &mut Environment) -> Result<(Value, bool), String> {
    if crate::runtime::stdlib::iterator::is_iterator_value(inner)
        && !is_async_iterator_value(inner)
    {
        iterator_next(inner, env)
    } else {
        async_step_source(inner, env)
    }
}

fn async_lazy_iterator_next(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    let Value::Object(map) = it else {
        return Err("asyncIterator.next() expects async iterator object".into());
    };
    if matches!(map.get(ASYNC_ITER_RETURNED), Some(Value::Bool(true))) {
        return Ok(resolved_promise(iterator_result(Value::Null, true)));
    }
    let kind = map
        .get(ASYNC_ITER_LAZY)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .ok_or("internal async lazy iterator missing kind")?
        .to_string();
    let mut src = map
        .get(ASYNC_ITER_SRC)
        .cloned()
        .ok_or("internal async lazy iterator missing source")?;
    let mut map = map.clone();
    let result = match kind.as_str() {
        "map" => {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async map iterator missing fn")?;
            loop {
                let (value, done) = async_step_source(&mut src, env)?;
                if done {
                    break Ok(iterator_result(Value::Null, true));
                }
                let mapped = call_fn(&func, vec![value], env)?;
                break Ok(iterator_result(mapped, false));
            }
        }
        "filter" => {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async filter iterator missing fn")?;
            let mut out = None;
            for _ in 0..4096 {
                let (value, done) = async_step_source(&mut src, env)?;
                if done {
                    out = Some(iterator_result(Value::Null, true));
                    break;
                }
                if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                    out = Some(iterator_result(value, false));
                    break;
                }
            }
            out.ok_or_else(|| "async filter iterator exceeded maximum steps".into())
        }
        "take" => match map.get(ASYNC_ITER_TAKE) {
            Some(Value::Number(n)) if *n > 0 => {
                let (value, done) = async_step_source(&mut src, env)?;
                map.insert(ASYNC_ITER_TAKE.into(), Value::Number(n - 1));
                if done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    Ok(iterator_result(value, false))
                }
            }
            _ => Ok(iterator_result(Value::Null, true)),
        },
        "pass" => {
            let (value, done) = async_step_source(&mut src, env)?;
            Ok(iterator_result(value, done))
        }
        "dropwhile" => {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async dropWhile iterator missing fn")?;
            if matches!(map.get(ASYNC_ITER_DROP_DONE), Some(Value::Bool(true))) {
                let (value, done) = async_step_source(&mut src, env)?;
                Ok(iterator_result(value, done))
            } else {
                let mut out = None;
                for _ in 0..4096 {
                    let (value, done) = async_step_source(&mut src, env)?;
                    if done {
                        out = Some(iterator_result(Value::Null, true));
                        break;
                    }
                    if !call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                        map.insert(ASYNC_ITER_DROP_DONE.into(), Value::Bool(true));
                        out = Some(iterator_result(value, false));
                        break;
                    }
                }
                out.ok_or_else(|| "async dropWhile iterator exceeded maximum steps".into())
            }
        }
        "takewhile" => {
            if matches!(map.get(ASYNC_ITER_TAKE_DONE), Some(Value::Bool(true))) {
                Ok(iterator_result(Value::Null, true))
            } else {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async takeWhile iterator missing fn")?;
            let (value, done) = async_step_source(&mut src, env)?;
            if done {
                Ok(iterator_result(Value::Null, true))
            } else if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
                Ok(iterator_result(value, false))
            } else {
                map.insert(ASYNC_ITER_TAKE_DONE.into(), Value::Bool(true));
                Ok(iterator_result(Value::Null, true))
            }
            }
        }
        "chain" => {
            let chain = match map.get(ASYNC_ITER_CHAIN) {
                Some(Value::Array(items)) => items.clone(),
                _ => return Err("internal async chain iterator missing iterables".into()),
            };
            let mut idx = match map.get(ASYNC_ITER_CHAIN_IDX) {
                Some(Value::Number(n)) if *n >= 0 => *n as usize,
                _ => 0,
            };
            let mut out = None;
            for _ in 0..4096 {
                let (value, done) = async_step_source(&mut src, env)?;
                if !done {
                    map.insert(ASYNC_ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
                    out = Some(iterator_result(value, false));
                    break;
                }
                if idx + 1 >= chain.len() {
                    map.insert(ASYNC_ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
                    out = Some(iterator_result(Value::Null, true));
                    break;
                }
                idx += 1;
                src = get_async_iterator(&chain[idx], env)?;
                map.insert(ASYNC_ITER_CHAIN_IDX.into(), Value::Number(idx as i64));
            }
            out.ok_or_else(|| "async chain iterator exceeded maximum steps".into())
        }
        "zip" => {
            let mut right = map
                .get(ASYNC_ITER_ZIP_RIGHT)
                .cloned()
                .ok_or("internal async zip iterator missing right source")?;
            let (left_val, left_done) = async_step_source(&mut src, env)?;
            if left_done {
                Ok(iterator_result(Value::Null, true))
            } else {
                let (right_val, right_done) = async_step_source(&mut right, env)?;
                if right_done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    map.insert(ASYNC_ITER_ZIP_RIGHT.into(), right);
                    Ok(iterator_result(
                        Value::Array(vec![left_val, right_val]),
                        false,
                    ))
                }
            }
        }
        "enumerate" => {
            let idx = match map.get(ASYNC_ITER_ENUM_IDX) {
                Some(Value::Number(n)) if *n >= 0 => *n,
                _ => 0,
            };
            let (value, done) = async_step_source(&mut src, env)?;
            if done {
                Ok(iterator_result(Value::Null, true))
            } else {
                let pair = Value::Array(vec![Value::Number(idx), value]);
                map.insert(ASYNC_ITER_ENUM_IDX.into(), Value::Number(idx + 1));
                Ok(iterator_result(pair, false))
            }
        }
        "pairwise" => {
            let mut out = None;
            for _ in 0..4096 {
                if let Some(prev) = map.get(ASYNC_ITER_PAIR_PREV).cloned() {
                    let (value, done) = async_step_source(&mut src, env)?;
                    if done {
                        map.remove(ASYNC_ITER_PAIR_PREV);
                        out = Some(iterator_result(Value::Null, true));
                        break;
                    }
                    map.insert(ASYNC_ITER_PAIR_PREV.into(), value.clone());
                    out = Some(iterator_result(
                        Value::Array(vec![prev, value]),
                        false,
                    ));
                    break;
                }
                let (value, done) = async_step_source(&mut src, env)?;
                if done {
                    out = Some(iterator_result(Value::Null, true));
                    break;
                }
                map.insert(ASYNC_ITER_PAIR_PREV.into(), value);
            }
            out.ok_or_else(|| "async pairwise iterator exceeded maximum steps".into())
        }
        "accumulate" => {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async accumulate iterator missing fn")?;
            let seeded = matches!(map.get(ASYNC_ITER_ACC_SEEDED), Some(Value::Bool(true)));
            if !seeded {
                let first_acc = if let Some(init) = map.get(ASYNC_ITER_ACC_INITIAL) {
                    init.clone()
                } else {
                    let (value, done) = async_step_source(&mut src, env)?;
                    if done {
                        return Ok(iterator_result(Value::Null, true));
                    }
                    value
                };
                map.insert(ASYNC_ITER_ACC.into(), first_acc.clone());
                map.insert(ASYNC_ITER_ACC_SEEDED.into(), Value::Bool(true));
                Ok(iterator_result(first_acc, false))
            } else {
                let acc = map
                    .get(ASYNC_ITER_ACC)
                    .cloned()
                    .ok_or("internal async accumulate iterator missing acc")?;
                let (value, done) = async_step_source(&mut src, env)?;
                if done {
                    Ok(iterator_result(Value::Null, true))
                } else {
                    let new_acc = call_fn(&func, vec![acc, value], env)?;
                    map.insert(ASYNC_ITER_ACC.into(), new_acc.clone());
                    Ok(iterator_result(new_acc, false))
                }
            }
        }
        "flatmap" => {
            let func = map
                .get(ASYNC_ITER_FN)
                .cloned()
                .ok_or("internal async flatmap iterator missing fn")?;
            let depth = async_flatmap_depth(&map);
            let mut out = None;
            for _ in 0..4096 {
                if let Some(mut inner) = map.get(ASYNC_ITER_FLAT_INNER).cloned() {
                    let (value, done) = async_step_inner(&mut inner, env)?;
                    if !done {
                        map.insert(ASYNC_ITER_FLAT_INNER.into(), inner);
                        out = Some(iterator_result(value, false));
                        break;
                    }
                    map.remove(ASYNC_ITER_FLAT_INNER);
                }
                let (item, done) = async_step_source(&mut src, env)?;
                if done {
                    out = Some(iterator_result(Value::Null, true));
                    break;
                }
                let mapped = call_fn(&func, vec![item], env)?;
                match crate::runtime::stdlib::iterator::flatmap_expand_mapped(mapped, depth, env)? {
                    crate::runtime::stdlib::iterator::FlatmapExpand::Iterable(mut inner) => {
                        let (value, inner_done) = async_step_inner(&mut inner, env)?;
                        if inner_done {
                            continue;
                        }
                        map.insert(ASYNC_ITER_FLAT_INNER.into(), inner);
                        out = Some(iterator_result(value, false));
                        break;
                    }
                    crate::runtime::stdlib::iterator::FlatmapExpand::Scalar(scalar) => {
                        out = Some(iterator_result(scalar, false));
                        break;
                    }
                }
            }
            out.ok_or_else(|| "async flatmap iterator exceeded maximum steps".into())
        }
        other => Err(format!("unknown async lazy iterator kind: {other}")),
    }?;
    map.insert(ASYNC_ITER_SRC.into(), src);
    attach_async_next(&mut map);
    *it = Value::Object(map);
    Ok(resolved_promise(result))
}

pub fn wrap_async_from_sync(sync_iter: Value) -> Value {
    let mut m = HashMap::new();
    m.insert(ASYNC_ITERATOR_MARKER.into(), Value::Bool(true));
    m.insert(
        ASYNC_SYNC_DELEGATE.into(),
        crate::runtime::stdlib::iterator::strip_iterator_bound_methods(sync_iter),
    );
    crate::runtime::stdlib::object::object_oid(&mut m);
    attach_async_next(&mut m);
    // Instance methods attach lazily on first property read (see opt::get_member_value).
    Value::Object(m)
}

pub fn normalize_async_iterator(v: Value, _env: &mut Environment) -> Result<Value, String> {
    if is_async_iterator_value(&v) {
        return Ok(v);
    }
    if let Value::Object(m) = &v {
        if m.get("next").is_some_and(is_callable_value) {
            let mut out = m.clone();
            out.insert(ASYNC_ITERATOR_MARKER.into(), Value::Bool(true));
            attach_async_iterator_instance_methods(&mut out);
            return Ok(Value::Object(out));
        }
    }
    let sync = normalize_iterator(v)?;
    Ok(wrap_async_from_sync(sync))
}

pub fn get_async_iterator(v: &Value, env: &mut Environment) -> Result<Value, String> {
    let owned_target;
    let effective: &Value = if let Some(target) =
        crate::runtime::stdlib::proxy::proxy_target_for_iteration(v)
    {
        owned_target = target;
        &owned_target
    } else {
        v
    };
    if is_async_iterator_value(effective) {
        return Ok(effective.clone());
    }
    if crate::runtime::stdlib::generator::is_async_generator_object(effective) {
        return Ok(effective.clone());
    }
    if crate::runtime::stdlib::iterator::is_iterator_value(effective) {
        return Ok(wrap_async_from_sync(effective.clone()));
    }
    if let Value::Object(map) = effective {
        if let Ok(Some(method)) = crate::runtime::stdlib::descriptor::get_own_symbol(
            map,
            SYMBOL_ASYNC_ITERATOR,
            effective,
            env,
        ) {
            let result = match call_fn(&method, vec![], env) {
                Ok(val) => val,
                Err(_) => call_fn(&method, vec![effective.clone()], env)?,
            };
            return normalize_async_iterator(result, env);
        }
    }
    if let Some(sync) = crate::runtime::stdlib::iterator::builtin_iterator(effective) {
        return Ok(wrap_async_from_sync(sync));
    }
    if let Some(sync) = crate::runtime::stdlib::iterator::object_with_next_iterator(effective) {
        return Ok(wrap_async_from_sync(sync));
    }
    if let Value::Object(map) = effective {
        if let Ok(Some(method)) = crate::runtime::stdlib::descriptor::get_own_symbol(
            map,
            crate::runtime::stdlib::iterator::SYMBOL_ITERATOR,
            effective,
            env,
        ) {
            let result = match call_fn(&method, vec![], env) {
                Ok(val) => val,
                Err(_) => call_fn(&method, vec![effective.clone()], env)?,
            };
            let sync = normalize_iterator(result)?;
            return Ok(wrap_async_from_sync(sync));
        }
    }
    Err("value is not async iterable".into())
}

pub fn async_iterator_next(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    let Value::Object(src) = it else {
        return Err("asyncIterator.next() expects async iterator object".into());
    };
    if src.contains_key(ASYNC_ITER_LAZY) {
        return async_lazy_iterator_next(it, env);
    }
    let mut map = src.clone();
    if map.contains_key(ASYNC_SYNC_DELEGATE) {
        let mut sync = map
            .get(ASYNC_SYNC_DELEGATE)
            .cloned()
            .ok_or("internal async iterator missing sync delegate")?;
        let (value, done) = iterator_next(&mut sync, env)?;
        map.insert(ASYNC_SYNC_DELEGATE.into(), sync);
        attach_async_next(&mut map);
        *it = Value::Object(map);
        return Ok(resolved_promise(iterator_result(value, done)));
    }
    if crate::runtime::stdlib::generator::is_async_generator_object(it) {
        return crate::runtime::stdlib::generator::advance_async_generator_next(it, None, env);
    }
    let next_fn = map
        .get("next")
        .cloned()
        .ok_or("async iterator object missing next()")?;
    let (result, updated) = crate::runtime::stdlib::iterator::call_delegate_next(
        next_fn,
        it.clone(),
        env,
    )?;
    *it = updated;
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(it, env);
    match result {
        Value::Promise(p) => Ok(Value::Promise(p)),
        other => Ok(resolved_promise(other)),
    }
}

fn async_iterator_next_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("asyncIterator.next()")?.clone();
    async_iterator_next(&mut it, env)
}

pub fn async_iterator_collect(it: &mut Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for _ in 0..4096 {
        let next_promise = async_iterator_next(it, env)?;
        let result = resolve_await_value(next_promise, env)?;
        let (value, done) = parse_iterator_result(&result)?;
        if done {
            break;
        }
        out.push(value);
    }
    if out.len() >= 4096 {
        return Err("async iterator exceeded maximum steps".into());
    }
    Ok(out)
}

/// Advance async iterator; returns a Promise of `{ value, done }`.
pub fn async_iterator_step(it: &mut Value, env: &mut Environment) -> Result<Value, String> {
    async_iterator_next(it, env)
}

fn async_iterator_step_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("asyncIterator.next()")?.clone();
    async_iterator_step(&mut it, env)
}

/// Materialize `for await (x of iterable)` as an array (async fn context).
pub fn for_await_of_items_with_env(v: &Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    let mut iter = get_async_iterator(v, env)?;
    async_iterator_collect(&mut iter, env)
}

/// `value[Symbol.asyncIterator]` for built-in iterables.
pub fn symbol_async_iterator_method(v: &Value) -> Option<Value> {
    if is_async_iterator_value(v)
        || crate::runtime::stdlib::generator::is_async_generator_object(v)
        || crate::runtime::stdlib::iterator::builtin_iterator(v).is_some()
    {
        Some(Value::BoundNative(
            Box::new(v.clone()),
            symbol_async_iterator_call_native,
        ))
    } else {
        None
    }
}

fn symbol_async_iterator_call_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let recv = args.first().ok_or("Symbol.asyncIterator()")?;
    if crate::runtime::stdlib::generator::is_async_generator_object(recv) {
        return Ok(recv.clone());
    }
    if is_async_iterator_value(recv) {
        return Ok(recv.clone());
    }
    let sync = crate::runtime::stdlib::iterator::builtin_iterator(recv)
        .ok_or_else(|| "value is not async iterable".to_string())?;
    Ok(wrap_async_from_sync(sync))
}

const ASYNC_ITER_METHODS: &str = "__kab_async_iter_methods";

const ASYNC_INSTANCE_METHODS: &[&str] = &[
    "map",
    "filter",
    "take",
    "skip",
    "flatMap",
    "dropWhile",
    "takeWhile",
    "pairwise",
    "accumulate",
    "zip",
    "enumerate",
    "chain",
    "toArray",
    "reduce",
    "some",
    "every",
    "forEach",
    "find",
    "findIndex",
    "includes",
    "return",
    "throw",
];

pub fn is_async_instance_method(field: &str) -> bool {
    ASYNC_INSTANCE_METHODS.contains(&field)
}

pub fn ensure_async_iterator_instance_methods(it: &mut Value) {
    let Value::Object(map) = it else {
        return;
    };
    attach_async_iterator_instance_methods(map);
}

pub fn needs_async_instance_methods(it: &Value) -> bool {
    match it {
        Value::Object(map) => {
            is_async_iterator_value(it) && !map.contains_key(ASYNC_ITER_METHODS)
        }
        _ => false,
    }
}

fn async_inner_sync(iter: &Value) -> Result<Value, String> {
    let Value::Object(m) = iter else {
        return Err("AsyncIterator method expects async iterator receiver".into());
    };
    m.get(ASYNC_SYNC_DELEGATE)
        .cloned()
        .ok_or_else(|| "AsyncIterator adapter requires sync delegate".into())
}

fn async_call_object_method(
    it: &mut Value,
    method: &str,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    let Value::Object(map) = it else {
        return Err("async iterator expects object receiver".into());
    };
    let Some(method_fn) = map.get(method).filter(|f| is_callable_value(f)).cloned() else {
        return Ok(None);
    };
    let result = if crate::runtime::stdlib::iterator::next_fn_uses_param(&method_fn) {
        let mut callee = method_fn;
        crate::runtime::closure_sync::call_with_closure_sync(
            &mut callee,
            args,
            env,
            Some(it),
        )?
    } else {
        let (result, updated) = crate::runtime::stdlib::object::call_object_method(
            method_fn,
            args,
            it.clone(),
            env,
        )?;
        *it = updated;
        result
    };
    Ok(Some(resolve_await_value(result, env)?))
}

fn mark_async_iterator_returned(it: &mut Value) {
    if let Value::Object(map) = it {
        map.insert(ASYNC_ITER_RETURNED.into(), Value::Bool(true));
        attach_async_next(map);
    }
}

fn async_inst_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.map(fn)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.map(fn)")?;
    Ok(create_async_lazy_iterator(
        "map",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_FN.into(), func.clone())]),
    ))
}

fn async_inst_filter_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.filter(fn)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.filter(fn)")?;
    Ok(create_async_lazy_iterator(
        "filter",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_FN.into(), func.clone())]),
    ))
}

fn async_inst_take_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.take(n)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("AsyncIterator.take(n) expects non-negative number".into()),
    };
    Ok(create_async_lazy_iterator(
        "take",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_TAKE.into(), Value::Number(n))]),
    ))
}

fn async_inst_skip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.skip(n)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("AsyncIterator.skip(n) expects non-negative number".into()),
    };
    let mut src = iter.clone();
    let mut remaining = n;
    while remaining > 0 {
        let (_, done) = async_step_source(&mut src, env)?;
        if done {
            break;
        }
        remaining -= 1;
    }
    Ok(create_async_lazy_iterator("pass", src, HashMap::new()))
}

fn async_inst_flat_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.flatMap(fn, depth?)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.flatMap(fn, depth?)")?;
    let depth = match args.get(2) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    };
    Ok(create_async_lazy_iterator(
        "flatmap",
        iter.clone(),
        HashMap::from([
            (ASYNC_ITER_FN.into(), func.clone()),
            (ASYNC_ITER_FLAT_DEPTH.into(), Value::Number(depth)),
        ]),
    ))
}

fn async_inst_drop_while_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.dropWhile(fn)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.dropWhile(fn)")?;
    Ok(create_async_lazy_iterator(
        "dropwhile",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_FN.into(), func.clone())]),
    ))
}

fn async_inst_take_while_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.takeWhile(fn)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.takeWhile(fn)")?;
    Ok(create_async_lazy_iterator(
        "takewhile",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_FN.into(), func.clone())]),
    ))
}

fn async_inst_pairwise_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.pairwise()")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    Ok(create_async_lazy_iterator("pairwise", iter.clone(), HashMap::new()))
}

fn async_inst_accumulate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.accumulate(fn, initial?)")?;
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let func = args.get(1).ok_or("AsyncIterator.accumulate(fn, initial?)")?;
    let mut extra = HashMap::from([(ASYNC_ITER_FN.into(), func.clone())]);
    if let Some(init) = args.get(2) {
        extra.insert(ASYNC_ITER_ACC_INITIAL.into(), init.clone());
    }
    Ok(create_async_lazy_iterator("accumulate", iter.clone(), extra))
}

fn async_inst_zip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.zip(iterable)")?;
    let other = args.get(1).ok_or("AsyncIterator.zip(iterable)")?;
    create_async_zip_on_iterator(iter, other, env)
}

fn async_inst_enumerate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.enumerate()")?;
    create_async_enumerate_on_iterator(iter)
}

fn async_inst_chain_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let iter = args.first().ok_or("AsyncIterator.chain(iterable, ...)")?;
    if args.len() < 2 {
        return Err("AsyncIterator.chain(iterable, ...) expects at least one iterable".into());
    }
    create_async_chain_on_iterator(iter, &args[1..])
}

fn async_inst_to_array_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.toArray()")?.clone();
    Ok(resolved_promise(Value::Array(async_iterator_collect(
        &mut iter, env,
    )?)))
}

fn async_inst_reduce_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.reduce(fn, initial?)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.reduce(fn, initial?)")?.clone();
    let initial = args.get(2).cloned();
    let items = async_iterator_collect(&mut iter, env)?;
    let mut acc = match initial {
        Some(v) => v,
        None => items
            .first()
            .cloned()
            .ok_or("reduce of empty iterator with no initial value")?,
    };
    let start = if args.get(2).is_some() { 0 } else { 1 };
    for item in items.into_iter().skip(start) {
        acc = call_fn(&func, vec![acc, item], env)?;
    }
    Ok(resolved_promise(acc))
}

fn async_inst_some_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.some(fn)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.some(fn)")?.clone();
    for item in async_iterator_collect(&mut iter, env)? {
        if call_fn(&func, vec![item], env)?.is_truthy() {
            return Ok(resolved_promise(Value::Bool(true)));
        }
    }
    Ok(resolved_promise(Value::Bool(false)))
}

fn async_inst_every_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.every(fn)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.every(fn)")?.clone();
    for item in async_iterator_collect(&mut iter, env)? {
        if !call_fn(&func, vec![item], env)?.is_truthy() {
            return Ok(resolved_promise(Value::Bool(false)));
        }
    }
    Ok(resolved_promise(Value::Bool(true)))
}

pub fn async_iterator_close(
    it: &mut Value,
    value: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_async_generator_object(it) {
        let result = crate::runtime::stdlib::generator::return_generator(it, value, env)?;
        return Ok(resolved_promise(result));
    }
    if let Value::Object(map) = &*it {
        if map.contains_key(ASYNC_ITER_LAZY) {
            if let Value::Object(updated) = it {
                updated.insert(ASYNC_ITER_RETURNED.into(), Value::Bool(true));
                attach_async_next(updated);
            }
            return Ok(resolved_promise(iterator_result(value, true)));
        }
        if let Some(sync) = map.get(ASYNC_SYNC_DELEGATE).cloned() {
            let mut sync = sync;
            let result = crate::runtime::stdlib::iterator::iterator_return(&mut sync, value, env)?;
            if let Value::Object(map) = it {
                map.insert(ASYNC_SYNC_DELEGATE.into(), sync);
                attach_async_next(map);
            }
            return Ok(resolved_promise(result));
        }
    }
    if let Some(resolved) = async_call_object_method(it, "return", vec![value.clone()], env)? {
        mark_async_iterator_returned(it);
        return Ok(resolved_promise(resolved));
    }
    mark_async_iterator_returned(it);
    Ok(resolved_promise(iterator_result(value, true)))
}

fn async_inst_return_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.return(value?)")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut iter, env);
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    if crate::runtime::stdlib::generator::is_async_generator_object(&iter) {
        let result = crate::runtime::stdlib::generator::return_generator(&mut iter, value, env)?;
        return Ok(resolved_promise(result));
    }
    if let Value::Object(map) = &iter {
        if map.contains_key(ASYNC_ITER_LAZY) {
            if let Value::Object(updated) = &mut iter {
                updated.insert(ASYNC_ITER_RETURNED.into(), Value::Bool(true));
                attach_async_next(updated);
            }
            return Ok(resolved_promise(iterator_result(value, true)));
        }
        if let Some(sync) = map.get(ASYNC_SYNC_DELEGATE).cloned() {
            let mut sync = sync;
            let result = crate::runtime::stdlib::iterator::iterator_return(&mut sync, value, env)?;
            if let Value::Object(map) = &mut iter {
                map.insert(ASYNC_SYNC_DELEGATE.into(), sync);
                attach_async_next(map);
            }
            return Ok(resolved_promise(result));
        }
    }
    if let Some(resolved) = async_call_object_method(&mut iter, "return", vec![value.clone()], env)? {
        mark_async_iterator_returned(&mut iter);
        return Ok(resolved_promise(resolved));
    }
    mark_async_iterator_returned(&mut iter);
    Ok(resolved_promise(iterator_result(value, true)))
}

fn async_inst_throw_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.throw(reason?)")?.clone();
    crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut iter, env);
    let reason = args.get(1).cloned().unwrap_or(Value::Null);
    if crate::runtime::stdlib::generator::is_async_generator_object(&iter) {
        let result = crate::runtime::stdlib::generator::throw_generator(&mut iter, reason, env)?;
        return Ok(resolved_promise(result));
    }
    if let Value::Object(map) = &iter {
        if map.contains_key(ASYNC_ITER_LAZY) {
            if let Value::Object(updated) = &mut iter {
                updated.insert(ASYNC_ITER_RETURNED.into(), Value::Bool(true));
                attach_async_next(updated);
            }
            return Ok(resolved_promise(iterator_result(reason, true)));
        }
        if let Some(sync) = map.get(ASYNC_SYNC_DELEGATE).cloned() {
            let mut sync = sync;
            let result = crate::runtime::stdlib::iterator::iterator_throw(&mut sync, reason, env)?;
            if let Value::Object(map) = &mut iter {
                map.insert(ASYNC_SYNC_DELEGATE.into(), sync);
                attach_async_next(map);
            }
            return Ok(resolved_promise(result));
        }
    }
    if let Some(resolved) = async_call_object_method(&mut iter, "throw", vec![reason.clone()], env)? {
        mark_async_iterator_returned(&mut iter);
        return Ok(resolved_promise(resolved));
    }
    mark_async_iterator_returned(&mut iter);
    Ok(resolved_promise(iterator_result(reason, true)))
}

fn async_inst_for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.forEach(fn)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.forEach(fn)")?.clone();
    loop {
        let next_p = async_iterator_next(&mut iter, env)?;
        let result = resolve_await_value(next_p, env)?;
        let (value, done) = crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
        if done {
            break;
        }
        call_fn(&func, vec![value], env)?;
    }
    Ok(resolved_promise(Value::Null))
}

fn async_inst_find_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.find(fn)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.find(fn)")?.clone();
    loop {
        let next_p = async_iterator_next(&mut iter, env)?;
        let result = resolve_await_value(next_p, env)?;
        let (value, done) = crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
        if done {
            break;
        }
        if call_fn(&func, vec![value.clone()], env)?.is_truthy() {
            return Ok(resolved_promise(value));
        }
    }
    Ok(resolved_promise(Value::Undefined))
}

fn async_inst_find_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.findIndex(fn)")?.clone();
    let func = args.get(1).ok_or("AsyncIterator.findIndex(fn)")?.clone();
    let mut index = 0i64;
    loop {
        let next_p = async_iterator_next(&mut iter, env)?;
        let result = resolve_await_value(next_p, env)?;
        let (value, done) = crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
        if done {
            break;
        }
        if call_fn(&func, vec![value], env)?.is_truthy() {
            return Ok(resolved_promise(Value::Number(index)));
        }
        index += 1;
    }
    Ok(resolved_promise(Value::Number(-1)))
}

fn async_inst_includes_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut iter = args.first().ok_or("AsyncIterator.includes(value)")?.clone();
    let needle = args.get(1).ok_or("AsyncIterator.includes(value)")?.clone();
    loop {
        let next_p = async_iterator_next(&mut iter, env)?;
        let result = resolve_await_value(next_p, env)?;
        let (value, done) = crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
        if done {
            break;
        }
        if crate::ops::values_equal(&value, &needle) {
            return Ok(resolved_promise(Value::Bool(true)));
        }
    }
    Ok(resolved_promise(Value::Bool(false)))
}

pub fn attach_async_iterator_instance_methods(map: &mut HashMap<String, Value>) {
    if map.contains_key(ASYNC_ITER_METHODS) {
        return;
    }
    map.insert(ASYNC_ITER_METHODS.into(), Value::Bool(true));
    let receiver = Value::Object(map.clone());
    for (name, native) in [
        ("map", async_inst_map_native as fn(&[Value], &mut Environment) -> Result<Value, String>),
        ("filter", async_inst_filter_native),
        ("take", async_inst_take_native),
        ("skip", async_inst_skip_native),
        ("flatMap", async_inst_flat_map_native),
        ("dropWhile", async_inst_drop_while_native),
        ("takeWhile", async_inst_take_while_native),
        ("pairwise", async_inst_pairwise_native),
        ("accumulate", async_inst_accumulate_native),
        ("zip", async_inst_zip_native),
        ("enumerate", async_inst_enumerate_native),
        ("chain", async_inst_chain_native),
        ("toArray", async_inst_to_array_native),
        ("reduce", async_inst_reduce_native),
        ("some", async_inst_some_native),
        ("every", async_inst_every_native),
        ("forEach", async_inst_for_each_native),
        ("find", async_inst_find_native),
        ("findIndex", async_inst_find_index_native),
        ("includes", async_inst_includes_native),
        ("return", async_inst_return_native),
        ("throw", async_inst_throw_native),
    ] {
        map.insert(
            name.into(),
            Value::BoundNative(Box::new(receiver.clone()), native),
        );
    }
}

pub(crate) fn create_async_map_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "map",
        src,
        HashMap::from([(ASYNC_ITER_FN.into(), func)]),
    ))
}

pub(crate) fn create_async_filter_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "filter",
        src,
        HashMap::from([(ASYNC_ITER_FN.into(), func)]),
    ))
}

pub(crate) fn create_async_take_from_iterable(
    v: &Value,
    n: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "take",
        src,
        HashMap::from([(ASYNC_ITER_TAKE.into(), Value::Number(n))]),
    ))
}

pub(crate) fn create_async_skip_from_iterable(
    v: &Value,
    n: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let mut src = get_async_iterator(v, env)?;
    let mut remaining = n;
    while remaining > 0 {
        let (_, done) = async_step_source(&mut src, env)?;
        if done {
            break;
        }
        remaining -= 1;
    }
    Ok(create_async_lazy_iterator("pass", src, HashMap::new()))
}

pub(crate) fn create_async_flat_map_from_iterable(
    v: &Value,
    func: Value,
    depth: i64,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "flatmap",
        src,
        HashMap::from([
            (ASYNC_ITER_FN.into(), func),
            (ASYNC_ITER_FLAT_DEPTH.into(), Value::Number(depth.max(1))),
        ]),
    ))
}

pub(crate) fn create_async_drop_while_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "dropwhile",
        src,
        HashMap::from([(ASYNC_ITER_FN.into(), func)]),
    ))
}

pub(crate) fn create_async_take_while_from_iterable(
    v: &Value,
    func: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "takewhile",
        src,
        HashMap::from([(ASYNC_ITER_FN.into(), func)]),
    ))
}

pub(crate) fn create_async_zip_from_iterables(
    a: &Value,
    b: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let left = get_async_iterator(a, env)?;
    let right = get_async_iterator(b, env)?;
    Ok(create_async_lazy_iterator(
        "zip",
        left,
        HashMap::from([(ASYNC_ITER_ZIP_RIGHT.into(), right)]),
    ))
}

pub(crate) fn create_async_enumerate_from_iterable(
    v: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator(
        "enumerate",
        src,
        HashMap::from([(ASYNC_ITER_ENUM_IDX.into(), Value::Number(0))]),
    ))
}

pub(crate) fn create_async_chain_from_iterables(
    iterables: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    if iterables.is_empty() {
        return Err("AsyncIterator.chain(iterable, ...) expects at least one iterable".into());
    }
    let first = get_async_iterator(&iterables[0], env)?;
    Ok(create_async_lazy_iterator(
        "chain",
        first,
        HashMap::from([
            (ASYNC_ITER_CHAIN.into(), Value::Array(iterables.to_vec())),
            (ASYNC_ITER_CHAIN_IDX.into(), Value::Number(0)),
        ]),
    ))
}

pub(crate) fn create_async_pairwise_from_iterable(
    v: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    Ok(create_async_lazy_iterator("pairwise", src, HashMap::new()))
}

pub(crate) fn create_async_accumulate_from_iterable(
    v: &Value,
    func: Value,
    initial: Option<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let src = get_async_iterator(v, env)?;
    let mut extra = HashMap::from([(ASYNC_ITER_FN.into(), func)]);
    if let Some(init) = initial {
        extra.insert(ASYNC_ITER_ACC_INITIAL.into(), init);
    }
    Ok(create_async_lazy_iterator("accumulate", src, extra))
}

pub(crate) fn create_async_zip_on_iterator(
    iter: &Value,
    other: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    let right = get_async_iterator(other, env)?;
    Ok(create_async_lazy_iterator(
        "zip",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_ZIP_RIGHT.into(), right)]),
    ))
}

pub(crate) fn create_async_enumerate_on_iterator(iter: &Value) -> Result<Value, String> {
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    Ok(create_async_lazy_iterator(
        "enumerate",
        iter.clone(),
        HashMap::from([(ASYNC_ITER_ENUM_IDX.into(), Value::Number(0))]),
    ))
}

pub(crate) fn create_async_chain_on_iterator(
    iter: &Value,
    rest: &[Value],
) -> Result<Value, String> {
    if !is_async_iterator_value(iter) {
        return Err("AsyncIterator method expects an async iterator receiver".into());
    }
    if rest.is_empty() {
        return Err("AsyncIterator.chain(iterable, ...) expects at least one iterable".into());
    }
    let mut chain_items = vec![Value::Null];
    chain_items.extend(rest.iter().cloned());
    Ok(create_async_lazy_iterator(
        "chain",
        iter.clone(),
        HashMap::from([
            (ASYNC_ITER_CHAIN.into(), Value::Array(chain_items)),
            (ASYNC_ITER_CHAIN_IDX.into(), Value::Number(0)),
        ]),
    ))
}

pub fn array_from_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("array_from_async(iterable)")?;
    let items = for_await_of_items_with_env(v, env)?;
    Ok(resolved_promise(Value::Array(items)))
}
