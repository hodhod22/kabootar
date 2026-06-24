//! Rust wave: enum, field types, if let, iterator pack.

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::format_value;

fn evaluate(code: &str) -> String {
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn user_enum_unit_and_payload() {
    let code = r#"
        enum Color { Red, Green, Blue }
        enum Msg { Quit, Move(x, y), Write(text) }
        let a = Color.Red
        let b = Msg.Move(3, 4)
        match a {
            Color.Red => 1,
            Color.Green => 2,
            _ => 0
        }
    "#;
    assert_eq!(evaluate(code), "1");
    assert_eq!(
        evaluate(
            r#"
            enum Msg { Quit, Move(x, y) }
            match Msg.Move(10, 5) {
                Msg.Move(x, y) => x + y,
                Msg.Quit => 0
            }
        "#
        ),
        "15"
    );
}

#[test]
fn if_let_option_and_result() {
    assert_eq!(
        evaluate(
            r#"
            if let Some(x) = Some(7) { x } else { 0 }
        "#
        ),
        "7"
    );
    assert_eq!(
        evaluate(
            r#"
            if let Ok(v) = Ok(42) { v } else { -1 }
        "#
        ),
        "42"
    );
}

#[test]
fn class_field_type_check() {
    let err = eval_source(
        r#"
        class User {
            age: number;
            fn init(a) { self.age = a }
        }
        User("x")
    "#,
        &mut create_global_env(),
    )
    .unwrap_err();
    assert!(err.contains("expected number"));

    assert_eq!(
        evaluate(
            r#"
            class User {
                age: number;
                fn init(a) { self.age = a }
            }
            let u = User(21)
            u.age
        "#
        ),
        "21"
    );
}

#[test]
fn iterator_pack_helpers() {
    assert_eq!(
        evaluate(r#"len(iterator_take([1, 2, 3, 4], 2))"#),
        "2"
    );
    assert_eq!(
        evaluate(r#"iterator_skip([1, 2, 3], 1)"#),
        "[2, 3]"
    );
    assert_eq!(
        evaluate(r#"iterator_zip([1, 2], [3, 4])"#),
        "[[1, 3], [2, 4]]"
    );
    assert_eq!(
        evaluate(r#"iterator_enumerate(["a", "b"])"#),
        "[[0, a], [1, b]]"
    );
}
