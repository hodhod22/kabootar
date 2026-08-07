//! Array helpers — parity with JS/Rust iterators.

use crate::value::{format_value, Environment, Value};

fn call_fn(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env)
}

fn array_arg(v: &Value) -> Result<&Vec<Value>, String> {
    match v {
        Value::Array(items) => Ok(items),
        _ => Err("expected array".into()),
    }
}

fn reduce_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("reduce(arr, fn, initial?)")?)?;
    let func = args.get(1).ok_or("reduce(arr, fn, initial?)")?;
    let mut i = 0usize;
    let mut acc = if let Some(init) = args.get(2) {
        init.clone()
    } else {
        let first = items.first().ok_or("reduce() empty array without initial")?;
        i = 1;
        first.clone()
    };
    while i < items.len() {
        acc = call_fn(func, vec![acc, items[i].clone()], env)?;
        i += 1;
    }
    Ok(acc)
}

fn reduce_right_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("reduce_right(arr, fn, initial?)")?)?;
    let func = args.get(1).ok_or("reduce_right(arr, fn, initial?)")?;
    let len = items.len();
    let mut i = len;
    let mut acc = if let Some(init) = args.get(2) {
        init.clone()
    } else {
        let last = items.last().ok_or("reduce_right() empty array without initial")?;
        i = len - 1;
        last.clone()
    };
    while i > 0 {
        i -= 1;
        acc = call_fn(func, vec![acc, items[i].clone()], env)?;
    }
    Ok(acc)
}

fn for_each_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("for_each(arr, fn)")?)?;
    let func = args.get(1).ok_or("for_each(arr, fn)")?;
    for (idx, item) in items.iter().enumerate() {
        call_fn(func, vec![item.clone(), Value::Number(idx as i64)], env)?;
    }
    Ok(Value::Null)
}

fn find_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("find(arr, fn)")?)?;
    let func = args.get(1).ok_or("find(arr, fn)")?;
    for item in items.iter() {
        if call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(item.clone());
        }
    }
    Ok(Value::Undefined)
}

fn find_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("find_index(arr, fn)")?)?;
    let func = args.get(1).ok_or("find_index(arr, fn)")?;
    for (i, item) in items.iter().enumerate() {
        if call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(Value::Number(i as i64));
        }
    }
    Ok(Value::Number(-1))
}

fn slice_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("slice(arr, start, end?)")?)?;
    let len = items.len() as i64;
    let start = match args.get(1) {
        Some(Value::Number(n)) => normalize_index(*n, len),
        _ => 0,
    };
    let end = match args.get(2) {
        Some(Value::Number(n)) => normalize_index(*n, len),
        _ => len,
    };
    let (s, e) = if start <= end {
        (start as usize, end as usize)
    } else {
        (end as usize, start as usize)
    };
    let s = s.min(items.len());
    let e = e.min(items.len());
    Ok(Value::from_array(items[s..e].to_vec()))
}

fn normalize_index(i: i64, len: i64) -> i64 {
    if i < 0 {
        (len + i).max(0)
    } else {
        i.min(len)
    }
}

fn concat_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut out = Vec::new();
    for arg in args.iter() {
        match arg {
            Value::Array(items) => out.extend(items.iter().cloned()),
            other => out.push(other.clone()),
        }
    }
    Ok(Value::from_array(out))
}

fn includes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("includes(arr, item)")?)?;
    let needle = args.get(1).ok_or("includes(arr, item)")?;
    Ok(Value::Bool(
        items
            .iter()
            .any(|v| format_value(v) == format_value(needle)),
    ))
}

fn some_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("some(arr, fn)")?)?;
    let func = args.get(1).ok_or("some(arr, fn)")?;
    for item in items.iter() {
        if call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn every_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("every(arr, fn)")?)?;
    let func = args.get(1).ok_or("every(arr, fn)")?;
    for item in items.iter() {
        if !call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn flat_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("flat(arr, depth?)")?)?;
    let depth = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 1,
    };
    Ok(Value::from_array(flatten(items, depth)))
}

pub(crate) fn flatten_values(items: &[Value], depth: usize) -> Vec<Value> {
    flatten(items, depth)
}

fn flatten(items: &[Value], depth: usize) -> Vec<Value> {
    if depth == 0 {
        return items.to_vec();
    }
    let mut out = Vec::new();
    for v in items.iter() {
        if let Value::Array(inner) = v {
            out.extend(flatten(inner, depth.saturating_sub(1)));
        } else {
            out.push(v.clone());
        }
    }
    out
}

fn index_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("index_of(arr, item, from?)")?)?;
    let needle = args.get(1).ok_or("index_of(arr, item, from?)")?;
    let from = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    for (i, v) in items.iter().enumerate().skip(from) {
        if format_value(v) == format_value(needle) {
            return Ok(Value::Number(i as i64));
        }
    }
    Ok(Value::Number(-1))
}

pub fn values_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("values(obj)")?;
    match v {
        Value::Object(map) => {
            let mut out = Vec::new();
            for key in crate::runtime::stdlib::descriptor::enumerable_own_keys(map) {
                if let Some(val) =
                    crate::runtime::stdlib::descriptor::get_own_property(map, &key, v, env)?
                {
                    out.push(val);
                }
            }
            Ok(Value::from_array(out))
        }
        Value::Array(items) => Ok(Value::from_array(items.as_ref().clone())),
        _ => Err("values() expects object or array".into()),
    }
}

fn sort_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("sort(arr, compare_fn?)")?)?;
    let mut out = items.clone();
    if let Some(compare) = args.get(1) {
        out.sort_by(|a, b| {
            let cmp = call_fn(compare, vec![a.clone(), b.clone()], env).unwrap_or(Value::Number(0));
            match cmp {
                Value::Number(n) => n.cmp(&0),
                Value::Float(f) => f.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            }
        });
    } else {
        out.sort_by(|a, b| format_value(a).cmp(&format_value(b)));
    }
    Ok(Value::from_array(out))
}

fn reverse_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("reverse(arr)")?)?;
    let mut out = items.clone();
    out.reverse();
    Ok(Value::from_array(out))
}

fn join_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("join(arr, sep?)")?)?;
    let sep = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => ",",
    };
    let parts: Vec<String> = items.iter().map(format_value).collect();
    Ok(Value::String(parts.join(sep)))
}

fn shift_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("shift(arr)")?)?;
    if items.is_empty() {
        return Ok(Value::Undefined);
    }
    let mut out = items.clone();
    let head = out.remove(0);
    Ok(Value::from_array(vec![head, Value::from_array(out)]))
}

fn unshift_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("unshift(arr, ...items)")?)?;
    let mut out = items.clone();
    for item in args.iter().skip(1) {
        out.insert(0, item.clone());
    }
    Ok(Value::from_array(out))
}

fn splice_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("splice(arr, start, delete_count?, ...insert)")?)?;
    let len = items.len() as i64;
    let start = match args.get(1) {
        Some(Value::Number(n)) => normalize_index(*n, len) as usize,
        _ => 0,
    };
    let delete_count = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => (*n as usize).min(items.len().saturating_sub(start)),
        _ => items.len().saturating_sub(start),
    };
    let mut out = items.clone();
    let end = start.saturating_add(delete_count).min(out.len());
    let removed: Vec<Value> = out.drain(start..end).collect();
    for (i, item) in args.iter().skip(3).enumerate() {
        out.insert(start + i, item.clone());
    }
    Ok(Value::from_array(vec![Value::from_array(out), Value::from_array(removed)]))
}

fn flat_map_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("flat_map(arr, fn)")?)?;
    let func = args.get(1).ok_or("flat_map(arr, fn)")?;
    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let mapped = call_fn(func, vec![item.clone(), Value::Number(idx as i64)], env)?;
        match mapped {
            Value::Array(inner) => out.extend(inner.iter().cloned()),
            other => out.push(other),
        }
    }
    Ok(Value::from_array(out))
}

fn last_index_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("last_index_of(arr, item, from?)")?)?;
    let needle = args.get(1).ok_or("last_index_of(arr, item, from?)")?;
    let from = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => (*n as usize).min(items.len().saturating_sub(1)),
        _ => items.len().saturating_sub(1),
    };
    for i in (0..=from).rev() {
        if format_value(&items[i]) == format_value(needle) {
            return Ok(Value::Number(i as i64));
        }
    }
    Ok(Value::Number(-1))
}

fn find_last_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("find_last(arr, fn)")?)?;
    let func = args.get(1).ok_or("find_last(arr, fn)")?;
    for item in items.iter().rev() {
        if call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(item.clone());
        }
    }
    Ok(Value::Undefined)
}

fn find_last_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("find_last_index(arr, fn)")?)?;
    let func = args.get(1).ok_or("find_last_index(arr, fn)")?;
    for (i, item) in items.iter().enumerate().rev() {
        if call_fn(func, vec![item.clone()], env)?.is_truthy() {
            return Ok(Value::Number(i as i64));
        }
    }
    Ok(Value::Number(-1))
}

fn at_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("at(arr, index)")?)?;
    let len = items.len() as i64;
    let idx = match args.get(1) {
        Some(Value::Number(n)) if *n < 0 => len + *n,
        Some(Value::Number(n)) => *n,
        _ => 0,
    };
    if idx < 0 || idx >= len {
        Ok(Value::Undefined)
    } else {
        Ok(items[idx as usize].clone())
    }
}

fn fill_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("fill(arr, value, start?, end?)")?)?;
    let value = args
        .get(1)
        .cloned()
        .unwrap_or(Value::Undefined);
    let len = items.len() as i64;
    let start = match args.get(2) {
        Some(Value::Number(n)) => normalize_index(*n, len),
        _ => 0,
    };
    let end = match args.get(3) {
        Some(Value::Number(n)) => normalize_index(*n, len),
        _ => len,
    };
    let mut out = items.clone();
    let s = start.max(0) as usize;
    let e = end.max(0).min(len) as usize;
    for slot in out.iter_mut().take(e).skip(s) {
        *slot = value.clone();
    }
    Ok(Value::from_array(out))
}

fn copy_within_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("copy_within(arr, target, start, end?)")?)?;
    let len = items.len();
    let target = match args.get(1) {
        Some(Value::Number(n)) => normalize_index(*n, len as i64) as usize,
        _ => 0,
    };
    let start = match args.get(2) {
        Some(Value::Number(n)) => normalize_index(*n, len as i64) as usize,
        _ => 0,
    };
    let end = match args.get(3) {
        Some(Value::Number(n)) => normalize_index(*n, len as i64) as usize,
        _ => len,
    };
    let mut out = items.clone();
    let slice_end = end.min(len);
    if start >= slice_end {
        return Ok(Value::from_array(out));
    }
    let copied: Vec<Value> = out[start..slice_end].to_vec();
    for (offset, value) in copied.into_iter().enumerate() {
        let dest = target + offset;
        if dest < len {
            out[dest] = value;
        }
    }
    Ok(Value::from_array(out))
}

fn to_spliced_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    match splice_native(args, env)? {
        Value::Array(parts) => Ok(parts
            .first()
            .cloned()
            .unwrap_or(Value::from_array(Vec::new()))),
        other => Ok(other),
    }
}

fn array_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::from_array(args.to_vec()))
}

fn array_from_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let src = args.first().ok_or("array_from(source, map_fn?)")?;
    let mut items: Vec<Value> = match src {
        Value::Array(xs) => xs.as_ref().clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("array_from() expects array or string".into()),
    };
    if let Some(func) = args.get(1) {
        let mut mapped = Vec::with_capacity(items.len());
        for (i, item) in items.drain(..).enumerate() {
            mapped.push(call_fn(func, vec![item, Value::Number(i as i64)], env)?);
        }
        items = mapped;
    }
    Ok(Value::from_array(items))
}

fn array_with_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("array_with(arr, index, value)")?)?;
    let len = items.len() as i64;
    let idx = match args.get(1) {
        Some(Value::Number(n)) if *n < 0 => len + *n,
        Some(Value::Number(n)) => *n,
        _ => 0,
    };
    let value = args.get(2).cloned().unwrap_or(Value::Undefined);
    if idx < 0 || idx >= len {
        return Err("array_with() index out of bounds".into());
    }
    let mut out = items.clone();
    out[idx as usize] = value;
    Ok(Value::from_array(out))
}

pub fn entries_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("entries(obj)")?;
    match v {
        Value::Object(map) => {
            let mut out = Vec::new();
            for key in crate::runtime::stdlib::descriptor::enumerable_own_keys(map) {
                if let Some(val) =
                    crate::runtime::stdlib::descriptor::get_own_property(map, &key, v, env)?
                {
                    out.push(Value::from_array(vec![Value::String(key), val]));
                }
            }
            Ok(Value::from_array(out))
        }
        Value::Array(items) => Ok(Value::from_array(
            items
                .iter()
                .enumerate()
                .map(|(i, v)| Value::from_array(vec![Value::Number(i as i64), v.clone()]))
                .collect(),
        )),
        _ => Err("entries() expects object or array".into()),
    }
}

pub fn array_to_locale_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("array_to_locale_string(arr)")?)?;
    let parts: Vec<String> = items.iter().map(crate::value::format_value).collect();
    Ok(Value::String(parts.join(",")))
}

pub fn array_to_locale_string_method(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    array_to_locale_string_native(args, env)
}

pub fn register_array(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("reduce", reduce_native),
        ("array_reduce", reduce_native),
        ("reduce_right", reduce_right_native),
        ("array_reduce_right", reduce_right_native),
        ("for_each", for_each_native),
        ("find", find_native),
        ("array_find", find_native),
        ("find_index", find_index_native),
        ("array_find_index", find_index_native),
        ("slice", slice_native),
        ("array_slice", slice_native),
        ("concat", concat_native),
        ("array_concat", concat_native),
        ("includes", includes_native),
        ("array_includes", includes_native),
        ("some", some_native),
        ("array_some", some_native),
        ("every", every_native),
        ("array_every", every_native),
        ("flat", flat_native),
        ("array_flat", flat_native),
        ("flat_map", flat_map_native),
        ("array_flat_map", flat_map_native),
        ("last_index_of", last_index_of_native),
        ("find_last", find_last_native),
        ("find_last_index", find_last_index_native),
        ("index_of", index_of_native),
        ("array_index_of", index_of_native),
        ("sort", sort_native),
        ("to_sorted", sort_native),
        ("array_sort", sort_native),
        ("reverse", reverse_native),
        ("to_reversed", reverse_native),
        ("array_reverse", reverse_native),
        ("join", join_native),
        ("array_join", join_native),
        ("shift", shift_native),
        ("unshift", unshift_native),
        ("splice", splice_native),
        ("at", at_native),
        ("fill", fill_native),
        ("copy_within", copy_within_native),
        ("to_spliced", to_spliced_native),
        ("array_from", array_from_native),
        ("array_of", array_of_native),
        ("array_with", array_with_native),
        ("values", values_native),
        ("entries", entries_native),
        ("array_to_locale_string", array_to_locale_string_native),
        ("to_locale_string", array_to_locale_string_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
