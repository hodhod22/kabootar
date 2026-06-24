//! road/ folder — roadmap text files synced with project progress.

use super::scan::{completion_percent, ProjectSnapshot};
use super::progress::dynamic_next_steps;

pub const ROAD_DIR: &str = "road";
pub const ROADMAP_PATH: &str = "road/ROADMAP.txt";
pub const ROAD_NOW_PATH: &str = "road/NOW.txt";
pub const ROAD_DONE_PATH: &str = "road/DONE.txt";

pub struct RoadFiles {
    pub roadmap: String,
    pub now: String,
    pub done: String,
}

pub fn build_road_files(snapshot: &ProjectSnapshot) -> RoadFiles {
    RoadFiles {
        roadmap: build_roadmap(snapshot),
        now: build_now(snapshot),
        done: build_done(snapshot),
    }
}

fn build_roadmap(snapshot: &ProjectSnapshot) -> String {
    let pct = completion_percent(snapshot);
    let phase = current_phase(snapshot);
    let phases = phase_list(snapshot);

    let mut out = String::new();
    out.push_str("================================================================================\n");
    out.push_str("ROADMAP — utvecklingsplan\n");
    out.push_str(&format!(
        "Projekt: {} | Mall: {} | Fas: {} | Framsteg: {}%\n",
        snapshot.entry, snapshot.template, phase, pct
    ));
    out.push_str("Uppdaterad av CodAI (code_project_sync)\n");
    out.push_str("================================================================================\n\n");

    for (name, status, detail) in phases {
        let mark = match status {
            PhaseStatus::Done => "[x]",
            PhaseStatus::Active => "[>]",
            PhaseStatus::Todo => "[ ]",
        };
        out.push_str(&format!("{mark} {name}\n"));
        out.push_str(&format!("    {detail}\n\n"));
    }

    out.push_str("--------------------------------------------------------------------------------\n");
    out.push_str("Kör code_project_sync(\".\") efter varje utvecklingssession.\n");
    out
}

fn build_now(snapshot: &ProjectSnapshot) -> String {
    let steps = dynamic_next_steps(snapshot);
    let mut out = String::new();
    out.push_str("NU — gör detta härnäst\n");
    out.push_str("=====================\n\n");

    if steps.is_empty() {
        out.push_str("Projektet ser komplett ut för nuvarande fas. Granska road/ROADMAP.txt.\n");
        return out;
    }

    for (i, step) in steps.iter().take(6).enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, step));
    }
    out.push_str("\nTips: använd code_suggest() och code_util() för kodbyggblock.\n");
    out
}

fn build_done(snapshot: &ProjectSnapshot) -> String {
    let mut out = String::new();
    out.push_str("KLART — det CodAI ser i projektet\n");
    out.push_str("=================================\n\n");

    if snapshot.accomplishments.is_empty() {
        out.push_str("Inget upptäckt ännu. Skapa filer eller kör code_project_scaffold.\n");
        return out;
    }

    for item in &snapshot.accomplishments {
        out.push_str(&format!("[x] {item}\n"));
    }

    out.push_str(&format!(
        "\nKabootar-filer: {} | Extra: {} | Routes: {} | SQL: {}\n",
        snapshot.kab_files.len(),
        snapshot.extra_kab_files.len(),
        snapshot.signals.http_routes,
        snapshot.signals.sql_statements
    ));
    out
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PhaseStatus {
    Done,
    Active,
    Todo,
}

fn current_phase(snapshot: &ProjectSnapshot) -> &'static str {
    current_phase_public(snapshot)
}

pub fn current_phase_public(snapshot: &ProjectSnapshot) -> &'static str {
    if !snapshot.has_manifest {
        return "Setup";
    }
    if snapshot.signals.http_routes == 0
        && snapshot.signals.sql_statements == 0
        && !snapshot.signals.has_science
    {
        return "Grund";
    }
    if !snapshot.has_compile_cache
        || snapshot.kab_files.iter().all(|f| !f.customized) && snapshot.extra_kab_files.is_empty()
    {
        return "Kärnfunktion";
    }
    if completion_percent(snapshot) < 70 {
        return "Utökning";
    }
    "Finputsning"
}

fn phase_list(snapshot: &ProjectSnapshot) -> Vec<(&'static str, PhaseStatus, String)> {
    let phase = current_phase(snapshot);
    let phases = vec![
        (
            "1. Setup — kabootar.toml och entry",
            if snapshot.has_manifest {
                PhaseStatus::Done
            } else if phase == "Setup" {
                PhaseStatus::Active
            } else {
                PhaseStatus::Todo
            },
            "Projektkonfiguration och startfil.".into(),
        ),
        (
            "2. Grund — routes, SQL eller science",
            phase_status_for(
                snapshot.signals.http_routes > 0
                    || snapshot.signals.sql_statements > 0
                    || snapshot.signals.has_science,
                phase,
                "Grund",
                "Kärnfunktion",
            ),
            template_core_hint(&snapshot.template),
        ),
        (
            "3. Moduler — lib/ och pub fn",
            phase_status_for(
                snapshot.signals.lib_modules > 0,
                phase,
                "Kärnfunktion",
                "Utökning",
            ),
            format!(
                "{} modulfiler i lib/.",
                snapshot.signals.lib_modules
            ),
        ),
        (
            "4. Anpassning — egna filer och logik",
            phase_status_for(
                snapshot.kab_files.iter().any(|f| f.customized)
                    || !snapshot.extra_kab_files.is_empty(),
                phase,
                "Utökning",
                "Finputsning",
            ),
            "Utvecklaren har börjat ändra och utöka.".into(),
        ),
        (
            "5. Verktyg — compile-cache och polish",
            phase_status_for(snapshot.has_compile_cache, phase, "Finputsning", "Finputsning"),
            if snapshot.has_compile_cache {
                "kabootar compile körd.".into()
            } else {
                "Kör kabootar compile på entry-filen.".into()
            },
        ),
        (
            "6. Drift — serve/run i produktion",
            phase_status_for(
                snapshot.signals.http_routes > 0 && snapshot.port.is_some(),
                phase,
                "Finputsning",
                "Finputsning",
            ),
            if let Some(p) = snapshot.port {
                format!("kabootar serve --port {p} {}", snapshot.entry)
            } else {
                format!("kabootar serve {}", snapshot.entry)
            },
        ),
    ];
    phases
}

fn phase_status_for(done: bool, current: &str, active_phase: &str, _next: &str) -> PhaseStatus {
    if done {
        PhaseStatus::Done
    } else if current == active_phase {
        PhaseStatus::Active
    } else {
        PhaseStatus::Todo
    }
}

fn template_core_hint(template: &str) -> String {
    match template {
        "web" => "HTTP-routes och eventuellt index.html.".into(),
        "api" | "api-crud" => "REST-endpoints och SQL-schema.".into(),
        "science" => "Data och statistik i lib/.".into(),
        "fullstack" => "Webb + API routes.".into(),
        "library" => "pub fn i lib/ och demo.kab.".into(),
        _ => "Implementera huvudfunktion i entry-filen.".into(),
    }
}
