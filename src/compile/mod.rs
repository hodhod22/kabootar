//! Parse cache and on-disk `.kbc` bytecode artifacts.

use crate::bytecode::{
    call_value, deserialize, run_module, serialize, BytecodeModule, FORMAT_HEADER,
};
use crate::evaluator::{drain_all_microtasks, eval_stmt};
use crate::modules;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

pub use crate::bytecode::{can_compile, compile_source, try_compile, CompiledProgram};

/// Which backend `kabootar compile` prefers (S2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilePrefer {
    /// Self-host first, Rust on failure (default for app `.kab`).
    SelfHostThenRust,
    /// Force Rust host compiler.
    Rust,
    /// Force self-host; error if it cannot produce bytecode.
    SelfHostOnly,
}

impl CompilePrefer {
    pub fn from_args_and_env(args: &[String]) -> Self {
        if args.iter().any(|a| a == "--rust" || a == "--host") {
            return Self::Rust;
        }
        if args.iter().any(|a| a == "--self-host") {
            return Self::SelfHostOnly;
        }
        match std::env::var("KABOOTAR_COMPILE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "rust" | "host" => Self::Rust,
            "self-host" | "self_host" | "selfhost" => Self::SelfHostOnly,
            _ => Self::SelfHostThenRust,
        }
    }
}

/// Skip self-host for heavy emit/parser/lexer/serialize/vm cores. H6e: these all
/// now have thin `pub let X = Ximpl` facades (see e.g. self_host/vm.kab,
/// self_host/vm_impl.kab) whose own source is tiny, so only the `_impl`/`_run`
/// bodies below need to stay skip-listed — the facades (`emit.kab`, `parser.kab`,
/// `lexer.kab`, `serialize.kab`, `vm_impl.kab`, `compile.kab`) self-host-compile
/// in CI-fast time and are intentionally NOT in this list. `import` of the heavy
/// cores still uses Rust via `load_program_for_file`.
fn should_attempt_self_host(path: &str, source: &str) -> bool {
    let norm = path.replace('\\', "/");
    let core = [
        "self_host/emit_impl.kab",
        "self_host/parser_impl.kab",
        "self_host/lexer_impl.kab",
        "self_host/serialize_impl.kab",
        "self_host/vm_run.kab",
    ];
    for c in core {
        if norm.ends_with(c) || norm.contains(&format!("/{c}")) {
            return false;
        }
    }
    // Legacy basename match when cwd is self_host/
    let base = Path::new(&norm)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if matches!(
        base,
        "emit_impl.kab" | "parser_impl.kab" | "lexer_impl.kab" | "serialize_impl.kab" | "vm_run.kab"
    ) && norm.contains("self_host")
    {
        return false;
    }
    if source.len() > 64 * 1024 {
        return false;
    }
    true
}

fn kab_vm_only_mode() -> bool {
    // Process-wide strict only via env. Default delete-gate for `.kbc` lives in
    // `kab/vm` `evalKbc` when `kabVmRunOk` (Rust may still host-fallback for large/kv8).
    matches!(
        std::env::var("KABOOTAR_VM").as_deref(),
        Ok("kab-only") | Ok("only") | Ok("kab_only")
    )
}

fn rough_stmt_count(source: &str) -> usize {
    source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count()
        .max(1)
}

/// Compile source text via `import "self_host/compile"` → `.kbc` text → deserialize.
pub fn compile_source_self_host(source: &str) -> Result<CompiledProgram, String> {
    use crate::bytecode::call_value;
    use crate::evaluator::create_global_env;
    use crate::value::Value;

    // Module resolution is cwd-relative; prefer the package root when available.
    let _cwd_guard = PackageRootGuard::enter();

    // Force host VM while running the self-host toolchain (avoid Kab meta-eval).
    let prev_exec = KAB_VM_EXEC_ACTIVE.swap(true, Ordering::AcqRel);
    let compiled = (|| {
        let mut env = create_global_env();
        crate::modules::import_module("self_host/compile", &mut env).map_err(|e| {
            crate::runtime::stdlib::error::format_runtime_error(&e)
        })?;
        let compile_fn = env
            .get("compile")
            .ok_or_else(|| "self_host/compile: missing export `compile`".to_string())?;
        let result = call_value(
            compile_fn,
            vec![Value::String(source.to_string())],
            &[],
            &[],
            &[],
            &[],
            &mut env,
        )
        .map_err(|e| crate::runtime::stdlib::error::format_runtime_error(&e))?;
        let Value::String(kbc) = result else {
            return Err(format!(
                "self_host compile must return .kbc text, got {}",
                crate::value::format_value(&result)
            ));
        };
        if !kbc.starts_with(FORMAT_HEADER) {
            return Err("self_host compile did not emit kabootar-bytecode header".into());
        }
        let module = deserialize(&kbc)?;
        Ok(CompiledProgram {
            stmts: Vec::new(),
            bytecode: Some(module.clone()),
            stmt_count: rough_stmt_count(source),
            memory_mode: module.memory_mode,
        })
    })();
    KAB_VM_EXEC_ACTIVE.store(prev_exec, Ordering::Release);
    compiled
}

/// Temporarily set cwd to the package root (directory containing `self_host/`).
struct PackageRootGuard {
    prev: Option<PathBuf>,
}

impl PackageRootGuard {
    fn enter() -> Self {
        let prev = std::env::current_dir().ok();
        let mut cand = prev.clone();
        // Walk up from cwd looking for self_host/compile.kab
        for _ in 0..6 {
            let Some(dir) = cand else { break };
            if dir.join("self_host").join("compile.kab").is_file() {
                let _ = std::env::set_current_dir(&dir);
                return Self { prev };
            }
            cand = dir.parent().map(|p| p.to_path_buf());
        }
        // Fallback: CARGO_MANIFEST_DIR when built as part of this package.
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let dir = PathBuf::from(manifest);
            if dir.join("self_host").join("compile.kab").is_file() {
                let _ = std::env::set_current_dir(&dir);
            }
        }
        Self { prev }
    }
}

impl Drop for PackageRootGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            let _ = std::env::set_current_dir(prev);
        }
    }
}

/// Whether `compile_file_self_host` will attempt this path (not skip-listed / oversize).
pub fn self_host_is_attemptable(path: &str) -> bool {
    match fs::read_to_string(path) {
        Ok(source) => should_attempt_self_host(path, &source),
        Err(_) => false,
    }
}

pub fn compile_file_self_host(path: &str) -> Result<CompiledProgram, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
    if !should_attempt_self_host(path, &source) {
        return Err("self-host skipped for this path/size".into());
    }
    let program = compile_source_self_host(&source)?;
    if let Ok(mut map) = cache().lock() {
        if let Some(t) = fs::metadata(path).ok().and_then(|m| m.modified().ok()) {
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

/// Compile a file with the preferred backend (S2). Returns `(program, backend_label)`.
pub fn compile_file_prefer(
    path: &str,
    prefer: CompilePrefer,
) -> Result<(CompiledProgram, &'static str), String> {
    match prefer {
        CompilePrefer::Rust => Ok((compile_file(path)?, "rust")),
        CompilePrefer::SelfHostOnly => {
            let program = compile_file_self_host(path)?;
            if !program.has_bytecode() {
                return Err("self-host compile produced no bytecode".into());
            }
            Ok((program, "self-host"))
        }
        CompilePrefer::SelfHostThenRust => {
            match compile_file_self_host(path) {
                Ok(program) if program.has_bytecode() => Ok((program, "self-host")),
                Ok(_) | Err(_) => Ok((compile_file(path)?, "rust")),
            }
        }
    }
}

static PARSE_CACHE: OnceLock<Mutex<HashMap<String, CachedProgram>>> = OnceLock::new();
static KAB_VM_RUN_ENABLED: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
static KAB_VM_POLICY_PROBE: AtomicBool = AtomicBool::new(false);
static KAB_VM_EXEC_ACTIVE: AtomicBool = AtomicBool::new(false);

fn kab_vm_run_enabled(env: &mut Environment) -> Result<bool, String> {
    // Prefer Kab VM for small .kbc when policy is healthy; evalKbc falls back to host VM
    // unless KABOOTAR_VM=kab-only (strict delete-gate: no Rust run_module).
    // KABOOTAR_VM=rust|host forces host; kab|kabootar|self|kab-only forces prefer Kab.
    if KAB_VM_POLICY_PROBE.load(Ordering::Acquire) || KAB_VM_EXEC_ACTIVE.load(Ordering::Acquire) {
        return Ok(false);
    }
    match std::env::var("KABOOTAR_VM").as_deref() {
        Ok("rust") | Ok("host") => return Ok(false),
        Ok("kab") | Ok("kabootar") | Ok("self") | Ok("kab-only") | Ok("only") | Ok("kab_only") => {
            return Ok(true)
        }
        _ => {}
    }
    let slot = KAB_VM_RUN_ENABLED.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = slot.lock() {
        if let Some(cached) = *guard {
            return Ok(cached);
        }
    }
    KAB_VM_POLICY_PROBE.store(true, Ordering::Release);
    let probed = (|| {
        modules::import_module("kab/vm", env)?;
        let f = env
            .get("kabVmRunEnabled")
            .ok_or("kab/vm: missing kabVmRunEnabled")?;
        let v = call_value(f, vec![], &[], &[], &[], &[], env)?;
        drain_all_microtasks(env)?;
        Ok::<bool, String>(v.is_truthy())
    })();
    KAB_VM_POLICY_PROBE.store(false, Ordering::Release);
    let enabled = probed.unwrap_or(false);
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(enabled);
    }
    Ok(enabled)
}

fn eval_kbc_via_kab_vm(kbc: &str, env: &mut Environment) -> Result<Value, String> {
    KAB_VM_EXEC_ACTIVE.store(true, Ordering::Release);
    let result = (|| {
        modules::import_module("kab/vm", env)?;
        // When Kab VM path is selected (healthy probe or KABOOTAR_VM=kab*), run
        // kab-only — no soft host `bytecode_run_kbc` fallback.
        let f = env
            .get("evalKbcKabOnly")
            .ok_or("kab/vm: missing evalKbcKabOnly")?;
        let v = call_value(
            f,
            vec![Value::String(kbc.to_string())],
            &[],
            &[],
            &[],
            &[],
            env,
        )?;
        drain_all_microtasks(env)?;
        Ok(v)
    })();
    KAB_VM_EXEC_ACTIVE.store(false, Ordering::Release);
    result
}

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

/// H6e deepen: product `run_file` prefers self-host compile (same as `kabootar compile`).
pub fn compile_file_prefer_cached(
    path: &str,
    prefer: CompilePrefer,
) -> Result<(CompiledProgram, &'static str), String> {
    let mtime = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok());
    if let (Some(t), Ok(map)) = (mtime, cache().lock()) {
        if let Some(cached) = map.get(path) {
            if cached.mtime == t {
                return Ok((cached.program.clone(), "cache"));
            }
        }
    }
    if let Some(t) = mtime {
        if let Some(bc) = read_bytecode_cache(path, t)? {
            let program = CompiledProgram {
                stmts: Vec::new(),
                bytecode: Some(bc.clone()),
                stmt_count: rough_stmt_count(
                    &fs::read_to_string(path).unwrap_or_default(),
                ),
                memory_mode: bc.memory_mode,
            };
            if let Ok(mut map) = cache().lock() {
                map.insert(
                    path.to_string(),
                    CachedProgram {
                        mtime: t,
                        program: program.clone(),
                    },
                );
            }
            return Ok((program, "disk-cache"));
        }
    }
    let (program, backend) = compile_file_prefer(path, prefer)?;
    if program.has_bytecode() {
        let _ = write_compile_marker(path, &program);
    }
    Ok((program, backend))
}

pub fn eval_program(program: &CompiledProgram, env: &mut Environment) -> Result<Value, String> {
    crate::runtime::ownership::set_memory_mode(env, program.memory_mode);
    if let Some(bytecode) = &program.bytecode {
        if bytecode.uses_bytecode() {
            if kab_vm_run_enabled(env)? {
                let kbc = serialize(bytecode);
                // Kab VM subset: small modules only; large ones stay on host VM
                // (unless kab-only, which rejects oversized modules).
                if kbc.len() <= 262144 {
                    match eval_kbc_via_kab_vm(&kbc, env) {
                        Ok(v) => return Ok(v),
                        Err(e) if kab_vm_only_mode() => {
                            let detail = crate::runtime::stdlib::error::format_runtime_error(&e);
                            return Err(format!("Kab VM only (no host fallback): {detail}"))
                        }
                        Err(_) => {}
                    }
                } else if kab_vm_only_mode() {
                    return Err("Kab VM only: .kbc exceeds 256KB size gate".into());
                }
            }
            if kab_vm_only_mode() {
                // Nested loads while evaluating via Kab (imports of kab/vm deps) may use host.
                if !KAB_VM_EXEC_ACTIVE.load(Ordering::Acquire) {
                    return Err("Kab VM only: host bytecode VM disabled".into());
                }
            }
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
                bytecode: Some(bc.clone()),
                stmt_count: 0,
                memory_mode: bc.memory_mode,
            });
        }
    }
    let program = compile_source(source)?;
    if program.has_bytecode() {
        let _ = write_compile_marker(path, &program);
    }
    Ok(program)
}

pub fn eval_file_cached(path: &str, env: &mut Environment) -> Result<Value, String> {
    // Product run path: prefer self-host .kbc (KABOOTAR_COMPILE=rust forces host).
    let prefer = CompilePrefer::from_args_and_env(&[]);
    let (program, _) = compile_file_prefer_cached(path, prefer)?;
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
    // Prefer cwd-relative keys so `self_host/lexer.kab` and `lib/kv8/lexer.kab`
    // do not share a basename-only `lexer.kab.kbc` (S2 collision).
    let abs = Path::new(path);
    let rel = std::env::current_dir()
        .ok()
        .and_then(|cwd| abs.strip_prefix(&cwd).ok().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| abs.to_path_buf());
    let key = rel
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .replace("..", "_")
        .replace('/', "__");
    let file_name = if key.is_empty() {
        "module.kab".to_string()
    } else if key.ends_with(".kab") || key.ends_with(".kabootar") {
        key
    } else {
        format!("{key}.kab")
    };
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
        let source = fs::read_to_string(path).unwrap_or_default();
        let fp = source_fingerprint(path, &source);
        text.push_str(&format!(
            "\nsource={path}\nstatements={}\nfingerprint={fp}\n",
            program.stmt_count
        ));
        text
    } else {
        let source = fs::read_to_string(path).unwrap_or_default();
        let fp = source_fingerprint(path, &source);
        format!(
            "kabootar-compile-cache/1\nsource={path}\nstatements={}\nfingerprint={fp}\n",
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
    // Reject basename-collision leftovers (e.g. kv8 lexer cached as lexer.kab.kbc).
    let norm = |p: &str| p.replace('\\', "/");
    if let Some(line) = text.lines().find(|l| l.starts_with("source=")) {
        let cached_src = line.trim_start_matches("source=");
        if norm(cached_src) != norm(path) {
            return Ok(None);
        }
    }
    // G8: content + import fingerprint — invalidate when source/imports change without mtime bump.
    if let Ok(source) = fs::read_to_string(path) {
        let expected = source_fingerprint(path, &source);
        if let Some(line) = text.lines().find(|l| l.starts_with("fingerprint=")) {
            let got = line.trim_start_matches("fingerprint=");
            if got != expected {
                return Ok(None);
            }
        } else {
            // Old cache entries without fingerprint are unsafe across path collisions.
            return Ok(None);
        }
    }
    Ok(Some(deserialize(&text)?))
}

/// Hash of file bytes plus mtimes of `import "…"` deps (incremental self-host cache key).
pub fn source_fingerprint(path: &str, source: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let base = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
    for dep in extract_kab_imports(source) {
        let candidate = base.join(&dep);
        let resolved = if candidate.exists() {
            candidate
        } else {
            PathBuf::from(&dep)
        };
        if let Ok(meta) = fs::metadata(&resolved) {
            if let Ok(mtime) = meta.modified() {
                if let Ok(dur) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                    dur.as_nanos().hash(&mut hasher);
                }
            }
        }
        dep.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}

fn extract_kab_imports(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("import ") {
            let rest = rest.trim();
            if let Some(q) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                if q.ends_with(".kab") || !q.contains('.') {
                    out.push(q.to_string());
                }
            } else if let Some(q) = rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                out.push(q.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_fingerprint_changes_with_content() {
        let a = source_fingerprint("x.kab", "let a = 1\n");
        let b = source_fingerprint("x.kab", "let a = 2\n");
        assert_ne!(a, b);
        assert_eq!(a, source_fingerprint("x.kab", "let a = 1\n"));
    }

    #[test]
    fn compile_source_parses_statements() {
        let p = compile_source("let x = 1\nx + 2").unwrap();
        assert_eq!(p.stmt_count, 2);
        assert!(p.has_bytecode());
    }
}