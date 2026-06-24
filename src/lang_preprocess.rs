//! Kabootar language sugar — directives, `html!`, `comptime`, `actor`.

use std::collections::HashMap;

const EFFECT_DIRECTIVES: &[&str] = &[
    "@pure",
    "@io",
    "@disk",
    "@network",
    "@gc",
    "@manual",
    "@simd",
    "@benchmark",
    "@packed",
];

/// Full source transform pipeline (safe sugar only).
pub fn preprocess(source: &str) -> String {
    let s = strip_header_directives(source);
    let s = expand_comptime_keyword(&s);
    let s = expand_html_blocks(&s);
    let s = expand_actor_declarations(&s);
    s
}

/// Strip effect/decorator lines from the module header (like `@version`).
pub fn strip_header_directives(source: &str) -> String {
    let mut out = Vec::new();
    let mut past_header = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if !past_header {
            if trimmed.is_empty() || trimmed.starts_with("//") {
                out.push(line.to_string());
                continue;
            }
            if trimmed.starts_with("@version ")
                || trimmed.starts_with("# kabootar-version:")
                || trimmed.starts_with("@persist ")
                || EFFECT_DIRECTIVES.iter().any(|d| {
                    trimmed.starts_with(d)
                        && (trimmed.len() == d.len()
                            || trimmed.as_bytes().get(d.len()) == Some(&b' '))
                })
            {
                if trimmed.starts_with("@persist let ") {
                    out.push(line.replace("@persist ", ""));
                }
                continue;
            }
            past_header = true;
        }
        if trimmed.starts_with("@persist let ") {
            out.push(line.replace("@persist ", ""));
            continue;
        }
        out.push(line.to_string());
    }

    if out.is_empty() {
        String::new()
    } else {
        out.join("\n")
    }
}

/// `comptime { ... }` → `{ ... }` (compile-time blocks run as normal const code today).
pub fn expand_comptime_keyword(source: &str) -> String {
    let mut out = String::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(rest) = source[i..].strip_prefix("comptime") {
            let j = i + 8;
            let mut k = j;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'{' {
                i = j;
                continue;
            }
            let _ = rest;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// `html! { <tag>...</tag> }` → `kv8_create()` + `kv8_run_ui`.
pub fn expand_html_blocks(source: &str) -> String {
    let mut out = String::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(rest) = source[i..].strip_prefix("html!") {
            let after_kw = i + 5;
            let mut k = after_kw;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'{' {
                if let Some((inner, end)) = parse_brace_block(&source[k..]) {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    let kml = escape_kml(inner.trim());
                    out.push_str("kv8_run_html(\"");
                    out.push_str(&escape_str(&kml));
                    out.push_str("\")");
                    i = k + end;
                    continue;
                }
            }
            let _ = rest;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// `actor Name { }` → `let Name_actor = actor_spawn("Name");`
pub fn expand_actor_declarations(source: &str) -> String {
    let mut out = String::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(rest) = source[i..].strip_prefix("actor") {
            let after_kw = i + 5;
            let mut k = after_kw;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            let name_start = k;
            while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
                k += 1;
            }
            if k > name_start {
                let name = &source[name_start..k];
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b'{' {
                    if let Some((body, end)) = parse_brace_block(&source[k..]) {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push_str(&format!(
                            "let {name}_actor = actor_spawn(\"{name}\");\n",
                            name = name
                        ));
                        let body = body.trim();
                        if !body.is_empty() {
                            out.push_str(body);
                            if !body.ends_with('\n') {
                                out.push('\n');
                            }
                        }
                        i = k + end;
                        continue;
                    }
                }
            }
            let _ = rest;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_brace_block(input: &str) -> Option<(String, usize)> {
    let input = input.trim_start();
    if !input.starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    let mut end = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = idx + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    Some((input[1..end - 1].to_string(), end))
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_kml(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

/// Collect stripped directive names for `lang_info()`.
pub fn scan_directives(source: &str) -> HashMap<String, bool> {
    let mut m = HashMap::new();
    for line in source.lines() {
        let t = line.trim();
        for d in EFFECT_DIRECTIVES {
            if t.starts_with(d) {
                m.insert(d.trim_start_matches('@').to_string(), true);
            }
        }
        if t.starts_with("@persist") {
            m.insert("persist".into(), true);
        }
        if t.contains("comptime") {
            m.insert("comptime".into(), true);
        }
        if t.contains("html!") {
            m.insert("html".into(), true);
        }
        if t.trim_start().starts_with("actor ") {
            m.insert("actor".into(), true);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_pure_and_io() {
        let out = strip_header_directives("@pure\n@io\nlet x = 1");
        assert!(!out.contains("@pure"));
        assert!(out.contains("let x = 1"));
    }

    #[test]
    fn expands_html_macro() {
        let src = r#"html! { <main>Hi</main> }"#;
        let out = expand_html_blocks(src);
        assert!(out.contains("kv8_run_html"));
        assert!(out.contains("<main>Hi</main>"));
        assert!(out.starts_with("kv8_run_html"));
    }

    #[test]
    fn expands_actor() {
        let src = "actor Worker { fn go() { return 1 } }";
        let out = expand_actor_declarations(src);
        assert!(out.contains("actor_spawn(\"Worker\")"));
        assert!(out.contains("fn go()"));
    }

    #[test]
    fn comptime_becomes_block() {
        let src = "comptime { let x = 1; x }";
        let out = expand_comptime_keyword(src);
        assert!(!out.contains("comptime"));
        assert!(out.contains("{ let x = 1"));
    }
}
