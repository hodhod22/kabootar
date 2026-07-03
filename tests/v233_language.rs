//! v2.33 — Result ? operator, object rest destructuring, callable spread calls

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_result_question_unwraps_ok() {
    assert!(can_compile(
        r#"
        fn ok_val() { return Ok(42) }
        ok_val()?
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn ok_val() { return Ok(42) }
        ok_val()?
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_result_question_propagates_err() {
    assert!(can_compile(
        r#"
        fn bad() { return Err("nope") }
        bad()?
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn bad() { return Err("nope") }
        bad()?
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Result(Err(e)) if matches!(e.as_ref(), Value::String(s) if s == "nope")));
}

#[test]
fn bytecode_result_question_in_function() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn step() { return Ok(7) }
        fn run() { return step()? }
        run()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_object_rest_destructuring() {
    assert!(can_compile(r#"let { a, ...rest } = { a: 1, b: 2 }; len(keys(rest))"#));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let { a, ...rest } = { a: 1, b: 2, c: 3 }
        a + len(keys(rest))
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_arrow_callable() {
    assert!(can_compile(r#"((n) => n + 1)(4)"#));
    let mut env = create_global_env();
    let v = eval_source("((n) => n + 1)(4)", &mut env).unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_spread_call_on_arrow() {
    assert!(can_compile(r#"let f = (a, b) => a + b; f(...[1, 2])"#));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = (a, b) => a + b
        f(...[1, 2])
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}
