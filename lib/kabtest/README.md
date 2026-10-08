# kabtest

**kabtest** är Kabootars **testrunner och testharnäss** — en egen modul under `lib/kabtest/`, samma klass som kOS och kbrowser.

Den ska kunna:

1. **Testa Kabootar** — kompilator, VM, JIT, stdlib, CLI, själv-host — i `.kab`, inte som evigt `cargo test`.
2. **Testa andra språk** — gäster som landar i **Kv8** (in-process) eller som **process** (stdout/stderr/exit), med samma rapportformat.

Plan: [ROADMAP.md](ROADMAP.md). Befintlig tunn DX: [`import "test"`](../test.kab) + Rust `kabootar test` är **skuld** (SH25) tills kabtest äger CLI.

## Varför egen modul

`lib/test.kab` är asserts. `src/cli/test_runner.rs` (raderad — KT8/SH25-gaten passerad) hittade `*_test.kab` och körde med **Rust-compile + host-VM**. Det räcker inte för:

- self-host / Kab-VM / SH-gates
- gästspråk (Kv8, senare Python-klass, shell, …)
- CI utan rustc (SH28)

kabtest är **produkten**. `cargo test` blir bara kvarsittande host-skuld.

## Kontrakt

```
Testdefinition  (.kab | gästfil + expect)
        ↓
   kabtest discover / filter / timeout
        ↓
   runner ──┬── Kab: compile + eval (.kbc / Kab-VM)
            ├── Kv8: in-process gästscript
            └── proc: spawn kompilator/tolk (tills kOS-process räcker)
        ↓
   rapport  (konsol + TAP + JSON; JUnit deepen)
```

Inget test-API ska kräva Rust. Host-process är en **kapabilitet** (`os` / kOS), policyn ligger i `import "kabtest/…"`.

## Kab vs andra språk

| Mål | Hur kabtest kör det |
|-----|---------------------|
| `.kab` | Self-host compile → bytecode → Kab-VM (SH6); golden `.kbc` / `lastCompileMs` |
| Kv8 (JS-lik gäst) | `import "kv8/eval"` in-process; samma asserts |
| Annat språk med Kv8-backend | Gäst → Kv8 → samma runner |
| Främmande kompilator/tolk | Adapter: argv, cwd, env, timeout, jämför stdout/exit (golden) |
| Negativa test | Förväntat fel (compile-fel, throw, icke-noll exit) |

kabtest **är inte** en C++-kompilator. Den är harnäss + adapters. Nya språk = ny adapterfil, inte en ny testrunner.

## Import

```kabootar
import "kabtest"           // asserts (KT1)
import "kabtest/core"      // samma
import "kabtest/run"       // discover (KT2)
import "kabtest/eval"      // ktRunSource (KT2)
import "kabtest/tap"       // TAP (KT3)
import "kabtest/report"    // JSON-fil (KT3 deepen)
import "kabtest/self"      // KT4: 40+2 utan compile/boot-DAG
import "kabtest/guest_kv8" // KT5: Kv8-adapter (eval-DAG deepen)
import "kabtest/guest_proc" // KT6: golden stdout/exit (spawn deepen)
import "kabtest/cov"        // KT7: module-hit / percent
import "kabtest/cli"        // KT8: `kabootar test` argv (test_runner raderad)
import "kabtest/ci"         // KT9: produkt-gate vs cargo src-skuld
```

Idag: **KT1–KT8** klara — `test_runner.rs` är raderad, `kabootar test` kör kabtest på Kab-VM. **KT9**: `import "kabtest/ci"` (`ktCiIsProductGate`) + `.github/workflows/kabtest.yml` kör `kabootar test` som produkt-gate; `cargo test` täcker bara `src/`-skuld.

**Migration `tests/*.rs` → `*_test.kab` (första batchen):** 6 filer porterade och gröna via `kabootar test` — `v19_language`, `stdlib_wave`, `app_module`, `science_sc_wave12`, `sh6_vm_optimizations`, `generics` (in-line semantik + monomorfisering via `compileIr`-records: `id$Number`, `Box$*`, `Option$*`, `echo$Number`, `extends`). Rust-originalen ligger kvar tills .kab-täckningen är ekvivalent — legs som är host-yta (AST-fält, host-fel-texter) eller riktiga Kab-VM-luckor (struct-metodanrop, ärvd `init`, `implements Trait<T>`) stannar som dokumenterad debt i filhuvudena. `nollKabtestProductCi` stannar `false` tills sviten är komplett.

## Relaterat

- SH25 / SH28: [docs/ROADMAP.md](../../docs/ROADMAP.md)
- DX asserts: [docs/DX_TOOLING.md](../../docs/DX_TOOLING.md)
- Kv8: [../kv8/README.md](../kv8/README.md)
- kOS process: [../kos/README.md](../kos/README.md)
