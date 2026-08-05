//! GP4d — debug draw gizmos on canvas2d.

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

#[test]
fn debug_draw_aabb_and_line() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r##"
        import "game/debug"
        let c = canvas_create(64, 64)
        c.strokeStyle = "#ffffff"
        drawLine2d(c, 0, 0, 10, 10)
        drawAabb(c, { x: 5, y: 5, w: 20, h: 12 })
        drawCircleApprox(c, 32, 32, 8, 8)
        true
        "##,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
