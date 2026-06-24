# Kabootar — runtime

## Dubbel plattform

Kabootar har **två lager** — host (befintlig OS/DOM/Chrome) och Kabootar-native (`os`, `kdom`, `kbrowser`). Standardläge är `hybrid` (båda registrerade).

```kabootar
platform_info();
platform_use("kabootar");   // eller "host" / "hybrid"
```

Full dokumentation: [PLATFORM.md](PLATFORM.md), [BROWSER.md](BROWSER.md).

## Dubbel DOM

### 1. Host DOM (`document`, `window`, `navigator`)

När Kabootar körs i en webbläsare eller mot värd-OS — Chrome-lik API:

```kabootar
document.createElement("div");
window.location.href;
navigator.userAgent;
```

Implementering: `src/runtime/browser_dom.rs`

### 2. Kabootar DOM (`kdom`)

Egen DOM-trädstruktur oberoende av webbläsaren. Renderas med **Kabootar Markup Language (KML)**.

```kabootar
let ui = kml("<div class=\"app\"><h1>Hej</h1></div>");
let div = kdom_create("section");
let tree = kdom_append(div, ui);
println(kdom_render(tree));
```

Live API: `kdom_create`, `kdom_append`, `kdom_set_attr`, `kdom_query`, `kdom_children`.

### 3. Kabootar Browser (`kbrowser`)

Chrome-inspirerad motor med **renderingspipeline**, compositor och VFS-navigation:

```kabootar
kb_mount(ui);
kb_paint();           // layout + paint → compositor frame
kb_composite();       // frame + OS-fönster + flikinfo
kb_navigate("kabootar://vfs/apps/page.kml");
```

Se [BROWSER.md](BROWSER.md) och [RENDERING.md](RENDERING.md).

Implementering: `src/runtime/kabootar_dom.rs`

```
DomNode { tag, attributes, children, text }
```

**Användningsfall:**

- Native desktop-appar utan webbläsare
- Server-side rendering
- Enhetlig UI-modell över plattformar

## Operativsystem (`os`)

Kabootar OS är en sandboxad kernel — enkel idag, byggd för att växa:

```kabootar
os_info();                      // kabootar-kernel 1.0.0
os_caps();                      // [vfs, sandbox, modules]
os_mkdir("/apps");
os_write("/apps/note.txt", "Hi");
os_stat("/apps/note.txt");      // [file, 2]
```

Se [OS.md](OS.md) för kernel-arkitektur och roadmap.

## HTTP (`http_*`)

Inbyggd request/response-routing för backend:

```kabootar
fn hello() { return http_response(200, "Hi") }
http_route("GET", "/hello", hello);
http_body(http_request("GET", "/hello"));
```

Se [HTTP.md](HTTP.md) för fullständig referens.

Implementering: `src/runtime/http.rs`

## Databas (`db` / `sql`)

PostgreSQL-**inspirerad** in-process databas med persistent lagring per session:

```kabootar
sql("CREATE TABLE users (id INTEGER, name TEXT)");
sql("INSERT INTO users (id, name) VALUES (1, 'Ada')");
sql("SELECT name FROM users WHERE id = $1", 1);   // Ada
sql("SELECT users.name, orders.total FROM users JOIN orders ON users.id = orders.user_id");
```

### SQL-stöd (v1.1)

| Funktion | Status |
|----------|--------|
| `CREATE TABLE` / `DROP TABLE` | ✅ |
| `PRIMARY KEY` | ✅ |
| `INSERT` / `UPDATE` / `DELETE` | ✅ |
| `SELECT` / `WHERE` / `JOIN` | ✅ |
| `ORDER BY` / `LIMIT` / `OFFSET` | ✅ |
| `COUNT(*)` | ✅ |
| `IS NULL` / `IS NOT NULL` | ✅ |
| Parametrar (`$1`) | ✅ |
| Transaktioner | Planerat |

Se [SQL.md](SQL.md) för fullständig referens.

Implementering: `src/runtime/db.rs`

## Frontend + backend

Samma Kabootar-kod kan köras:

| Miljö | Entry |
|-------|-------|
| Webbläsare | WASM + `evaluate()` |
| Server | Native binary (planerat) |
| REPL | `cargo run` |

Backend-runtime delar samma `evaluator` och `runtime`-moduler.
