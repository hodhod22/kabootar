//! v2.11 — TLS custom CA and certificate pinning

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[cfg(not(target_arch = "wasm32"))]
struct LocalTlsServer {
    port: u16,
    ca_pem: String,
    pin_hex: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_local_tls_server() -> LocalTlsServer {
    use kabootar_lib::runtime::tls_trust::cert_pem_sha256_hex;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let pin_hex = cert_pem_sha256_hex(&ca_pem).expect("pin");
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());

    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server_cfg = Arc::new(server_cfg);
    thread::spawn(move || {
        if let Ok((mut tcp, _)) = listener.accept() {
            let mut server_conn =
                rustls::ServerConnection::new(server_cfg.clone()).expect("server conn");
            let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
            let mut buf = [0u8; 4096];
            let _ = tls.read(&mut buf);
            let response =
                "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\ntls-ok";
            tls.write_all(response.as_bytes()).expect("write");
            let _ = server_conn.send_close_notify();
            while server_conn.wants_write() {
                server_conn.write_tls(&mut tcp).expect("flush tls");
            }
        }
    });
    thread::sleep(std::time::Duration::from_millis(50));

    LocalTlsServer {
        port,
        ca_pem,
        pin_hex,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_local_tls_redirect_server(
    location: Option<&str>,
    expected_authorization: Option<&str>,
) -> LocalTlsServer {
    use kabootar_lib::runtime::tls_trust::cert_pem_sha256_hex;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let pin_hex = cert_pem_sha256_hex(&ca_pem).expect("pin");
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server_cfg = Arc::new(server_cfg);
    let location = location.map(|location| {
        if location == "absolute" {
            format!("https://localhost:{port}/final")
        } else {
            location.to_string()
        }
    });
    let expected_authorization = expected_authorization.map(str::to_string);
    let responses = if let Some(location) = location {
        vec![
            format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            "HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\nredirect-tls-ok!"
                .to_string(),
        ]
    } else {
        vec![
            "HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
        ]
    };

    thread::spawn(move || {
        for response in responses {
            let (mut tcp, _) = listener.accept().expect("accept TLS connection");
            let mut server_conn =
                rustls::ServerConnection::new(server_cfg.clone()).expect("server conn");
            let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
            let mut buf = [0u8; 4096];
            let read = tls.read(&mut buf).expect("read request");
            if let Some(expected_authorization) = &expected_authorization {
                let request = std::str::from_utf8(&buf[..read]).expect("request UTF-8");
                assert!(
                    request.lines().any(|line| {
                        line.eq_ignore_ascii_case(&format!(
                            "Authorization: {expected_authorization}"
                        ))
                    }),
                    "request did not retain Authorization header: {request:?}"
                );
            }
            tls.write_all(response.as_bytes()).expect("write response");
            let _ = server_conn.send_close_notify();
            while server_conn.wants_write() {
                server_conn.write_tls(&mut tcp).expect("flush tls");
            }
        }
    });
    thread::sleep(std::time::Duration::from_millis(50));

    LocalTlsServer {
        port,
        ca_pem,
        pin_hex,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_local_tls_redirect_loop_server() -> LocalTlsServer {
    use kabootar_lib::runtime::tls_trust::cert_pem_sha256_hex;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let pin_hex = cert_pem_sha256_hex(&ca_pem).expect("pin");
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let redirect_url = format!("https://localhost:{port}/loop");
    let server_cfg = Arc::new(server_cfg);

    thread::spawn(move || {
        for _ in 0..=10 {
            let (mut tcp, _) = listener.accept().expect("accept TLS redirect connection");
            let mut server_conn =
                rustls::ServerConnection::new(server_cfg.clone()).expect("server conn");
            let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
            let mut buf = [0u8; 4096];
            let _ = tls.read(&mut buf);
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {redirect_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            tls.write_all(response.as_bytes()).expect("write response");
            let _ = server_conn.send_close_notify();
            while server_conn.wants_write() {
                server_conn.write_tls(&mut tcp).expect("flush tls");
            }
        }
    });
    thread::sleep(std::time::Duration::from_millis(50));

    LocalTlsServer {
        port,
        ca_pem,
        pin_hex,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_local_tls_post_redirect_server(
    redirect_status: u16,
) -> (LocalTlsServer, std::thread::JoinHandle<()>) {
    use kabootar_lib::runtime::tls_trust::cert_pem_sha256_hex;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    fn read_http_request(tls: &mut rustls::Stream<'_, rustls::ServerConnection, TcpStream>) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];
        let header_end = loop {
            let read = tls.read(&mut buf).expect("read TLS request");
            assert!(read > 0, "client closed TLS connection before sending request");
            request.extend_from_slice(&buf[..read]);
            if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break end + 4;
            }
        };
        let headers = std::str::from_utf8(&request[..header_end]).expect("request headers UTF-8");
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().expect("valid Content-Length"))
                })
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = tls.read(&mut buf).expect("read TLS request body");
            assert!(read > 0, "client closed TLS connection before sending request body");
            request.extend_from_slice(&buf[..read]);
        }
        request
    }

    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
    let ca_pem = cert.cert.pem();
    let pin_hex = cert_pem_sha256_hex(&ca_pem).expect("pin");
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let server_cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(WebPkiClientVerifier::no_client_auth())
        .with_single_cert(vec![cert_der], key_der)
        .expect("server config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server_cfg = Arc::new(server_cfg);
    let redirect_url = format!("https://localhost:{port}/final");

    let server_thread = std::thread::spawn(move || {
        let (mut tcp, _) = listener.accept().expect("accept POST TLS connection");
        let mut server_conn =
            rustls::ServerConnection::new(server_cfg.clone()).expect("server conn");
        let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
        let first_request = read_http_request(&mut tls);
        let first_headers_end = first_request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("first request headers") + 4;
        assert!(
            first_request.starts_with(b"POST /start HTTP/1.1\r\n"),
            "expected POST /start, got {:?}",
            String::from_utf8_lossy(&first_request[..first_headers_end])
        );
        let expected_body = if matches!(redirect_status, 301 | 302 | 303) {
            b"post-body-to-get".as_slice()
        } else {
            b"post-body-preserved".as_slice()
        };
        assert_eq!(&first_request[first_headers_end..], expected_body);
        let response = format!(
            "HTTP/1.1 {redirect_status} Redirect\r\nLocation: {redirect_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        tls.write_all(response.as_bytes())
            .expect("write redirect response");
        let _ = server_conn.send_close_notify();
        while server_conn.wants_write() {
            server_conn.write_tls(&mut tcp).expect("flush TLS");
        }

        let (mut tcp, _) = listener.accept().expect("accept redirect TLS connection");
        let mut server_conn =
            rustls::ServerConnection::new(server_cfg).expect("server conn");
        let mut tls = rustls::Stream::new(&mut server_conn, &mut tcp);
        let second_request = read_http_request(&mut tls);
        let second_headers_end = second_request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("second request headers") + 4;
        if matches!(redirect_status, 301 | 302 | 303) {
            assert!(
                second_request.starts_with(b"GET /final HTTP/1.1\r\n"),
                "expected GET /final, got {:?}",
                String::from_utf8_lossy(&second_request[..second_headers_end])
            );
            assert!(
                second_request[second_headers_end..].is_empty(),
                "redirected GET retained POST body: {:?}",
                String::from_utf8_lossy(&second_request[second_headers_end..])
            );
        } else {
            assert!(
                second_request.starts_with(b"POST /final HTTP/1.1\r\n"),
                "expected POST /final, got {:?}",
                String::from_utf8_lossy(&second_request[..second_headers_end])
            );
            assert_eq!(
                &second_request[second_headers_end..],
                b"post-body-preserved",
                "redirected POST did not preserve body"
            );
        }
        let response =
            "HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\npost-303-tls-ok!!!";
        tls.write_all(response.as_bytes()).expect("write final response");
        let _ = server_conn.send_close_notify();
        while server_conn.wants_write() {
            server_conn.write_tls(&mut tcp).expect("flush TLS");
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(50));

    (
        LocalTlsServer {
            port,
            ca_pem,
            pin_hex,
        },
        server_thread,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn pem_literal(pem: &str) -> String {
    pem.replace('\\', "\\\\").replace('"', "\\\"")
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_cert_sha256_native() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let v = eval_source(
        &format!(r#"tls_cert_sha256("{pem_lit}")"#),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == server.pin_hex));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_ca_only_allows_self_signed_fetch() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let v = eval_source(
        &format!(
            r#"
            tls_ca_only("{pem_lit}")
            async fn load() {{
                return await http_fetch_async("GET", "https://localhost:{}/", "")
            }}
            let r = await load()
            http_body(r)
        "#,
            server.port
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "tls-ok"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_pin_rejects_wrong_fingerprint() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let err = eval_source(
        &format!(
            r#"
            tls_ca_only("{pem_lit}")
            tls_pin("localhost", "0000000000000000000000000000000000000000000000000000000000000000")
            async fn load() {{
                return await http_fetch_async("GET", "https://localhost:{}/", "")
            }}
            await load()
        "#,
            server.port
        ),
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("pin mismatch") || err.contains("Certificate pin"));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_pin_accepts_matching_fingerprint() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let v = eval_source(
        &format!(
            r#"
            tls_ca_only("{pem_lit}")
            tls_pin("localhost", "{pin}")
            async fn load() {{
                return await http_fetch_async("GET", "https://localhost:{port}/", "")
            }}
            http_status(await load())
        "#,
            pin = server.pin_hex,
            port = server.port
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(200)));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_add_ca_trusts_extra_certificate() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let v = eval_source(
        &format!(
            r#"
            tls_add_ca("{pem_lit}")
            async fn load() {{
                return await http_fetch_async("GET", "https://localhost:{}/", "")
            }}
            http_status(await load())
        "#,
            server.port
        ),
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(200)));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_uses_outer_tls_trust() {
    let server = spawn_local_tls_server();
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("GET", "https://localhost:{port}/", "")
http_body(response)"#,
            port = server.port,
            pin = server.pin_hex
        ),
    )
    .expect("write Kab TLS fetch program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-fetch".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program = compile_file_cached(&path_for_thread).expect("compile Kab TLS fetch");
            let value = eval_program(&program, &mut env).expect("run Kab TLS fetch");
            assert!(matches!(value, Value::String(s) if s == "tls-ok"));
        })
        .expect("spawn Kab TLS fetch thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS fetch thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_follows_tls_redirect_with_ca_and_pin() {
    let server = spawn_local_tls_redirect_server(Some("absolute"), None);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-redirect-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("GET", "https://localhost:{port}/start", "")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "redirect-tls-ok!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS redirect thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_preserves_headers_across_tls_redirect() {
    let server =
        spawn_local_tls_redirect_server(Some("absolute"), Some("Bearer redirect-token"));
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-header-redirect-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("GET", "https://localhost:{port}/start", "", {{"Authorization": "Bearer redirect-token"}})
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS header redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-header-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS header redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS header redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "redirect-tls-ok!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS header redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS header redirect thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_follows_relative_tls_redirect_with_ca_and_pin() {
    let server = spawn_local_tls_redirect_server(Some("/final"), None);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-relative-redirect-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("GET", "https://localhost:{port}/start", "")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS relative redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-relative-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS relative redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS relative redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "redirect-tls-ok!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS relative redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS relative redirect thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_rejects_redirect_without_location() {
    let server = spawn_local_tls_redirect_server(None, None);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-redirect-no-location-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
await httpFetch("GET", "https://localhost:{port}/start", "")"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS redirect-without-location program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-redirect-no-location".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS redirect no Location");
            let error =
                eval_program(&program, &mut env).expect_err("redirect without Location must reject");
            assert!(
                error.contains("redirect 302 missing Location"),
                "expected missing Location error, got {error:?}"
            );
        })
        .expect("spawn Kab TLS redirect no Location thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS redirect no Location thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_rejects_too_many_redirects() {
    let server = spawn_local_tls_redirect_loop_server();
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-redirect-loop-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
await httpFetch("GET", "https://localhost:{port}/loop", "")"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS redirect-loop program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-redirect-loop".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS redirect loop");
            let error = eval_program(&program, &mut env).expect_err("redirect loop must reject");
            assert!(
                error.contains("Too many HTTP redirects (max 10)"),
                "expected redirect-limit error, got {error:?}"
            );
        })
        .expect("spawn Kab TLS redirect loop thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS redirect loop thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_post_303_redirect_changes_to_get() {
    let (server, server_thread) = spawn_local_tls_post_redirect_server(303);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-post-303-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("POST", "https://localhost:{port}/start", "post-body-to-get")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS POST redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-post-303-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS POST redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS POST redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "post-303-tls-ok!!!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS POST redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS POST redirect thread");
    server_thread.join().expect("join TLS POST redirect server");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_post_302_redirect_changes_to_get() {
    let (server, server_thread) = spawn_local_tls_post_redirect_server(302);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-post-302-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("POST", "https://localhost:{port}/start", "post-body-to-get")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS POST 302 redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-post-302-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS POST 302 redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS POST 302 redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "post-303-tls-ok!!!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS POST 302 redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS POST 302 redirect thread");
    server_thread.join().expect("join TLS POST 302 redirect server");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_post_301_redirect_changes_to_get() {
    let (server, server_thread) = spawn_local_tls_post_redirect_server(301);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-post-301-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("POST", "https://localhost:{port}/start", "post-body-to-get")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS POST 301 redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-post-301-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS POST 301 redirect");
            let value =
                eval_program(&program, &mut env).expect("run Kab TLS POST 301 redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "post-303-tls-ok!!!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS POST 301 redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS POST 301 redirect thread");
    server_thread.join().expect("join TLS POST 301 redirect server");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_post_307_redirect_preserves_method_and_body() {
    let (server, server_thread) = spawn_local_tls_post_redirect_server(307);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-post-307-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("POST", "https://localhost:{port}/start", "post-body-preserved")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS POST 307 redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-post-307-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS POST 307 redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS POST 307 redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "post-303-tls-ok!!!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS POST 307 redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS POST 307 redirect thread");
    server_thread.join().expect("join TLS POST 307 redirect server");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_post_308_redirect_preserves_method_and_body() {
    let (server, server_thread) = spawn_local_tls_post_redirect_server(308);
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-post-308-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "{pin}")
let response = await httpFetch("POST", "https://localhost:{port}/start", "post-body-preserved")
http_body(response)"#,
            pin = server.pin_hex,
            port = server.port
        ),
    )
    .expect("write Kab TLS POST 308 redirect program");

    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-post-308-redirect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program =
                compile_file_cached(&path_for_thread).expect("compile Kab TLS POST 308 redirect");
            let value = eval_program(&program, &mut env).expect("run Kab TLS POST 308 redirect");
            assert!(
                matches!(value, Value::String(ref s) if s == "post-303-tls-ok!!!"),
                "expected final redirect body, got {value:?}"
            );
        })
        .expect("spawn Kab TLS POST 308 redirect thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS POST 308 redirect thread");
    server_thread.join().expect("join TLS POST 308 redirect server");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_imported_http_fetch_rejects_wrong_tls_pin() {
    let server = spawn_local_tls_server();
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-wrong-pin-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_pin("localhost", "0000000000000000000000000000000000000000000000000000000000000000")
await httpFetch("GET", "https://localhost:{port}/", "")"#,
            port = server.port
        ),
    )
    .expect("write Kab TLS wrong-pin program");
    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-wrong-pin".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program = compile_file_cached(&path_for_thread).expect("compile Kab TLS wrong-pin");
            let error = eval_program(&program, &mut env).expect_err("Kab TLS pin must reject");
            assert!(error.contains("pin mismatch") || error.contains("Certificate pin"));
        })
        .expect("spawn Kab TLS wrong-pin thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS wrong-pin thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn kab_vm_tls_reset_restores_default_trust() {
    let server = spawn_local_tls_server();
    let pem_lit = pem_literal(&server.ca_pem);
    let path = std::env::temp_dir().join(format!(
        "kab-v211-tls-http-fetch-reset-{}-{}.kab",
        std::process::id(),
        server.port
    ));
    std::fs::write(
        &path,
        format!(
            r#"import "kab/http/http_fetch"
tls_add_ca("{pem_lit}")
tls_reset()
await httpFetch("GET", "https://localhost:{port}/", "")"#,
            port = server.port
        ),
    )
    .expect("write Kab TLS reset program");
    let path_for_thread = path.to_string_lossy().into_owned();
    let result = std::thread::Builder::new()
        .name("v211-kab-vm-tls-reset".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};

            let mut env = create_global_env();
            let program = compile_file_cached(&path_for_thread).expect("compile Kab TLS reset");
            let error = eval_program(&program, &mut env).expect_err("Kab TLS reset must reject");
            assert!(error.contains("certificate") || error.contains("UnknownIssuer"));
        })
        .expect("spawn Kab TLS reset thread")
        .join();
    let _ = std::fs::remove_file(&path);
    result.expect("join Kab TLS reset thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn tls_reset_restores_default_trust() {
    let server = spawn_local_tls_server();
    let mut env = create_global_env();
    let pem_lit = pem_literal(&server.ca_pem);
    let err = eval_source(
        &format!(
            r#"
            tls_ca_only("{pem_lit}")
            tls_reset()
            async fn load() {{
                return await http_fetch_async("GET", "https://localhost:{}/", "")
            }}
            await load()
        "#,
            server.port
        ),
        &mut env,
    )
    .unwrap_err();
    assert!(
        err.contains("certificate")
            || err.contains("TLS")
            || err.contains("handshake")
            || err.contains("UnknownIssuer")
            || err.contains("verify")
    );
}
