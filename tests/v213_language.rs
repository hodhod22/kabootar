//! v2.13 — HTTP string object keys, redirects, http_header()

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn spawn_header_echo_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("local_addr").port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]);
            let mut content_type = String::new();
            for line in raw.split("\r\n") {
                if let Some((key, value)) = line.split_once(':') {
                    if key.trim().eq_ignore_ascii_case("content-type") {
                        content_type = value.trim().to_string();
                    }
                }
            }
            let body = format!("ct:{content_type}");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(50));
    port
}

fn spawn_redirect_server() -> (u16, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let base = format!("http://127.0.0.1:{port}");
    let start_url = format!("{base}/start");
    thread::spawn(move || {
        for _ in 0..2 {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]);
                let response = if raw.contains("GET /start") {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: {base}/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 8\r\nConnection: close\r\n\r\nredirect".to_string()
                };
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });
    thread::sleep(Duration::from_millis(50));
    (port, start_url)
}

#[test]
fn object_literal_string_keys_for_headers() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/json");

    let json_body = "{}";
    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async(
                "POST",
                "{url}",
                "{json_body}",
                {{ "Content-Type": "application/json" }}
            )
            return http_body(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "ct:application/json"));
}

#[test]
fn http_header_reads_single_response_header() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/meta");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "")
            return http_header(res, "Content-Type")
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "text/plain"));
}

#[test]
fn http_header_missing_returns_undefined() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/none");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "")
            return is_undefined(http_header(res, "X-Missing"))
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn http_fetch_follows_redirects() {
    let (_port, start_url) = spawn_redirect_server();

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{start_url}", "")
            return http_body(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "redirect"));
}

#[test]
fn post_redirect_302_becomes_get() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let base = format!("http://127.0.0.1:{port}");
    let start_url = format!("{base}/post-start");

    thread::spawn(move || {
        for _ in 0..2 {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]);
                let response = if raw.contains("/post-start") {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: {base}/done\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else if raw.starts_with("GET /done") {
                    "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nget".to_string()
                } else {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\nConnection: close\r\n\r\nwrong".to_string()
                };
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });
    thread::sleep(Duration::from_millis(50));

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("POST", "{start_url}", "payload")
            return http_body(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "get"));
}
