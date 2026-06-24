//! Auto-generated plain-text progress file — accomplishments + next steps.

use super::projects::{blueprint_by_id, ProjectBlueprint};
use super::scan::{completion_percent, ProjectSnapshot};

pub const PROGRESS_PATH: &str = "PROGRESS.txt";
pub const NOTES_HEADER: &str = "ANTECKNINGAR (skriv själv)\n--------------------------\n";
pub const DEFAULT_NOTES: &str =
    "Du kan redigera eller radera denna fil när som helst.\n\nDatum:\nGjort idag:\nBlockerat av:\nPlan för nästa session:\n";

/// Vanlig textfil från mall-id (statisk).
pub fn progress_report(id: &str) -> Result<String, String> {
    let bp = blueprint_by_id(id).ok_or_else(|| format!("unknown project: {id}"))?;
    Ok(build_progress_static(bp))
}

/// Uppdaterad textfil från skannat projekttillstånd.
pub fn progress_from_snapshot(snapshot: &ProjectSnapshot) -> String {
    let title = blueprint_by_id(&snapshot.template)
        .map(|b| b.title)
        .unwrap_or("Kabootar-projekt");
    let desc = blueprint_by_id(&snapshot.template)
        .map(|b| b.description)
        .unwrap_or("Utvecklingsprojekt");
    let (_, _, tips) = profile(&snapshot.template);
    let pct = completion_percent(snapshot);
    let next = dynamic_next_steps(snapshot);

    let mut out = String::new();
    out.push_str("================================================================================\n");
    out.push_str(&format!("PROJEKTSTATUS — {}\n", title));
    out.push_str(&format!(
        "Uppdaterad av CodAI sync. Mall: {} ({}% klart)\n",
        snapshot.template, pct
    ));
    out.push_str("================================================================================\n\n");
    out.push_str(&format!("{desc}\n\n"));

    out.push_str("VAD DU HAR ÅSTADKOMMIT\n");
    out.push_str("----------------------\n");
    if snapshot.accomplishments.is_empty() {
        out.push_str("CodAI hittade inga tydliga framsteg ännu.\n\n");
    } else {
        for item in &snapshot.accomplishments {
            out.push_str(&format!("[x] {item}\n"));
        }
        out.push('\n');
    }

    out.push_str("FILER I PROJEKTET\n");
    out.push_str("-----------------\n");
    for f in &snapshot.kab_files {
        let mark = if f.customized { "x" } else { " " };
        out.push_str(&format!(
            "[{mark}] {} ({} rader)\n",
            f.path, f.lines
        ));
    }
    for path in &snapshot.extra_kab_files {
        out.push_str(&format!("[x] {path} (ny fil)\n"));
    }
    if snapshot.has_compile_cache {
        out.push_str("[x] .kabootar/cache/\n");
    }
    out.push('\n');

    out.push_str("NÄSTA STEG (baserat på nuvarande kod)\n");
    out.push_str("-------------------------------------\n");
    if next.is_empty() {
        out.push_str("1. Fortsätt utveckla — kör code_project_sync efter ändringar\n");
    } else {
        for (i, step) in next.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }
    out.push('\n');

    out.push_str("TIPS FÖR DIN PROJEKTTYP\n");
    out.push_str("-----------------------\n");
    out.push_str(tips);
    out.push_str("\n\n");

    out.push_str(NOTES_HEADER);
    out.push_str(DEFAULT_NOTES);
    out.push('\n');

    out.push_str("--------------------------------------------------------------------------------\n");
    out.push_str("Synka igen: code_project_sync(\".\")\n");
    out.push_str("Roadmap: road/ROADMAP.txt | Nu: road/NOW.txt | IDE: road/IDE.txt\n");

    out
}

/// Behåll användarens anteckningar vid sync om de skrivit något.
pub fn preserve_notes(existing: &str) -> Option<String> {
    let start = existing.find(NOTES_HEADER)? + NOTES_HEADER.len();
    let rest = &existing[start..];
    let end = rest
        .find("\n--------------------------------------------------------------------------------")
        .unwrap_or(rest.len());
    let body = rest[..end].trim();
    if body == DEFAULT_NOTES.trim() || body.is_empty() {
        return None;
    }
    Some(body.to_string())
}

pub fn apply_preserved_notes(mut progress: String, notes: &str) -> String {
    if let Some(idx) = progress.find(NOTES_HEADER) {
        let after_header = idx + NOTES_HEADER.len();
        if let Some(footer) = progress[after_header..].find("\n--------------------------------------------------------------------------------") {
            progress = format!(
                "{}{}{}\n\n--------------------------------------------------------------------------------{}",
                &progress[..after_header],
                notes,
                "\n",
                &progress[after_header + footer..]
            );
        }
    }
    progress
}

/// Dynamiska nästa steg utifrån vad som finns i mappen.
pub fn dynamic_next_steps(snapshot: &ProjectSnapshot) -> Vec<String> {
    let mut steps: Vec<String> = Vec::new();
    let s = &snapshot.signals;
    let entry = &snapshot.entry;

    if !snapshot.has_manifest {
        steps.push("Skapa kabootar.toml med version, template och entry".into());
    }
    if snapshot.kab_files.is_empty() && snapshot.extra_kab_files.is_empty() {
        steps.push(format!("Skapa entry-filen {entry}"));
    }
    if !snapshot.has_compile_cache && !snapshot.kab_files.is_empty() {
        steps.push(format!("Kör kabootar compile {entry}"));
    }

    match snapshot.template.as_str() {
        "web" | "fullstack" => {
            if s.http_routes == 0 {
                steps.push("Registrera http_route i main.kab".into());
            } else if s.http_routes < 3 {
                steps.push(format!(
                    "Du har {} route(s) — lägg till /health och fler sidor",
                    s.http_routes
                ));
            }
            if !s.has_html && snapshot.template == "fullstack" {
                steps.push("Skapa eller utöka index.html".into());
            }
            if s.http_routes > 0 {
                steps.push(format!("Kör kabootar serve --watch {entry}"));
            }
        }
        "api" | "api-crud" => {
            if s.sql_statements == 0 {
                steps.push("Lägg till SQL CREATE TABLE i main.kab eller lib/db.kab".into());
            }
            if s.http_routes < 2 {
                steps.push("Lägg till health-endpoint och minst ett API-anrop".into());
            }
            if s.http_routes > 0 {
                steps.push("Testa endpoints med curl eller webbläsare".into());
            }
            if s.lib_modules < 2 {
                steps.push("Dela handlers i lib/-filer (routes, db, validators)".into());
            }
        }
        "science" => {
            if !s.has_science {
                steps.push("import \"science\" och anropa stat_* eller mat_*".into());
            }
            if s.lib_modules < 2 {
                steps.push("Separera data och analys i lib/data.kab och lib/analysis.kab".into());
            }
            steps.push(format!("Kör kabootar run {entry}"));
        }
        "library" => {
            if s.pub_fns == 0 {
                steps.push("Exportera funktioner med pub fn i lib/".into());
            }
            steps.push("Kör demo.kab eller skapa tests via import".into());
        }
        _ => {
            if s.http_routes > 0 {
                steps.push(format!("Kör kabootar serve {entry}"));
            } else {
                steps.push(format!("Kör kabootar run {entry}"));
            }
        }
    }

    if s.lib_modules == 0 && !snapshot.kab_files.is_empty() {
        steps.push("Skapa lib/-mapp och flytta ut logik i moduler".into());
    }
    if snapshot.extra_kab_files.is_empty()
        && snapshot.kab_files.iter().all(|f| !f.customized)
        && !snapshot.kab_files.is_empty()
    {
        steps.push("Anpassa mallkoden — ersätt stubbar med din egen logik".into());
    }

    steps.push("Kör code_project_sync(\".\") för att uppdatera PROGRESS.txt och road/".into());

    // Dedupe while preserving order
    let mut seen = std::collections::BTreeSet::new();
    steps
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .take(8)
        .collect()
}

fn build_progress_static(bp: &ProjectBlueprint) -> String {
    let (accomplished, next_steps, tips) = profile(bp.id);

    let mut out = String::new();
    out.push_str("================================================================================\n");
    out.push_str(&format!("PROJEKTSTATUS — {}\n", bp.title));
    out.push_str(&format!("Genererad av CodAI. Mall: {}\n", bp.id));
    out.push_str("================================================================================\n\n");
    out.push_str(&format!("{}\n\n", bp.description));

    out.push_str("VAD DU HAR ÅSTADKOMMIT (startpunkt)\n");
    out.push_str("-----------------------------------\n");
    out.push_str("CodAI har skapat projektstrukturen. Bocka av [ ] när du verifierat:\n\n");
    for file in bp.files {
        out.push_str(&format!("[ ] {} — {}\n", file.path, file.description));
    }
    out.push_str("[ ] .kabootar/cache/ — skapas för kabootar compile\n\n");

    out.push_str("SAMMANFATTNING\n");
    out.push_str("--------------\n");
    out.push_str(accomplished);
    out.push_str("\n\n");

    out.push_str("NÄSTA STEG (rekommenderat)\n");
    out.push_str("--------------------------\n");
    for (i, step) in next_steps.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, step));
    }
    out.push('\n');

    out.push_str("TIPS FÖR DIN PROJEKTTYP\n");
    out.push_str("-----------------------\n");
    out.push_str(tips);
    out.push_str("\n\n");

    out.push_str("ANTECKNINGAR (skriv själv)\n");
    out.push_str("--------------------------\n");
    out.push_str("Du kan redigera eller radera denna fil när som helst.\n\n");
    out.push_str("Datum:\n");
    out.push_str("Gjort idag:\n");
    out.push_str("Blockerat av:\n");
    out.push_str("Plan för nästa session:\n\n");

    out.push_str("--------------------------------------------------------------------------------\n");
    out.push_str(&format!(
        "Förhandsgranska: code_project_progress(\"{}\")\n",
        bp.id
    ));
    out.push_str("Synka vid utveckling: code_project_sync(\".\")\n");

    out
}

type Profile = (&'static str, &'static [&'static str], &'static str);

pub(crate) fn profile(id: &str) -> Profile {
    match id {
        "web" => (
            "Du har en fungerande webb-start: HTTP-routes i main.kab, konfig i lib/config.kab \
             och en statisk index.html. Projektet är redo för kabootar serve.",
            &[
                "Kör kabootar serve --watch main.kab och öppna http://localhost:8080",
                "Ändra SITE_TITLE i lib/config.kab till ditt projektnamn",
                "Utöka home() i main.kab — returnera HTML eller JSON",
                "Koppla index.html till backend (fetch mot /health eller egna routes)",
                "Lägg till fler routes i main.kab (t.ex. /about, /api/info)",
            ],
            "För webb: håll statiska filer i rot eller static/. Dela konstanter via pub let i lib/. \
             Använd import \"docai\" om du behöver hjälp med HTTP-syntax.",
        ),
        "api" => (
            "Du har ett REST API-skelett: SQL-tabell items, health-endpoint och CRUD-stubbar i lib/routes.kab. \
             Databas och routes delar samma process.",
            &[
                "Starta med kabootar serve --watch main.kab",
                "Testa GET /health och GET /api/items i webbläsare eller curl",
                "Implementera riktig body-läsning i create_item() (ersätt hårdkodat värde)",
                "Lägg till validering och felkoder (http_response(400, ...))",
                "Flytta fler endpoints till lib/routes.kab när main.kab blir lång",
                "Kör kabootar compile main.kab för parse-cache",
            ],
            "Läs Kabootar docs/SQL.md för parametrar ($1, $2). Dela handlers i lib/. \
             Överväg api-crud-mallen om du behöver full CRUD.",
        ),
        "api-crud" => (
            "Du har full CRUD för users: schema i lib/db.kab, handlers i lib/users.kab, \
             routes registrerade i main.kab.",
            &[
                "Kör servern och testa alla verb: GET/POST/PUT/DELETE på /api/users",
                "Ersätt hårdkodade värden i users_create med data från request-body",
                "Lägg till WHERE-filter och paginering i users_list",
                "Inför unik constraint på email i init_db()",
                "Skapa lib/validators.kab för indata-kontroll",
                "Dokumentera API:et i denna textfil under Anteckningar",
            ],
            "Nästa nivå: autentisering (import \"crypto\"), versionering i kabootar.toml, \
             och separata tabeller per resurs.",
        ),
        "science" => (
            "Du har dataanalys-upplägg: exempeldata i lib/data.kab, analys i lib/analysis.kab, \
             entry i main.kab som skriver ut rapport.",
            &[
                "Kör kabootar run main.kab och kontrollera utskriften",
                "Byt ut SAMPLE_X / SAMPLE_Y mot dina mätvärden",
                "Utöka analyze() med stat_std, mat_mul eller stat_linreg",
                "Spara resultat till fil med os_write om du behöver export",
                "Lägg till fler dataset i separata lib/*.kab-filer",
            ],
            "För tyngre beräkningar: dela pipeline i steg (load, clean, analyze, report). \
             Använd import \"science\" för alla stat/matris-funktioner.",
        ),
        "fullstack" => (
            "Du har webb + API kombinerat: frontend (index.html, static/app.css), \
             API i lib/api.kab, config i lib/config.kab.",
            &[
                "Starta kabootar serve --watch main.kab",
                "Verifiera /, /health och /api/status",
                "Utöka static/app.css och hämta API-data med fetch i index.html",
                "Lägg till SQL eller fler endpoints i lib/api.kab",
                "Flytta frontend till static/ om projektet växer",
                "Sätt port i kabootar.toml om 8080 är upptagen",
            ],
            "Fullstack: håll main.kab som router-träd, lägg affärslogik i lib/. \
             Överväg att splitta till separata web + api-projekt vid skalning.",
        ),
        "library" => (
            "Du har ett återanvändbart bibliotek: lib/greet.kab, lib/utils.kab och demo.kab \
             som visar hur man importerar.",
            &[
                "Kör kabootar run demo.kab och verifiera output",
                "Lägg till pub fn / pub let i lib-filer",
                "Sätt @version i varje lib-modul och i kabootar.toml [dependencies]",
                "Dela paketet via lib/-mappen eller KABOOTAR_PATH",
                "Ta bort demo.kab när du integrerar i ett större projekt",
            ],
            "Bibliotek ska vara fria från HTTP/SQL om de är generella. Testa med demo.kab \
             innan du publicerar modulen.",
        ),
        _ => (
            "Projektstruktur skapad av CodAI.",
            &[
                "Granska filerna i projektroten och lib/",
                "Kör entry-filen med kabootar run eller kabootar serve",
                "Anpassa innehåll efter ditt use case",
            ],
            "Använd code_suggest() och code_util() för fler kodbyggblock.",
        ),
    }
}
