//! In-process HTTP routing and request/response types for Kabootar backend.

use crate::value::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl HttpRequest {
    pub fn new(method: impl Into<String>, path: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            body: body.into(),
            headers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: i64,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl HttpResponse {
    pub fn new(status: i64, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: HashMap::new(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found")
    }

    pub fn to_http_string(&self) -> String {
        let status_text = status_text(self.status);
        format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            self.status,
            status_text,
            self.body.len(),
            self.body
        )
    }
}

fn status_text(code: i64) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpRouter {
    inner: Arc<Mutex<Vec<Route>>>,
}

#[derive(Debug, Clone)]
struct Route {
    method: String,
    path: String,
    handler: Value,
}

impl HttpRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_route(&self, method: String, path: String, handler: Value) -> Result<(), String> {
        match &handler {
            Value::Function { .. } | Value::BytecodeFn(_) => {}
            _ => return Err("http_route() expects a function handler".into()),
        }
        let mut routes = self
            .inner
            .lock()
            .map_err(|_| "HTTP router lock poisoned".to_string())?;
        routes.retain(|r| !(r.method == method && r.path == path));
        routes.push(Route {
            method: method.to_uppercase(),
            path: normalize_path(&path),
            handler,
        });
        Ok(())
    }

    pub fn find_handler(&self, method: &str, path: &str) -> Option<Value> {
        let routes = self.inner.lock().ok()?;
        let method = method.to_uppercase();
        let path = normalize_path(path);
        routes
            .iter()
            .find(|r| r.method == method && r.path == path)
            .map(|r| r.handler.clone())
    }
}

pub fn parse_http_request(raw: &str) -> Result<HttpRequest, String> {
    let mut lines = raw.split("\r\n");
    let request_line = lines
        .next()
        .filter(|l| !l.is_empty())
        .ok_or("Empty HTTP request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or("Expected HTTP method")?
        .to_string();
    let path = parts
        .next()
        .ok_or("Expected HTTP path")?
        .to_string();

    let mut headers = HashMap::new();
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let body: String = lines.collect::<Vec<_>>().join("\r\n");
    Ok(HttpRequest {
        method,
        path,
        body,
        headers,
    })
}

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

pub fn http_globals(env: &mut crate::value::Environment) {
    use crate::value::Value;
    env.set("http_router".to_string(), Value::HttpRouter(HttpRouter::new()));
    env.set(
        "http_route".to_string(),
        Value::NativeFunction(http_route_native),
    );
    env.set(
        "http_request".to_string(),
        Value::NativeFunction(http_request_native),
    );
    env.set(
        "http_response".to_string(),
        Value::NativeFunction(http_response_native),
    );
    env.set(
        "http_status".to_string(),
        Value::NativeFunction(http_status_native),
    );
    env.set(
        "http_body".to_string(),
        Value::NativeFunction(http_body_native),
    );
    env.set(
        "http_headers".to_string(),
        Value::NativeFunction(http_headers_native),
    );
    env.set(
        "http_header".to_string(),
        Value::NativeFunction(http_header_native),
    );
    env.set(
        "http_set_timeout".to_string(),
        Value::NativeFunction(http_set_timeout_native),
    );
    env.set(
        "http_reset_timeout".to_string(),
        Value::NativeFunction(http_reset_timeout_native),
    );
    env.set(
        "http_process".to_string(),
        Value::NativeFunction(http_process_native),
    );
    #[cfg(not(target_arch = "wasm32"))]
    env.set(
        "http_serve_once".to_string(),
        Value::NativeFunction(http_serve_once_native),
    );
    #[cfg(not(target_arch = "wasm32"))]
    env.set(
        "http_serve".to_string(),
        Value::NativeFunction(http_serve_native),
    );
}

fn http_route_native(args: &[Value], env: &mut crate::value::Environment) -> Result<Value, String> {
    let method = expect_string(args, 0, "http_route()")?;
    let path = expect_string(args, 1, "http_route()")?;
    let handler = args
        .get(2)
        .ok_or("http_route() expects method, path, and handler")?
        .clone();
    let router = get_router(env)?;
    router.add_route(method, path, handler)?;
    Ok(Value::Null)
}

fn http_request_native(args: &[Value], env: &mut crate::value::Environment) -> Result<Value, String> {
    let method = expect_string(args, 0, "http_request()")?;
    let path = expect_string(args, 1, "http_request()")?;
    let body = args
        .get(2)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => format_value(other),
        })
        .unwrap_or_default();
    let req = HttpRequest::new(method, path, body);
    let res = crate::http_dispatch::dispatch(env, &req)?;
    Ok(Value::HttpResponse(res))
}

fn http_process_native(args: &[Value], env: &mut crate::value::Environment) -> Result<Value, String> {
    let raw = expect_string(args, 0, "http_process()")?;
    let req = parse_http_request(&raw)?;
    let res = crate::http_dispatch::dispatch(env, &req)?;
    Ok(Value::String(res.to_http_string()))
}

fn http_response_native(args: &[Value], _env: &mut crate::value::Environment) -> Result<Value, String> {
    let status = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => return Err("http_response() expects status code as first argument".into()),
    };
    let body = args
        .get(1)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => format_value(other),
        })
        .unwrap_or_default();
    Ok(Value::HttpResponse(HttpResponse::new(status, body)))
}

fn http_status_native(args: &[Value], _env: &mut crate::value::Environment) -> Result<Value, String> {
    let res = expect_response(args, 0, "http_status()")?;
    Ok(Value::Number(res.status))
}

fn http_body_native(args: &[Value], _env: &mut crate::value::Environment) -> Result<Value, String> {
    let res = expect_response(args, 0, "http_body()")?;
    Ok(Value::String(res.body))
}

fn http_headers_native(args: &[Value], _env: &mut crate::value::Environment) -> Result<Value, String> {
    let res = expect_response(args, 0, "http_headers()")?;
    let mut obj = HashMap::new();
    for (key, value) in &res.headers {
        obj.insert(key.clone(), Value::String(value.clone()));
    }
    Ok(Value::Object(obj))
}

fn http_header_native(args: &[Value], _env: &mut crate::value::Environment) -> Result<Value, String> {
    let res = expect_response(args, 0, "http_header()")?;
    let name = expect_string(args, 1, "http_header()")?;
    let key = name.to_ascii_lowercase();
    Ok(res
        .headers
        .get(&key)
        .cloned()
        .map(Value::String)
        .unwrap_or(Value::Undefined))
}

fn http_set_timeout_native(
    args: &[Value],
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => return Err("http_set_timeout(ms) expects a non-negative number".into()),
    };
    *env.http_fetch_timeout_ms_mut() = ms;
    Ok(Value::Null)
}

fn http_reset_timeout_native(
    _args: &[Value],
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    *env.http_fetch_timeout_ms_mut() = 0;
    Ok(Value::Null)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_serve_once_native(
    args: &[Value],
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let port = match args.first() {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("http_serve_once() expects a valid port number".into()),
    };

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("Failed to accept connection: {}", e))?;

    let mut buffer = [0u8; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read request: {}", e))?;
    let raw = String::from_utf8_lossy(&buffer[..n]).to_string();
    let req = parse_http_request(&raw)?;
    let res = crate::http_dispatch::dispatch(env, &req)?;
    stream
        .write_all(res.to_http_string().as_bytes())
        .map_err(|e| format!("Failed to write response: {}", e))?;
    Ok(Value::Number(res.status))
}

#[cfg(not(target_arch = "wasm32"))]
fn http_serve_native(
    args: &[Value],
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    let port = match args.first() {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("http_serve() expects a valid port number".into()),
    };
    let bind = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("0.0.0.0");
    http_serve_loop(port, bind, env)?;
    Ok(Value::Null)
}

/// Run an HTTP accept loop until interrupted (native only).
#[cfg(not(target_arch = "wasm32"))]
pub fn http_serve_loop(
    port: u16,
    bind: &str,
    env: &mut crate::value::Environment,
) -> Result<(), String> {
    http_serve_loop_with_poll(port, bind, env, None::<fn(&mut crate::value::Environment) -> bool>)
}

/// HTTP loop with optional poll callback (e.g. hot reload).
#[cfg(not(target_arch = "wasm32"))]
pub fn http_serve_loop_with_poll<F>(
    port: u16,
    bind: &str,
    env: &mut crate::value::Environment,
    mut poll: Option<F>,
) -> Result<(), String>
where
    F: FnMut(&mut crate::value::Environment) -> bool,
{
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    let addr = format!("{bind}:{port}");
    let listener =
        TcpListener::bind(&addr).map_err(|e| format!("Failed to bind {addr}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set nonblocking: {e}"))?;
    eprintln!("Kabootar HTTP listening on http://{addr}");

    loop {
        if let Some(ref mut p) = poll {
            if p(env) {
                eprintln!("Kabootar: reloaded");
            }
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0u8; 8192];
                let n = stream
                    .read(&mut buffer)
                    .map_err(|e| format!("Failed to read request: {e}"))?;
                if n == 0 {
                    continue;
                }
                let raw = String::from_utf8_lossy(&buffer[..n]).to_string();
                let response = match parse_http_request(&raw) {
                    Ok(req) => crate::http_dispatch::dispatch(env, &req)?,
                    Err(e) => HttpResponse::new(400, e),
                };
                stream
                    .write_all(response.to_http_string().as_bytes())
                    .map_err(|e| format!("Failed to write response: {e}"))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(format!("Failed to accept connection: {e}")),
        }
    }
}

fn get_router(env: &crate::value::Environment) -> Result<HttpRouter, String> {
    let router = env
        .get("http_router")
        .ok_or("HTTP router not available")?;
    let Value::HttpRouter(r) = router else {
        return Err("HTTP router not available".into());
    };
    Ok(r)
}

fn expect_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{} expects a string argument at position {}", name, index)),
    }
}

fn expect_response(args: &[Value], index: usize, name: &str) -> Result<HttpResponse, String> {
    match args.get(index) {
        Some(Value::HttpResponse(r)) => Ok(r.clone()),
        _ => Err(format!("{} expects an HTTP response", name)),
    }
}

fn format_value(val: &Value) -> String {
    crate::value::format_value(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let req = parse_http_request("GET /hello HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/hello");
    }
}
