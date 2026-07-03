//! v2.12 — HTTP fetch headers (request + response)

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
            let mut auth = String::new();
            for line in raw.split("\r\n") {
                if let Some((key, value)) = line.split_once(':') {
                    if key.trim().eq_ignore_ascii_case("authorization") {
                        auth = value.trim().to_string();
                    }
                }
            }
            let body = format!("echo:{auth}");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nX-Seen-Auth: {}\r\nConnection: close\r\n\r\n{body}",
                body.len(),
                auth
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(50));
    port
}

#[test]
fn http_fetch_async_sends_custom_request_headers() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/secure");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "", {{ Authorization: "secret-token" }})
            return http_body(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "echo:secret-token"));
}

#[test]
fn http_headers_reads_response_headers() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/meta");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "", {{ Authorization: "tok" }})
            let h = http_headers(res)
            return h["content-type"] + "|" + h["x-seen-auth"]
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "text/plain|tok"));
}

#[test]
fn http_fetch_async_without_headers_still_works() {
    let port = spawn_header_echo_server();
    let url = format!("http://127.0.0.1:{port}/plain");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "")
            return http_status(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::Number(200)));
}

#[test]
fn http_headers_empty_for_in_process_response() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn ok() {
            return http_response(200, "local")
        }
        http_route("GET", "/h", ok)
        async fn load() {
            let res = await http_request_async("GET", "/h", "")
            let h = http_headers(res)
            return len(keys(h))
        }
        await load()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(0)));
}
