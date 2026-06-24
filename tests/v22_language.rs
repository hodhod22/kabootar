//! v2.2 language — JS-paritet (minus problematiska delar) + lånade konstruktioner

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn const_binding_and_reassign_error() {
    let mut env = create_global_env();
    eval_source("const PI = 3", &mut env).unwrap();
    let err = eval_source("PI = 4", &mut env).unwrap_err();
    assert!(err.contains("const"));
}

#[test]
fn array_index_read_write() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [10, 20, 30]
        xs[1] = 99
        xs[1]
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(99)));
}

#[test]
fn object_literal_and_member() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let u = { name: "Ada", age: 36 }
        u.name
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Ada"));
}

#[test]
fn for_of_over_array() {
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
fn template_literal_interpolation() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let name = "Kabootar"
        `Hej ${name}!`
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Hej Kabootar!"));
}

#[test]
fn ternary_and_unary_not() {
    let mut env = create_global_env();
    let v = eval_source("true ? 1 : 2", &mut env).unwrap();
    assert!(matches!(v, Value::Number(1)));
    let v = eval_source("!false", &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn modulo_operator() {
    let mut env = create_global_env();
    let v = eval_source("10 % 3", &mut env).unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn map_filter_and_length() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let xs = [1, 2, 3, 4]
        fn double(x) { return x * 2 }
        fn gt4(x) { return x > 4 }
        let ys = map(xs, double)
        let zs = filter(ys, gt4)
        zs.length
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn line_comments_ignored() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        // comment
        let x = 1 // trailing
        x
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn match_option_and_result() {
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
