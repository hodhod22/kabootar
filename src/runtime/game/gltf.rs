//! GP2a — minimal glTF 2.0 JSON loader (one mesh + material + optional translation animation).

use crate::runtime::stdlib::base64_decode;
use crate::value::{Environment, Value};
use serde_json::Value as Json;
use std::collections::HashMap;

fn f64_num(v: &Json) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

fn usize_num(v: &Json) -> Option<usize> {
    v.as_u64()
        .map(|n| n as usize)
        .or_else(|| v.as_i64().filter(|&n| n >= 0).map(|n| n as usize))
}

fn decode_buffer(buffers: &[Json]) -> Result<Vec<u8>, String> {
    let buf0 = buffers
        .first()
        .ok_or("gltf: expected at least one buffer")?;
    if let Some(uri) = buf0.get("uri").and_then(|u| u.as_str()) {
        let b64 = if let Some(rest) = uri.strip_prefix("data:") {
            let comma = rest
                .find(',')
                .ok_or("gltf: data URI missing comma")?;
            &rest[comma + 1..]
        } else {
            return Err("gltf: only data URI buffers supported in subset".into());
        };
        return base64_decode(b64);
    }
    Err("gltf: buffer.uri required (data URI base64)".into())
}

fn slice_view<'a>(bin: &'a [u8], views: &[Json], view_i: usize) -> Result<&'a [u8], String> {
    let view = views
        .get(view_i)
        .ok_or_else(|| format!("gltf: missing bufferView {view_i}"))?;
    let offset = view
        .get("byteOffset")
        .and_then(usize_num)
        .unwrap_or(0);
    let length = view
        .get("byteLength")
        .and_then(usize_num)
        .ok_or("gltf: bufferView.byteLength required")?;
    let end = offset
        .checked_add(length)
        .ok_or("gltf: bufferView overflow")?;
    if end > bin.len() {
        return Err("gltf: bufferView exceeds buffer".into());
    }
    Ok(&bin[offset..end])
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, String> {
    if offset + 4 > bytes.len() {
        return Err("gltf: float read OOB".into());
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[offset..offset + 4]);
    Ok(f32::from_le_bytes(arr))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
    if offset + 2 > bytes.len() {
        return Err("gltf: u16 read OOB".into());
    }
    let mut arr = [0u8; 2];
    arr.copy_from_slice(&bytes[offset..offset + 2]);
    Ok(u16::from_le_bytes(arr))
}

fn accessor_floats(bin: &[u8], root: &Json, acc_i: usize, comps: usize) -> Result<Vec<f64>, String> {
    let accessors = root
        .get("accessors")
        .and_then(|a| a.as_array())
        .ok_or("gltf: missing accessors")?;
    let views = root
        .get("bufferViews")
        .and_then(|a| a.as_array())
        .ok_or("gltf: missing bufferViews")?;
    let acc = accessors
        .get(acc_i)
        .ok_or_else(|| format!("gltf: missing accessor {acc_i}"))?;
    let ctype = acc
        .get("componentType")
        .and_then(usize_num)
        .ok_or("gltf: accessor.componentType")?;
    if ctype != 5126 {
        return Err(format!("gltf: expected FLOAT accessor, got {ctype}"));
    }
    let count = acc
        .get("count")
        .and_then(usize_num)
        .ok_or("gltf: accessor.count")?;
    let view_i = acc
        .get("bufferView")
        .and_then(usize_num)
        .ok_or("gltf: accessor.bufferView")?;
    let byte_offset = acc
        .get("byteOffset")
        .and_then(usize_num)
        .unwrap_or(0);
    let view = slice_view(bin, views, view_i)?;
    let mut out = Vec::with_capacity(count * comps);
    for i in 0..count {
        let base = byte_offset + i * comps * 4;
        for c in 0..comps {
            out.push(read_f32_le(view, base + c * 4)? as f64);
        }
    }
    Ok(out)
}

fn accessor_u16(bin: &[u8], root: &Json, acc_i: usize) -> Result<Vec<i64>, String> {
    let accessors = root
        .get("accessors")
        .and_then(|a| a.as_array())
        .ok_or("gltf: missing accessors")?;
    let views = root
        .get("bufferViews")
        .and_then(|a| a.as_array())
        .ok_or("gltf: missing bufferViews")?;
    let acc = accessors
        .get(acc_i)
        .ok_or_else(|| format!("gltf: missing accessor {acc_i}"))?;
    let ctype = acc
        .get("componentType")
        .and_then(usize_num)
        .ok_or("gltf: accessor.componentType")?;
    if ctype != 5123 {
        return Err(format!("gltf: expected UNSIGNED_SHORT indices, got {ctype}"));
    }
    let count = acc
        .get("count")
        .and_then(usize_num)
        .ok_or("gltf: accessor.count")?;
    let view_i = acc
        .get("bufferView")
        .and_then(usize_num)
        .ok_or("gltf: accessor.bufferView")?;
    let byte_offset = acc
        .get("byteOffset")
        .and_then(usize_num)
        .unwrap_or(0);
    let view = slice_view(bin, views, view_i)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(read_u16_le(view, byte_offset + i * 2)? as i64);
    }
    Ok(out)
}

fn floats_to_value(floats: Vec<f64>) -> Value {
    Value::Array(floats.into_iter().map(Value::Float).collect())
}

/// Load a glTF 2.0 JSON string into a Kab object:
/// `{ floats, indices?, color, animations }`
pub fn load_json(text: &str) -> Result<Value, String> {
    let root: Json = serde_json::from_str(text).map_err(|e| format!("gltf JSON: {e}"))?;
    let buffers = root
        .get("buffers")
        .and_then(|b| b.as_array())
        .ok_or("gltf: missing buffers")?;
    let bin = decode_buffer(buffers)?;

    let meshes = root
        .get("meshes")
        .and_then(|m| m.as_array())
        .ok_or("gltf: missing meshes")?;
    let mesh0 = meshes.first().ok_or("gltf: empty meshes")?;
    let prim = mesh0
        .get("primitives")
        .and_then(|p| p.as_array())
        .and_then(|a| a.first())
        .ok_or("gltf: mesh missing primitives")?;
    let pos_i = prim
        .pointer("/attributes/POSITION")
        .and_then(usize_num)
        .ok_or("gltf: POSITION accessor required")?;
    let floats = accessor_floats(&bin, &root, pos_i, 3)?;

    let mut out = HashMap::new();
    out.insert("floats".into(), floats_to_value(floats));

    if let Some(idx_i) = prim.get("indices").and_then(usize_num) {
        let indices = accessor_u16(&bin, &root, idx_i)?;
        out.insert(
            "indices".into(),
            Value::Array(indices.into_iter().map(Value::Number).collect()),
        );
    }

    let color = root
        .pointer("/materials/0/pbrMetallicRoughness/baseColorFactor")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(f64_num)
                .map(Value::Float)
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() >= 4)
        .unwrap_or_else(|| {
            vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
            ]
        });
    out.insert("color".into(), Value::Array(color));

    let mut anims = Vec::new();
    if let Some(animations) = root.get("animations").and_then(|a| a.as_array()) {
        if let Some(anim0) = animations.first() {
            if let Some(ch0) = anim0
                .get("channels")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
            {
                let path = ch0
                    .pointer("/target/path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                if path == "translation" {
                    let sampler_i = ch0
                        .get("sampler")
                        .and_then(usize_num)
                        .ok_or("gltf: animation channel.sampler")?;
                    let sampler = anim0
                        .get("samplers")
                        .and_then(|s| s.as_array())
                        .and_then(|a| a.get(sampler_i))
                        .ok_or("gltf: missing animation sampler")?;
                    let input_i = sampler
                        .get("input")
                        .and_then(usize_num)
                        .ok_or("gltf: sampler.input")?;
                    let output_i = sampler
                        .get("output")
                        .and_then(usize_num)
                        .ok_or("gltf: sampler.output")?;
                    let times = accessor_floats(&bin, &root, input_i, 1)?;
                    let values = accessor_floats(&bin, &root, output_i, 3)?;
                    let mut channel = HashMap::new();
                    channel.insert("path".into(), Value::String("translation".into()));
                    channel.insert("times".into(), floats_to_value(times));
                    channel.insert("translations".into(), floats_to_value(values));
                    anims.push(Value::Object(channel));
                }
            }
        }
    }
    out.insert("animations".into(), Value::Array(anims));

    Ok(Value::Object(out))
}

pub fn gltf_load_json_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("gltf_load_json(text)".into()),
    };
    load_json(text)
}
