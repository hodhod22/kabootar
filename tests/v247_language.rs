//! v2.47 — match float and string literal patterns

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_match_float_compiles() {
    assert!(can_compile("match 1.5 { 1.5 => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_float_true_arm() {
    let mut env = create_global_env();
    let v = eval_source("match 1.5 { 1.5 => 7, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_match_float_wildcard_fallback() {
    let mut env = create_global_env();
    let v = eval_source("match 2.0 { 1.5 => 1, _ => 2 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_float_cross_number_value() {
    let mut env = create_global_env();
    let v = eval_source("match 1 { 1.0 => 9, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_match_string_compiles() {
    assert!(can_compile("match \"hi\" { \"hi\" => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_string_true_arm() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match "kabootar" {
            "kabootar" => 3,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_string_wildcard_fallback() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match "a" {
            "b" => 1,
            _ => 2
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_string_literal_not_variable_bind() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match "x" {
            "y" => 1,
            s => len(s)
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_string_variable_bind() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match "abc" {
            "zzz" => 0,
            s => len(s)
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_float_with_guard() {
    assert!(can_compile("match 2.0 { 2.0 if 1 < 3 => 5, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match 2.0 {
            2.0 if 1 < 3 => 5,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}
