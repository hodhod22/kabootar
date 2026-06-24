//! v2.36 — fn expressions + string-key object destructuring

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_object_literal_string_keys() {
    assert!(can_compile(r#"let h = { "Content-Type": "json" }; h["Content-Type"]"#));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let h = { "Content-Type": "application/json" }
        h["Content-Type"]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "application/json"));
}

#[test]
fn bytecode_object_destructure_string_key() {
    assert!(can_compile(
        r#"let o = { "x-key": 10 }; let { "x-key": n } = o; n"#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { "x-key": 10 }
        let { "x-key": n } = o
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn bytecode_match_object_string_key() {
    assert!(can_compile(
        r#"
        match { "id": 7 } {
            { "id": n } => n,
            _ => 0
        }
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match { "id": 7 } {
            { "id": n } => n,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_fn_expression_value() {
    assert!(can_compile(
        r#"
        let f = fn helper() {
            return 42
        }
        helper()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = fn helper() {
            return 42
        }
        f()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_fn_expression_registers_name() {
    let mut env = create_global_env();
    eval_source(
        r#"
        let f = fn helper() {
            return 1
        }
    "#,
        &mut env,
    )
    .unwrap();
    let v = eval_source("helper()", &mut env).unwrap();
    assert!(matches!(v, Value::Number(1)));
    let v2 = eval_source("f()", &mut env).unwrap();
    assert!(matches!(v2, Value::Number(1)));
}

#[test]
fn bytecode_async_fn_expression() {
    assert!(can_compile(
        r#"
        let f = async fn worker() {
            return 5
        }
        await f()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = async fn worker() {
            return 5
        }
        await f()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}
