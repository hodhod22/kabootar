# Kabootar — projekt (v2.1)

## Snabbstart

```bash
kabootar mod init api
kabootar compile main.kab  # bytecode-cache (v2.18)
kabootar serve --watch --port 8080 main.kab
```

## `kabootar.toml`

```toml
version = "0.1.0"
template = "api"
entry = "main.kab"
port = 8080

[dependencies]
greet = "1.0.0"
```

| Fält | Beskrivning |
|------|-------------|
| `version` | Projektsversion |
| `template` | `web` eller `api` |
| `entry` | Startfil (standard `main.kab`) |
| `port` | HTTP-port för `kabootar serve` |
| `[dependencies]` | Modulversioner (matchar `@version` i `.kab`) |

## Kommandon

| Kommando | Beskrivning |
|----------|-------------|
| `kabootar` | Interaktiv REPL |
| `kabootar run <fil>` | Kör ett script |
| `kabootar compile <fil>` | Bytecode-cache (`.kabootar/cache/*.kbc`) när kompilerbart (v2.18) |
| `kabootar publish <fil\|namn>` | Publicera modul till `.kabootar/registry/` (v2.17) |
| `kabootar install [namn@ver]` | Installera från registry till `.kabootar/packages/` (v2.17) |
| `kabootar serve [--watch] [--port N] <fil>` | HTTP-server |
| `kabootar mod init <mall>` | Skapa nytt projekt |
| `kabootar mod run` | Kör `entry` från `kabootar.toml` |

## Moduler

```kabootar
// lib/config.kab
@version "1.0.0"
pub let API_NAME = "Kabootar"
pub fn hello() { return API_NAME }
```

```kabootar
import "config@1.0"
hello()
```

## Hot reload

`--watch` övervakar `main.kab`, `lib/*.kab` och beroenden — laddar om routes vid ändring.

## Deploy

```bash
kabootar serve --bind 0.0.0.0 --port 8080 main.kab
```

Eller från Kabootar:

```kabootar
http_serve(8080, "0.0.0.0")
```
