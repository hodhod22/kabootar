//! `Iterator` / `AsyncIterator` classes — static factories (Kabootar uses classes, not prototypes).

use crate::runtime::stdlib::async_iterator::{
    create_async_accumulate_from_iterable, create_async_chain_from_iterables,
    create_async_drop_while_from_iterable,
    create_async_enumerate_from_iterable, create_async_filter_from_iterable,
    create_async_flat_map_from_iterable, create_async_map_from_iterable,
    create_async_pairwise_from_iterable, create_async_skip_from_iterable,
    create_async_take_from_iterable,
    create_async_take_while_from_iterable, create_async_zip_from_iterables,
    get_async_iterator, is_async_iterator_value,
};
use crate::runtime::stdlib::iterator::{
    create_accumulate_iterator_from_iterable, create_chain_iterator_from_iterables,
    create_enumerate_iterator_from_iterable,
    create_filter_iterator_from_iterable, create_flat_map_iterator_from_iterable,
    create_drop_while_iterator_from_iterable, create_pairwise_iterator_from_iterable,
    create_take_while_iterator_from_iterable,
    create_map_iterator_from_iterable,
    create_skip_iterator_from_iterable, create_take_iterator_from_iterable,
    create_zip_iterator_from_iterables, is_iterator_value, iterator_from_iterable,
};
use crate::value::{Environment, Value};
use std::collections::HashMap;

pub fn is_iterator_class_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get("__kab_iterator_class"), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_async_iterator_class_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => {
            matches!(m.get("__kab_async_iterator_class"), Some(Value::Bool(true)))
        }
        _ => false,
    }
}

fn insert_native(
    map: &mut HashMap<String, Value>,
    name: &str,
    func: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    map.insert(name.into(), Value::NativeFunction(func));
}

fn iterator_is_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.is(v)")?;
    Ok(Value::Bool(is_iterator_value(v)))
}

fn iterator_from_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.from(iterable)")?;
    iterator_from_iterable(v, env)
}

fn iterator_from_async_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.fromAsync(asyncIterable)")?;
    get_async_iterator(v, env)
}

fn iterator_zip_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("Iterator.zip(a, b)")?;
    let b = args.get(1).ok_or("Iterator.zip(a, b)")?;
    create_zip_iterator_from_iterables(a, b, env)
}

fn iterator_enumerate_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.enumerate(iterable)")?;
    create_enumerate_iterator_from_iterable(v, env)
}

fn iterator_chain_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("Iterator.chain(iterable, ...) expects at least one iterable".into());
    }
    create_chain_iterator_from_iterables(args, env)
}

fn iterator_map_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.map(iterable, fn)")?;
    let func = args.get(1).ok_or("Iterator.map(iterable, fn)")?;
    create_map_iterator_from_iterable(v, func.clone(), env)
}

fn iterator_filter_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.filter(iterable, fn)")?;
    let func = args.get(1).ok_or("Iterator.filter(iterable, fn)")?;
    create_filter_iterator_from_iterable(v, func.clone(), env)
}

fn iterator_take_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.take(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("Iterator.take(iterable, n) expects non-negative number".into()),
    };
    create_take_iterator_from_iterable(v, n, env)
}

fn iterator_skip_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.skip(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("Iterator.skip(iterable, n) expects non-negative number".into()),
    };
    create_skip_iterator_from_iterable(v, n, env)
}

fn iterator_flat_map_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.flatMap(iterable, fn, depth?)")?;
    let func = args.get(1).ok_or("Iterator.flatMap(iterable, fn, depth?)")?;
    let depth = match args.get(2) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    };
    create_flat_map_iterator_from_iterable(v, func.clone(), depth, env)
}

fn iterator_drop_while_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.dropWhile(iterable, fn)")?;
    let func = args.get(1).ok_or("Iterator.dropWhile(iterable, fn)")?;
    create_drop_while_iterator_from_iterable(v, func.clone(), env)
}

fn iterator_take_while_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.takeWhile(iterable, fn)")?;
    let func = args.get(1).ok_or("Iterator.takeWhile(iterable, fn)")?;
    create_take_while_iterator_from_iterable(v, func.clone(), env)
}

fn iterator_pairwise_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.pairwise(iterable)")?;
    create_pairwise_iterator_from_iterable(v, env)
}

fn iterator_accumulate_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Iterator.accumulate(iterable, fn, initial?)")?;
    let func = args.get(1).ok_or("Iterator.accumulate(iterable, fn, initial?)")?;
    let initial = args.get(2).cloned();
    create_accumulate_iterator_from_iterable(v, func.clone(), initial, env)
}

fn async_iterator_is_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.is(v)")?;
    Ok(Value::Bool(is_async_iterator_value(v)))
}

fn async_iterator_from_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.from(iterable)")?;
    crate::runtime::stdlib::async_iterator::get_async_iterator(v, env)
}

fn async_iterator_zip_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("AsyncIterator.zip(a, b)")?;
    let b = args.get(1).ok_or("AsyncIterator.zip(a, b)")?;
    create_async_zip_from_iterables(a, b, env)
}

fn async_iterator_enumerate_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.enumerate(iterable)")?;
    create_async_enumerate_from_iterable(v, env)
}

fn async_iterator_chain_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("AsyncIterator.chain(iterable, ...) expects at least one iterable".into());
    }
    create_async_chain_from_iterables(args, env)
}

fn async_iterator_map_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.map(iterable, fn)")?;
    let func = args.get(1).ok_or("AsyncIterator.map(iterable, fn)")?;
    create_async_map_from_iterable(v, func.clone(), env)
}

fn async_iterator_filter_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.filter(iterable, fn)")?;
    let func = args.get(1).ok_or("AsyncIterator.filter(iterable, fn)")?;
    create_async_filter_from_iterable(v, func.clone(), env)
}

fn async_iterator_take_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.take(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("AsyncIterator.take(iterable, n) expects non-negative number".into()),
    };
    create_async_take_from_iterable(v, n, env)
}

fn async_iterator_skip_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.skip(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("AsyncIterator.skip(iterable, n) expects non-negative number".into()),
    };
    create_async_skip_from_iterable(v, n, env)
}

fn async_iterator_flat_map_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.flatMap(iterable, fn, depth?)")?;
    let func = args.get(1).ok_or("AsyncIterator.flatMap(iterable, fn, depth?)")?;
    let depth = match args.get(2) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    };
    create_async_flat_map_from_iterable(v, func.clone(), depth, env)
}

fn async_iterator_drop_while_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.dropWhile(iterable, fn)")?;
    let func = args.get(1).ok_or("AsyncIterator.dropWhile(iterable, fn)")?;
    create_async_drop_while_from_iterable(v, func.clone(), env)
}

fn async_iterator_take_while_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.takeWhile(iterable, fn)")?;
    let func = args.get(1).ok_or("AsyncIterator.takeWhile(iterable, fn)")?;
    create_async_take_while_from_iterable(v, func.clone(), env)
}

fn async_iterator_pairwise_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.pairwise(iterable)")?;
    create_async_pairwise_from_iterable(v, env)
}

fn async_iterator_accumulate_static(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("AsyncIterator.accumulate(iterable, fn, initial?)")?;
    let func = args.get(1).ok_or("AsyncIterator.accumulate(iterable, fn, initial?)")?;
    let initial = args.get(2).cloned();
    create_async_accumulate_from_iterable(v, func.clone(), initial, env)
}

pub fn build_iterator_class() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_iterator_class".into(), Value::Bool(true));
    insert_native(&mut m, "is", iterator_is_native);
    insert_native(&mut m, "from", iterator_from_static);
    insert_native(&mut m, "fromAsync", iterator_from_async_static);
    insert_native(&mut m, "zip", iterator_zip_static);
    insert_native(&mut m, "enumerate", iterator_enumerate_static);
    insert_native(&mut m, "chain", iterator_chain_static);
    insert_native(&mut m, "map", iterator_map_static);
    insert_native(&mut m, "filter", iterator_filter_static);
    insert_native(&mut m, "take", iterator_take_static);
    insert_native(&mut m, "skip", iterator_skip_static);
    insert_native(&mut m, "flatMap", iterator_flat_map_static);
    insert_native(&mut m, "dropWhile", iterator_drop_while_static);
    insert_native(&mut m, "takeWhile", iterator_take_while_static);
    insert_native(&mut m, "pairwise", iterator_pairwise_static);
    insert_native(&mut m, "accumulate", iterator_accumulate_static);
    Value::from_object(m)
}

pub fn build_async_iterator_class() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_async_iterator_class".into(), Value::Bool(true));
    insert_native(&mut m, "is", async_iterator_is_native);
    insert_native(&mut m, "from", async_iterator_from_static);
    insert_native(&mut m, "zip", async_iterator_zip_static);
    insert_native(&mut m, "enumerate", async_iterator_enumerate_static);
    insert_native(&mut m, "chain", async_iterator_chain_static);
    insert_native(&mut m, "map", async_iterator_map_static);
    insert_native(&mut m, "filter", async_iterator_filter_static);
    insert_native(&mut m, "take", async_iterator_take_static);
    insert_native(&mut m, "skip", async_iterator_skip_static);
    insert_native(&mut m, "flatMap", async_iterator_flat_map_static);
    insert_native(&mut m, "dropWhile", async_iterator_drop_while_static);
    insert_native(&mut m, "takeWhile", async_iterator_take_while_static);
    insert_native(&mut m, "pairwise", async_iterator_pairwise_static);
    insert_native(&mut m, "accumulate", async_iterator_accumulate_static);
    Value::from_object(m)
}

pub fn register_iterator_classes(env: &mut Environment) {
    env.set("Iterator".to_string(), build_iterator_class());
    env.set("AsyncIterator".to_string(), build_async_iterator_class());
}
