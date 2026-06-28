//! Kv8 module root — Kabootar's own JS/DOM/CSS engine.

mod bundle;
mod ast;
mod bytecode_bridge;
mod context;
mod eval;
mod jit;
mod lexer;
mod opt;
mod promise;
mod register;
mod smoke;
mod vfs_module;

pub use ast::{Expr, Kv8Program, Stmt};
pub use context::{Kv8Context, Kv8Value};
pub use eval::{dom_to_kabootar, drain_event_loop, drain_microtasks, drain_timers, eval_script, parse_program, run_program};
pub use register::{kv8_globals, kv8_register};
pub use bundle::{
    load_react_dom_runtime, load_react_dom_umd, load_react_runtime, load_react_umd, react_bundle_info,
};
pub use smoke::{minimum_app_shell, probe_report_value, react_bundle_smoke_path, react_smoke_path, run_all_probes, SmokeProbe, SmokeResult, PROBES};
pub use vfs_module::{parse_kv8_module, Kv8Module};
