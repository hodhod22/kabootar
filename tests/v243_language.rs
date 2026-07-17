//! v2.43 — nested member/index assignment

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_nested_member_assign() {
    assert!(can_compile("let o = { a: { b: 1 } }\no.a.b = 2\no.a.b"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { a: { b: 1 } }
        o.a.b = 2
        o.a.b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_nested_index_assign() {
    assert!(can_compile("let xs = [[1]]\nxs[0][0] = 9\nxs[0][0]"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [[1]]
        xs[0][0] = 9
        xs[0][0]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_mixed_member_index_assign() {
    assert!(can_compile("let o = { items: [1] }\no.items[0] = 5\no.items[0]"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { items: [1] }
        o.items[0] = 5
        o.items[0]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_index_member_assign() {
    assert!(can_compile("let xs = [{ x: 1 }]\nxs[0].x = 7\nxs[0].x"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [{ x: 1 }]
        xs[0].x = 7
        xs[0].x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_nested_member_assign_returns_value() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { a: { b: 1 } }
        o.a.b = 42
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_simple_member_assign_still_works() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 1 }
        o.x = 3
        o.x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_simple_index_assign_still_works() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1]
        xs[0] = 8
        xs[0]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(8)));
}

#[test]
fn bytecode_this_nested_member_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Box {
            x: Number
            fn init(v) {
                this.x = v
            }
            fn set(n) {
                let inner = { v: 0 }
                inner.v = n
                return inner.v
            }
        }
        let b = Box(1)
        b.set(11)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(11)));
}
