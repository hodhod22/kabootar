//! CodAI — utility-first kodassistent (Tailwind för Kabootar-kod).
//!
//! `import "codai"` registrerar `code_*`-natives.

mod compose;
mod ide;
mod progress;
mod project_ops;
mod projects;
mod roadmap;
mod scan;
mod search;
mod sync;
mod snippets;

pub use compose::{complete, compose, explain};
pub use ide::{ide_recommendations, IDE_PATH};
pub use progress::{progress_from_snapshot, progress_report, preserve_notes, PROGRESS_PATH};
pub use project_ops::{
    format_scaffold_report, project_plan, project_tree, resolve_base_path, scaffold_project,
    suggest_projects, ProjectHit, ScaffoldReport,
};
pub use projects::{all_project_ids, blueprint_by_id, ProjectBlueprint, BLUEPRINTS};
pub use roadmap::{build_road_files, ROAD_DIR, ROAD_DONE_PATH, ROAD_NOW_PATH, ROADMAP_PATH};
pub use scan::{scan_project, completion_percent, ProjectSnapshot};
pub use search::{suggest, UtilHit};
pub use sync::{format_sync_report, sync_project, SyncReport};
pub use snippets::{categories, util_by_id, all_ids, CodeUtil, UTILS};

/// Formaterad katalog över alla utilities, grupperad per kategori.
pub fn catalog() -> String {
    let mut out = String::from("CodAI utilities (utility-first kodbyggblock):\n\n");
    let mut last_cat = "";
    for util in UTILS {
        if util.category != last_cat {
            out.push_str(&format!("\n## {}\n", util.category));
            last_cat = util.category;
        }
        out.push_str(&format!("  {} — {}\n", util.id, util.title));
        out.push_str(&format!("    {}\n", util.description));
    }
    out
}

/// Hämta kod för en utility (Tailwind-liknande `code_util("http-route-get")`).
pub fn util(id: &str) -> Result<String, String> {
    util_by_id(id)
        .map(|u| u.code.to_string())
        .ok_or_else(|| {
            let hints = suggest(id, 3)
                .into_iter()
                .map(|h| h.id)
                .collect::<Vec<_>>()
                .join(", ");
            if hints.is_empty() {
                format!("unknown utility: {id}. Use code_utils() to list all.")
            } else {
                format!("unknown utility: {id}. Did you mean: {hints}?")
            }
        })
}

/// Snabb hjälp för en kategori eller utility.
pub fn help(topic: &str) -> String {
    let t = topic.trim().to_lowercase();
    if t.is_empty() || t == "all" {
        return catalog();
    }

    if let Some(u) = util_by_id(&t) {
        return format!(
            "# {}\n{}\n\n```kabootar\n{}\n```",
            u.id, u.description, u.code
        );
    }

    let cat_utils: Vec<_> = UTILS
        .iter()
        .filter(|u| u.category == t)
        .collect();

    if !cat_utils.is_empty() {
        let mut out = format!("# category: {t}\n\n");
        for u in cat_utils {
            out.push_str(&format!("  {} — {}\n", u.id, u.title));
        }
        out.push_str("\nAnvänd code_util(id) för full kod.");
        return out;
    }

    let hits = suggest(topic, 5);
    if hits.is_empty() {
        format!("Ingen träff för '{topic}'. Kategorier: {}", categories().join(", "))
    } else {
        let mut out = format!("# förslag för '{topic}':\n\n");
        for h in hits {
            out.push_str(&format!("  {} — {} ({})\n", h.id, h.title, h.description));
        }
        out
    }
}
