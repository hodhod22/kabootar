//! Scan project directory to detect development progress.

use crate::project::manifest::load_manifest;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct KabFileInfo {
    pub path: String,
    pub lines: usize,
    pub bytes: usize,
    pub customized: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectSignals {
    pub http_routes: usize,
    pub sql_statements: usize,
    pub pub_fns: usize,
    pub pub_lets: usize,
    pub imports: BTreeSet<String>,
    pub has_science: bool,
    pub has_class: bool,
    pub has_html: bool,
    pub has_css: bool,
    pub lib_modules: usize,
}

#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub base: PathBuf,
    pub template: String,
    pub template_inferred: bool,
    pub entry: String,
    pub version: Option<String>,
    pub port: Option<u16>,
    pub kab_files: Vec<KabFileInfo>,
    pub extra_kab_files: Vec<String>,
    pub signals: ProjectSignals,
    pub has_manifest: bool,
    pub has_compile_cache: bool,
    pub accomplishments: Vec<String>,
}

pub fn scan_project(base: &Path) -> Result<ProjectSnapshot, String> {
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let mut snapshot = ProjectSnapshot {
        base: base.clone(),
        template: "unknown".into(),
        template_inferred: true,
        entry: "main.kab".into(),
        version: None,
        port: None,
        kab_files: Vec::new(),
        extra_kab_files: Vec::new(),
        signals: ProjectSignals::default(),
        has_manifest: false,
        has_compile_cache: false,
        accomplishments: Vec::new(),
    };

    let manifest_path = base.join("kabootar.toml");
    if manifest_path.exists() {
        snapshot.has_manifest = true;
        if let Ok(m) = load_manifest(&manifest_path) {
            if let Some(t) = m.template {
                snapshot.template = t;
                snapshot.template_inferred = false;
            }
            if let Some(e) = m.entry {
                snapshot.entry = e;
            }
            snapshot.version = m.version;
            snapshot.port = m.port;
        }
        snapshot
            .accomplishments
            .push("kabootar.toml — projektkonfiguration finns".into());
    }

    let cache_dir = base.join(".kabootar/cache");
    if cache_dir.exists() && dir_has_files(&cache_dir) {
        snapshot.has_compile_cache = true;
        snapshot
            .accomplishments
            .push(".kabootar/cache/ — compile-cache används".into());
    }

    let blueprint_files = blueprint_file_set(&snapshot.template);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    collect_kab_files(&base, &base, &mut snapshot, &blueprint_files, &mut seen)?;
    collect_root_assets(&base, &mut snapshot);

    if snapshot.template_inferred || snapshot.template == "unknown" {
        snapshot.template = infer_template(&snapshot);
        snapshot.template_inferred = true;
    }

    derive_accomplishments(&mut snapshot);
    Ok(snapshot)
}

fn collect_kab_files(
    base: &Path,
    dir: &Path,
    snapshot: &mut ProjectSnapshot,
    blueprint_files: &BTreeSet<String>,
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, ".kabootar" | "road" | "target" | "node_modules") {
                continue;
            }
            collect_kab_files(base, &path, snapshot, blueprint_files, seen)?;
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("kab") {
            continue;
        }

        let rel = path
            .strip_prefix(base)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.display().to_string());

        if !seen.insert(rel.clone()) {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        analyze_kab_content(&content, &mut snapshot.signals);

        let in_blueprint = blueprint_files.contains(&rel);
        let customized = !in_blueprint || is_customized(&snapshot.template, &rel, &content);

        if in_blueprint {
            snapshot.kab_files.push(KabFileInfo {
                path: rel,
                lines: content.lines().count(),
                bytes: content.len(),
                customized,
            });
        } else {
            snapshot.extra_kab_files.push(rel);
        }
    }
    Ok(())
}

fn is_customized(template: &str, rel: &str, content: &str) -> bool {
    use super::projects::blueprint_by_id;
    let Some(bp) = blueprint_by_id(template) else {
        return true;
    };
    let Some(file) = bp.files.iter().find(|f| f.path == rel) else {
        return true;
    };
    normalize(&content) != normalize(file.content)
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_root_assets(base: &Path, snapshot: &mut ProjectSnapshot) {
    if base.join("index.html").exists() {
        snapshot.signals.has_html = true;
        snapshot
            .accomplishments
            .push("index.html — frontend/startside".into());
    }
    if base.join("static").is_dir() {
        snapshot.signals.has_css = true;
    }
}

fn analyze_kab_content(content: &str, signals: &mut ProjectSignals) {
    let lower = content.to_lowercase();
    signals.http_routes += count_occurrences(content, "http_route(");
    signals.sql_statements += count_occurrences(content, "sql(");
    signals.pub_fns += count_occurrences(content, "pub fn");
    signals.pub_lets += count_occurrences(content, "pub let");

    if lower.contains("import \"science\"") || lower.contains("stat_") || lower.contains("mat_") {
        signals.has_science = true;
    }
    if content.contains("class ") {
        signals.has_class = true;
    }

    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("import \"") {
            if let Some(name) = rest.split('"').next() {
                signals.imports.insert(name.to_string());
            }
        }
    }
}

fn derive_accomplishments(snapshot: &mut ProjectSnapshot) {
    let s = &snapshot.signals;
    if s.http_routes > 0 {
        snapshot.accomplishments.push(format!(
            "HTTP — {} route(s) registrerade",
            s.http_routes
        ));
    }
    if s.sql_statements > 0 {
        snapshot.accomplishments.push(format!(
            "SQL — {} databasanrop/schema",
            s.sql_statements
        ));
    }
    if s.pub_fns > 0 {
        snapshot.accomplishments.push(format!(
            "Moduler — {} exporterade funktioner (pub fn)",
            s.pub_fns
        ));
    }
    if s.has_science {
        snapshot
            .accomplishments
            .push("Science — statistik/beräkningar används".into());
    }
    if s.has_class {
        snapshot
            .accomplishments
            .push("Klasser — OOP-struktur introducerad".into());
    }

    let lib_count = snapshot
        .kab_files
        .iter()
        .filter(|f| f.path.starts_with("lib/"))
        .count()
        + snapshot
            .extra_kab_files
            .iter()
            .filter(|p| p.starts_with("lib/"))
            .count();
    snapshot.signals.lib_modules = lib_count;
    if lib_count > 0 {
        snapshot
            .accomplishments
            .push(format!("lib/ — {lib_count} modulfil(er)"));
    }

    for f in &snapshot.kab_files {
        if f.customized {
            snapshot
                .accomplishments
                .push(format!("{} — anpassad av utvecklare", f.path));
        }
    }
    for path in &snapshot.extra_kab_files {
        snapshot
            .accomplishments
            .push(format!("{path} — ny fil (ej från mall)"));
    }
}

fn infer_template(s: &ProjectSnapshot) -> String {
    if s.signals.has_science && s.signals.http_routes == 0 {
        return "science".into();
    }
    if s.signals.http_routes > 0 && s.signals.has_html && s.signals.sql_statements > 0 {
        return "fullstack".into();
    }
    if s.signals.http_routes >= 4 && s.signals.sql_statements > 0 {
        return "api-crud".into();
    }
    if s.signals.http_routes > 0 && s.signals.sql_statements > 0 {
        return "api".into();
    }
    if s.signals.http_routes > 0 {
        return "web".into();
    }
    if s.signals.lib_modules > 0 && s.signals.http_routes == 0 {
        return "library".into();
    }
    "unknown".into()
}

fn blueprint_file_set(template: &str) -> BTreeSet<String> {
    use super::projects::blueprint_by_id;
    blueprint_by_id(template)
        .map(|bp| bp.files.iter().map(|f| f.path.to_string()).collect())
        .unwrap_or_default()
}

fn dir_has_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .ok()
        .is_some_and(|mut rd| rd.flatten().next().is_some())
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

pub fn completion_percent(snapshot: &ProjectSnapshot) -> u8 {
    let mut score = 0u8;
    let mut max = 0u8;

    let checks: &[(bool, u8)] = &[
        (snapshot.has_manifest, 15),
        (!snapshot.kab_files.is_empty() || !snapshot.extra_kab_files.is_empty(), 15),
        (snapshot.signals.lib_modules > 0, 10),
        (snapshot.signals.http_routes > 0, 15),
        (snapshot.signals.sql_statements > 0, 10),
        (snapshot.has_compile_cache, 10),
        (snapshot.kab_files.iter().any(|f| f.customized), 15),
        (!snapshot.extra_kab_files.is_empty(), 10),
    ];

    for (ok, weight) in checks {
        max += weight;
        if *ok {
            score += weight;
        }
    }
    if max == 0 {
        return 0;
    }
    ((score as u16 * 100) / max as u16).min(100) as u8
}
