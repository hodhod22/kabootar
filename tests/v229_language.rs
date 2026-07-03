//! v2.29 — bytecode pub fn/pub let + file module exports

use kabootar_lib::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::project::version::strip_version_directive;
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
fn bytecode_pub_fn_and_pub_let_compile() {
    assert!(can_compile("pub fn dbl(n) { return n * 2 }\ndbl(21)"));
    assert!(can_compile("pub let x = 42\nx"));

    let mut env = create_global_env();
    let v = eval_source("pub fn dbl(n) { return n * 2 }\ndbl(21)", &mut env).unwrap();
    assert!(matches!(v, Value::Number(42)));
    assert!(env.is_exported("dbl"));
}

#[test]
fn bytecode_exports_serialize_roundtrip() {
    let program = compile_source(
        r#"
        pub let x = 1
        pub fn f() { return x }
        f()
    "#,
    )
    .unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    assert!(bc.exports.contains(&"x".to_string()));
    assert!(bc.exports.contains(&"f".to_string()));
    let restored = deserialize(&serialize(bc)).unwrap();
    assert_eq!(restored.exports, bc.exports);
    assert_eq!(restored.functions[0].name, "f");
}

#[test]
fn file_module_exports_only_public_bindings() {
    with_project_root(|| {
        let raw = fs::read_to_string("lib/greet.kab").unwrap();
        let (_, source) = strip_version_directive(&raw);
        assert!(can_compile(&source));

        let mut env = create_global_env();
        import_module("greet", &mut env).unwrap();
        assert!(env.get("greet").is_some());
        assert!(env.get("secret").is_none());

        let v = eval_source(r#"greet("Ada")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "Hello, Ada"));
    });
}

#[test]
fn file_module_pub_let_exports() {
    with_project_root(|| {
        let mut env = create_global_env();
        import_module("config", &mut env).unwrap();
        assert!(matches!(
            env.get("APP_NAME"),
            Some(Value::String(s)) if s == "Kabootar"
        ));
        assert!(matches!(env.get("MAX_ITEMS"), Some(Value::Number(100))));
        let v = eval_source("limit_ok(50)", &mut env).unwrap();
        assert!(matches!(v, Value::Bool(true)));
    });
}
