//! v2.3 — destructuring, spread, klassisk for, try/catch på Result

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn array_destructuring() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [a, b, c] = [1, 2, 3]
        a + b + c
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn object_destructuring_shorthand() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let { name, age } = { name: "Ada", age: 36 }
        name
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Ada"));
}

#[test]
fn object_destructuring_rename() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let { name: n } = { name: "Lin" }
        n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Lin"));
}

#[test]
fn array_spread_literal() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = [1, 2]
        let b = [0, ...a, 3]
        len(b)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn object_spread_literal() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let base = { x: 1, y: 2 }
        let o = { ...base, z: 3 }
        o.z
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn spread_in_function_call() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn sum3(a, b, c) { return a + b + c }
        let xs = [1, 2, 3]
        sum3(...xs)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn classic_for_loop() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        for let i = 0; i < 5; i = i + 1 {
            sum = sum + i
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn try_catch_on_result_err() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        try {
            Err("boom")
        } catch (e) {
            e
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "boom"));
}

#[test]
fn try_catch_on_result_ok() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        try {
            Ok(99)
        } catch (e) {
            0
        }
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(99)));
}

#[test]
fn assign_destructuring() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let pair = [10, 20]
        let x = 0
        let y = 0
        [x, y] = pair
        x + y
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(30)));
}

#[test]
fn array_rest_destructuring() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let [head, ...tail] = [1, 2, 3, 4]
        head + len(tail)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(4)));
}
