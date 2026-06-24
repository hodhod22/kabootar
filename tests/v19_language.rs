//! v1.9 language core tests

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::modules::import_module;
use kabootar::value::{Environment, Value};

#[test]
fn array_literal_evaluates() {
    let mut env = create_global_env();
    let v = eval_source("[1, 2, 3]", &mut env).unwrap();
    assert!(matches!(v, Value::Array(items) if items.len() == 3));
}

#[test]
fn nested_functions_do_not_blow_memory() {
    let mut env = create_global_env();
    let code = r#"
        fn a() {
            fn b() {
                fn c() { return 1 }
                return c()
            }
            return b()
        }
        a()
    "#;
    let v = eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn import_file_module() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&root).unwrap();
    let mut env = create_global_env();
    import_module("greet", &mut env).unwrap();
    let v = eval_source(r#"greet("Kabootar")"#, &mut env).unwrap();
    if let Some(dir) = old {
        let _ = std::env::set_current_dir(dir);
    }
    assert!(matches!(v, Value::String(s) if s.contains("Kabootar")));
}

#[test]
fn undefined_variable_suggests_similar() {
    let mut env = create_global_env();
    env.set("science".into(), Value::Number(1));
    let err = eval_source("scince", &mut env).unwrap_err();
    assert!(err.contains("did you mean"));
}
