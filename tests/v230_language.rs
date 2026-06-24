//! v2.30 — module-scope bindings visible to calls + chained file imports

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::modules::import_module;
use kabootar::value::Value;
use std::fs;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_lib<F: FnOnce(&std::path::PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v230_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("lib")).unwrap();
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&dir).unwrap();
    f(&dir);
    if let Some(prev) = old {
        let _ = std::env::set_current_dir(prev);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bytecode_fn_reads_module_scope_let() {
    assert!(can_compile(
        r#"
        let base = 5
        fn add(n) { return base + n }
        add(7)
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let base = 5
        fn add(n) { return base + n }
        add(7)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(12)));
}

#[test]
fn bytecode_fn_reads_module_scope_const() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        const SCALE = 3
        fn mul(n) { return n * SCALE }
        mul(4)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(12)));
}

#[test]
fn bytecode_pub_fn_uses_pub_let_in_same_module() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        pub let MAX = 10
        pub fn ok(n) { return n <= MAX }
        ok(7)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
    assert!(env.is_exported("ok"));
    assert!(env.is_exported("MAX"));
}

#[test]
fn chained_file_imports_use_bytecode() {
    with_temp_lib(|dir| {
        fs::write(
            dir.join("lib/double.kab"),
            r#"pub fn twice(n) { return n * 2 }"#,
        )
        .unwrap();
        fs::write(
            dir.join("lib/quad.kab"),
            r#"
            import "double"
            pub fn four(n) { return twice(twice(n)) }
        "#,
        )
        .unwrap();

        let quad_src = fs::read_to_string(dir.join("lib/quad.kab")).unwrap();
        assert!(can_compile(&quad_src));

        let mut env = create_global_env();
        import_module("quad", &mut env).unwrap();
        assert!(env.get("four").is_some());
        assert!(env.get("twice").is_none());

        let v = eval_source("four(3)", &mut env).unwrap();
        assert!(matches!(v, Value::Number(12)));
    });
}
