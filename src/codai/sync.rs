//! Sync PROGRESS.txt and road/ from live project state.

use super::ide::{ide_recommendations, IDE_PATH};
use super::progress::{
    apply_preserved_notes, preserve_notes, progress_from_snapshot, PROGRESS_PATH,
};
use super::roadmap::{build_road_files, current_phase_public, ROAD_DONE_PATH, ROAD_NOW_PATH, ROADMAP_PATH, ROAD_DIR};
use super::scan::{completion_percent, scan_project};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub updated: Vec<String>,
    pub template: String,
    pub phase: String,
    pub completion_pct: u8,
}

pub fn sync_project(base: &Path) -> Result<SyncReport, String> {
    let snapshot = scan_project(base)?;
    let base = &snapshot.base;

    let mut progress = progress_from_snapshot(&snapshot);
    if let Ok(existing) = std::fs::read_to_string(base.join(PROGRESS_PATH)) {
        if let Some(notes) = preserve_notes(&existing) {
            progress = apply_preserved_notes(progress, &notes);
        }
    }

    let road = build_road_files(&snapshot);
    let mut report = SyncReport {
        template: snapshot.template.clone(),
        phase: current_phase_public(&snapshot).to_string(),
        completion_pct: completion_percent(&snapshot),
        ..Default::default()
    };

    write_file(base, PROGRESS_PATH, &progress, &mut report.updated)?;
    std::fs::create_dir_all(base.join(ROAD_DIR))
        .map_err(|e| format!("Failed to create {ROAD_DIR}/: {e}"))?;
    write_file(base, ROADMAP_PATH, &road.roadmap, &mut report.updated)?;
    write_file(base, ROAD_NOW_PATH, &road.now, &mut report.updated)?;
    write_file(base, ROAD_DONE_PATH, &road.done, &mut report.updated)?;
    write_file(base, IDE_PATH, &ide_recommendations(&snapshot), &mut report.updated)?;

    Ok(report)
}

fn write_file(base: &Path, rel: &str, content: &str, updated: &mut Vec<String>) -> Result<(), String> {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
    }
    std::fs::write(&path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    updated.push(rel.to_string());
    Ok(())
}

pub fn format_sync_report(report: &SyncReport) -> String {
    let mut out = String::new();
    out.push_str("CodAI sync — projekt uppdaterat\n\n");
    out.push_str(&format!("Mall: {}\n", report.template));
    out.push_str(&format!("Fas: {}\n", report.phase));
    out.push_str(&format!("Framsteg: {}%\n\n", report.completion_pct));
    out.push_str("Uppdaterade filer:\n");
    for f in &report.updated {
        out.push_str(&format!("  * {f}\n"));
    }
    out.push_str("\nKör code_project_sync efter varje utvecklingssession.\n");
    out
}
