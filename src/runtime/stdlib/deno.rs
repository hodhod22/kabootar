//! Deno runtime parity — `Deno.env`, streams, `serve`, WebSocket helpers.

use crate::runtime::http::{HttpRequest, HttpResponse};
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::rc::Rc;

static NEXT_WS: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static KAB_ENV: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    static WS_INBOX: RefCell<HashMap<u64, Vec<String>>> = RefCell::new(HashMap::new());
    static WS_LINKS: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

pub const SERVE_HANDLER_KEY: &str = "__serve_handler";

fn ensure_host_env() {
    KAB_ENV.with(|m| {
        if m.borrow().is_empty() {
            let mut map = HashMap::new();
            for (k, v) in std::env::vars() {
                map.insert(k, v);
            }
            *m.borrow_mut() = map;
        }
    });
}

pub fn request_to_value(req: &HttpRequest) -> Value {
    let mut obj = HashMap::new();
    obj.insert("method".into(), Value::String(req.method.clone()));
    obj.insert("url".into(), Value::String(req.path.clone()));
    obj.insert("path".into(), Value::String(req.path.clone()));
    obj.insert("body".into(), Value::String(req.body.clone()));
    let mut headers = HashMap::new();
    for (k, v) in &req.headers {
        headers.insert(k.clone(), Value::String(v.clone()));
    }
    obj.insert("headers".into(), Value::from_object(headers));
    Value::from_object(obj)
}

pub fn coerce_deno_response(value: Value) -> Result<HttpResponse, String> {
    match value {
        Value::HttpResponse(res) => Ok(res),
        Value::String(body) => Ok(HttpResponse::new(200, body)),
        Value::Number(n) => Ok(HttpResponse::new(200, n.to_string())),
        Value::Null => Ok(HttpResponse::new(204, "")), Value::Object(map) => {
            let status = match map.get("status") {
                Some(Value::Number(n)) => n,
                _ => &200,
            };
            let body = match map.get("body") {
                Some(Value::String(s)) => s.clone(),
                Some(v) => crate::value::format_value(v),
                None => String::new(),
            };
            let mut res = HttpResponse::new(*status, body);
            if let Some(Value::Object(h)) = map.get("headers") {
                for (k, v) in h.iter() {
                    if let Value::String(s) = v {
                        res.headers.insert(k.clone(), s.clone());
                    }
                }
            }
            Ok(res)
        }
        other => Err(format!(
            "handler must return http_response(...) or {{ status, body }}, got {:?}",
            other
        )),
    }
}

fn env_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    ensure_host_env();
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("env_get(key)".into()),
    };
    Ok(KAB_ENV.with(|m| {
        m.borrow()
            .get(key)
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Undefined)
    }))
}

fn env_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    ensure_host_env();
    let key = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("env_set(key, value)".into()),
    };
    let val = match args.get(1) {
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    KAB_ENV.with(|m| {
        m.borrow_mut().insert(key, val);
    });
    Ok(Value::Undefined)
}

fn env_has_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    ensure_host_env();
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("env_has(key)".into()),
    };
    Ok(Value::Bool(KAB_ENV.with(|m| m.borrow().contains_key(key))))
}

fn env_delete_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    ensure_host_env();
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("env_delete(key)".into()),
    };
    KAB_ENV.with(|m| {
        m.borrow_mut().remove(key);
    });
    Ok(Value::Undefined)
}

fn env_to_object_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    ensure_host_env();
    let mut obj = HashMap::new();
    KAB_ENV.with(|m| {
        for (k, v) in m.borrow().iter() {
            obj.insert(k.clone(), Value::String(v.clone()));
        }
    });
    Ok(Value::from_object(obj))
}

pub(crate) fn stream_id_pub(v: &Value) -> Result<u64, String> {
    crate::runtime::web_streams::stream_id_pub(v)
}

pub(crate) fn stream_allocate() -> u64 {
    crate::runtime::web_streams::stream_allocate()
}

pub(crate) fn stream_object_pub(id: u64) -> Value {
    crate::runtime::web_streams::stream_object_pub(id)
}

pub(crate) fn stream_push(id: u64, chunk: Value) {
    crate::runtime::web_streams::stream_push(id, chunk);
}

pub(crate) fn stream_push_capped(id: u64, chunk: Value, max: usize) {
    crate::runtime::web_streams::stream_push_capped(id, chunk, max);
}

pub(crate) fn stream_remove(id: u64) {
    crate::runtime::web_streams::stream_remove(id);
}

pub(crate) fn stream_read_impl(id: u64) -> Result<Value, String> {
    crate::runtime::web_streams::stream_read_impl(id)
}

pub(crate) fn stream_read_all_impl(id: u64) -> Result<Value, String> {
    crate::runtime::web_streams::stream_read_all_impl(id)
}

pub(crate) fn stream_pipe_to_impl(src_id: u64, dest_id: u64) -> Result<(), String> {
    crate::runtime::web_streams::stream_pipe_to_impl(src_id, dest_id)
}

pub(crate) fn writable_id_pub(v: &Value) -> Result<u64, String> {
    crate::runtime::web_streams::writable_id_pub(v)
}

fn stream_from_array_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(a)) => a.as_ref().clone(),
        _ => return Err("stream_from_array(items)".into()),
    };
    Ok(crate::runtime::web_streams::from_array(items))
}

fn stream_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_read(stream)")?)?;
    stream_read_impl(id)
}

fn stream_read_all_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_read_all(stream)")?)?;
    stream_read_all_impl(id)
}

fn stream_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(crate::runtime::web_streams::stream_new())
}

fn stream_from_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    Ok(crate::runtime::web_streams::from_string(text))
}

fn stream_cancel_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_cancel(stream)")?)?;
    crate::runtime::web_streams::stream_cancel(id)?;
    Ok(Value::Undefined)
}

fn stream_abort_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_abort(stream)")?)?;
    let reason = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        other => Some(crate::value::format_value(other)),
    });
    crate::runtime::web_streams::stream_abort(id, reason)?;
    Ok(Value::Undefined)
}

fn stream_state_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_state(stream)")?)?;
    Ok(Value::String(crate::runtime::web_streams::stream_state(id)?))
}

fn stream_enqueue_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_enqueue(stream, chunk)")?)?;
    let chunk = args.get(1).cloned().unwrap_or(Value::Undefined);
    crate::runtime::web_streams::stream_enqueue(id, chunk)?;
    Ok(Value::Undefined)
}

fn stream_close_readable_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::stream_id(args.first().ok_or("stream_close_readable(stream)")?)?;
    crate::runtime::web_streams::stream_close_readable(id)?;
    Ok(Value::Undefined)
}

fn stream_tee_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_tee(stream)")?)?;
    crate::runtime::web_streams::stream_tee(id)
}

fn stream_pipe_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let src_id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_pipe_to(src, dest)")?)?;
    let dest_id =
        crate::runtime::web_streams::writable_id(args.get(1).ok_or("stream_pipe_to(src, dest)")?)?;
    stream_pipe_to_impl(src_id, dest_id)?;
    Ok(Value::Undefined)
}

fn stream_get_reader_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_get_reader(stream)")?)?;
    crate::runtime::web_streams::get_reader(id)
}

fn reader_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::reader_id(args.first().ok_or("reader_read(reader)")?)?;
    crate::runtime::web_streams::reader_read(id)
}

fn reader_release_lock_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::reader_id(args.first().ok_or("reader_release_lock(reader)")?)?;
    crate::runtime::web_streams::reader_release_lock(id)?;
    Ok(Value::Undefined)
}

fn reader_cancel_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::reader_id(args.first().ok_or("reader_cancel(reader)")?)?;
    let reason = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        other => Some(crate::value::format_value(other)),
    });
    crate::runtime::web_streams::reader_cancel(id, reason)?;
    Ok(Value::Undefined)
}

fn transform_stream_new_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let transform = args.first().ok_or("transform_stream_new(transform_fn)")?.clone();
    crate::runtime::web_streams::transform_stream_new(transform, env)
}

fn byte_stream_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(crate::runtime::web_streams::byte_stream_new())
}

fn byte_stream_from_bytes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let bytes: Vec<u8> = match args.first() {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Number(n) if *n >= 0 && *n <= 255 => Some(*n as u8),
                _ => None,
            })
            .collect(),
        _ => return Err("byte_stream_from_bytes(data)".into()),
    };
    Ok(crate::runtime::web_streams::byte_stream_from_bytes(&bytes))
}

fn byte_stream_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("byte_stream_read(stream)")?)?;
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 1024,
    };
    crate::runtime::web_streams::byte_stream_read(id, max)
}

fn byte_stream_byob_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::stream_id(args.first().ok_or("byte_stream_byob_read(stream, buffer)")?)?;
    let Value::Array(items) = args.get(1).ok_or("byte_stream_byob_read(stream, buffer)")? else {
        return Err("byte_stream_byob_read expects buffer array".into());
    };
    let mut buffer: Vec<i64> = items
        .iter()
        .map(|v| match v {
            Value::Number(n) => *n as i64,
            _ => 0,
        })
        .collect();
    let read = crate::runtime::web_streams::byte_stream_byob_read(id, &mut buffer)?;
    let mut out = HashMap::new();
    out.insert("read".into(), Value::Number(read as i64));
    out.insert(
        "buffer".into(), Value::from_array(buffer.into_iter().map(Value::Number).collect()),
    );
    Ok(Value::from_object(out))
}

fn stream_transfer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_transfer(stream)")?)?;
    crate::runtime::web_streams::stream_transfer(id)
}

fn stream_from_transfer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let token = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("stream_from_transfer(token)".into()),
    };
    crate::runtime::web_streams::stream_from_transfer(token)
}

fn tcp_connect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tcp_connect(host, port)".into()),
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("tcp_connect(host, port)".into()),
    };
    Ok(Value::Number(
        crate::runtime::tcp::tcp_connect(host, port)? as i64,
    ))
}

fn tcp_listen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        None => "0.0.0.0",
        _ => return Err("tcp_listen(host, port)".into()),
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("tcp_listen(host, port)".into()),
    };
    Ok(Value::Number(
        crate::runtime::tcp::tcp_listen(host, port)? as i64,
    ))
}

fn tcp_accept_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listener = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_accept(listener)".into()),
    };
    Ok(Value::Number(
        crate::runtime::tcp::tcp_accept(listener)? as i64,
    ))
}

fn tcp_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_read(socket, max?)".into()),
    };
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 4096,
    };
    Ok(Value::String(crate::runtime::tcp::tcp_read(sock, max)?))
}

fn tcp_read_bytes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_read_bytes(socket, max?)".into()),
    };
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 4096,
    };
    let bytes = crate::runtime::tcp::tcp_read_bytes(sock, max)?;
    Ok(Value::from_array(
        bytes.into_iter().map(|b| Value::Number(b as i64)).collect(),
    ))
}

fn tcp_write_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_write(socket, data)".into()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    crate::runtime::tcp::tcp_write(sock, &data)?;
    Ok(Value::Undefined)
}

fn tcp_write_bytes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_write_bytes(socket, bytes)".into()),
    };
    let items = match args.get(1) {
        Some(Value::Array(items)) => items.as_ref().clone(),
        Some(Value::Object(map)) => {
            let n = match map.get("n") {
                Some(Value::Number(n)) if *n >= 0 => *n as usize,
                _ => return Err("tcp_write_bytes(socket, bytes) expects byte values".into()),
            };
            let mut values = Vec::with_capacity(n);
            for i in 0..n {
                values.push(
                    map.get(&i.to_string())
                        .cloned()
                        .ok_or("tcp_write_bytes(socket, bytes) expects byte values")?,
                );
            }
            values
        }
        _ => return Err("tcp_write_bytes(socket, bytes)".into()),
    };
    let mut bytes = Vec::with_capacity(items.len());
    for item in items.iter() {
        match item {
            Value::Number(n) if (0..=255).contains(n) => bytes.push(*n as u8),
            _ => return Err("tcp_write_bytes(socket, bytes) expects byte values".into()),
        }
    }
    crate::runtime::tcp::tcp_write_bytes(sock, &bytes)?;
    Ok(Value::Undefined)
}

fn tcp_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_close(handle)".into()),
    };
    crate::runtime::tcp::tcp_close(handle)?;
    Ok(Value::Undefined)
}

fn tcp_start_tls_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tcp_start_tls(socket, hostname)".into()),
    };
    let hostname = match args.get(1) {
        Some(Value::String(s)) if !s.is_empty() => s.as_str(),
        _ => return Err("tcp_start_tls(socket, hostname) expects hostname string".into()),
    };
    let id = crate::runtime::tcp::tcp_start_tls(sock, hostname, &env.tls_trust())?;
    Ok(Value::Number(id as i64))
}

fn deno_run_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("deno_run(name)".into()),
    };
    Ok(Value::Number(
        crate::runtime::os::spawn_process(env, name)? as i64,
    ))
}

fn run_command_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let program = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("run_command(program, args?)".into()),
    };
    let cmd_args: Vec<String> = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                other => Ok(crate::value::format_value(other)),
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => Vec::new(),
    };
    let out = crate::runtime::host_cmd::run_command(program, &cmd_args)?;
    let mut map = HashMap::new();
    map.insert("code".into(), Value::Number(out.code));
    map.insert("stdout".into(), Value::String(out.stdout));
    map.insert("stderr".into(), Value::String(out.stderr));
    map.insert("success".into(), Value::Bool(out.code == 0));
    Ok(Value::from_object(map))
}

fn chdir_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("chdir(path)".into()),
    };
    crate::runtime::os::host_chdir(path)?;
    Ok(Value::Undefined)
}

fn resolve_dns_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("resolve_dns(host, port?)".into()),
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 && *n < 65536 => *n as u16,
        _ => 80,
    };
    let addrs = crate::runtime::host_cmd::resolve_dns(host, port)?;
    Ok(Value::from_array(
        addrs.into_iter().map(Value::String).collect(),
    ))
}

fn udp_bind_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        None => "0.0.0.0",
        _ => return Err("udp_bind(host, port)".into()),
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 && *n < 65536 => *n as u16,
        _ => return Err("udp_bind(host, port)".into()),
    };
    Ok(Value::Number(
        crate::runtime::udp::udp_bind(host, port)? as i64,
    ))
}

fn udp_local_addr_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("udp_local_addr(socket)".into()),
    };
    Ok(Value::String(crate::runtime::udp::udp_local_addr(sock)?))
}

fn udp_send_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("udp_send(socket, host, port, data)".into()),
    };
    let host = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("udp_send(socket, host, port, data)".into()),
    };
    let port = match args.get(2) {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("udp_send(socket, host, port, data)".into()),
    };
    let data = match args.get(3) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    Ok(Value::Number(
        crate::runtime::udp::udp_send(sock, &host, port, &data)?,
    ))
}

fn udp_recv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("udp_recv(socket, max?)".into()),
    };
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 4096,
    };
    let (data, peer) = crate::runtime::udp::udp_recv(sock, max)?;
    let mut map = HashMap::new();
    map.insert("data".into(), Value::String(data));
    map.insert("peer".into(), Value::String(peer));
    Ok(Value::from_object(map))
}

fn udp_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("udp_close(socket)".into()),
    };
    crate::runtime::udp::udp_close(sock)?;
    Ok(Value::Undefined)
}

fn stream_locked_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_locked(stream)")?)?;
    Ok(Value::Bool(crate::runtime::web_streams::stream_locked(id)))
}

fn stream_lock_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_lock(stream)")?)?;
    crate::runtime::web_streams::stream_lock(id)?;
    Ok(Value::Undefined)
}

fn stream_unlock_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::stream_id(args.first().ok_or("stream_unlock(stream)")?)?;
    crate::runtime::web_streams::stream_unlock(id);
    Ok(Value::Undefined)
}

fn stream_desired_size_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::stream_id(args.first().ok_or("stream_desired_size(stream)")?)?;
    Ok(Value::Number(crate::runtime::web_streams::stream_desired_size(id)?))
}

fn writable_locked_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::writable_id(args.first().ok_or("writable_locked(stream)")?)?;
    Ok(Value::Bool(crate::runtime::web_streams::writable_locked(id)))
}

fn writable_desired_size_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writable_id(
        args.first().ok_or("writable_desired_size(stream)")?,
    )?;
    Ok(Value::Number(
        crate::runtime::web_streams::writable_desired_size(id),
    ))
}

fn writable_stream_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(crate::runtime::web_streams::writable_stream_new())
}

fn writable_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writable_id(args.first().ok_or("writable_write(stream, chunk)")?)?;
    let chunk = args.get(1).cloned().unwrap_or(Value::Undefined);
    crate::runtime::web_streams::writable_write(id, chunk, env)?;
    Ok(Value::Undefined)
}

fn writable_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writable_id(args.first().ok_or("writable_close(stream)")?)?;
    crate::runtime::web_streams::writable_close(id)?;
    Ok(Value::Undefined)
}

fn writable_abort_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writable_id(args.first().ok_or("writable_abort(stream)")?)?;
    let reason = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        other => Some(crate::value::format_value(other)),
    });
    crate::runtime::web_streams::writable_abort(id, reason)?;
    Ok(Value::Undefined)
}

fn writable_read_all_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::writable_id(args.first().ok_or("writable_read_all(stream)")?)?;
    crate::runtime::web_streams::writable_read_all(id)
}

fn writable_get_writer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writable_id(
        args.first().ok_or("writable_get_writer(stream)")?,
    )?;
    crate::runtime::web_streams::get_writer(id)
}

fn writer_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writer_id(args.first().ok_or("writer_write(writer, chunk)")?)?;
    let chunk = args.get(1).cloned().unwrap_or(Value::Undefined);
    crate::runtime::web_streams::writer_write(id, chunk, env)?;
    Ok(Value::Undefined)
}

fn writer_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writer_id(args.first().ok_or("writer_close(writer)")?)?;
    crate::runtime::web_streams::writer_close(id)?;
    Ok(Value::Undefined)
}

fn writer_abort_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::web_streams::writer_id(args.first().ok_or("writer_abort(writer)")?)?;
    let reason = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        other => Some(crate::value::format_value(other)),
    });
    crate::runtime::web_streams::writer_abort(id, reason)?;
    Ok(Value::Undefined)
}

fn writer_release_lock_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id =
        crate::runtime::web_streams::writer_id(args.first().ok_or("writer_release_lock(writer)")?)?;
    crate::runtime::web_streams::writer_release_lock(id)?;
    Ok(Value::Undefined)
}

fn cwd_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(crate::runtime::os::host_cwd()?))
}

fn read_text_file_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("read_text_file(path)".into()),
    };
    Ok(Value::String(
        crate::runtime::os::read_text_file(env, path)?,
    ))
}

fn write_text_file_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("write_text_file(path, text)".into()),
    };
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    crate::runtime::os::write_text_file(env, path, &content)?;
    Ok(Value::Undefined)
}

fn ws_object_tcp(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_ws".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    m.insert("__kab_tcp".into(), Value::Bool(true));
    Value::from_object(m)
}

fn tcp_ws_id(v: &Value) -> Option<u64> {
    let Value::Object(o) = v else {
        return None;
    };
    if !matches!(o.get("__kab_tcp"), Some(Value::Bool(true))) {
        return None;
    }
    ws_id(v).ok()
}

fn ws_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_ws".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::from_object(m)
}

fn ws_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected websocket".into());
    };
    if !matches!(o.get("__kab_ws"), Some(Value::Bool(true))) {
        return Err("expected websocket".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid websocket handle".into()),
    }
}

fn ws_channel_pair_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = NEXT_WS.fetch_add(1, Ordering::Relaxed);
    let b = NEXT_WS.fetch_add(1, Ordering::Relaxed);
    WS_INBOX.with(|m| {
        m.borrow_mut().insert(a, Vec::new());
        m.borrow_mut().insert(b, Vec::new());
    });
    let mut pair = HashMap::new();
    pair.insert("a".into(), ws_object(a));
    pair.insert("b".into(), ws_object(b));
    pair.insert("link_a".into(), Value::Number(b as i64));
    pair.insert("link_b".into(), Value::Number(a as i64));
    Ok(Value::from_object(pair))
}

fn ws_send_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ws = args.first().ok_or("ws_send(ws, message)")?;
    let msg = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    if let Some(id) = tcp_ws_id(ws) {
        crate::runtime::ws::ws_tcp_send(id, &msg)?;
        return Ok(Value::Undefined);
    }
    let id = ws_id(ws)?;
    let peer = WS_LINKS.with(|m| m.borrow().get(&id).copied()).unwrap_or_else(|| {
        let Value::Object(meta) = ws else {
            return id;
        };
        match meta.get("link") {
            Some(Value::Number(n)) if *n > 0 => *n as u64,
            _ => id,
        }
    });
    WS_INBOX.with(|m| {
        m.borrow_mut()
            .entry(peer)
            .or_default()
            .push(msg);
    });
    Ok(Value::Undefined)
}

fn ws_recv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ws = args.first().ok_or("ws_recv(ws)")?;
    if let Some(id) = tcp_ws_id(ws) {
        return match crate::runtime::ws::ws_tcp_recv(id)? {
            Some(s) => Ok(Value::String(s)),
            None => Ok(Value::Null),
        };
    }
    let id = ws_id(ws)?;
    WS_INBOX.with(|m| {
        let mut map = m.borrow_mut();
        let inbox = map.get_mut(&id).ok_or_else(|| format!("invalid ws id {id}"))?;
        if inbox.is_empty() {
            return Ok(Value::Null);
        }
        Ok(Value::String(inbox.remove(0)))
    })
}

fn ws_link_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ws = args.first().ok_or("ws_link(ws, peer_id)")?;
    let peer = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("ws_link(ws, peer_id) expects peer id".into()),
    };
    let id = ws_id(ws)?;
    WS_LINKS.with(|m| m.borrow_mut().insert(id, peer));
    let Value::Object(ref mut o) = ws.clone() else {
        return Err("expected websocket".into());
    };
    Rc::make_mut(o).insert("link".into(), Value::Number(peer as i64));
    Ok(ws.clone())
}

fn ws_connect_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let url = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ws_connect(url)".into()),
    };
    let id = crate::runtime::ws::ws_tcp_connect_with_trust(url, &env.tls_trust())?;
    Ok(ws_object_tcp(id))
}

fn request_method_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let req = args.first().ok_or("request_method(req)")?;
    let Value::Object(o) = req else {
        return Err("request_method() expects request object".into());
    };
    Ok(o.get("method")
        .cloned()
        .unwrap_or(Value::Undefined))
}

fn request_url_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let req = args.first().ok_or("request_url(req)")?;
    let Value::Object(o) = req else {
        return Err("request_url() expects request object".into());
    };
    Ok(o.get("url")
        .or_else(|| o.get("path"))
        .cloned()
        .unwrap_or(Value::Undefined))
}

fn request_body_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let req = args.first().ok_or("request_body(req)")?;
    let Value::Object(o) = req else {
        return Err("request_body() expects request object".into());
    };
    Ok(o.get("body")
        .cloned()
        .unwrap_or(Value::String(String::new())))
}

fn response_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let status = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => 200,
    };
    let body = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    let mut map = HashMap::new();
    map.insert("status".into(), Value::Number(status));
    map.insert("body".into(), Value::String(body));
    if let Some(Value::Object(h)) = args.get(2) {
        map.insert("headers".into(), Value::Object(h.clone()));
    }
    Ok(Value::from_object(map))
}

fn call_handler(handler: &Value, req: &HttpRequest, env: &mut Environment) -> Result<Value, String> {
    let req_val = request_to_value(req);
    crate::bytecode::call_value(handler.clone(), vec![req_val], &[], &[], &[], &[], env)
}

fn serve_handler_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handler = args
        .first()
        .ok_or("serve_handler(fn)")?
        .clone();
    env.set(SERVE_HANDLER_KEY.to_string(), handler);
    Ok(Value::Undefined)
}

fn serve_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (port, handler) = parse_serve_args(args)?;
    env.set(SERVE_HANDLER_KEY.to_string(), handler);
    #[cfg(not(target_arch = "wasm32"))]
    {
        return crate::runtime::http::http_serve_loop(port, "0.0.0.0", env).map(|_| Value::Null);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = port;
        Err("serve() is not available on wasm32".into())
    }
}

fn parse_serve_args(args: &[Value]) -> Result<(u16, Value), String> {
    if args.is_empty() {
        return Err("serve(options?, handler)".into());
    }
    if args.len() == 1 {
        return Ok((8000, args[0].clone()));
    }
    if let Some(Value::Object(map)) = args.first() {
        let port = match map.get("port") {
            Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
            _ => 8000,
        };
        let handler = args
            .get(1)
            .ok_or("serve({ port }, handler) expects handler")?
            .clone();
        return Ok((port, handler));
    }
    let port = match args.first() {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => 8000,
    };
    let handler = args
        .get(1)
        .ok_or("serve(port, handler) expects handler")?
        .clone();
    Ok((port, handler))
}

fn open_kv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("open_kv(path)".into()),
    };
    let id = crate::runtime::open_kv::open_kv(path)?;
    Ok(crate::runtime::open_kv::kv_object(id))
}

fn kv_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_get(kv, key)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_get(kv, key) expects key array".into()),
    };
    crate::runtime::open_kv::kv_get(id, key_parts)
}

fn kv_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_set(kv, key, value)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_set(kv, key, value) expects key array".into()),
    };
    let value = args.get(2).cloned().unwrap_or(Value::Null);
    crate::runtime::open_kv::kv_set(id, key_parts, value)?;
    Ok(Value::Undefined)
}

fn kv_delete_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_delete(kv, key)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_delete(kv, key) expects key array".into()),
    };
    crate::runtime::open_kv::kv_delete(id, key_parts)?;
    Ok(Value::Undefined)
}

fn kv_list_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_list(kv, prefix?)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let prefix_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[][..],
    };
    let entries = crate::runtime::open_kv::kv_list(id, prefix_parts)?;
    Ok(Value::from_array(
        entries
            .into_iter()
            .map(|(k, v)| {
                let mut pair = HashMap::new();
                pair.insert("key".into(), k);
                pair.insert("value".into(), v);
                Value::from_object(pair)
            })
            .collect(),
    ))
}

fn kv_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_close(kv)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    crate::runtime::open_kv::kv_close(id)?;
    Ok(Value::Undefined)
}

fn kv_get_entry_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_get_entry(kv, key)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_get_entry(kv, key) expects key array".into()),
    };
    crate::runtime::open_kv::kv_get_entry(id, key_parts)
}

fn kv_get_version_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_get_version(kv, key)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_get_version(kv, key) expects key array".into()),
    };
    crate::runtime::open_kv::kv_get_version(id, key_parts)
}

fn kv_listen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_listen(kv, prefix?)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let prefix_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[][..],
    };
    crate::runtime::open_kv::kv_listen(id, prefix_parts)
}

fn kv_listen_recv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listen = args.first().ok_or("kv_listen_recv(listen)")?;
    crate::runtime::open_kv::kv_listen_recv(listen)
}

fn kv_listen_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listen = args.first().ok_or("kv_listen_close(listen)")?;
    crate::runtime::open_kv::kv_listen_close(listen)?;
    Ok(Value::Undefined)
}

fn open_kv_db_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let conn = env.get("db").ok_or("open_kv_db() requires db_open() first")?;
    let Value::DbConnection(db) = conn else {
        return Err("open_kv_db() requires db_open() first".into());
    };
    if db.persist_path().is_none() {
        return Err("open_kv_db() requires a persistent db_open(path)".into());
    }
    let id = crate::runtime::open_kv::open_kv_db(db.clone())?;
    Ok(crate::runtime::open_kv::kv_object(id))
}

fn kv_watch_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_watch(kv, prefix?)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let prefix_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[][..],
    };
    crate::runtime::open_kv::kv_watch(id, prefix_parts)
}

fn kv_atomic_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_atomic(kv, ops)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let ops = match args.get(1) {
        Some(Value::Array(items)) => items.as_slice(),
        _ => return Err("kv_atomic(kv, ops) expects ops array".into()),
    };
    crate::runtime::open_kv::kv_atomic(id, ops)
}

fn kv_list_entries_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_list_entries(kv, prefix?)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let prefix_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[][..],
    };
    let entries = crate::runtime::open_kv::kv_list_entries(id, prefix_parts)?;
    Ok(Value::from_array(entries))
}

fn kv_enqueue_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_enqueue(kv, key, value)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_enqueue(kv, key, value) expects key array".into()),
    };
    let value = args.get(2).cloned().unwrap_or(Value::Null);
    crate::runtime::open_kv::kv_enqueue(id, key_parts, value)
}

fn kv_dequeue_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kv = args.first().ok_or("kv_dequeue(kv, key)")?;
    let id = crate::runtime::open_kv::kv_id(kv)?;
    let key_parts = match args.get(1) {
        Some(Value::Array(a)) => a.as_slice(),
        _ => return Err("kv_dequeue(kv, key) expects key array".into()),
    };
    crate::runtime::open_kv::kv_dequeue(id, key_parts)
}

fn kv_listen_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let listen = args.first().ok_or("kv_listen_async(listen)")?;
    let stream_id = crate::runtime::open_kv::kv_listen_stream_id(listen)?;
    Ok(crate::runtime::io_async::schedule_io_promise(
        crate::value::IoOp::KvListenRead { stream_id },
        env,
        None,
    ))
}

fn kv_watch_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let stream = args.first().ok_or("kv_watch_async(stream)")?;
    let id = crate::runtime::stdlib::deno::stream_id_pub(stream)?;
    Ok(crate::runtime::io_async::schedule_io_promise(
        crate::value::IoOp::StreamRead { stream_id: id },
        env,
        None,
    ))
}

fn unix_connect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("unix_connect(path)".into()),
    };
    Ok(Value::Number(
        crate::runtime::unix_sock::unix_connect(path)? as i64,
    ))
}

fn unix_listen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("unix_listen(path)".into()),
    };
    Ok(Value::Number(
        crate::runtime::unix_sock::unix_listen(path)? as i64,
    ))
}

fn unix_accept_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listener = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("unix_accept(listener)".into()),
    };
    Ok(Value::Number(
        crate::runtime::unix_sock::unix_accept(listener)? as i64,
    ))
}

fn unix_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("unix_read(socket, max?)".into()),
    };
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 4096,
    };
    Ok(Value::String(crate::runtime::unix_sock::unix_read(sock, max)?))
}

fn unix_write_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("unix_write(socket, data)".into()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    crate::runtime::unix_sock::unix_write(sock, &data)?;
    Ok(Value::Undefined)
}

fn unix_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("unix_close(handle)".into()),
    };
    crate::runtime::unix_sock::unix_close(handle)?;
    Ok(Value::Undefined)
}

fn worker_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = crate::runtime::worker::worker_new();
    Ok(crate::runtime::worker::worker_object(id))
}

fn worker_start_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_start(worker, code)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    let code = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("worker_start(worker, code) expects string".into()),
    };
    crate::runtime::worker::worker_start(id, code)?;
    Ok(Value::Undefined)
}

fn worker_start_file_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_start_file(worker, path)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    let path = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("worker_start_file(worker, path) expects string".into()),
    };
    crate::runtime::worker::worker_start_file(id, path)?;
    Ok(Value::Undefined)
}

fn worker_join_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_join(worker)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    crate::runtime::worker::worker_join(id)?;
    Ok(Value::Undefined)
}

fn worker_post_message_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_post_message(worker, msg, transfer?)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    let mut msg = args.get(1).cloned().unwrap_or(Value::Null);
    if let Some(Value::Array(transfers)) = args.get(2) {
        let tokens = crate::runtime::web_streams::encode_transfer_list(transfers)?;
        match &mut msg {
            Value::Object(map) => {
                Rc::make_mut(map).insert("transfers".into(), Value::from_array(tokens));
            }
            other => {
                let mut map = HashMap::new();
                map.insert("payload".into(), other.clone());
                map.insert("transfers".into(), Value::from_array(tokens));
                msg = Value::from_object(map);
            }
        }
    }
    crate::runtime::worker::worker_post_message(id, msg)?;
    Ok(Value::Undefined)
}

fn worker_recv_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_recv(worker)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    let msg = crate::runtime::worker::worker_recv(id)?;
    let msg = crate::runtime::web_streams::adopt_transfers_in_message(&msg)?;
    crate::runtime::worker::dispatch_main_onmessage(env, id, &msg)?;
    Ok(msg)
}

fn worker_recv_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_recv_async(worker)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    Ok(crate::runtime::io_async::schedule_io_promise(
        crate::value::IoOp::WorkerRecv { worker_id: id },
        env,
        None,
    ))
}

fn worker_onmessage_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_onmessage(worker, handler)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    let handler = args.get(1).cloned().ok_or("worker_onmessage(worker, handler)")?;
    crate::runtime::worker::worker_set_onmessage(id, handler)?;
    Ok(Value::Undefined)
}

fn worker_poll_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let timeout_ms = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => 5000,
    };
    Ok(crate::runtime::io_async::schedule_io_promise(
        crate::value::IoOp::WorkerPoll { timeout_ms },
        env,
        None,
    ))
}

fn worker_terminate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let worker = args.first().ok_or("worker_terminate(worker)")?;
    let id = crate::runtime::worker::worker_id(worker)?;
    crate::runtime::worker::worker_terminate(id)?;
    Ok(Value::Undefined)
}

fn ffi_load_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ffi_load(path)".into()),
    };
    Ok(Value::Number(
        crate::runtime::ffi::ffi_load(path)? as i64,
    ))
}

fn ffi_call_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let lib_id = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("ffi_call(lib, symbol, args?)".into()),
    };
    let symbol = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ffi_call(lib, symbol, args?)".into()),
    };
    let call_args: Vec<i64> = match args.get(2) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n),
                _ => Err("ffi_call args must be numbers".into()),
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => Vec::new(),
    };
    Ok(Value::Number(
        crate::runtime::ffi::ffi_call_i64(lib_id, symbol, &call_args)?,
    ))
}

fn ffi_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let lib_id = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("ffi_close(lib)".into()),
    };
    crate::runtime::ffi::ffi_close(lib_id)?;
    Ok(Value::Undefined)
}

fn npm_install_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("npm_install(name, version?)".into()),
    };
    let version = match args.get(1) {
        Some(Value::String(s)) => Some(s.as_str()),
        Some(Value::Number(n)) => {
            let s = n.to_string();
            return crate::runtime::npm_ts::npm_install(name, Some(&s));
        }
        None => None,
        _ => return Err("npm_install(name, version?) expects string or number".into()),
    };
    crate::runtime::npm_ts::npm_install(name, version)
}

fn npm_fetch_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("npm_fetch(name, version?)".into()),
    };
    match args.get(1) {
        Some(Value::String(s)) => crate::runtime::npm_ts::npm_fetch(name, Some(s.as_str())),
        Some(Value::Number(n)) => {
            let s = n.to_string();
            crate::runtime::npm_ts::npm_fetch(name, Some(&s))
        }
        None => crate::runtime::npm_ts::npm_fetch(name, None),
        _ => Err("npm_fetch(name, version?) expects string or number".into()),
    }
}

fn jsr_fetch_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("jsr_fetch(name, version?)".into()),
    };
    match args.get(1) {
        Some(Value::String(s)) => crate::runtime::npm_ts::jsr_fetch(name, Some(s.as_str())),
        Some(Value::Number(n)) => {
            let s = n.to_string();
            crate::runtime::npm_ts::jsr_fetch(name, Some(&s))
        }
        None => crate::runtime::npm_ts::jsr_fetch(name, None),
        _ => Err("jsr_fetch(name, version?) expects string or number".into()),
    }
}

fn npm_resolve_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("npm_resolve(name, version?)".into()),
    };
    match args.get(1) {
        Some(Value::String(s)) => crate::runtime::npm_ts::npm_resolve(name, Some(s.as_str())),
        Some(Value::Number(n)) => {
            let s = n.to_string();
            crate::runtime::npm_ts::npm_resolve(name, Some(&s))
        }
        None => crate::runtime::npm_ts::npm_resolve(name, None),
        _ => Err("npm_resolve(name, version?) expects string or number".into()),
    }
}

fn npm_parse_spec_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let spec = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("npm_parse_spec(spec)".into()),
    };
    crate::runtime::npm_ts::npm_parse_spec(spec)
}

fn npm_list_cache_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    crate::runtime::npm_ts::npm_list_cache()
}

fn npm_import_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("npm_import(name, version?)".into()),
    };
    let version = match args.get(1) {
        Some(Value::String(s)) => Some(s.as_str()),
        Some(Value::Number(n)) => {
            let s = n.to_string();
            let text = crate::runtime::npm_ts::npm_import_source(name, Some(&s))?;
            return Ok(Value::String(text));
        }
        None => None,
        _ => return Err("npm_import(name, version?) expects string or number".into()),
    };
    Ok(Value::String(
        crate::runtime::npm_ts::npm_import_source_prepared(name, version)?,
    ))
}

fn ts_transpile_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let source = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ts_transpile(source)".into()),
    };
    Ok(crate::runtime::npm_ts::ts_transpile(source))
}

fn ts_compile_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let source = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ts_compile(source)".into()),
    };
    Ok(crate::runtime::npm_ts::ts_compile(source))
}

fn ts_compile_file_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ts_compile_file(path)".into()),
    };
    crate::runtime::npm_ts::ts_compile_file(path)
}

#[allow(non_snake_case)]
fn Deno_emit_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    ts_compile_native(args, env)
}

fn ts_strip_types_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let source = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ts_strip_types(source)".into()),
    };
    Ok(Value::String(crate::runtime::npm_ts::ts_strip_types(
        source,
    )))
}

pub fn dispatch_serve(
    env: &mut Environment,
    req: &HttpRequest,
    handler: &Value,
) -> Result<HttpResponse, String> {
    let result = call_handler(handler, req, env)?;
    coerce_deno_response(result)
}

pub fn register_deno(env: &mut Environment) {
    env.set(SERVE_HANDLER_KEY.to_string(), Value::Null);
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("env_get", env_get_native),
        ("env_set", env_set_native),
        ("env_has", env_has_native),
        ("env_delete", env_delete_native),
        ("env_to_object", env_to_object_native),
        ("Deno_env_get", env_get_native),
        ("Deno_env_set", env_set_native),
        ("stream_from_array", stream_from_array_native),
        ("stream_read", stream_read_native),
        ("stream_read_all", stream_read_all_native),
        ("stream_new", stream_new_native),
        ("stream_from_string", stream_from_string_native),
        ("stream_cancel", stream_cancel_native),
        ("stream_tee", stream_tee_native),
        ("stream_pipe_to", stream_pipe_to_native),
        ("stream_locked", stream_locked_native),
        ("stream_lock", stream_lock_native),
        ("stream_unlock", stream_unlock_native),
        ("stream_desired_size", stream_desired_size_native),
        ("stream_get_reader", stream_get_reader_native),
        ("reader_read", reader_read_native),
        ("reader_release_lock", reader_release_lock_native),
        ("reader_cancel", reader_cancel_native),
        ("stream_abort", stream_abort_native),
        ("stream_state", stream_state_native),
        ("stream_enqueue", stream_enqueue_native),
        ("stream_close_readable", stream_close_readable_native),
        ("transform_stream_new", transform_stream_new_native),
        ("byte_stream_new", byte_stream_new_native),
        ("byte_stream_from_bytes", byte_stream_from_bytes_native),
        ("byte_stream_read", byte_stream_read_native),
        ("byte_stream_byob_read", byte_stream_byob_read_native),
        ("stream_transfer", stream_transfer_native),
        ("stream_from_transfer", stream_from_transfer_native),
        ("writable_stream_new", writable_stream_new_native),
        ("writable_write", writable_write_native),
        ("writable_close", writable_close_native),
        ("writable_abort", writable_abort_native),
        ("writable_read_all", writable_read_all_native),
        ("writable_get_writer", writable_get_writer_native),
        ("writer_write", writer_write_native),
        ("writer_close", writer_close_native),
        ("writer_abort", writer_abort_native),
        ("writer_release_lock", writer_release_lock_native),
        ("writable_locked", writable_locked_native),
        ("writable_desired_size", writable_desired_size_native),
        ("cwd", cwd_native),
        ("Deno_cwd", cwd_native),
        ("chdir", chdir_native),
        ("Deno_chdir", chdir_native),
        ("read_text_file", read_text_file_native),
        ("Deno_readTextFile", read_text_file_native),
        ("write_text_file", write_text_file_native),
        ("Deno_writeTextFile", write_text_file_native),
        ("tcp_connect", tcp_connect_native),
        ("Deno_connect", tcp_connect_native),
        ("tcp_listen", tcp_listen_native),
        ("Deno_listen", tcp_listen_native),
        ("tcp_accept", tcp_accept_native),
        ("tcp_read", tcp_read_native),
        ("tcp_read_bytes", tcp_read_bytes_native),
        ("tcp_write", tcp_write_native),
        ("tcp_write_bytes", tcp_write_bytes_native),
        ("tcp_close", tcp_close_native),
        ("tcp_start_tls", tcp_start_tls_native),
        ("Deno_startTls", tcp_start_tls_native),
        ("deno_run", deno_run_native),
        ("Deno_run", deno_run_native),
        ("run_command", run_command_native),
        ("Deno_command", run_command_native),
        ("resolve_dns", resolve_dns_native),
        ("Deno_resolveDns", resolve_dns_native),
        ("udp_bind", udp_bind_native),
        ("udp_local_addr", udp_local_addr_native),
        ("udp_send", udp_send_native),
        ("udp_recv", udp_recv_native),
        ("udp_close", udp_close_native),
        ("ws_channel_pair", ws_channel_pair_native),
        ("ws_connect", ws_connect_native),
        ("ws_link", ws_link_native),
        ("ws_send", ws_send_native),
        ("ws_recv", ws_recv_native),
        ("request_method", request_method_native),
        ("request_url", request_url_native),
        ("request_body", request_body_native),
        ("response_new", response_new_native),
        ("serve_handler", serve_handler_native),
        ("serve", serve_native),
        ("Deno_serve", serve_native),
        ("open_kv", open_kv_native),
        ("Deno_openKv", open_kv_native),
        ("open_kv_db", open_kv_db_native),
        ("kv_get", kv_get_native),
        ("kv_get_entry", kv_get_entry_native),
        ("kv_get_version", kv_get_version_native),
        ("kv_set", kv_set_native),
        ("kv_delete", kv_delete_native),
        ("kv_list", kv_list_native),
        ("kv_close", kv_close_native),
        ("kv_watch", kv_watch_native),
        ("kv_listen", kv_listen_native),
        ("kv_listen_recv", kv_listen_recv_native),
        ("kv_listen_close", kv_listen_close_native),
        ("kv_listen_async", kv_listen_async_native),
        ("kv_watch_async", kv_watch_async_native),
        ("kv_atomic", kv_atomic_native),
        ("kv_list_entries", kv_list_entries_native),
        ("kv_enqueue", kv_enqueue_native),
        ("kv_dequeue", kv_dequeue_native),
        ("unix_connect", unix_connect_native),
        ("unix_listen", unix_listen_native),
        ("unix_accept", unix_accept_native),
        ("unix_read", unix_read_native),
        ("unix_write", unix_write_native),
        ("unix_close", unix_close_native),
        ("worker_new", worker_new_native),
        ("worker_start", worker_start_native),
        ("worker_start_file", worker_start_file_native),
        ("worker_post_message", worker_post_message_native),
        ("worker_recv", worker_recv_native),
        ("worker_recv_async", worker_recv_async_native),
        ("worker_onmessage", worker_onmessage_native),
        ("worker_poll_async", worker_poll_async_native),
        ("worker_join", worker_join_native),
        ("worker_terminate", worker_terminate_native),
        ("ffi_load", ffi_load_native),
        ("ffi_call", ffi_call_native),
        ("ffi_close", ffi_close_native),
        ("npm_install", npm_install_native),
        ("npm_fetch", npm_fetch_native),
        ("jsr_fetch", jsr_fetch_native),
        ("npm_resolve", npm_resolve_native),
        ("npm_parse_spec", npm_parse_spec_native),
        ("npm_list_cache", npm_list_cache_native),
        ("npm_import", npm_import_native),
        ("ts_transpile", ts_transpile_native),
        ("ts_compile", ts_compile_native),
        ("ts_compile_file", ts_compile_file_native),
        ("Deno_emit", Deno_emit_native),
        ("ts_strip_types", ts_strip_types_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
    crate::runtime::stdlib::deno_wave_b::register_wave_b(env);
    {
        let mut bind =
            |names: &[&str],
             f: fn(&[Value], &mut Environment) -> Result<Value, String>| {
                for n in names {
                    env.set((*n).into(), Value::NativeFunction(f));
                }
            };
        crate::runtime::mqtt_client::register(&mut bind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{create_global_env, eval_source};

    #[test]
    fn env_roundtrip() {
        let mut env = create_global_env();
        eval_source(r#"env_set("FOO", "bar")"#, &mut env).unwrap();
        let v = eval_source(r#"env_get("FOO")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "bar"));
    }
}
