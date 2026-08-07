//! ECMAScript `WeakMap` and `WeakSet` — object-keyed weak collections.

use crate::runtime::stdlib::object::object_oid;
use crate::runtime::stdlib::weak::is_oid_reachable;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

const WEAK_MAP_MARKER: &str = "__kab_weakmap";
const WEAK_SET_MARKER: &str = "__kab_weakset";

static NEXT_WEAK_MAP: AtomicU64 = AtomicU64::new(1);
static NEXT_WEAK_SET: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static WEAK_MAP_STORE: RefCell<HashMap<u64, HashMap<u64, Value>>> =
        RefCell::new(HashMap::new());
    static WEAK_SET_STORE: RefCell<HashMap<u64, HashSet<u64>>> =
        RefCell::new(HashMap::new());
}

fn weak_key_oid(key: &Value) -> Result<u64, String> {
    let mut key = key.clone();
    let Value::Object(map) = &mut key else {
        return Err("WeakMap/WeakSet key must be an object".into());
    };
    Ok(object_oid(Value::object_make_mut(map)))
}

fn weak_map_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected WeakMap".into());
    };
    if !matches!(o.get(WEAK_MAP_MARKER), Some(Value::Bool(true))) {
        return Err("expected WeakMap".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid WeakMap handle".into()),
    }
}

fn weak_set_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected WeakSet".into());
    };
    if !matches!(o.get(WEAK_SET_MARKER), Some(Value::Bool(true))) {
        return Err("expected WeakSet".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid WeakSet handle".into()),
    }
}

pub fn is_weakmap_value(v: &Value) -> bool {
    matches!(
        v, Value::Object(o) if matches!(o.get(WEAK_MAP_MARKER), Some(Value::Bool(true)))
    )
}

pub fn is_weakset_value(v: &Value) -> bool {
    matches!(
        v, Value::Object(o) if matches!(o.get(WEAK_SET_MARKER), Some(Value::Bool(true)))
    )
}

fn weak_map_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_WEAK_MAP.fetch_add(1, Ordering::Relaxed);
    WEAK_MAP_STORE.with(|s| s.borrow_mut().insert(id, HashMap::new()));
    let mut m = HashMap::new();
    m.insert(WEAK_MAP_MARKER.into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Ok(Value::from_object(m))
}

fn weak_map_set_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let map = args.first().ok_or("weak_map_set(map, key, value)")?;
    let key = args.get(1).ok_or("weak_map_set(map, key, value)")?;
    let value = args
        .get(2)
        .cloned()
        .unwrap_or(Value::Undefined);
    let map_id = weak_map_id(map)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Err("WeakMap key must be reachable".into());
    }
    WEAK_MAP_STORE.with(|s| {
        if let Some(inner) = s.borrow_mut().get_mut(&map_id) {
            inner.insert(key_oid, value);
        }
    });
    Ok(Value::Undefined)
}

fn weak_map_get_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let map = args.first().ok_or("weak_map_get(map, key)")?;
    let key = args.get(1).ok_or("weak_map_get(map, key)")?;
    let map_id = weak_map_id(map)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Ok(Value::Undefined);
    }
    let value = WEAK_MAP_STORE.with(|s| {
        s.borrow()
            .get(&map_id)
            .and_then(|inner| inner.get(&key_oid).cloned())
    });
    Ok(value.unwrap_or(Value::Undefined))
}

fn weak_map_has_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let map = args.first().ok_or("weak_map_has(map, key)")?;
    let key = args.get(1).ok_or("weak_map_has(map, key)")?;
    let map_id = weak_map_id(map)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Ok(Value::Bool(false));
    }
    let found = WEAK_MAP_STORE.with(|s| {
        s.borrow()
            .get(&map_id)
            .is_some_and(|inner| inner.contains_key(&key_oid))
    });
    Ok(Value::Bool(found))
}

fn weak_map_delete_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let map = args.first().ok_or("weak_map_delete(map, key)")?;
    let key = args.get(1).ok_or("weak_map_delete(map, key)")?;
    let map_id = weak_map_id(map)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Ok(Value::Bool(false));
    }
    let removed = WEAK_MAP_STORE.with(|s| {
        s.borrow_mut()
            .get_mut(&map_id)
            .is_some_and(|inner| inner.remove(&key_oid).is_some())
    });
    Ok(Value::Bool(removed))
}

fn weak_set_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_WEAK_SET.fetch_add(1, Ordering::Relaxed);
    WEAK_SET_STORE.with(|s| s.borrow_mut().insert(id, HashSet::new()));
    let mut m = HashMap::new();
    m.insert(WEAK_SET_MARKER.into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Ok(Value::from_object(m))
}

fn weak_set_add_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let set = args.first().ok_or("weak_set_add(set, value)")?;
    let key = args.get(1).ok_or("weak_set_add(set, value)")?;
    let set_id = weak_set_id(set)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Err("WeakSet value must be reachable".into());
    }
    WEAK_SET_STORE.with(|s| {
        if let Some(inner) = s.borrow_mut().get_mut(&set_id) {
            inner.insert(key_oid);
        }
    });
    Ok(Value::Undefined)
}

fn weak_set_has_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let set = args.first().ok_or("weak_set_has(set, value)")?;
    let key = args.get(1).ok_or("weak_set_has(set, value)")?;
    let set_id = weak_set_id(set)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Ok(Value::Bool(false));
    }
    let found = WEAK_SET_STORE.with(|s| {
        s.borrow()
            .get(&set_id)
            .is_some_and(|inner| inner.contains(&key_oid))
    });
    Ok(Value::Bool(found))
}

fn weak_set_delete_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let set = args.first().ok_or("weak_set_delete(set, value)")?;
    let key = args.get(1).ok_or("weak_set_delete(set, value)")?;
    let set_id = weak_set_id(set)?;
    let key_oid = weak_key_oid(key)?;
    if !is_oid_reachable(env, key_oid) {
        return Ok(Value::Bool(false));
    }
    let removed = WEAK_SET_STORE.with(|s| {
        s.borrow_mut()
            .get_mut(&set_id)
            .is_some_and(|inner| inner.remove(&key_oid))
    });
    Ok(Value::Bool(removed))
}

fn is_weakmap_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_weakmap(v)")?;
    Ok(Value::Bool(is_weakmap_value(v)))
}

fn is_weakset_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_weakset(v)")?;
    Ok(Value::Bool(is_weakset_value(v)))
}

pub fn register_weak_collections(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("weak_map_new", weak_map_new_native),
        ("weak_map_set", weak_map_set_native),
        ("weak_map_get", weak_map_get_native),
        ("weak_map_has", weak_map_has_native),
        ("weak_map_delete", weak_map_delete_native),
        ("weak_set_new", weak_set_new_native),
        ("weak_set_add", weak_set_add_native),
        ("weak_set_has", weak_set_has_native),
        ("weak_set_delete", weak_set_delete_native),
        ("is_weakmap", is_weakmap_native),
        ("is_weakset", is_weakset_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
