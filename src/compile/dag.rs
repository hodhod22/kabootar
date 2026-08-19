//! SH0/SH1 — self_host import DAG inventory and committed compiler-image seeds.

use super::{compile_file, extract_kab_imports, source_fingerprint, CompiledProgram};
use crate::bytecode::{deserialize, serialize, BytecodeModule, FORMAT_HEADER};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn read_matching_seed(path: &str) -> Result<Option<BytecodeModule>, String> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let expected = source_fingerprint(path, &source);
    for seed in seed_kbc_candidates(path) {
        let text = match fs::read_to_string(&seed) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !text.starts_with(FORMAT_HEADER) {
            continue;
        }
        let Some(line) = text.lines().find(|l| l.starts_with("fingerprint=")) else {
            continue;
        };
        let got = line.trim_start_matches("fingerprint=");
        if got != expected {
            continue;
        }
        return Ok(Some(deserialize(&text)?));
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
    let program = compile_file(path)?;
    write_seed_dag_file(path, &program)
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

/// Full compile DAG → `self_host/seed/dag/*.kbc`. Opt-in: `KABOOTAR_SH1_WARM=1`.
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
    Ok(n)
}

pub fn facade_kab_names() -> &'static [&'static str] {
    FACADE_KAB
}
