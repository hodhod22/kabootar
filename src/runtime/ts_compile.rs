//! TypeScript → Kabootar transpiler (Deno våg 16).

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsDiagnostic {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct TsCompileOptions {
    pub strip_enums: bool,
}

fn line_number(source: &str, pos: usize) -> usize {
    source[..pos.min(source.len())].matches('\n').count() + 1
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_part(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn read_ident(bytes: &[u8], mut i: usize) -> (String, usize) {
    let start = i;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if is_ident_part(ch) {
            i += 1;
        } else {
            break;
        }
    }
    let s = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
    (s.to_string(), i)
}

fn starts_with_word(bytes: &[u8], i: usize, word: &str) -> bool {
    if i + word.len() > bytes.len() {
        return false;
    }
    if &bytes[i..i + word.len()] != word.as_bytes() {
        return false;
    }
    if i > 0 {
        let prev = bytes[i - 1] as char;
        if is_ident_part(prev) {
            return false;
        }
    }
    let after = i + word.len();
    if after < bytes.len() {
        let next = bytes[after] as char;
        if is_ident_part(next) {
            return false;
        }
    }
    true
}

fn skip_line(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

fn skip_string(bytes: &[u8], mut i: usize, quote: u8) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn skip_template(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            return i + 1;
        }
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            i += 2;
            let mut depth = 1i32;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    b'"' | b'\'' => i = skip_string(bytes, i, bytes[i]),
                    b'`' => i = skip_template(bytes, i),
                    _ => i += 1,
                }
            }
            continue;
        }
        i += 1;
    }
    bytes.len()
}

fn match_balanced(bytes: &[u8], mut i: usize, open: u8, close: u8) -> Option<usize> {
    if i >= bytes.len() || bytes[i] != open {
        return None;
    }
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = skip_string(bytes, i, b'"'),
            b'\'' => i = skip_string(bytes, i, b'\''),
            b'`' => i = skip_template(bytes, i),
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => i = skip_line(bytes, i),
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => i = skip_block_comment(bytes, i),
            c if c == open => {
                depth += 1;
                i += 1;
            }
            c if c == close => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

fn remove_ts_declaration_blocks(source: &str, diagnostics: &mut Vec<TsDiagnostic>) -> String {
    let bytes = source.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i;
            i = skip_string(bytes, i, b'"');
            out.push_str(&source[start..i]);
            continue;
        }
        if bytes[i] == b'\'' {
            let start = i;
            i = skip_string(bytes, i, b'\'');
            out.push_str(&source[start..i]);
            continue;
        }
        if bytes[i] == b'`' {
            let start = i;
            i = skip_template(bytes, i);
            out.push_str(&source[start..i]);
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let start = i;
            i = skip_line(bytes, i);
            out.push_str(&source[start..i]);
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let start = i;
            i = skip_block_comment(bytes, i);
            out.push_str(&source[start..i]);
            continue;
        }

        let line_start = i;
        let ws = skip_ws(bytes, i);
        let kw = [
            ("export interface", "removed export interface"),
            ("export type", "removed export type"),
            ("interface", "removed interface"),
            ("type", "removed type alias"),
            ("declare", "removed declare block"),
            ("namespace", "removed namespace"),
        ];
        let mut removed = false;
        for (word, msg) in kw {
            if starts_with_word(bytes, ws, word) {
                let mut j = ws + word.len();
                j = skip_ws(bytes, j);
                let (name, after_name) = read_ident(bytes, j);
                j = after_name;
                if bytes.get(j) == Some(&b'<') {
                    if let Some(end) = match_balanced(bytes, j, b'<', b'>') {
                        j = end;
                    }
                }
                j = skip_ws(bytes, j);
                if bytes.get(j) == Some(&b'{') {
                    if let Some(end) = match_balanced(bytes, j, b'{', b'}') {
                        diagnostics.push(TsDiagnostic {
                            line: line_number(source, line_start),
                            message: format!("{msg} `{name}`"),
                        });
                        i = end;
                        removed = true;
                        break;
                    }
                } else if bytes.get(j) == Some(&b'=') {
                    let semi = source[j..].find(';').map(|p| j + p + 1).unwrap_or(bytes.len());
                    diagnostics.push(TsDiagnostic {
                        line: line_number(source, line_start),
                        message: format!("{msg} `{name}`"),
                    });
                    i = semi;
                    removed = true;
                    break;
                }
            }
        }
        if removed {
            if i < bytes.len() && bytes[i] == b'\n' {
                out.push('\n');
                i += 1;
            }
            continue;
        }

        if starts_with_word(bytes, ws, "enum") {
            let mut j = ws + 4;
            j = skip_ws(bytes, j);
            let (_name, after_name) = read_ident(bytes, j);
            j = after_name;
            j = skip_ws(bytes, j);
            if bytes.get(j) == Some(&b'{') {
                if let Some(end) = match_balanced(bytes, j, b'{', b'}') {
                    let converted = convert_enum_block(&source[ws..end]);
                    if !converted.is_empty() {
                        out.push_str(&converted);
                        out.push('\n');
                    }
                    diagnostics.push(TsDiagnostic {
                        line: line_number(source, line_start),
                        message: "converted enum to object literal".into(),
                    });
                    i = end;
                    if i < bytes.len() && bytes[i] == b'\n' {
                        i += 1;
                    }
                    continue;
                }
            }
        }

        if starts_with_word(bytes, ws, "import") {
            let after = skip_ws(bytes, ws + 6);
            if starts_with_word(bytes, after, "type") {
                let semi = source[ws..].find(';').map(|p| ws + p + 1).unwrap_or(bytes.len());
                diagnostics.push(TsDiagnostic {
                    line: line_number(source, line_start),
                    message: "removed import type".into(),
                });
                i = semi;
                if i < bytes.len() && bytes[i] == b'\n' {
                    out.push('\n');
                    i += 1;
                }
                continue;
            }
        }

        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn convert_enum_block(block: &str) -> String {
    let trimmed = block.trim_start();
    let after_enum = trimmed.strip_prefix("enum").unwrap_or(trimmed).trim_start();
    let (name, rest) = after_enum
        .split_once('{')
        .unwrap_or((after_enum, ""));
    let name = name.trim();
    let inner = rest.trim_end_matches('}').trim();
    if inner.is_empty() {
        return format!("let {name} = {{}}");
    }
    let mut entries = Vec::new();
    let mut idx = 0i64;
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            entries.push(format!("{key}: {val}"));
            if let Ok(n) = val.parse::<i64>() {
                idx = n + 1;
            }
        } else {
            entries.push(format!("{part}: {idx}"));
            idx += 1;
        }
    }
    format!("let {name} = {{ {} }}", entries.join(", "))
}

fn skip_type_expr(bytes: &[u8], mut i: usize) -> usize {
    let mut angle = 0i32;
    let mut paren = 0i32;
    let mut bracket = 0i32;
    let mut brace = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = skip_string(bytes, i, b'"'),
            b'\'' => i = skip_string(bytes, i, b'\''),
            b'`' => i = skip_template(bytes, i),
            b'<' => {
                angle += 1;
                i += 1;
            }
            b'>' => {
                angle -= 1;
                i += 1;
                if angle <= 0 && paren == 0 && bracket == 0 && brace == 0 {
                    return i;
                }
            }
            b'(' => {
                paren += 1;
                i += 1;
            }
            b')' => {
                paren -= 1;
                i += 1;
                if angle <= 0 && paren <= 0 && bracket == 0 && brace == 0 {
                    return i;
                }
            }
            b'[' => {
                bracket += 1;
                i += 1;
            }
            b']' => {
                bracket -= 1;
                i += 1;
                if angle <= 0 && paren <= 0 && bracket <= 0 && brace == 0 {
                    return i;
                }
            }
            b'{' => {
                brace += 1;
                i += 1;
            }
            b'}' => {
                brace -= 1;
                i += 1;
                if angle <= 0 && paren <= 0 && bracket <= 0 && brace <= 0 {
                    return i;
                }
            }
            b',' if angle <= 0 && paren <= 0 && bracket <= 0 && brace <= 0 => return i,
            b'=' if angle <= 0 && paren <= 0 && bracket <= 0 && brace <= 0 => return i,
            b';' if angle <= 0 && paren <= 0 && bracket <= 0 && brace <= 0 => return i,
            _ => i += 1,
        }
    }
    i
}

const MODIFIER_WORDS: &[&str] = &[
    "public", "private", "protected", "readonly", "abstract", "override", "declare",
];

fn erase_inline_types(source: &str, diagnostics: &mut Vec<TsDiagnostic>) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i;
            i = skip_string(bytes, i, b'"');
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }
        if bytes[i] == b'\'' {
            let start = i;
            i = skip_string(bytes, i, b'\'');
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }
        if bytes[i] == b'`' {
            let start = i;
            i = skip_template(bytes, i);
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let start = i;
            i = skip_line(bytes, i);
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let start = i;
            i = skip_block_comment(bytes, i);
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }

        let mut skipped_modifier = false;
        for word in MODIFIER_WORDS {
            if starts_with_word(bytes, i, word) {
                i += word.len();
                i = skip_ws(bytes, i);
                skipped_modifier = true;
                break;
            }
        }
        if skipped_modifier {
            continue;
        }

        if bytes[i] == b'<' && i > 0 {
            let prev = bytes[i - 1] as char;
            if is_ident_part(prev) || prev == ')' {
                let end = match_balanced(bytes, i, b'<', b'>');
                if let Some(end) = end {
                    diagnostics.push(TsDiagnostic {
                        line: line_number(source, i),
                        message: "removed generic type parameters".into(),
                    });
                    i = end;
                    continue;
                }
            }
        }

        if bytes[i] == b':' && i > 0 {
            let prev = bytes[i - 1] as char;
            if is_ident_part(prev) || prev == ')' || prev == ']' {
                let next_i = skip_ws(bytes, i + 1);
                if bytes.get(next_i).map(|c| c.is_ascii_digit()).unwrap_or(false)
                    || bytes.get(next_i) == Some(&b'"')
                    || bytes.get(next_i) == Some(&b'\'')
                {
                    out.push(bytes[i]);
                    i += 1;
                    continue;
                }
                if starts_with_word(bytes, next_i, "string")
                    || starts_with_word(bytes, next_i, "number")
                    || starts_with_word(bytes, next_i, "boolean")
                    || starts_with_word(bytes, next_i, "void")
                    || starts_with_word(bytes, next_i, "any")
                    || starts_with_word(bytes, next_i, "unknown")
                    || starts_with_word(bytes, next_i, "never")
                    || bytes.get(next_i) == Some(&b'{')
                    || bytes.get(next_i).map(|c| *c as char).map(is_ident_start).unwrap_or(false)
                {
                    let end = skip_type_expr(bytes, next_i);
                    diagnostics.push(TsDiagnostic {
                        line: line_number(source, i),
                        message: "removed type annotation".into(),
                    });
                    i = end;
                    continue;
                }
            }
        }

        if starts_with_word(bytes, i, "as") {
            let next_i = skip_ws(bytes, i + 2);
            if bytes.get(next_i).map(|c| *c as char).map(is_ident_start).unwrap_or(false)
                || bytes.get(next_i) == Some(&b'{')
            {
                let end = skip_type_expr(bytes, next_i);
                diagnostics.push(TsDiagnostic {
                    line: line_number(source, i),
                    message: "removed type assertion".into(),
                });
                i = end;
                continue;
            }
        }

        if starts_with_word(bytes, i, "implements") {
            let end = skip_type_expr(bytes, i + 10);
            diagnostics.push(TsDiagnostic {
                line: line_number(source, i),
                message: "removed implements clause".into(),
            });
            i = skip_ws(bytes, end);
            continue;
        }

        if starts_with_word(bytes, i, "satisfies") {
            let next_i = skip_ws(bytes, i + 9);
            let end = skip_type_expr(bytes, next_i);
            diagnostics.push(TsDiagnostic {
                line: line_number(source, i),
                message: "removed satisfies expression".into(),
            });
            i = end;
            continue;
        }

        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| source.to_string())
}

fn collapse_blank_lines(text: &str) -> String {
    let mut out = Vec::new();
    let mut blank_run = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push(String::new());
            }
        } else {
            blank_run = 0;
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

pub fn compile_to_kabootar(source: &str, _options: &TsCompileOptions) -> (String, Vec<TsDiagnostic>) {
    let mut diagnostics = Vec::new();
    let pass1 = remove_ts_declaration_blocks(source, &mut diagnostics);
    let pass2 = erase_inline_types(&pass1, &mut diagnostics);
    let code = collapse_blank_lines(&pass2);
    (code, diagnostics)
}

pub fn diagnostics_to_values(diags: &[TsDiagnostic]) -> Vec<crate::value::Value> {
    diags
        .iter()
        .map(|d| {
            let mut m = HashMap::new();
            m.insert("line".into(), crate::value::Value::Number(d.line as i64));
            m.insert(
                "message".into(),
                crate::value::Value::String(d.message.clone()),
            );
            crate::value::Value::Object(m)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_variable_type() {
        let (code, _) = compile_to_kabootar("let x: number = 1", &TsCompileOptions::default());
        assert!(!code.contains(": number"));
        assert!(code.contains("let x"));
    }

    #[test]
    fn removes_interface_block() {
        let src = "interface User { name: string }\nlet x = 1";
        let (code, diags) = compile_to_kabootar(src, &TsCompileOptions::default());
        assert!(!code.contains("interface"));
        assert!(code.contains("let x = 1"));
        assert!(!diags.is_empty());
    }

    #[test]
    fn converts_numeric_enum() {
        let src = "enum Color { Red, Green, Blue }\nlet c = Color.Red";
        let (code, _) = compile_to_kabootar(src, &TsCompileOptions::default());
        assert!(code.contains("let Color ="));
        assert!(code.contains("Red: 0"));
        assert!(code.contains("let c = Color.Red"));
    }

    #[test]
    fn strips_generics_and_modifiers() {
        let src = "class Box<T> { private value: T }";
        let (code, _) = compile_to_kabootar(src, &TsCompileOptions::default());
        assert!(!code.contains("<T>"));
        assert!(!code.contains("private"));
        assert!(!code.contains(": T"));
    }
}
