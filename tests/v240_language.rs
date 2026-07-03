//! v2.40 — anonymous rest in match array patterns

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_match_rest_tail_anonymous() {
    assert!(can_compile("match [1, 2, 3] { [x, ...] => x, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3] {
            [x, ...] => x,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_match_rest_prefix_anonymous() {
    assert!(can_compile("match [1, 2, 3] { [..., x] => x, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3] {
            [..., x] => x,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_rest_between_anonymous() {
    assert!(can_compile(
        "match [1, 2, 3, 4] { [first, ..., last] => first + last, _ => 0 }"
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3, 4] {
            [first, ..., last] => first + last,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_match_rest_prefix_two_fixed() {
    assert!(can_compile("match [1, 2, 3, 4] { [..., a, b] => a + b, _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3, 4] {
            [..., a, b] => a + b,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_match_named_rest_still_works() {
    assert!(can_compile("match [1, 2, 3] { [x, ...rest] => x + len(rest), _ => 0 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1, 2, 3] {
            [x, ...rest] => x + len(rest),
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_rest_anonymous_no_capture() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        match [1, 2, 3] {
            [x, ...] => rest,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("Undefined") || err.contains("undefined"));
}

#[test]
fn bytecode_match_rest_anonymous_single_element() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [9] {
            [..., x] => x,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_match_rest_anonymous_fallback_arm() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match [1] {
            [a, b, ...] => 99,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(0)));
}
