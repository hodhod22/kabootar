//! JSON parse/stringify for Kabootar `Value` (no external serde).

use crate::value::Value;
use std::collections::HashMap;

pub fn stringify(v: &Value) -> String {
    stringify_pretty(v, None)
}

pub fn stringify_pretty(v: &Value, indent: Option<usize>) -> String {
    match indent {
        Some(spaces) if spaces > 0 => pretty_value(v, spaces, 0),
        _ => compact_stringify(v),
    }
}

fn compact_stringify(v: &Value) -> String {
    match v {
        Value::Null | Value::Undefined => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "null".into()
            } else {
                f.to_string()
            }
        }
        Value::String(s) => format!("\"{}\"", escape(s)),
        Value::Array(items) => {
            let inner: Vec<_> = items.iter().map(compact_stringify).collect();
            format!("[{}]", inner.join(","))
        }
        Value::Object(map) => {
            let mut pairs: Vec<_> = map
                .iter()
                .filter(|(k, _)| !k.starts_with("__kab_"))
                .map(|(k, v)| format!("\"{}\":{}", escape(k), compact_stringify(v)))
                .collect();
            pairs.sort();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Option(Some(inner)) => compact_stringify(inner),
        Value::Option(None) => "null".into(),
        Value::Result(Ok(inner)) => compact_stringify(inner),
        Value::Result(Err(inner)) => format!(
            "{{\"Ok\":false,\"Err\":{}}}",
            compact_stringify(inner)
        ),
        _ => "null".into(),
    }
}

fn pretty_value(v: &Value, indent: usize, depth: usize) -> String {
    let pad = " ".repeat(indent * depth);
    let pad_inner = " ".repeat(indent * (depth + 1));
    match v {
        Value::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            let inner: Vec<_> = items
                .iter()
                .map(|item| format!("{pad_inner}{}", pretty_value(item, indent, depth + 1)))
                .collect();
            format!("[\n{}\n{pad}]", inner.join(",\n"))
        }
        Value::Object(map) => {
            let mut pairs: Vec<_> = map
                .iter()
                .filter(|(k, _)| !k.starts_with("__kab_"))
                .map(|(k, val)| {
                    format!(
                        "{pad_inner}\"{}\": {}",
                        escape(k),
                        pretty_value(val, indent, depth + 1)
                    )
                })
                .collect();
            pairs.sort();
            if pairs.is_empty() {
                return "{}".into();
            }
            format!("{{\n{}\n{pad}}}", pairs.join(",\n"))
        }
        other => compact_stringify(other),
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut p = JsonParser::new(input.trim());
    let v = p.parse_value()?;
    p.skip_ws();
    if !p.rest().is_empty() {
        return Err(format!("trailing JSON at offset {}", p.i));
    }
    Ok(v)
}

struct JsonParser<'a> {
    s: &'a str,
    bytes: &'a [u8],
    i: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s,
            bytes: s.as_bytes(),
            i: 0,
        }
    }

    fn rest(&self) -> &str {
        &self.s[self.i..]
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b't') => self.expect_literal("true").map(|_| Value::Bool(true)),
            Some(b'f') => self.expect_literal("false").map(|_| Value::Bool(false)),
            Some(b'n') => self.expect_literal("null").map(|_| Value::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(c) => Err(format!("unexpected JSON char '{c}' at {}", self.i)),
            None => Err("unexpected end of JSON".into()),
        }
    }

    fn expect_literal(&mut self, lit: &str) -> Result<(), String> {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            Ok(())
        } else {
            Err(format!("expected {lit} at {}", self.i))
        }
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.bump();
        self.skip_ws();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.bump() {
                Some(b']') => break,
                Some(b',') => self.skip_ws(),
                _ => return Err(format!("expected , or ] in array at {}", self.i)),
            }
        }
        Ok(Value::Array(items))
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.bump();
        self.skip_ws();
        let mut map = HashMap::new();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(format!("expected : in object at {}", self.i));
            }
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_ws();
            match self.bump() {
                Some(b'}') => break,
                Some(b',') => self.skip_ws(),
                _ => return Err(format!("expected , or }} in object at {}", self.i)),
            }
        }
        Ok(Value::Object(map))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.bump() != Some(b'"') {
            return Err(format!("expected string at {}", self.i));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => {
                        let hex = &self.s[self.i..self.i.saturating_add(4).min(self.s.len())];
                        if hex.len() < 4 {
                            return Err("invalid unicode escape".into());
                        }
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| "invalid unicode escape".to_string())?;
                        self.i += 4;
                        let ch = char::from_u32(code).ok_or("invalid unicode codepoint")?;
                        out.push(ch);
                    }
                    _ => return Err("invalid escape".into()),
                },
                Some(b) if b < 0x20 => return Err("control char in string".into()),
                Some(b) => out.push(b as char),
                None => return Err("unterminated string".into()),
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
            let slice = &self.s[start..self.i];
            let f: f64 = slice
                .parse()
                .map_err(|_| format!("invalid number at {start}"))?;
            return Ok(Value::Float(f));
        }
        let slice = &self.s[start..self.i];
        let n: i64 = slice
            .parse()
            .map_err(|_| format!("invalid integer at {start}"))?;
        Ok(Value::Number(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let v = parse(r#"{"a":1,"b":[true,null]}"#).unwrap();
        let s = stringify(&v);
        assert!(s.contains("\"a\":1"));
        let back = parse(&s).unwrap();
        assert_eq!(stringify(&v), stringify(&back));
    }
}
