//! Kv8 JIT — compile hot loops to Kabootar bytecode.

use super::ast::Stmt;
use super::bytecode_bridge::{try_compile_loop_body, Kv8BytecodeFn};
use super::context::Kv8Context;
use std::collections::HashMap;

pub const JIT_THRESHOLD: u64 = 8;

#[derive(Default)]
pub struct Kv8Jit {
    loop_hits: HashMap<String, u64>,
    compiled_loops: HashMap<String, Kv8BytecodeFn>,
}

impl Kv8Jit {
    pub fn record_loop(&mut self, key: &str) -> bool {
        let c = self.loop_hits.entry(key.to_string()).or_insert(0);
        *c += 1;
        *c >= JIT_THRESHOLD
    }

    pub fn get_loop(&self, key: &str) -> Option<&Kv8BytecodeFn> {
        self.compiled_loops.get(key)
    }

    pub fn compile_loop(&mut self, key: &str, body: &[Stmt]) -> Result<bool, String> {
        if self.compiled_loops.contains_key(key) {
            return Ok(true);
        }
        if let Some(f) = try_compile_loop_body(body)? {
            self.compiled_loops.insert(key.to_string(), f);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn stats(&self) -> (usize, u64) {
        let total: u64 = self.loop_hits.values().sum();
        (self.compiled_loops.len(), total)
    }
}

impl std::fmt::Debug for Kv8Jit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kv8Jit")
            .field("compiled_loops", &self.compiled_loops.len())
            .field("loop_hits", &self.loop_hits.len())
            .finish()
    }
}

pub fn loop_key(var: &str, cond: &str) -> String {
    format!("for:{var}:{cond}")
}

pub fn attach_jit(ctx: &Kv8Context) -> Result<(), String> {
    ctx.with_mut(|inner| {
        if inner.jit.is_none() {
            inner.jit = Some(Kv8Jit::default());
        }
        Ok(())
    })
}
