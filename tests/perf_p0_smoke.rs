//! P0 subset — frame-budget smoke (`performance.now` + `game_tick`).

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn frame_budget_smoke_delta_ms_finite() {
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let t0 = performance.now()
        fn noop(dt) {}
        requestAnimationFrame(noop)
        let a = game_tick()
        let b = game_tick()
        let c = game_tick()
        let t1 = performance.now()
        let d = c["delta_ms"]
        // P9: tighter CI budget (was 200ms).
        d == d && typeof(d) == "number" && d < 100 && t1 >= t0 && a["frame"] >= 1
        "#,
        &mut env,
    )
    .expect("frame budget smoke should eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
