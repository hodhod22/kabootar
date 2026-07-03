//! v2.1 project features tests

use kabootar_lib::cli;
use kabootar_lib::compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::project::manifest::{parse_manifest, version_matches};
use kabootar_lib::value::Value;
use std::fs;
use std::path::PathBuf;

fn with_project_root<F: FnOnce()>(f: F) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&root).unwrap();
    f();
    if let Some(dir) = old {
        let _ = std::env::set_current_dir(dir);
    }
}

#[test]
fn pub_let_exported_from_module() {
    with_project_root(|| {
        let mut env = create_global_env();
        import_module("config", &mut env).unwrap();
        assert!(matches!(env.get("APP_NAME"), Some(Value::String(s)) if s == "Kabootar"));
        assert!(env.get("MAX_ITEMS").is_some());
        assert!(env.get("limit_ok").is_some());
    });
}

#[test]
fn import_with_version_constraint() {
    with_project_root(|| {
        let mut env = create_global_env();
        import_module("greet@1.0", &mut env).unwrap();
        let v = eval_source(r#"greet("x")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s.contains("x")));
    });
}

#[test]
fn import_wrong_version_fails() {
    with_project_root(|| {
        let mut env = create_global_env();
        let err = import_module("greet@9.9", &mut env).unwrap_err();
        assert!(err.contains("version"));
    });
}

#[test]
fn compile_source_caches_statements() {
    let p = compile::compile_source("pub let x = 42").unwrap();
    assert_eq!(p.stmt_count, 1);
}

#[test]
fn compile_file_writes_marker() {
    let dir = std::env::temp_dir().join(format!("kabootar_compile_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.kab");
    fs::write(&path, "1 + 1").unwrap();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let n = cli::compile_file_report("t.kab").unwrap();
    assert_eq!(n.0, 1);
    assert!(dir.join(".kabootar/cache/t.kab.kbc").is_file());
    let _ = std::env::set_current_dir(old);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn manifest_parses_dependencies() {
    let m = parse_manifest(
        r#"
version = "1.0.0"
[dependencies]
greet = "1.0.0"
"#,
    )
    .unwrap();
    assert_eq!(m.dependencies.get("greet").map(String::as_str), Some("1.0.0"));
}

#[test]
fn version_match_prefix() {
    assert!(version_matches("1.0.2", "1.0"));
}
