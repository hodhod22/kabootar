//! TLS server — `Deno.listenTls` parity (native host).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_TLS_LISTENER: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static TLS_LISTENERS: RefCell<HashMap<u64, TlsListenerState>> = RefCell::new(HashMap::new());
    static TLS_SERVER_CONNS: RefCell<HashMap<u64, TlsServerConn>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
struct TlsListenerState {
    listener: std::net::TcpListener,
    config: Arc<rustls::ServerConfig>,
}

#[cfg(not(target_arch = "wasm32"))]
struct TlsServerConn {
    conn: rustls::ServerConnection,
    tcp: std::net::TcpStream,
}

#[cfg(not(target_arch = "wasm32"))]
fn init_tls() {
    use std::sync::Once;
    static TLS_INIT: Once = Once::new();
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_server_config(cert_pem: &str, key_pem: &str) -> Result<Arc<rustls::ServerConfig>, String> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls_pemfile::{certs, pkcs8_private_keys};

    init_tls();

    let mut cert_reader = std::io::Cursor::new(cert_pem.as_bytes());
    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse TLS certificate: {e}"))?;
    if certs.is_empty() {
        return Err("TLS certificate PEM is empty".into());
    }

    let mut key_reader = std::io::Cursor::new(key_pem.as_bytes());
    let keys = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse TLS private key: {e}"))?;
    let key = keys
        .into_iter()
        .next()
        .ok_or_else(|| "TLS private key PEM is empty".to_string())?;
    let key = PrivateKeyDer::Pkcs8(key);

    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map(Arc::new)
        .map_err(|e| format!("Failed to build TLS server config: {e}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_listen(host: &str, port: u16, cert_pem: &str, key_pem: &str) -> Result<u64, String> {
    if port == 0 {
        return Err("tls_listen: invalid port".into());
    }
    let config = build_server_config(cert_pem, key_pem)?;
    let addr = format!("{host}:{port}");
    let listener = std::net::TcpListener::bind(&addr)
        .map_err(|e| format!("tls_listen failed: {e}"))?;
    let id = NEXT_TLS_LISTENER.fetch_add(1, Ordering::Relaxed);
    TLS_LISTENERS.with(|m| {
        m.borrow_mut().insert(
            id,
            TlsListenerState {
                listener,
                config,
            },
        );
    });
    Ok(id)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_reload_certs(listener_id: u64, cert_pem: &str, key_pem: &str) -> Result<(), String> {
    let config = build_server_config(cert_pem, key_pem)?;
    TLS_LISTENERS.with(|m| {
        let mut map = m.borrow_mut();
        let state = map
            .get_mut(&listener_id)
            .ok_or_else(|| format!("invalid tls listener id {listener_id}"))?;
        state.config = config;
        Ok(())
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_accept(listener_id: u64) -> Result<u64, String> {
    TLS_LISTENERS.with(|m| {
        let listeners = m.borrow();
        let state = listeners
            .get(&listener_id)
            .ok_or_else(|| format!("invalid tls listener id {listener_id}"))?;
        let (tcp, _) = state
            .listener
            .accept()
            .map_err(|e| format!("tls_accept failed: {e}"))?;
        let config = state.config.clone();
        drop(listeners);
        let conn = rustls::ServerConnection::new(config)
            .map_err(|e| format!("TLS server connection failed: {e}"))?;
        let id = NEXT_TLS_LISTENER.fetch_add(1, Ordering::Relaxed);
        TLS_SERVER_CONNS.with(|c| {
            c.borrow_mut().insert(id, TlsServerConn { conn, tcp });
        });
        Ok(id)
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_server_read(sock_id: u64, max: usize) -> Result<String, String> {
    let max = max.clamp(1, 65536);
    TLS_SERVER_CONNS.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid tls server socket id {sock_id}"))?;
        let mut buf = vec![0u8; max];
        let mut tls = rustls::Stream::new(&mut conn.conn, &mut conn.tcp);
        let n = tls
            .read(&mut buf)
            .map_err(|e| format!("tls_server_read failed: {e}"))?;
        buf.truncate(n);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_server_write(sock_id: u64, data: &str) -> Result<(), String> {
    TLS_SERVER_CONNS.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid tls server socket id {sock_id}"))?;
        let mut tls = rustls::Stream::new(&mut conn.conn, &mut conn.tcp);
        tls.write_all(data.as_bytes())
            .map_err(|e| format!("tls_server_write failed: {e}"))?;
        tls.flush()
            .map_err(|e| format!("tls_server_write flush failed: {e}"))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tls_server_close(sock_id: u64) -> Result<(), String> {
    if TLS_SERVER_CONNS.with(|m| m.borrow_mut().remove(&sock_id).is_some()) {
        return Ok(());
    }
    if TLS_LISTENERS.with(|m| m.borrow_mut().remove(&sock_id).is_some()) {
        return Ok(());
    }
    Err(format!("invalid tls server handle id {sock_id}"))
}

#[cfg(target_arch = "wasm32")]
pub fn tls_listen(_host: &str, _port: u16, _cert_pem: &str, _key_pem: &str) -> Result<u64, String> {
    Err("tls_listen() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tls_reload_certs(_listener_id: u64, _cert_pem: &str, _key_pem: &str) -> Result<(), String> {
    Err("tls_reload_certs() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tls_accept(_listener_id: u64) -> Result<u64, String> {
    Err("tls_accept() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tls_server_read(_sock_id: u64, _max: usize) -> Result<String, String> {
    Err("tls_server_read() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tls_server_write(_sock_id: u64, _data: &str) -> Result<(), String> {
    Err("tls_server_write() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tls_server_close(_sock_id: u64) -> Result<(), String> {
    Err("tls_server_close() is not available on wasm32".into())
}
