//! v2.32 — class field expression defaults + pub import re-exports

use kabootar_lib::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::value::Value;
use std::fs;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_lib<F: FnOnce(&std::path::PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v232_{}_{}",
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
fn bytecode_class_field_expr_default() {
    assert!(can_compile(
        r#"
        class Counter {
            count: number = 1 + 2;
        }
        let c = Counter()
        c.count
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Counter {
            count: number = 1 + 2;
        }
        let c = Counter()
        c.count
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_class_field_default_uses_module_scope() {
    assert!(can_compile(
        r#"
        let base = 10
        class Box {
            size: number = base + 5;
        }
        let b = Box()
        b.size
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let base = 10
        class Box {
            size: number = base + 5;
        }
        let b = Box()
        b.size
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(15)));
}

#[test]
fn class_field_expr_default_serialize_roundtrip() {
    let program = compile_source(
        r#"
        class N {
            v: number = 2 * 3;
        }
        N().v
    "#,
    )
    .unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let field = &bc.classes[0].fields[0];
    assert_eq!(field.name, "v");
    assert!(field.default_const.is_none());
    assert!(!field.default_code.is_empty());

    let restored = deserialize(&serialize(bc)).unwrap();
    let rf = &restored.classes[0].fields[0];
    assert_eq!(rf.default_code, field.default_code);
    assert_eq!(rf.default_globals, field.default_globals);
}

#[test]
fn bytecode_pub_import_reexports() {
    with_temp_lib(|dir| {
        fs::write(
            dir.join("lib/greet.kab"),
            r#"
            pub fn hello(name) {
                return "Hi " + name
            }
        "#,
        )
        .unwrap();
        fs::write(
            dir.join("lib/bridge.kab"),
            r#"
            pub import "greet"
        "#,
        )
        .unwrap();

        assert!(can_compile(
            &fs::read_to_string(dir.join("lib/bridge.kab")).unwrap()
        ));

        let mut env = create_global_env();
        import_module("bridge", &mut env).unwrap();
        assert!(env.get("hello").is_some());
        assert!(env.get("greet").is_none());

        let v = eval_source(r#"hello("Ada")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "Hi Ada"));
    });
}

#[test]
fn bytecode_pub_import_in_program() {
    with_temp_lib(|dir| {
        fs::write(
            dir.join("lib/scale.kab"),
            r#"pub fn double(n) { return n * 2 }"#,
        )
        .unwrap();
        fs::write(
            dir.join("lib/wrap.kab"),
            r#"
            pub import "scale"
            pub fn quad(n) { return double(double(n)) }
        "#,
        )
        .unwrap();

        let mut env = create_global_env();
        import_module("wrap", &mut env).unwrap();
        assert!(env.get("quad").is_some());
        assert!(env.get("double").is_some());

        let v = eval_source("quad(3)", &mut env).unwrap();
        assert!(matches!(v, Value::Number(12)));
        let v2 = eval_source("double(5)", &mut env).unwrap();
        assert!(matches!(v2, Value::Number(10)));
    });
}
