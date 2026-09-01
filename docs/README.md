<p align="center">
  <img src="../assets/logo.png" alt="Kabootar logo" width="128">
</p>

# Kabootar

**Kabootar** (tidigare *Nova*) är ett fullstack-programmeringsspråk med inbyggd runtime för frontend, backend, databas och operativsystem.

**Slutmål:** hela produkten är `.kab` — kompilator, VM, JIT, GC, CLI, stdlib, OS, browser. Rust i `src/` är **skuld** som ska **ersättas och raderas** ([SH28](ROADMAP.md#kabootar-på-egna-fötter--noll-rust)). Ny `.rs`-feature är regression. En användare ska bygga och köra Kabootar **utan rustc**. `cargo` / rustc nedan är **bootstrap för CI och toolchain**, inte taket.

## Snabbstart

```bash
# Produkt-CLI när den finns (idag: samma REPL via rustc-bootstrap)
kabootar
# rustc-bootstrap / skuld — inte produktvägen
cargo run

# Notebook
kabootar notebook run examples/explore_smoke.knb --science

# Host-tester (skuld tills SH25/SH28: kabootar test)
cargo test

# Language Server (IDE) — rustc-bootstrap
cargo build --features lsp

# DocAI — fråga dokumentationen
cargo run --bin kabootar-docai

# VS Code / Cursor extension
cd editor/vscode-kabootar && npm install && npm run compile

# WASM (webbläsare) — rustc-bootstrap tills SH27/SH28
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/kabootar_lib.wasm --out-dir pkg --target web
```

## Dokumentation

| Dokument | Innehåll |
|----------|----------|
| **[JAVASCRIPT.md](JAVASCRIPT.md)** | **För JS-utvecklare — bara skillnader mot JavaScript** |
| **[FEATURES.md](FEATURES.md)** | **Språkmatris — JS-paritet, lånade delar, status** |
| [IDE.md](IDE.md) | VS Code & Cursor — extension, LSP, CodAI |
| [OVERVIEW.md](OVERVIEW.md) | Vision, mål och arkitektur |
| [LANGUAGE.md](LANGUAGE.md) | Syntax och konstruktioner |
| [TYPES.md](TYPES.md) | `null` / `undefined`, `NaN`, sanning; typed arrays |
| [GENERICS.md](GENERICS.md) | Generics + traits (Våg T ✅) + struct (Våg R ✅) |
| [OWNERSHIP.md](OWNERSHIP.md) | `@manual` Owned / borrow (Våg O ✅ subset) |
| [CLASSES.md](CLASSES.md) | C#-inspirerade klasser |
| [KML.md](KML.md) | Kabootar Markup Language |
| [OS.md](OS.md) | Kernel/host `os_*` — **kOS-produkt:** [lib/kos/README.md](../lib/kos/README.md) |
| [BROWSER.md](BROWSER.md) | Pekare — **kbrowser-produkt:** [lib/kbrowser/README.md](../lib/kbrowser/README.md) |
| [MODULES.md](MODULES.md) | `import` — builtins + `lib/` pillars |
| [STDLIB.md](STDLIB.md) | Natives + `lib/std` + kDOM/Kv8 |
| [HTTP.md](HTTP.md) | Backend HTTP-routing |
| [SQL.md](SQL.md) | Inbyggd SQL-databas |
| [LSP.md](LSP.md) | Language Server, VS Code-extension |
| [SECURITY.md](SECURITY.md) | Krypto, secure memory, enheter, providers |
| [SCIENCE.md](SCIENCE.md) | `import "science"` — ndarray, linalg, HOAD, PDE/FEM, ML |
| [GAME.md](GAME.md) | Spelmotor — GPU, audio, physics, editor |
| [XR.md](XR.md) | `import "game/xr"` — OpenXR/WebXR host FFI |
| [SHIP.md](SHIP.md) | Ship / packaging |
| [APP.md](APP.md) | App-pillar |
| [IOT.md](IOT.md) | IoT |
| [SIM.md](SIM.md) | Simulation / robotics |
| [DATA.md](DATA.md) | Data pillar |
| [COMPILE.md](COMPILE.md) | Kompilering / `.kbc` |
| [DX_TOOLING.md](DX_TOOLING.md) | Exploration DX tooling |
| [FRAMEWORK_PILLARS.md](FRAMEWORK_PILLARS.md) | Pillar-översikt |
| [PLATFORM.md](PLATFORM.md) | Plattformar |
| [CANVAS.md](CANVAS.md) | Canvas / WebGL |
| [KV8.md](KV8.md) | Kv8 JS-motor |
| [EXPLORATION.md](EXPLORATION.md) | REPL + `.knb` notebook |
| [DOCAI.md](DOCAI.md) | DocAI — fråga dokumentationen |
| [RUNTIME.md](RUNTIME.md) | DOM, OS, databas |
| [ROADMAP.md](ROADMAP.md) | Implementationsplan — **noll Rust**, SH-ordning, **Nästa** |

*Ny i språket och kan redan JS? Börja med [JAVASCRIPT.md](JAVASCRIPT.md), inte [LANGUAGE.md](LANGUAGE.md). Synk-`fn*` / iterator-protokoll: [LANGUAGE.md](LANGUAGE.md) **10ae–** (kab-only).*

## Projektstruktur

```
self_host/                      # Produktkompilator (.kab) + seed/compiler.kbcb
lib/kab/                        # Produkt-VM och primitives (.kab)
lib/                            # Fil-moduler (game, std, science, kos, kbrowser, …)
src/                            # Rust-skuld — ersätts och raderas (SH28). Inte språkets tak.
  lexer.rs, parser.rs, …        # gammal host-compiler (appar: self-host, SH16)
  bytecode/vm.rs                # host-VM (körning: Kab-VM kab-only default, SH6)
  evaluator.rs                  # AST-tolk — skuld, inte produktväg
docs/                           # Denna dokumentation
```

## Licens

Se projektägaren.
