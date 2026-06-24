# Deno-paritet i Kabootar

Kabootar är **inte** Deno/TypeScript, men flera **Deno runtime-API:er** har motsvarigheter för enklare porting.

## API-mappning

| Deno | Kabootar | Not |
|------|----------|-----|
| `Deno.env.get(k)` | `env_get(k)` / `Deno_env_get(k)` | Host-`std::env` + `env_set` |
| `Deno.env.set(k, v)` | `env_set(k, v)` / `Deno_env_set(k, v)` | |
| `Deno.env.has(k)` | `env_has(k)` | |
| `Deno.env.delete(k)` | `env_delete(k)` | |
| `Deno.env.toObject()` | `env_to_object()` | |
| `Deno.serve(handler)` | `serve_handler(fn)` + `http_serve(port)` eller `serve(port, fn)` | Blockerande loop (native) |
| `Request` | Request-objekt `{ method, url, body, headers }` | Via `serve_handler` |
| `Response` | `response_new(status, body, headers?)` | Returneras från handler |
| `request.method` | `request_method(req)` | |
| `request.url` | `request_url(req)` | Path som URL i v1 |
| `request.text()` | `request_body(req)` | |
| `ReadableStream` | `stream_from_array`, `stream_read`, `stream_read_all` | Förenklad v1 |
| `WebSocket` (par) | `ws_channel_pair`, `ws_link`, `ws_send`, `ws_recv` | In-process |
| `WebSocket` (TCP) | `ws_connect(url)` | `ws://` och `wss://` (våg 3) |
| `Deno.connect` | `tcp_connect(host, port)` / `Deno_connect` | Rå TCP |
| `Deno.listen` | `tcp_listen(host, port)` / `Deno_listen` | Rå TCP |
| TCP I/O | `tcp_accept`, `tcp_read`, `tcp_write`, `tcp_close` | |
| `Deno.startTls` | `tcp_start_tls(socket, hostname)` / `Deno_startTls` | Uppgraderar TCP → TLS våg 13 |
| `Deno.run` | `deno_run(name)` / `Deno_run` | Sandbox-process |
| Streams `tee`/`pipeTo` | `stream_tee`, `stream_pipe_to` | Förenklad v3 |
| Stream backpressure | `stream_locked`, `stream_lock`, `stream_desired_size`, `writable_desired_size` | våg 4 |
| `Deno.chdir` | `chdir()` / `Deno_chdir()` | |
| `Deno.resolveDns` | `resolve_dns(host, port?)` | |
| UDP | `udp_bind`, `udp_send`, `udp_recv`, `udp_close` | våg 4 |
| `Deno.Command` | `run_command(program, args?)` / `Deno_command` | Host-subprocess våg 4 |
| `Deno.cwd()` | `cwd()` / `Deno_cwd()` | Host working directory |
| `Deno.readTextFile` | `read_text_file(path)` / `Deno_readTextFile(path)` | Alias till `os_read` |
| `Deno.writeTextFile` | `write_text_file(path, text)` | Alias till `os_write` |
| `Deno.readFile` | `read_file(path)` → byte-array | våg 12 |
| `Deno.writeFile` | `write_file(path, data)` | string eller byte-array våg 12 |
| `Deno.readDir` | `read_dir(path)` → `[{ name, isFile, isDirectory }]` | våg 12 |
| `Deno.mkdir` | `mkdir(path)` | våg 12 |
| `Deno.stat` | `stat(path)` → `{ isFile, size, mtime, ... }` | våg 12 |
| `Deno.remove` | `remove(path)` | våg 12 |
| `Deno.exists` | `exists(path)` | våg 12 |
| `ReadableStream` | `stream_new`, `stream_from_string`, `stream_cancel` | Förenklad v2 |
| `WritableStream` | `writable_stream_new`, `writable_write`, `writable_close` | Förenklad v2 |
| Async Web Streams | `stream_read_async`, `stream_read_all_async`, `stream_pipe_to_async`, `reader_read_async` | Promise + IO-kö våg 5 |
| Full Web Streams (våg 15) | `stream_get_reader`, `reader_read`, `reader_cancel`, `writable_get_writer`, `writer_write`, `writer_abort`, `transform_stream_new`, `byte_stream_*`, `stream_transfer`, `stream_abort`, `stream_state`, `stream_enqueue` | WHATWG reader/writer, bytes, transfer |
| Unix sockets | `unix_connect`, `unix_listen`, `unix_accept`, `unix_read`, `unix_write`, `unix_close` | Unix våg 5 |
| `Deno.openKv` | `open_kv`, `kv_get`, `kv_set`, `kv_delete`, `kv_list`, `kv_close` / `Deno_openKv` | Kabootar SQL (`_kab_kv` + WAL) våg 6 |
| `Kv.watch` | `kv_watch(kv, prefix?)` → ReadableStream | våg 7 |
| `Kv.atomic` | `kv_atomic(kv, ops)` — `set`/`delete`/`get`/`check` i en transaktion | våg 7 |
| Delad DB | `open_kv_db()` efter `db_open(path)` — samma SQL-motor | våg 8 |
| Versionstamps | `kv_get_entry`, `kv_get_version`, `check` med `version` i `kv_atomic` | våg 8 |
| `Kv.listen` | `kv_listen`, `kv_listen_recv`, `kv_listen_close` | våg 8 |
| Atomic `sum`/`max`/`min` | `kv_atomic` ops `sum`, `max`, `min` | våg 9 |
| Queue | `kv_enqueue`, `kv_dequeue`, atomic `enqueue` | våg 9 |
| List m. version | `kv_list_entries` | våg 9 |
| Async watch | `kv_listen_async`, `kv_watch_async` | våg 9 |
| Workers | `worker_new`, `worker_start`, `worker_start_file`, `worker_post_message`, `worker_recv`, `worker_recv_async`, `worker_onmessage`, `worker_poll_async`, `worker_join`, `worker_terminate`, `importScripts`, `onmessage`, `postMessage`, `worker_poll_wait`, `worker_run_message_loop` | OS-tråd-isolat våg 10–11 |
| FFI | `ffi_load`, `ffi_call`, `ffi_close` | `libloading` våg 5 |
| npm / TypeScript | `npm_install`, `npm_fetch`, `jsr_fetch`, `npm_resolve`, `npm_parse_spec`, `npm_list_cache`, `npm_import`, `import "npm:…"`, `import "jsr:…"`, `ts_transpile`, `ts_strip_types`, `ts_compile`, `ts_compile_file`, `Deno_emit` | npm/JSR våg 14, TS våg 16 |
| Node.js compat | `node_resolve`, `node_list`, `node_import`, `import "node:fs"`, `import "node:path"`, `import "node:process"`, `import "node:os"`, `import "node:url"`, `import "node:buffer"`, `import "node:crypto"`, `import "node:fs/promises"` | våg 17 |
| SharedArrayBuffer | `sab_new`, `sab_transfer`, `sab_from_transfer`, `uint8_array_*`, `int32_array_*`, `atomics_*`, `worker_post_message(w, msg, [sab])` | våg 18 |
| `Deno.serve` (async) | `serve_dispatch(handler, method, path, body?)`, `serve_async_ready(handler, port?)` | våg B1 — synk dispatch + ready-Promise (ingen HTTP/2-loop än) |
| `Deno.permissions` | `permissions_query/request/revoke/grant`, `Deno_permissions` | våg B3 |
| `Deno.test` / `Deno.bench` | `deno_test`, `deno_bench`, `deno_test_report`, `deno_bench_report` | våg B4 |
| Lockfile | `lockfile_read`, `lockfile_sync` → `kabootar.lock` | våg B5 |
| `Deno.realPath` / `symlink` / `link` | `realpath`, `symlink`, `link` | våg B6 |
| `Deno.listenTls` | `tls_listen`, `tls_reload_certs`, `tls_accept`, `tls_server_*` | våg B7 (native) |
| SharedWorker | `shared_worker_connect`, `shared_worker_post_message`, `shared_worker_recv` | våg B8 — in-process |

## Exempel: Deno.serve-lik handler

```kabootar
serve_handler((req) => {
    if request_url(req) == "/health" {
        return response_new(200, "ok")
    }
    return response_new(404, "not found")
})

// Testa utan att binda port:
let raw = "GET /health HTTP/1.1\r\n\r\n"
println(http_process(raw))
```

## Exempel: env

```kabootar
env_set("APP_ENV", "dev")
println(env_get("APP_ENV"))
```

## Exempel: stream

```kabootar
let s = stream_from_array([1, 2, 3])
let chunk = stream_read(s)
stream_read_all(s)
```

## Exempel: WebSocket TCP

```kabootar
let ws = ws_connect("ws://127.0.0.1:8080/")
ws_send(ws, "hello")
println(ws_recv(ws))
```

## Exempel: fil-API

```kabootar
println(cwd())
let text = read_text_file("README.md")
write_text_file("out.txt", text)
```

## Exempel: Deno.fs (våg 12)

```kabootar
mkdir("/data")
write_file("/data/app.bin", [72, 105])
let info = stat("/data/app.bin")
println(info["isFile"])
println(read_file("/data/app.bin"))
println(read_dir("/data"))
remove("/data/app.bin")
```

## Exempel: TLS på TCP (våg 13)

```kabootar
tls_add_ca(pem_text)   // eller tls_ca_only(pem) för test-CA
let sock = tcp_connect("example.com", 443)
tcp_start_tls(sock, "example.com")
tcp_write(sock, "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
println(tcp_read(sock, 4096))
```

## Exempel: npm / JSR (våg 14)

```kabootar
let spec = npm_parse_spec("jsr:@std/fmt@1.0.0")
npm_fetch("npm:is-number", "7.0.0")      // cache under .kabootar/npm/
jsr_fetch("@std/fmt", "1")               // cache under .kabootar/jsr/
let src = npm_import("npm:is-number", "7")
println(len(npm_list_cache()))
import "npm:math-lite@1.0.0"             // Kabootar-kompatibel entry i cache
```

## Exempel: Full Web Streams (våg 15)

```kabootar
let pair = transform_stream_new((chunk) => chunk)
let reader = stream_get_reader(pair["readable"])
writable_write(pair["writable"], "hi")
writable_close(pair["writable"])
println(reader_read(reader)["value"])

let bytes = byte_stream_from_bytes([72, 105])
println(byte_stream_read(bytes, 8))

let token = stream_transfer(stream_from_array([1, 2]))
worker_post_message(w, { "ok": true }, [stream_from_array([3])])
```

## Exempel: TypeScript (våg 16)

```kabootar
let out = ts_compile("interface User { id: number }\nlet id: number = 1")
println(out["code"])
println(len(out["diagnostics"]))

let file = ts_compile_file("app.ts")
println(Deno_emit("enum Color { Red, Green }"))
```

## Exempel: Node.js-compat (våg 17)

```kabootar
import "node:path"
import "node:fs"

println(join("/tmp", "out.txt"))
println(node_list())
println(node_resolve("node:fs"))

mkdirSync("/data")
writeFileSync("/data/app.bin", [72, 105])
println(readFileSync("/data/app.bin"))
```

`node:fs/promises`, `node:process`, `node:os`, `node:url`, `node:buffer`, `node:crypto` finns också (sync-shim för promises i v17).

## Exempel: SharedArrayBuffer (våg 18)

```kabootar
let sab = sab_new(16)
let bytes = uint8_array_new(sab, 0, 4)
uint8_array_set(bytes, 0, 42)

let ints = int32_array_new(sab, 0, 2)
atomics_store(ints, 0, 100)
println(atomics_add(ints, 0, 1))

let w = worker_new()
worker_post_message(w, { "go": true }, [sab])
// Worker: let sab2 = worker_poll()["transfers"][0]; atomics_add(...)
```

## Exempel: Wave B — serve, permissions, lockfile

```kabootar
fn handler(req) {
    return response_new(200, request_url(req))
}
let res = serve_dispatch(handler, "GET", "/api/health")
println(res["status"])

permissions_grant({ "name": "read", "path": "/tmp" })
println(permissions_query({ "name": "read", "path": "/tmp" }))

deno_test("smoke", fn() { 1 + 1 })
println(deno_test_report()["passed"])

let lf = lockfile_read()
println(lf["version"])
```

## Exempel: Wave B — paths, TLS, SharedWorker

```kabootar
println(realpath("."))

// TLS (native host):
// let listener = tls_listen("127.0.0.1", 8443, cert_pem, key_pem)
// let sock = tls_accept(listener)

let id1 = shared_worker_connect("pool")
let id2 = shared_worker_connect("pool")
println(id1 == id2)
```

## Saknas ännu (Deno)

- Full async `Deno.serve`-loop med HTTP/2
- Riktig multi-isolate SharedWorker (nu: namn→worker-id i processen)

## Exempel: openKv

```kabootar
let kv = open_kv("data.kdb")   // samma motor som db_open / sql()
kv_set(kv, ["app", "version"], "1.0")
println(kv_get(kv, ["app", "version"]))
kv_close(kv)
```

## Exempel: Worker

```kabootar
let w = worker_new()
worker_post_message(w, "ping")
worker_start(w, "worker_reply(worker_poll())")
worker_join(w)
println(worker_recv(w))
```

```kabootar
async fn main() {
    let w = worker_new()
    worker_post_message(w, "ping")
    worker_start(w, "worker_reply(worker_poll())")
    println(await worker_recv_async(w))
    worker_join(w)
}
```

## Exempel: async stream

```kabootar
async fn drain() {
    let s = stream_from_array([1, 2, 3])
    return await stream_read_all_async(s)
}
println(await drain())
```

## Exempel: watch + atomic batch

```kabootar
let kv = open_kv("data.kdb")
let changes = kv_watch(kv, ["users"])

kv_atomic(kv, [
    { "op": "set", "key": ["users", "1"], "value": "alice" },
    { "op": "check", "key": ["users", "1"], "value": "alice" }
])

let ev = stream_read(changes)
println(ev["kind"])   // "set"
kv_close(kv)
```

## Kabootar-unikt (finns inte i Deno)

- `sql()` / in-process databas
- `http_route` / in-process router
- `import "science"`, KML, bytecode-VM

Se [FEATURES.md](FEATURES.md) för full JS-paritet och [JAVASCRIPT.md](JAVASCRIPT.md) för språkskillnader.
