//! Map and Set — first-class collections (competitor parity).

use crate::bytecode::call_value;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_MAP: AtomicU64 = AtomicU64::new(1);
static NEXT_SET: AtomicU64 = AtomicU64::new(1);
static NEXT_COUNTER: AtomicU64 = AtomicU64::new(1);
static NEXT_DEFAULTDICT: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static MAPS: RefCell<HashMap<u64, HashMap<String, Value>>> = RefCell::new(HashMap::new());
    static SETS: RefCell<HashMap<u64, HashMap<String, Value>>> = RefCell::new(HashMap::new());
    static COUNTERS: RefCell<HashMap<u64, HashMap<String, i64>>> = RefCell::new(HashMap::new());
    static DEFAULTDICTS: RefCell<HashMap<u64, (HashMap<String, Value>, Value)>> = RefCell::new(HashMap::new());
}

fn value_key(v: &Value) -> String {
    crate::value::format_value(v)
}

fn call_fn(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env)
}

pub fn map_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_map".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

pub fn set_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_set".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

fn counter_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_counter".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

fn defaultdict_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_defaultdict".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

pub(crate) fn map_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected map".into());
    };
    if !matches!(o.get("__kab_map"), Some(Value::Bool(true))) {
        return Err("expected map".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid map handle".into()),
    }
}

pub(crate) fn set_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected set".into());
    };
    if !matches!(o.get("__kab_set"), Some(Value::Bool(true))) {
        return Err("expected set".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid set handle".into()),
    }
}

pub fn is_counter_value(v: &Value) -> bool {
    counter_id(v).is_ok()
}

pub fn is_defaultdict_value(v: &Value) -> bool {
    defaultdict_id(v).is_ok()
}

pub(crate) fn counter_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected counter".into());
    };
    if !matches!(o.get("__kab_counter"), Some(Value::Bool(true))) {
        return Err("expected counter".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid counter handle".into()),
    }
}

pub(crate) fn defaultdict_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected defaultdict".into());
    };
    if !matches!(o.get("__kab_defaultdict"), Some(Value::Bool(true))) {
        return Err("expected defaultdict".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid defaultdict handle".into()),
    }
}

fn i64_arg(v: &Value) -> Result<i64, String> {
    match v {
        Value::Number(n) => Ok(*n),
        _ => Err("expected number".into()),
    }
}

pub(crate) fn range_items(start: i64, end: i64, step: i64) -> Result<Vec<Value>, String> {
    if step == 0 {
        return Err("range step cannot be 0".into());
    }
    let mut items = Vec::new();
    if step > 0 {
        let mut i = start;
        while i < end {
            items.push(Value::Number(i));
            i += step;
        }
    } else {
        let mut i = start;
        while i > end {
            items.push(Value::Number(i));
            i += step;
        }
    }
    Ok(items)
}

fn range_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    match args.len() {
        1 => {
            let end = i64_arg(args.first().ok_or("range(end)")?)?;
            Ok(Value::Range {
                start: 0,
                end,
                step: 1,
            })
        }
        2 => {
            let start = i64_arg(args.first().ok_or("range(start, end)")?)?;
            let end = i64_arg(args.get(1).ok_or("range(start, end)")?)?;
            Ok(Value::Range {
                start,
                end,
                step: 1,
            })
        }
        3 => {
            let start = i64_arg(args.first().ok_or("range(start, end, step)")?)?;
            let end = i64_arg(args.get(1).ok_or("range(start, end, step)")?)?;
            let step = i64_arg(args.get(2).ok_or("range(start, end, step)")?)?;
            if step == 0 {
                return Err("range step cannot be 0".into());
            }
            Ok(Value::Range { start, end, step })
        }
        _ => Err("range(end) or range(start, end) or range(start, end, step)".into()),
    }
}

fn counter_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_COUNTER.fetch_add(1, Ordering::Relaxed);
    COUNTERS.with(|c| {
        c.borrow_mut().insert(id, HashMap::new());
    });
    Ok(counter_object(id))
}

fn counter_inc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = counter_id(args.first().ok_or("counter_inc(counter, key, n?)")?)?;
    let key = str_arg(args, 1)?;
    let delta = match args.get(2) {
        Some(v) => i64_arg(v)?,
        None => 1,
    };
    COUNTERS.with(|c| {
        let mut inner = c.borrow_mut();
        let bucket = inner.get_mut(&id).ok_or("invalid counter")?;
        let entry = bucket.entry(key).or_insert(0);
        *entry += delta;
        Ok(Value::Number(*entry))
    })
}

fn counter_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = counter_id(args.first().ok_or("counter_get(counter, key)")?)?;
    let key = str_arg(args, 1)?;
    Ok(COUNTERS.with(|c| {
        c.borrow()
            .get(&id)
            .and_then(|inner| inner.get(&key).copied())
            .map(Value::Number)
            .unwrap_or(Value::Number(0))
    }))
}

fn counter_items_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = counter_id(args.first().ok_or("counter_items(counter)")?)?;
    Ok(COUNTERS.with(|c| {
        let mut pairs = Vec::new();
        if let Some(inner) = c.borrow().get(&id) {
            let mut keys: Vec<_> = inner.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let count = inner[&key];
                pairs.push(Value::Array(vec![Value::String(key), Value::Number(count)]));
            }
        }
        Value::Array(pairs)
    }))
}

fn defaultdict_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let default = args
        .first()
        .cloned()
        .unwrap_or(Value::Null);
    let id = NEXT_DEFAULTDICT.fetch_add(1, Ordering::Relaxed);
    DEFAULTDICTS.with(|d| {
        d.borrow_mut().insert(id, (HashMap::new(), default));
    });
    Ok(defaultdict_object(id))
}

fn defaultdict_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = defaultdict_id(args.first().ok_or("defaultdict_get(dd, key)")?)?;
    let key = str_arg(args, 1)?;
    DEFAULTDICTS.with(|d| {
        let inner = d.borrow();
        let (map, default) = inner.get(&id).ok_or("invalid defaultdict")?;
        Ok(map.get(&key).cloned().unwrap_or_else(|| default.clone()))
    })
}

fn defaultdict_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = defaultdict_id(args.first().ok_or("defaultdict_set(dd, key, value)")?)?;
    let key = str_arg(args, 1)?;
    let value = args.get(2).cloned().unwrap_or(Value::Null);
    DEFAULTDICTS.with(|d| {
        let mut inner = d.borrow_mut();
        let (map, _) = inner.get_mut(&id).ok_or("invalid defaultdict")?;
        map.insert(key, value);
        Ok(Value::Null)
    })
}

fn str_arg(args: &[Value], i: usize) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err("expected string key".into()),
    }
}

fn map_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_MAP.fetch_add(1, Ordering::Relaxed);
    MAPS.with(|m| {
        m.borrow_mut().insert(id, HashMap::new());
    });
    Ok(map_object(id))
}

fn map_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_set(map, key, value)")?)?;
    let key = str_arg(args, 1)?;
    let val = args.get(2).cloned().unwrap_or(Value::Null);
    MAPS.with(|m| {
        let mut map = m.borrow_mut();
        let inner = map.get_mut(&id).ok_or("unknown map")?;
        inner.insert(key, val);
        Ok(Value::Bool(true))
    })
}

fn map_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_get(map, key)")?)?;
    let key = str_arg(args, 1)?;
    MAPS.with(|m| {
        Ok(m.borrow()
            .get(&id)
            .and_then(|inner| inner.get(&key))
            .cloned()
            .unwrap_or(Value::Undefined))
    })
}

fn map_has_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_has(map, key)")?)?;
    let key = str_arg(args, 1)?;
    MAPS.with(|m| {
        Ok(Value::Bool(
            m.borrow()
                .get(&id)
                .map(|inner| inner.contains_key(&key))
                .unwrap_or(false),
        ))
    })
}

fn map_delete_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_delete(map, key)")?)?;
    let key = str_arg(args, 1)?;
    MAPS.with(|m| {
        Ok(Value::Bool(
            m.borrow_mut()
                .get_mut(&id)
                .map(|inner| inner.remove(&key).is_some())
                .unwrap_or(false),
        ))
    })
}

fn map_clear_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_clear(map)")?)?;
    MAPS.with(|m| {
        if let Some(inner) = m.borrow_mut().get_mut(&id) {
            inner.clear();
        }
        Ok(Value::Null)
    })
}

fn map_size_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_size(map)")?)?;
    MAPS.with(|m| {
        Ok(Value::Number(
            m.borrow()
                .get(&id)
                .map(|inner| inner.len() as i64)
                .unwrap_or(0),
        ))
    })
}

fn map_keys_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_keys(map)")?)?;
    MAPS.with(|m| {
        let keys: Vec<Value> = m
            .borrow()
            .get(&id)
            .map(|inner| inner.keys().cloned().map(Value::String).collect())
            .unwrap_or_default();
        Ok(Value::Array(keys))
    })
}

fn map_values_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_values(map)")?)?;
    MAPS.with(|m| {
        let values: Vec<Value> = m
            .borrow()
            .get(&id)
            .map(|inner| inner.values().cloned().collect())
            .unwrap_or_default();
        Ok(Value::Array(values))
    })
}

fn map_entries_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = map_id(args.first().ok_or("map_entries(map)")?)?;
    MAPS.with(|m| {
        let entries: Vec<Value> = m
            .borrow()
            .get(&id)
            .map(|inner| {
                inner
                    .iter()
                    .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()]))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Value::Array(entries))
    })
}

fn map_from_entries_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pairs = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("map_from_entries(pairs) expects array".into()),
    };
    let id = NEXT_MAP.fetch_add(1, Ordering::Relaxed);
    let mut inner = HashMap::new();
    for pair in pairs {
        let Value::Array(entry) = pair else {
            return Err("map_from_entries() expects [[key, value], ...]".into());
        };
        if entry.len() < 2 {
            return Err("map_from_entries() entry needs key and value".into());
        }
        let key = match &entry[0] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => {
                return Err(format!(
                    "map_from_entries() key must be string or number, got {:?}",
                    other
                ));
            }
        };
        inner.insert(key, entry[1].clone());
    }
    MAPS.with(|m| m.borrow_mut().insert(id, inner));
    Ok(map_object(id))
}

fn map_for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let map_val = args.first().ok_or("map_for_each(map, fn)")?.clone();
    let id = map_id(&map_val)?;
    let func = args.get(1).ok_or("map_for_each(map, fn)")?;
    let snapshot: Vec<(String, Value)> = MAPS.with(|m| {
        m.borrow()
            .get(&id)
            .map(|inner| {
                inner
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    });
    for (k, v) in snapshot {
        call_fn(func, vec![v, Value::String(k), map_val.clone()], env)?;
    }
    Ok(Value::Null)
}

fn set_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_SET.fetch_add(1, Ordering::Relaxed);
    SETS.with(|s| {
        s.borrow_mut().insert(id, HashMap::new());
    });
    Ok(set_object(id))
}

fn set_add_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_add(set, value)")?)?;
    let item = args.get(1).ok_or("set_add(set, value)")?;
    let key = value_key(item);
    SETS.with(|s| {
        let mut set = s.borrow_mut();
        let inner = set.get_mut(&id).ok_or("unknown set")?;
        if inner.contains_key(&key) {
            Ok(Value::Bool(false))
        } else {
            inner.insert(key, item.clone());
            Ok(Value::Bool(true))
        }
    })
}

fn set_has_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_has(set, value)")?)?;
    let item = args.get(1).ok_or("set_has(set, value)")?;
    let key = value_key(item);
    SETS.with(|s| {
        Ok(Value::Bool(
            s.borrow()
                .get(&id)
                .map(|inner| inner.contains_key(&key))
                .unwrap_or(false),
        ))
    })
}

fn set_delete_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_delete(set, value)")?)?;
    let item = args.get(1).ok_or("set_delete(set, value)")?;
    let key = value_key(item);
    SETS.with(|s| {
        Ok(Value::Bool(
            s.borrow_mut()
                .get_mut(&id)
                .map(|inner| inner.remove(&key).is_some())
                .unwrap_or(false),
        ))
    })
}

fn set_clear_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_clear(set)")?)?;
    SETS.with(|s| {
        if let Some(inner) = s.borrow_mut().get_mut(&id) {
            inner.clear();
        }
        Ok(Value::Null)
    })
}

fn set_size_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_size(set)")?)?;
    SETS.with(|s| {
        Ok(Value::Number(
            s.borrow()
                .get(&id)
                .map(|inner| inner.len() as i64)
                .unwrap_or(0),
        ))
    })
}

fn set_values_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = set_id(args.first().ok_or("set_values(set)")?)?;
    SETS.with(|s| {
        let values: Vec<Value> = s
            .borrow()
            .get(&id)
            .map(|inner| inner.values().cloned().collect())
            .unwrap_or_default();
        Ok(Value::Array(values))
    })
}

fn set_for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let set_val = args.first().ok_or("set_for_each(set, fn)")?.clone();
    let id = set_id(&set_val)?;
    let func = args.get(1).ok_or("set_for_each(set, fn)")?;
    let items: Vec<Value> = set_values_list(id);
    for item in items {
        call_fn(func, vec![item.clone(), item, set_val.clone()], env)?;
    }
    Ok(Value::Null)
}

fn set_values_list(id: u64) -> Vec<Value> {
    SETS.with(|s| {
        s.borrow()
            .get(&id)
            .map(|inner| inner.values().cloned().collect())
            .unwrap_or_default()
    })
}

/// Values yielded by `for...of` / `Set` iteration.
pub fn set_iteration_values(v: &Value) -> Result<Vec<Value>, String> {
    Ok(set_values_list(set_id(v)?))
}

/// Map key order snapshot for lazy `Map` iterators.
pub(crate) fn map_key_list(id: u64) -> Vec<String> {
    MAPS.with(|m| {
        m.borrow()
            .get(&id)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    })
}

/// Live map lookup by internal id (used while stepping a `Map` iterator).
pub(crate) fn map_get_at_id(id: u64, key: &str) -> Option<Value> {
    MAPS.with(|m| {
        m.borrow()
            .get(&id)
            .and_then(|inner| inner.get(key).cloned())
    })
}

/// Values snapshot for lazy `Set` iterators.
pub(crate) fn set_values_for_iteration(id: u64) -> Vec<Value> {
    set_values_list(id)
}

/// Map entries as `[key, value]` arrays for `for...of` / `Map` iteration.
pub fn map_iteration_entries(v: &Value) -> Result<Vec<Value>, String> {
    let id = map_id(v)?;
    Ok(MAPS.with(|m| {
        m.borrow()
            .get(&id)
            .map(|inner| {
                inner
                    .iter()
                    .map(|(k, val)| {
                        Value::Array(vec![Value::String(k.clone()), val.clone()])
                    })
                    .collect()
            })
            .unwrap_or_default()
    }))
}

fn set_from_values(values: Vec<Value>) -> Value {
    let id = NEXT_SET.fetch_add(1, Ordering::Relaxed);
    let mut inner = HashMap::new();
    for v in values {
        let key = value_key(&v);
        if !inner.contains_key(&key) {
            inner.insert(key, v);
        }
    }
    SETS.with(|s| s.borrow_mut().insert(id, inner));
    set_object(id)
}

fn set_union_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_union(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_union(a, b)")?)?;
    let mut out = set_values_list(a);
    for v in set_values_list(b) {
        let key = value_key(&v);
        if !out.iter().any(|x| value_key(x) == key) {
            out.push(v);
        }
    }
    Ok(set_from_values(out))
}

fn set_intersection_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_intersection(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_intersection(a, b)")?)?;
    let b_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&b)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let out: Vec<Value> = set_values_list(a)
        .into_iter()
        .filter(|v| b_keys.contains(&value_key(v)))
        .collect();
    Ok(set_from_values(out))
}

fn set_difference_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_difference(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_difference(a, b)")?)?;
    let b_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&b)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let out: Vec<Value> = set_values_list(a)
        .into_iter()
        .filter(|v| !b_keys.contains(&value_key(v)))
        .collect();
    Ok(set_from_values(out))
}

fn set_is_subset_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_is_subset(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_is_subset(a, b)")?)?;
    let b_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&b)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let ok = set_values_list(a)
        .iter()
        .all(|v| b_keys.contains(&value_key(v)));
    Ok(Value::Bool(ok))
}

fn set_is_superset_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_is_superset(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_is_superset(a, b)")?)?;
    let a_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&a)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let ok = set_values_list(b)
        .iter()
        .all(|v| a_keys.contains(&value_key(v)));
    Ok(Value::Bool(ok))
}

fn set_is_disjoint_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_is_disjoint(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_is_disjoint(a, b)")?)?;
    let b_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&b)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let ok = set_values_list(a)
        .iter()
        .all(|v| !b_keys.contains(&value_key(v)));
    Ok(Value::Bool(ok))
}

fn set_symmetric_difference_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = set_id(args.first().ok_or("set_symmetric_difference(a, b)")?)?;
    let b = set_id(args.get(1).ok_or("set_symmetric_difference(a, b)")?)?;
    let b_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&b)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let a_keys: HashSet<String> = SETS.with(|s| {
        s.borrow()
            .get(&a)
            .map(|inner| inner.keys().cloned().collect())
            .unwrap_or_default()
    });
    let out: Vec<Value> = set_values_list(a)
        .into_iter()
        .filter(|v| !b_keys.contains(&value_key(v)))
        .chain(
            set_values_list(b)
                .into_iter()
                .filter(|v| !a_keys.contains(&value_key(v))),
        )
        .collect();
    Ok(set_from_values(out))
}

fn group_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => crate::value::format_value(other),
    }
}

fn map_group_by_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(xs)) => xs,
        _ => return Err("map_group_by(items, fn) expects array".into()),
    };
    let func = args.get(1).ok_or("map_group_by(items, fn)")?;
    let id = NEXT_MAP.fetch_add(1, Ordering::Relaxed);
    let mut inner: HashMap<String, Value> = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        let key_v = call_fn(func, vec![item.clone(), Value::Number(i as i64)], env)?;
        let key = group_key(&key_v);
        match inner.get_mut(&key) {
            Some(Value::Array(bucket)) => bucket.push(item.clone()),
            _ => {
                inner.insert(key, Value::Array(vec![item.clone()]));
            }
        }
    }
    MAPS.with(|m| m.borrow_mut().insert(id, inner));
    Ok(map_object(id))
}

/// `for...of` with optional `Symbol.iterator` (id 1).
pub fn for_of_items_with_env(v: &Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    let owned_target;
    let effective: &Value = if let Some(target) =
        crate::runtime::stdlib::proxy::proxy_target_for_iteration(v)
    {
        owned_target = target;
        &owned_target
    } else {
        v
    };
    if let Value::Object(map) = effective {
        if let Ok(Some(iter)) =
            crate::runtime::stdlib::descriptor::get_own_symbol(map, 1, effective, env)
        {
            let result = match call_fn(&iter, vec![], env) {
                Ok(val) => val,
                Err(_) => call_fn(&iter, vec![effective.clone()], env)?,
            };
            if let Value::Array(items) = result {
                return Ok(items);
            }
            let mut iter_obj = crate::runtime::stdlib::iterator::normalize_iterator(result)?;
            return crate::runtime::stdlib::iterator::iterator_collect(&mut iter_obj, env);
        }
    }
    if let Some(mut iter) = crate::runtime::stdlib::iterator::builtin_iterator(effective) {
        return crate::runtime::stdlib::iterator::iterator_collect(&mut iter, env);
    }
    if crate::runtime::stdlib::iterator::is_iterator_value(effective) {
        let mut iter = effective.clone();
        return crate::runtime::stdlib::iterator::iterator_collect(&mut iter, env);
    }
    if let Some(mut iter) = crate::runtime::stdlib::iterator::object_with_next_iterator(effective) {
        return crate::runtime::stdlib::iterator::iterator_collect(&mut iter, env);
    }
    for_of_items(effective)
}

/// Materialize `for x of iterable` as an array (Map entries, Set values, etc.).
pub fn for_of_items(v: &Value) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(items) => Ok(items.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ if is_map_value(v) => {
            let id = map_id(v)?;
            Ok(MAPS.with(|m| {
                m.borrow()
                    .get(&id)
                    .map(|inner| {
                        inner
                            .iter()
                            .map(|(k, val)| {
                                Value::Array(vec![Value::String(k.clone()), val.clone()])
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }))
        }
        _ if is_set_value(v) => {
            let id = set_id(v)?;
            Ok(set_values_list(id))
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map
                .keys()
                .filter(|k| !k.starts_with("__kab_"))
                .cloned()
                .collect();
            keys.sort();
            Ok(keys
                .into_iter()
                .filter_map(|k| map.get(&k).cloned())
                .collect())
        }
        Value::Range { start, end, step } => range_items(*start, *end, *step),
        _ => Err("for-of requires array, string, object, map, set, or range".into()),
    }
}

fn for_await_of_items_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("for_await_of_items(iterable)")?;
    Ok(Value::Array(
        crate::runtime::stdlib::async_iterator::for_await_of_items_with_env(v, env)?,
    ))
}

fn for_of_items_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("for_of_items(iterable)")?;
    Ok(Value::Array(for_of_items_with_env(v, env)?))
}

fn iterator_from_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_from(iterable)")?;
    crate::runtime::stdlib::iterator::iterator_from_iterable(v, env)
}

fn iterator_from_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_from_async(asyncIterable)")?;
    crate::runtime::stdlib::async_iterator::get_async_iterator(v, env)
}

fn iterator_map_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_map(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_map(iterable, fn)")?.clone();
    crate::runtime::stdlib::iterator::create_map_iterator_from_iterable(v, func, env)
}

fn iterator_filter_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_filter(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_filter(iterable, fn)")?.clone();
    crate::runtime::stdlib::iterator::create_filter_iterator_from_iterable(v, func, env)
}

fn iterator_take_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_take(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("iterator_take(iterable, n) expects non-negative number".into()),
    };
    crate::runtime::stdlib::iterator::create_take_iterator_from_iterable(v, n, env)
}

fn iterator_begin_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_begin(iterable)")?;
    crate::runtime::stdlib::iterator::get_sync_iterator(v, env)
}

fn iterator_step_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator_step(iterator)")?.clone();
    crate::runtime::stdlib::iterator::iterator_step(&mut it, env)
}

fn async_iterator_begin_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("async_iterator_begin(iterable)")?;
    crate::runtime::stdlib::async_iterator::get_async_iterator(v, env)
}

fn async_iterator_step_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("async_iterator_step(iterator)")?.clone();
    crate::runtime::stdlib::async_iterator::async_iterator_step(&mut it, env)
}

fn iterator_close_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator_close(iterator, value?)")?.clone();
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    crate::runtime::stdlib::iterator::iterator_return(&mut it, value, env)
}

fn async_iterator_close_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args
        .first()
        .ok_or("async_iterator_close(iterator, value?)")?
        .clone();
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    crate::runtime::stdlib::async_iterator::async_iterator_close(&mut it, value, env)
}

fn iterator_pack_collect(iter: Value, env: &mut Environment) -> Result<Value, String> {
    let mut iter = iter;
    Ok(Value::Array(
        crate::runtime::stdlib::iterator::iterator_collect(&mut iter, env)?,
    ))
}

fn iterator_skip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_skip(iterable, n)")?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n,
        _ => return Err("iterator_skip(iterable, n) expects non-negative number".into()),
    };
    iterator_pack_collect(
        crate::runtime::stdlib::iterator::create_skip_iterator_from_iterable(v, n, env)?,
        env,
    )
}

fn iterator_zip_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("iterator_zip(a, b)")?;
    let b = args.get(1).ok_or("iterator_zip(a, b)")?;
    iterator_pack_collect(
        crate::runtime::stdlib::iterator::create_zip_iterator_from_iterables(a, b, env)?,
        env,
    )
}

fn iterator_enumerate_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_enumerate(iterable)")?;
    iterator_pack_collect(
        crate::runtime::stdlib::iterator::create_enumerate_iterator_from_iterable(v, env)?,
        env,
    )
}

fn iterator_chain_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("iterator_chain(iterable, ...) expects at least one iterable".into());
    }
    iterator_pack_collect(
        crate::runtime::stdlib::iterator::create_chain_iterator_from_iterables(args, env)?,
        env,
    )
}

fn iterator_flat_map_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_flat_map(iterable, fn, depth?)")?;
    let func = args.get(1).ok_or("iterator_flat_map(iterable, fn, depth?)")?.clone();
    let depth = match args.get(2) {
        Some(Value::Number(n)) if *n >= 1 => *n,
        _ => 1,
    };
    crate::runtime::stdlib::iterator::create_flat_map_iterator_from_iterable(v, func, depth, env)
}

fn iterator_drop_while_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_drop_while(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_drop_while(iterable, fn)")?.clone();
    crate::runtime::stdlib::iterator::create_drop_while_iterator_from_iterable(v, func, env)
}

fn iterator_take_while_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_take_while(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_take_while(iterable, fn)")?.clone();
    crate::runtime::stdlib::iterator::create_take_while_iterator_from_iterable(v, func, env)
}

fn iterator_to_array_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator_to_array(iterator)")?.clone();
    Ok(Value::Array(
        crate::runtime::stdlib::iterator::iterator_collect(&mut it, env)?,
    ))
}

fn iterator_reduce_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut it = args.first().ok_or("iterator_reduce(iterator, fn, initial?)")?.clone();
    let func = args.get(1).ok_or("iterator_reduce(iterator, fn, initial?)")?.clone();
    let initial = args.get(2).cloned();
    crate::runtime::stdlib::iterator::iterator_reduce(&mut it, func, initial, env)
}

fn iterator_for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_for_each(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_for_each(iterable, fn)")?.clone();
    let mut it = crate::runtime::stdlib::iterator::get_sync_iterator(v, env)?;
    crate::runtime::stdlib::iterator::iterator_for_each(&mut it, func, env)
}

fn iterator_find_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_find(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_find(iterable, fn)")?.clone();
    let mut it = crate::runtime::stdlib::iterator::get_sync_iterator(v, env)?;
    crate::runtime::stdlib::iterator::iterator_find(&mut it, func, env)
}

fn iterator_find_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_find_index(iterable, fn)")?;
    let func = args.get(1).ok_or("iterator_find_index(iterable, fn)")?.clone();
    let mut it = crate::runtime::stdlib::iterator::get_sync_iterator(v, env)?;
    crate::runtime::stdlib::iterator::iterator_find_index(&mut it, func, env)
}

fn iterator_includes_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_includes(iterable, value)")?;
    let needle = args.get(1).ok_or("iterator_includes(iterable, value)")?.clone();
    let mut it = crate::runtime::stdlib::iterator::get_sync_iterator(v, env)?;
    crate::runtime::stdlib::iterator::iterator_includes(&mut it, needle, env)
}

fn iterator_accumulate_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_accumulate(iterable, fn, initial?)")?;
    let func = args.get(1).ok_or("iterator_accumulate(iterable, fn, initial?)")?.clone();
    let initial = args.get(2).cloned();
    crate::runtime::stdlib::iterator::create_accumulate_iterator_from_iterable(v, func, initial, env)
}

fn iterator_pairwise_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("iterator_pairwise(iterable)")?;
    crate::runtime::stdlib::iterator::create_pairwise_iterator_from_iterable(v, env)
}

pub fn register_map_set(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("map_new", map_new_native),
        ("map_set", map_set_native),
        ("map_get", map_get_native),
        ("map_has", map_has_native),
        ("map_delete", map_delete_native),
        ("map_clear", map_clear_native),
        ("map_size", map_size_native),
        ("map_keys", map_keys_native),
        ("map_values", map_values_native),
        ("map_entries", map_entries_native),
        ("map_from_entries", map_from_entries_native),
        ("map_for_each", map_for_each_native),
        ("set_new", set_new_native),
        ("set_add", set_add_native),
        ("set_has", set_has_native),
        ("set_delete", set_delete_native),
        ("set_clear", set_clear_native),
        ("set_size", set_size_native),
        ("set_values", set_values_native),
        ("set_for_each", set_for_each_native),
        ("set_union", set_union_native),
        ("set_intersection", set_intersection_native),
        ("set_difference", set_difference_native),
        ("set_is_subset", set_is_subset_native),
        ("set_is_superset", set_is_superset_native),
        ("set_is_disjoint", set_is_disjoint_native),
        ("set_symmetric_difference", set_symmetric_difference_native),
        ("map_group_by", map_group_by_native),
        ("for_of_items", for_of_items_native),
        ("for_await_of_items", for_await_of_items_native),
        ("array_from_async", crate::runtime::stdlib::async_iterator::array_from_async_native),
        ("iterator_from", iterator_from_native),
        ("iterator_from_async", iterator_from_async_native),
        ("iterator_begin", iterator_begin_native),
        ("iterator_step", iterator_step_native),
        ("iterator_close", iterator_close_native),
        ("async_iterator_begin", async_iterator_begin_native),
        ("async_iterator_step", async_iterator_step_native),
        ("async_iterator_close", async_iterator_close_native),
        ("iterator_map", iterator_map_native),
        ("iterator_filter", iterator_filter_native),
        ("iterator_take", iterator_take_native),
        ("iterator_skip", iterator_skip_native),
        ("iterator_zip", iterator_zip_native),
        ("iterator_enumerate", iterator_enumerate_native),
        ("iterator_chain", iterator_chain_native),
        ("iterator_flat_map", iterator_flat_map_native),
        ("iterator_drop_while", iterator_drop_while_native),
        ("iterator_take_while", iterator_take_while_native),
        ("iterator_to_array", iterator_to_array_native),
        ("iterator_reduce", iterator_reduce_native),
        ("iterator_for_each", iterator_for_each_native),
        ("iterator_find", iterator_find_native),
        ("iterator_find_index", iterator_find_index_native),
        ("iterator_includes", iterator_includes_native),
        ("iterator_accumulate", iterator_accumulate_native),
        ("iterator_pairwise", iterator_pairwise_native),
        ("range", range_native),
        ("counter_new", counter_new_native),
        ("counter_inc", counter_inc_native),
        ("counter_get", counter_get_native),
        ("counter_items", counter_items_native),
        ("defaultdict_new", defaultdict_new_native),
        ("defaultdict_get", defaultdict_get_native),
        ("defaultdict_set", defaultdict_set_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

pub fn is_map_value(v: &Value) -> bool {
    map_id(v).is_ok()
}

pub fn is_set_value(v: &Value) -> bool {
    set_id(v).is_ok()
}
