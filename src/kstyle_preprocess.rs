//! Expand `kstyle { ... }` blocks into native calls (KSS in Kabootar syntax).

use std::collections::HashMap;

/// Transform `kstyle { .sel { prop: val; } }` into `kstyle_rule(...); kstyle_commit();`
pub fn expand_kstyle_blocks(source: &str) -> String {
    let mut out = String::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < source.len() {
        if let Some(rest) = source[i..].strip_prefix("kstyle") {
            let after_kw = i + 6;
            let mut k = after_kw;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'{' {
                if let Some((rules, end)) = parse_kstyle_block(&source[k..]) {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("kstyle_reset();\n");
                    for (sel, decls) in rules {
                        for (prop, val) in decls {
                            out.push_str(&format!(
                                "kstyle_rule(\"{}\", \"{}\", \"{}\");\n",
                                escape_str(&sel),
                                escape_str(&prop),
                                escape_str(&val),
                            ));
                        }
                    }
                    out.push_str("kstyle_commit();\n");
                    i = k + end;
                    continue;
                }
            }
            let _ = rest;
        }
        if let Some(ch) = source[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_kstyle_block(input: &str) -> Option<(Vec<(String, HashMap<String, String>)>, usize)> {
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
    let inner = &input[1..end - 1];
    Some((parse_rules(inner), end))
}

fn parse_rules(inner: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut rules = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let sel_start = i;
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let selector = String::from_utf8_lossy(&bytes[sel_start..i]).trim().to_string();
        i += 1;
        let decl_start = i;
        let mut depth = 1i32;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                i += 1;
            }
        }
        let decls_src = String::from_utf8_lossy(&bytes[decl_start..i]).to_string();
        i += 1;
        if !selector.is_empty() {
            rules.push((selector, parse_decls(&decls_src)));
        }
    }
    rules
}

fn parse_decls(input: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for part in input.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once(':') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_kstyle_block() {
        let src = r#"
kstyle {
  .app {
    color: #fff;
    padding: 16px;
  }
}
let x = 1;
"#;
        let out = expand_kstyle_blocks(src);
        assert!(out.contains("kstyle_rule(\".app\", \"color\", \"#fff\")"));
        assert!(out.contains("kstyle_commit()"));
        assert!(out.contains("let x = 1"));
    }

    #[test]
    fn preserves_utf8_outside_kstyle() {
        let src = "// note — em dash\nlet x = 1;\n";
        let out = expand_kstyle_blocks(src);
        assert_eq!(out, src);
    }
}
