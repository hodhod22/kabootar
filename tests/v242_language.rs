//! v2.42 — pub let with destructuring exports

use kabootar_lib::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_pub_let_array_destructure() {
    assert!(can_compile("pub let [a, b] = [1, 2]\na + b"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        pub let [a, b] = [1, 2]
        a + b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
    assert!(env.is_exported("a"));
    assert!(env.is_exported("b"));
}

#[test]
fn bytecode_pub_let_object_shorthand() {
    assert!(can_compile("pub let { x, y } = { x: 10, y: 20 }\nx + y"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        pub let { x, y } = { x: 10, y: 20 }
        x + y
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(30)));
    assert!(env.is_exported("x"));
    assert!(env.is_exported("y"));
}

#[test]
fn bytecode_pub_let_object_field_rename() {
    assert!(can_compile("pub let { a: n } = { a: 7 }\nn"));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        pub let { a: n } = { a: 7 }
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
    assert!(env.is_exported("n"));
    assert!(!env.is_exported("a"));
}

#[test]
fn bytecode_pub_let_array_rest_anonymous_exports_head_only() {
    assert!(can_compile("pub let [head, ...] = [1, 2, 3]\nhead"));
    let mut env = create_global_env();
    eval_source(
        r#"
        pub let [head, ...] = [1, 2, 3]
        head
    "#,
        &mut env,
    )
    .unwrap();
    assert!(env.is_exported("head"));
}

#[test]
fn bytecode_pub_let_object_rest_anonymous_exports_bound_only() {
    let mut env = create_global_env();
    eval_source(
        r#"
        pub let { a, ... } = { a: 1, b: 2 }
        a
    "#,
        &mut env,
    )
    .unwrap();
    assert!(env.is_exported("a"));
}

#[test]
fn bytecode_pub_let_destructure_exports_serialize() {
    let program = compile_source(
        r#"
        pub let [x, y] = [1, 2]
        pub let { a: n } = { a: 3 }
        x + y + n
    "#,
    )
    .unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    assert!(bc.exports.contains(&"x".to_string()));
    assert!(bc.exports.contains(&"y".to_string()));
    assert!(bc.exports.contains(&"n".to_string()));
    let restored = deserialize(&serialize(bc)).unwrap();
    assert_eq!(restored.exports, bc.exports);
}

#[test]
fn bytecode_pub_let_nested_pattern_exports() {
    let mut env = create_global_env();
    eval_source(
        r#"
        pub let { pair: [u, v] } = { pair: [4, 5] }
        u + v
    "#,
        &mut env,
    )
    .unwrap();
    assert!(env.is_exported("u"));
    assert!(env.is_exported("v"));
}

#[test]
fn bytecode_pub_let_name_still_works() {
    assert!(can_compile("pub let x = 42\nx"));
    let mut env = create_global_env();
    eval_source("pub let x = 42", &mut env).unwrap();
    assert!(env.is_exported("x"));
}
