//! v2.19 — bytecode arrays, index, while, assign

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_array_literal_and_index() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [10, 20, 30]
        xs[1]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(20)));
}

#[test]
fn bytecode_array_length_member() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1, 2, 3, 4]
        xs.length
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn bytecode_while_loop_with_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1, 2, 3]
        let i = 0
        let sum = 0
        while i < xs.length {
            sum = sum + xs[i]
            i = i + 1
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_can_compile_arrays_and_while() {
    let src = r#"
        let xs = [1, 2]
        let i = 0
        while i < xs.length {
            i = i + 1
        }
        xs[0]
    "#;
    assert!(can_compile(src));
    let mut env = create_global_env();
    let v = eval_source(
        r#"bytecode_can_compile("let a = [1, 2, 3]; a[1]")"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn spread_array_uses_bytecode() {
    assert!(can_compile("let xs = [...a, 1]"));
}
