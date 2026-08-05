//! Playable 2D mini-demo smoke.

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
fn game_playable_2d_runs_ticks() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let src = include_str!("../examples/game_playable_2d.kab");
    let mut env = create_global_env();
    let v = eval_source(src, &mut env).expect("playable demo");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
