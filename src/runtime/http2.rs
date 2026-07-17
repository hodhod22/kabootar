//! Minimal HTTP/2 preface + SETTINGS handshake for `Deno.serve` / `serve_async`.
//!
//! Full multiplexed streams and HPACK are out of scope; this unlocks ALPN/`http2_supported`
//! and a cleartext preface handshake so clients can negotiate h2 without hanging.

/// RFC 7540 connection preface.
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

pub fn supported() -> bool {
    true
}

/// True when `buf` begins with the HTTP/2 connection preface (full or short PRI form).
pub fn is_preface(buf: &[u8]) -> bool {
    buf.starts_with(CONNECTION_PREFACE) || buf.starts_with(b"PRI * HTTP/2.0")
}

fn frame(length: u32, ty: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + payload.len());
    out.push(((length >> 16) & 0xff) as u8);
    out.push(((length >> 8) & 0xff) as u8);
    out.push((length & 0xff) as u8);
    out.push(ty);
    out.push(flags);
    out.push(((stream_id >> 24) & 0x7f) as u8);
    out.push(((stream_id >> 16) & 0xff) as u8);
    out.push(((stream_id >> 8) & 0xff) as u8);
    out.push((stream_id & 0xff) as u8);
    out.extend_from_slice(payload);
    out
}

/// Empty SETTINGS frame (type 0x4).
pub fn settings_frame() -> Vec<u8> {
    frame(0, 0x04, 0x00, 0, &[])
}

/// SETTINGS ACK (type 0x4, flags ACK).
pub fn settings_ack_frame() -> Vec<u8> {
    frame(0, 0x04, 0x01, 0, &[])
}

/// Bytes to write after accepting a cleartext h2 preface.
pub fn handshake_response() -> Vec<u8> {
    let mut out = settings_frame();
    out.extend_from_slice(&settings_ack_frame());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preface_and_frames() {
        assert!(is_preface(CONNECTION_PREFACE));
        assert!(is_preface(b"PRI * HTTP/2.0\r\n"));
        assert!(!is_preface(b"GET / HTTP/1.1\r\n"));
        let settings = settings_frame();
        assert_eq!(settings.len(), 9);
        assert_eq!(settings[3], 0x04);
        let ack = settings_ack_frame();
        assert_eq!(ack[4], 0x01);
        assert!(handshake_response().len() >= 18);
        assert!(supported());
    }
}
