//! v2.48 — match undefined and NaN literal patterns

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_match_undefined_compiles() {
    assert!(can_compile("match undefined { undefined => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_undefined_true_arm() {
    let mut env = create_global_env();
    let v = eval_source(
        "match undefined { undefined => 1, _ => 0 }",
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_undefined_wildcard_fallback() {
    let mut env = create_global_env();
    let v = eval_source(
        "match null { undefined => 1, _ => 2 }",
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_nan_compiles() {
    assert!(can_compile("match NaN { NaN => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_nan_true_arm() {
    let mut env = create_global_env();
    let v = eval_source("match NaN { NaN => 7, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_match_nan_wildcard_fallback() {
    let mut env = create_global_env();
    let v = eval_source("match 0 { NaN => 1, _ => 2 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_nan_distinct_from_null() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match NaN {
            null => 1,
            NaN => 2,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_undefined_distinct_from_null() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match undefined {
            null => 1,
            undefined => 2,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_nan_via_computed_value() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x = 0.0 / 0.0
        match x {
            NaN => 5,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_match_undefined_with_guard() {
    assert!(can_compile(
        "match undefined { undefined if 1 < 2 => 3, _ => 0 }"
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match undefined {
            undefined if 1 < 2 => 3,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}
