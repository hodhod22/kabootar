//! Raw UDP sockets for Deno parity (native host).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UDP: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static UDP_SOCKETS: RefCell<HashMap<u64, std::net::UdpSocket>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
fn next_id() -> u64 {
    NEXT_UDP.fetch_add(1, Ordering::Relaxed)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn udp_bind(host: &str, port: u16) -> Result<u64, String> {
    let bind_host = if host.is_empty() || host == "0.0.0.0" {
        "0.0.0.0"
    } else {
        host
    };
    let addr = if port == 0 {
        format!("{bind_host}:0")
    } else {
        format!("{bind_host}:{port}")
    };
    let socket = std::net::UdpSocket::bind(&addr)
        .map_err(|e| format!("udp_bind failed: {e}"))?;
    let _ = socket.set_read_timeout(Some(std::time::Duration::from_millis(200)));
    let id = next_id();
    UDP_SOCKETS.with(|m| m.borrow_mut().insert(id, socket));
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
pub fn udp_bind(_host: &str, _port: u16) -> Result<u64, String> {
    Err("udp_bind() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn udp_local_addr(sock_id: u64) -> Result<String, String> {
    UDP_SOCKETS.with(|m| {
        let map = m.borrow();
        let socket = map
            .get(&sock_id)
            .ok_or_else(|| format!("invalid udp socket id {sock_id}"))?;
        Ok(socket
            .local_addr()
            .map_err(|e| format!("udp_local_addr failed: {e}"))?
            .to_string())
    })
}

#[cfg(target_arch = "wasm32")]
pub fn udp_local_addr(_sock_id: u64) -> Result<String, String> {
    Err("udp_local_addr() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn udp_send(sock_id: u64, host: &str, port: u16, data: &str) -> Result<i64, String> {
    UDP_SOCKETS.with(|m| {
        let map = m.borrow();
        let socket = map
            .get(&sock_id)
            .ok_or_else(|| format!("invalid udp socket id {sock_id}"))?;
        let n = socket
            .send_to(data.as_bytes(), (host, port))
            .map_err(|e| format!("udp_send failed: {e}"))?;
        Ok(n as i64)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn udp_send(_sock_id: u64, _host: &str, _port: u16, _data: &str) -> Result<i64, String> {
    Err("udp_send() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn udp_recv(sock_id: u64, max: usize) -> Result<(String, String), String> {
    let max = max.clamp(1, 65536);
    UDP_SOCKETS.with(|m| {
        let map = m.borrow();
        let socket = map
            .get(&sock_id)
            .ok_or_else(|| format!("invalid udp socket id {sock_id}"))?;
        let mut buf = vec![0u8; max];
        match socket.recv_from(&mut buf) {
            Ok((n, peer)) => {
                buf.truncate(n);
                Ok((
                    String::from_utf8_lossy(&buf).into_owned(),
                    peer.to_string(),
                ))
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                Ok((String::new(), String::new()))
            }
            Err(e) => Err(format!("udp_recv failed: {e}")),
        }
    })
}

#[cfg(target_arch = "wasm32")]
pub fn udp_recv(_sock_id: u64, _max: usize) -> Result<(String, String), String> {
    Err("udp_recv() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn udp_close(sock_id: u64) -> Result<(), String> {
    UDP_SOCKETS.with(|m| {
        if m.borrow_mut().remove(&sock_id).is_some() {
            Ok(())
        } else {
            Err(format!("invalid udp socket id {sock_id}"))
        }
    })
}

#[cfg(target_arch = "wasm32")]
pub fn udp_close(_sock_id: u64) -> Result<(), String> {
    Err("udp_close() is not available on wasm32".into())
}
