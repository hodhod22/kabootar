//! Kabootar OS network driver — TCP/UDP + host bridge on native.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct NetInterface {
    pub name: String,
    pub mac: String,
    pub ipv4: String,
    pub up: bool,
    pub mtu: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Connecting,
    Listening,
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Tcp,
    Udp,
    Simulated,
}

#[derive(Debug)]
enum SockBackend {
    #[cfg(not(target_arch = "wasm32"))]
    TcpStream(std::net::TcpStream),
    #[cfg(not(target_arch = "wasm32"))]
    TcpListener(std::net::TcpListener),
    #[cfg(not(target_arch = "wasm32"))]
    Udp(std::net::UdpSocket),
    Simulated { rx: Vec<u8> },
}

#[derive(Debug)]
struct SocketEntry {
    state: SocketState,
    sock_type: SocketType,
    peer: String,
    backend: SockBackend,
}

impl SocketEntry {
    fn simulated_tcp(peer: String, greeting: &[u8]) -> Self {
        Self {
            state: SocketState::Open,
            sock_type: SocketType::Simulated,
            peer,
            backend: SockBackend::Simulated {
                rx: greeting.to_vec(),
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tcp(peer: String, stream: std::net::TcpStream) -> Self {
        Self {
            state: SocketState::Open,
            sock_type: SocketType::Tcp,
            peer,
            backend: SockBackend::TcpStream(stream),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn listener(addr: String, listener: std::net::TcpListener) -> Self {
        Self {
            state: SocketState::Listening,
            sock_type: SocketType::Tcp,
            peer: addr,
            backend: SockBackend::TcpListener(listener),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn udp(addr: String, socket: std::net::UdpSocket) -> Self {
        Self {
            state: SocketState::Open,
            sock_type: SocketType::Udp,
            peer: addr,
            backend: SockBackend::Udp(socket),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PollEvent {
    pub socket: u64,
    pub kind: String,
}

pub struct NetDriver {
    interfaces: Vec<NetInterface>,
    sockets: HashMap<u64, SocketEntry>,
    pending_accepts: HashMap<u64, Vec<(String, SockBackend)>>,
    next_sock: AtomicU64,
    bytes_sent: u64,
    bytes_recv: u64,
}

impl Default for NetDriver {
    fn default() -> Self {
        Self {
            interfaces: vec![NetInterface {
                name: "eth0".into(),
                mac: "02:4b:ab:00:01:00".into(),
                ipv4: "10.0.2.15".into(),
                up: true,
                mtu: 1500,
            }],
            sockets: HashMap::new(),
            pending_accepts: HashMap::new(),
            next_sock: AtomicU64::new(1),
            bytes_sent: 0,
            bytes_recv: 0,
        }
    }
}

impl NetDriver {
    pub fn interfaces(&self) -> &[NetInterface] {
        &self.interfaces
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        (self.bytes_sent, self.bytes_recv, self.sockets.len())
    }

    fn new_sock_id(&self) -> u64 {
        self.next_sock.fetch_add(1, Ordering::SeqCst)
    }

    pub fn connect(&mut self, host: &str, port: u16) -> Result<u64, String> {
        if host.is_empty() {
            return Err("net connect: host required".into());
        }
        if port == 0 && host != "loopback" {
            return Err("net connect: invalid port".into());
        }
        let id = self.new_sock_id();
        let peer = format!("{host}:{port}");

        if host == "loopback" || host == "kabootar-loopback" {
            self.sockets.insert(
                id,
                SocketEntry::simulated_tcp(peer, b"Kabootar loopback socket ready\n"),
            );
            return Ok(id);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::net::TcpStream;
            use std::time::Duration;

            let addr = format!("{host}:{port}");
            match TcpStream::connect(&addr) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(Duration::from_millis(50))).ok();
                    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
                    self.sockets.insert(id, SocketEntry::tcp(peer, stream));
                    return Ok(id);
                }
                Err(e) if host == "127.0.0.1" || host == "localhost" => {
                    self.sockets.insert(
                        id,
                        SocketEntry::simulated_tcp(
                            peer,
                            format!("Kabootar local socket (fallback: {e})\n").as_bytes(),
                        ),
                    );
                    return Ok(id);
                }
                Err(e) => return Err(format!("net connect: {e}")),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = (&host, port);
            self.sockets.insert(
                id,
                SocketEntry::simulated_tcp(peer, b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK"),
            );
            Ok(id)
        }
    }

    pub fn listen(&mut self, host: &str, port: u16) -> Result<u64, String> {
        if port == 0 && host != "loopback" && host != "kabootar-loopback" {
            return Err("net listen: invalid port".into());
        }
        let id = self.new_sock_id();

        if host == "loopback" || host == "kabootar-loopback" {
            self.sockets.insert(
                id,
                SocketEntry {
                    state: SocketState::Listening,
                    sock_type: SocketType::Simulated,
                    peer: format!("loopback:{port}"),
                    backend: SockBackend::Simulated { rx: Vec::new() },
                },
            );
            return Ok(id);
        }

        let bind_host = if host.is_empty() || host == "0.0.0.0" {
            "0.0.0.0"
        } else {
            host
        };
        let addr = format!("{bind_host}:{port}");

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::net::TcpListener;
            use std::time::Duration;

            let listener = TcpListener::bind(&addr).map_err(|e| format!("net listen: {e}"))?;
            listener
                .set_nonblocking(true)
                .map_err(|e| format!("net listen nonblocking: {e}"))?;
            self.sockets
                .insert(id, SocketEntry::listener(addr.clone(), listener));
            return Ok(id);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = bind_host;
            self.sockets.insert(
                id,
                SocketEntry {
                    state: SocketState::Listening,
                    sock_type: SocketType::Simulated,
                    peer: addr,
                    backend: SockBackend::Simulated { rx: Vec::new() },
                },
            );
            Ok(id)
        }
    }

    pub fn accept(&mut self, sock: u64) -> Result<u64, String> {
        if let Some(q) = self.pending_accepts.get_mut(&sock) {
            if let Some((peer, backend)) = q.pop() {
                let id = self.new_sock_id();
                self.sockets.insert(
                    id,
                    SocketEntry {
                        state: SocketState::Open,
                        sock_type: SocketType::Tcp,
                        peer,
                        backend,
                    },
                );
                return Ok(id);
            }
        }

        let entry = self
            .sockets
            .get(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;
        if entry.state != SocketState::Listening {
            return Err("socket not listening".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::Duration;

            let SockBackend::TcpListener(listener) = &entry.backend else {
                return Err("not a tcp listener".into());
            };
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    stream.set_read_timeout(Some(Duration::from_millis(50))).ok();
                    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
                    let id = self.new_sock_id();
                    let peer = peer_addr.to_string();
                    self.sockets
                        .insert(id, SocketEntry::tcp(peer, stream));
                    return Ok(id);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Err("accept: would block".into());
                }
                Err(e) => return Err(format!("accept: {e}")),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let id = self.new_sock_id();
            self.sockets.insert(
                id,
                SocketEntry::simulated_tcp(
                    format!("client-{id}"),
                    b"Kabootar simulated accept\n",
                ),
            );
            Ok(id)
        }
    }

    pub fn udp_bind(&mut self, host: &str, port: u16) -> Result<u64, String> {
        if port == 0 {
            return Err("udp bind: invalid port".into());
        }
        let bind_host = if host.is_empty() || host == "0.0.0.0" {
            "0.0.0.0"
        } else {
            host
        };
        let id = self.new_sock_id();
        let addr = format!("{bind_host}:{port}");

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::net::UdpSocket;
            use std::time::Duration;

            let socket = UdpSocket::bind(&addr).map_err(|e| format!("udp bind: {e}"))?;
            socket
                .set_read_timeout(Some(Duration::from_millis(50)))
                .ok();
            self.sockets.insert(id, SocketEntry::udp(addr, socket));
            return Ok(id);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = bind_host;
            self.sockets.insert(
                id,
                SocketEntry {
                    state: SocketState::Open,
                    sock_type: SocketType::Udp,
                    peer: addr,
                    backend: SockBackend::Simulated { rx: Vec::new() },
                },
            );
            Ok(id)
        }
    }

    pub fn udp_send(&mut self, sock: u64, host: &str, port: u16, data: &[u8]) -> Result<usize, String> {
        let entry = self
            .sockets
            .get_mut(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;
        if entry.sock_type != SocketType::Udp && entry.sock_type != SocketType::Simulated {
            return Err("not a udp socket".into());
        }
        if data.is_empty() {
            return Err("udp send: empty data".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::net::UdpSocket;
            if let SockBackend::Udp(socket) = &entry.backend {
                let n = socket
                    .send_to(data, format!("{host}:{port}"))
                    .map_err(|e| format!("udp send: {e}"))?;
                self.bytes_sent += n as u64;
                return Ok(n);
            }
        }

        let n = data.len();
        self.bytes_sent += n as u64;
        Ok(n)
    }

    pub fn udp_recv(&mut self, sock: u64, max: usize) -> Result<(Vec<u8>, String), String> {
        let max = max.clamp(1, 65536);
        let entry = self
            .sockets
            .get_mut(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let SockBackend::Udp(socket) = &entry.backend {
                let mut buf = vec![0u8; max];
                match socket.recv_from(&mut buf) {
                    Ok((n, peer)) => {
                        buf.truncate(n);
                        self.bytes_recv += n as u64;
                        return Ok((buf, peer.to_string()));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        return Ok((Vec::new(), String::new()));
                    }
                    Err(e) => return Err(format!("udp recv: {e}")),
                }
            }
        }

        if let SockBackend::Simulated { rx } = &mut entry.backend {
            let n = rx.len().min(max);
            let out = rx.drain(..n).collect::<Vec<_>>();
            self.bytes_recv += out.len() as u64;
            return Ok((out, "simulated:0".into()));
        }

        Err("not a udp socket".into())
    }

    pub fn poll(&mut self, sockets: &[u64]) -> Vec<PollEvent> {
        let mut out = Vec::new();
        for &sid in sockets {
            let Some(entry) = self.sockets.get(&sid) else {
                continue;
            };
            match entry.state {
                SocketState::Listening => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(entry) = self.sockets.get(&sid) {
                            if let SockBackend::TcpListener(listener) = &entry.backend {
                                if let Ok((stream, peer_addr)) = listener.accept() {
                                    use std::time::Duration;
                                    stream.set_read_timeout(Some(Duration::from_millis(50))).ok();
                                    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
                                    self.pending_accepts
                                        .entry(sid)
                                        .or_default()
                                        .push((
                                            peer_addr.to_string(),
                                            SockBackend::TcpStream(stream),
                                        ));
                                    out.push(PollEvent {
                                        socket: sid,
                                        kind: "accept".into(),
                                    });
                                }
                            }
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    if entry.sock_type == SocketType::Simulated {
                        out.push(PollEvent {
                            socket: sid,
                            kind: "accept".into(),
                        });
                    }
                }
                SocketState::Open => {
                    match &entry.backend {
                        #[cfg(not(target_arch = "wasm32"))]
                        SockBackend::TcpStream(stream) => {
                            use std::io::Read;
                            let mut buf = [0u8; 1];
                            match stream.peek(&mut buf) {
                                Ok(0) => {}
                                Ok(_) => out.push(PollEvent {
                                    socket: sid,
                                    kind: "read".into(),
                                }),
                                Err(e)
                                    if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                                Err(_) => {}
                            }
                        }
                        SockBackend::Simulated { rx } if !rx.is_empty() => {
                            out.push(PollEvent {
                                socket: sid,
                                kind: "read".into(),
                            });
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        SockBackend::Udp(socket) => {
                            let mut buf = [0u8; 1];
                            match socket.peek_from(&mut buf) {
                                Ok(_) => out.push(PollEvent {
                                    socket: sid,
                                    kind: "read".into(),
                                }),
                                Err(e)
                                    if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                                Err(_) => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn send(&mut self, sock: u64, data: &[u8]) -> Result<usize, String> {
        let entry = self
            .sockets
            .get_mut(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;
        if entry.state != SocketState::Open {
            return Err("socket closed".into());
        }
        if data.is_empty() {
            return Err("net send: empty data".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Write;
            if let SockBackend::TcpStream(stream) = &mut entry.backend {
                let n = stream
                    .write(data)
                    .map_err(|e| format!("net send: {e}"))?;
                self.bytes_sent += n as u64;
                return Ok(n);
            }
        }

        let n = data.len();
        self.bytes_sent += n as u64;
        Ok(n)
    }

    pub fn recv(&mut self, sock: u64, max: usize) -> Result<Vec<u8>, String> {
        let max = max.clamp(1, 65536);
        let entry = self
            .sockets
            .get_mut(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;
        if entry.state != SocketState::Open {
            return Err("socket closed".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Read;
            if let SockBackend::TcpStream(stream) = &mut entry.backend {
                let mut buf = vec![0u8; max];
                match stream.read(&mut buf) {
                    Ok(0) => Ok(Vec::new()),
                    Ok(n) => {
                        buf.truncate(n);
                        self.bytes_recv += n as u64;
                        Ok(buf)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        Ok(Vec::new())
                    }
                    Err(e) => Err(format!("net recv: {e}")),
                }
            } else if let SockBackend::Simulated { rx } = &mut entry.backend {
                let n = rx.len().min(max);
                let out = rx.drain(..n).collect::<Vec<_>>();
                self.bytes_recv += out.len() as u64;
                Ok(out)
            } else {
                Err("not a connected tcp socket".into())
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let SockBackend::Simulated { rx } = &mut entry.backend {
                let n = rx.len().min(max);
                let out = rx.drain(..n).collect::<Vec<_>>();
                self.bytes_recv += out.len() as u64;
                Ok(out)
            } else {
                Ok(Vec::new())
            }
        }
    }

    pub fn close(&mut self, sock: u64) -> Result<(), String> {
        let entry = self
            .sockets
            .get_mut(&sock)
            .ok_or_else(|| format!("invalid socket: {sock}"))?;
        entry.state = SocketState::Closed;
        #[cfg(not(target_arch = "wasm32"))]
        match &entry.backend {
            SockBackend::TcpStream(stream) => {
                use std::net::Shutdown;
                let _ = stream.shutdown(Shutdown::Both);
            }
            SockBackend::TcpListener(_) | SockBackend::Udp(_) => {}
            SockBackend::Simulated { .. } => {}
        }
        Ok(())
    }
}
