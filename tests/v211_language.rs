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
