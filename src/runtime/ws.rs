//! WebSocket TCP client (`ws://`) for Deno parity.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TCP_WS: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static WS_TCP: RefCell<HashMap<u64, WsTcpConn>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
struct ParsedWsUrl {
    host: String,
    port: u16,
    path: String,
    tls: bool,
}

#[cfg(not(target_arch = "wasm32"))]
enum WsTransport {
    Plain(std::net::TcpStream),
    Tls {
        conn: rustls::ClientConnection,
        tcp: std::net::TcpStream,
    },
}

#[cfg(not(target_arch = "wasm32"))]
struct WsTcpConn {
    transport: WsTransport,
    read_buf: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_ws_url(url: &str) -> Result<ParsedWsUrl, String> {
    let (rest, default_port, tls) = if let Some(r) = url.strip_prefix("wss://") {
        (r, 443u16, true)
    } else if let Some(r) = url.strip_prefix("ws://") {
        (r, 80u16, false)
    } else {
        return Err("WebSocket URL must start with ws:// or wss://".into());
    };
    let (authority, path) = match rest.split_once('/') {
        Some((auth, path)) => (auth, format!("/{path}")),
        None => (rest, "/".to_string()),
    };
    if authority.is_empty() {
        return Err("Invalid WebSocket URL: missing host".into());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p
                .parse()
                .map_err(|_| format!("Invalid port in WebSocket URL: {p}"))?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), default_port),
    };
    Ok(ParsedWsUrl {
        host,
        port,
        path,
        tls,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64_encode(&hasher.finalize())
}

#[cfg(not(target_arch = "wasm32"))]
fn random_key() -> String {
    let n = NEXT_TCP_WS.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut bytes = [0u8; 16];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (n.wrapping_mul(0x9E37).wrapping_add(t).wrapping_shr((i * 5) as u32) & 0xFF) as u8;
    }
    base64_encode(&bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_client_text_frame(text: &str) -> Vec<u8> {
    let payload = text.as_bytes();
    let mut frame = Vec::new();
    frame.push(0x81);
    let len = payload.len();
    if len < 126 {
        frame.push(0x80 | len as u8);
    } else if len <= 65535 {
        frame.push(0x80 | 126);
        frame.push((len >> 8) as u8);
        frame.push((len & 0xFF) as u8);
    } else {
        frame.push(0x80 | 127);
        let len64 = len as u64;
        for i in (0..8).rev() {
            frame.push((len64 >> (i * 8)) as u8);
        }
    }
    let mask: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
    frame.extend_from_slice(&mask);
    for (i, b) in payload.iter().enumerate() {
        frame.push(b ^ mask[i % 4]);
    }
    frame
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_pong_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![0x8A];
    let len = payload.len();
    if len <= 125 {
        frame.push(len as u8);
    } else if len <= 65535 {
        frame.push(126);
        frame.push((len >> 8) as u8);
        frame.push((len & 0xFF) as u8);
    } else {
        frame.push(127);
        let len64 = len as u64;
        for i in (0..8).rev() {
            frame.push((len64 >> (i * 8)) as u8);
        }
    }
    frame.extend_from_slice(payload);
    frame
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_server_text_frame(buf: &[u8]) -> Result<(Option<String>, usize), String> {
    if buf.len() < 2 {
        return Err("Incomplete WebSocket frame".into());
    }
    let opcode = buf[0] & 0x0F;
    if opcode == 0x8 {
        return Err("WebSocket connection closed by peer".into());
    }
    let masked = (buf[1] & 0x80) != 0;
    let mut len = (buf[1] & 0x7F) as usize;
    let mut idx = 2usize;
    if len == 126 {
        if buf.len() < 4 {
            return Err("Incomplete WebSocket frame".into());
        }
        len = ((buf[2] as usize) << 8) | (buf[3] as usize);
        idx = 4;
    } else if len == 127 {
        if buf.len() < 10 {
            return Err("Incomplete WebSocket frame".into());
        }
        len = 0;
        for &b in &buf[2..10] {
            len = (len << 8) | b as usize;
        }
        idx = 10;
    }
    if masked {
        if buf.len() < idx + 4 + len {
            return Err("Incomplete WebSocket frame".into());
        }
        let mask = &buf[idx..idx + 4];
        idx += 4;
        let consumed = idx + len;
        if opcode == 0x9 {
            return Ok((None, consumed));
        }
        if opcode != 0x1 && opcode != 0x0 {
            return Err(format!("Unsupported WebSocket opcode {opcode}"));
        }
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            out.push(buf[idx + i] ^ mask[i % 4]);
        }
        let text = String::from_utf8(out)
            .map_err(|e| format!("Invalid UTF-8 in WebSocket frame: {e}"))?;
        return Ok((Some(text), consumed));
    }
    if buf.len() < idx + len {
        return Err("Incomplete WebSocket frame".into());
    }
    let consumed = idx + len;
    if opcode == 0x9 {
        return Ok((None, consumed));
    }
    if opcode != 0x1 && opcode != 0x0 {
        return Err(format!("Unsupported WebSocket opcode {opcode}"));
    }
    let payload = &buf[idx..idx + len];
    let text = String::from_utf8(payload.to_vec())
        .map_err(|e| format!("Invalid UTF-8 in WebSocket frame: {e}"))?;
    Ok((Some(text), consumed))
}

#[cfg(not(target_arch = "wasm32"))]
impl WsTransport {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        match self {
            WsTransport::Plain(stream) => stream
                .read(buf)
                .map_err(|e| format!("WebSocket read failed: {e}")),
            WsTransport::Tls { conn, tcp } => {
                let mut tls = rustls::Stream::new(conn, tcp);
                tls.read(buf)
                    .map_err(|e| format!("WebSocket TLS read failed: {e}"))
            }
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), String> {
        match self {
            WsTransport::Plain(stream) => stream
                .write_all(buf)
                .map_err(|e| format!("WebSocket send failed: {e}")),
            WsTransport::Tls { conn, tcp } => {
                let mut tls = rustls::Stream::new(conn, tcp);
                tls.write_all(buf)
                    .map_err(|e| format!("WebSocket TLS send failed: {e}"))
            }
        }
    }

    fn flush(&mut self) -> Result<(), String> {
        match self {
            WsTransport::Plain(stream) => stream
                .flush()
                .map_err(|e| format!("WebSocket flush failed: {e}")),
            WsTransport::Tls { conn, tcp } => {
                let mut tls = rustls::Stream::new(conn, tcp);
                tls.flush()
                    .map_err(|e| format!("WebSocket TLS flush failed: {e}"))
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_tls_config(trust: &crate::runtime::tls_trust::TlsTrust) -> Result<rustls::ClientConfig, String> {
    use rustls::pki_types::CertificateDer;
    use std::sync::{Arc, Once};

    static TLS_INIT: Once = Once::new();
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

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
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

#[cfg(not(target_arch = "wasm32"))]
fn connect_transport(
    parsed: &ParsedWsUrl,
    trust: &crate::runtime::tls_trust::TlsTrust,
) -> Result<WsTransport, String> {
    use std::net::TcpStream;
    use std::sync::Arc;

    let tcp = TcpStream::connect((parsed.host.as_str(), parsed.port))
        .map_err(|e| format!("WebSocket TCP connect failed: {e}"))?;
    let _ = tcp.set_read_timeout(None);
    let _ = tcp.set_write_timeout(None);

    if !parsed.tls {
        let mut stream = tcp;
        let key = random_key();
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            parsed.path, parsed.host, key
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("WebSocket handshake write failed: {e}"))?;
        stream
            .flush()
            .map_err(|e| format!("WebSocket handshake flush failed: {e}"))?;
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        loop {
            let n = stream
                .read(&mut tmp)
                .map_err(|e| format!("WebSocket handshake read failed: {e}"))?;
            if n == 0 {
                return Err("WebSocket handshake: connection closed".into());
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
            if buf.len() > 8192 {
                return Err("WebSocket handshake response too large".into());
            }
        }
        let head = String::from_utf8_lossy(&buf);
        let status = head
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .ok_or("Invalid WebSocket handshake response")?;
        if status != "101" {
            return Err(format!("WebSocket upgrade failed with status {status}"));
        }
        let accept = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                if k.trim().eq_ignore_ascii_case("sec-websocket-accept") {
                    Some(v.trim().to_string())
                } else {
                    None
                }
            })
            .ok_or("WebSocket handshake missing Sec-WebSocket-Accept")?;
        let expected = ws_accept_key(&key);
        if accept != expected {
            return Err("WebSocket Sec-WebSocket-Accept mismatch".into());
        }
        return Ok(WsTransport::Plain(stream));
    }

    let config = build_tls_config(trust)?;
    let server_name = rustls::pki_types::ServerName::try_from(parsed.host.as_str())
        .map_err(|_| {
            format!(
                "Invalid TLS server name: {} (use a hostname for wss://)",
                parsed.host
            )
        })?
        .to_owned();
    let mut tcp = tcp;
    let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name)
        .map_err(|e| format!("TLS handshake setup failed: {e}"))?;
    handshake_tls(&mut conn, &mut tcp, parsed)?;
    if !trust.pins.is_empty() {
        let certs = conn
            .peer_certificates()
            .ok_or_else(|| format!("Certificate pin set for {} but peer sent no certificate", parsed.host))?;
        let leaf = certs
            .first()
            .ok_or_else(|| format!("Certificate pin set for {} but peer sent no certificate", parsed.host))?;
        crate::runtime::tls_trust::verify_peer_pin(&parsed.host, leaf.as_ref(), trust)?;
    }
    Ok(WsTransport::Tls { conn, tcp })
}

#[cfg(not(target_arch = "wasm32"))]
fn handshake_tls(
    conn: &mut rustls::ClientConnection,
    tcp: &mut std::net::TcpStream,
    parsed: &ParsedWsUrl,
) -> Result<(), String> {
    let key = random_key();
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
        parsed.path, parsed.host, key
    );
    {
        let mut tls = rustls::Stream::new(conn, tcp);
        tls.write_all(req.as_bytes())
            .map_err(|e| format!("WebSocket handshake write failed: {e}"))?;
        tls.flush()
            .map_err(|e| format!("WebSocket handshake flush failed: {e}"))?;
    }

    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = {
            let mut tls = rustls::Stream::new(conn, tcp);
            tls.read(&mut tmp)
                .map_err(|e| format!("WebSocket handshake read failed: {e}"))?
        };
        if n == 0 {
            return Err("WebSocket handshake: connection closed".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            return Err("WebSocket handshake response too large".into());
        }
    }

    let head = String::from_utf8_lossy(&buf);
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .ok_or("Invalid WebSocket handshake response")?;
    if status != "101" {
        return Err(format!("WebSocket upgrade failed with status {status}"));
    }
    let accept = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("sec-websocket-accept") {
                Some(v.trim().to_string())
            } else {
                None
            }
        })
        .ok_or("WebSocket handshake missing Sec-WebSocket-Accept")?;
    let expected = ws_accept_key(&key);
    if accept != expected {
        return Err("WebSocket Sec-WebSocket-Accept mismatch".into());
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
impl WsTcpConn {
    fn read_text(&mut self) -> Result<Option<String>, String> {
        loop {
            if let Ok((maybe_text, consumed)) = decode_server_text_frame(&self.read_buf) {
                self.read_buf.drain(..consumed);
                if let Some(text) = maybe_text {
                    return Ok(Some(text));
                }
                let pong = encode_pong_frame(&[]);
                self.transport.write_all(&pong)?;
                self.transport.flush()?;
                continue;
            }
            let mut tmp = [0u8; 4096];
            let n = self.transport.read(&mut tmp)?;
            if n == 0 {
                return Ok(None);
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }

    fn send_text(&mut self, text: &str) -> Result<(), String> {
        let frame = encode_client_text_frame(text);
        self.transport.write_all(&frame)?;
        self.transport.flush()?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_tcp_connect(url: &str) -> Result<u64, String> {
    ws_tcp_connect_with_trust(url, &crate::runtime::tls_trust::TlsTrust::default())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_tcp_connect_with_trust(
    url: &str,
    trust: &crate::runtime::tls_trust::TlsTrust,
) -> Result<u64, String> {
    let parsed = parse_ws_url(url)?;
    let transport = connect_transport(&parsed, trust)?;
    let id = NEXT_TCP_WS.fetch_add(1, Ordering::Relaxed);
    WS_TCP.with(|m| {
        m.borrow_mut().insert(
            id,
            WsTcpConn {
                transport,
                read_buf: Vec::new(),
            },
        );
    });
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
pub fn ws_tcp_connect(_url: &str) -> Result<u64, String> {
    Err("ws_connect() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_tcp_send(id: u64, text: &str) -> Result<(), String> {
    WS_TCP.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid tcp websocket id {id}"))?;
        conn.send_text(text)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn ws_tcp_send(_id: u64, _text: &str) -> Result<(), String> {
    Err("ws_connect() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_tcp_recv(id: u64) -> Result<Option<String>, String> {
    WS_TCP.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid tcp websocket id {id}"))?;
        conn.read_text()
    })
}

#[cfg(target_arch = "wasm32")]
pub fn ws_tcp_recv(_id: u64) -> Result<Option<String>, String> {
    Err("ws_connect() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ws_tcp_close(id: u64) {
    WS_TCP.with(|m| {
        m.borrow_mut().remove(&id);
    });
}

#[cfg(target_arch = "wasm32")]
pub fn ws_tcp_close(_id: u64) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn is_tcp_ws(id: u64) -> bool {
    WS_TCP.with(|m| m.borrow().contains_key(&id))
}

#[cfg(target_arch = "wasm32")]
pub fn is_tcp_ws(_id: u64) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_echo_server_for_test() -> u16 {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).expect("read handshake");
        let req = String::from_utf8_lossy(&buf[..n]);
        let key = req
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                    Some(v.trim().to_string())
                } else {
                    None
                }
            })
            .expect("key");
        let accept = ws_accept_key(&key);
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        stream.write_all(resp.as_bytes()).expect("write 101");
        let n = stream.read(&mut buf).expect("read frame");
        if let Ok((Some(text), _)) = decode_server_text_frame(&buf[..n]) {
            let payload = text.as_bytes();
            let mut frame = vec![0x81, payload.len() as u8];
            frame.extend_from_slice(payload);
            stream.write_all(&frame).expect("echo");
        }
    });
    port
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn spawn_echo_server() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).expect("read handshake");
            let req = String::from_utf8_lossy(&buf[..n]);
            let key = req
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
                .expect("key");
            let accept = ws_accept_key(&key);
            let resp = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
            );
            stream.write_all(resp.as_bytes()).expect("write 101");
            let n = stream.read(&mut buf).expect("read frame");
            if let Ok((Some(text), consumed)) = decode_server_text_frame(&buf[..n]) {
                let frame = encode_server_text_frame(&text);
                stream.write_all(&frame).expect("echo");
                let _ = consumed;
            }
        });
        port
    }

    fn encode_server_text_frame(text: &str) -> Vec<u8> {
        let payload = text.as_bytes();
        let mut frame = vec![0x81];
        let len = payload.len();
        if len < 126 {
            frame.push(len as u8);
        } else {
            frame.push(126);
            frame.push((len >> 8) as u8);
            frame.push((len & 0xFF) as u8);
        }
        frame.extend_from_slice(payload);
        frame
    }

    #[test]
    fn ws_tcp_roundtrip() {
        let port = spawn_echo_server();
        thread::sleep(std::time::Duration::from_millis(30));
        let id = ws_tcp_connect(&format!("ws://127.0.0.1:{port}/")).expect("connect");
        ws_tcp_send(id, "ping").expect("send");
        let msg = ws_tcp_recv(id).expect("recv").expect("message");
        assert_eq!(msg, "ping");
        ws_tcp_close(id);
    }

    #[test]
    fn ws_tls_roundtrip() {
        use rcgen::generate_simple_self_signed;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls::server::WebPkiClientVerifier;
        use std::sync::Arc;

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
                let Ok(mut server_conn) = rustls::ServerConnection::new(server_cfg.clone()) else {
                    return;
                };
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
                            if let Ok((Some(text), _)) = decode_server_text_frame(&buf) {
                                let payload = text.as_bytes();
                                let mut frame = vec![0x81, payload.len() as u8];
                                frame.extend_from_slice(payload);
                                let _ = tls.write_all(&frame);
                            }
                        }
                    }
                }
            }
        });

        let mut trust = crate::runtime::tls_trust::TlsTrust::default();
        trust.set_ca_only_pem(&ca_pem).expect("ca");
        thread::sleep(std::time::Duration::from_millis(40));
        let id =
            ws_tcp_connect_with_trust(&format!("wss://localhost:{port}/"), &trust).expect("connect");
        ws_tcp_send(id, "tls").expect("send");
        let msg = ws_tcp_recv(id).expect("recv").expect("message");
        assert_eq!(msg, "tls");
        ws_tcp_close(id);
    }
}
