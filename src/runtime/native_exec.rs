//! SH28 — real machine-code exec: AOT/JIT image bytes run on the actual CPU
//! via an anonymous RW mapping + `make_exec` + a direct call — NOT the
//! simulated guest MM (`os_mm_call` interprets template bytes; this runs the
//! bytes as real native code). This is the primitive the `exec-image` verb
//! and the `os_native_exec` Kab native sit on.

use std::io;

/// Page cap for exec payloads — templates/images are tiny; anything larger
/// is a malformed image, not a code section.
const MAX_EXEC_BYTES: usize = 4096;

/// Run `bytes` as a real `extern "C" fn() -> i64` on the host CPU. The byte
/// slice must be position-independent, preserve callee-saved registers, keep
/// the stack 16-byte aligned at its own calls, and end in `ret` (0xC3) — the
/// AOT emitters in lib/kab/aot guarantee that contract for their domain.
#[cfg(not(target_arch = "wasm32"))]
pub fn exec_bytes(bytes: &[u8]) -> Result<i64, String> {
    if bytes.is_empty() || bytes.len() > MAX_EXEC_BYTES {
        return Err(format!("native_exec: bad length {}", bytes.len()));
    }
    let mut mm = memmap2::MmapOptions::new()
        .len(bytes.len().max(1))
        .map_anon()
        .map_err(|e: io::Error| format!("native_exec: map: {e}"))?;
    mm[..].copy_from_slice(bytes);
    let code = mm
        .make_exec()
        .map_err(|e: io::Error| format!("native_exec: make_exec: {e}"))?;
    let f: unsafe extern "C" fn() -> i64 = unsafe { std::mem::transmute(code.as_ptr()) };
    Ok(unsafe { f() })
}

#[cfg(target_arch = "wasm32")]
pub fn exec_bytes(_bytes: &[u8]) -> Result<i64, String> {
    Err("native_exec: not available on wasm32".into())
}

/// Decode a lowercase hex string into bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let b = s.as_bytes();
    if b.len() % 2 != 0 {
        return Err("native_exec: odd hex length".into());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(format!("native_exec: bad hex digit {}", c as char)),
        }
    };
    let mut out = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        out.push((val(b[i])? << 4) | val(b[i + 1])?);
        i += 2;
    }
    Ok(out)
}

/// Extract the `|code:<arch>:<hex>|` payload from a `kabootar-native/1`
/// image; `arch` must match the host (`x64` on x86_64).
fn image_code_bytes(image: &str) -> Result<Vec<u8>, String> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err("native_exec: unsupported host arch".into());
    };
    let needle = format!("|code:{arch}:");
    let Some(at) = image.find(&needle) else {
        return Err(format!("native_exec: image lacks code:{arch}"));
    };
    let start = at + needle.len();
    let end = image[start..]
        .find('|')
        .map(|i| start + i)
        .unwrap_or(image.len());
    hex_decode(&image[start..end])
}

/// Parse `|<key>:<num>|` out of a `kabootar-native/1` image record.
fn image_field_num(image: &str, key: &str) -> Option<usize> {
    let needle = format!("|{key}:");
    let at = image.find(&needle)? + needle.len();
    let end = image[at..].find('|').map(|i| at + i).unwrap_or(image.len());
    image[at..end].parse().ok()
}

/// `kabootar exec-image` shared leg: validate a `kabootar-native/1` image,
/// pull its host-arch code section, run the bytes at `entry:` (0 for the
/// fused templates), and return rax — evidence that an image-carried
/// machine-code payload executes on the real CPU.
pub fn exec_image_text(image: &str) -> Result<i64, String> {
    if !image.starts_with("kabootar-native/1|") {
        return Err("native_exec: not a kabootar-native/1 image".into());
    }
    let bytes = image_code_bytes(image)?;
    let entry = image_field_num(image, "entry").unwrap_or(0);
    if entry >= bytes.len() {
        return Err(format!(
            "native_exec: entry {entry} past code len {}",
            bytes.len()
        ));
    }
    exec_bytes(&bytes[entry..])
}

/// `kabootar exec-image <path>` leg: load an image from the host FS.
pub fn exec_image_file(path: &str) -> Result<i64, String> {
    let image = std::fs::read_to_string(path)
        .map_err(|e| format!("native_exec: read {path}: {e}"))?;
    exec_image_text(&image)
}

/// `kabootar exec-image <arg>`: `<arg>` may be a path to an image on the
/// host FS, or the `kabootar-native/1|...` image text itself (argv-carried —
/// how the Kab VFS-only producer hands an image to a real child process).
pub fn exec_image_arg(arg: &str) -> Result<i64, String> {
    if arg.starts_with("kabootar-native/1|") {
        exec_image_text(arg)
    } else {
        exec_image_file(arg)
    }
}
