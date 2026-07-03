//! v2.37 — block expressions as values

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_block_expression_value() {
    assert!(can_compile("let x = { let y = 2\ny }\nx"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x = { let y = 2
        y }
        x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_bare_block_expression() {
    assert!(can_compile("{ let a = 10\na }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        { let a = 10
        a }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn bytecode_block_in_binary_expression() {
    assert!(can_compile("1 + { let n = 3\nn }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        1 + { let n = 3
        n }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn bytecode_block_in_ternary_expression() {
    assert!(can_compile("true ? { let n = 3\nn + 4 } : 0"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        true ? { let n = 3
        n + 4 } : 0
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_minimal_block_expression() {
    assert!(can_compile("{ null }"));
    let mut env = create_global_env();
    let v = eval_source("{ null }", &mut env).unwrap();
    assert!(matches!(v, Value::Null));
}

#[test]
fn bytecode_nested_block_expression() {
    assert!(can_compile("{ { let inner = 5\ninner } + 1 }"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        { { let inner = 5
        inner } + 1 }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_empty_block_expression() {
    assert!(can_compile("let x = { }\nlen(keys(x))"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x = { }
        len(keys(x))
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(0)));
}

#[test]
fn bytecode_block_in_call_argument() {
    assert!(can_compile("let f = (x) => x * 2\nf({ let n = 4\nn })"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = (x) => x * 2
        f({ let n = 4
        n })
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(8)));
}
