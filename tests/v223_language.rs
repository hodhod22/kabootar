//! v2.23 — bytecode v2.4 subset: sync arrow functions, match (guards)

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_arrow_expression() {
    assert!(can_compile(
        r#"
        let double = (x) => x * 2
        double(5)
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let double = (x) => x * 2
        double(5)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn bytecode_arrow_block_body() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let add = (a, b) => {
            return a + b
        }
        add(3, 4)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_arrow_in_map() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1, 2, 3]
        len(map(xs, (x) => x * 2))
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn bytecode_match_option() {
    assert!(can_compile(
        r#"
        let x = Some(42)
        match x {
            Some(n) => n,
            _ => 0
        }
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let x = Some(42)
        match x {
            Some(n) => n,
            _ => 0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_match_result_err() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match Err("bad") {
            Ok(_) => 1,
            Err(e) => e
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "bad"));
}

#[test]
fn bytecode_match_literal_and_wildcard() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        match 7 {
            7 => "seven",
            _ => "other"
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "seven"));
}

#[test]
fn bytecode_match_guard() {
    assert!(can_compile(
        r#"
        match 2 {
            x if x > 10 => "big",
            x if x > 0 => "small",
            _ => "other"
        }
    "#
    ));
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
fn bytecode_async_arrow_uses_bytecode() {
    assert!(can_compile(
        r#"
        let f = async (n) => n + 1
        await f(9)
    "#
    ));
}
