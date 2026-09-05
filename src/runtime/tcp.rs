//! Raw TCP sockets for Deno `listen` / `connect` / `startTls` parity (native host).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TCP: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static TCP_CONNS: RefCell<HashMap<u64, TcpConn>> = RefCell::new(HashMap::new());
    static TCP_LISTENERS: RefCell<HashMap<u64, std::net::TcpListener>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
enum TcpTransport {
    Plain(std::net::TcpStream),
    Tls {
        conn: rustls::ClientConnection,
        tcp: std::net::TcpStream,
    },
}

#[cfg(not(target_arch = "wasm32"))]
struct TcpConn {
    transport: TcpTransport,
}

#[cfg(not(target_arch = "wasm32"))]
fn next_id() -> u64 {
    NEXT_TCP.fetch_add(1, Ordering::Relaxed)
}

#[cfg(not(target_arch = "wasm32"))]
fn insert_stream(stream: std::net::TcpStream) -> u64 {
    let id = next_id();
    TCP_CONNS.with(|m| {
        m.borrow_mut().insert(
            id,
            TcpConn {
                transport: TcpTransport::Plain(stream),
            },
        );
    });
    id
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_connect(host: &str, port: u16) -> Result<u64, String> {
    if port == 0 {
        return Err("tcp_connect: invalid port".into());
    }
    let stream = std::net::TcpStream::connect((host, port))
        .map_err(|e| format!("tcp_connect failed: {e}"))?;
    let _ = stream.set_read_timeout(None);
    let _ = stream.set_write_timeout(None);
    Ok(insert_stream(stream))
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_connect(_host: &str, _port: u16) -> Result<u64, String> {
    Err("tcp_connect() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_listen(host: &str, port: u16) -> Result<u64, String> {
    if port == 0 {
        return Err("tcp_listen: invalid port".into());
    }
    let addr = format!("{host}:{port}");
    let listener = std::net::TcpListener::bind(&addr)
        .map_err(|e| format!("tcp_listen failed: {e}"))?;
    let _ = listener.set_nonblocking(false);
    let id = next_id();
    TCP_LISTENERS.with(|m| m.borrow_mut().insert(id, listener));
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_listen(_host: &str, _port: u16) -> Result<u64, String> {
    Err("tcp_listen() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_accept(listener_id: u64) -> Result<u64, String> {
    TCP_LISTENERS.with(|m| {
        let listeners = m.borrow();
        let listener = listeners
            .get(&listener_id)
            .ok_or_else(|| format!("invalid tcp listener id {listener_id}"))?;
        let (stream, _peer) = listener
            .accept()
            .map_err(|e| format!("tcp_accept failed: {e}"))?;
        drop(listeners);
        Ok(insert_stream(stream))
    })
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_accept(_listener_id: u64) -> Result<u64, String> {
    Err("tcp_accept() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_start_tls(
    sock_id: u64,
    hostname: &str,
    trust: &crate::runtime::tls_trust::TlsTrust,
) -> Result<u64, String> {
    TCP_CONNS.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .remove(&sock_id)
            .ok_or_else(|| format!("invalid tcp socket id {sock_id}"))?;
        match conn.transport {
            TcpTransport::Plain(tcp) => {
                let (tls_conn, tcp) =
                    crate::runtime::tls_client::upgrade_tcp_to_tls(tcp, hostname, trust)?;
                map.insert(
                    sock_id,
                    TcpConn {
                        transport: TcpTransport::Tls {
                            conn: tls_conn,
                            tcp,
                        },
                    },
                );
                Ok(sock_id)
            }
            TcpTransport::Tls { .. } => Err("tcp_start_tls: socket already uses TLS".into()),
        }
    })
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_start_tls(
    _sock_id: u64,
    _hostname: &str,
    _trust: &crate::runtime::tls_trust::TlsTrust,
) -> Result<u64, String> {
    Err("tcp_start_tls() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_read_bytes(sock_id: u64, max: usize) -> Result<Vec<u8>, String> {
    let max = max.clamp(1, 65536);
    TCP_CONNS.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid tcp socket id {sock_id}"))?;
        let mut buf = vec![0u8; max];
        let n = match &mut conn.transport {
            TcpTransport::Plain(stream) => stream
                .read(&mut buf)
                .map_err(|e| format!("tcp_read failed: {e}"))?,
            TcpTransport::Tls { conn: tls, tcp } => {
                crate::runtime::tls_client::tls_read(tls, tcp, &mut buf)?
            }
        };
        buf.truncate(n);
        Ok(buf)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_read_bytes(_sock_id: u64, _max: usize) -> Result<Vec<u8>, String> {
    Err("tcp_read() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_read(sock_id: u64, max: usize) -> Result<String, String> {
    Ok(String::from_utf8_lossy(&tcp_read_bytes(sock_id, max)?).into_owned())
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_read(_sock_id: u64, _max: usize) -> Result<String, String> {
    Err("tcp_read() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_write(sock_id: u64, data: &str) -> Result<(), String> {
    tcp_write_bytes(sock_id, data.as_bytes())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_write_bytes(sock_id: u64, data: &[u8]) -> Result<(), String> {
    TCP_CONNS.with(|m| {
        let mut map = m.borrow_mut();
        let conn = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid tcp socket id {sock_id}"))?;
        match &mut conn.transport {
            TcpTransport::Plain(stream) => {
                stream
                    .write_all(data)
                    .map_err(|e| format!("tcp_write failed: {e}"))?;
                stream
                    .flush()
                    .map_err(|e| format!("tcp_write flush failed: {e}"))?;
            }
            TcpTransport::Tls { conn: tls, tcp } => {
                crate::runtime::tls_client::tls_write_all(tls, tcp, data)?;
            }
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_write(_sock_id: u64, _data: &str) -> Result<(), String> {
    Err("tcp_write() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_write_bytes(_sock_id: u64, _data: &[u8]) -> Result<(), String> {
    Err("tcp_write_bytes() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn tcp_close(sock_id: u64) -> Result<(), String> {
    let removed_stream = TCP_CONNS.with(|m| m.borrow_mut().remove(&sock_id).is_some());
    if removed_stream {
        return Ok(());
    }
    let removed_listener = TCP_LISTENERS.with(|m| m.borrow_mut().remove(&sock_id).is_some());
    if removed_listener {
        return Ok(());
    }
    Err(format!("invalid tcp handle id {sock_id}"))
}

#[cfg(target_arch = "wasm32")]
pub fn tcp_close(_sock_id: u64) -> Result<(), String> {
    Err("tcp_close() is not available on wasm32".into())
}
