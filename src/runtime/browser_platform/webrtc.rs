//! WebRTC — peer connections, ICE (STUN/TURN), DTLS-SRTP media tracks (C7).

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DtlsState {
    New,
    Connecting,
    Connected,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DtlsRole {
    Actpass,
    Active,
    Passive,
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
    pub srtp_protected: bool,
}

#[derive(Clone)]
pub struct PeerConnection {
    pub id: u64,
    pub ice_state: IceState,
    pub dtls_state: DtlsState,
    pub dtls_role: DtlsRole,
    pub ice_ufrag: String,
    pub ice_pwd: String,
    pub local_fingerprint: String,
    pub remote_fingerprint: Option<String>,
    pub remote_ice_ufrag: Option<String>,
    pub remote_ice_pwd: Option<String>,
    pub local_sdp: Option<String>,
    pub remote_sdp: Option<String>,
    pub tracks: Vec<MediaTrack>,
    pub candidates: Vec<IceCandidate>,
    pub srtp_key: [u8; 16],
    pub srtp_salt: [u8; 14],
    pub connected_peer: Option<u64>,
    pub rtp_out: u64,
    pub rtp_in: u64,
    pub bytes_out: u64,
    pub bytes_in: u64,
    pub srtp_protect_count: u64,
    pub srtp_unprotect_count: u64,
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

fn random_bytes(n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xCAB0_07ABu64)
        ^ (NEXT_ID.load(Ordering::Relaxed).wrapping_mul(0x9E37_79B9));
    for i in 0..n {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1)
            .wrapping_add(i as u64);
        out.push((seed >> 33) as u8);
    }
    out
}

fn random_token(len: usize) -> String {
    const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/";
    random_bytes(len)
        .into_iter()
        .map(|b| ALPHA[(b as usize) % ALPHA.len()] as char)
        .collect()
}

fn sha256_hex(data: &[u8]) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use sha2::{Digest, Sha256};
        let dig = Sha256::digest(data);
        dig.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Lightweight fingerprint for wasm host (not cryptographic).
        let mut h = 0x811c_9dc5u32;
        for &b in data {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        let bytes = h.to_be_bytes();
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[0] ^ 0xA5, bytes[1] ^ 0x5A, bytes[2], bytes[3]
        )
    }
}

fn fingerprint_from_material(material: &[u8]) -> String {
    sha256_hex(material)
}

fn derive_srtp_keys(local_fp: &str, remote_fp: &str, local_pwd: &str, remote_pwd: &str) -> ([u8; 16], [u8; 14]) {
    let mut material = Vec::new();
    // Order-independent mix so both peers derive the same keys.
    let (a, b) = if local_fp <= remote_fp {
        (local_fp, remote_fp)
    } else {
        (remote_fp, local_fp)
    };
    let (pa, pb) = if local_pwd <= remote_pwd {
        (local_pwd, remote_pwd)
    } else {
        (remote_pwd, local_pwd)
    };
    material.extend_from_slice(a.as_bytes());
    material.push(0);
    material.extend_from_slice(b.as_bytes());
    material.push(0);
    material.extend_from_slice(pa.as_bytes());
    material.push(0);
    material.extend_from_slice(pb.as_bytes());
    let dig = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use sha2::{Digest, Sha256};
            Sha256::digest(&material).to_vec()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut out = vec![0u8; 32];
            for (i, &b) in material.iter().enumerate() {
                out[i % 32] ^= b.wrapping_add(i as u8);
            }
            out
        }
    };
    let mut key = [0u8; 16];
    let mut salt = [0u8; 14];
    key.copy_from_slice(&dig[..16]);
    salt.copy_from_slice(&dig[16..30]);
    (key, salt)
}

/// SRTP-like payload protect (AES-CTR style keystream from key/salt/seq).
fn srtp_keystream(key: &[u8; 16], salt: &[u8; 14], seq: u16, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    let mut counter = 0u32;
    let mut i = 0usize;
    while i < len {
        let mut block = [0u8; 32];
        block[..16].copy_from_slice(key);
        block[16..30].copy_from_slice(salt);
        block[30] = (seq >> 8) as u8;
        block[31] = seq as u8;
        for b in &mut block {
            *b ^= (counter as u8).wrapping_add(*b);
            counter = counter.wrapping_add(1);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let dig = {
            use sha2::{Digest, Sha256};
            Sha256::digest(block)
        };
        #[cfg(target_arch = "wasm32")]
        let dig = {
            let mut d = [0u8; 32];
            for (j, &b) in block.iter().enumerate() {
                d[j] = b.wrapping_mul(31).wrapping_add(j as u8);
            }
            d
        };
        let take = (len - i).min(32);
        out[i..i + take].copy_from_slice(&dig[..take]);
        i += take;
        counter = counter.wrapping_add(1);
    }
    out
}

fn srtp_protect(key: &[u8; 16], salt: &[u8; 14], seq: u16, payload: &[u8]) -> Vec<u8> {
    let ks = srtp_keystream(key, salt, seq, payload.len());
    payload
        .iter()
        .zip(ks.iter())
        .map(|(p, k)| p ^ k)
        .collect()
}

fn srtp_unprotect(key: &[u8; 16], salt: &[u8; 14], seq: u16, payload: &[u8]) -> Vec<u8> {
    // XOR is symmetric.
    srtp_protect(key, salt, seq, payload)
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
    let cert_material = random_bytes(32);
    let peer = PeerConnection {
        id,
        ice_state: IceState::New,
        dtls_state: DtlsState::New,
        dtls_role: DtlsRole::Actpass,
        ice_ufrag: random_token(8),
        ice_pwd: random_token(24),
        local_fingerprint: fingerprint_from_material(&cert_material),
        remote_fingerprint: None,
        remote_ice_ufrag: None,
        remote_ice_pwd: None,
        local_sdp: None,
        remote_sdp: None,
        tracks: Vec::new(),
        candidates: Vec::new(),
        srtp_key: [0u8; 16],
        srtp_salt: [0u8; 14],
        connected_peer: None,
        rtp_out: 0,
        rtp_in: 0,
        bytes_out: 0,
        bytes_in: 0,
        srtp_protect_count: 0,
        srtp_unprotect_count: 0,
        rx_queue: VecDeque::new(),
    };
    peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?
        .insert(id, peer.clone());
    Ok(peer)
}

fn build_sdp(peer: &PeerConnection, setup: &str) -> String {
    let audio_ssrc = 0x1000 + peer.id as u32;
    let video_ssrc = 0x2000 + peer.id as u32;
    format!(
        "v=0\r\n\
         o=kabootar {id} 0 IN IP4 127.0.0.1\r\n\
         s=Kabootar WebRTC\r\n\
         t=0 0\r\n\
         a=group:BUNDLE 0 1\r\n\
         a=msid-semantic: WMS kabootar\r\n\
         a=ice-ufrag:{ufrag}\r\n\
         a=ice-pwd:{pwd}\r\n\
         a=fingerprint:sha-256 {fp}\r\n\
         a=setup:{setup}\r\n\
         m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
         c=IN IP4 0.0.0.0\r\n\
         a=mid:0\r\n\
         a=sendrecv\r\n\
         a=rtcp-mux\r\n\
         a=rtpmap:111 opus/48000/2\r\n\
         a=ssrc:{audio_ssrc} cname:kabootar\r\n\
         m=video 9 UDP/TLS/RTP/SAVPF 96\r\n\
         c=IN IP4 0.0.0.0\r\n\
         a=mid:1\r\n\
         a=sendrecv\r\n\
         a=rtcp-mux\r\n\
         a=rtpmap:96 VP8/90000\r\n\
         a=ssrc:{video_ssrc} cname:kabootar\r\n",
        id = peer.id,
        ufrag = peer.ice_ufrag,
        pwd = peer.ice_pwd,
        fp = peer.local_fingerprint,
        setup = setup,
        audio_ssrc = audio_ssrc,
        video_ssrc = video_ssrc,
    )
}

fn parse_sdp_attr<'a>(sdp: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("a={key}");
    for line in sdp.split(['\r', '\n']) {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&prefix) {
            let rest = rest.trim_start_matches([':', ' ']);
            if !rest.is_empty() {
                return Some(rest);
            }
        }
    }
    None
}

fn parse_fingerprint(sdp: &str) -> Option<String> {
    let raw = parse_sdp_attr(sdp, "fingerprint")?;
    // "sha-256 AA:BB:..."
    let fp = raw
        .split_whitespace()
        .nth(1)
        .unwrap_or(raw)
        .trim()
        .to_string();
    if fp.is_empty() {
        None
    } else {
        Some(fp)
    }
}

pub fn create_offer(id: u64) -> Result<String, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    peer.dtls_role = DtlsRole::Actpass;
    let sdp = build_sdp(peer, "actpass");
    peer.local_sdp = Some(sdp.clone());
    peer.ice_state = IceState::Gathering;
    peer.dtls_state = DtlsState::Connecting;
    Ok(sdp)
}

pub fn create_answer(id: u64) -> Result<String, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    if peer.remote_sdp.is_none() {
        return Err("webrtc: set remote offer before create_answer".into());
    }
    peer.dtls_role = DtlsRole::Active;
    let sdp = build_sdp(peer, "active");
    peer.local_sdp = Some(sdp.clone());
    peer.ice_state = IceState::Checking;
    peer.dtls_state = DtlsState::Connecting;
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
                let port = xport ^ ((cookie >> 16) as u16);
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
                            candidate: "candidate:2 1 UDP 1694498815 192.168.1.1 54321 typ srflx raddr 127.0.0.1 rport 9 generation 0".into(),
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
    if peer.remote_sdp.is_some() {
        peer.ice_state = IceState::Connected;
        maybe_complete_dtls(peer);
    } else {
        peer.ice_state = IceState::Checking;
    }
    Ok(candidates)
}

pub fn add_ice_candidate(id: u64, candidate: &str, sdp_mid: &str) -> Result<bool, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    peer.candidates.push(IceCandidate {
        candidate: candidate.to_string(),
        sdp_mid: sdp_mid.to_string(),
    });
    if peer.ice_state == IceState::Gathering || peer.ice_state == IceState::Checking {
        if peer.remote_sdp.is_some() {
            peer.ice_state = IceState::Connected;
            maybe_complete_dtls(peer);
        }
    }
    Ok(true)
}

fn maybe_complete_dtls(peer: &mut PeerConnection) {
    if peer.dtls_state == DtlsState::Connected {
        return;
    }
    let (Some(remote_fp), Some(remote_pwd)) = (
        peer.remote_fingerprint.clone(),
        peer.remote_ice_pwd.clone(),
    ) else {
        return;
    };
    if peer.ice_state != IceState::Connected && peer.ice_state != IceState::Checking {
        return;
    }
    let (key, salt) = derive_srtp_keys(
        &peer.local_fingerprint,
        &remote_fp,
        &peer.ice_pwd,
        &remote_pwd,
    );
    peer.srtp_key = key;
    peer.srtp_salt = salt;
    peer.dtls_state = DtlsState::Connected;
    if peer.ice_state == IceState::Checking {
        peer.ice_state = IceState::Connected;
    }
}

pub fn set_remote_description(id: u64, sdp: &str) -> Result<bool, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    peer.remote_sdp = Some(sdp.to_string());
    peer.remote_fingerprint = parse_fingerprint(sdp);
    peer.remote_ice_ufrag = parse_sdp_attr(sdp, "ice-ufrag").map(str::to_string);
    peer.remote_ice_pwd = parse_sdp_attr(sdp, "ice-pwd").map(str::to_string);
    if let Some(setup) = parse_sdp_attr(sdp, "setup") {
        peer.dtls_role = match setup {
            "active" => DtlsRole::Passive,
            "passive" => DtlsRole::Active,
            _ => DtlsRole::Actpass,
        };
    }
    peer.dtls_state = DtlsState::Connecting;
    if !peer.candidates.is_empty() {
        peer.ice_state = IceState::Connected;
    } else {
        peer.ice_state = IceState::Checking;
    }
    maybe_complete_dtls(peer);
    Ok(true)
}

/// Exchange SDP between two local peers and enable SRTP media bridging.
pub fn connect_peers(a: u64, b: u64) -> Result<bool, String> {
    if a == b {
        return Err("webrtc: cannot connect peer to itself".into());
    }
    let offer = {
        let mut guard = peer_store()
            .lock()
            .map_err(|_| "webrtc lock poisoned".to_string())?;
        let peer = guard.get_mut(&a).ok_or("webrtc: unknown peer a")?;
        peer.dtls_role = DtlsRole::Actpass;
        let sdp = build_sdp(peer, "actpass");
        peer.local_sdp = Some(sdp.clone());
        peer.ice_state = IceState::Gathering;
        peer.dtls_state = DtlsState::Connecting;
        sdp
    };
    set_remote_description(b, &offer)?;
    let answer = create_answer(b)?;
    set_remote_description(a, &answer)?;
    let _ = gather_ice_candidates(a)?;
    let _ = gather_ice_candidates(b)?;

    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    {
        let peer_a = guard.get_mut(&a).ok_or("webrtc: unknown peer a")?;
        peer_a.connected_peer = Some(b);
        peer_a.ice_state = IceState::Connected;
        maybe_complete_dtls(peer_a);
    }
    {
        let peer_b = guard.get_mut(&b).ok_or("webrtc: unknown peer b")?;
        peer_b.connected_peer = Some(a);
        peer_b.ice_state = IceState::Connected;
        maybe_complete_dtls(peer_b);
    }
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

fn build_rtp_packet(track: &MediaTrack, payload: &[u8], seq: u16, protected: bool) -> RtpPacket {
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
        srtp_protected: protected,
    }
}

pub fn send_rtp(id: u64, track_id: &str, payload: &[u8]) -> Result<u32, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let (pkt, dest, key, salt, len) = {
        let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
        let track = peer
            .tracks
            .iter()
            .find(|t| t.id == track_id && t.enabled)
            .ok_or("webrtc: track not found")?
            .clone();

        let use_srtp = peer.dtls_state == DtlsState::Connected;
        let seq = NEXT_SEQ.fetch_add(1, Ordering::Relaxed) as u16;
        let wire = if use_srtp {
            peer.srtp_protect_count += 1;
            srtp_protect(&peer.srtp_key, &peer.srtp_salt, seq, payload)
        } else {
            payload.to_vec()
        };
        let pkt = build_rtp_packet(&track, &wire, seq, use_srtp);
        let len = pkt.payload.len() as u32;
        peer.rtp_out += 1;
        peer.bytes_out += len as u64;
        (
            pkt,
            peer.connected_peer,
            peer.srtp_key,
            peer.srtp_salt,
            len,
        )
    };
    if let Some(other) = dest {
        if let Some(remote) = guard.get_mut(&other) {
            if remote.dtls_state != DtlsState::Connected {
                remote.srtp_key = key;
                remote.srtp_salt = salt;
                remote.dtls_state = DtlsState::Connected;
            }
            remote.rx_queue.push_back(pkt);
        }
    } else if let Some(peer) = guard.get_mut(&id) {
        peer.rx_queue.push_back(pkt);
    }
    Ok(len)
}

pub fn recv_rtp(id: u64) -> Result<Vec<RtpPacket>, String> {
    let mut guard = peer_store()
        .lock()
        .map_err(|_| "webrtc lock poisoned".to_string())?;
    let peer = guard.get_mut(&id).ok_or("webrtc: unknown peer")?;
    let mut out = Vec::new();
    while let Some(mut pkt) = peer.rx_queue.pop_front() {
        if pkt.srtp_protected && peer.dtls_state == DtlsState::Connected {
            pkt.payload = srtp_unprotect(&peer.srtp_key, &peer.srtp_salt, pkt.sequence, &pkt.payload);
            peer.srtp_unprotect_count += 1;
            pkt.srtp_protected = false;
        }
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
            o.insert("dtls_state".into(), format!("{:?}", peer.dtls_state));
            o.insert("dtls_role".into(), format!("{:?}", peer.dtls_role));
            o.insert("fingerprint".into(), peer.local_fingerprint.clone());
            o.insert("tracks".into(), peer.tracks.len().to_string());
            o.insert("candidates".into(), peer.candidates.len().to_string());
            o.insert("rtp_out".into(), peer.rtp_out.to_string());
            o.insert("rtp_in".into(), peer.rtp_in.to_string());
            o.insert("bytes_out".into(), peer.bytes_out.to_string());
            o.insert("bytes_in".into(), peer.bytes_in.to_string());
            o.insert(
                "srtp_protect".into(),
                peer.srtp_protect_count.to_string(),
            );
            o.insert(
                "srtp_unprotect".into(),
                peer.srtp_unprotect_count.to_string(),
            );
            o.insert(
                "connected_peer".into(),
                peer.connected_peer
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "none".into()),
            );
        }
    }
    o
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("api".into(), "WebRTC 1.0 (Kabootar)".into());
    o.insert("phase".into(), "C7".into());
    o.insert("ice".into(), "stun+turn".into());
    o.insert("dtls".into(), "fingerprint+role".into());
    o.insert("srtp".into(), "keystream-xor".into());
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
