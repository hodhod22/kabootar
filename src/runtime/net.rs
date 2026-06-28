//! Real network HTTP client (native TCP + TLS) for Kabootar.

use crate::runtime::http::HttpResponse;
use crate::runtime::tls_trust::{TlsTrust, verify_peer_pin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

#[derive(Debug, Clone)]
pub struct ParsedUrl {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub fn parse_url(url: &str) -> Result<ParsedUrl, String> {
    if let Some(rest) = url.strip_prefix("https://") {
        return parse_authority(rest, Scheme::Https, 443);
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return parse_authority(rest, Scheme::Http, 80);
    }
    Err("URL must start with http:// or https://".into())
}

/// Back-compat alias used by older tests.
pub fn parse_http_url(url: &str) -> Result<ParsedUrl, String> {
    parse_url(url)
}

fn parse_authority(rest: &str, scheme: Scheme, default_port: u16) -> Result<ParsedUrl, String> {
    let (authority, path) = match rest.split_once('/') {
        Some((auth, path)) => (auth, format!("/{path}")),
        None => (rest, "/".to_string()),
    };
    if authority.is_empty() {
        return Err("Invalid URL: missing host".into());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p
                .parse()
                .map_err(|_| format!("Invalid port in URL: {}", p))?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), default_port),
    };
    Ok(ParsedUrl {
        scheme,
        host,
        port,
        path,
    })
}

fn parse_http_head(head: &str) -> Result<(i64, std::collections::HashMap<String, String>), String> {
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or("Invalid HTTP response: empty")?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or("Invalid HTTP response status line")?
        .parse::<i64>()
        .map_err(|_| format!("Invalid HTTP status code in: {}", status_line))?;
    let mut headers = std::collections::HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok((status, headers))
}

pub fn parse_http_response(raw: &str) -> Result<HttpResponse, String> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or("Invalid HTTP response: missing header/body separator")?;
    let (status, headers) = parse_http_head(head)?;
    Ok(HttpResponse {
        status,
        body: body.to_string(),
        headers,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_http_response_bytes(raw: &[u8]) -> Result<(HttpResponse, Vec<u8>), String> {
    let sep = b"\r\n\r\n";
    let pos = raw
        .windows(sep.len())
        .position(|window| window == sep)
        .ok_or("Invalid HTTP response: missing header/body separator")?;
    let head = std::str::from_utf8(&raw[..pos])
        .map_err(|e| format!("Invalid HTTP response headers (not UTF-8): {e}"))?;
    let (status, headers) = parse_http_head(head)?;
    let mut body = raw[pos + sep.len()..].to_vec();
    if let Some(len) = headers.get("content-length") {
        if let Ok(n) = len.trim().parse::<usize>() {
            body.truncate(n.min(body.len()));
        }
    }
    let text_body = String::from_utf8_lossy(&body).into_owned();
    Ok((
        HttpResponse {
            status,
            body: text_body,
            headers,
        },
        body,
    ))
}

fn build_http_request(
    method: &str,
    host: &str,
    path: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
) -> String {
    let mut lines = vec![
        format!("{method} {path} HTTP/1.1"),
        format!("Host: {host}"),
    ];
    let mut has_content_length = false;
    let mut has_connection = false;
    for (key, value) in headers {
        let key_lower = key.to_ascii_lowercase();
        if key_lower == "host" {
            continue;
        }
        if key_lower == "content-length" {
            has_content_length = true;
        }
        if key_lower == "connection" {
            has_connection = true;
        }
        lines.push(format!("{key}: {value}"));
    }
    if !body.is_empty() && !has_content_length {
        lines.push(format!("Content-Length: {}", body.len()));
    }
    if !has_connection {
        lines.push("Connection: close".to_string());
    }
    lines.push(String::new());
    lines.push(body.to_string());
    lines.join("\r\n")
}

const MAX_REDIRECTS: u32 = 10;

fn is_redirect_status(status: i64) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn resolve_redirect_url(parsed: &ParsedUrl, location: &str) -> Result<String, String> {
    let location = location.trim();
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    if location.starts_with('/') {
        let scheme = match parsed.scheme {
            Scheme::Http => "http",
            Scheme::Https => "https",
        };
        let port_suffix = match parsed.scheme {
            Scheme::Http if parsed.port != 80 => format!(":{}", parsed.port),
            Scheme::Https if parsed.port != 443 => format!(":{}", parsed.port),
            _ => String::new(),
        };
        return Ok(format!(
            "{scheme}://{}{port_suffix}{location}",
            parsed.host
        ));
    }
    Err(format!("Unsupported redirect Location: {location}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn format_io_timeout_error(e: std::io::Error, timeout_ms: u64, context: &str) -> String {
    if timeout_ms > 0
        && (e.kind() == std::io::ErrorKind::TimedOut
            || e.kind() == std::io::ErrorKind::WouldBlock)
    {
        return format!("HTTP fetch timed out after {timeout_ms}ms");
    }
    format!("{context}: {e}")
}

#[cfg(not(target_arch = "wasm32"))]
fn connect_tcp(host: &str, port: u16, timeout_ms: u64) -> Result<std::net::TcpStream, String> {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS lookup failed for {host}:{port}: {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err(format!("No addresses resolved for {host}:{port}"));
    }

    let stream = if timeout_ms > 0 {
        let dur = Duration::from_millis(timeout_ms);
        let mut last_err = None;
        let mut connected = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, dur) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }
        connected.ok_or_else(|| {
            format_io_timeout_error(
                last_err.unwrap_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "connect failed")
                }),
                timeout_ms,
                "TCP connect failed",
            )
        })?
    } else {
        TcpStream::connect((host, port))
            .map_err(|e| format_io_timeout_error(e, timeout_ms, "TCP connect failed"))?
    };

    if timeout_ms > 0 {
        let dur = Some(Duration::from_millis(timeout_ms));
        stream
            .set_read_timeout(dur)
            .map_err(|e| format!("Failed to set read timeout: {e}"))?;
        stream
            .set_write_timeout(dur)
            .map_err(|e| format!("Failed to set write timeout: {e}"))?;
    }
    Ok(stream)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_fetch_once(
    parsed: &ParsedUrl,
    method: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    match parsed.scheme {
        Scheme::Http => http_fetch_plain(parsed, method, body, headers, timeout_ms),
        Scheme::Https => http_fetch_tls(parsed, method, body, headers, trust, timeout_ms),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_over_stream<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    method: &str,
    host: &str,
    path: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    let (response, _) = fetch_over_stream_bytes(stream, method, host, path, body, headers, timeout_ms)?;
    Ok(response)
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_over_stream_bytes<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    method: &str,
    host: &str,
    path: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    timeout_ms: u64,
) -> Result<(HttpResponse, Vec<u8>), String> {
    let request = build_http_request(method, host, path, body, headers);
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format_io_timeout_error(e, timeout_ms, "Failed to write HTTP request"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format_io_timeout_error(e, timeout_ms, "Failed to read HTTP response"))?;
    parse_http_response_bytes(&response)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_fetch_plain(
    parsed: &ParsedUrl,
    method: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    let mut stream = connect_tcp(&parsed.host, parsed.port, timeout_ms)?;
    fetch_over_stream(
        &mut stream,
        method,
        &parsed.host,
        &parsed.path,
        body,
        headers,
        timeout_ms,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn build_root_store(trust: &TlsTrust) -> Result<rustls::RootCertStore, String> {
    use rustls::pki_types::CertificateDer;

    let mut root_store = rustls::RootCertStore::empty();
    if trust.mozilla_roots {
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    for der in &trust.extra_ca_der {
        root_store
            .add(CertificateDer::from(der.clone()))
            .map_err(|e| format!("Failed to add custom CA certificate: {e:?}"))?;
    }
    if root_store.is_empty() {
        return Err(
            "No trusted TLS root certificates configured (use tls_add_ca or tls_ca_only)"
                .into(),
        );
    }
    Ok(root_store)
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_tls_pin(host: &str, tls_conn: &rustls::ClientConnection, trust: &TlsTrust) -> Result<(), String> {
    let key = host.to_ascii_lowercase();
    if !trust.pins.contains_key(&key) {
        return Ok(());
    }
    let certs = tls_conn
        .peer_certificates()
        .ok_or_else(|| format!("Certificate pin set for {host} but peer sent no certificate"))?;
    let leaf = certs
        .first()
        .ok_or_else(|| format!("Certificate pin set for {host} but peer sent no certificate"))?;
    verify_peer_pin(host, leaf.as_ref(), trust)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_fetch_tls(
    parsed: &ParsedUrl,
    method: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    use std::sync::{Arc, Once};

    static TLS_INIT: Once = Once::new();
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    let root_store = build_root_store(trust)?;

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let server_name = rustls::pki_types::ServerName::try_from(parsed.host.as_str())
        .map_err(|_| {
            format!(
                "Invalid TLS server name: {} (use a hostname, not an IP, for HTTPS)",
                parsed.host
            )
        })?
        .to_owned();

    let mut tcp = connect_tcp(&parsed.host, parsed.port, timeout_ms)?;
    let mut tls_conn =
        rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| format!("TLS handshake setup failed: {e}"))?;
    let mut tls = rustls::Stream::new(&mut tls_conn, &mut tcp);
    let result = fetch_over_stream(
        &mut tls,
        method,
        &parsed.host,
        &parsed.path,
        body,
        headers,
        timeout_ms,
    )?;
    verify_tls_pin(&parsed.host, &tls_conn, trust)?;
    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn http_fetch(
    method: &str,
    url: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<HttpResponse, String> {
    let mut url = url.to_string();
    let mut method = method.to_string();
    let mut body = body.to_string();

    for hop in 0..=MAX_REDIRECTS {
        let parsed = parse_url(&url)?;
        let response = http_fetch_once(&parsed, &method, &body, headers, trust, timeout_ms)?;

        if !is_redirect_status(response.status) {
            return Ok(response);
        }
        if hop == MAX_REDIRECTS {
            return Err(format!("Too many HTTP redirects (max {MAX_REDIRECTS})"));
        }

        let location = response
            .headers
            .get("location")
            .ok_or_else(|| {
                format!(
                    "HTTP redirect {} missing Location header",
                    response.status
                )
            })?;
        url = resolve_redirect_url(&parsed, location)?;

        if matches!(response.status, 301 | 302 | 303) {
            method = "GET".to_string();
            body.clear();
        }
    }

    Err(format!("Too many HTTP redirects (max {MAX_REDIRECTS})"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn http_fetch_default(method: &str, url: &str, body: &str) -> Result<HttpResponse, String> {
    http_fetch(
        method,
        url,
        body,
        &std::collections::HashMap::new(),
        &TlsTrust::default(),
        0,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn http_fetch_bytes(
    method: &str,
    url: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<Vec<u8>, String> {
    let mut url = url.to_string();
    let mut method = method.to_string();
    let mut body = body.to_string();

    for hop in 0..=MAX_REDIRECTS {
        let parsed = parse_url(&url)?;
        let (response, bytes) =
            http_fetch_bytes_once(&parsed, &method, &body, headers, trust, timeout_ms)?;
        if !is_redirect_status(response.status) {
            if response.status < 200 || response.status >= 300 {
                return Err(format!(
                    "HTTP {} for {url}",
                    response.status
                ));
            }
            return Ok(bytes);
        }
        if hop == MAX_REDIRECTS {
            return Err(format!("Too many HTTP redirects (max {MAX_REDIRECTS})"));
        }
        let location = response
            .headers
            .get("location")
            .ok_or_else(|| {
                format!(
                    "HTTP redirect {} missing Location header",
                    response.status
                )
            })?;
        url = resolve_redirect_url(&parsed, location)?;
        if matches!(response.status, 301 | 302 | 303) {
            method = "GET".to_string();
            body.clear();
        }
    }
    Err(format!("Too many HTTP redirects (max {MAX_REDIRECTS})"))
}

#[cfg(not(target_arch = "wasm32"))]
fn http_fetch_bytes_once(
    parsed: &ParsedUrl,
    method: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<(HttpResponse, Vec<u8>), String> {
    match parsed.scheme {
        Scheme::Http => {
            let mut stream = connect_tcp(&parsed.host, parsed.port, timeout_ms)?;
            fetch_over_stream_bytes(
                &mut stream,
                method,
                &parsed.host,
                &parsed.path,
                body,
                headers,
                timeout_ms,
            )
        }
        Scheme::Https => http_fetch_tls_bytes(parsed, method, body, headers, trust, timeout_ms),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn http_fetch_tls_bytes(
    parsed: &ParsedUrl,
    method: &str,
    body: &str,
    headers: &std::collections::HashMap<String, String>,
    trust: &TlsTrust,
    timeout_ms: u64,
) -> Result<(HttpResponse, Vec<u8>), String> {
    use std::sync::{Arc, Once};

    static TLS_INIT: Once = Once::new();
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    let root_store = build_root_store(trust)?;
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let server_name = rustls::pki_types::ServerName::try_from(parsed.host.as_str())
        .map_err(|_| {
            format!(
                "Invalid TLS server name: {} (use a hostname, not an IP, for HTTPS)",
                parsed.host
            )
        })?
        .to_owned();

    let mut tcp = connect_tcp(&parsed.host, parsed.port, timeout_ms)?;
    let mut tls_conn =
        rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| format!("TLS handshake setup failed: {e}"))?;
    let mut tls = rustls::Stream::new(&mut tls_conn, &mut tcp);
    let result = fetch_over_stream_bytes(
        &mut tls,
        method,
        &parsed.host,
        &parsed.path,
        body,
        headers,
        timeout_ms,
    )?;
    verify_tls_pin(&parsed.host, &tls_conn, trust)?;
    Ok(result)
}

#[cfg(target_arch = "wasm32")]
pub fn http_fetch(
    _method: &str,
    _url: &str,
    _body: &str,
    _headers: &std::collections::HashMap<String, String>,
    _trust: &TlsTrust,
    _timeout_ms: u64,
) -> Result<HttpResponse, String> {
    Err("http_fetch requires native runtime (not available on WASM)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_url_with_port_and_path() {
        let u = parse_url("http://127.0.0.1:9090/api/health").unwrap();
        assert_eq!(u.scheme, Scheme::Http);
        assert_eq!(u.host, "127.0.0.1");
        assert_eq!(u.port, 9090);
        assert_eq!(u.path, "/api/health");
    }

    #[test]
    fn parse_https_url_default_port() {
        let u = parse_url("https://example.com/data").unwrap();
        assert_eq!(u.scheme, Scheme::Https);
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 443);
        assert_eq!(u.path, "/data");
    }

    #[test]
    fn parse_response_headers() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Test: abc\r\n\r\nok";
        let res = parse_http_response(raw).unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.body, "ok");
        assert_eq!(res.headers.get("content-type"), Some(&"text/plain".to_string()));
        assert_eq!(res.headers.get("x-test"), Some(&"abc".to_string()));
    }

    #[test]
    fn resolve_relative_redirect_url() {
        let parsed = parse_url("http://example.com/old").unwrap();
        let url = resolve_redirect_url(&parsed, "/new").unwrap();
        assert_eq!(url, "http://example.com/new");
    }

    #[test]
    fn resolve_absolute_redirect_url() {
        let parsed = parse_url("http://example.com/old").unwrap();
        let url = resolve_redirect_url(&parsed, "https://other.test/path").unwrap();
        assert_eq!(url, "https://other.test/path");
    }

    #[test]
    fn build_request_with_custom_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer tok".to_string());
        headers.insert("X-Custom".to_string(), "1".to_string());
        let raw = build_http_request("GET", "example.com", "/api", "", &headers);
        assert!(raw.contains("Authorization: Bearer tok"));
        assert!(raw.contains("X-Custom: 1"));
        assert!(raw.contains("Host: example.com"));
        assert!(raw.contains("Connection: close"));
    }

    #[test]
    fn build_request_with_string_key_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let raw = build_http_request("POST", "example.com", "/api", "{}", &headers);
        assert!(raw.contains("Content-Type: application/json"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tls_local_self_signed_roundtrip() {
        use rcgen::generate_simple_self_signed;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls::server::WebPkiClientVerifier;
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::Arc;
        use std::thread;

        let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("cert");
        let cert_der = CertificateDer::from(cert.cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());

        let mut server_roots = rustls::RootCertStore::empty();
        server_roots
            .add(cert_der.clone())
            .expect("add test cert");

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
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\ntls-ok";
                tls.write_all(response.as_bytes()).expect("write");
                let _ = server_conn.send_close_notify();
                while server_conn.wants_write() {
                    server_conn.write_tls(&mut tcp).expect("flush tls");
                }
            }
        });
        thread::sleep(std::time::Duration::from_millis(50));

        let mut client_roots = rustls::RootCertStore::empty();
        client_roots
            .add(cert.cert.der().as_ref().into())
            .expect("client root");
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(client_roots)
            .with_no_client_auth();
        let server_name = rustls::pki_types::ServerName::try_from("localhost")
            .expect("name")
            .to_owned();
        let mut tcp = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let mut tls_conn =
            rustls::ClientConnection::new(Arc::new(config), server_name).expect("client conn");
        let mut tls = rustls::Stream::new(&mut tls_conn, &mut tcp);
        tls.write_all(
            b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .expect("write");
        let mut response = Vec::new();
        tls.read_to_end(&mut response).expect("read");
        let res = parse_http_response(&String::from_utf8_lossy(&response)).expect("parse");
        assert_eq!(res.status, 200);
        assert_eq!(res.body, "tls-ok");
    }
}
