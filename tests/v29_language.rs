//! v2.9 — riktig nätverks-IO (HTTP fetch över TCP)

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn spawn_one_shot_http_server(body: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("local_addr").port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(50));
    port
}

#[test]
fn http_fetch_async_real_network() {
    let port = spawn_one_shot_http_server("network-ok");
    let url = format!("http://127.0.0.1:{port}/health");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn load() {{
            let res = await http_fetch_async("GET", "{url}", "")
            return http_body(res)
        }}
        await load()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "network-ok"));
}

#[test]
fn http_fetch_async_post_with_body() {
    let port = spawn_one_shot_http_server("received");
    let url = format!("http://127.0.0.1:{port}/echo");

    let mut env = create_global_env();
    let code = format!(
        r#"
        async fn post() {{
            let res = await http_fetch_async("POST", "{url}", "payload")
            return http_status(res)
        }}
        await post()
    "#
    );
    let v = eval_source(&code, &mut env).unwrap();
    assert!(matches!(v, Value::Number(200)));
}

#[test]
fn in_process_and_network_fetch_differ() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn ok() {
            return http_response(200, "local")
        }
        http_route("GET", "/local", ok)

        async fn local() {
            return http_body(await http_request_async("GET", "/local", ""))
        }
        await local()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "local"));
}
