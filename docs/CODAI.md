# Kabootar CodAI — utility-first kodassistent

**CodAI** hjälper utvecklare skriva Kabootar-kod snabbare — ungefär som **Tailwind gör för CSS**: färdiga *utilities* (byggblock) istället för att skriva allt från scratch.

## Snabbstart

### Interaktiv CLI

```bash
cargo run --bin kabootar-codai
```

```
codai> :suggest REST API
codai> :util http-route-get
codai> :compose http-health,http-serve
codai> :categories
codai> :quit
```

### I Kabootar-kod

```kabootar
import "codai";

code_util("http-route-get");
code_suggest("SQL tabell", 5);
code_compose(["http-health", "http-serve"]);
code_complete("http-route");
code_explain("import \"http\"; http_route(...)");
code_help("sql");
code_utils();
code_categories();

// Projektmallar — mappstruktur + filer
code_projects();
code_project_suggest("jag vill bygga ett REST API");
code_project_tree("api");
code_project_plan("api");
code_project_scaffold("api", "./my-app");
```

## Koncept: utilities

Varje utility är en **färdig kodmall** för ett vanligt mönster:

| Kategori | Exempel-utilities |
|----------|-------------------|
| `http` | `http-route-get`, `http-route-post`, `http-health`, `http-serve` |
| `sql` | `sql-create-table`, `sql-insert`, `sql-select`, `sql-update` |
| `mod` | `mod-pub-fn`, `mod-pub-let`, `mod-import-file`, `mod-versioned` |
| `class` | `class-basic` |
| `science` | `science-stats`, `science-linreg`, `science-matrix` |
| `project` | `project-main`, `project-api` |
| `lang` | `array-literal`, `closure-fn` |
| `crypto` | `crypto-hash` |

`import "codai"` registrerar natives — funktionerna finns **inte** globalt förrän import.

## Funktioner

| Funktion | Beskrivning | Exempel |
|----------|-------------|---------|
| `code_utils()` | Alla utility-id | `code_utils()` |
| `code_util(id)` | Full kodmall | `code_util("sql-insert")` |
| `code_suggest(query, limit?)` | Föreslå utilities utifrån intent | `code_suggest("statistik", 5)` |
| `code_compose(ids)` | Slå ihop flera utilities (dedupe imports) | `code_compose(["http-health", "http-serve"])` |
| `code_complete(partial)` | Komplettera partiell kod/id | `code_complete("http-route")` |
| `code_explain(code)` | Förklara vad kod gör | `code_explain(src)` |
| `code_help(topic?)` | Katalog eller kategori-hjälp | `code_help("http")` |
| `code_categories()` | Lista kategorier | `code_categories()` |

## Projektmallar — mappstruktur & scaffolding

CodAI föreslår **hela projektstrukturer** beroende på vad du vill bygga, och kan **skapa mappar och filer** med startkod. Du kan sedan radera, byta ut eller utveckla innehållet fritt.

| Projektmall | Beskrivning |
|-------------|-------------|
| `web` | HTTP-server + `index.html` + `lib/config.kab` |
| `api` | REST API med SQL + `lib/routes.kab` |
| `api-crud` | Full CRUD för users |
| `science` | Dataanalys med `lib/data.kab`, `lib/analysis.kab` |
| `fullstack` | Webb + API + `static/app.css` |
| `library` | Återanvändbar `lib/` utan HTTP |

| Funktion | Beskrivning | Exempel |
|----------|-------------|---------|
| `code_projects()` | Alla projektmallar | `code_projects()` |
| `code_project_suggest(query, limit?)` | Föreslå mall utifrån intent | `code_project_suggest("dataanalys")` |
| `code_project_tree(id)` | ASCII-mappträd + filbeskrivningar | `code_project_tree("api")` |
| `code_project_plan(id)` | Lista filer utan att skriva till disk | `code_project_plan("web")` |
| `code_project_scaffold(id, path?, force?)` | Skapa mappar/filer | `code_project_scaffold("api", "./shop")` |
| `code_project_progress(id)` | Förhandsgranska mall-text | `code_project_progress("api")` |
| `code_project_sync(path?)` | **Uppdatera** `PROGRESS.txt` och `road/` från mappen | `code_project_sync(".")` |

**Sync:** CodAI skannar `.kab`-filer, `kabootar.toml` och kodmönster (routes, SQL, science …) och uppdaterar status i takt med utvecklingen. Kör efter varje session.

**Standard scaffold:** befintliga kodfiler **skrivs inte över** (`force = false`). `PROGRESS.txt` och `road/` uppdateras alltid vid scaffold och sync.

### Exempel — skapa ett API-projekt

```kabootar
import "codai";

// 1. Få förslag
code_project_suggest("REST API med databas");

// 2. Förhandsgranska struktur
code_project_tree("api-crud");

// 3. Skapa filer i ./my-api
code_project_scaffold("api-crud", "./my-api");

// 4. Läs textfilen med åstadkommet + nästa steg
code_project_progress("api-crud");
```

Efter scaffold skapas **`PROGRESS.txt`** och mappen **`road/`**:

| Fil | Innehåll |
|-----|----------|
| `PROGRESS.txt` | Åstadkommit, filer, nästa steg (uppdateras med sync) |
| `road/ROADMAP.txt` | Faser i utvecklingsplanen |
| `road/NOW.txt` | Vad du bör göra härnäst |
| `road/DONE.txt` | Det CodAI ser att du redan gjort |
| `road/IDE.txt` | VS Code & Cursor rekommendationer |

**Kan du JavaScript?** Börja med [JAVASCRIPT.md](JAVASCRIPT.md) — bara Kabootar-specifika skillnader.

Se [IDE.md](IDE.md) för full IDE-guide.

```kabootar
// Efter att du kodat — uppdatera i takt med utvecklingen:
code_project_sync(".");
```

**Skapade filer (exempel):**

```
my-api/
├── kabootar.toml
├── main.kab
├── PROGRESS.txt       ← synkas med code_project_sync
├── road/
│   ├── ROADMAP.txt
│   ├── NOW.txt
│   └── DONE.txt
└── lib/
    ├── db.kab
    └── users.kab
```

Kör sedan:

```bash
cd my-api
kabootar serve --watch main.kab
```

## Exempel — bygg ett API snabbt

```kabootar
import "codai";

let health = code_util("http-health");
let serve = code_util("http-serve");

// Eller i ett steg:
let app = code_compose(["http-health", "http-route-get", "http-serve"]);
```

### Föreslå utilities

```kabootar
import "codai";
code_suggest("jag behöver en databastabell och insert");
```

**Förväntat:** träffar som `sql-create-table`, `sql-insert`.

### Kombinera utilities

```kabootar
import "codai";
code_compose(["sql-create-table", "sql-insert", "sql-select"]);
```

Imports dedupliceras automatiskt (`import "sql"` skrivs bara en gång).

## CLI-flaggor

```bash
kabootar-codai --utils
kabootar-codai --categories
kabootar-codai --util http-route-get
kabootar-codai --suggest "REST API" --limit 5
kabootar-codai --compose http-health,http-serve
kabootar-codai --help-topic science
kabootar-codai --projects
kabootar-codai --project-suggest "fullstack webapp"
kabootar-codai --project-tree fullstack
kabootar-codai --project-scaffold api --dir ./my-api
kabootar-codai --project-sync
kabootar-codai --project-sync --dir ./my-api
```

## CodAI vs DocAI

| | **DocAI** | **CodAI** |
|---|-----------|-----------|
| Syfte | Svara på dokumentationsfrågor | Generera/föreslå **kod** |
| Data | `docs/*.md` | Inbäddade kodmallar |
| Analogi | FAQ / sökmotor | Tailwind utilities |

Använd **DocAI** när du undrar *hur* något fungerar. Använd **CodAI** när du vill *skriva kod snabbare*.

## Utöka utilities

Utilities definieras i `src/codai/snippets.rs`. Projektmallar i `src/codai/projects.rs`.

Se även [MODULES.md](MODULES.md) och [DOCAI.md](DOCAI.md).
