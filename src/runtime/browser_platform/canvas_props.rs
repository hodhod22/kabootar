//! Canvas 2D context property getters/setters (`ctx.fillStyle = "#f00"`).

use super::canvas_host;
use crate::runtime::render::canvas2d;
use crate::value::Value;
use std::collections::HashMap;

pub fn is_canvas_ctx(map: &HashMap<String, Value>) -> bool {
    matches!(map.get("__kab_ctx"), Some(Value::Bool(true)))
}

fn native_id_from_ctx(map: &HashMap<String, Value>) -> Result<u64, String> {
    match map.get("id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("canvas context missing id".into()),
    }
}

fn host_ctx_id(map: &HashMap<String, Value>) -> Option<u64> {
    match map.get("host_ctx_id") {
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    }
}

fn expect_str(val: &Value, prop: &str) -> Result<String, String> {
    match val {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!("{prop} expects a string")),
    }
}

fn expect_f64(val: &Value, prop: &str) -> Result<f64, String> {
    match val {
        Value::Number(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(format!("{prop} expects a number")),
    }
}

/// Read a virtual canvas property. Returns `None` if the field is not a known style property.
pub fn try_read_property(map: &HashMap<String, Value>, field: &str) -> Option<Value> {
    if !is_canvas_ctx(map) {
        return None;
    }
    match field {
        "fillStyle" | "strokeStyle" | "globalAlpha" | "lineWidth" | "font" => {
            if let Some(v) = map.get(field) {
                return Some(v.clone());
            }
            Some(match field {
                "fillStyle" => Value::String("#000000".into()),
                "strokeStyle" => Value::String("#000000".into()),
                "globalAlpha" => Value::Float(1.0),
                "lineWidth" => Value::Float(1.0),
                "font" => Value::String("16px sans-serif".into()),
                _ => unreachable!(),
            })
        }
        _ => None,
    }
}

/// Apply `ctx.prop = value` for known canvas style properties. Returns `true` if handled.
pub fn try_write_property(
    map: &mut HashMap<String, Value>,
    field: &str,
    val: &Value,
) -> Result<bool, String> {
    if !is_canvas_ctx(map) {
        return Ok(false);
    }
    let native_id = native_id_from_ctx(map)?;
    let host_id = host_ctx_id(map);

    match field {
        "fillStyle" => {
            let color = expect_str(val, "fillStyle")?;
            canvas2d::set_fill_style(native_id, &color)?;
            if let Some(hid) = host_id {
                canvas_host::sync_fill_style(hid, &color)?;
            }
            map.insert(field.into(), Value::String(color));
            Ok(true)
        }
        "strokeStyle" => {
            let color = expect_str(val, "strokeStyle")?;
            canvas2d::set_stroke_style(native_id, &color)?;
            if let Some(hid) = host_id {
                canvas_host::sync_stroke_style(hid, &color)?;
            }
            map.insert(field.into(), Value::String(color));
            Ok(true)
        }
        "globalAlpha" => {
            let alpha = expect_f64(val, "globalAlpha")?;
            canvas2d::set_global_alpha(native_id, alpha)?;
            if let Some(hid) = host_id {
                canvas_host::sync_global_alpha(hid, alpha)?;
            }
            map.insert(field.into(), val.clone());
            Ok(true)
        }
        "lineWidth" => {
            let w = expect_f64(val, "lineWidth")?;
            canvas2d::set_line_width(native_id, w)?;
            if let Some(hid) = host_id {
                canvas_host::sync_line_width(hid, w)?;
            }
            map.insert(field.into(), val.clone());
            Ok(true)
        }
        "font" => {
            let spec = expect_str(val, "font")?;
            canvas2d::set_font(native_id, &spec)?;
            if let Some(hid) = host_id {
                canvas_host::sync_font(hid, &spec)?;
            }
            map.insert(field.into(), Value::String(spec));
            Ok(true)
        }
        _ => Ok(false),
    }
}
