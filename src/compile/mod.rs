//! Parse cache and on-disk `.kbc` bytecode artifacts.

use crate::bytecode::{
    deserialize, run_module, serialize, BytecodeModule, FORMAT_HEADER,
};
use crate::evaluator::{drain_all_microtasks, eval_stmt};
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

pub use crate::bytecode::{can_compile, compile_source, try_compile, CompiledProgram};

static PARSE_CACHE: OnceLock<Mutex<HashMap<String, CachedProgram>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct CachedProgram {
    mtime: SystemTime,
    program: CompiledProgram,
}

fn cache() -> &'static Mutex<HashMap<String, CachedProgram>> {
    PARSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn compile_file(path: &str) -> Result<CompiledProgram, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {path}: {e}"))?;
    let mtime = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok());
    let program = compile_source(&source)?;
    if let Ok(mut map) = cache().lock() {
        if let Some(t) = mtime {
            map.insert(
                path.to_string(),
                CachedProgram {
                    mtime: t,
                    program: program.clone(),
                },
            );
        }
    }
    Ok(program)
}

pub fn compile_file_cached(path: &str) -> Result<CompiledProgram, String> {
    let mtime = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok());
    if let (Some(t), Ok(map)) = (mtime, cache().lock()) {
        if let Some(cached) = map.get(path) {
            if cached.mtime == t {
                return Ok(cached.program.clone());
            }
        }
    }
    compile_file(path)
}

pub fn eval_program(program: &CompiledProgram, env: &mut Environment) -> Result<Value, String> {
    if let Some(bytecode) = &program.bytecode {
        if bytecode.uses_bytecode() {
            let result = run_module(bytecode, env)?;
            drain_all_microtasks(env)?;
            return Ok(result);
        }
    }
    let mut last = Value::Null;
    for stmt in &program.stmts {
        last = eval_stmt(stmt, env)?;
    }
    drain_all_microtasks(env)?;
    Ok(last)
}

pub fn load_program_for_file(path: &str, source: &str) -> Result<CompiledProgram, String> {
    let mtime = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok());
    if let Some(mtime) = mtime {
        if let Some(bc) = read_bytecode_cache(path, mtime)? {
            return Ok(CompiledProgram {
                stmts: Vec::new(),
                bytecode: Some(bc),
                stmt_count: 0,
            });
        }
    }
    compile_source(source)
}

pub fn eval_file_cached(path: &str, env: &mut Environment) -> Result<Value, String> {
    let program = compile_file_cached(path)?;
    eval_program(&program, env)
}

pub fn invalidate_file_cache(path: &str) {
    if let Ok(mut map) = cache().lock() {
        map.remove(path);
    }
    if let Ok(base) = std::env::current_dir() {
        let marker = cache_path_for(&base, path);
        let _ = fs::remove_file(marker);
    }
}

pub fn cache_dir() -> PathBuf {
    PathBuf::from(".kabootar").join("cache")
}

pub fn cache_path_for(base: &Path, path: &str) -> PathBuf {
    let file_name = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.replace(['/', '\\'], "_"));
    base.join(".kabootar").join("cache").join(format!("{file_name}.kbc"))
}

pub fn write_compile_marker(path: &str, program: &CompiledProgram) -> Result<(), String> {
    let base = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {e}"))?;
    write_compile_marker_at(&base, path, program)
}

pub fn write_compile_marker_at(
    base: &Path,
    path: &str,
    program: &CompiledProgram,
) -> Result<(), String> {
    let marker = cache_path_for(base, path);
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cache dir: {e}"))?;
    }
    let content = if program.has_bytecode() {
        let mut text = serialize(program.bytecode.as_ref().unwrap());
        text.push_str(&format!("\nsource={path}\nstatements={}\n", program.stmt_count));
        text
    } else {
        format!(
            "kabootar-compile-cache/1\nsource={path}\nstatements={}\n",
            program.stmt_count
        )
    };
    fs::write(marker, content).map_err(|e| format!("Failed to write cache marker: {e}"))
}

pub fn read_bytecode_cache(path: &str, source_mtime: SystemTime) -> Result<Option<BytecodeModule>, String> {
    let base = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {e}"))?;
    let marker = cache_path_for(&base, path);
    let text = match fs::read_to_string(&marker) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    if !text.starts_with(FORMAT_HEADER) {
        return Ok(None);
    }
    let cache_mtime = fs::metadata(&marker)
        .ok()
        .and_then(|m| m.modified().ok());
    if let Some(cache_mtime) = cache_mtime {
        if source_mtime > cache_mtime {
            return Ok(None);
        }
    }
    Ok(Some(deserialize(&text)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_source_parses_statements() {
        let p = compile_source("let x = 1\nx + 2").unwrap();
        assert_eq!(p.stmt_count, 2);
        assert!(p.has_bytecode());
    }
}