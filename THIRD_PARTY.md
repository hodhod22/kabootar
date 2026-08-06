# Third-party licenses

Kabootar’s **first-party** code is under the [MIT License](LICENSE).
Dependencies keep their **own** licenses. MIT does not relicense them.

This file is a high-level summary of crates commonly linked into Kabootar
builds. For authoritative SPDX identifiers, see each crate on crates.io /
its repository. When redistributing binaries, retain copyright and NOTICE
files required by those licenses (especially Apache-2.0).

## Typical stack (permissive)

| Area | Examples | Common license |
|------|----------|----------------|
| Crypto | `aes-gcm`, `chacha20poly1305`, `sha2`/`sha3`, `argon2`, `rsa`, `p256` | MIT / Apache-2.0 |
| Text / regex | `regex`, `fancy-regex`, `unicode-normalization` | MIT / Apache-2.0 |
| Serialization | `serde_json` | MIT / Apache-2.0 |
| Images / fonts | `png`, `fontdue` | MIT / Apache-2.0 / Zlib (per crate) |
| Numerics | `num-bigint`, `num-traits`, `matrixmultiply` | MIT / Apache-2.0 |
| TLS (host) | `rustls`, `webpki-roots` | Apache-2.0 / ISC / MIT (per crate) |
| Data (host) | `parquet`, `arrow-array`, `arrow-schema` | Apache-2.0 |
| Optional GPU/UI | `wgpu`, `winit`, `softbuffer` | MIT / Apache-2.0 |
| Optional HW | `cpal`, `serialport`, `nusb`, `hidapi` | Apache-2.0 / MIT / etc. |
| WASM | `wasm-bindgen`, `web-sys`, `js-sys`, `wasmi` | MIT / Apache-2.0 |

Kabootar intentionally prefers **MIT / Apache-2.0 / BSD**-class dependencies so
the first-party MIT license does not conflict with copyleft obligations from
the dependency graph. Do not add GPL/LGPL dependencies without an explicit
project decision and documentation update here.
