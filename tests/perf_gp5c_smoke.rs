//! GP5c — performance budget smoke (frame timing + coarse mem).

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn gp5c_idle_frame_budget() {
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn noop(dt) {}
        // Warm-up so compile/setup time is not counted in delta_ms.
        requestAnimationFrame(noop)
        game_tick()
        let mem0 = os_mem_stats()
        let t0 = performance.now()
        let n = 0
        let sum = 0.0
        while n < 8 {
            requestAnimationFrame(noop)
            let f = game_tick()
            sum = sum + f["delta_ms"]
            n = n + 1
        }
        let t1 = performance.now()
        let mem1 = os_mem_stats()
        let avg = sum / 8.0
        // CI-loose budget: avg frame delta under 50ms after warm-up.
        // os_mem_stats → [regions, used, limit]
        avg < 50.0 && t1 >= t0 && mem1[1] >= 0 && mem0[1] >= 0
        "#,
        &mut env,
    )
    .expect("gp5c smoke");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
