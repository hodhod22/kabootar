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
| **KT1** | `lib/kabtest/` + `import "kabtest"` (asserts) | Smoke `examples/kabtest/kabtest_smoke.kab` | ✅ subset |
| **KT2** | Discover `*_test.kab` / `*.test.kab`; kör fil → pass om `true` / `{ ok: true }` | En katalog körs utan Rust-test_runner | ✅ real eval: `ktEvalSource` kör källan via `evalSourceKabVm` (Kab-VM); throw-guard (fn+try wrapper) gör kast till `{ok,err}`-records; `ktDiscover`/`ktRunDir`/`ktRunFile`/`ktTapReport` i `kabtest/runner` via VFS `os_list`/`os_read` — `kabtest_engine_smoke`. Känt gap: källor med top-level `import` kan inte throw-guardas (en kast där korsar eval-gränsen fatalt); `kabootar test`-kommandot (KT8) är fortfarande Rust `test_runner` |
| **KT3** | Reporter: konsol + TAP + JSON-fil | Maskinläsbar CI | ✅ subset: TAP + `kabtest/report` `ktJsonResult` / `ktJsonWrite` (os_write); JUnit deepen |
| **KT4** | Suites för Kab: tiny compile, `bootPipelineOk`, `jitGprCount`, VM `40+2` | Ersätter en bit `tests/sh_wave` i `.kab` | ✅ subset: JIT + `ktSelfArith`. `import "kab/boot"` från app: SH16 `@version` / stack — rör inte `boot.kab` (knäcker övriga smokes) |
| **KT5** | Guest **Kv8**: eval källtext, assert resultat | En JS-lik fil i suite | ✅ subset: `kabtest/guest_kv8` + `ok.kv8`; `kv8/eval` DAG/`@version` deepen |
| **KT6** | Guest **proc**: spawn, timeout, golden stdout/exit | t.ex. `python -c` *eller* Kab-binär — adapter, inte hårdkodat språk | ✅ subset: `kabtest/guest_proc` + `ok.golden`; `os_spawn`/timeout deepen |
| **KT7** | Coverage: importerade moduler + rad-approx i Kab | Rapport utan `src/cli/test_runner` coverage | ✅ subset: `kabtest/cov` `ktCovPct` / `ktCovIsMod` / `ktCovLineHint`; instrumentation deepen |
| **KT8** | `kabootar test` anropar kabtest; radera `test_runner.rs` | SH25 delete-gate | ✅ subset: `kabtest/cli` `ktCliIsTest` / `ktCliDefaultRoot` / `ktCliIsCoverage` / `ktCliExit` / `ktCliHostDeleteOk`; **CLI-bryggan är kopplad** — `test_cmd` monterar testroten i VFS (`/kt_test_root`, env `KAB_KT_ROOT`/`KAB_KT_FILE`/`KAB_KT_COV`) och evaluerar `kabtest/cli_main` på Kab-VM:n — `kabootar test tests/dx_smoke_test.kab` ger `1 passed` via `ktRunDir`. Rekursiv discovery via `ktWalk` (os_list-baserad, kastar aldrig på saknad dir). Kvar: `test_runner.rs` står kvar som fallback-skuld — `ktCliHostDeleteOk`/`gcHostDeleteOk`-gaten förblir false tills hela produktsviten körs grönt via Kab-vägen |
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
