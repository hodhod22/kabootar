use super::snippets::{util_by_id, UTILS};
use std::collections::HashSet;

/// Merge multiple utilities into one source file (dedupe imports).
pub fn compose(ids: &[&str]) -> Result<String, String> {
    if ids.is_empty() {
        return Err("compose requires at least one utility id".into());
    }

    let mut imports: Vec<String> = Vec::new();
    let mut seen_imports = HashSet::new();
    let mut bodies: Vec<String> = Vec::new();

    for id in ids {
        let util = util_by_id(id).ok_or_else(|| format!("unknown utility: {id}"))?;
        let (imps, body) = split_imports(util.code);
        for imp in imps {
            if seen_imports.insert(imp.clone()) {
                imports.push(imp);
            }
        }
        if !body.trim().is_empty() {
            bodies.push(body.trim().to_string());
        }
    }

    let had_imports = !imports.is_empty();
    let mut out = String::new();
    for imp in imports {
        out.push_str(&imp);
        if !imp.ends_with('\n') {
            out.push('\n');
        }
    }
    if had_imports && !bodies.is_empty() {
        out.push('\n');
    }
    out.push_str(&bodies.join("\n\n"));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn split_imports(code: &str) -> (Vec<String>, String) {
    let mut imports = Vec::new();
    let mut body_lines = Vec::new();

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            imports.push(trimmed.to_string());
        } else {
            body_lines.push(line);
        }
    }

    (imports, body_lines.join("\n"))
}

/// Complete partial input: match utility id prefix or keyword in code.
pub fn complete(partial: &str) -> Option<String> {
    let p = partial.trim().to_lowercase();
    if p.is_empty() {
        return None;
    }

    if let Some(util) = util_by_id(&p) {
        return Some(util.code.to_string());
    }

    let matches: Vec<_> = UTILS
        .iter()
        .filter(|u| u.id.starts_with(&p) || u.id.contains(&p))
        .collect();

    if matches.len() == 1 {
        return Some(matches[0].code.to_string());
    }

    // Keyword in partial code → best utility
    let hits = super::search::suggest(partial, 1);
    if let Some(hit) = hits.first() {
        if hit.score >= 15 {
            return util_by_id(&hit.id).map(|u| u.code.to_string());
        }
    }

    None
}

/// Explain Kabootar code with simple pattern matching.
pub fn explain(code: &str) -> String {
    let lower = code.to_lowercase();
    let mut parts: Vec<String> = Vec::new();

    if lower.contains("import \"http\"") || lower.contains("http_route") {
        parts.push("HTTP-modul: registrerar routes och svarar med http_response/ok.".into());
    }
    if lower.contains("import \"sql\"") || lower.contains("sql_exec") || lower.contains("sql_query") {
        parts.push("SQL: kör DDL/DML eller SELECT mot inbäddad databas.".into());
    }
    if lower.contains("import \"science\"") {
        parts.push("Science: fysik, statistik, matriser och numerisk analys.".into());
    }
    if lower.contains("pub fn") {
        parts.push("Exporterar funktion från filmodul (synlig vid import).".into());
    }
    if lower.contains("pub let") {
        parts.push("Exporterar konstant från filmodul.".into());
    }
    if lower.contains("class ") {
        parts.push("Klass med self-fält och metoder.".into());
    }
    if lower.contains("http_serve") {
        parts.push("Startar HTTP-server på angiven port.".into());
    }
    if lower.contains("stat_") {
        parts.push("Statistik-funktion från science-modulen.".into());
    }
    if lower.contains("mat_") {
        parts.push("Matrisoperation från science-modulen.".into());
    }

    if parts.is_empty() {
        "Ingen specifik utility-match — använd code_suggest() för att hitta rätt byggblock.".into()
    } else {
        parts.join(" ")
    }
}
