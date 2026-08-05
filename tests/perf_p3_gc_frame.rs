//! P3 — frame-aware GC budget smoke.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn gc_frame_stats_reset_on_tick() {
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn noop(dt) {}
        fn busy(dt) {
            let i = 0
            while i < 40 {
                let _ = { n: i }
                i = i + 1
            }
        }
        requestAnimationFrame(busy)
        game_tick()
        let s = gc_frame_stats()
        typeof(s["allocs"]) == "number" && typeof(s["budget"]) == "number" && s["budget"] > 0
        "#,
        &mut env,
    )
    .expect("gc frame stats");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gc_set_frame_budget_native() {
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        gc_set_frame_budget(100)
        let s = gc_frame_stats()
        s["budget"] == 100
        "#,
        &mut env,
    )
    .expect("set budget");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
