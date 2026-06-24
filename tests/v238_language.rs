//! v2.38 — grouped parenthesized expressions

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_grouped_arithmetic() {
    assert!(can_compile("(1 + 2)"));
    let mut env = create_global_env();
    let v = eval_source("(1 + 2)", &mut env).unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_grouped_nested_parens() {
    assert!(can_compile("((10))"));
    let mut env = create_global_env();
    let v = eval_source("((10))", &mut env).unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn bytecode_let_with_grouped_init() {
    assert!(can_compile("let x = (1 + 4)\nx"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x = (1 + 4)
        x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_grouped_object_spread() {
    assert!(can_compile("let o = { a: 1 }\n({ ...o, b: 2 })"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { a: 1 }
        let merged = ({ ...o, b: 2 })
        merged["a"] + merged["b"]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_grouped_block_expression() {
    assert!(can_compile("( { let n = 6\nn } )"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        ( { let n = 6
        n } )
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_paren_arrow_still_works() {
    assert!(can_compile("let f = (n) => n + 1\nf(4)"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = (n) => n + 1
        f(4)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_iife_grouped_arrow_still_works() {
    assert!(can_compile("((n) => n * 3)(5)"));
    let mut env = create_global_env();
    let v = eval_source("((n) => n * 3)(5)", &mut env).unwrap();
    assert!(matches!(v, Value::Number(15)));
}
