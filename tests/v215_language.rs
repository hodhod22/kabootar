//! v2.15 — array and object patterns in match

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn match_empty_array_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [] {
            [] => 1,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn match_fixed_length_array_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [10, 20] {
            [a, b] => a + b,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(30)));
}

#[test]
fn match_array_rest_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3, 4] {
            [head, ...tail] => head + len(tail),
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn match_array_rest_between_fixed_ends() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3, 4] {
            [first, ...mid, last] => first + last + len(mid),
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn match_object_field_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match { name: "Ada", age: 36 } {
            { name: n } => n,
            _ => ""
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Ada"));
}

#[test]
fn match_object_shorthand_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let u = { name: "Lin", age: 22 }
        match u {
            { name, age } => age,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(22)));
}

#[test]
fn match_object_rest_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match { a: 1, b: 2, c: 3 } {
            { a: x, ...rest } => x + len(keys(rest)),
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn match_nested_result_and_array() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match Ok([2, 3]) {
            Ok([a, b]) => a * b,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn match_empty_object_pattern() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match {} {
            {} => 1,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}
