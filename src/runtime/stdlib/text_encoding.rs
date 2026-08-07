//! `TextEncoder` / `TextDecoder` and `globalThis` helpers.

use crate::value::{Environment, Value};
use std::collections::HashMap;

fn text_encode_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.as_str(),
        Some(v) => return Err(format!("text_encode() expects string, got {:?}", v)),
        None => return Err("text_encode(text)".into()),
    };
    let bytes: Vec<Value> = s.bytes().map(|b| Value::Number(b as i64)).collect();
    Ok(Value::from_array(bytes))
}

fn text_decode_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arr = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("text_decode(bytes)".into()),
    };
    let mut bytes = Vec::with_capacity(arr.len());
    for v in arr.iter() {
        match v {
            Value::Number(n) if (0..=255).contains(n) => bytes.push(*n as u8),
            _ => return Err("text_decode() expects byte array (0..255)".into()),
        }
    }
    let s = String::from_utf8(bytes).map_err(|_| "text_decode() invalid UTF-8".to_string())?;
    Ok(Value::String(s))
}

fn global_this_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut map = HashMap::new();
    for name in env.all_binding_names() {
        if let Some(v) = env.get(&name) {
            map.insert(name, v);
        }
    }
    Ok(Value::from_object(map))
}

fn btoa_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.as_str(),
        Some(v) => return Err(format!("btoa() expects string, got {:?}", v)),
        None => return Err("btoa(string)".into()),
    };
    Ok(Value::String(super::base64::btoa_string(s)?))
}

fn atob_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.as_str(),
        Some(v) => return Err(format!("atob() expects string, got {:?}", v)),
        None => return Err("atob(string)".into()),
    };
    Ok(Value::String(super::base64::atob_string(s)?))
}

pub fn register_text_encoding(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("text_encode", text_encode_native),
        ("text_decoder_decode", text_decode_native),
        ("text_decode", text_decode_native),
        ("btoa", btoa_native),
        ("atob", atob_native),
        ("global_this", global_this_native),
        ("globalThis", global_this_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
