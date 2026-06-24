//! URI encoding — parity with JS `encodeURI` / `decodeURI`.

use crate::value::{Environment, Value};

fn str_arg(v: &Value) -> Result<&str, String> {
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err("expected string".into()),
    }
}

fn is_uri_unreserved(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~')
}

fn is_uri_reserved(c: char) -> bool {
    matches!(c, ';' | ',' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | '#')
}

fn is_component_unreserved(c: char) -> bool {
    is_uri_unreserved(c) || matches!(c, '!' | '*' | '\'' | '(' | ')' )
}

fn percent_encode_byte(b: u8) -> String {
    format!("%{:02X}", b)
}

pub fn encode_uri_component(s: &str) -> String {
    encode(s, true)
}

pub fn decode_uri_component(s: &str) -> Result<String, String> {
    decode(s)
}

fn encode(s: &str, component: bool) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let keep = if component {
            is_component_unreserved(ch)
        } else {
            is_uri_unreserved(ch) || is_uri_reserved(ch) || ch == '#'
        };
        if keep && ch.is_ascii() {
            out.push(ch);
        } else {
            for b in ch.to_string().as_bytes() {
                out.push_str(&percent_encode_byte(*b));
            }
        }
    }
    out
}

fn decode(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("invalid percent-encoding in URI".into());
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| "invalid percent-encoding in URI".to_string())?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| "invalid percent-encoding in URI".to_string())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "invalid UTF-8 in decoded URI".into())
}

fn encode_uri_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("encode_uri(s)")?)?;
    Ok(Value::String(encode(s, false)))
}

fn decode_uri_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("decode_uri(s)")?)?;
    Ok(Value::String(decode(s)?))
}

fn encode_uri_component_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("encode_uri_component(s)")?)?;
    Ok(Value::String(encode(s, true)))
}

fn decode_uri_component_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("decode_uri_component(s)")?)?;
    Ok(Value::String(decode(s)?))
}

pub fn register_encoding(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("encode_uri", encode_uri_native),
        ("decode_uri", decode_uri_native),
        ("encode_uri_component", encode_uri_component_native),
        ("decode_uri_component", decode_uri_component_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
