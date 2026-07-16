//! v2.22 — bytecode v2.3: spread, destructuring, try/catch, break/continue

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_array_destructuring() {
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
fn bytecode_object_destructuring() {
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
fn bytecode_array_spread() {
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
fn bytecode_object_spread() {
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
fn bytecode_spread_call() {
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
fn bytecode_try_catch_err_and_ok() {
    let mut env = create_global_env();
    let err = eval_source(
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
    assert!(matches!(err, Value::String(s) if s == "boom"));

    let ok = eval_source(
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
    assert!(matches!(ok, Value::Number(99)));
}

#[test]
fn bytecode_try_catch_nested_throw() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn inner() { throw "boom" }
        fn mid() { return inner() }
        try {
            mid()
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
fn bytecode_assign_destructuring() {
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
fn bytecode_array_rest_destructuring() {
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

#[test]
fn bytecode_simple_while() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let i = 0
        while i < 5 { i = i + 1 }
        i
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_while_break_only() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        let i = 0
        while i < 10 {
            i = i + 1
            if i > 5 { break }
            sum = sum + i
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(15)));
}

#[test]
fn bytecode_while_continue_only() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        let i = 0
        while i < 10 {
            i = i + 1
            if i % 2 == 0 { continue }
            sum = sum + i
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(25)));
}

#[test]
fn bytecode_break_and_continue() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let sum = 0
        let i = 0
        while i < 10 {
            i = i + 1
            if i > 5 {
                break
            }
            if i % 2 == 0 {
                continue
            }
            sum = sum + i
        }
        sum
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_and_short_circuit_in_if_condition() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let lxPos = 0
        let lxSrc = ""
        if lxPos < len(lxSrc) && char_at(lxSrc, lxPos) == "/" {
            while lxPos < len(lxSrc) {
                break
            }
        }
        42
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_can_compile_v23_subset() {
    assert!(can_compile("let [a] = [1]"));
    assert!(can_compile("let b = [0, ...[1]]"));
    assert!(can_compile("try { Ok(1) } catch (e) { e }"));
}
