//! v2.27 — bytecode super + interface/implements

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_super_init_in_constructor() {
    assert!(can_compile(
        r#"
        class Animal {
            name: string;
            fn init(n) { this.name = n }
        }
        class Dog extends Animal {
            breed: string;
            fn init(n, b) {
                super.init(n)
                this.breed = b
            }
        }
        let d = Dog("Rex", "lab")
        d.name + " " + d.breed
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;
            fn init(n) { this.name = n }
        }
        class Dog extends Animal {
            breed: string;
            fn init(n, b) {
                super.init(n)
                this.breed = b
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
fn bytecode_super_method_call() {
    assert!(can_compile(
        r#"
        class Animal {
            name: string;
            fn init(n) { this.name = n }
            fn greet() { return "hi " + this.name }
        }
        class Dog extends Animal {
            fn greet() { return super.greet() + "!" }
        }
        let d = Dog("Rex")
        d.greet()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Animal {
            name: string;
            fn init(n) { this.name = n }
            fn greet() { return "hi " + this.name }
        }
        class Dog extends Animal {
            fn greet() { return super.greet() + "!" }
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
fn bytecode_super_outside_method_errors() {
    assert!(can_compile("super.init(1)"));
    let mut env = create_global_env();
    let err = eval_source("super.init(1)", &mut env).unwrap_err();
    assert!(err.contains("super"));
}

#[test]
fn bytecode_interface_implements_success() {
    assert!(can_compile(
        r#"
        interface Greeter { fn greet(); }
        class Person implements Greeter {
            name: string;
            fn init(n) { this.name = n }
            fn greet() { return "hi " + this.name }
        }
        let p = Person("Ada")
        is_impl(p, "Greeter")
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        interface Greeter { fn greet(); }
        class Person implements Greeter {
            name: string;
            fn init(n) { this.name = n }
            fn greet() { return "hi " + this.name }
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
fn bytecode_interface_missing_method_errors() {
    assert!(can_compile(
        r#"
        interface Worker { fn work(); }
        class Broken implements Worker {
            fn play() { return 1 }
        }
    "#
    ));
    let mut env = create_global_env();
    let err = eval_source(
        r#"
        interface Worker { fn work(); }
        class Broken implements Worker {
            fn play() { return 1 }
        }
    "#,
        &mut env,
    )
    .unwrap_err();
    assert!(err.contains("does not implement"));
}

#[test]
fn bytecode_inherited_method_satisfies_interface() {
    assert!(can_compile(
        r#"
        interface Greeter { fn greet(); }
        class Base { fn greet() { return "ok" } }
        class Child extends Base implements Greeter {}
        let c = Child()
        is_impl(c, "Greeter")
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        interface Greeter { fn greet(); }
        class Base { fn greet() { return "ok" } }
        class Child extends Base implements Greeter {}
        let c = Child()
        is_impl(c, "Greeter")
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}
