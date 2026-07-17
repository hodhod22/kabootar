//! Deno Wave B — serve dispatch, paths, lockfile, TLS listen, shared workers.

use crate::runtime::http::HttpRequest;
use crate::runtime::stdlib::deno::dispatch_serve;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

fn serve_dispatch_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handler = args.first().ok_or("serve_dispatch(handler, method, path, body?)")?;
    let method = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("serve_dispatch expects method string".into()),
    };
    let path = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("serve_dispatch expects path string".into()),
    };
    let body = args
        .get(3)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Undefined | Value::Null => None,
            other => Some(crate::value::format_value(other)),
        })
        .unwrap_or_default();
    let req = HttpRequest {
        method,
        path: path.clone(),
        body,
        headers: HashMap::from([("host".into(), "localhost".into())]),
    };
    let res = dispatch_serve(env, &req, handler)?;
    let mut out = HashMap::new();
    out.insert("status".into(), Value::Number(res.status));
    out.insert("body".into(), Value::String(res.body));
    let mut headers = HashMap::new();
    for (k, v) in res.headers {
        headers.insert(k, Value::String(v));
    }
    out.insert("headers".into(), Value::Object(headers));
    Ok(Value::Object(out))
}

fn serve_async_ready_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handler = args.first().ok_or("serve_async_ready(handler, port?)")?;
    env.set(
        crate::runtime::stdlib::deno::SERVE_HANDLER_KEY.to_string(),
        handler.clone(),
    );
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as u16,
        _ => pick_ephemeral_port()?,
    };
    let serve_id = crate::runtime::serve_async::spawn_serve(port, "127.0.0.1")?;
    let mut map = HashMap::new();
    map.insert("port".into(), Value::Number(port as i64));
    map.insert("ready".into(), Value::Bool(true));
    map.insert("http2".into(), Value::Bool(crate::runtime::http2::supported()));
    map.insert("serveId".into(), Value::Number(serve_id as i64));
    Ok(Value::Object(map))
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_ephemeral_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("serve_async: failed to pick port: {e}"))?;
    Ok(listener.local_addr().map_err(|e| e.to_string())?.port())
}

#[cfg(target_arch = "wasm32")]
fn pick_ephemeral_port() -> Result<u16, String> {
    Err("serve_async is not available on wasm32".into())
}

fn serve_async_stop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("serve_async_stop(serveId)".into()),
    };
    crate::runtime::serve_async::stop_serve(id)?;
    Ok(Value::Undefined)
}

fn serve_async_poll_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(
        crate::runtime::serve_async::poll_serve(env)? as i64,
    ))
}

fn http2_supported_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Bool(crate::runtime::http2::supported()))
}

fn http2_preface_ok_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let raw = match args.first() {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        Some(Value::Array(vals)) => vals
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n as u8),
                _ => Err("http2_preface_ok expects string or byte array".into()),
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => return Err("http2_preface_ok(bytes)".into()),
    };
    Ok(Value::Bool(crate::runtime::http2::is_preface(&raw)))
}

fn realpath_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("realpath(path)".into()),
    };
    Ok(Value::String(crate::runtime::os::host_realpath(path)?))
}

fn symlink_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let target = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("symlink(target, path)".into()),
    };
    let link = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("symlink(target, path)".into()),
    };
    crate::runtime::os::host_symlink(target, link)?;
    Ok(Value::Undefined)
}

fn link_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let old = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("link(oldPath, newPath)".into()),
    };
    let new = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("link(oldPath, newPath)".into()),
    };
    crate::runtime::os::host_link(old, new)?;
    Ok(Value::Undefined)
}

fn lockfile_read_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let root = crate::project::root::project_root()?;
    let lock = crate::project::lockfile::read_lockfile(&crate::project::lockfile::lockfile_path(
        &root,
    ))?;
    Ok(crate::project::lockfile::lockfile_to_value(&lock))
}

fn lockfile_sync_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let lock = crate::project::lockfile::sync_lockfile_from_manifest()?;
    Ok(crate::project::lockfile::lockfile_to_value(&lock))
}

fn tls_listen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => "127.0.0.1",
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as u16,
        _ => return Err("tls_listen(host, port, certPem, keyPem)".into()),
    };
    let cert = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tls_listen expects cert PEM string".into()),
    };
    let key = match args.get(3) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tls_listen expects key PEM string".into()),
    };
    let id = crate::runtime::tls_server::tls_listen(host, port, cert, key)?;
    Ok(Value::Number(id as i64))
}

fn tls_reload_certs_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listener = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tls_reload_certs(listener, certPem, keyPem)".into()),
    };
    let cert = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tls_reload_certs expects cert PEM".into()),
    };
    let key = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tls_reload_certs expects key PEM".into()),
    };
    crate::runtime::tls_server::tls_reload_certs(listener, cert, key)?;
    Ok(Value::Undefined)
}

fn tls_accept_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let listener = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tls_accept(listener)".into()),
    };
    let id = crate::runtime::tls_server::tls_accept(listener)?;
    Ok(Value::Number(id as i64))
}

fn tls_server_read_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tls_server_read(socket, max?)".into()),
    };
    let max = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 4096,
    };
    Ok(Value::String(
        crate::runtime::tls_server::tls_server_read(sock, max)?,
    ))
}

fn tls_server_write_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tls_server_write(socket, data)".into()),
    };
    let data = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("tls_server_write expects string data".into()),
    };
    crate::runtime::tls_server::tls_server_write(sock, data)?;
    Ok(Value::Undefined)
}

fn tls_server_close_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sock = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("tls_server_close(socket)".into()),
    };
    crate::runtime::tls_server::tls_server_close(sock)?;
    Ok(Value::Undefined)
}

struct SharedWorkerState {
    id: u64,
    inbox: VecDeque<Value>,
}

thread_local! {
    static SHARED_WORKERS: RefCell<HashMap<String, SharedWorkerState>> = RefCell::new(HashMap::new());
}

fn shared_worker_connect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("shared_worker_connect(name)".into()),
    };
    SHARED_WORKERS.with(|m| {
        let mut map = m.borrow_mut();
        if let Some(state) = map.get(&name) {
            return Ok(Value::Number(state.id as i64));
        }
        let id = crate::runtime::worker::worker_new();
        map.insert(
            name,
            SharedWorkerState {
                id,
                inbox: VecDeque::new(),
            },
        );
        Ok(Value::Number(id as i64))
    })
}

fn shared_worker_post_message_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("shared_worker_post_message(name, msg, transfers?)".into()),
    };
    let msg = args.get(1).cloned().unwrap_or(Value::Null);
    let transfers = args.get(2).and_then(|v| match v {
        Value::Array(items) => Some(items.as_slice()),
        _ => None,
    });
    let wire = if let Some(list) = transfers {
        let encoded = crate::runtime::web_streams::encode_transfer_list(list)?;
        let mut map = match msg {
            Value::Object(m) => m,
            other => {
                let mut m = HashMap::new();
                m.insert("payload".into(), other);
                m
            }
        };
        map.insert("transfers".into(), Value::Array(encoded));
        Value::Object(map)
    } else {
        msg
    };
    SHARED_WORKERS.with(|m| {
        let mut map = m.borrow_mut();
        let state = map
            .get_mut(&name)
            .ok_or_else(|| format!("unknown shared worker {name}"))?;
        state.inbox.push_back(wire);
        Ok(Value::Undefined)
    })
}

fn shared_worker_recv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("shared_worker_recv(name)".into()),
    };
    SHARED_WORKERS.with(|m| {
        let mut map = m.borrow_mut();
        let state = map
            .get_mut(&name)
            .ok_or_else(|| format!("unknown shared worker {name}"))?;
        Ok(state.inbox.pop_front().unwrap_or(Value::Null))
    })
}

pub fn register_wave_b(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("serve_dispatch", serve_dispatch_native),
        ("serve_async_ready", serve_async_ready_native),
        ("serve_async_poll", serve_async_poll_native),
        ("serve_async_stop", serve_async_stop_native),
        ("http2_supported", http2_supported_native),
        ("http2_preface_ok", http2_preface_ok_native),
        ("realpath", realpath_native),
        ("Deno_realPath", realpath_native),
        ("symlink", symlink_native),
        ("Deno_symlink", symlink_native),
        ("link", link_native),
        ("Deno_link", link_native),
        ("lockfile_read", lockfile_read_native),
        ("lockfile_sync", lockfile_sync_native),
        ("tls_listen", tls_listen_native),
        ("Deno_listenTls", tls_listen_native),
        ("tls_reload_certs", tls_reload_certs_native),
        ("tls_accept", tls_accept_native),
        ("tls_server_read", tls_server_read_native),
        ("tls_server_write", tls_server_write_native),
        ("tls_server_close", tls_server_close_native),
        ("shared_worker_connect", shared_worker_connect_native),
        ("shared_worker_post_message", shared_worker_post_message_native),
        ("shared_worker_recv", shared_worker_recv_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
    crate::runtime::stdlib::deno_permissions::register_permissions(env);
    crate::runtime::stdlib::deno_testing::register_testing(env);
}
