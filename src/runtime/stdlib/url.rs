//! `URL` and `URLSearchParams` — lightweight JS parity.

use crate::runtime::net::parse_url;
use crate::runtime::stdlib::encoding;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_USP: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static USP_STORE: RefCell<HashMap<u64, Vec<(String, String)>>> = RefCell::new(HashMap::new());
}

fn split_path_query_hash(path: &str) -> (String, String, String) {
    let (without_hash, hash) = match path.split_once('#') {
        Some((p, h)) => (p, format!("#{h}")),
        None => (path, String::new()),
    };
    let (pathname, search) = match without_hash.split_once('?') {
        Some((p, q)) => (p.to_string(), format!("?{q}")),
        None => (without_hash.to_string(), String::new()),
    };
    (pathname, search, hash)
}

fn protocol_name(scheme: crate::runtime::net::Scheme) -> &'static str {
    match scheme {
        crate::runtime::net::Scheme::Http => "http:",
        crate::runtime::net::Scheme::Https => "https:",
    }
}

fn parse_query_pairs(search: &str) -> Vec<(String, String)> {
    let q = search.strip_prefix('?').unwrap_or(search);
    if q.is_empty() {
        return Vec::new();
    }
    q.split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some((k, v)) = part.split_once('=') {
                (
                    percent_decode(k),
                    percent_decode(v),
                )
            } else {
                (percent_decode(part), String::new())
            }
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    encoding::decode_uri_component(s).unwrap_or_else(|_| s.to_string())
}

fn percent_encode(s: &str) -> String {
    encoding::encode_uri_component(s)
}

fn usp_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_usp".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

fn usp_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected URLSearchParams".into());
    };
    if !matches!(o.get("__kab_usp"), Some(Value::Bool(true))) {
        return Err("expected URLSearchParams".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid URLSearchParams handle".into()),
    }
}

fn usp_pairs(id: u64) -> Result<Vec<(String, String)>, String> {
    USP_STORE
        .with(|m| m.borrow().get(&id).cloned())
        .ok_or_else(|| format!("invalid URLSearchParams id {id}"))
}

fn usp_set_pairs(id: u64, pairs: Vec<(String, String)>) {
    USP_STORE.with(|m| {
        m.borrow_mut().insert(id, pairs);
    });
}

fn str_arg(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => Ok(crate::value::format_value(other)),
    }
}

fn url_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let href = str_arg(args.first().ok_or("url_new(href)")?)?;
    let parsed = parse_url(&href)?;
    let (pathname, search, hash) = split_path_query_hash(&parsed.path);
    let protocol = protocol_name(parsed.scheme);
    let mut obj = HashMap::new();
    obj.insert("href".into(), Value::String(href.clone()));
    obj.insert("protocol".into(), Value::String(protocol.into()));
    obj.insert("hostname".into(), Value::String(parsed.host.clone()));
    obj.insert("host".into(), Value::String(format!("{}:{}", parsed.host, parsed.port)));
    obj.insert("port".into(), Value::String(parsed.port.to_string()));
    obj.insert("pathname".into(), Value::String(pathname));
    obj.insert("search".into(), Value::String(search.clone()));
    obj.insert("hash".into(), Value::String(hash));
    let usp = url_search_params_new_from_search(&search)?;
    obj.insert("searchParams".into(), usp);
    Ok(Value::Object(obj))
}

fn url_search_params_new_from_search(search: &str) -> Result<Value, String> {
    let id = NEXT_USP.fetch_add(1, Ordering::Relaxed);
    usp_set_pairs(id, parse_query_pairs(search));
    Ok(usp_object(id))
}

fn url_search_params_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let init = match args.first() {
        Some(v) => str_arg(v)?,
        None => String::new(),
    };
    let search = init.strip_prefix('?').unwrap_or(&init);
    url_search_params_new_from_search(&format!("?{search}"))
}

fn usp_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let usp = args.first().ok_or("usp_get(usp, key)")?;
    let key = str_arg(args.get(1).ok_or("usp_get(usp, key)")?)?;
    let id = usp_id(usp)?;
    let pairs = usp_pairs(id)?;
    for (k, v) in pairs {
        if k == key {
            return Ok(Value::String(v));
        }
    }
    Ok(Value::Null)
}

fn usp_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let usp = args.first().ok_or("usp_set(usp, key, value)")?;
    let key = str_arg(args.get(1).ok_or("usp_set(usp, key, value)")?)?;
    let value = str_arg(args.get(2).ok_or("usp_set(usp, key, value)")?)?;
    let id = usp_id(usp)?;
    let mut pairs = usp_pairs(id)?;
    let mut found = false;
    for (k, v) in &mut pairs {
        if *k == key {
            *v = value.clone();
            found = true;
            break;
        }
    }
    if !found {
        pairs.push((key, value));
    }
    usp_set_pairs(id, pairs);
    Ok(Value::Undefined)
}

fn usp_to_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let usp = args.first().ok_or("usp_to_string(usp)")?;
    let id = usp_id(usp)?;
    let pairs = usp_pairs(id)?;
    let body = pairs
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                percent_encode(k)
            } else {
                format!("{}={}", percent_encode(k), percent_encode(v))
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    Ok(Value::String(body))
}

pub fn register_url(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("url_new", url_new_native),
        ("url_search_params_new", url_search_params_new_native),
        ("usp_get", usp_get_native),
        ("usp_set", usp_set_native),
        ("usp_to_string", usp_to_string_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
