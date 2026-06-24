//! v2.46 — match literal patterns (null, true, false)

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_match_null_compiles() {
    assert!(can_compile("match null { null => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_null_true_arm() {
    let mut env = create_global_env();
    let v = eval_source("match null { null => 1, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_null_wildcard() {
    let mut env = create_global_env();
    let v = eval_source("match 0 { null => 1, _ => 2 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_true_compiles() {
    assert!(can_compile("match true { true => 1, _ => 0 }"));
}

#[test]
fn bytecode_match_true_true_arm() {
    let mut env = create_global_env();
    let v = eval_source("match true { true => 7, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_match_false_compiles() {
    assert!(can_compile("match false { false => 3, _ => 0 }"));
}

#[test]
fn bytecode_match_false_true_arm() {
    let mut env = create_global_env();
    let v = eval_source("match false { false => 3, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_bool_literal_no_match_falls_through() {
    let mut env = create_global_env();
    let v = eval_source("match false { true => 1, _ => 0 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(0)));
}

#[test]
fn bytecode_match_number_wildcard_fallback() {
    let mut env = create_global_env();
    let v = eval_source("match 0 { 7 => 1, _ => 2 }", &mut env).unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_match_null_distinct_from_option_none() {
    assert!(can_compile("match None { None => 1, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match null {
            null => 1,
            None => 2,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_bool_with_guard() {
    assert!(can_compile("match true { true if 1 < 2 => 5, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match false {
            false if 2 > 1 => 5,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}
