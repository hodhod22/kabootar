//! v2.31 — array spread literals, versioned import, module .kbc, top-level return

use kabootar::bytecode::can_compile;
use kabootar::bytecode::compile_source;
use kabootar::compile::{read_bytecode_cache, write_compile_marker};
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::modules::import_module;
use kabootar::value::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn with_project_root<F: FnOnce()>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&root).unwrap();
    f();
    if let Some(dir) = old {
        let _ = std::env::set_current_dir(dir);
    }
}

fn with_temp_lib<F: FnOnce(&std::path::PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v231_{}_{}",
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
fn bytecode_array_spread_literal() {
    assert!(can_compile("let a = [1, 2]; let xs = [...a, 3]; len(xs)"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = [1, 2]
        let xs = [...a, 3, 4]
        len(xs)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn bytecode_object_spread_literal() {
    assert!(can_compile(r#"let o = { ...{ x: 1 }, y: 2 }; o.y"#));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let base = { a: 1 }
        let o = { ...base, b: 2 }
        o.a + o.b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_versioned_import() {
    with_project_root(|| {
        assert!(can_compile(r#"import "greet@1.0"; greet("x")"#));
        let mut env = create_global_env();
        import_module("greet@1.0", &mut env).unwrap();
        let v = eval_source(r#"greet("Ada")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "Hello, Ada"));
    });
}

#[test]
fn bytecode_top_level_return() {
    assert!(can_compile("return 10 + 5"));
    let mut env = create_global_env();
    let v = eval_source("return 10 + 5", &mut env).unwrap();
    assert!(matches!(v, Value::Number(15)));
}

#[test]
fn module_kbc_cache_preserves_exports() {
    with_temp_lib(|dir| {
        let source = "pub fn triple(n) { return n * 3 }";
        fs::write(dir.join("lib/arith.kab"), source).unwrap();
        let program = compile_source(source).unwrap();
        let bc = program.bytecode.as_ref().unwrap();
        assert_eq!(bc.exports, vec!["triple".to_string()]);
        assert_eq!(bc.functions.len(), 1);

        write_compile_marker("lib/arith.kab", &program).unwrap();
        let mtime = fs::metadata(dir.join("lib/arith.kab"))
            .unwrap()
            .modified()
            .unwrap();
        let cached = read_bytecode_cache("lib/arith.kab", mtime)
            .unwrap()
            .expect("cached bytecode");
        assert_eq!(cached.exports, bc.exports);
        assert_eq!(cached.functions.len(), 1);
        assert_eq!(cached.functions[0].name, "triple");

        let mut env = create_global_env();
        import_module("arith", &mut env).unwrap();
        let v = eval_source("triple(4)", &mut env).unwrap();
        assert!(matches!(v, Value::Number(12)));
        assert!(env.get("triple").is_some());
    });
}

#[test]
fn bytecode_pub_async_fn_export() {
    with_temp_lib(|dir| {
        fs::write(
            dir.join("lib/worker.kab"),
            r#"
            pub async fn one() {
                return 1
            }
        "#,
        )
        .unwrap();

        let mut env = create_global_env();
        import_module("worker", &mut env).unwrap();
        let v = eval_source("await one()", &mut env).unwrap();
        assert!(matches!(v, Value::Number(1)));
    });
}
