//! Base64 encode/decode — shared by `btoa`/`atob` and WebSocket.

const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(data: &[u8]) -> String {
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

pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;
    for ch in input.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if ch == b'=' {
            break;
        }
        let val = TABLE
            .iter()
            .position(|&t| t == ch)
            .ok_or_else(|| format!("invalid base64 character: {ch}"))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(bytes)
}

/// JS `btoa` — each code unit must be U+0000..U+00FF (Latin-1 byte).
pub fn btoa_string(s: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        if cp > 0xFF {
            return Err("btoa: string contains characters outside Latin-1 range".into());
        }
        bytes.push(cp as u8);
    }
    Ok(encode(&bytes))
}

/// JS `atob` — returns a string with one byte per char (Latin-1 code units).
pub fn atob_string(encoded: &str) -> Result<String, String> {
    let bytes = decode(encoded)?;
    let mut out = String::with_capacity(bytes.len());
    for b in bytes {
        out.push(char::from_u32(b as u32).ok_or("atob: invalid decoded byte")?);
    }
    Ok(out)
}
