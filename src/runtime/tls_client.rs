//! Shared TLS client helpers for TCP, HTTP, and WebSocket.

use crate::runtime::tls_trust::TlsTrust;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Once};

static TLS_INIT: Once = Once::new();

pub fn init_tls() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub fn build_client_config(trust: &TlsTrust) -> Result<rustls::ClientConfig, String> {
    use rustls::pki_types::CertificateDer;

    init_tls();

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
            "No trusted TLS root certificates configured (use tls_add_ca or tls_ca_only)".into(),
        );
    }
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

pub fn handshake_client(
    conn: &mut rustls::ClientConnection,
    tcp: &mut TcpStream,
) -> Result<(), String> {
    while conn.is_handshaking() {
        conn.complete_io(tcp)
            .map_err(|e| format!("TLS handshake failed: {e}"))?;
    }
    Ok(())
}

pub fn upgrade_tcp_to_tls(
    mut tcp: TcpStream,
    hostname: &str,
    trust: &TlsTrust,
) -> Result<(rustls::ClientConnection, TcpStream), String> {
    let config = build_client_config(trust)?;
    let server_name = rustls::pki_types::ServerName::try_from(hostname)
        .map_err(|_| format!("Invalid TLS server name: {hostname}"))?
        .to_owned();
    let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name)
        .map_err(|e| format!("TLS handshake setup failed: {e}"))?;
    handshake_client(&mut conn, &mut tcp)?;
    if !trust.pins.is_empty() {
        let certs = conn.peer_certificates().ok_or_else(|| {
            format!("Certificate pin set for {hostname} but peer sent no certificate")
        })?;
        let leaf = certs.first().ok_or_else(|| {
            format!("Certificate pin set for {hostname} but peer sent no certificate")
        })?;
        crate::runtime::tls_trust::verify_peer_pin(hostname, leaf.as_ref(), trust)?;
    }
    Ok((conn, tcp))
}

pub fn tls_read(
    conn: &mut rustls::ClientConnection,
    tcp: &mut TcpStream,
    buf: &mut [u8],
) -> Result<usize, String> {
    let mut tls = rustls::Stream::new(conn, tcp);
    tls.read(buf).map_err(|e| format!("TLS read failed: {e}"))
}

pub fn tls_write_all(
    conn: &mut rustls::ClientConnection,
    tcp: &mut TcpStream,
    data: &[u8],
) -> Result<(), String> {
    let mut tls = rustls::Stream::new(conn, tcp);
    tls.write_all(data)
        .map_err(|e| format!("TLS write failed: {e}"))?;
    tls.flush().map_err(|e| format!("TLS flush failed: {e}"))
}
