use super::progress::PROGRESS_PATH;
use super::sync::sync_project;
use super::projects::{blueprint_by_id, ProjectBlueprint, BLUEPRINTS};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHit {
    pub id: String,
    pub title: String,
    pub description: String,
    pub score: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScaffoldReport {
    pub created: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn suggest_projects(query: &str, limit: usize) -> Vec<ProjectHit> {
    let terms = tokenize(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut hits: Vec<ProjectHit> = BLUEPRINTS
        .iter()
        .map(|b| {
            let score = score_blueprint(b, &terms, &query_lower);
            ProjectHit {
                id: b.id.to_string(),
                title: b.title.to_string(),
                description: b.description.to_string(),
                score,
            }
        })
        .filter(|h| h.score > 0)
        .collect();

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    hits.truncate(limit);
    hits
}

fn score_blueprint(bp: &ProjectBlueprint, terms: &[String], query_lower: &str) -> i32 {
    let mut score = 0i32;
    let id_lower = bp.id.to_lowercase();
    let title_lower = bp.title.to_lowercase();
    let desc_lower = bp.description.to_lowercase();

    if id_lower == query_lower {
        return 100;
    }
    if id_lower.contains(query_lower) || title_lower.contains(query_lower) {
        score += 35;
    }
    if desc_lower.contains(query_lower) {
        score += 25;
    }

    for term in terms {
        if id_lower.contains(term) {
            score += 15;
        }
        if title_lower.contains(term) {
            score += 12;
        }
        if desc_lower.contains(term) {
            score += 8;
        }
        for tag in bp.tags {
            if tag.contains(term.as_str()) {
                score += 10;
            }
        }
    }

    let intents: &[(&[&str], &[&str])] = &[
        (
            &["webb", "web", "sida", "html", "frontend", "website"],
            &["web", "fullstack"],
        ),
        (
            &["api", "rest", "backend", "endpoint", "json"],
            &["api", "api-crud", "fullstack"],
        ),
        (
            &["crud", "users", "databas", "database", "sql"],
            &["api-crud", "api"],
        ),
        (
            &["statistik", "science", "data", "analys", "matris"],
            &["science"],
        ),
        (
            &["fullstack", "hela", "komplett", "app"],
            &["fullstack"],
        ),
        (
            &["bibliotek", "library", "modul", "paket", "lib"],
            &["library"],
        ),
    ];

    for (keywords, ids) in intents {
        if keywords.iter().any(|k| query_lower.contains(k)) {
            if ids.contains(&bp.id) {
                score += 20;
            }
        }
    }

    score
}

fn tokenize(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect()
}

/// ASCII-träd över projektets mappstruktur.
pub fn project_tree(id: &str) -> Result<String, String> {
    let bp = blueprint_by_id(id).ok_or_else(|| unknown_project_msg(id))?;
    let mut out = format!("# {} — {}\n\n", bp.title, bp.description);
    out.push_str("projekt/\n");

    let mut root_files: Vec<_> = bp
        .files
        .iter()
        .filter(|f| !f.path.contains('/'))
        .collect();
    root_files.sort_by_key(|f| f.path);

    let mut dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for file in bp.files {
        if let Some((dir, _)) = file.path.split_once('/') {
            dirs.insert(dir.to_string());
        }
    }

    for file in &root_files {
        out.push_str(&format!("├── {}\n", file.path));
    }
    let dir_list: Vec<_> = dirs.into_iter().collect();
    for (i, dir) in dir_list.iter().enumerate() {
        let is_last_dir = i + 1 == dir_list.len();
        let branch = if is_last_dir { "└── " } else { "├── " };
        out.push_str(&format!("{branch}{dir}/\n"));

        let mut children: Vec<_> = bp
            .files
            .iter()
            .filter(|f| f.path.starts_with(&format!("{dir}/")))
            .map(|f| f.path.strip_prefix(&format!("{dir}/")).unwrap_or(f.path))
            .collect();
        children.sort();

        for (j, child) in children.iter().enumerate() {
            let is_last = j + 1 == children.len();
            let prefix = if is_last_dir { "    " } else { "│   " };
            let leaf = if is_last { "└── " } else { "├── " };
            out.push_str(&format!("{prefix}{leaf}{child}\n"));
        }
    }

    out.push_str("\nFiler:\n");
    for file in bp.files {
        out.push_str(&format!("  {} — {}\n", file.path, file.description));
    }
    out.push_str(&format!("  {PROGRESS_PATH} — textfil: status (uppdateras med sync)\n"));
    out.push_str("  road/ROADMAP.txt — utvecklingsplan\n");
    out.push_str("  road/NOW.txt — gör härnäst\n");
    out.push_str("  road/IDE.txt — VS Code & Cursor rekommendationer\n");
    Ok(out)
}

/// Plan utan att skriva till disk — förhandsgranska filer.
pub fn project_plan(id: &str) -> Result<Vec<(String, String)>, String> {
    let bp = blueprint_by_id(id).ok_or_else(|| unknown_project_msg(id))?;
    Ok(bp
        .files
        .iter()
        .map(|f| (f.path.to_string(), f.description.to_string()))
        .collect())
}

/// Skapa mappar och filer på disk. Skriver inte över befintliga filer om `force` är false.
pub fn scaffold_project(id: &str, base: &Path, force: bool) -> Result<ScaffoldReport, String> {
    let bp = blueprint_by_id(id).ok_or_else(|| unknown_project_msg(id))?;
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());

    let mut report = ScaffoldReport::default();

    for file in bp.files {
        let path = base.join(file.path);
        if path.exists() && !force {
            report.skipped.push(file.path.to_string());
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, file.content)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        report.created.push(file.path.to_string());
    }

    // Standard cache-mapp (tom) — skippas om den redan finns
    let cache = base.join(".kabootar/cache");
    if !cache.exists() {
        std::fs::create_dir_all(&cache)
            .map_err(|e| format!("Failed to create .kabootar/cache: {e}"))?;
        if !report.created.iter().any(|p| p.starts_with(".kabootar")) {
            report.created.push(".kabootar/cache/".to_string());
        }
    }

    sync_project(&base).map(|sync| {
        for f in sync.updated {
            if !report.created.contains(&f) {
                report.created.push(f);
            }
        }
    })?;

    Ok(report)
}

pub fn format_scaffold_report(id: &str, report: &ScaffoldReport) -> String {
    let mut out = format!("# scaffold: {id}\n\n");
    if !report.created.is_empty() {
        out.push_str("Skapade:\n");
        for p in &report.created {
            out.push_str(&format!("  + {p}\n"));
        }
    }
    if !report.skipped.is_empty() {
        out.push_str("\nHoppade över (finns redan — radera manuellt om du vill ersätta):\n");
        for p in &report.skipped {
            out.push_str(&format!("  ~ {p}\n"));
        }
    }
    if report.created.is_empty() && report.skipped.is_empty() {
        out.push_str("Inga filer skapade.\n");
    }
    out.push_str("\nRedigera, radera eller utöka filerna fritt.");
    out.push_str(&format!("\nKör code_project_sync(\".\") för att uppdatera {PROGRESS_PATH} och road/."));
    out
}

pub fn resolve_base_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.as_os_str().is_empty() || path == "." {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        p.to_path_buf()
    }
}

fn unknown_project_msg(id: &str) -> String {
    let hints = suggest_projects(id, 3)
        .into_iter()
        .map(|h| h.id)
        .collect::<Vec<_>>()
        .join(", ");
    if hints.is_empty() {
        format!("unknown project: {id}. Use code_projects() to list templates.")
    } else {
        format!("unknown project: {id}. Did you mean: {hints}?")
    }
}
