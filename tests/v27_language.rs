//! v2.7 — sleep_ticks och interface/implements

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn sleep_ticks_yields_to_other_tasks() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn slow() {
            await sleep_ticks(1)
            return 10
        }
        async fn fast() {
            return 1
        }
        let p1 = slow()
        let p2 = fast()
        await p1 + await p2
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(11)));
}

#[test]
fn sleep_ticks_zero_resolves_quickly() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        async fn work() {
            await sleep_ticks(0)
            return 7
        }
        await work()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn interface_implements_success() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        interface Greeter {
            fn greet();
        }

        class Person implements Greeter {
            name: string;

            fn init(n) {
                this.name = n
            }

            fn greet() {
                return "hi " + this.name
            }
        }

        let p = Person("Ada")
        is_impl(p, "Greeter")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn trait_implements_success_g5() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        trait Greeter {
            fn greet();
        }

        class Person implements Greeter {
            name: string;

            fn init(n) {
                this.name = n
            }

            fn greet() {
                return "hi " + this.name
            }
        }

        let p = Person("Ada")
        is_impl(p, "Greeter")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn interface_missing_method_errors() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        interface Worker {
            fn work();
        }

        class Broken implements Worker {
            fn play() {
                return 1
            }
        }
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("does not implement"));
}

#[test]
fn inherited_method_satisfies_interface() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        interface Greeter {
            fn greet();
        }

        class Base {
            fn greet() {
                return "ok"
            }
        }

        class Child extends Base implements Greeter {}

        let c = Child()
        is_impl(c, "Greeter")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}
