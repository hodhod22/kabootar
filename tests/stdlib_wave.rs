//! Stdlib wave G1 — hyperbolic Math + string match/search.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval_num(src: &str) -> f64 {
    match eval_source(src, &mut create_global_env()).unwrap() {
        Value::Number(n) => n as f64,
        Value::Float(f) => f,
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn math_sinh_cosh_tanh() {
    let s = eval_num("sinh(0)");
    assert!(s.abs() < 1e-9);
    assert!(eval_num("cosh(0)") - 1.0 < 1e-9);
    assert!(eval_num("tanh(0)").abs() < 1e-9);
}

#[test]
fn math_asinh_at_domain() {
    assert!(eval_num("asinh(0)").abs() < 1e-9);
    assert!(eval_num("acosh(1)").abs() < 1e-9);
    assert!(eval_num("atanh(0)").abs() < 1e-9);
}

#[test]
fn string_match_and_search() {
    assert_eq!(eval_num("str_search(\"abc123\", \"\\\\d+\")"), 3.0);
    let m = eval_source(
        "str_match(\"foo42bar\", \"(\\\\d+)\")",
        &mut create_global_env(),
    )
    .unwrap();
    let Value::Array(items) = m else {
        panic!("expected array match, got {m:?}");
    };
    assert!(items.len() >= 1);
    assert!(matches!(&items[0], Value::String(s) if s == "42"));
}

#[test]
fn string_locale_compare() {
    assert_eq!(eval_num("str_locale_compare(\"a\", \"b\")"), -1.0);
    assert_eq!(eval_num("str_locale_compare(\"b\", \"a\")"), 1.0);
    assert_eq!(eval_num("str_locale_compare(\"x\", \"x\")"), 0.0);
}
