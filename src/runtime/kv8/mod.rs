//! Kv8 module root — Kabootar's own JS/DOM/CSS engine.

mod ast;
mod bytecode_bridge;
mod context;
mod eval;
mod jit;
mod lexer;
mod opt;
mod register;
mod vfs_module;

pub use ast::{Expr, Kv8Program, Stmt};
pub use context::{Kv8Context, Kv8Value};
pub use eval::{dom_to_kabootar, eval_script, parse_program, run_program};
pub use register::{kv8_globals, kv8_register};
pub use vfs_module::{parse_kv8_module, Kv8Module};
