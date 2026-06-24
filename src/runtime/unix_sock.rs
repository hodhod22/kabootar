//! Unix domain sockets for Deno parity.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UNIX: AtomicU64 = AtomicU64::new(1);

#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::cell::RefCell;
#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::collections::HashMap;

#[cfg(all(not(target_arch = "wasm32"), unix))]
thread_local! {
    static UNIX_STREAMS: RefCell<HashMap<u64, std::os::unix::net::UnixStream>> =
        RefCell::new(HashMap::new());
    static UNIX_LISTENERS: RefCell<HashMap<u64, std::os::unix::net::UnixListener>> =
        RefCell::new(HashMap::new());
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
fn next_id() -> u64 {
    NEXT_UNIX.fetch_add(1, Ordering::Relaxed)
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_connect(path: &str) -> Result<u64, String> {
    use std::os::unix::net::UnixStream;
    let stream = UnixStream::connect(path).map_err(|e| format!("unix_connect failed: {e}"))?;
    let id = next_id();
    UNIX_STREAMS.with(|m| m.borrow_mut().insert(id, stream));
    Ok(id)
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_listen(path: &str) -> Result<u64, String> {
    use std::os::unix::net::UnixListener;
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|e| format!("unix_listen failed: {e}"))?;
    let id = next_id();
    UNIX_LISTENERS.with(|m| m.borrow_mut().insert(id, listener));
    Ok(id)
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_accept(listener_id: u64) -> Result<u64, String> {
    UNIX_LISTENERS.with(|m| {
        let listeners = m.borrow();
        let listener = listeners
            .get(&listener_id)
            .ok_or_else(|| format!("invalid unix listener id {listener_id}"))?;
        let (stream, _) = listener
            .accept()
            .map_err(|e| format!("unix_accept failed: {e}"))?;
        let id = next_id();
        drop(listeners);
        UNIX_STREAMS.with(|s| s.borrow_mut().insert(id, stream));
        Ok(id)
    })
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_read(sock_id: u64, max: usize) -> Result<String, String> {
    use std::io::Read;
    let max = max.clamp(1, 65536);
    UNIX_STREAMS.with(|m| {
        let mut map = m.borrow_mut();
        let stream = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid unix socket id {sock_id}"))?;
        let mut buf = vec![0u8; max];
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("unix_read failed: {e}"))?;
        buf.truncate(n);
        Ok(String::from_utf8_lossy(&buf).into_owned())
    })
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_write(sock_id: u64, data: &str) -> Result<(), String> {
    use std::io::Write;
    UNIX_STREAMS.with(|m| {
        let mut map = m.borrow_mut();
        let stream = map
            .get_mut(&sock_id)
            .ok_or_else(|| format!("invalid unix socket id {sock_id}"))?;
        stream
            .write_all(data.as_bytes())
            .map_err(|e| format!("unix_write failed: {e}"))?;
        stream
            .flush()
            .map_err(|e| format!("unix_write flush failed: {e}"))?;
        Ok(())
    })
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub fn unix_close(handle_id: u64) -> Result<(), String> {
    if UNIX_STREAMS.with(|m| m.borrow_mut().remove(&handle_id).is_some()) {
        return Ok(());
    }
    if UNIX_LISTENERS.with(|m| m.borrow_mut().remove(&handle_id).is_some()) {
        return Ok(());
    }
    Err(format!("invalid unix handle id {handle_id}"))
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_connect(_path: &str) -> Result<u64, String> {
    Err("unix sockets are not available on this platform".into())
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_listen(_path: &str) -> Result<u64, String> {
    Err("unix sockets are not available on this platform".into())
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_accept(_listener_id: u64) -> Result<u64, String> {
    Err("unix sockets are not available on this platform".into())
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_read(_sock_id: u64, _max: usize) -> Result<String, String> {
    Err("unix sockets are not available on this platform".into())
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_write(_sock_id: u64, _data: &str) -> Result<(), String> {
    Err("unix sockets are not available on this platform".into())
}

#[cfg(any(target_arch = "wasm32", not(unix)))]
pub fn unix_close(_handle_id: u64) -> Result<(), String> {
    Err("unix sockets are not available on this platform".into())
}
