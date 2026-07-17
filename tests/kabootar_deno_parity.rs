//! Deno runtime parity tests.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[cfg(not(target_arch = "wasm32"))]
fn spawn_ws_echo_server() -> u16 {
    kabootar_lib::runtime::ws::spawn_echo_server_for_test()
}

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn deno_env_get_set() {
    let out = eval(
        r#"
        env_set("KAB_TEST", "42")
        env_get("KAB_TEST")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "42"));

    let out = eval(r#"env_has("KAB_TEST")"#);
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn deno_streams() {
    let out = eval(
        r#"
        let s = stream_from_array([10, 20, 30])
        stream_read_all(s)
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert_eq!(items.len(), 3);
}

#[test]
fn deno_websocket_channel() {
    let out = eval(
        r#"
        let p = ws_channel_pair()
        ws_link(p["a"], p["link_a"])
        ws_link(p["b"], p["link_b"])
        ws_send(p["a"], "hello")
        ws_recv(p["b"])
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "hello"));
}

#[test]
fn deno_serve_handler() {
    let out = eval(
        r#"
        serve_handler((req) => response_new(200, request_url(req)))
        let crlf = from_char_code(13, 10)
        let raw = "GET /hello HTTP/1.1" + crlf + "Host: localhost" + crlf + crlf
        http_process(raw)
        "#,
    );
    let Value::String(http) = out else {
        panic!("expected http string, got {:?}", out);
    };
    assert!(http.contains("200"));
    assert!(http.contains("/hello"));
}

#[test]
fn deno_response_object() {
    let out = eval(
        r#"
        serve_handler((req) => response_new(201, request_body(req)))
        let crlf = from_char_code(13, 10)
        let raw = "POST /x HTTP/1.1" + crlf + "Content-Length: 3" + crlf + crlf + "abc"
        http_process(raw)
        "#,
    );
    let Value::String(http) = out else {
        panic!("expected http string, got {:?}", out);
    };
    assert!(http.contains("201"));
    assert!(http.contains("abc"));
}

#[test]
fn deno_cwd() {
    let out = eval(r#"cwd()"#);
    let Value::String(path) = out else {
        panic!("expected cwd string");
    };
    assert!(!path.is_empty());
}

#[test]
fn deno_read_write_text_file() {
    let out = eval(
        r#"
        let p = "deno_parity_tmp.txt"
        write_text_file(p, "deno-v2")
        read_text_file(p)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "deno-v2"));
}

#[test]
fn deno_fs_read_write_stat_dir() {
    let out = eval(
        r#"
        mkdir("/fsdir")
        write_file("/fsdir/a.bin", [65, 66, 67])
        let info = stat("/fsdir/a.bin")
        let bytes = read_file("/fsdir/a.bin")
        let entries = read_dir("/fsdir")
        remove("/fsdir/a.bin")
        { "info": info, "bytes": bytes, "entries": entries }
        "#,
    );
    let Value::Object(o) = out else {
        panic!("expected object");
    };
    let info = o.get("info").expect("info");
    let Value::Object(info) = info else {
        panic!("expected stat object");
    };
    assert!(matches!(info.get("isFile"), Some(Value::Bool(true))));
    assert!(matches!(info.get("size"), Some(Value::Number(3))));
    let bytes = o.get("bytes").expect("bytes");
    let Value::Array(bytes) = bytes else {
        panic!("expected byte array");
    };
    assert_eq!(bytes.len(), 3);
    assert!(matches!(bytes[0], Value::Number(65)));
    let entries = o.get("entries").expect("entries");
    let Value::Array(entries) = entries else {
        panic!("expected dir entries");
    };
    assert!(!entries.is_empty());
    assert!(matches!(
        entries[0],
        Value::Object(ref e) if matches!(e.get("name"), Some(Value::String(s)) if s == "a.bin")
    ));
}

#[test]
fn deno_writable_stream() {
    let out = eval(
        r#"
        let w = writable_stream_new()
        writable_write(w, "a")
        writable_write(w, 42)
        writable_close(w)
        writable_read_all(w)
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn deno_stream_from_string() {
    let out = eval(
        r#"
        let s = stream_from_string("xy")
        let c1 = stream_read(s)
        let c2 = stream_read(s)
        [c1["value"], c2["done"]]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(items[0], Value::String(ref s) if s == "xy"));
    assert!(matches!(items[1], Value::Bool(true)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_ws_tcp_connect() {
    let port = spawn_ws_echo_server();
    std::thread::sleep(std::time::Duration::from_millis(40));
    let out = eval(&format!(
        r#"
        let ws = ws_connect("ws://127.0.0.1:{port}/")
        ws_send(ws, "kab")
        ws_recv(ws)
        "#
    ));
    assert!(matches!(out, Value::String(s) if s == "kab"));
}

#[test]
fn deno_stream_tee_and_pipe() {
    let out = eval(
        r#"
        let s = stream_from_array([1, 2])
        let branches = stream_tee(s)
        let w = writable_stream_new()
        stream_pipe_to(branches[0], w)
        let a = stream_read_all(branches[1])
        let b = writable_read_all(w)
        [a[0], b[1]]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_tcp_listen_connect() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).expect("read");
        stream.write_all(&buf[..n]).expect("echo");
    });

    let out = eval(&format!(
        r#"
        let sock = tcp_connect("127.0.0.1", {port})
        tcp_write(sock, "echo-me")
        tcp_read(sock, 64)
        "#
    ));
    server.join().expect("server");
    assert!(matches!(out, Value::String(s) if s == "echo-me"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_tcp_start_tls() {
    use kabootar_lib::evaluator::eval_source;
    use kabootar_lib::evaluator::create_global_env;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let server_cfg = Arc::new(server_cfg);
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = thread::spawn(move || {
        if let Ok((mut tcp, _)) = listener.accept() {
            if let Ok(mut server_conn) = rustls::ServerConnection::new(server_cfg.clone()) {
                let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
                let mut buf = [0u8; 64];
                if let Ok(n) = tls.read(&mut buf) {
                    let _ = tls.write_all(&buf[..n]);
                    let _ = tls.flush();
                }
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut env = create_global_env();
    env.tls_trust_mut().set_ca_only_pem(&ca_pem).unwrap();
    let out = eval_source(
        &format!(
            r#"
            let sock = tcp_connect("127.0.0.1", {port})
            tcp_start_tls(sock, "localhost")
            tcp_write(sock, "tls-echo")
            tcp_read(sock, 64)
            "#
        ),
        &mut env,
    )
    .expect("eval");
    server.join().expect("server");
    assert!(matches!(out, Value::String(s) if s == "tls-echo"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_wss_connect() {
    use kabootar_lib::runtime::ws::ws_accept_key;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let server_cfg = Arc::new(server_cfg);
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    thread::spawn(move || {
        if let Ok((mut tcp, _)) = listener.accept() {
            if let Ok(mut server_conn) = rustls::ServerConnection::new(server_cfg.clone()) {
                let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
                let mut buf = [0u8; 4096];
                if tls.read(&mut buf).is_ok() {
                    let req = String::from_utf8_lossy(&buf);
                    if let Some(key) = req.lines().find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                            Some(v.trim().to_string())
                        } else {
                            None
                        }
                    }) {
                        let accept = ws_accept_key(&key);
                        let resp = format!(
                            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
                        );
                        let _ = tls.write_all(resp.as_bytes());
                        if tls.read(&mut buf).is_ok() {
                            let payload_len = (buf[1] & 0x7F) as usize;
                            let mut idx = 2usize;
                            if payload_len == 126 {
                                idx = 4;
                            }
                            if buf.len() > idx + 4 {
                                let mask = &buf[idx..idx + 4];
                                idx += 4;
                                let end = buf.len().min(idx + payload_len);
                                let unmasked: String = buf[idx..end]
                                    .iter()
                                    .enumerate()
                                    .map(|(i, b)| (b ^ mask[i % 4]) as char)
                                    .collect();
                                let bytes = unmasked.as_bytes();
                                let mut frame = vec![0x81, bytes.len() as u8];
                                frame.extend_from_slice(bytes);
                                let _ = tls.write_all(&frame);
                            }
                        }
                    }
                }
            }
        }
    });

    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut env = create_global_env();
    env.tls_trust_mut().set_ca_only_pem(&ca_pem).unwrap();
    let out = eval_source(
        &format!(
            r#"
            let ws = ws_connect("wss://localhost:{port}/")
            ws_send(ws, "secure")
            ws_recv(ws)
            "#
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(out, Value::String(s) if s == "secure"));
}

#[test]
fn deno_run_spawns_process() {
    let out = eval(r#"deno_run("main")"#);
    assert!(matches!(out, Value::Number(n) if n > 0));
}

#[test]
fn deno_stream_backpressure() {
    let out = eval(
        r#"
        let s = stream_from_array([1, 2, 3])
        stream_lock(s)
        let locked = stream_locked(s)
        let size_locked = stream_desired_size(s)
        stream_unlock(s)
        let size_open = stream_desired_size(s)
        [locked, size_locked, size_open]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(items[0], Value::Bool(true)));
    assert!(matches!(items[1], Value::Number(0)));
    assert!(matches!(items[2], Value::Number(3)));
}

#[test]
fn deno_resolve_dns_localhost() {
    let out = eval(r#"resolve_dns("localhost", 80)"#);
    let Value::Array(addrs) = out else {
        panic!("expected address array");
    };
    assert!(!addrs.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_udp_roundtrip() {
    let out = eval(
        r#"
        let srv = udp_bind("127.0.0.1", 0)
        let addr = udp_local_addr(srv)
        let parts = split(addr, ":")
        let port = parse_int(parts[1])
        let cli = udp_bind("127.0.0.1", 0)
        udp_send(cli, "127.0.0.1", port, "udp-pkt")
        udp_recv(srv, 64)
        "#,
    );
    let Value::Object(map) = out else {
        panic!("expected udp recv object");
    };
    assert!(matches!(
        map.get("data"),
        Some(Value::String(s)) if s == "udp-pkt"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[cfg(target_os = "windows")]
fn deno_run_command() {
    let out = eval(r#"run_command("cmd", ["/C", "echo", "kab"])"#);
    let Value::Object(map) = out else {
        panic!("expected command result");
    };
    assert!(matches!(map.get("code"), Some(Value::Number(0))));
    let stdout = map.get("stdout").and_then(|v| match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    });
    assert!(stdout.is_some_and(|s| s.contains("kab")));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_open_kv_roundtrip() {
    let path = format!("deno_kv_parity_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        kv_set(kv, ["users", "1"], "alice")
        let v = kv_get(kv, ["users", "1"])
        kv_close(kv)
        v
        "#
    ));
    assert!(matches!(out, Value::String(s) if s == "alice"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_open_kv_reopen_persists() {
    let path = format!("deno_kv_reopen_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));

    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        kv_set(kv, ["persist"], "yes")
        kv_close(kv)
        let kv2 = open_kv("{path}")
        let v = kv_get(kv2, ["persist"])
        kv_close(kv2)
        v
        "#
    ));
    assert!(matches!(out, Value::String(s) if s == "yes"));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_atomic_batch() {
    let path = format!("deno_kv_atomic_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        let res = kv_atomic(kv, [
            {{ "op": "set", "key": ["a"], "value": 10 }},
            {{ "op": "set", "key": ["b"], "value": 20 }},
            {{ "op": "get", "key": ["a"] }}
        ])
        kv_close(kv)
        res
        "#
    ));
    let Value::Object(map) = out else {
        panic!("expected atomic result object");
    };
    assert!(matches!(map.get("ok"), Some(Value::Bool(true))));
    let Value::Array(results) = map.get("results").cloned().unwrap_or(Value::Array(vec![])) else {
        panic!("expected results array");
    };
    assert!(matches!(results.first(), Some(Value::Object(obj))
        if matches!(obj.get("value"), Some(Value::Number(10)))));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_watch_stream() {
    let path = format!("deno_kv_watch_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        let s = kv_watch(kv, ["evt"])
        kv_set(kv, ["evt", "1"], "changed")
        let chunk = stream_read(s)
        kv_close(kv)
        chunk["value"]["kind"]
        "#
    ));
    assert!(matches!(out, Value::String(s) if s == "set"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_listen_and_version() {
    let path = format!("deno_kv_v8_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        db_open("{path}")
        let kv = open_kv_db()
        kv_set(kv, ["app"], "v1")
        let entry = kv_get_entry(kv, ["app"])
        let listen = kv_listen(kv, ["app"])
        kv_set(kv, ["app"], "v2")
        let ev = kv_listen_recv(listen)
        kv_close(kv)
        [entry["version"], ev["version"], ev["kind"]]
        "#
    ));
    let Value::Array(items) = out else {
        panic!("expected array result");
    };
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
    assert!(matches!(&items[2], Value::String(s) if s == "set"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_shared_sql() {
    let path = format!("deno_kv_sqlshare_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        kv_set(kv, ["via"], "kv")
        db_open("{path}")
        sql("SELECT kv_value FROM _kab_kv WHERE kv_key = $1", "via")
        "#
    ));
    assert!(matches!(out, Value::String(s) if s.contains("kv")));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_sum_and_queue() {
    let path = format!("deno_kv_v9_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        let kv = open_kv("{path}")
        let sum1 = kv_atomic(kv, [{{ "op": "sum", "key": ["n"], "value": 10 }}])
        kv_enqueue(kv, ["q"], "job1")
        let item = kv_dequeue(kv, ["q"])
        let entries = kv_list_entries(kv, ["n"])
        kv_close(kv)
        [sum1["results"][0], item, entries[0]["value"]]
        "#
    ));
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(items[0], Value::Number(10)));
    assert!(matches!(&items[1], Value::String(s) if s == "job1"));
    assert!(matches!(items[2], Value::Number(10)));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn deno_kv_listen_async() {
    let path = format!("deno_kv_listen_async_{}.json", std::process::id());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
    let out = eval(&format!(
        r#"
        async fn wait_change() {{
            let kv = open_kv("{path}")
            let listen = kv_listen(kv, ["evt"])
            kv_set(kv, ["evt", "1"], "async")
            let ev = await kv_listen_async(listen)
            kv_close(kv)
            ev["kind"]
        }}
        await wait_change()
        "#
    ));
    assert!(matches!(out, Value::String(s) if s == "set"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.wal"));
    let _ = std::fs::remove_file(format!("{path}.wal2"));
}

#[test]
fn deno_worker_message() {
    let out = eval(
        r#"
        let w = worker_new()
        worker_post_message(w, "hello")
        worker_start(w, "let m = worker_poll(); worker_reply(m)")
        worker_recv(w)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "hello"));
}

#[test]
fn deno_worker_thread_isolate() {
    let out = eval(
        r#"
        let w = worker_new()
        worker_post_message(w, 41)
        worker_start(w, "worker_reply(worker_poll() + 1)")
        worker_join(w)
        worker_recv(w)
        "#,
    );
    assert!(matches!(out, Value::Number(42)));
}

#[test]
fn deno_worker_import_scripts() {
    let dir = std::env::temp_dir().join(format!("kab_worker_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let script = dir.join("worker_script.kab");
    std::fs::write(&script, r#"worker_reply("from-file")"#).unwrap();
    let path = script.to_string_lossy().replace('\\', "/");
    let src = format!(
        r#"
        let w = worker_new()
        worker_start_file(w, "{path}")
        worker_join(w)
        worker_recv(w)
        "#
    );
    let out = eval(&src);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(out, Value::String(s) if s == "from-file"));
}

#[test]
fn deno_worker_recv_async() {
    let out = eval(
        r#"
        async fn run() {
            let w = worker_new()
            worker_post_message(w, "async-ok")
            worker_start(w, "worker_reply(worker_poll())")
            return await worker_recv_async(w)
        }
        await run()
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "async-ok"));
}

#[test]
fn deno_worker_onmessage_main() {
    let out = eval(
        r#"
        env_set("KAB_LAST", "")
        let w = worker_new()
        worker_onmessage(w, (msg) => { env_set("KAB_LAST", msg) })
        worker_post_message(w, "evt")
        worker_start(w, "postMessage(worker_poll())")
        worker_join(w)
        worker_recv(w)
        env_get("KAB_LAST")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "evt"));
}

#[test]
fn deno_worker_poll_async_in_worker() {
    let out = eval(
        r#"
        async fn main() {
            let w = worker_new()
            worker_post_message(w, "loop")
            worker_start(w, "async fn run() { return await worker_poll_async(5000) }; postMessage(await run())")
            worker_join(w)
            return worker_recv(w)
        }
        await main()
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "loop"));
}

#[test]
fn deno_worker_message_loop() {
    let out = eval(
        r#"
        let w = worker_new()
        worker_post_message(w, "ping")
        worker_start(w, "onmessage((m) => postMessage(m)); worker_run_message_loop()")
        let msg = worker_recv(w)
        worker_terminate(w)
        msg
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "ping"));
}

#[test]
fn deno_ts_strip_types() {
    let out = eval(
        r#"
        ts_strip_types("let x: number = 1")
        "#,
    );
    let Value::String(s) = out else {
        panic!("expected string");
    };
    assert!(!s.contains(": number"));
    assert!(s.contains("let x"));
}

#[test]
fn deno_stream_read_async() {
    let out = eval(
        r#"
        async fn load() {
            let s = stream_from_array([7, 8])
            return await stream_read_all_async(s)
        }
        await load()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(7)));
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
#[test]
fn deno_unix_socket_echo() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    let path = std::env::temp_dir().join(format!("kab_unix_{}", std::process::id()));
    let path_str = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 64];
            if let Ok(n) = stream.read(&mut buf) {
                let _ = stream.write_all(&buf[..n]);
            }
        }
    });

    let out = eval(&format!(
        r#"
        let sock = unix_connect("{path_str}")
        unix_write(sock, "unix-echo")
        unix_read(sock, 64)
        "#
    ));
    server.join().expect("server");
    let _ = std::fs::remove_file(&path);
    assert!(matches!(out, Value::String(s) if s == "unix-echo"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[cfg(not(target_os = "windows"))]
fn deno_run_command() {
    let out = eval(r#"run_command("echo", ["kab"])"#);
    let Value::Object(map) = out else {
        panic!("expected command result");
    };
    assert!(matches!(map.get("code"), Some(Value::Number(0))));
    assert!(matches!(
        map.get("stdout"),
        Some(Value::String(s)) if s.trim() == "kab"
    ));
}

fn seed_npm_cache_math_lite(base: &std::path::Path) {
    use kabootar_lib::runtime::npm_remote::{package_dir, RegistryKind};
    let install = package_dir(RegistryKind::Npm, "math-lite", "1.0.0", base);
    let pkg = install.join("package");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.json"),
        r#"{"name":"math-lite","main":"index.js"}"#,
    )
    .unwrap();
    std::fs::write(
        pkg.join("index.js"),
        "pub fn twice(x) { return x + x }",
    )
    .unwrap();
}

#[test]
fn deno_npm_parse_spec() {
    let out = eval(r#"npm_parse_spec("jsr:@std/fmt@1.0.0")"#);
    let Value::Object(map) = out else {
        panic!("expected spec object");
    };
    assert!(matches!(map.get("kind"), Some(Value::String(s)) if s == "jsr"));
    assert!(matches!(map.get("name"), Some(Value::String(s)) if s == "@std/fmt"));
    assert!(matches!(map.get("version"), Some(Value::String(s)) if s == "1.0.0"));
}

#[test]
fn deno_npm_cache_import_and_list() {
    let base = std::env::temp_dir().join(format!(
        "kabootar_deno_npm_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    seed_npm_cache_math_lite(&base);

    let prev = std::env::var("KABOOTAR_PROJECT_ROOT").ok();
    std::env::set_var("KABOOTAR_PROJECT_ROOT", &base);

    let mut env = create_global_env();
    let src =
        eval_source(r#"npm_import("npm:math-lite", "1.0")"#, &mut env).unwrap();
    assert!(matches!(src, Value::String(s) if s.contains("twice")));

    let list = eval_source(r#"npm_list_cache()"#, &mut env).unwrap();
    let Value::Array(items) = list else {
        panic!("expected cache list");
    };
    assert_eq!(items.len(), 1);

    let out = eval_source(
        r#"
        import "npm:math-lite@1.0"
        twice(21)
        "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(out, Value::Number(42)));

    match prev {
        Some(v) => std::env::set_var("KABOOTAR_PROJECT_ROOT", v),
        None => std::env::remove_var("KABOOTAR_PROJECT_ROOT"),
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn deno_stream_reader_and_state() {
    let out = eval(
        r#"
        let s = stream_from_array([7, 8])
        let reader = stream_get_reader(s)
        let chunk = reader_read(reader)
        reader_release_lock(reader)
        [chunk["value"], stream_state(s)]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(items[0], Value::Number(7)));
    assert!(matches!(&items[1], Value::String(s) if s == "readable"));
}

#[test]
fn deno_transform_stream() {
    let out = eval(
        r#"
        fn double(x) { return x + x }
        let pair = transform_stream_new(double)
        writable_write(pair["writable"], 11)
        writable_close(pair["writable"])
        let reader = stream_get_reader(pair["readable"])
        let chunk = reader_read(reader)
        reader_release_lock(reader)
        chunk["value"]
        "#,
    );
    assert!(matches!(out, Value::Number(22)));
}

#[test]
fn deno_byte_stream_byob() {
    let out = eval(
        r#"
        let s = byte_stream_from_bytes([65, 66, 67])
        let buf = [0, 0, 0]
        let res = byte_stream_byob_read(s, buf)
        [res["read"], res["buffer"][0], res["buffer"][1]]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(items[0], Value::Number(3)));
    assert!(matches!(items[1], Value::Number(65)));
    assert!(matches!(items[2], Value::Number(66)));
}

#[test]
fn deno_stream_transfer_token() {
    let out = eval(
        r#"
        let s = stream_from_array([99])
        let token = stream_transfer(s)
        let s2 = stream_from_transfer(token["token"])
        let reader = stream_get_reader(s2)
        let chunk = reader_read(reader)
        reader_release_lock(reader)
        chunk["value"]
        "#,
    );
    assert!(matches!(out, Value::Number(99)));
}

#[test]
fn deno_ts_compile() {
    let out = eval(
        r#"
        let nl = from_char_code(10)
        let src = "interface X { a: number }" + nl + "enum E { A, B }" + nl + "let n: number = 1"
        let result = ts_compile(src)
        [len(result["diagnostics"]), result["code"]]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    let Value::String(code) = &items[1] else {
        panic!("expected code string");
    };
    assert!(!code.contains("interface"));
    assert!(code.contains("let E ="));
    assert!(!code.contains(": number"));
}

#[test]
fn deno_ts_compile_enum_usable() {
    let out = eval(
        r#"
        let nl = from_char_code(10)
        let src = "enum status { ok = 200, err = 500 }" + nl + "let code = status.ok"
        let result = ts_compile(src)
        result["code"]
        "#,
    );
    let Value::String(code) = out else {
        panic!("expected transpiled code");
    };
    let mut env = create_global_env();
    let result = eval_source(&code, &mut env).unwrap();
    assert!(matches!(result, Value::Number(200)));
}

#[test]
fn deno_node_list_and_resolve() {
    let list = eval(r#"node_list()"#);
    let Value::Array(items) = list else {
        panic!("expected module list");
    };
    assert!(items.len() >= 7);
    assert!(items.iter().any(|v| matches!(v, Value::String(s) if s == "node:path")));

    let out = eval(r#"node_resolve("node:path")"#);
    assert!(matches!(out, Value::String(s) if s == "node:path"));

    let bad = eval(r#"node_resolve("node:nope")"#);
    let Value::Object(map) = bad else {
        panic!("expected error object");
    };
    assert!(map.get("error").is_some());
}

#[test]
fn deno_node_path_import() {
    let out = eval(
        r#"
        import "node:path"
        join("/tmp", "kabootar.txt")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s.ends_with("kabootar.txt")));

    let out = eval(
        r#"
        import "node:path"
        extname("app.bundle.js")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == ".js"));
}

#[test]
fn deno_node_fs_import() {
    let out = eval(
        r#"
        import "node:fs"
        mkdirSync("/nodefs")
        writeFileSync("/nodefs/data.bin", [9, 8, 7])
        let bytes = readFileSync("/nodefs/data.bin")
        rmSync("/nodefs/data.bin")
        len(bytes)
        "#,
    );
    assert!(matches!(out, Value::Number(3)));
}

#[test]
fn deno_node_os_and_buffer() {
    let out = eval(
        r#"
        import "node:os"
        import "node:buffer"
        let plat = platform()
        let buf = from("hi")
        [plat, len(buf), isBuffer(buf)]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(&items[0], Value::String(_)));
    assert!(matches!(&items[1], Value::Number(2)));
    assert!(matches!(&items[2], Value::Bool(true)));
}

#[test]
fn deno_node_import_source() {
    let out = eval(r#"node_import("node:process")"#);
    assert!(matches!(out, Value::String(s) if s.contains("pub fn cwd")));
}

#[test]
fn deno_sab_uint8_and_transfer() {
    let out = eval(
        r#"
        let sab = sab_new(4)
        let view = uint8_array_new(sab, 0, 4)
        uint8_array_set(view, 0, 65)
        uint8_array_set(view, 1, 66)
        let token = sab_transfer(sab)
        let sab2 = sab_from_transfer(token["token"])
        let view2 = uint8_array_new(sab2, 0, 2)
        [uint8_array_get(view2, 0), uint8_array_get(view2, 1), sab_byte_length(sab2)]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(&items[0], Value::Number(65)));
    assert!(matches!(&items[1], Value::Number(66)));
    assert!(matches!(&items[2], Value::Number(4)));
}

#[test]
fn deno_sab_atomics_add_and_cas() {
    let out = eval(
        r#"
        let sab = sab_new(4)
        let view = int32_array_new(sab, 0, 1)
        atomics_store(view, 0, 10)
        let old = atomics_add(view, 0, 5)
        let swapped = atomics_compare_exchange(view, 0, 15, 99)
        let val = atomics_load(view, 0)
        [old, swapped, val]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert!(matches!(&items[0], Value::Number(10)));
    assert!(matches!(&items[1], Value::Bool(true)));
    assert!(matches!(&items[2], Value::Number(99)));
}

#[test]
fn deno_sab_worker_transfer() {
    let dir = std::env::temp_dir().join(format!("kab_sab_worker_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let script = dir.join("sab_worker.kab");
    std::fs::write(
        &script,
        r#"
let msg = worker_poll()
let transfers = msg["transfers"]
let sab2 = transfers[0]
let v = int32_array_new(sab2, 0, 1)
atomics_add(v, 0, 3)
worker_reply(atomics_load(v, 0))
"#,
    )
    .unwrap();
    let path = script.to_string_lossy().replace('\\', "/");
    let src = format!(
        r#"
        let sab = sab_new(4)
        let view = int32_array_new(sab, 0, 1)
        atomics_store(view, 0, 7)
        let w = worker_new()
        worker_post_message(w, {{ "op": "add" }}, [sab])
        worker_start_file(w, "{path}")
        worker_join(w)
        worker_recv(w)
        "#
    );
    let out = eval(&src);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(out, Value::Number(10)));
}

fn pem_literal(pem: &str) -> String {
    pem.replace('\\', "\\\\").replace('"', "\\\"")
}

fn eval_with_project_root(code: &str, root: &std::path::Path) -> Value {
    let old_root = std::env::var("KABOOTAR_PROJECT_ROOT").ok();
    let old_cwd = std::env::current_dir().ok();
    std::env::set_var("KABOOTAR_PROJECT_ROOT", root);
    std::env::set_current_dir(root).unwrap();
    let out = eval(code);
    match old_root {
        Some(r) => std::env::set_var("KABOOTAR_PROJECT_ROOT", r),
        None => std::env::remove_var("KABOOTAR_PROJECT_ROOT"),
    }
    if let Some(c) = old_cwd {
        let _ = std::env::set_current_dir(c);
    }
    out
}

#[test]
fn js_wave_b1_serve_dispatch() {
    let out = eval(
        r#"
        fn handler(req) {
            return response_new(200, request_url(req))
        }
        let res = serve_dispatch(handler, "GET", "/api/health")
        res.status == 200 && res.body == "/api/health"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn handler(req) { return response_new(200, "ok") }
        let ready = serve_async_ready(handler, 8080)
        is_object(ready) && ready.ready == true
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(r#"http2_supported() == true"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        http2_preface_ok("PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n") == true
            && http2_preface_ok("GET / HTTP/1.1\r\n") == false
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn js_wave_b1_serve_async_live() {
    use kabootar_lib::evaluator::{create_global_env, eval_source};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    let mut env = create_global_env();
    let port_val = eval_source(
        r#"
        fn handler(req) { return response_new(200, "live") }
        let ready = serve_async_ready(handler, 0)
        ready.port
        "#,
        &mut env,
    )
    .unwrap();
    let Value::Number(port) = port_val else {
        panic!("expected port");
    };

    let client = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("tcp connect");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("write");
        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).expect("read");
        String::from_utf8_lossy(&buf[..n]).contains("live")
    });

    for _ in 0..50 {
        let _ = eval_source("serve_async_poll()", &mut env);
        thread::sleep(Duration::from_millis(10));
        if client.is_finished() {
            break;
        }
    }
    assert!(client.join().unwrap());
}

#[test]
fn js_wave_b2_stream_tee_cancel_propagates() {
    let out = eval(
        r#"
        let s = stream_from_array([1, 2])
        let branches = stream_tee(s)
        let a = branches[0]
        let b = branches[1]
        stream_cancel(a)
        stream_state(b) == "cancelled"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b3_permissions() {
    let out = eval(
        r#"
        permissions_grant({ name: "read", path: "/tmp" })
        permissions_query({ name: "read", path: "/tmp" }) == "granted"
            && permissions_request({ name: "net" }) == "granted"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        permissions_revoke({ name: "read", path: "/tmp" })
        permissions_query({ name: "read", path: "/tmp" }) == "denied"
            && Deno_permissions.query({ name: "net" }) == "granted"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b4_deno_test_and_bench() {
    let out = eval(
        r#"
        fn adds_test() { 1 + 1 }
        fn noop_bench() { null }
        deno_test("adds", adds_test)
        deno_bench("noop", noop_bench)
        let tr = deno_test_report()
        let br = deno_bench_report()
        tr.passed == 1 && tr.failed == 0 && len(br.benches) == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b5_lockfile_read() {
    let out = eval(
        r#"
        let lf = lockfile_read()
        lf.version >= 0 && is_object(lf.packages)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b5_lockfile_sync_from_manifest() {
    let dir = std::env::temp_dir().join(format!("kab_lockfile_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(
        dir.join("kabootar.toml"),
        r#"
version = "0.1.0"

[dependencies]
lodash = "^4.17.21"
"#,
    )
    .unwrap();

    let out = eval_with_project_root(
        r#"
        let lf = lockfile_sync()
        is_object(lf.packages) && lf.packages["lodash"]["version"] == "^4.17.21"
        "#,
        &dir,
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(out, Value::Bool(true)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn js_wave_b6_realpath_cwd() {
    let out = eval(
        r#"
        let p = realpath(".")
        len(p) > 0 && len(cwd()) > 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn js_wave_b6_hard_link() {
    let dir = std::env::temp_dir().join(format!("kab_b6_link_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.join("target.txt");
    std::fs::write(&target, "linked").unwrap();
    let hard = dir.join("hard.txt");
    let target_s = target.to_string_lossy().replace('\\', "/");
    let hard_s = hard.to_string_lossy().replace('\\', "/");

    let out = eval(&format!(
        r#"
        link("{target_s}", "{hard_s}")
        true
        "#
    ));
    let hard_ok = std::fs::read_to_string(&hard).is_ok_and(|s| s == "linked");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(out, Value::Bool(true)) && hard_ok);
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
#[test]
fn js_wave_b6_symlink() {
    let dir = std::env::temp_dir().join(format!("kab_b6_symlink_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.join("target.txt");
    std::fs::write(&target, "symlinked").unwrap();
    let link_path = dir.join("link.txt");
    let target_s = target.to_string_lossy().replace('\\', "/");
    let link_s = link_path.to_string_lossy().replace('\\', "/");

    let out = eval(&format!(
        r#"
        Deno_symlink("{target_s}", "{link_s}")
        true
        "#
    ));
    let link_ok = std::fs::read_to_string(&link_path).is_ok_and(|s| s == "symlinked");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(matches!(out, Value::Bool(true)) && link_ok);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn js_wave_b7_tls_listen_accept_read() {
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::CertificateDer;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = probe.local_addr().expect("addr").port();
    drop(probe);

    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let client = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(cert_der).expect("add cert");
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let server_name =
            rustls::pki_types::ServerName::try_from("localhost".to_string()).expect("server name");
        let mut tcp = TcpStream::connect(format!("127.0.0.1:{port}")).expect("connect");
        let mut conn =
            rustls::ClientConnection::new(Arc::new(config), server_name).expect("client");
        let mut tls = rustls::Stream::new(&mut conn, &mut tcp);
        tls.write_all(b"kab-tls").expect("write");
        let _ = tls.flush();
        thread::sleep(Duration::from_millis(300));
    });

    let code = format!(
        r#"
        let lid = tls_listen("127.0.0.1", {port}, "{cert_pem}", "{key_pem}")
        let sock = tls_accept(lid)
        let text = tls_server_read(sock, 64)
        tls_server_close(sock)
        tls_server_close(lid)
        lid > 0 && text == "kab-tls"
        "#,
        port = port,
        cert_pem = pem_literal(&cert_pem),
        key_pem = pem_literal(&key_pem),
    );
    let out = eval(&code);
    let _ = client.join();
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b8_shared_worker() {
    let out = eval(
        r#"
        let id1 = shared_worker_connect("pool")
        let id2 = shared_worker_connect("pool")
        id1 == id2
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_b8_shared_worker_post_message() {
    let out = eval(
        r#"
        let name = "msg-pool"
        shared_worker_connect(name)
        shared_worker_post_message(name, { "ping": 1 })
        let msg = shared_worker_recv(name)
        msg["ping"] == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}
