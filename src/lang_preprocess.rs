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

/// File-level directives captured before stripping (ownership / effects).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleDirectives {
    /// `@manual` — opt-in systems memory (move/drop). Default is GC.
    pub manual: bool,
    /// Explicit `@gc` in header (documentation / future).
    pub gc: bool,
}

/// Memory mode for a compiled module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryMode {
    #[default]
    Gc,
    Manual,
}

impl MemoryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryMode::Gc => "gc",
            MemoryMode::Manual => "manual",
        }
    }
}

impl ModuleDirectives {
    pub fn memory_mode(&self) -> MemoryMode {
        if self.manual {
            MemoryMode::Manual
        } else {
            MemoryMode::Gc
        }
    }
}

/// Full source transform pipeline (safe sugar only).
pub fn preprocess(source: &str) -> String {
    preprocess_with_meta(source).0
}

/// Preprocess and retain header directive metadata (`@manual`, `@gc`, …).
pub fn preprocess_with_meta(source: &str) -> (String, ModuleDirectives) {
    let meta = scan_header_directives(source);
    let s = strip_header_directives(source);
    let s = expand_comptime_keyword(&s);
    let s = expand_html_blocks(&s);
    let s = expand_actor_declarations(&s);
    (s, meta)
}

/// Scan only the module header for effect directives (before first real statement).
pub fn scan_header_directives(source: &str) -> ModuleDirectives {
    let mut meta = ModuleDirectives::default();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
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
            if trimmed == "@manual"
                || trimmed.starts_with("@manual ")
            {
                meta.manual = true;
            }
            if trimmed == "@gc" || trimmed.starts_with("@gc ") {
                meta.gc = true;
            }
            continue;
        }
        break;
    }
    meta
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
/// `comptime { ... }` → `{ ... }` (compile-time blocks run as normal const code today).
pub fn expand_comptime_keyword(source: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        // Kolla om vi har "comptime" här
        if i + 8 <= chars.len()
            && chars[i] == 'c'
            && chars[i+1] == 'o'
            && chars[i+2] == 'm'
            && chars[i+3] == 'p'
            && chars[i+4] == 't'
            && chars[i+5] == 'i'
            && chars[i+6] == 'm'
            && chars[i+7] == 'e'
        {
            let mut k = i + 8;
            
            // Hoppa över whitespace
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            
            // Kolla om nästa tecken är '{'
            if k < chars.len() && chars[k] == '{' {
                // Hitta matchande '}'
                let mut depth = 1;
                let mut end = k + 1;
                while end < chars.len() && depth > 0 {
                    if chars[end] == '{' { depth += 1; }
                    else if chars[end] == '}' { depth -= 1; }
                    end += 1;
                }
                
                if depth == 0 {
                    // Vi hoppar över "comptime" och allt fram till '{'
                    // Vill du behålla '{' och '}'? 
                    // Om ja, lägg till '{' + body + '}'
                    let body: String = chars[k+1..end-1].iter().collect();
                    out.push('{');
                    out.push_str(&body);
                    out.push('}');
                    i = end;
                    continue;
                }
            }
        }
        
        // Lägg till aktuellt tecken
        out.push(chars[i]);
        i += 1;
    }
    
    out
}
/// `html! { <tag>...</tag> }` → `kv8_create()` + `kv8_run_ui`.
pub fn expand_html_blocks(source: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < source.len() {
        if source[i..].starts_with("html!") {
            let after_kw = i + 5;
            let mut k = after_kw;
            while k < source.len() && source.as_bytes()[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < source.len() && source.as_bytes()[k] == b'{' {
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
        }
        let ch = source[i..].chars().next().expect("valid utf-8");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `actor Name { }` → `let Name_actor = actor_spawn("Name");`
pub fn expand_actor_declarations(source: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let chars: Vec<char> = source.chars().collect();
    
    while i < chars.len() {
        // Kolla om vi har "actor" här
        if i + 4 < chars.len() 
            && chars[i] == 'a'
            && chars[i+1] == 'c'
            && chars[i+2] == 't'
            && chars[i+3] == 'o'
            && chars[i+4] == 'r' 
        {
            let _start = i;
            let mut k = i + 5; // efter "actor"
            
            // Hoppa över whitespace
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            
            // Läs namnet
            let name_start = k;
            while k < chars.len() && (chars[k].is_alphanumeric() || chars[k] == '_') {
                k += 1;
            }
            
            if k > name_start {
                let name: String = chars[name_start..k].iter().collect();
                
                // Hoppa över whitespace efter namnet
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                
                if k < chars.len() && chars[k] == '{' {
                    // Hitta matchande '}'
                    let mut depth = 1;
                    let mut end = k + 1;
                    while end < chars.len() && depth > 0 {
                        if chars[end] == '{' { depth += 1; }
                        else if chars[end] == '}' { depth -= 1; }
                        end += 1;
                    }
                    
                    if depth == 0 {
                        // Body är mellan '{' och '}'
                        let body_start = k + 1;
                        let body_end = end - 1;
                        let body: String = chars[body_start..body_end].iter().collect();
                        
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
                        i = end;
                        continue;
                    }
                }
            }
        }
        
        out.push(chars[i]);
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
    fn preprocess_with_meta_captures_manual() {
        let (out, meta) = preprocess_with_meta("@version \"1.0.0\"\n@manual\nlet x = 1");
        assert!(meta.manual);
        assert!(!out.contains("@manual"));
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
