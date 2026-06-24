//! v2.20 — bytecode v2.2 constructs (objects, index write, for-in, for-classic)

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_object_literal_and_member() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let u = { name: "Ada", age: 36 }
        u.name
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Ada"));
}

#[test]
fn bytecode_array_index_write() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [10, 20, 30]
        xs[1] = 99
        xs[1]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(99)));
}

#[test]
fn bytecode_for_of_over_array() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        for x of [1, 2, 3] {
            sum = sum + x
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_for_classic_loop() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let total = 0
        for let i = 0; i < 4; i = i + 1 {
            total = total + i
        }
        total
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_can_compile_v22_subset() {
    assert!(can_compile(
        r#"
        let u = { x: 1 }
        u.x
    "#
    ));
    assert!(can_compile("for x of [1] { x }"));
    assert!(can_compile("let xs = [...a, 1]"));
}
