//! v2.49 — assignment to super.member

use kabootar_lib::bytecode::can_compile;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn bytecode_super_member_assign_compiles() {
    assert!(can_compile(
        r#"
        class Base { count: number; fn init() { this.count = 0 } }
        class Child extends Base {
            fn init() {
                super.init()
                super.count = 1
            }
        }
        Child().count
    "#
    ));
}

#[test]
fn bytecode_super_member_assign() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { count: number; fn init() { this.count = 0 } }
        class Child extends Base {
            fn init() {
                super.init()
                super.count = 1
            }
        }
        Child().count
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_super_member_plus_assign() {
    assert!(can_compile(
        r#"
        class Base { n: number; fn init() { this.n = 0 } }
        class Child extends Base {
            fn init() {
                super.init()
                super.n += 2
            }
        }
        Child().n
    "#
    ));
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { n: number; fn init() { this.n = 0 } }
        class Child extends Base {
            fn init() {
                super.init()
                super.n += 2
            }
        }
        Child().n
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn bytecode_super_member_assign_returns_value() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { x: number; fn init() { this.x = 0 } }
        class Child extends Base {
            fn set() { super.x = 9 }
        }
        Child().set()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(9)));
}

#[test]
fn bytecode_super_member_read_field() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { label: string; fn init() { this.label = "base" } }
        class Child extends Base {
            fn tag() { return super.label }
        }
        Child().tag()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "base"));
}

#[test]
fn bytecode_super_member_method_still_bound() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base { fn f() { return 1 } }
        class Child extends Base {
            fn g() { return super.f }
        }
        let c = Child()
        let m = c.g()
        m()
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(1)));
}

#[test]
fn bytecode_super_member_assign_after_parent_init() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        class Base {
            a: number;
            b: number;
            fn init() {
                this.a = 1
                this.b = 2
            }
        }
        class Child extends Base {
            fn init() {
                super.init()
                super.b = 20
            }
        }
        let c = Child()
        c.a + c.b
    "#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(21)));
}

#[test]
fn bytecode_super_member_assign_outside_method_errors() {
    assert!(can_compile("super.x = 1"));
    let mut env = create_global_env();
    let err = eval_source("super.x = 1", &mut env).unwrap_err();
    assert!(err.contains("super") || err.contains("this"));
}
