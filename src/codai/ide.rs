//! IDE-rekommendationer för VS Code och Cursor.

use super::scan::ProjectSnapshot;

pub const IDE_PATH: &str = "road/IDE.txt";

pub fn ide_recommendations(snapshot: &ProjectSnapshot) -> String {
    let entry = &snapshot.entry;
    let template = &snapshot.template;
    let workspace_hint = if snapshot.has_manifest {
        "Projektmappen är öppen som workspace — bra för LSP och CodAI."
    } else {
        "Öppna projektmappen (med kabootar.toml) som workspace i VS Code eller Cursor."
    };

    format!(
        r#"================================================================================
IDE — VS Code & Cursor (rekommendationer)
================================================================================

Kabootar är anpassad för både VS Code och Cursor med samma extension.
{workspace_hint}

BYGG (en gång per maskin / efter git clone)
-------------------------------------------
  cargo build --features lsp
  cargo build --bin kabootar-docai --bin kabootar-codai

  cd editor/vscode-kabootar
  npm install && npm run compile
  code --install-extension .
  (Cursor: Extensions → Install from VSIX, eller F5 "Run Extension")

INSTÄLLNINGAR (valfritt — auto-detekterar target/debug)
-------------------------------------------------------
  kabootar.languageServer.path  → kabootar-lsp
  kabootar.docai.path           → kabootar-docai
  kabootar.codai.path           → kabootar-codai

KOMMANDON I VS CODE / CURSOR (Command Palette: Ctrl+Shift+P)
------------------------------------------------------------
  Kabootar: CodAI — synka projekt     → uppdaterar PROGRESS.txt och road/
  Kabootar: CodAI — föreslå utility   → Tailwind-liknande kodbyggblock
  Kabootar: CodAI — föreslå projekt    → web, api, science, …
  Kabootar: Öppna CodAI-panel         → sync + förslag i sidopanel
  Kabootar: Fråga DocAI               → dokumentationsfrågor
  Kabootar: Öppna DocAI-panel         → docs-chatt

REKOMMENDERAT ARBETSFLÖDE
-------------------------
  1. Öppna projektmappen som workspace
  2. Redigera {entry} och lib/*.kab (LSP ger fel, autocomplete, go-to-definition)
  3. Efter ändringar: Command Palette → "Kabootar: CodAI — synka projekt"
  4. Läs road/NOW.txt och PROGRESS.txt för nästa steg
  5. Terminal: kabootar serve --watch {entry}  eller  kabootar run {entry}

CURSOR-SPECIFIKT
----------------
  • Använd AI-chatten med @PROGRESS.txt och @road/ROADMAP.txt som kontext
  • Be Cursor: "uppdatera enligt road/NOW.txt" efter code_project_sync
  • import "codai" i .kab för code_util, code_suggest, code_compose
  • DocAI för syntax-frågor: import "docai" eller DocAI-panelen

VS CODE-SPECIFIKT
-----------------
  • Split editor: kod + PROGRESS.txt eller road/NOW.txt
  • Högerklick i .kab → Fråga DocAI
  • Go to definition på import "science" → kabootar://module/science
  • Problems-panel visar LSP-parsefel direkt

TERMINAL (båda IDE:erna)
------------------------
  kabootar serve --watch {entry}
  kabootar compile {entry}
  kabootar-codai --project-sync
  kabootar-codai --suggest "REST API"

FILer ATT HA ÖPPNA VID UTVECKLING
---------------------------------
  • {entry} — entrypoint
  • PROGRESS.txt — status (synkas)
  • road/NOW.txt — nästa steg
  • road/IDE.txt — VS Code / Cursor tips
  • kabootar.toml — port och dependencies

NY I SPRÅKET MEN KAN JAVASCRIPT?
--------------------------------
  Läs docs/JAVASCRIPT.md — bara skillnader mot JS, inget repetitivt.

Mall för detta projekt: {template}

================================================================================
"#,
        workspace_hint = workspace_hint,
        entry = entry,
        template = template,
    )
}
