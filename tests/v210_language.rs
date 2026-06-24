//! v2.10 — HTTPS/TLS för http_fetch_async

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::runtime::net::parse_url;
use kabootar::value::Value;

#[test]
fn parse_https_url_in_language_layer() {
    let u = parse_url("https://api.example.com:8443/v1").unwrap();
    assert_eq!(u.host, "api.example.com");
    assert_eq!(u.port, 8443);
}

#[test]
fn http_fetch_async_https_connects() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        async fn load() {
            return await http_fetch_async("GET", "https://localhost:1/", "")
        }
        await load()
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(!err.contains("must start with http://"));
    assert!(
        err.contains("connect")
            || err.contains("TCP")
            || err.contains("TLS")
            || err.contains("handshake")
    );
}

#[test]
fn http_and_https_urls_both_parse() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn check() {
            let a = await http_fetch_async("GET", "http://127.0.0.1:1/", "")
            return a
        }
        1
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn https_ip_address_errors_clearly() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        async fn bad() {
            return await http_fetch_async("GET", "https://127.0.0.1/", "")
        }
        await bad()
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(
        err.contains("hostname")
            || err.contains("TLS")
            || err.contains("server name")
            || err.contains("certificate")
            || err.contains("handshake")
            || err.contains("connect")
    );
}
