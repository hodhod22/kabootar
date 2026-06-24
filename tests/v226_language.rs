//! v2.26 — bytecode classes: init, methods, inheritance, field defaults

use kabootar::bytecode::can_compile;
use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

#[test]
fn bytecode_class_constructor_and_method() {
    assert!(can_compile(
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
    "#
    ));
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
fn bytecode_class_inheritance() {
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
