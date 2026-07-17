//! v2.26 — bytecode classes: init, methods, inheritance, field defaults

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_class_constructor_and_method() {
    assert!(can_compile(
        r#"
        class Point {
            x: number;
            y: number;
            fn init(a, b) {
                this.x = a
                this.y = b
            }
            fn sum() {
                return this.x + this.y
            }
        }
        let p = Point(3, 4)
        p.sum()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Point {
            x: number;
            y: number;

            fn init(a, b) {
                this.x = a
                this.y = b
            }

            fn sum() {
                return this.x + this.y
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
fn bytecode_class_inheritance() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;

            fn init(n) {
                this.name = n
            }

            fn label() {
                return this.name
            }
        }

        class Dog extends Animal {
            breed: string;

            fn init(n, b) {
                this.name = n
                this.breed = b
            }

            fn label() {
                return this.name + " (" + this.breed + ")"
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
fn bytecode_inherited_field_default() {
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
