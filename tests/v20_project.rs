//! v2.0 project lifecycle tests

use kabootar_lib::cli::{self, templates};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::value::{Environment, Value};
use std::fs;

#[test]
fn pub_fn_exported_from_file_module() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&root).unwrap();
    let mut env = create_global_env();
    import_module("greet", &mut env).unwrap();
    if let Some(dir) = old {
        let _ = std::env::set_current_dir(dir);
    }
    assert!(env.get("greet").is_some());
    assert!(env.get("secret").is_none());
}

#[test]
fn science_quadratic_roots() {
    let mut env = create_global_env();
    import_module("science", &mut env).unwrap();
    let code = r#"
        quadratic(1, -5, 6)
    "#;
    let v = eval_source(code, &mut env).unwrap();
    match v {
        Value::Array(items) => assert_eq!(items.len(), 2),
        _ => panic!("expected array"),
    }
}

#[test]
fn mod_init_writes_template_files() {
    let dir = std::env::temp_dir().join(format!("kabootar_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    templates::write_project("api", &dir).unwrap();
    assert!(dir.join("kabootar.toml").is_file());
    assert!(dir.join("main.kab").is_file());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn run_file_evaluates_expression() {
    let dir = std::env::temp_dir().join(format!("kabootar_run_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.kab");
    fs::write(&path, "1 + 2").unwrap();
    let v = cli::run_file(path.to_str().unwrap()).unwrap();
    assert!(matches!(v, Value::Number(3)));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn private_fn_not_exported_from_math_builtin_still_works() {
    let mut env = create_global_env();
    import_module("math", &mut env).unwrap();
    assert!(env.get("add").is_some());
}
