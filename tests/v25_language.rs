//! v2.5 — super i arvade klasser

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn super_init_in_constructor() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;

            fn init(n) {
                self.name = n
            }
        }

        class Dog extends Animal {
            breed: string;

            fn init(n, b) {
                super.init(n)
                self.breed = b
            }
        }

        let d = Dog("Rex", "lab")
        d.name + " " + d.breed
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Rex lab"));
}

#[test]
fn super_method_call() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;

            fn init(n) {
                self.name = n
            }

            fn greet() {
                return "hi " + self.name
            }
        }

        class Dog extends Animal {
            fn greet() {
                return super.greet() + "!"
            }
        }

        let d = Dog("Rex")
        d.greet()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "hi Rex!"));
}

#[test]
fn super_outside_method_errors() {
    let mut env = create_global_env();
    let err = eval_source("super.init(1)", &mut env).unwrap_err();
    assert!(err.contains("super"));
}
