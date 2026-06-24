//! v2.25 — bytecode async/await: async fn, async arrows, await

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_async_fn_and_await() {
    assert!(can_compile(
        r#"
        async fn fetch() {
            return 42
        }
        let p = fetch()
        await p
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn fetch() {
            return 42
        }
        let p = fetch()
        await p
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn bytecode_async_arrow_and_await() {
    assert!(can_compile(
        r#"
        let f = async (n) => n + 1
        await f(9)
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let f = async (n) => n + 1
        await f(9)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn bytecode_await_non_promise_passthrough() {
    let mut env = create_global_env();
    let v = eval_source("await 5", &mut env).unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_async_fn_return_await() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn inner() {
            return 7
        }
        async fn outer() {
            return await inner()
        }
        await outer()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}
