# Kabootar — HTTP (backend)

Kabootar har en inbyggd HTTP-request/response-modell för backend-kod.

## Routing

Registrera handlers och dispatcha förfrågningar in-process. **Alla HTTP-verb** stöds — ange metoden som sträng, eller använd `import "http"` för tydliga hjälpare.

```kabootar
fn hello() {
    return http_response(200, "Hello Kabootar")
}

http_route("GET", "/hello", hello);
http_body(http_request("GET", "/hello"));   // Hello Kabootar
```

### `import "http"` — verb-hjälpare

| Kategori | Funktioner |
|----------|------------|
| Metodnamn | `method_get()`, `method_post()`, `method_put()`, `method_patch()`, `method_delete()`, `method_head()`, `method_options()` |
| Server-routes | `route_get`, `route_post`, `route_put`, `route_patch`, `route_delete`, `route_head`, `route_options` |
| In-process | `request_get`, `request_post`, `request_put`, `request_patch`, `request_delete`, `request_head`, `request_options` |
| In-process async | `request_*_async` (samma verb) |
| Extern fetch | `fetch_get`, `fetch_post`, `fetch_put`, `fetch_patch`, `fetch_delete`, `fetch_head`, `fetch_options` |
| Fetch + headers | `fetch_get_headers`, `fetch_post_headers`, … |
| Svar | `ok`, `created`, `no_content`, `not_found`, `method_not_allowed` |

```kabootar
import "http"

fn list()   { return ok("[]") }
fn create() { return created(req_body) }
fn update() { return ok(req_body) }
fn remove() { return no_content() }

route_get("/api/users", list)
route_post("/api/users", create)
route_put("/api/users/1", update)
route_patch("/api/users/1", update)
route_delete("/api/users/1", remove)

http_body(request_get("/api/users"))
http_status(request_delete("/api/users/1"))   // 204
```

Extern API (Promise — använd `await`):

```kabootar
import "http"

async fn load() {
    let res = await fetch_get("https://api.example.com/data")
    return http_body(res)
}

async fn save(data) {
    let res = await fetch_put_headers(
        "https://api.example.com/data",
        data,
        { "Content-Type": "application/json" }
    )
    return http_status(res)
}
```

Du kan fortfarande använda `http_route("PUT", …)` och `http_fetch_async("PATCH", …)` direkt — hjälparna är bara wrappers.

**Tips:** Döp inte handlers till `remove` — det krockar med den inbyggda `remove()` för filsystemet. Använd t.ex. `delete_user`.

## Request-kontext

Handlers har tillgång till:

| Variabel | Innehåll |
|----------|----------|
| `req_method` | `"GET"`, `"POST"`, … |
| `req_path` | `"/hello"` |
| `req_body` | Request body som sträng |

```kabootar
fn echo() {
    return http_response(200, req_body)
}

http_route("POST", "/echo", echo);
http_request("POST", "/echo", "ping");
```

## API

| Funktion | Beskrivning |
|----------|-------------|
| `http_route(method, path, handler)` | Registrera route |
| `http_request(method, path)` | Simulera request → response |
| `http_request(method, path, body)` | Request med body |
| `http_response(status, body)` | Skapa response |
| `http_status(response)` | Hämta statuskod |
| `http_body(response)` | Hämta body |
| `http_headers(response)` | Hämta headers som objekt (lowercase-nycklar) |
| `http_header(response, name)` | Hämta en header (case-insensitive), `undefined` om saknas |
| `http_process(raw)` | Parsa rå HTTP och returnera rå respons |

### Native (CLI/server)

| Funktion | Beskrivning |
|----------|-------------|
| `http_serve_once(port)` | Ta emot **en** TCP-anslutning (ej WASM) |

### Extern fetch (v2.9+)

| Funktion | Beskrivning |
|----------|-------------|
| `http_fetch_async(method, url, body)` | HTTP/TCP mot extern URL |
| `http_fetch_async(method, url, body, headers)` | Med custom headers-objekt (v2.12) |
| `http_fetch_async(method, url, body, headers, timeout_ms)` | Per-request timeout i ms (v2.16) |
| `http_set_timeout(ms)` | Global standard-timeout för fetch (0 = ingen, v2.16) |
| `http_reset_timeout()` | Återställ global fetch-timeout (v2.16) |

Objektnycklar kan vara **identifierare** (`Authorization`) eller **strängar** (`"Content-Type"`) — v2.13.

`http_fetch_async` följer automatiskt **redirects** (301/302/303/307/308, max 10 hopp). Vid 301/302/303 blir nästa request **GET** utan body.

**Timeout (v2.16):** Sätt global timeout med `http_set_timeout(5000)` (millisekunder), eller skicka timeout som 5:e argument. `0` betyder ingen timeout. Vid timeout avbryts connect/read/write med felmeddelande `HTTP fetch timed out after Nms`.

```kabootar
http_set_timeout(3000)
async fn load() {
    let res = await http_fetch_async("GET", "https://api.example.com/data", "")
    return http_body(res)
}
```

```kabootar
async fn load() {
    let res = await http_fetch_async(
        "POST",
        "https://api.example.com/data",
        "{\"ok\":true}",
        { "Content-Type": "application/json", Authorization: "Bearer tok" },
        5000
    )
    let ct = http_header(res, "content-type")
    return http_body(res)
}
```

## Rå HTTP

```kabootar
http_process("GET /ping HTTP/1.1\r\n\r\n")
// "HTTP/1.1 200 OK\r\nContent-Length: ...\r\n\r\npong"
```

## 404

Saknad route ger status `404`:

```kabootar
http_status(http_request("GET", "/missing"))   // 404
```

## Implementation

- Typer och router: `src/runtime/http.rs`
- Dispatch: `src/http_dispatch.rs`

Se [RUNTIME.md](RUNTIME.md) för backend-arkitektur.
