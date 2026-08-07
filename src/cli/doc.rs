//! `kabootar doc` — extract `///` docs from `.kab` into Markdown.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DocItem {
    pub name: String,
    pub kind: String,
    pub docs: String,
    pub file: String,
}

pub fn extract_kab_docs(source: &str, file: &str) -> Vec<DocItem> {
    let mut items = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("///") {
            pending.push(rest.trim().to_string());
            continue;
        }
        if pending.is_empty() {
            continue;
        }
        if let Some(name) = parse_decl_name(t) {
            items.push(DocItem {
                name,
                kind: if t.contains("fn ") {
                    "fn".into()
                } else if t.contains("class ") {
                    "class".into()
                } else {
                    "decl".into()
                },
                docs: pending.join("\n"),
                file: file.to_string(),
            });
        }
        pending.clear();
    }
    items
}

fn parse_decl_name(t: &str) -> Option<String> {
    let t = t.trim();
    // pub fn name(  |  fn name(
    for prefix in ["pub fn ", "fn ", "pub class ", "class "] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

pub fn render_markdown(title: &str, items: &[DocItem]) -> String {
    let mut out = format!("# {title}\n\n");
    if items.is_empty() {
        out.push_str("_No `///` documentation found._\n");
        return out;
    }
    let mut by_file: std::collections::BTreeMap<String, Vec<&DocItem>> =
        std::collections::BTreeMap::new();
    for it in items.iter() {
        by_file.entry(it.file.clone()).or_default().push(it);
    }
    for (file, list) in by_file {
        out.push_str(&format!("## `{file}`\n\n"));
        for it in list.iter() {
            out.push_str(&format!("### {} `{}`\n\n{}\n\n", it.kind, it.name, it.docs));
        }
    }
    out
}

pub fn collect_kab_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("kab") {
            out.push(path.to_path_buf());
        }
        return Ok(out);
    }
    if !path.is_dir() {
        return Err(format!("not a file or directory: {}", path.display()));
    }
    collect_dir(path, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for ent in fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))? {
        let ent = ent.map_err(|e| format!("read_dir entry: {e}"))?;
        let p = ent.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "target" || name.starts_with('.') || name.starts_with("target-") {
                continue;
            }
            collect_dir(&p, out)?;
        } else if p.extension().and_then(|e| e.to_str()) == Some("kab") {
            out.push(p);
        }
    }
    Ok(())
}

pub fn generate_docs(path: &Path) -> Result<(usize, String), String> {
    let files = collect_kab_files(path)?;
    let mut items = Vec::new();
    for f in &files {
        let src = fs::read_to_string(f).map_err(|e| format!("read {}: {e}", f.display()))?;
        let rel = f.to_string_lossy().replace('\\', "/");
        items.extend(extract_kab_docs(&src, &rel));
    }
    let title = format!("Kabootar API — {}", path.display());
    Ok((items.len(), render_markdown(&title, &items)))
}
