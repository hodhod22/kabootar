//! v2.44 — compound assignment operators

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_plus_assign() {
    assert!(can_compile("let n = 1\nn += 2\nn"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 1
        n += 2
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_minus_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 5
        n -= 2
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_star_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 3
        n *= 4
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(12)));
}

#[test]
fn bytecode_slash_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 20
        n /= 4
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_percent_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 10
        n %= 3
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_index_plus_assign() {
    assert!(can_compile("let xs = [1]\nxs[0] += 3\nxs[0]"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1]
        xs[0] += 3
        xs[0]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn bytecode_member_plus_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 2 }
        o.x += 5
        o.x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_nested_member_plus_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { a: { b: 1 } }
        o.a.b += 4
        o.a.b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_plus_assign_returns_value() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 1
        n += 9
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}
