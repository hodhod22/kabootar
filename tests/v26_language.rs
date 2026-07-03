//! v2.6 — microtask-kö och schemalagd async

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn async_tasks_scheduled_on_call() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn a() {
            return 1
        }
        async fn b() {
            return 2
        }
        let p1 = a()
        let p2 = b()
        let x = await p1
        let y = await p2
        x + y
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn nested_async_await_order() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn second() {
            return 5
        }
        async fn first() {
            let x = await second()
            return x + 1
        }
        await first()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn shared_promise_identity() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn fetch() {
            return 99
        }
        let p = fetch()
        await p + await p
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(198)));
}

#[test]
fn async_still_works_v24() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn fetch() {
            return 42
        }
        await fetch()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn fire_and_forget_drains_at_end() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn work() {
            return 7
        }
        work()
        1
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}
