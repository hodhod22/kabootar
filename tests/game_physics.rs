//! GP3a — 2D AABB / circle physics subset.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn aabb_overlap_and_resolve() {
    let v = eval(
        r#"
        import "game/physics"
        let a = { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }
        let b = { x: 8.0, y: 0.0, w: 10.0, h: 10.0 }
        let hit = aabbOverlap(a, b)
        let fixed = resolveAabb(a, b)
        hit && fixed["x"] == -2.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn circle_overlap() {
    let v = eval(
        r#"
        import "game/physics"
        let a = { x: 0.0, y: 0.0, r: 5.0 }
        let b = { x: 8.0, y: 0.0, r: 5.0 }
        let c = { x: 20.0, y: 0.0, r: 5.0 }
        circleOverlap(a, b) && circleOverlap(a, c) == false
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
