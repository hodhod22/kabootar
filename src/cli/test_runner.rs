//! `kabootar test` — run `*_test.kab` files; optional module/line coverage report.

use crate::compile;
use crate::evaluator::create_global_env;
use crate::value::{format_value, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TestResult {
    pub path: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct CoverageReport {
    pub files: Vec<CoverageFile>,
}

#[derive(Debug)]
pub struct CoverageFile {
    pub path: String,
    pub lines: usize,
    pub non_empty: usize,
    pub hit: bool,
}

pub fn discover_tests(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(out);
    }
    if !path.is_dir() {
        return Err(format!("test path not found: {}", path.display()));
    }
    walk_tests(path, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_tests(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for ent in fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))? {
        let ent = ent.map_err(|e| format!("read_dir: {e}"))?;
        let p = ent.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') || name.starts_with("target") {
                continue;
            }
            walk_tests(&p, out)?;
        } else if is_test_file(&p) {
            out.push(p);
        }
    }
    Ok(())
}

fn is_test_file(p: &Path) -> bool {
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name.ends_with("_test.kab") || name.ends_with(".test.kab")
}

pub fn run_test_file(path: &Path) -> TestResult {
    let path_s = path.to_string_lossy().replace('\\', "/");
    let mut env = create_global_env();
    // Force host compiler: self-host .kbc can eval to Null for small scripts.
    let prev = std::env::var("KABOOTAR_COMPILE").ok();
    std::env::set_var("KABOOTAR_COMPILE", "rust");
    std::env::set_var("KABOOTAR_VM", "host");
    compile::invalidate_file_cache(&path_s);
    let result = match fs::read_to_string(path) {
        Ok(src) => crate::evaluator::eval_source(&src, &mut env),
        Err(e) => Err(format!("read {path_s}: {e}")),
    };
    match prev {
        Some(v) => std::env::set_var("KABOOTAR_COMPILE", v),
        None => std::env::remove_var("KABOOTAR_COMPILE"),
    }
    match result {
        Ok(v) => {
            let ok = match &v {
                Value::Bool(b) => *b,
                Value::Object(m) => matches!(m.get("ok"), Some(Value::Bool(true))),
                Value::Null => true,
                _ => true,
            };
            TestResult {
                path: path_s,
                ok,
                message: format_value(&v),
            }
        }
        Err(e) => TestResult {
            path: path_s,
            ok: false,
            message: e,
        },
    }
}

pub fn run_tests(paths: &[PathBuf]) -> (usize, usize, Vec<TestResult>) {
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut results = Vec::new();
    for p in paths {
        let r = run_test_file(p);
        if r.ok {
            pass += 1;
        } else {
            fail += 1;
        }
        results.push(r);
    }
    (pass, fail, results)
}

/// Module-level coverage: mark lib/*.kab as hit if their path substr appears in test sources
/// or if they were successfully parsed; plus non-empty line counts.
pub fn coverage_for(roots: &[PathBuf], test_paths: &[PathBuf]) -> Result<CoverageReport, String> {
    let mut report = CoverageReport::default();
    let mut test_blob = String::new();
    for t in test_paths {
        if let Ok(s) = fs::read_to_string(t) {
            test_blob.push_str(&s);
            test_blob.push('\n');
        }
    }
    for root in roots {
        let files = super::doc::collect_kab_files(root)?;
        for f in files {
            let src = fs::read_to_string(&f).unwrap_or_default();
            let lines = src.lines().count();
            let non_empty = src.lines().filter(|l| !l.trim().is_empty()).count();
            let rel = f.to_string_lossy().replace('\\', "/");
            let stem = f
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let hit = test_blob.contains(&stem)
                || test_blob.contains(&rel)
                || test_blob.contains(&format!("\"{stem}\""))
                || test_blob.contains(&format!("import \"{stem}\""));
            report.files.push(CoverageFile {
                path: rel,
                lines,
                non_empty,
                hit,
            });
        }
    }
    Ok(report)
}

pub fn format_coverage(report: &CoverageReport) -> String {
    let total = report.files.len().max(1);
    let hit = report.files.iter().filter(|f| f.hit).count();
    let mut out = format!(
        "Coverage (module hit ≈ import mention): {hit}/{total} files ({:.0}%)\n",
        100.0 * hit as f64 / total as f64
    );
    for f in &report.files {
        if !f.hit {
            continue;
        }
        out.push_str(&format!(
            "  [HIT ] {} ({} non-empty lines)\n",
            f.path, f.non_empty
        ));
    }
    let miss = total.saturating_sub(hit);
    if miss > 0 {
        out.push_str(&format!("  ({miss} other lib files not mentioned in tests)\n"));
    }
    out
}
