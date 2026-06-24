//! Bytecode compilation and execution (v2.18).

mod classes;
mod compiler;
mod optimize;
mod types;
mod vm;

pub use compiler::{can_compile, try_compile};
pub use optimize::{optimize_module, OptStats};
pub use types::{
    deserialize, serialize, BytecodeFnDef, BytecodeModule, Constant, Opcode, FORMAT_HEADER,
};
pub use vm::{
    bind_bytecode_params, call_value, find_try_region_for_ip, run_bytecode_fn,
    run_bytecode_fn_with_locals, run_expr_snippet, run_generator_step, run_module, ChunkCursor,
    GeneratorResume,
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
    let source = crate::lang_preprocess::preprocess(source);
    let source = crate::kstyle_preprocess::expand_kstyle_blocks(&source);
    let (_, body) = strip_version_directive(&source);
    let tokens = tokenize(&body).map_err(|e| format!("Lexer error: {e}"))?;
    let mut parser = Parser::with_eof(tokens);
    let stmts = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {e}"))?;
    let stmt_count = stmts.len();
    let bytecode = try_compile(&stmts);
    Ok(CompiledProgram {
        stmts,
        bytecode,
        stmt_count,
    })
}
