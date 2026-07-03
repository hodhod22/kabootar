//! Embeds Kabootar UI font placeholder at build time (BSD-3 Null.ttf).

fn main() {
    let out = std::path::Path::new("assets/fonts/KabootarUI.ttf");
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(out, decode_font());
    println!("cargo:rerun-if-changed=build.rs");

    // Self-hosted compile/parse chains nest deeply; default 1 MiB stack overflows on Windows.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg-bin=kabootar=/STACK:16777216");
        println!("cargo:rustc-link-arg=/STACK:16777216");
    }
}

/// Minimal valid TrueType (Null.ttf, BSD-3-Clause, grzegorzrolek/null-ttf).
/// Used as placeholder; real rendering uses Kabootar builtin metrics. Override with
/// `KABOOTAR_FONT` or `assets/fonts/KabootarUI.ttf` for full TTF shaping.
fn decode_font() -> Vec<u8> {
    const B64: &str = "AAEAAAAJAIAAAwCIY21hcAkAAAAAAAC0AAAAEGdseWYAAAAAAAAAqAAAAApoZWFke/+BJQAAASgAAAA2aGhlYQEAAQAAAAEEAAAAJGhtdHgAAAAAAAAAnAAAAARsb2NhBQAAAAAAAKAAAAAGbWF4cAAAAwAAAADEAAAAIG5hbWWTTlIAAAABYAAAAEZwb3N0AAADAAAAAOQAAAAgAAAAAAAAAAUABQAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAIAAAAAAABAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAANSxUlNfDzz1AAAAEAAAAAAAAAAAAAAAAM8SxQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAUAQgABAAAAAAABAAQAAAABAAAAAAACAAQAAAABAAAAAAADAAQAAAABAAAAAAAEAAQAAAABAAAAAAAGAAQAAE51bGwAAA==";
    decode_b64(B64)
}

fn decode_b64(input: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u8;
    for b in input.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => continue,
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}
