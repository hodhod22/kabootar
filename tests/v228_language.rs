//! v2.28 — .kbc serialization for classes/interfaces + bytecode import

use kabootar::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar::bytecode::run_module;
use kabootar::compile::{eval_program, read_bytecode_cache, write_compile_marker, CompiledProgram};
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;
use std::fs;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_dir<F: FnOnce(&std::path::PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v228_{}_{}",
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
fn bytecode_import_math_uses_bytecode_path() {
    assert!(can_compile(r#"import "math"; add(2, 3)"#));
    let mut env = create_global_env();
    let v = eval_source(r#"import "math"; add(2, 3)"#, &mut env).unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn kbc_roundtrip_preserves_classes_and_interfaces() {
    let source = r#"
        interface Greeter { fn greet(); }
        class Person implements Greeter {
            name: string;
            fn init(n) { self.name = n }
            fn greet() { return "hi " + self.name }
        }
        let p = Person("Ada")
        is_impl(p, "Greeter")
    "#;
    assert!(can_compile(source));
    let program = compile_source(source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let text = serialize(bc);
    let restored = deserialize(&text).unwrap();
    assert_eq!(restored.interfaces.len(), 1);
    assert_eq!(restored.classes.len(), 1);
    let mut env = create_global_env();
    let v = run_module(&restored, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kbc_cache_loads_class_program() {
    with_temp_dir(|dir| {
        let source = r#"
            class Point {
                x: number;
                y: number;
                fn init(a, b) {
                    self.x = a
                    self.y = b
                }
                fn sum() { return self.x + self.y }
            }
            let p = Point(3, 4)
            p.sum()
        "#;
        fs::write(dir.join("point.kab"), source).unwrap();
        let program = compile_source(source).unwrap();
        assert!(program.has_bytecode());
        write_compile_marker("point.kab", &program).unwrap();
        let mtime = fs::metadata(dir.join("point.kab"))
            .unwrap()
            .modified()
            .unwrap();
        let bc = read_bytecode_cache("point.kab", mtime)
            .unwrap()
            .expect("bc");
        assert_eq!(bc.classes.len(), 1);
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
        assert!(matches!(v, Value::Number(7)));
    });
}
