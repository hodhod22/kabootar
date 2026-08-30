<p align="center">
  <img src="assets/logo.png" alt="Kabootar logo" width="128">
</p>

# Kabootar

Fullstack-programmeringsspråk — tidigare kallat Nova.

**Slutmål:** hela produkten är `.kab` — kompilator, VM, JIT, GC, CLI, stdlib, OS, browser. Rust i `src/` är **skuld** som ska **ersättas och raderas** ([SH28](docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust)). Ny `.rs`-feature är regression. En användare ska bygga och köra Kabootar **utan rustc**.

**[📖 Dokumentation](docs/README.md)** · **[Roadmap — noll Rust](docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust)**

```bash
kabootar              # REPL (produkt-CLI när den finns; idag bootstrap)
cargo run             # samma REPL via rustc — bootstrap/skuld, inte tak
cargo test            # host-tester — skuld tills SH25/SH28 (`kabootar test`)
```

Produktkompilatorn är `self_host/compile.kab`. Körning är Kab-VM (**kab-only default**). Plan och **Nästa:** [docs/ROADMAP.md](docs/ROADMAP.md). Vision: [docs/OVERVIEW.md](docs/OVERVIEW.md).

## Licens

Kabootar (språk, runtime, OS och webbläsare — första-parts-kod) är
**[MIT](LICENSE)**. Tredjepartsbibliotek behåller egna licenser — se
[THIRD_PARTY.md](THIRD_PARTY.md).

Kommersiella paket ovanpå (t.ex. spel-ramverk i stil med Unity) kan ha
egen licens; själva Kabootar-plattformen är fri MIT.
