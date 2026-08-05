//! GP2c — game audio bus over virtual PCM device.

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
fn play_tone_writes_pcm() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/audio"
        let bus = createBus("sfx")
        bus = setBusVolume(bus, 0.5)
        let n = playTone(bus, 440, 5)
        n > 0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
