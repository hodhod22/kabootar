//! Module metadata — `@version` directive in `.kab` files.

pub fn strip_version_directive(source: &str) -> (Option<String>, String) {
    let mut version = None;
    let mut out_lines = Vec::new();
    let mut past_header = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if !past_header {
            if trimmed.is_empty() || trimmed.starts_with("//") {
                out_lines.push(line);
                continue;
            }
            if let Some(v) = parse_version_line(trimmed) {
                version = Some(v);
                continue;
            }
            past_header = true;
        }
        out_lines.push(line);
    }

    let body = if out_lines.is_empty() {
        String::new()
    } else {
        out_lines.join("\n")
    };
    (version, body)
}

fn parse_version_line(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("@version ") {
        return Some(rest.trim().trim_matches('"').to_string());
    }
    if let Some(rest) = line.strip_prefix("# kabootar-version:") {
        return Some(rest.trim().trim_matches('"').to_string());
    }
    None
}

/// Split `import "mod@1.0"` into (`mod`, Some(`1.0`)).
pub fn split_import_spec(name: &str) -> (String, Option<String>) {
    if let Some((module, ver)) = name.rsplit_once('@') {
        if !module.is_empty() && !ver.is_empty() {
            return (module.to_string(), Some(ver.to_string()));
        }
    }
    (name.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_at_version() {
        let (ver, body) = strip_version_directive("@version \"1.0.0\"\npub fn x() {}");
        assert_eq!(ver.as_deref(), Some("1.0.0"));
        assert!(body.contains("pub fn x"));
    }

    #[test]
    fn splits_import_version() {
        let (m, v) = split_import_spec("greet@1.0");
        assert_eq!(m, "greet");
        assert_eq!(v.as_deref(), Some("1.0"));
    }
}
