//! v2.39 — `in` membership operator

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_in_array_true() {
    assert!(can_compile("1 in [1, 2, 3]"));
    let mut env = create_global_env();
    let v = eval_source("1 in [1, 2, 3]", &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn bytecode_in_array_false() {
    assert!(can_compile("9 in [1, 2, 3]"));
    let mut env = create_global_env();
    let v = eval_source("9 in [1, 2, 3]", &mut env).unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn bytecode_in_object_key() {
    assert!(can_compile("let o = { x: 1, y: 2 }\n\"x\" in o"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 1, y: 2 }
        "x" in o
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn bytecode_in_object_missing_key() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 1 }
        "z" in o
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn bytecode_in_string_substring() {
    assert!(can_compile(r#""ab" in "abc""#));
    let mut env = create_global_env();
    let v = eval_source(r#""ab" in "abc""#, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn bytecode_in_with_logical_and() {
    assert!(can_compile("1 in [1, 2] && 3 in [1, 2]"));
    let mut env = create_global_env();
    let v = eval_source("1 in [1, 2] && 3 in [1, 2]", &mut env).unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn bytecode_for_of_loop_still_works() {
    assert!(can_compile("let sum = 0\nfor x of [1, 2, 3] { sum = sum + x }\nsum"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        for x of [1, 2, 3] {
            sum = sum + x
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn bytecode_for_in_yields_indices() {
    assert!(can_compile("let xs = []\nfor i in [10, 20, 30] { push(xs, i) }\nxs"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = []
        for i in [10, 20, 30] {
            xs = push(xs, i)
        }
        xs
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(
        v,
        Value::Array(items) if items.len() == 3
            && matches!(&items[0], Value::Number(0))
            && matches!(&items[2], Value::Number(2))
    ));
}

#[test]
fn bytecode_in_class_field_key() {
    assert!(can_compile(
        "class Box { x: Number\nfn init(v) { this.x = v } }\nlet b = Box(1)\n\"x\" in b"
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Box {
            x: Number
            fn init(v) {
                this.x = v
            }
        }
        let b = Box(1)
        "x" in b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}
