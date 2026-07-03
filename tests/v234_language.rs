//! v2.34 — Undefined/NaN literals, spread class constructor, bytecode is()

use kabootar_lib::bytecode::{can_compile, compile_source, deserialize, serialize};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_undefined_and_nan_literals() {
    assert!(can_compile("typeof(undefined)"));
    assert!(can_compile("is_nan(NaN)"));
    let mut env = create_global_env();
    let u = eval_source("typeof(undefined)", &mut env).unwrap();
    assert!(matches!(u, Value::String(s) if s == "undefined"));
    let n = eval_source("is_nan(NaN)", &mut env).unwrap();
    assert!(matches!(n, Value::Bool(true)));
}

#[test]
fn undefined_nan_serialize_roundtrip() {
    let program = compile_source("undefined").unwrap();
    let bc = program.bytecode.as_ref().unwrap();
    assert!(bc.constants.iter().any(|c| *c == kabootar_lib::bytecode::Constant::Undefined));
    let restored = deserialize(&serialize(bc)).unwrap();
    assert_eq!(restored.constants, bc.constants);
}

#[test]
fn bytecode_class_spread_constructor() {
    assert!(can_compile(
        r#"
        class Pair {
            x: number;
            y: number;
            fn init(a, b) {
                self.x = a
                self.y = b
            }
        }
        let args = [3, 4]
        let p = Pair(...args)
        p.x + p.y
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Pair {
            x: number;
            y: number;
            fn init(a, b) {
                self.x = a
                self.y = b
            }
        }
        let args = [3, 4]
        let p = Pair(...args)
        p.x + p.y
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_is_with_class() {
    assert!(can_compile(
        r#"
        class Animal { }
        class Dog extends Animal { }
        let d = Dog()
        instanceof(d, "Dog")
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal { }
        class Dog extends Animal { }
        let d = Dog()
        instanceof(d, "Dog")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn bytecode_is_inherited_class() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal { }
        class Dog extends Animal { }
        let d = Dog()
        instanceof(d, "Animal")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn bytecode_class_field_undefined_default() {
    assert!(can_compile(
        r#"
        class Box {
            tag: string = undefined;
        }
        let b = Box()
        typeof(b.tag)
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Box {
            tag: string = undefined;
        }
        let b = Box()
        typeof(b.tag)
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "undefined"));
}
