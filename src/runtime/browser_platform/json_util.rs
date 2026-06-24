//! Minimal JSON field extraction (no serde dependency).

pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

pub fn json_string(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

pub fn extract_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')? + 1;
    let tail = rest[colon..].trim_start();
    if tail.starts_with('"') {
        let inner = &tail[1..];
        let end = inner.find('"')?;
        return Some(inner[..end].to_string());
    }
    None
}

pub fn extract_array_strings(json: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let Some(pos) = json.find(&needle) else {
        return Vec::new();
    };
    let rest = &json[pos + needle.len()..];
    let Some(start) = rest.find('[') else {
        return Vec::new();
    };
    let inner = &rest[start + 1..];
    let Some(end) = inner.find(']') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for part in inner[..end].split(',') {
        let part = part.trim();
        if let Some(s) = part.strip_prefix('"').and_then(|p| p.strip_suffix('"')) {
            out.push(s.to_string());
        }
    }
    out
}
