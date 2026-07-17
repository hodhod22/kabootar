pub mod ownership_check;
pub mod bytecode;
pub mod cli;
pub mod compile;
pub mod ops;
pub mod http_dispatch;
pub mod modules;
pub mod project;
pub mod registry;
pub mod sql;
pub mod kml;
pub mod kstyle_preprocess;
pub mod lang_preprocess;
pub mod span;
pub mod lexer;
pub mod generics;
pub mod ast;
pub mod parser;
pub mod value;
pub mod class;
pub mod shell;
pub mod runtime;
pub mod evaluator;
pub mod language;
pub mod docai;
pub mod codai;

pub use value::{format_value, Environment, Value};

use wasm_bindgen::prelude::*;
use evaluator::{create_global_env, eval_source};

/// Evaluate Kabootar source code and return the last expression value as a string.
#[wasm_bindgen]
pub fn evaluate(code: &str) -> String {
    let mut env = create_global_env();
    match eval_source(code, &mut env) {
        Ok(val) => format_value(&val),
        Err(e) => e,
    }
}

/// Returns the last compositor HTML frame from Kabootar browser paint.
#[wasm_bindgen]
pub fn kb_last_frame() -> String {
    runtime::frame_buffer::last_frame_html().unwrap_or_default()
}

/// Returns pixel buffer metadata from last compositor frame.
#[wasm_bindgen]
pub fn kb_last_pixels() -> String {
    match runtime::frame_buffer::last_frame_pixels() {
        Some((w, h, px)) => format!("{w}x{h}:{len}", len = px.len()),
        None => String::new(),
    }
}

/// Run Kabootar UI code, paint, and return compositor HTML for host mount.
#[wasm_bindgen]
pub fn kb_run_ui(code: &str) -> String {
    let mut env = create_global_env();
    let script = format!(
        "{code}\nkb_host_sync(); host_paint();"
    );
    match eval_source(&script, &mut env) {
        Ok(val) => match val {
            Value::String(s) => s,
            other => format_value(&other),
        },
        Err(e) => e,
    }
}

/// DevTools snapshot JSON for `kabootar-shell.html` (Elements + Console + breakpoints).
#[wasm_bindgen]
pub fn kb_devtools_json() -> String {
    let env = create_global_env();
    runtime::browser_platform::shell_snapshot_from_env(&env)
}
