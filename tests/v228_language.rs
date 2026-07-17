//! v2.28 — .kbc serialization for classes/interfaces + bytecode import

use kabootar_lib::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar_lib::bytecode::run_module;
use kabootar_lib::compile::{eval_program, read_bytecode_cache, write_compile_marker, CompiledProgram};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
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
                memory_mode: kabootar_lib::lang_preprocess::MemoryMode::Gc,
            },
            &mut env,
        )
        .unwrap();
        assert!(matches!(v, Value::Number(7)));
    });
}

#[test]
fn bytecode_fn_locals_survive_nested_call() {
    let source = r#"
fn inner() {
    let slot = 99
    return slot
}
fn outer() {
    let slot = 0
    inner()
    return slot
}
return outer()
"#;
    let program = compile_source(source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).unwrap();
    assert!(matches!(v, Value::Number(0)), "outer slot must not be clobbered by inner");
}

#[test]
fn bytecode_recursive_closure_captures_survive() {
    // L1: MakeArrowFn must not assign captured locals into the shared parent env.
    let source = r#"
fn walk(n) {
    let slot = n
    let get = () => slot
    if n <= 0 {
        return get()
    }
    let inner = walk(n - 1)
    return get() * 10 + inner
}
return walk(3)
"#;
    let program = compile_source(source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).unwrap();
    assert!(
        matches!(v, Value::Number(60)),
        "recursive frames must keep their own captured slot (got {v:?})"
    );
}

#[test]
fn bytecode_arrow_sees_later_store_local_in_same_frame() {
    let source = r#"
fn run() {
    let slot = 1
    let get = () => slot
    slot = 2
    return get()
}
return run()
"#;
    let program = compile_source(source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).unwrap();
    assert!(
        matches!(v, Value::Number(2)),
        "arrow should share the activation frame (got {v:?})"
    );
}

#[test]
fn bytecode_module_scales_to_many_top_level_fns() {
    // L2: register_functions must not deep-clone env per fn (was OOM/~7 fns).
    let mut source = String::new();
    for i in 0..40 {
        source.push_str(&format!("fn f{i}() {{\n    return {i}\n}}\n"));
    }
    source.push_str("return f0() + f39()\n");
    let program = compile_source(&source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    assert!(
        bc.functions.len() >= 40,
        "expected ≥40 top-level fns, got {}",
        bc.functions.len()
    );
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).unwrap();
    assert!(
        matches!(v, Value::Number(39)),
        "f0()+f39() must be 39 (got {v:?})"
    );
}

#[test]
fn bytecode_object_param_mutations_write_back() {
    let source = r#"
fn setKey(obj, k, v) {
    obj[k] = v
    return obj
}
fn run() {
    let env = {}
    env["el"] = 1
    setKey(env, "root", 2)
    return env["el"] + env["root"]
}
return run()
"#;
    let program = compile_source(source).unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).unwrap();
    assert!(
        matches!(v, Value::Number(3)),
        "object mutations must write back to caller local"
    );
}
