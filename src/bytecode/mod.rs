//! Bytecode compilation and execution (v2.18).

mod classes;
mod compiler;
mod jit;
mod kbcb;
mod optimize;
mod typed;
mod types;
mod vm;

pub use compiler::{can_compile, take_hard_compile_error, try_compile};
pub use optimize::{optimize_module, OptStats};
pub use kbcb::{
    deserialize_kbcb, deserialize_kbcb_v2, looks_like_kbcb, serialize_kbcb, serialize_kbcb_v1,
    serialize_kbcb_v2, KBCB_MAGIC, KBCB_VERSION,
};
pub use jit::{
    jit_add_loop, jit_call_threshold, jit_reset_for_tests, jit_set_call_threshold_for_tests,
    jit_stats, JIT_CALL_THRESHOLD_DEFAULT,
};
pub use typed::{fn_is_typed_i64, typed_i64_reset_for_tests, typed_i64_stats};
pub use types::{
    deserialize, serialize, BytecodeFnDef, BytecodeModule, Constant, Opcode, FORMAT_HEADER,
};
pub use vm::{
    bind_bytecode_params, call_ic_mega_hits, call_ic_reset_for_tests, call_ic_stats, call_value,
    find_try_region_for_ip, global_ic_reset_for_tests, global_ic_stats,
    member_ic_reset_for_tests, member_ic_stats, prepare_exported_bytecode_fn,
    run_bytecode_fn, run_bytecode_fn_with_locals, run_expr_snippet, run_generator_step,
    run_module, ChunkCursor, GeneratorResume,
};

use crate::ast::Stmt;
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::project::version::strip_version_directive;
use crate::value::Value;
use std::cell::Cell;

thread_local! {
    static COMPTIME_DEPTH: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub stmts: Vec<Stmt>,
    pub bytecode: Option<BytecodeModule>,
    pub stmt_count: usize,
    pub memory_mode: crate::lang_preprocess::MemoryMode,
}

impl CompiledProgram {
    pub fn has_bytecode(&self) -> bool {
        self.bytecode
            .as_ref()
            .map(|b| b.uses_bytecode())
            .unwrap_or(false)
    }
}

/// Comptime 3.0 — evaluate `comptime { … }` to a source literal before parse.
/// Uses AST eval (not nested `compile_source`) so folding is reentrant-safe.
pub fn fold_comptime_source(source: &str) -> Result<String, String> {
    let depth = COMPTIME_DEPTH.get();
    if depth >= 8 {
        return Ok(crate::lang_preprocess::expand_comptime_keyword(source));
    }
    crate::lang_preprocess::rewrite_comptime_blocks(source, |body| {
        COMPTIME_DEPTH.set(depth + 1);
        let val = eval_comptime_body(body);
        COMPTIME_DEPTH.set(depth);
        value_to_kab_src(&val.map_err(|e| format!("comptime: {e}"))?)
    })
}

fn eval_comptime_body(body: &str) -> Result<Value, String> {
    let (source, meta) = crate::lang_preprocess::preprocess_with_meta(body);
    let source = crate::kstyle_preprocess::expand_kstyle_blocks(&source);
    let (_, body) = strip_version_directive(&source);
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    let source = format!("({body})");
    let tokens = tokenize(&source).map_err(|e| format!("lexer: {e}"))?;
    let mut parser = Parser::with_eof(tokens);
    let stmts = parser
        .parse_program()
        .map_err(|e| format!("parse: {e}"))?;
    crate::ownership_check::check_ownership(&stmts, meta.memory_mode())?;
    let mut env = crate::value::Environment::new();
    crate::runtime::lang_features_globals(&mut env);
    let mut last = Value::Null;
    for stmt in &stmts {
        last = crate::evaluator::eval_stmt(stmt, &mut env)?;
    }
    Ok(last)
}

fn value_to_kab_src(val: &Value) -> Result<String, String> {
    match val {
        Value::Undefined => Ok("undefined".into()),
        Value::Null => Ok("null".into()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Float(n) if n.is_nan() => Ok("NaN".into()),
        Value::Float(n) => Ok(n.to_string()),
        Value::Bool(true) => Ok("true".into()),
        Value::Bool(false) => Ok("false".into()),
        Value::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
            Ok(out)
        }
        Value::Array(items) => {
            let mut parts = Vec::new();
            for v in items.iter() {
                parts.push(value_to_kab_src(v)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut parts = Vec::new();
            for k in keys {
                let vs = value_to_kab_src(&map[&k])?;
                parts.push(format!("\"{}\": {vs}", k.replace('\\', "\\\\").replace('"', "\\\"")));
            }
            Ok(format!("{{{}}}", parts.join(", ")))
        }
        other => Err(format!(
            "comptime result is not a literal ({})",
            crate::value::format_value(other)
        )),
    }
}

pub fn compile_source(source: &str) -> Result<CompiledProgram, String> {
    let source = fold_comptime_source(source)?;
    let (source, meta) = crate::lang_preprocess::preprocess_with_meta(&source);
    let source = crate::kstyle_preprocess::expand_kstyle_blocks(&source);
    let (_, body) = strip_version_directive(&source);
    let tokens = tokenize(&body).map_err(|e| format!("Lexer error: {e}"))?;
    let mut parser = Parser::with_eof(tokens);
    let stmts = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {e}"))?;
    let stmt_count = stmts.len();
    let memory_mode = meta.memory_mode();
    crate::ownership_check::check_ownership(&stmts, memory_mode)?;
    let bytecode = try_compile(&stmts).map(|mut m| {
        m.memory_mode = memory_mode;
        m
    });
    if let Some(err) = take_hard_compile_error() {
        return Err(err);
    }
    Ok(CompiledProgram {
        stmts,
        bytecode,
        stmt_count,
        memory_mode,
    })
}
