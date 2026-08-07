//! `fetch` — JS Fetch API parity via `http_fetch_async`.

use crate::runtime::http::HttpResponse;
use crate::runtime::io_async::schedule_io_promise;
use crate::runtime::stdlib::abort::{abort_reason, is_aborted, rejected_abort_promise, signal_id};
use crate::value::{format_value, Environment, IoOp, Value};
use std::collections::HashMap;

pub fn response_from_http(res: &HttpResponse) -> Value {
    let mut obj = HashMap::new();
    obj.insert("status".into(), Value::Number(res.status));
    obj.insert(
        "ok".into(),
        Value::Bool(res.status >= 200 && res.status < 300),
    );
    obj.insert("body".into(), Value::String(res.body.clone()));
    let mut headers = HashMap::new();
    for (k, v) in &res.headers {
        headers.insert(k.clone(), Value::String(v.clone()));
    }
    obj.insert("headers".into(), Value::from_object(headers));
    Value::from_object(obj)
}

fn parse_fetch_options(opt: Option<&Value>) -> (String, String, HashMap<String, String>, Option<u64>) {
    let mut method = "GET".to_string();
    let mut body = String::new();
    let mut headers = HashMap::new();
    let mut abort_id = None;
    let Some(Value::Object(map)) = opt else {
        return (method, body, headers, abort_id);
    };
    if let Some(Value::String(m)) = map.get("method") {
        method = m.to_ascii_uppercase();
    }
    if let Some(b) = map.get("body") {
        body = match b {
            Value::String(s) => s.clone(),
            other => format_value(other),
        };
    }
    if let Some(Value::Object(h)) = map.get("headers") {
        for (k, v) in h.iter() {
            if k.starts_with("__kab_") {
                continue;
            }
            headers.insert(
                k.clone(),
                match v {
                    Value::String(s) => s.clone(),
                    other => format_value(other),
                },
            );
        }
    }
    if let Some(sig) = map.get("signal") {
        abort_id = signal_id(sig);
    }
    (method, body, headers, abort_id)
}

fn fetch_map_response_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("fetch response mapper")?;
    match v {
        Value::HttpResponse(res) => Ok(response_from_http(res)), Value::Object(_) => Ok(v.clone()),
        other => Err(format!("fetch expected HttpResponse, got {:?}", other)),
    }
}

fn fetch_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let url = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("fetch(url, options?) expects url string".into()),
    };
    let (method, body, headers, abort_id) = parse_fetch_options(args.get(1));
    if let Some(id) = abort_id {
        if is_aborted(id) {
            return Ok(rejected_abort_promise(abort_reason(id)));
        }
    }
    let timeout_ms = env.http_fetch_timeout_ms();
    let raw = schedule_io_promise(
        IoOp::HttpFetch {
            method,
            url,
            body,
            headers,
            timeout_ms,
        },
        env,
        abort_id,
    );
    crate::runtime::stdlib::promise::promise_then_native(
        &[raw, Value::NativeFunction(fetch_map_response_native)],
        env,
    )
}

fn response_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("response_text(res)")?;
    match v {
        Value::Object(map) => Ok(map
            .get("body")
            .cloned()
            .unwrap_or(Value::String(String::new()))),
        Value::HttpResponse(res) => Ok(Value::String(res.body.clone())),
        _ => Err("response_text() expects fetch response object".into()),
    }
}

fn response_json_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let text = response_text_native(args, env)?;
    let Value::String(s) = text else {
        return Err("response_json() body must be string".into());
    };
    crate::runtime::stdlib::json::parse(&s)
}

fn response_ok_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("response_ok(res)")?;
    match v {
        Value::Object(map) => Ok(map
            .get("ok")
            .cloned()
            .unwrap_or(Value::Bool(false))),
        Value::HttpResponse(res) => Ok(Value::Bool(res.status >= 200 && res.status < 300)),
        _ => Ok(Value::Bool(false)),
    }
}

pub fn register_fetch(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("fetch", fetch_native),
        ("response_text", response_text_native),
        ("response_json", response_json_native),
        ("response_ok", response_ok_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
