//! Bytecode compilation and execution (v2.18).

mod classes;
mod compiler;
mod optimize;
mod types;
mod vm;

pub use compiler::{can_compile, take_hard_compile_error, try_compile};
pub use optimize::{optimize_module, OptStats};
pub use types::{
    deserialize, serialize, BytecodeFnDef, BytecodeModule, Constant, Opcode, FORMAT_HEADER,
};
pub use vm::{
    bind_bytecode_params, call_value, find_try_region_for_ip, member_ic_reset_for_tests,
    member_ic_stats, prepare_exported_bytecode_fn, run_bytecode_fn, run_bytecode_fn_with_locals,
    run_expr_snippet, run_generator_step, run_module, ChunkCursor, GeneratorResume,
};

use crate::ast::Stmt;
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::project::version::strip_version_directive;

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

pub fn compile_source(source: &str) -> Result<CompiledProgram, String> {
    let (source, meta) = crate::lang_preprocess::preprocess_with_meta(source);
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
