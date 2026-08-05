//! GP4c — profiler frame samples.

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
fn profiler_samples_and_overlay() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/profiler"
        let p = createProfiler(8)
        let i = 0
        while i < 3 {
            p = beginFrame(p)
            p = endFrame(p)
            i = i + 1
        }
        let s = sample(p)
        let c = canvas_create(128, 64)
        drawOverlay(c, s)
        s["count"] == 3 && s["avg"] >= 0 && s["max"] >= s["avg"]
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
