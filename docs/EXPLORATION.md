# Kabootar Exploration — REPL & notebook (vs Python)

Python är starkt för **utforskning** (IPython, Jupyter). Kabootar ska vinna genom **samma session → samma runtime → ship** utan venv/kernel-split.

## Snabbstart

```bash
# Modern REPL (persistent env)
kabootar

> :science
> let a = nd_from([[1.0, 2.0], [3.0, 4.0]])
> nd_matmul(a, nd_from([[1.0, 0.0], [0.0, 1.0]]))
> _
> :quit

# Notebook (.knb)
kabootar notebook run examples/explore_smoke.knb --science

# Web UI (WASM optional)
# öppna kabootar-notebook.html efter wasm-bindgen till ./pkg
```

## REPL-kommandon

| Kommando | Betydelse |
|----------|-----------|
| `:help` | Hjälp |
| `:quit` | Avsluta |
| `:reset` | Ny miljö |
| `:load file.kab` | Kör fil i sessionen |
| `:vars` | Lista bindings |
| `:science` | `import "science"` |
| `:type name` | Typ/kind för binding |
| `_` | Senaste resultat |

**Multiline:** öppna `{`/`(`/`[` eller avsluta rad med `\`.

## `.knb`-format

```json
{
  "version": 1,
  "cells": [
    { "id": "c1", "source": "1 + 2" },
    { "id": "c2", "source": "let x = _\nx * 10" }
  ]
}
```

Celler delar `Session` (samma env). Flagga `--science` förladdar science-natives.

## Varför detta slår Python för *produkt*-utforskning

| | Kabootar | Python |
|--|----------|--------|
| STEM | `import "science"` i REPL/notebook | NumPy/pandas i venv |
| UI / OS | Canvas, kOS, HTTP i samma runtime | Separata kernels / processer |
| Ship | Cell → `.kab` / `mod run` | Ofta omskrivning ur notebook |
| Deploy | En binär / WASM | pip + runtime + notebooks-server |

## Roadmap

Se [ROADMAP.md](ROADMAP.md) **Våg DX** (DX0–DX6 ✅; DX7 Kab-session 📋) och **Våg SC**. Rich display: `session_eval_rich` + `rich_display` (HTML-tabell / plot-image). Readline: `~/.kabootar_history`. WASM: `session_eval` / `session_eval_rich` / `session_science` / `session_reset`.
