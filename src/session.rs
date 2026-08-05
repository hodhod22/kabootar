//! Shared exploration session — REPL + notebook cells (Våg DX).

use crate::evaluator::{eval_source, create_global_env};
use crate::modules::import_module;
use crate::value::{format_value, Environment, Value};

/// Persistent Kabootar exploration session (IPython-style).
pub struct Session {
    pub env: Environment,
    pub last: Value,
    pub cell_count: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            env: create_global_env(),
            last: Value::Undefined,
            cell_count: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Evaluate one cell / REPL chunk. Binds `_` to the last result.
    pub fn eval_cell(&mut self, source: &str) -> Result<Value, String> {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return Ok(Value::Undefined);
        }
        self.cell_count = self.cell_count.saturating_add(1);
        let result = eval_source(trimmed, &mut self.env)?;
        self.last = result.clone();
        if self.env.assign("_", result.clone()).is_err() {
            self.env.set("_".into(), result.clone());
        }
        Ok(result)
    }

    pub fn format_last(&self) -> String {
        format_value(&self.last)
    }

    pub fn load_file(&mut self, path: &str) -> Result<Value, String> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| format!(":load {path}: {e}"))?;
        self.eval_cell(&src)
    }

    pub fn import_science(&mut self) -> Result<(), String> {
        import_module("science", &mut self.env)
    }

    /// Binding names suitable for `:vars` (skip internal `__kab_*`).
    pub fn var_names(&self) -> Vec<String> {
        let mut names = self.env.all_binding_names();
        names.retain(|n| !n.starts_with("__kab_") && n != "this");
        names.sort();
        names
    }
}

/// True if `source` looks syntactically incomplete (continue multiline).
pub fn needs_more_input(source: &str) -> bool {
    let s = source.trim_end();
    if s.ends_with('\\') {
        return true;
    }
    let mut depth_brace = 0i32;
    let mut depth_paren = 0i32;
    let mut depth_brack = 0i32;
    let mut in_str = false;
    let mut str_ch = '"';
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if c == '\\' {
                let _ = chars.next();
                continue;
            }
            if c == str_ch {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                in_str = true;
                str_ch = c;
            }
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_brack += 1,
            ']' => depth_brack -= 1,
            _ => {}
        }
    }
    depth_brace > 0 || depth_paren > 0 || depth_brack > 0
}

/// Strip trailing line-continuation backslashes used for explicit multiline.
pub fn strip_continuations(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let t = line.trim_end();
            if let Some(stripped) = t.strip_suffix('\\') {
                stripped.trim_end()
            } else {
                t
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
