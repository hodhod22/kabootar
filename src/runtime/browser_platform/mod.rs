//! Browser Platform v2 — post-Kv8: WASM, WebGL, WebRTC, DevTools, Extensions, PWA.

pub mod canvas_host;
pub mod canvas_props;
pub(crate) mod canvas_register;
mod devtools;
mod extensions;
mod json_util;
mod pwa;
mod register;
mod wasm_guest;
pub(crate) mod webgl;
pub(crate) mod webgl_register;
mod webrtc;

pub use devtools::{dom_tree_value, kv8_console_log, shell_snapshot_from_env};
pub use extensions::inject_on_navigate;
pub use register::browser_platform_globals;
pub use wasm_guest::{load_wasm, minimal_add_wasm};
