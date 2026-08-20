# kabtest — roadmap

**Mål:** Kabootar testar **sig själv** och **andra språk** med en runner skriven i `.kab`. `cargo test` och `src/cli/test_runner.rs` är skuld.

**Klart när:** produkt-CI kör `kabootar test` via kabtest (inte rustc) för `.kab`-gates; minst en gästadapter (Kv8) och en process-adapter (golden stdout) är gröna.

**Icke-mål:** att bli LLVM-lit, att kräva pytest/jest som runtime, att lägga ny testlogik i `src/`.

## Ordning (hoppa inte)

```
KT0 inventering
  → KT1 lib/kabtest core (asserts)
  → KT2 discover + run .kab i Kab (inte tvinga KABOOTAR_COMPILE=rust)
  → KT3 rapport TAP/JSON
  → KT4 Kabootar self-test (compiler/VM/JIT-smoke som .kab)
  → KT5 Kv8-gäst in-process
  → KT6 process-adapter (andra språk / binärer)
  → KT7 coverage i Kab
  → KT8 CLI `kabootar test` = kabtest (radera test_runner.rs)
  → KT9 CI-gates utan cargo test för produkt
```

## Steg

| Steg | Vad | Gate | Status |
|------|-----|------|--------|
| **KT0** | Kartlägg `lib/test.kab`, `kabootar test`, `cargo test`, DX-coverage | Denna roadmap + README | ✅ |
| **KT1** | `lib/kabtest/` + `import "kabtest"` (asserts) | Smoke `examples/kabtest_smoke.kab` | ✅ subset |
| **KT2** | Discover `*_test.kab` / `*.test.kab`; kör fil → pass om `true` / `{ ok: true }` | En katalog körs utan Rust-test_runner | ✅ subset: discover + `ktRunSource`; Kab-VM eval väntar på SH6 (för stor import-DAG) |
| **KT3** | Reporter: konsol + TAP + JSON-fil | Maskinläsbar CI | ✅ subset: TAP + `kabtest/report` `ktJsonResult` / `ktJsonWrite` (os_write); JUnit deepen |
| **KT4** | Suites för Kab: tiny compile, `bootPipelineOk`, `jitGprCount`, VM `40+2` | Ersätter en bit `tests/sh_wave` i `.kab` | ✅ subset: JIT + `ktSelfArith`. `import "kab/boot"` från app: SH16 `@version` / stack — rör inte `boot.kab` (knäcker övriga smokes) |
| **KT5** | Guest **Kv8**: eval källtext, assert resultat | En JS-lik fil i suite | ✅ subset: `kabtest/guest_kv8` + `ok.kv8`; `kv8/eval` DAG/`@version` deepen |
| **KT6** | Guest **proc**: spawn, timeout, golden stdout/exit | t.ex. `python -c` *eller* Kab-binär — adapter, inte hårdkodat språk | ✅ subset: `kabtest/guest_proc` + `ok.golden`; `os_spawn`/timeout deepen |
| **KT7** | Coverage: importerade moduler + rad-approx i Kab | Rapport utan `src/cli/test_runner` coverage | ✅ subset: `kabtest/cov` `ktCovPct` / `ktCovIsMod` / `ktCovLineHint`; instrumentation deepen |
| **KT8** | `kabootar test` anropar kabtest; radera `test_runner.rs` | SH25 delete-gate | ✅ subset: `kabtest/cli` `ktCliIsTest` / `ktCliDefaultRoot` / `ktCliExit`; Rust `test_cmd` + delete deepen |
| **KT9** | Produkt-CI: kabtest-gates; `cargo test` bara kvarvarande `src/`-skuld | SH28 närmare | ✅ subset: `kabtest/ci` `ktCiIsProductGate` / `ktCiCargoForSrcSkuld`; workflow utan rustc deepen |

## Adapters (KT5–KT6)

Varje gäst är en `.kab`-fil, t.ex. `kabtest/guest_kv8.kab`, `kabtest/guest_proc.kab`:

- `canRun(spec)` — känner fil/typ
- `run(spec)` — `{ ok, out, err, code }`
- `spec` har `path` / `source` / `expect` / `timeoutMs`

Nya språk = ny guest-fil + registrering. Ingen ny C-testrunner.

## Koppling till huvudplanen

| Huvudsteg | kabtest |
|-----------|---------|
| SH16 | Tester kompileras som appar (ingen rust-fallback) |
| SH6 | Runner eval:ar på Kab-VM |
| SH17 | JIT-smoke som kabtest-suite (KT4) |
| SH21 | Process-adapter via kOS/os i stället för Rust `std::process` |
| SH25 | CLI i Kab |
| SH28 | rustc inte i testvägen för produkten |
