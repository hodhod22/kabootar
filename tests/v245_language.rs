//! v2.45 — super.method as bound method value

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_super_method_value_compiles() {
    assert!(can_compile(
        r#"
        class Base { fn f() { return 1 } }
        class Child extends Base {
            fn g() { return super.f }
        }
        let c = Child()
        c.g()
    "#
    ));
}

#[test]
fn bytecode_super_method_value_returns_bound_method() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base {
            n: number;
            fn init(v) { self.n = v }
            fn get() { return self.n }
        }
        class Child extends Base {
            fn capture() { return super.get }
        }
        let c = Child(7)
        let m = c.capture()
        m()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));
}

#[test]
fn bytecode_super_method_value_call_later() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { fn f() { return "base" } }
        class Child extends Base {
            fn stash() {
                let m = super.f
                return m()
            }
        }
        let c = Child()
        c.stash()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "base"));
}

#[test]
fn bytecode_super_method_value_as_callback() {
    assert!(can_compile(
        r#"
        class Base { fn f() { return 2 } }
        class Child extends Base {
            fn run(cb) { return cb() }
            fn via() { return self.run(super.f) }
        }
        let c = Child()
        c.via()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { fn f() { return 2 } }
        class Child extends Base {
            fn run(cb) { return cb() }
            fn via() { return self.run(super.f) }
        }
        let c = Child()
        c.via()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_super_method_value_parenthesized_call() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { fn f(x) { return x + 1 } }
        class Child extends Base {
            fn call() { return (super.f)(4) }
        }
        let c = Child()
        c.call()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(5)));
}

#[test]
fn bytecode_super_method_value_overridden_chain() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class A { fn tag() { return "A" } }
        class B extends A { fn tag() { return "B" } }
        class C extends B {
            fn parentTag() { return super.tag() }
            fn boundParent() {
                let m = super.tag
                return m()
            }
        }
        let c = C()
        c.parentTag() + c.boundParent()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "BB"));
}

#[test]
fn bytecode_super_method_value_init_reference() {
    assert!(can_compile(
        r#"
        class Base { fn init() {} }
        class Child extends Base {
            fn setup() {
                let parentInit = super.init
                parentInit()
            }
        }
        Child().setup()
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base {
            n: number;
            fn init() { self.n = 9 }
        }
        class Child extends Base {
            fn setup() {
                let parentInit = super.init
                parentInit()
                return self.n
            }
        }
        Child().setup()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_super_method_value_still_errors_outside_method() {
    assert!(can_compile("super.f"));
    let mut env = create_global_env();
    let err = eval_source("super.f", &mut env).unwrap_err();
    assert!(err.contains("super"));
}
