//! v2.41 — anonymous rest in destructuring

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_let_array_rest_tail_anonymous() {
    assert!(can_compile("let [a, ...] = [1, 2, 3]\na"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [a, ...] = [1, 2, 3]
        a
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_let_array_rest_prefix_anonymous() {
    assert!(can_compile("let [..., b] = [1, 2, 3]\nb"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [..., b] = [1, 2, 3]
        b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_let_array_rest_between_anonymous() {
    assert!(can_compile("let [f, ..., l] = [1, 2, 3, 4]\nf + l"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [f, ..., l] = [1, 2, 3, 4]
        f + l
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_assign_array_rest_tail_anonymous() {
    assert!(can_compile("let a = 0\n[a, ...] = [7, 8, 9]\na"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = 0
        [a, ...] = [7, 8, 9]
        a
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_let_object_rest_anonymous() {
    assert!(can_compile("let { a, ... } = { a: 1, b: 2 }\na"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let { a, ... } = { a: 1, b: 2 }
        a
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_object_rest_anonymous() {
    assert!(can_compile("match { a: 1, b: 2 } { { a, ... } => a, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match { a: 1, b: 2 } {
            { a, ... } => a,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_named_array_rest_still_works() {
    assert!(can_compile("let [a, ...rest] = [1, 2, 3]\na + len(rest)"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [a, ...rest] = [1, 2, 3]
        a + len(rest)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_destructure_rest_anonymous_no_binding() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        let [x, ...] = [1, 2]
        rest
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("Undefined") || err.contains("undefined"));
}
