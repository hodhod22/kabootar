//! v2.35 — let/const without init + array destructuring rest-between

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_let_without_init() {
    assert!(can_compile("let x\ntypeof(x)"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x
        typeof(x)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "undefined"));
}

#[test]
fn bytecode_const_without_init() {
    assert!(can_compile("const slot\ntypeof(slot)"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        const slot
        typeof(slot)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "undefined"));
}

#[test]
fn bytecode_array_destructure_rest_between() {
    assert!(can_compile(
        "let [first, ...mid, last] = [1, 2, 3, 4]\nfirst + last + len(mid)"
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [first, ...mid, last] = [1, 2, 3, 4]
        first + last + len(mid)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_assign_array_destructure_rest_between() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let pair = [10, 20, 30, 40]
        let first = 0
        let mid = []
        let last = 0
        [first, ...mid, last] = pair
        first + last + len(mid)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(52)));
}

#[test]
fn bytecode_match_array_rest_between_compiles() {
    assert!(can_compile(
        r#"
        match [1, 2, 3, 4] {
            [first, ...mid, last] => first + last + len(mid),
            _ => 0
        }
    "#
    ));
}

#[test]
fn bytecode_multiple_uninitialized_lets() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a
        let b
        typeof(a) + "," + typeof(b)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "undefined,undefined"));
}
