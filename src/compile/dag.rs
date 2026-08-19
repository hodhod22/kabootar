//! SH0/SH1 — self_host import DAG inventory and committed compiler-image seeds.

use super::{compile_file, extract_kab_imports, source_fingerprint, CompiledProgram};
use crate::bytecode::{deserialize, serialize, BytecodeModule, FORMAT_HEADER};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Packed compiler-image (catalog of `.kbc` blobs). Not P10f single-module `KBCB`.
const IMAGE_MAGIC: &[u8; 4] = b"SH1I";
const IMAGE_VERSION: u8 = 1;

const COMPILE_ENTRY: &str = "self_host/compile";

const FACADE_KAB: &[&str] = &[
    "compile.kab",
    "parse.kab",
    "emit.kab",
    "serialize.kab",
    "ownership.kab",
];

#[derive(Debug, Clone)]
pub struct SelfHostInventory {
    pub kab_files: usize,
    pub vm_files: usize,
    pub probe_files: usize,
    pub compile_dag: Vec<String>,
}

pub fn self_host_dir() -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("self_host");
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from("self_host")
}

fn is_probe_name(name: &str) -> bool {
    name.starts_with('_')
        || name.starts_with("test_")
        || name.contains("_probe")
        || name.contains("_bisect")
        || name.contains("_acc_repro")
}

pub fn collect_self_host_inventory() -> Result<SelfHostInventory, String> {
    let dir = self_host_dir();
    let mut kab_files = 0usize;
    let mut vm_files = 0usize;
    let mut probe_files = 0usize;
    let entries = fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".kab") {
            continue;
        }
        if is_probe_name(&name) {
            probe_files += 1;
            continue;
        }
        kab_files += 1;
        if name.starts_with("vm_") || name == "vm.kab" {
            vm_files += 1;
        }
    }
    let compile_dag = walk_compile_dag()?;
    Ok(SelfHostInventory {
        kab_files,
        vm_files,
        probe_files,
        compile_dag,
    })
}

pub fn walk_compile_dag() -> Result<Vec<String>, String> {
    walk_import_dag(COMPILE_ENTRY)
}

pub fn walk_import_dag(entry: &str) -> Result<Vec<String>, String> {
    let dir = self_host_dir();
    let root = dir.parent().unwrap_or(&dir);
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back(entry.replace('\\', "/"));
    while let Some(spec) = q.pop_front() {
        let spec = spec.trim_end_matches(".kab").to_string();
        if !seen.insert(spec.clone()) {
            continue;
        }
        let rel = if spec.ends_with(".kab") {
            spec.clone()
        } else {
            format!("{spec}.kab")
        };
        let path = root.join(&rel);
        if !path.is_file() {
            continue;
        }
        order.push(rel.replace('\\', "/"));
        let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        for dep in extract_kab_imports(&source) {
            q.push_back(dep);
        }
    }
    Ok(order)
}

/// BFS depth from `self_host/compile` (SH10 import-depth budget).
pub fn dag_max_import_depth() -> Result<usize, String> {
    let dir = self_host_dir();
    let root = dir.parent().unwrap_or(&dir);
    let mut seen = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back((COMPILE_ENTRY.replace('\\', "/"), 0usize));
    let mut max_d = 0usize;
    while let Some((spec, depth)) = q.pop_front() {
        let spec = spec.trim_end_matches(".kab").to_string();
        if !seen.insert(spec.clone()) {
            continue;
        }
        max_d = max_d.max(depth);
        let rel = format!("{spec}.kab");
        let path = root.join(&rel);
        if !path.is_file() {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        for dep in extract_kab_imports(&source) {
            q.push_back((dep, depth + 1));
        }
    }
    Ok(max_d)
}

#[derive(Debug, Clone, Default)]
pub struct DirtyCompileStats {
    pub dirty: usize,
    pub compiled: usize,
    pub failed: usize,
}

/// SH7: rust-compile only DAG shards whose seed fingerprint is stale, then pack the image.
pub fn compile_dirty_dag_seeds() -> Result<DirtyCompileStats, String> {
    let dirty = missing_compiler_dag_seeds()?;
    let n = dirty.len();
    eprintln!("SH7 dirty={n}");
    let mut stats = DirtyCompileStats {
        dirty: n,
        compiled: 0,
        failed: 0,
    };
    if n == 0 {
        return Ok(stats);
    }
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(2)
        .min(n)
        .max(1);
    if workers == 1 {
        for rel in &dirty {
            match rust_compile_write_seed(rel) {
                Ok(_) => stats.compiled += 1,
                Err(e) => {
                    eprintln!("SH7 fail {rel}: {e}");
                    stats.failed += 1;
                }
            }
        }
    } else {
        let chunk = (n + workers - 1) / workers;
        let mut joins = Vec::new();
        for part in dirty.chunks(chunk) {
            let part = part.to_vec();
            joins.push(std::thread::spawn(move || {
                let mut compiled = 0usize;
                let mut failed = 0usize;
                for rel in part {
                    match rust_compile_write_seed(&rel) {
                        Ok(_) => compiled += 1,
                        Err(e) => {
                            eprintln!("SH7 fail {rel}: {e}");
                            failed += 1;
                        }
                    }
                }
                (compiled, failed)
            }));
        }
        for j in joins {
            let (c, f) = j.join().map_err(|_| "SH7 worker panicked".to_string())?;
            stats.compiled += c;
            stats.failed += f;
        }
    }
    if stats.failed == 0 {
        write_image_from_dag_dir()?;
    }
    Ok(stats)
}

/// SH7b: rust-compile dirty files in an import DAG (app/`lib` trees), write `.kabootar/cache`.
pub fn compile_dirty_product_tree(entry: &str) -> Result<DirtyCompileStats, String> {
    let dag = walk_import_dag(entry)?;
    let mut dirty_paths = Vec::new();
    for rel in dag {
        let path = resolve_kab_path(&rel);
        if !Path::new(&path).is_file() {
            continue;
        }
        let mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        let hit = mtime
            .and_then(|t| super::read_bytecode_cache(&path, t).ok().flatten())
            .is_some();
        if !hit {
            dirty_paths.push(path);
        }
    }
    let n = dirty_paths.len();
    eprintln!("SH7b dirty={n} entry={entry}");
    let mut stats = DirtyCompileStats {
        dirty: n,
        compiled: 0,
        failed: 0,
    };
    if n == 0 {
        return Ok(stats);
    }
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(2)
        .min(n)
        .max(1);
    let compile_one = |path: &str| -> Result<(), String> {
        let program = compile_file(path)?;
        super::write_compile_marker(path, &program)?;
        Ok(())
    };
    if workers == 1 {
        for path in &dirty_paths {
            match compile_one(path) {
                Ok(()) => stats.compiled += 1,
                Err(e) => {
                    eprintln!("SH7b fail {path}: {e}");
                    stats.failed += 1;
                }
            }
        }
    } else {
        let chunk = (n + workers - 1) / workers;
        let mut joins = Vec::new();
        for part in dirty_paths.chunks(chunk) {
            let part = part.to_vec();
            joins.push(std::thread::spawn(move || {
                let mut compiled = 0usize;
                let mut failed = 0usize;
                for path in part {
                    match compile_file(&path).and_then(|p| {
                        super::write_compile_marker(&path, &p)?;
                        Ok(())
                    }) {
                        Ok(()) => compiled += 1,
                        Err(e) => {
                            eprintln!("SH7b fail {path}: {e}");
                            failed += 1;
                        }
                    }
                }
                (compiled, failed)
            }));
        }
        for j in joins {
            let (c, f) = j.join().map_err(|_| "SH7b worker panicked".to_string())?;
            stats.compiled += c;
            stats.failed += f;
        }
    }
    Ok(stats)
}

pub fn compiler_image_path() -> PathBuf {
    self_host_dir().join("seed").join("compiler.kbcb")
}

#[derive(Clone)]
struct ImageMod {
    fingerprint: String,
    module: BytecodeModule,
}

fn image_cache() -> &'static Mutex<Option<HashMap<String, ImageMod>>> {
    static CACHE: Mutex<Option<HashMap<String, ImageMod>>> = Mutex::new(None);
    &CACHE
}

pub fn reload_compiler_image() {
    if let Ok(mut slot) = image_cache().lock() {
        *slot = load_compiler_image().ok();
    }
}

fn compiler_image_mod(basename: &str) -> Option<ImageMod> {
    let mut slot = image_cache().lock().ok()?;
    if slot.is_none() {
        *slot = load_compiler_image().ok();
    }
    slot.as_ref()?.get(basename).cloned()
}

fn fingerprint_from_seed_text(text: &str) -> Option<&str> {
    text.lines()
        .find(|l| l.starts_with("fingerprint="))
        .map(|l| l.trim_start_matches("fingerprint="))
}

fn load_compiler_image() -> Result<HashMap<String, ImageMod>, String> {
    let bytes = fs::read(compiler_image_path()).map_err(|e| e.to_string())?;
    parse_compiler_image(&bytes)
}

fn parse_compiler_image(bytes: &[u8]) -> Result<HashMap<String, ImageMod>, String> {
    if bytes.len() < 9 || bytes[0..4] != *IMAGE_MAGIC {
        return Err("not an SH1 compiler image".into());
    }
    if bytes[4] != IMAGE_VERSION {
        return Err(format!("unsupported SH1 image version {}", bytes[4]));
    }
    let n = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let mut i = 9usize;
    let mut map = HashMap::new();
    for _ in 0..n {
        if i + 4 > bytes.len() {
            return Err("SH1 image truncated (name len)".into());
        }
        let nl = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        i += 4;
        if i + nl + 4 > bytes.len() {
            return Err("SH1 image truncated (name)".into());
        }
        let name = std::str::from_utf8(&bytes[i..i + nl])
            .map_err(|e| e.to_string())?
            .to_string();
        i += nl;
        let bl = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        i += 4;
        if i + bl > bytes.len() {
            return Err("SH1 image truncated (blob)".into());
        }
        let text = std::str::from_utf8(&bytes[i..i + bl]).map_err(|e| e.to_string())?;
        i += bl;
        let Some(fp) = fingerprint_from_seed_text(text) else {
            continue;
        };
        let module = deserialize(text)?;
        map.insert(
            name,
            ImageMod {
                fingerprint: fp.to_string(),
                module,
            },
        );
    }
    Ok(map)
}

fn pack_compiler_image(entries: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(IMAGE_MAGIC);
    out.push(IMAGE_VERSION);
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, text) in entries {
        let nb = name.as_bytes();
        out.extend_from_slice(&(nb.len() as u32).to_le_bytes());
        out.extend_from_slice(nb);
        let body = text.as_bytes();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
    }
    out
}

/// Product `self_host/vm*.kab` (not probes): rust-compile like the SH1 compiler DAG.
/// Densified SH6 shards are too large for live self-host compile on first import.
pub fn is_self_host_vm_path(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    let name = norm.rsplit('/').next().unwrap_or(&norm);
    if is_probe_name(name) {
        return false;
    }
    name == "vm.kab" || name.starts_with("vm_")
}

pub fn is_compile_dag_path(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    static DAG: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let dag = DAG.get_or_init(|| walk_compile_dag().unwrap_or_default());
    dag.iter().any(|rel| {
        let rel = rel.replace('\\', "/");
        norm == rel || norm.ends_with(&rel)
    })
}

/// DAG `.kab` paths whose fingerprint does not match seed/image.
pub fn missing_compiler_dag_seeds() -> Result<Vec<String>, String> {
    let mut missing = Vec::new();
    for rel in walk_compile_dag()? {
        if is_probe_name(&rel) {
            continue;
        }
        if read_matching_seed(&rel)?.is_none() {
            missing.push(rel);
        }
    }
    Ok(missing)
}

pub fn seed_kbc_candidates(path: &str) -> Vec<PathBuf> {
    let norm = path.replace('\\', "/");
    let base_name = Path::new(&norm)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if !base_name.ends_with(".kab") {
        return Vec::new();
    }
    if !(norm.contains("self_host/") || norm.contains("self_host")) {
        return Vec::new();
    }
    let seed_root = self_host_dir().join("seed");
    vec![
        seed_root.join(format!("{base_name}.kbc")),
        seed_root.join("dag").join(format!("{base_name}.kbc")),
    ]
}

fn seed_text_if_fingerprint(text: &str, expected: &str) -> Result<Option<BytecodeModule>, String> {
    if !text.starts_with(FORMAT_HEADER) {
        return Ok(None);
    }
    let Some(line) = text.lines().find(|l| l.starts_with("fingerprint=")) else {
        return Ok(None);
    };
    let got = line.trim_start_matches("fingerprint=");
    if got != expected {
        return Ok(None);
    }
    Ok(Some(deserialize(text)?))
}

pub fn read_matching_seed(path: &str) -> Result<Option<BytecodeModule>, String> {
    read_matching_seed_src(path, None)
}

pub fn read_matching_seed_src(
    path: &str,
    source: Option<&str>,
) -> Result<Option<BytecodeModule>, String> {
    let path = resolve_kab_path(path);
    let owned;
    let source = match source {
        Some(s) => s,
        None => {
            owned = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => return Ok(None),
            };
            owned.as_str()
        }
    };
    let expected = source_fingerprint(&path, source);
    let base_name = Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    for key in [format!("{base_name}.kbc"), base_name.to_string()] {
        if let Some(entry) = compiler_image_mod(&key) {
            if entry.fingerprint == expected {
                return Ok(Some(entry.module));
            }
        }
    }
    for seed in seed_kbc_candidates(&path) {
        let text = match fs::read_to_string(&seed) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Some(bc) = seed_text_if_fingerprint(&text, &expected)? {
            return Ok(Some(bc));
        }
    }
    Ok(None)
}

pub fn write_seed_dag_file(path: &str, program: &CompiledProgram) -> Result<PathBuf, String> {
    let Some(bc) = program.bytecode.as_ref() else {
        return Err("no bytecode".into());
    };
    let base_name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("seed basename")?;
    let dir = self_host_dir().join("seed").join("dag");
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir seed/dag: {e}"))?;
    let dest = dir.join(format!("{base_name}.kbc"));
    let mut text = serialize(bc);
    let source = fs::read_to_string(path).unwrap_or_default();
    let fp = source_fingerprint(path, &source);
    let rel = format!("self_host/{base_name}");
    text.push_str(&format!(
        "\nsource={rel}\nstatements={}\nfingerprint={fp}\n",
        program.stmt_count
    ));
    fs::write(&dest, text).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(dest)
}

pub fn rust_compile_write_seed(path: &str) -> Result<PathBuf, String> {
    let program = compile_file(&resolve_kab_path(path))?;
    write_seed_dag_file(&resolve_kab_path(path), &program)
}

fn resolve_kab_path(path: &str) -> String {
    if Path::new(path).is_file() {
        return path.replace('\\', "/");
    }
    let host = self_host_dir();
    let root = host.parent().unwrap_or(Path::new("."));
    let p = root.join(path);
    if p.is_file() {
        return p.to_string_lossy().replace('\\', "/");
    }
    path.replace('\\', "/")
}

fn write_image_from_dag_dir() -> Result<(), String> {
    let dir = self_host_dir().join("seed").join("dag");
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir seed/dag: {e}"))?;
    let mut names: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| format!("read seed/dag: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("kbc"))
        .collect();
    names.sort();
    let mut entries = Vec::new();
    for path in names {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        entries.push((name, text));
    }
    let packed = pack_compiler_image(&entries);
    fs::write(compiler_image_path(), packed)
        .map_err(|e| format!("write compiler.kbcb: {e}"))?;
    reload_compiler_image();
    Ok(())
}

/// SH1 facades: compile.kab + parse/emit/serialize/ownership (not the 500-file DAG).
pub fn write_compiler_facade_seeds() -> Result<usize, String> {
    let dir = self_host_dir();
    let mut n = 0usize;
    for name in FACADE_KAB {
        let path = dir.join(name);
        let path_s = path.to_string_lossy().replace('\\', "/");
        rust_compile_write_seed(&path_s)?;
        n += 1;
    }
    Ok(n)
}

/// Full compile DAG → `self_host/seed/dag/*.kbc` + packed `seed/compiler.kbcb`.
pub fn write_compiler_dag_seeds() -> Result<usize, String> {
    let dag = walk_compile_dag()?;
    let mut n = 0usize;
    for rel in dag {
        if is_probe_name(&rel) {
            continue;
        }
        rust_compile_write_seed(&rel)?;
        n += 1;
    }
    write_image_from_dag_dir()?;
    Ok(n)
}

pub fn facade_kab_names() -> &'static [&'static str] {
    FACADE_KAB
}
