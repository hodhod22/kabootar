//! v2.16 — HTTP fetch timeout

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn spawn_slow_server(delay_ms: u64) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("local_addr").port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            thread::sleep(Duration::from_millis(delay_ms));
            let response =
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 4\r\nConnection: close\r\n\r\nslow";
            let _ = stream.write_all(response.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(50));
    port
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn per_request_timeout_aborts_slow_fetch() {
    let port = spawn_slow_server(500);
    let url = format!("http://127.0.0.1:{port}/slow");

    let mut env = create_global_env();
    let err = eval_source(
        &format!(
            r#"
            async fn load() {{
                return await http_fetch_async("GET", "{url}", "", {{}}, 100)
            }}
            await load()
        "#
        ),
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("timed out"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn per_request_timeout_allows_fast_fetch() {
    let port = spawn_slow_server(50);
    let url = format!("http://127.0.0.1:{port}/fast");

    let mut env = create_global_env();
    let v = eval_source(
        &format!(
            r#"
            async fn load() {{
                return http_body(await http_fetch_async("GET", "{url}", "", {{ X: "1" }}, 2000))
            }}
            await load()
        "#
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "slow"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn http_set_timeout_applies_globally() {
    let port = spawn_slow_server(500);
    let url = format!("http://127.0.0.1:{port}/global");

    let mut env = create_global_env();
    let err = eval_source(
        &format!(
            r#"
            http_set_timeout(100)
            async fn load() {{
                return await http_fetch_async("GET", "{url}", "")
            }}
            await load()
        "#
        ),
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("timed out"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn http_reset_timeout_disables_global_limit() {
    let port = spawn_slow_server(150);
    let url = format!("http://127.0.0.1:{port}/ok");

    let mut env = create_global_env();
    let v = eval_source(
        &format!(
            r#"
            http_set_timeout(50)
            http_reset_timeout()
            async fn load() {{
                return http_body(await http_fetch_async("GET", "{url}", ""))
            }}
            await load()
        "#
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "slow"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn per_request_timeout_overrides_global() {
    let port = spawn_slow_server(400);
    let url = format!("http://127.0.0.1:{port}/override");

    let mut env = create_global_env();
    let err = eval_source(
        &format!(
            r#"
            http_set_timeout(2000)
            async fn load() {{
                return await http_fetch_async("GET", "{url}", "", {{}}, 100)
            }}
            await load()
        "#
        ),
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("timed out"));
}
