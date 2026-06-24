# Kabootar

**Kabootar** (tidigare *Nova*) är ett fullstack-programmeringsspråk med inbyggd runtime för frontend, backend, databas och operativsystem.

## Snabbstart

```bash
# REPL (CLI)
cargo run

# Tester
cargo test

# Language Server (IDE)
cargo build --features lsp

# DocAI — fråga dokumentationen
cargo run --bin kabootar-docai

# VS Code / Cursor extension
cd editor/vscode-kabootar && npm install && npm run compile

# WASM (webbläsare)
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/kabootar.wasm --out-dir pkg --target web
# Öppna index.html med en lokal server
```

## Dokumentation

| Dokument | Innehåll |
|----------|----------|
| **[JAVASCRIPT.md](JAVASCRIPT.md)** | **För JS-utvecklare — bara skillnader mot JavaScript** |
| **[FEATURES.md](FEATURES.md)** | **Språkmatris — JS-paritet, lånade delar, status** |
| [IDE.md](IDE.md) | VS Code & Cursor — extension, LSP, CodAI |
| [OVERVIEW.md](OVERVIEW.md) | Vision, mål och arkitektur |
| [LANGUAGE.md](LANGUAGE.md) | Syntax och konstruktioner |
| [TYPES.md](TYPES.md) | `null`, `undefined`, `NaN`, sanning |
| [CLASSES.md](CLASSES.md) | C#-inspirerade klasser |
| [KML.md](KML.md) | Kabootar Markup Language |
| [OS.md](OS.md) | Kabootar OS (kernel + VFS) |
| [MODULES.md](MODULES.md) | `import` och inbyggda moduler |
| [HTTP.md](HTTP.md) | Backend HTTP-routing |
| [SQL.md](SQL.md) | Inbyggd SQL-databas |
| [LSP.md](LSP.md) | Language Server, VS Code-extension |
| [SECURITY.md](SECURITY.md) | Krypto, secure memory, enheter, providers |
| [SCIENCE.md](SCIENCE.md) | `import "science"` — komplexa tal & STEM |
| [DOCAI.md](DOCAI.md) | DocAI — fråga dokumentationen (`kabootar-docai`) |
| [RUNTIME.md](RUNTIME.md) | DOM, OS, databas |
| [ROADMAP.md](ROADMAP.md) | Implementationsplan |

*Ny i språket och kan redan JS? Börja med [JAVASCRIPT.md](JAVASCRIPT.md), inte [LANGUAGE.md](LANGUAGE.md).*

## Projektstruktur

```
src/
  lexer.rs, parser.rs, ast.rs   # Språkfront-end
  value.rs                      # Typmodell
  evaluator.rs                  # Tolk
  class/                        # Klassystem
  kml/                          # KML-parser och renderer
  sql/                          # SQL-motor
  modules/                      # import-system
  editor/vscode-kabootar/       # VS Code/Cursor extension
  runtime/
    os/                         # Kabootar OS (kernel + vfs)
    http.rs                     # HTTP router
    browser_dom.rs              # Värd-webbläsarens DOM
    kabootar_dom.rs             # Egen DOM (KML)
    os/                         # Kabootar OS kernel
    db.rs                       # Inbyggd SQL-databas
```

## Licens

Se projektägaren.
