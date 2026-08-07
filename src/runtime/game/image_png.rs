//! GP2b — PNG decode → `{ width, height, rgba }` (RGBA8 byte array).

use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::io::Cursor;

fn value_to_bytes(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    Value::Number(n) if (0..=255).contains(n) => out.push(*n as u8),
                    Value::Float(f) if *f >= 0.0 && *f <= 255.0 => out.push(*f as u8),
                    _ => return Err("image_decode_png: bytes must be 0..255".into()),
                }
            }
            Ok(out)
        }
        Value::String(s) => {
            let mut out = Vec::with_capacity(s.len());
            for ch in s.chars() {
                let cp = ch as u32;
                if cp > 0xFF {
                    return Err("image_decode_png: string contains non-Latin-1".into());
                }
                out.push(cp as u8);
            }
            Ok(out)
        }
        other if crate::runtime::shared_memory::is_uint8_array(other) => {
            crate::runtime::shared_memory::uint8_array_to_vec(other)
        }
        _ => Err("image_decode_png(bytes) expects Array, Uint8Array, or Latin-1 string".into()),
    }
}

/// Decode PNG bytes to `{ width, height, rgba: number[] }` (RGBA8).
pub fn decode_png(bytes: &[u8]) -> Result<Value, String> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png decode: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png frame: {e}"))?;
    let width = info.width;
    let height = info.height;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => {
            let rgb = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity((rgb.len() / 3) * 4);
            for chunk in rgb.chunks_exact(3) {
                out.extend_from_slice(chunk);
                out.push(255);
            }
            out
        }
        png::ColorType::Grayscale => {
            let g = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity(g.len() * 4);
            for &b in g {
                out.extend_from_slice(&[b, b, b, 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let ga = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity((ga.len() / 2) * 4);
            for chunk in ga.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        other => return Err(format!("png: unsupported color type {other:?}")),
    };
    let mut m = HashMap::new();
    m.insert("width".into(), Value::Number(width as i64));
    m.insert("height".into(), Value::Number(height as i64));
    m.insert(
        "rgba".into(), Value::from_array(rgba.into_iter().map(|b| Value::Number(b as i64)).collect()),
    );
    Ok(Value::from_object(m))
}

pub fn image_decode_png_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let bytes = value_to_bytes(args.first().ok_or("image_decode_png(bytes)")?)?;
    decode_png(&bytes)
}
