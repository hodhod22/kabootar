# Kabootar — översikt

## Vision

Kabootar ska vara **ett språk för hela stacken**: samma kod och samma mentala modell i webbläsare, på server och i inbyggda miljöer — utan JavaScripts vanligaste fallgropar. **Hela produkten ska stå på `.kab`** (kompilator, VM, JIT, GC, CLI) — Rust i `src/` är skuld som raderas ([SH28](ROADMAP.md#kabootar-på-egna-fötter--noll-rust)).

## Kärnprinciper

1. **JavaScript där det fungerar** — syntax och API:er som utvecklare redan kan, minus de delar som orsakar buggar. Se [JAVASCRIPT.md](JAVASCRIPT.md) om du redan kan JS.
2. **Rust-inspirerad säkerhet** — tydliga typer, `Result`/`Option`, inget tyst `NaN`-gift för heltal.
3. **C#-inspirerade klasser** — riktiga klasser med fält, konstruktorer och metoder (inte bara prototyper).
4. **Dubbel plattform** — två lager: host (riktig OS/DOM/Chrome) *och* Kabootar-native (`os`, `kdom`, `kbrowser`). Se [PLATFORM.md](PLATFORM.md).
5. **Dubbel DOM** — värd-DOM i webbläsaren *och* Kabootars egen DOM med eget markup-språk (KML).
6. **Inbyggd databas** — PostgreSQL-inspirerad motor med SQL som förstaklass-språk.
7. **Inbyggt OS** — sandboxad kernel, enkel idag, utbyggbar till fullständigt OS.
8. **Säkerhetsverktygslåda** — krypto, secure memory, enheter; säkerhetsagnostisk design.

## Arkitektur

```
┌─────────────────────────────────────────────────────────┐
│                    Kabootar-källkod (.kab)               │
└─────────────────────────┬───────────────────────────────┘
                          │
              ┌───────────▼───────────┐
              │  self_host/ compile    │
              │  tokenize → parse → emit │
              └───────────┬───────────┘
                          │
              ┌───────────▼───────────┐
              │   .kbc / packed .kbcb  │
              └───────────┬───────────┘
                          │
              ┌───────────▼───────────┐
              │  Kab-VM (kab-only)     │
              │  self_host/vm_*        │
              └───────────┬───────────┘
                          │
     ┌────────────────────┼────────────────────┐
     │                    │                    │
┌────▼────┐      ┌────────▼────────┐   ┌──────▼──────┐
│ Host    │      │  Kabootar DOM   │   │  Kabootar   │
│ DOM +   │      │     (KML)       │   │  OS + DB +  │
│ Chrome  │      │  + kbrowser     │   │  Browser    │
└─────────┘      └─────────────────┘   └─────────────┘
```

Produktvägen är **`self_host/` → bytecode → Kab-VM**. AST-evaluator och rust-VM i `src/` är host-skuld. Synk-**`fn*`** / **`yield`** / **`yield*`** / **`try`/`finally`** i generatorer körs på Kab-VM (se [LANGUAGE.md](LANGUAGE.md) 10ae–). Se [PLATFORM.md](PLATFORM.md) och [BROWSER.md](BROWSER.md). Plan: [ROADMAP.md — noll Rust](ROADMAP.md#kabootar-på-egna-fötter--noll-rust).

## Målplattformar

| Plattform | Status |
|-----------|--------|
| WASM / webbläsare | ✅ Tolk + `document` |
| Native CLI (REPL) | ✅ |
| Backend server | ✅ HTTP + routing |
| Kabootar OS | ✅ Kernel + VFS (v1.0) |
| Kabootar DB | ✅ SQL in-process |

## Namnbyte

Projektet hette **Nova** fram till v0.2. Paketnamn, WASM-export och dokumentation använder nu **Kabootar**.
