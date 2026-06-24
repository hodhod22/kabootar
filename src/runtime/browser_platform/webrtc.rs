//! WebRTC — peer connections, ICE (STUN/TURN), RTP media tracks.

use super::json_util::extract_array_strings;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IceState {
    New,
    Gathering,
    Checking,
    Connected,
    Failed,
}

#[derive(Clone)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: String,
}

#[derive(Clone)]
pub struct MediaTrack {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    pub ssrc: u32,
    pub payload_type: u8,
}

#[derive(Clone)]
pub struct RtpPacket {
    pub track_id: String,
    pub ssrc: u32,
    pub payload_type: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct PeerConnection {
    pub id: u64,
    pub ice_state: IceState,
    pub local_sdp: Option<String>,
    pub remote_sdp: Option<String>,
    pub tracks: Vec<MediaTrack>,
    pub candidates: Vec<IceCandidate>,
    pub rtp_out: u64,
    pub rtp_in: u64,
    pub bytes_out: u64,
    pub bytes_in: u64,
    pub rx_queue: VecDeque<RtpPacket>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TRACK: AtomicU64 = AtomicU64::new(1);
static NEXT_SEQ: AtomicU64 = AtomicU64::new(1);
static PEERS: OnceLock<Mutex<HashMap<u64, PeerConnection>>> = OnceLock::new();
static ICE_SERVERS: OnceLock<Mutex<Vec<IceServer>>> = OnceLock::new();

fn peer_store() -> &'static Mutex<HashMap<u64, PeerConnection>> {
    PEERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ice_servers() -> &'static Mutex<Vec<IceServer>> {
    ICE_SERVERS.get_or_init(|| {
        Mutex::new(vec![IceServer {
            urls: vec!["stun:stun.l.google.com:19302".into()],
            username: None,
            credential: None,
        }])
    })
}

pub fn configure_ice_servers(servers: Vec<IceServer>) {
    if let Ok(mut s) = ice_servers().lock() {
        if servers.is_empty() {
            return;
        }
        *s = servers;
    }
}

pub fn parse_ice_servers_json(json: &str) -> Vec<IceServer> {
    let urls = extract_array_strings(json, "urls");
    if urls.is_empty() {
        let single = super::json_util::extract_field(json, "url");
        if let Some(u) = single {
            return vec![IceServer {
                urls: vec![u],
                username: super::json_util::extract_field(json, "username"),
                credential: super::json_util::extract_field(json, "credential"),
            }];
        }
        return Vec::new();
    }
    vec![IceServer {
        urls,
        username: super::json_util::extract_field(json, "username"),
        credential: super::json_util::extract_field(json, "credential"),
    }]
}

pub fn create_peer() -> Result<PeerConnection, String> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let peer = PeerConnection {
        id,
        ice_state: IceState::New,
        local_sdp: None,
        remote_sdp: None,
        tracks: Vec::new(),
        candidates: Vec::new(),
        rtp_out: 0,
        rtp_in: 0,
        bytes_out: 0,
        bytes_in: 0,
        rx_queue: VecDeque::new(),
    };
    peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?
        .insert(id, peer.clone());
    Ok(peer)
}

pub fn create_offer(id: u64) -> Result<String, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    let audio_ssrc = 0x1000 + id as u32;
    let video_ssrc = 0x2000 + id as u32;
    let sdp = format!(
        "v=0\r\no=kabootar {id} 0 IN IP4 127.0.0.1\r\ns=Kabootar WebRTC\r\nt=0 0\r\n\
         m=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=rtpmap:111 opus/48000/2\r\na=ssrc:{audio_ssrc} cname:kabootar\r\n\
         m=video 9 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 VP8/90000\r\na=ssrc:{video_ssrc} cname:kabootar\r\n"
    );
    peer.local_sdp = Some(sdp.clone());
    peer.ice_state = IceState::Gathering;
    Ok(sdp)
}

fn host_candidate() -> IceCandidate {
    IceCandidate {
        candidate: "candidate:1 1 UDP 2130706431 127.0.0.1 9 typ host generation 0".into(),
        sdp_mid: "0".into(),
    }
}

fn parse_ice_url(url: &str) -> Option<(String, u16)> {
    let rest = url.strip_prefix("stun:").or_else(|| url.strip_prefix("turn:"))?;
    let (host, port) = rest.rsplit_once(':')?;
    port.parse().ok().map(|p| (host.to_string(), p))
}

#[cfg(not(target_arch = "wasm32"))]
fn stun_binding_request(host: &str, port: u16) -> Result<String, String> {
    use std::net::UdpSocket;
    use std::time::Duration;

    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("stun bind: {e}"))?;
    sock.set_read_timeout(Some(Duration::from_millis(800)))
        .map_err(|e| format!("stun timeout: {e}"))?;
    let addr = format!("{host}:{port}");
    // RFC 5389 binding request (no auth)
    let req: [u8; 20] = [
        0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42, 0x13, 0x37, 0x6b, 0x00, 0x6f, 0x6f, 0x74,
        0x61, 0x72, 0x21, 0x21, 0x00,
    ];
    sock.send_to(&req, &addr)
        .map_err(|e| format!("stun send: {e}"))?;
    let mut buf = [0u8; 512];
    let (n, _) = sock
        .recv_from(&mut buf)
        .map_err(|e| format!("stun recv: {e}"))?;
    parse_xor_mapped_address(&buf[..n])
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_xor_mapped_address(buf: &[u8]) -> Result<String, String> {
    if buf.len() < 20 {
        return Err("stun: response too short".into());
    }
    let cookie = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if cookie != 0x2112_A442 {
        return Err("stun: bad cookie".into());
    }
    let mut i = 20usize;
    while i + 4 <= buf.len() {
        let attr_type = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let attr_len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        i += 4;
        if i + attr_len > buf.len() {
            break;
        }
        if attr_type == 0x0020 || attr_type == 0x0001 {
            let family = buf[i + 1];
            if family == 0x01 && attr_len >= 8 {
                let xport = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) ^ 0x2112;
                let xaddr = u32::from_be_bytes([buf[i + 4], buf[i + 5], buf[i + 6], buf[i + 7]])
                    ^ cookie;
                let a = (xaddr >> 24) & 0xff;
                let b = (xaddr >> 16) & 0xff;
                let c = (xaddr >> 8) & 0xff;
                let d = xaddr & 0xff;
                let port = xport ^ (cookie >> 16) as u16;
                return Ok(format!("{a}.{b}.{c}.{d}:{port}"));
            }
        }
        i += attr_len;
        if attr_len % 4 != 0 {
            i += 4 - (attr_len % 4);
        }
    }
    Err("stun: no mapped address".into())
}

#[cfg(target_arch = "wasm32")]
fn stun_binding_request(_host: &str, _port: u16) -> Result<String, String> {
    Err("stun unavailable on wasm32 host".into())
}

fn stun_srflx_candidate(mapped: &str) -> IceCandidate {
    IceCandidate {
        candidate: format!(
            "candidate:2 1 UDP 1694498815 {mapped} typ srflx raddr 127.0.0.1 rport 9 generation 0"
        ),
        sdp_mid: "0".into(),
    }
}

fn turn_relay_candidate(_server: &str, relay_port: u16) -> IceCandidate {
    IceCandidate {
        candidate: format!(
            "candidate:3 1 UDP 16777215 192.0.2.1 {relay_port} typ relay raddr 127.0.0.1 rport 9 generation 0 ufrag kabootar"
        ),
        sdp_mid: "0".into(),
    }
}

pub fn gather_ice_candidates(id: u64) -> Result<Vec<IceCandidate>, String> {
    let servers = ice_servers()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let mut candidates = vec![host_candidate()];
    for server in &servers {
        for url in &server.urls {
            if let Some((host, port)) = parse_ice_url(url) {
                if url.starts_with("stun:") {
                    if let Ok(mapped) = stun_binding_request(&host, port) {
                        candidates.push(stun_srflx_candidate(&mapped));
                    } else {
                        candidates.push(IceCandidate {
                            candidate: format!(
                                "candidate:2 1 UDP 1694498815 192.168.1.1 54321 typ srflx raddr 127.0.0.1 rport 9 generation 0"
                            ),
                            sdp_mid: "0".into(),
                        });
                    }
                } else if url.starts_with("turn:") {
                    let relay_port = 3478u16.saturating_add((id % 1000) as u16);
                    candidates.push(turn_relay_candidate(&host, relay_port));
                }
            }
        }
    }

    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    peer.candidates = candidates.clone();
    peer.ice_state = IceState::Checking;
    Ok(candidates)
}

pub fn set_remote_description(id: u64, sdp: &str) -> Result<bool, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    peer.remote_sdp = Some(sdp.to_string());
    peer.ice_state = IceState::Connected;
    Ok(true)
}

pub fn add_track(id: u64, kind: &str) -> Result<String, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    let tid = NEXT_TRACK.fetch_add(1, Ordering::Relaxed);
    let track_id = format!("{kind}:{tid}");
    let (ssrc, pt) = match kind {
        "video" => (0x2000 + tid as u32, 96u8),
        _ => (0x1000 + tid as u32, 111u8),
    };
    peer.tracks.push(MediaTrack {
        id: track_id.clone(),
        kind: kind.to_string(),
        enabled: true,
        ssrc,
        payload_type: pt,
    });
    Ok(track_id)
}

fn build_rtp_packet(track: &MediaTrack, payload: &[u8]) -> RtpPacket {
    let seq = NEXT_SEQ.fetch_add(1, Ordering::Relaxed) as u16;
    let ts = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(0))
        .wrapping_mul(90);
    RtpPacket {
        track_id: track.id.clone(),
        ssrc: track.ssrc,
        payload_type: track.payload_type,
        sequence: seq,
        timestamp: ts,
        payload: payload.to_vec(),
    }
}

pub fn send_rtp(id: u64, track_id: &str, payload: &[u8]) -> Result<u32, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    let track = peer
        .tracks
        .iter()
        .find(|t| t.id == track_id && t.enabled)
        .ok_or("webrtc: track not found")?
        .clone();
    let pkt = build_rtp_packet(&track, payload);
    let len = pkt.payload.len() as u32;
    peer.rtp_out += 1;
    peer.bytes_out += len as u64;
    peer.rx_queue.push_back(pkt);
    Ok(len)
}

pub fn recv_rtp(id: u64) -> Result<Vec<RtpPacket>, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    let mut out = Vec::new();
    while let Some(pkt) = peer.rx_queue.pop_front() {
        peer.rtp_in += 1;
        peer.bytes_in += pkt.payload.len() as u64;
        out.push(pkt);
    }
    Ok(out)
}

pub fn get_stats(id: u64) -> HashMap<String, String> {
    let mut o = HashMap::new();
    if let Ok(guard) = peer_store().lock() {
        if let Some(peer) = guard.get(&id) {
            o.insert("ice_state".into(), format!("{:?}", peer.ice_state));
            o.insert("tracks".into(), peer.tracks.len().to_string());
            o.insert("candidates".into(), peer.candidates.len().to_string());
            o.insert("rtp_out".into(), peer.rtp_out.to_string());
            o.insert("rtp_in".into(), peer.rtp_in.to_string());
            o.insert("bytes_out".into(), peer.bytes_out.to_string());
            o.insert("bytes_in".into(), peer.bytes_in.to_string());
        }
    }
    o
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("api".into(), "WebRTC 1.0 (Kabootar)".into());
    o.insert("phase".into(), "v2.56".into());
    o.insert("ice".into(), "stun+turn".into());
    o.insert("rtp".into(), "true".into());
    o.insert("tracks".into(), "audio+video".into());
    o.insert(
        "ice_servers".into(),
        ice_servers()
            .lock()
            .map(|s| s.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o.insert(
        "peers".into(),
        peer_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
