# Kabootar — desktop ship (GP5a) + self-sufficiency (GP5d)

En binär med GPU för att köra spel/demos och (valfritt) shell — **utan** Unity, Unreal eller C#-toolchain.

## Bygg

```bash
# CLI + GPU 3D
cargo build --release --features "default,gpu"

# Desktop window (winit) + GPU present
cargo build --release --bin kabootar --features "default,shell,gpu"
# optional host audio: add ,hw
```

Alternativ: CI bygger en nedladdningsbar release-binär — `.github/workflows/kabootar-artifact.yml` → artefakt `kabootar-linux-x64` (30 dagars retention). Fortfarande rustc-producerad; den är producent-sidan mot en framtida rustc-fri konsument (`nollRustcCiGone`/`nollUserNoRustc` = false).

## Kör

```bash
./target/release/kabootar run examples/game/game_3d_triangle.kab
./target/release/kabootar run examples/game/game_2d_smoke.kab
kabootar mod init game3d && kabootar mod run

# Shell (kräver shell-feature)
cargo run --features "docai,codai,shell,gpu" --bin kabootar -- shell
```

## Host / WASM (GP5b)

```kabootar
import "game/host"
let surf = createHostSurface(320, 240)
presentOnce(surf)   // layer == "host"
```

På native = KDOM-compositor med `layer: "host"`. På `wasm32` = web_sys canvas.

## GP5d — självständighet (checklista)

- [x] Bygg/kör med endast Kabootar + Rust (`cargo` + `kabootar`)
- [x] Spelmallar: `kabootar mod init game|game3d`
- [x] Assets: VFS + `import "game/assets"` + hot reload (`game/hot`)
- [x] Editor-data: `import "game/editor"` (hierarki/inspector)
- [x] Ship smoke: `tests/ship_desktop_smoke.rs`, `tests/game_host.rs`
- [x] Ingen Unity Hub / Unreal / .NET game workload krävs
- [x] CI release-artefakt: `kabootar-artifact.yml` → `kabootar-linux-x64` (rustc-byggd; rustc-fri kanal = SH28)
- [ ] rustc-fri distribuerbar binär (AOT-image eller release-asset utan cargo — `nollUserNoRustc`)

## Smoke

- `tests/ship_desktop_smoke.rs` — 3D surface present
- Med `--features gpu`: `webgl_info()["gpu3d"]` ska vara `wgpu-pipeline*` när adapter finns

Se [GAME.md](GAME.md), [RENDERING.md](RENDERING.md), [OS.md](OS.md).
