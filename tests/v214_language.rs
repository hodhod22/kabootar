//! v2.14 — ? operator, match guards, is()

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn result_question_unwraps_ok() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn ok_val() {
            return Ok(42)
        }
        ok_val()?
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn result_question_propagates_err() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn bad() {
            return Err("nope")
        }
        bad()?
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Result(Err(e)) if matches!(e.as_ref(), Value::String(s) if s == "nope")));
}

#[test]
fn result_question_in_function_return() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn step() {
            return Ok(7)
        }
        fn run() {
            return step()?
        }
        run()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn ternary_still_works_with_question_colon() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = 3
        n > 0 ? "yes" : "no"
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "yes"));
}

#[test]
fn match_guard_filters_arms() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn label(n) {
            match n {
                x if x > 0 => "positive",
                x if x < 0 => "negative",
                _ => "zero"
            }
        }
        label(5)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "positive"));
}

#[test]
fn match_guard_skips_non_matching_guard() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match 2 {
            x if x > 10 => "big",
            x if x > 0 => "small",
            _ => "other"
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "small"));
}

#[test]
fn is_checks_exact_class() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal { }
        class Dog extends Animal { }
        let d = Dog()
        instanceof(d, "Dog")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn is_checks_inherited_class() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal { }
        class Dog extends Animal { }
        let d = Dog()
        instanceof(d, "Animal")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn is_rejects_unrelated_class() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Cat { }
        class Dog { }
        let d = Dog()
        instanceof(d, "Cat")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn is_non_instance_is_false() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        instanceof(42, "Number")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(false)));
}
