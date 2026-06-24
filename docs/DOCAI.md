# Kabootar DocAI — dokumentationsassistent

**DocAI** låter utvecklare ställa frågor om Kabootar-dokumentationen och få svar baserade på inbäddade `docs/*.md`-filer.

## Snabbstart

### Interaktiv CLI

```bash
cargo run --bin kabootar-docai
```

```
docai> hur importerar jag science?
docai> vad gör stat_mean?
docai> :topics
docai> :search SQL INSERT
docai> :quit
```

### I Kabootar-kod

```kabootar
import "docai";

doc_ask("hur fungerar import science");
doc_search("PLC timer", 5);
doc_sources("SQL WHERE");
doc_topics();
```

## Funktioner

| Funktion | Beskrivning | Exempel |
|----------|-------------|---------|
| `doc_ask(query)` | Svar syntetiserat från relevanta doc-avsnitt | `doc_ask("KML attribut")` |
| `doc_search(query, limit?)` | Rankad lista av träffar (default 5) | `doc_search("os_write", 3)` |
| `doc_sources(query)` | Källor (fil + rubrik) för en fråga | `doc_sources("crypto sha256")` |
| `doc_topics()` | Alla indexerade dokument | `doc_topics()` |

`import "docai"` registrerar natives — funktionerna finns **inte** globalt förrän import.

## Exempel — vanliga frågor

### Moduler och science

```kabootar
import "docai";
doc_ask("hur importerar jag science modulen");
```

**Förväntat:** svar med `import "science"` och referens till `SCIENCE.md`.

### Statistik

```kabootar
import "docai";
doc_ask("hur beräknar jag medelvärde med stat_mean");
```

### SQL

```kabootar
import "docai";
doc_search("SQL PRIMARY KEY", 3);
```

### Säkerhet / krypto

```kabootar
import "docai";
doc_ask("crypto_secure minneswipe");
```

### OS och filer

```kabootar
import "docai";
doc_ask("hur läser jag fil med os_read");
```

## CLI-kommandon

| Kommando | Effekt |
|----------|--------|
| `:topics` | Lista alla dokument (README, SCIENCE, SQL, …) |
| `:search <fråga>` | Visa råa sökträffar med poäng |
| `:quit` / `:exit` | Avsluta |

## Hur det fungerar

1. Alla markdown-filer i `docs/` bäddas in vid kompilering (`include_str!`).
2. Varje `##` / `###`-rubrik blir ett sökbart avsnitt.
3. Din fråga tokeniseras (svenska + engelska, stopwords filtreras bort).
4. Avsnitt poängsätts efter rubrik, titel, filnamn och innehåll.
5. Toppträffar sammanställs till ett svar med kodexempel och källhänvisningar.

```
Fråga → tokenisering → poängsättning → topp 5 avsnitt → svar + källor
```

## Indexerade dokument

| Fil | Ämne |
|-----|------|
| README.md | Översikt, snabbstart |
| LANGUAGE.md | Syntax |
| TYPES.md | Typer |
| CLASSES.md | Klasser |
| MODULES.md | `import` |
| SCIENCE.md | STEM, statistik, matriser |
| SQL.md | Databas |
| HTTP.md | Backend |
| OS.md | Kabootar OS |
| SECURITY.md | Krypto |
| KML.md | Markup |
| LSP.md | IDE |
| RUNTIME.md | Runtime |
| … | Se `:topics` |

## Tips för bättre svar

- Var specifik: `"stat_mean medelvärde"` bättre än `"statistik"`.
- Nämn modulnamn: `"import science"`, `"docai"`, `"crypto"`.
- Använd `:search` först om svaret känns brett.
- Kombinera med `doc_sources()` för att se exakt vilka avsnitt som användes.

## Implementation

| Del | Plats |
|-----|-------|
| Index + sök | `src/docai/` |
| Natives | `src/runtime/docai.rs` |
| CLI | `src/bin/kabootar-docai.rs` |
| Modul | `import "docai"` i `src/modules/mod.rs` |

## Felsökning

| Problem | Åtgärd |
|---------|--------|
| Tomt svar | Prova `:search` med kortare nyckelord |
| `Module not found` | `import "docai";` |
| `doc_ask expects string` | Skicka frågan som strängliteral |

## VS Code / Cursor

Efter `cargo build --bin kabootar-docai` och `npm run compile` i `editor/vscode-kabootar/`:

| Kommando | Effekt |
|----------|--------|
| **Kabootar: Fråga DocAI** | Input-ruta → svar i DocAI-panel |
| **Kabootar: Öppna DocAI-panel** | Chattpanel vid sidan av editorn |
| **Kabootar: Sök i dokumentation** | Råa träffar i ny flik |
| **Kabootar: DocAI ämnen** | Välj dokument → fråga om ämnet |

Högerklick i `.kab`-filer → **Fråga DocAI**.

Inställning: `kabootar.docai.path` (tom = auto-detect `target/release/kabootar-docai`).

## Framtida utökning

- Koppling till extern LLM (OpenAI-kompatibel API) för friare formulering
- Flerspråkiga embeddings

Se [ROADMAP.md](ROADMAP.md).
