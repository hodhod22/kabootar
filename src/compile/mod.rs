//! Parse cache and on-disk `.kbc` bytecode artifacts.

use crate::bytecode::{
    call_value, deserialize, deserialize_kbcb, looks_like_kbcb, run_module, serialize,
    serialize_kbcb, BytecodeModule, FORMAT_HEADER,
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

mod dag;

pub use crate::bytecode::{can_compile, compile_source, try_compile, CompiledProgram};
pub use dag::{
    collect_self_host_inventory, compiler_image_path, is_compile_dag_path,
    missing_compiler_dag_seeds, rust_compile_write_seed, walk_compile_dag,
    write_compiler_dag_seeds, write_compiler_facade_seeds, SelfHostInventory,
};

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

/// Skip self-host for the heavy emit/parser/lexer/serialize/vm leaf shards. H6e:
/// every public core is reached through thin `pub let X = Ximpl` facades
/// (`vm.kab` → `vm_impl` → `vm_run` → `vm_run_body` → `vm_run_exec_core`, `serialize` →
/// `serialize_impl` → `serialize_body` → `serialize_acc` / `serialize_pure`). Only the
/// leaf shards below stay skip-listed (self-host AST density makes them CI-slow).
/// Facades above them self-host-compile in CI-fast time. Product `import` prefers
/// self-host via `load_program_for_file` → `compile_file_prefer_cached` (Rust only
/// while `KAB_VM_EXEC_ACTIVE`, or for these leaves / oversize).
///
/// P6b: skip-list empty — emit/parser/lexer impls are thin drivers that
/// self-host-compile under `P6_SELF_HOST_LEAF_CI_FAST_MS`. Heavy bodies live in
/// densified shards. Committed `self_host/seed/*.kbc` remain a kab-only cache.
pub const SELF_HOST_SKIP_LISTED_LEAVES: &[&str] = &[];

fn should_attempt_self_host(path: &str, source: &str) -> bool {
    let norm = path.replace('\\', "/");
    for c in SELF_HOST_SKIP_LISTED_LEAVES {
        if norm.ends_with(c) || norm.contains(&format!("/{c}")) {
            return false;
        }
    }
    if source.len() > 64 * 1024 {
        return false;
    }
    true
}

/// P6b: skip-list empty; seeds remain as kab-only cache for historical leaves.
pub fn self_host_skip_policy() -> &'static str {
    "attempt-all"
}

/// P6 gate: max self-host compile time (ms) per leaf before emptying skip-list (P6b).
pub const P6_SELF_HOST_LEAF_CI_FAST_MS: u64 = 10_000;

/// P6b: flip to `true` only after `p6_leaf_self_host_compile_budget` shows all leaves < budget.
pub const P6B_EMPTY_SKIP_LIST_READY: bool = true;

/// True when `path` is a skip-listed self-host leaf (needs Rust compile or `.kbc` cache).
pub fn self_host_is_skip_listed(path: &str) -> bool {
    match fs::read_to_string(path) {
        Ok(source) => !should_attempt_self_host(path, &source),
        // Missing file: not a skip-list concern.
        Err(_) => false,
    }
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
    use crate::evaluator::create_module_env;
    use crate::value::Value;

    // Module resolution is cwd-relative; prefer the package root when available.
    let _cwd_guard = PackageRootGuard::enter();

    // Force host VM while running the self-host toolchain (avoid Kab meta-eval).
    let prev_exec = KAB_VM_EXEC_ACTIVE.swap(true, Ordering::AcqRel);
    let compiled = (|| {
        let t_import = std::time::Instant::now();
        let mut env = SELF_HOST_TOOLCHAIN.with(|slot| slot.borrow_mut().take());
        let cache_hit = env.is_some();
        if env.is_none() {
            let mut fresh = create_module_env();
            crate::modules::import_module("self_host/compile", &mut fresh).map_err(|e| {
                crate::runtime::stdlib::error::format_runtime_error(&e)
            })?;
            env = Some(fresh);
        }
        let import_ms = t_import.elapsed().as_secs_f64() * 1000.0;
        let mut guard = SelfHostToolchainGuard { env };
        let env = guard.env.as_mut().expect("self-host toolchain env");
        let compile_fn = env
            .get("compile")
            .ok_or_else(|| "self_host/compile: missing export `compile`".to_string())?;
        let t_pipe = std::time::Instant::now();
        let result = call_value(
            compile_fn,
            vec![Value::String(source.to_string())],
            &[],
            &[],
            &[],
            &[],
            env,
        )
        .map_err(|e| crate::runtime::stdlib::error::format_runtime_error(&e))?;
        let pipe_ms = t_pipe.elapsed().as_secs_f64() * 1000.0;
        let Value::String(kbc) = result else {
            return Err(format!(
                "self_host compile must return .kbc text, got {}",
                crate::value::format_value(&result)
            ));
        };
        if !kbc.starts_with(FORMAT_HEADER) {
            return Err("self_host compile did not emit kabootar-bytecode header".into());
        }
        let t_deser = std::time::Instant::now();
        let module = deserialize(&kbc)?;
        let deser_ms = t_deser.elapsed().as_secs_f64() * 1000.0;
        if std::env::var("KABOOTAR_P10_PROFILE").as_deref() == Ok("1") {
            let (shard_evals, unique_modules) = crate::modules::import_shard_stats();
            eprintln!(
                "PROFILE self_host_host import_ms={import_ms:.1} cache_hit={cache_hit} pipe_ms={pipe_ms:.1} deserialize_ms={deser_ms:.1} shard_evals={shard_evals} unique_modules={unique_modules} kbc_bytes={}",
                kbc.len()
            );
        }
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

thread_local! {
    static SELF_HOST_TOOLCHAIN: std::cell::RefCell<Option<crate::value::Environment>> =
        std::cell::RefCell::new(None);
}

struct SelfHostToolchainGuard {
    env: Option<crate::value::Environment>,
}

impl Drop for SelfHostToolchainGuard {
    fn drop(&mut self) {
        if let Some(env) = self.env.take() {
            SELF_HOST_TOOLCHAIN.with(|s| *s.borrow_mut() = Some(env));
        }
    }
}

/// Drop the cached `self_host/compile` env (tests / after editing toolchain sources).
pub fn reset_self_host_toolchain_cache() {
    SELF_HOST_TOOLCHAIN.with(|s| *s.borrow_mut() = None);
}

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
    // H6e/SH1: committed seed `.kbc` (historical leaves + compiler DAG image).
    if let Some(bc) = read_seed_bytecode(path)? {
        let program = CompiledProgram {
            stmts: Vec::new(),
            bytecode: Some(bc.clone()),
            stmt_count: rough_stmt_count(&fs::read_to_string(path).unwrap_or_default()),
            memory_mode: bc.memory_mode,
        };
        if let (Some(t), Ok(mut map)) = (mtime, cache().lock()) {
            map.insert(
                path.to_string(),
                CachedProgram {
                    mtime: t,
                    program: program.clone(),
                },
            );
        }
        return Ok((program, "seed"));
    }
    // SH1: compiler DAG without image must not self-host-compile the compiler.
    if dag::is_compile_dag_path(path) && prefer != CompilePrefer::SelfHostOnly {
        let program = compile_file(path)?;
        if program.has_bytecode() {
            let _ = write_compile_marker(path, &program);
        }
        return Ok((program, "rust"));
    }
    if dag::is_compile_dag_path(path) && prefer == CompilePrefer::SelfHostOnly {
        return Err(format!(
            "SH1: no compiler-image seed for `{path}` (run KABOOTAR_SH1_WARM=1 / write_compiler_dag_seeds)"
        ));
    }
    // H6e delete-gate: kab-only must not live-Rust-compile skip-listed leaves.
    // Soften while the self-host toolchain / Kab VM is already executing
    // (`KAB_VM_EXEC_ACTIVE`), and when the caller forced `--rust` / KABOOTAR_COMPILE=rust.
    if kab_vm_only_mode()
        && prefer != CompilePrefer::Rust
        && !KAB_VM_EXEC_ACTIVE.load(Ordering::Acquire)
    {
        let source = fs::read_to_string(path).unwrap_or_default();
        if !should_attempt_self_host(path, &source) {
            return Err(format!(
                "Kab VM only: skip-listed core `{path}` needs Rust compile (no .kbc cache/seed)"
            ));
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
    // H6e: product `import` prefers self-host the same way as `run_file`
    // (`compile_file_prefer_cached`). Force Rust while the self-host toolchain or
    // Kab VM is already active so nested loads cannot recurse into another
    // full self-host compile of the compiler.
    let _ = source; // caller already read; prefer path re-reads + fingerprint/cache.
    let prefer = if KAB_VM_EXEC_ACTIVE.load(Ordering::Acquire) {
        CompilePrefer::Rust
    } else {
        CompilePrefer::from_args_and_env(&[])
    };
    let (program, _) = compile_file_prefer_cached(path, prefer)?;
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
        let _ = fs::remove_file(&marker);
        let _ = fs::remove_file(cache_path_kbcb(&base, path));
    }
}

/// Drop the in-process compile cache without deleting `.kbc` / `.kbcb` on disk.
pub fn invalidate_memory_cache_for_tests(path: &str) {
    if let Ok(mut map) = cache().lock() {
        map.remove(path);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SelfHostWarmStats {
    pub compiled: usize,
    pub cached: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Rust-compile `self_host/*.kab` (skip `_` / `test_` probes) so the next process
/// loads `.kbc` / `.kbcb` instead of re-emitting every shard.
pub fn warm_self_host_disk_cache(root: &Path) -> Result<SelfHostWarmStats, String> {
    let _guard = PackageRootGuard::enter();
    let dir = root.join("self_host");
    let mut stats = SelfHostWarmStats::default();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("kab"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if name.starts_with('_') || name.starts_with("test_") {
            stats.skipped += 1;
            continue;
        }
        let path_s = path.to_string_lossy().replace('\\', "/");
        match compile_file_prefer_cached(&path_s, CompilePrefer::Rust) {
            Ok((_, "disk-cache" | "cache" | "seed")) => stats.cached += 1,
            Ok(_) => stats.compiled += 1,
            Err(_) => stats.failed += 1,
        }
    }
    Ok(stats)
}

pub fn cache_dir() -> PathBuf {
    PathBuf::from(".kabootar").join("cache")
}

/// Committed seed `.kbc` (`self_host/seed/<file>.kbc` or `seed/dag/`).
pub fn seed_kbc_path(path: &str) -> Option<PathBuf> {
    let cands = dag::seed_kbc_candidates(path);
    cands
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .or_else(|| cands.into_iter().next())
}

/// Load committed seed bytecode when fingerprint matches current source.
pub fn read_seed_bytecode(path: &str) -> Result<Option<BytecodeModule>, String> {
    dag::read_matching_seed(path)
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

fn cache_path_kbcb(base: &Path, path: &str) -> PathBuf {
    cache_path_for(base, path).with_extension("kbcb")
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
        let kbcb = serialize_kbcb(program.bytecode.as_ref().unwrap());
        let kbcb_path = cache_path_kbcb(base, path);
        let _ = fs::write(&kbcb_path, kbcb);
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
    let kbcb_path = cache_path_kbcb(&base, path);
    if let Ok(bytes) = fs::read(&kbcb_path) {
        if looks_like_kbcb(&bytes) {
            let cache_mtime = fs::metadata(&kbcb_path)
                .ok()
                .and_then(|m| m.modified().ok());
            if let Some(cache_mtime) = cache_mtime {
                if source_mtime > cache_mtime {
                    return Ok(None);
                }
            }
            if let Ok(source) = fs::read_to_string(path) {
                let expected = source_fingerprint(path, &source);
                if let Ok(text) = fs::read_to_string(&marker) {
                    if let Some(line) = text.lines().find(|l| l.starts_with("fingerprint=")) {
                        let got = line.trim_start_matches("fingerprint=");
                        if got != expected {
                            return Ok(None);
                        }
                    }
                    let norm = |p: &str| p.replace('\\', "/");
                    if let Some(line) = text.lines().find(|l| l.starts_with("source=")) {
                        let cached_src = line.trim_start_matches("source=");
                        if norm(cached_src) != norm(path) {
                            return Ok(None);
                        }
                    }
                }
            }
            return Ok(Some(deserialize_kbcb(&bytes)?));
        }
    }
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

pub(crate) fn extract_kab_imports(source: &str) -> Vec<String> {
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