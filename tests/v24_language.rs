//! v2.4 — pilfunktioner, async/await, klass-arv, konstruktor

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn arrow_function_expression() {
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
fn arrow_function_block_body() {
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
fn arrow_in_map() {
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
fn async_fn_and_await() {
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
fn async_arrow_and_await() {
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
fn class_constructor_init() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Point {
            x: number;
            y: number;

            fn init(a, b) {
                self.x = a
                self.y = b
            }

            fn sum() {
                return self.x + self.y
            }
        }
        let p = Point(3, 4)
        p.sum()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn class_inheritance_extends() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;

            fn init(n) {
                self.name = n
            }

            fn label() {
                return self.name
            }
        }

        class Dog extends Animal {
            breed: string;

            fn init(n, b) {
                self.name = n
                self.breed = b
            }

            fn label() {
                return self.name + " (" + self.breed + ")"
            }
        }

        let d = Dog("Rex", "lab")
        d.label()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Rex (lab)"));
}

#[test]
fn inherited_field_exists() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base {
            value: number = 10;
        }
        class Child extends Base {}
        let c = Child()
        c.value
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn await_non_promise_passthrough() {
    let mut env = create_global_env();
    let v = eval_source("await 5", &mut env).unwrap();
    assert!(matches!(v, Value::Number(5)));
}
