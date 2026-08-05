# Kabootar — desktop ship (GP5a subset)

En binär med GPU för att köra spel/demos och (valfritt) shell.

## Bygg

```bash
# CLI + GPU 3D
cargo build --release --features "default,gpu"

# Desktop window (winit) + GPU present
cargo build --release --bin kabootar --features "default,shell,gpu"
# optional host audio: add ,hw
```

## Kör

```bash
./target/release/kabootar run examples/game_3d_triangle.kab
./target/release/kabootar run examples/game_2d_smoke.kab
kabootar mod init game3d && kabootar mod run

# Shell (kräver shell-feature)
cargo run --features "docai,codai,shell,gpu" --bin kabootar -- shell
```

## Smoke

- `tests/ship_desktop_smoke.rs` — 3D surface present
- Med `--features gpu`: `webgl_info()["gpu3d"]` ska vara `wgpu-pipeline*` när adapter finns

Se [GAME.md](GAME.md), [RENDERING.md](RENDERING.md), [OS.md](OS.md).
