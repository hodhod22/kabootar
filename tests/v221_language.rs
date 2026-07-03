//! v2.21 — bytecode const, method calls, template literals

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_const_reassign_error_in_program() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        const PI = 3
        PI = 4
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("const"));
}

#[test]
fn bytecode_const_persists_to_env() {
    let mut env = create_global_env();
    eval_source("const PI = 3", &mut env).unwrap();
    let err = eval_source("PI = 4", &mut env).unwrap_err();
    assert!(err.contains("const"));
}

#[test]
fn bytecode_template_literal() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let name = "Kabootar"
        `Hej ${name}!`
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Hej Kabootar!"));
}

#[test]
fn bytecode_map_with_user_functions() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1, 2, 3, 4]
        fn double(x) { return x * 2 }
        fn gt4(x) { return x > 4 }
        let ys = map(xs, double)
        let zs = filter(ys, gt4)
        zs.length
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_method_call_on_object_field() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn bump(n) { return n + 1 }
        let tools = { f: bump }
        tools.f(41)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_can_compile_v221_features() {
    assert!(can_compile("const X = 1"));
    assert!(can_compile(r#"let n = "a"; `x ${n}`"#));
    assert!(can_compile("let xs = [1]; xs.length"));
}
