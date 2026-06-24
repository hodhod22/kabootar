//! v2.18 — bytecode VM and .kbc cache

use kabootar::bytecode::{can_compile, compile_source, CompiledProgram};
use kabootar::compile::{eval_program, read_bytecode_cache, write_compile_marker};
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;
use std::fs;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_dir<F: FnOnce(&std::path::PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v218_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&dir).unwrap();
    f(&dir);
    if let Some(prev) = old {
        let _ = std::env::set_current_dir(prev);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bytecode_runs_arithmetic_faster_path() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = 6
        let b = 7
        a * b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_compiles_user_function_calls() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn inc(n) { return n + 1 }
        inc(41)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_can_compile_detects_support() {
    let mut env = create_global_env();
    let v = eval_source(r#"bytecode_can_compile("1 + 1")"#, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));

    assert!(can_compile(r#"import "math"; add(1, 1)"#));
}

#[test]
fn compile_writes_bytecode_kbc_file() {
    with_temp_dir(|dir| {
        fs::write(dir.join("fast.kab"), "let n = 20 + 22\nn").unwrap();
        let program = compile_source(&fs::read_to_string(dir.join("fast.kab")).unwrap()).unwrap();
        assert!(program.has_bytecode());
        write_compile_marker("fast.kab", &program).unwrap();
        let cache = dir.join(".kabootar/cache/fast.kab.kbc");
        assert!(cache.is_file());
        let text = fs::read_to_string(cache).unwrap();
        assert!(text.starts_with("kabootar-bytecode/1"));
    });
}

#[test]
fn bytecode_cache_loads_and_runs() {
    with_temp_dir(|dir| {
        fs::write(dir.join("cached.kab"), "let z = 100 - 58\nz").unwrap();
        let program = compile_source(&fs::read_to_string(dir.join("cached.kab")).unwrap()).unwrap();
        write_compile_marker("cached.kab", &program).unwrap();
        let mtime = fs::metadata(dir.join("cached.kab"))
            .unwrap()
            .modified()
            .unwrap();
        let bc = read_bytecode_cache("cached.kab", mtime).unwrap().expect("bc");
        let mut env = create_global_env();
        let v = eval_program(
            &CompiledProgram {
                stmts: vec![],
                bytecode: Some(bc),
                stmt_count: 0,
            },
            &mut env,
        )
        .unwrap();
        assert!(matches!(v, Value::Number(42)));
    });
}

#[test]
fn import_uses_bytecode_when_rest_compiles() {
    assert!(can_compile(r#"import "math"; add(2, 3)"#));
    let mut env = create_global_env();
    eval_source(r#"import "math"; add(2, 3)"#, &mut env).unwrap();
}

#[test]
fn ternary_in_bytecode() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 4
        n > 3 ? "big" : "small"
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "big"));
}
