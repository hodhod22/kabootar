# Kabootar — moduler (`import`)

Kabootar har ett inbyggt modulsystem för att dela kod.

## Syntax

```kabootar
import "math";
add(2, 3);   // 5
```

Modulnamn är strängliteraler. Importerade bindingar läggs i den aktuella miljön.

## Inbyggda moduler (v1.0)

| Modul | Innehåll |
|-------|----------|
| `std` | `parse`, `stringify`, `info` — JSON/stdlib wrappers |
| `json` | `parse`, `dump` — JSON helpers |
| `collections` | `map_new_empty`, `set_new_empty`, `from_pairs` |
| `strings` | `clean`, `parts`, `has_prefix` |
| `math` | `add(a, b)`, `mul(a, b)` |
| `http` | `ok`, `not_found`, **alla verb** (`route_*`, `request_*`, `fetch_*`, `method_get()` …) — se [HTTP.md](HTTP.md) |
| `crypto` | `sha256(data)`, `secure(data)` — wrappers för kryptofunktioner |
| `science` | fysik, kemi, statistik, **ndarray**, **linalg/signal/sparse**, **autograd HOAD**, **PDE/FEM**, **ML** — se [SCIENCE.md](SCIENCE.md) / Våg SC |
| `docai` | Dokumentations-AI — `doc_ask`, `doc_search` — se [DOCAI.md](DOCAI.md) |
| `codai` | Kodassistent (utility-first) — `code_util`, `code_suggest` — se [CODAI.md](CODAI.md) |

### Exempel: math

```kabootar
import "math";
mul(3, 4);   // 12
```

### Exempel: http

```kabootar
import "http";

fn home() { return ok("Kabootar") }
fn update() { return ok(req_body) }

route_get("/", home);
route_put("/api/item", update);

http_body(request_get("/"));
http_status(request_put("/api/item", "{\"ok\":true}"));
```

Full REST med alla verb: `route_get`, `route_post`, `route_put`, `route_patch`, `route_delete`, `route_head`, `route_options`.

### Exempel: science

```kabootar
import "science";
c_abs(cplx(3, 4));        // 5
ph(0.001);                // 3
kinetic_energy(2, 3);     // 9
compound(1000, 0.05, 2);  // sammansatt ränta
ohms_v(10, 2);            // 20 V
stat_mean([1, 2, 3, 4, 5]); // 3

// Deepen (se SCIENCE.md § Science deepen):
import "science/nd";
import "science/linalg";
einsum("ij,jk->ik", from([[1.0, 0.0], [0.0, 1.0]], [2, 2]), from([[2.0, 3.0], [4.0, 5.0]], [2, 2]));
randomizedSvd([[4.0, 1.0], [1.0, 3.0]], 1);
```

### Exempel: docai

```kabootar
import "docai";
doc_ask("hur importerar jag science");
doc_topics();
```

Se [DOCAI.md](DOCAI.md).

### Exempel: codai

```kabootar
import "codai";
code_util("http-route-get");
code_suggest("REST API");
code_compose(["http-health", "http-serve"]);
```

Se [CODAI.md](CODAI.md).

### Exempel: filmodul (v2.0)

Skapa `lib/greet.kab`:

```kabootar
pub fn greet(name) {
    return "Hello, " + name
}

fn secret() {
    return "internal"
}
```

Endast `pub fn` exporteras till importören:

```kabootar
import "greet";
greet("Kabootar");   // OK
secret();            // fel — privat funktion
```

Sökvägar: `./name.kab`, `lib/name.kab`, eller `KABOOTAR_PATH`.

## Projektlivscykel (v2.0+)

```bash
kabootar mod init api      # skapar kabootar.toml + main.kab
kabootar compile main.kab  # parse-cache (v2.1)
kabootar mod run             # kör entry från kabootar.toml
kabootar serve --watch main.kab   # HTTP med hot reload (v2.1)
kabootar run script.kab      # kör en fil
```

### `kabootar.toml` med beroenden (v2.1)

```toml
version = "0.1.0"
entry = "main.kab"
port = 8080

[dependencies]
greet = "1.0.0"
```

Moduler deklarerar version med `@version "1.0.0"` överst i `.kab`-filen.
Importera med `import "greet@1.0"` eller lita på `[dependencies]`.

## Lokalt paketregistry (v2.17)

Publicera och installera moduler som npm-liknande paket — helt lokalt (ingen nätverks-server ännu).

```bash
kabootar publish lib/greet.kab    # → .kabootar/registry/greet/1.0.0/greet.kab
kabootar install greet@1.0        # → .kabootar/packages/greet/1.0.0/greet.kab
kabootar install                  # alla [dependencies] i kabootar.toml
```

`import "greet"` löser i ordning: `lib/greet.kab` (projekt), sedan installerade paket under `.kabootar/packages/`, sedan `KABOOTAR_PATH`.

Natives för automation (respekterar `KABOOTAR_PROJECT_ROOT` om satt):

```kabootar
registry_publish("lib/greet.kab")
registry_install("greet", "1.0")
registry_list()   // array av { name, version }
registry_search("http")   // sök builtin, lib, registry, installed
registry_seed()           // publicera alla lib/*.kab med @version
registry_uninstall("greet", "1.0")
ecosystem_info()          // räknare + stage
modules_catalog()         // alla tillgängliga moduler
```

### Exempelbibliotek i `lib/`

| Paket | Innehåll |
|-------|----------|
| `greet` | Hälsning |
| `config` | `APP_NAME`, `MAX_ITEMS`, `limit_ok` |
| `validation` | `is_email`, `is_non_empty`, `in_range` |
| `http_json` | `json_ok`, `json_error`, `parse_body` |
| `pagination` | `page(items, offset, limit)` |

Kör `registry_seed()` för att publicera `lib/*.kab` till lokalt registry.

## Pillar-moduler (`lib/`)

| Import | Doc | Innehåll (subset) |
|--------|-----|-------------------|
| `game` / `game/audio` / `game/xr` / … | [GAME.md](GAME.md), [XR.md](XR.md) | GPU, audio (PCM + Uint8), physics, editor, XR FFI |
| `std` / `std/array` / … | [STDLIB.md](STDLIB.md) | Fil-stdlib wrappers |
| `science` / `science/nd` / … | [SCIENCE.md](SCIENCE.md) | ndarray, linalg, ML, PDE |
| `app` | [APP.md](APP.md) | App shell |
| `iot` | [IOT.md](IOT.md) | IoT |
| `sim` | [SIM.md](SIM.md) | Robot / digital twin |
| `data` | [DATA.md](DATA.md) | Data pillar |
| `cad` | [FRAMEWORK_PILLARS.md](FRAMEWORK_PILLARS.md) | CAD |
| `web` / `kdom` / `kstyle` / `kv8` | [RUNTIME.md](RUNTIME.md), [KV8.md](KV8.md) | DOM, CSS, Kv8 |
| `doc` / `dx` / `os` | [DOCAI.md](DOCAI.md), [DX_TOOLING.md](DX_TOOLING.md), [OS.md](OS.md) | DocAI, exploration DX, kOS |

Sökordning: projekt-`lib/`, installerade paket, `KABOOTAR_PATH`.

Kräver `@version` i modulfilen vid publicering.

### `pub let` (v2.1)

```kabootar
pub let API_VERSION = "1.0"
pub fn handler() { ... }
```

Mallar: `web`, `api`. Se [PROJECT.md](PROJECT.md).

## Implementation

- Modulregister: `src/modules/mod.rs`
- `import` parsas som `Stmt::Import` i AST
- `math`, `http`, `crypto`: modulkörning via `eval_source()` i samma environment
- `science`, `docai`: natives registreras direkt (`science_register` / `docai_register`) — stub-källkod finns endast för LSP/goto
- Filmoduler (v2.0): evalueras i isolerad miljö; endast `pub fn` exporteras

## Framtida utveckling

- [x] Filer på disk (`import "my/app"`) — v1.9
- [x] `pub fn` / modulisolering — v2.0
- [x] `kabootar mod` CLI — v2.0
- Versionshantering och beroenden

Se [ROADMAP.md](ROADMAP.md).
