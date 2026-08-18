//! P10f — binary `.kbcb` envelope around `kabootar-bytecode/1` text.
//! Magic + version + payload length, then UTF-8 `.kbc`. Load skips a second
//! `read_to_string` guess: bytes are framed; deserialize still parses v1 lines
//! (no pre-collected `Vec` of every line).

use super::types::{deserialize, serialize, BytecodeModule};

pub const KBCB_MAGIC: &[u8; 4] = b"KBCB";
pub const KBCB_VERSION: u8 = 1;

pub fn serialize_kbcb(module: &BytecodeModule) -> Vec<u8> {
    let text = serialize(module);
    let mut out = Vec::with_capacity(9 + text.len());
    out.extend_from_slice(KBCB_MAGIC);
    out.push(KBCB_VERSION);
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}

pub fn looks_like_kbcb(bytes: &[u8]) -> bool {
    bytes.len() >= 9 && bytes.starts_with(KBCB_MAGIC)
}

pub fn deserialize_kbcb(bytes: &[u8]) -> Result<BytecodeModule, String> {
    if !looks_like_kbcb(bytes) {
        return Err("not a .kbcb file".into());
    }
    if bytes[4] != KBCB_VERSION {
        return Err(format!("unsupported kbcb version {}", bytes[4]));
    }
    let n = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let start: usize = 9;
    let end = start
        .checked_add(n)
        .ok_or_else(|| "kbcb length overflow".to_string())?;
    if end > bytes.len() {
        return Err("kbcb truncated".into());
    }
    let text = std::str::from_utf8(&bytes[start..end]).map_err(|e| e.to_string())?;
    deserialize(text)
}
