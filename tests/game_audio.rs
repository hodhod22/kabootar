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

#[test]
fn audio_spatial_group_duck_stream() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/audio"
        let bus = createBus("sfx")
        let g = createGroup("master")
        g = addBusToGroup(g, bus)
        g = setGroupVolume(g, 0.8)
        bus = g["buses"][0]
        let listener = createListener(0.0, 0.0, 0.0)
        let near = playSpatial(bus, makeTone(440, 5, 8000), 0.5, 0.0, 0.0, listener)
        let far = playSpatial(bus, makeTone(440, 5, 8000), 20.0, 0.0, 0.0, listener)
        bus = duck(bus, 0.1, 10)
        let ducked = bus["volume"]
        bus = unduck(bus)
        let st = createStream(bus, [makeTone(220, 3, 8000), makeTone(330, 3, 8000)])
        let p1 = streamPump(st)
        st = p1["stream"]
        let p2 = streamPump(st)
        near["gain"] > far["gain"] && ducked == 0.1 && bus["volume"] == 0.8 && p1["written"] > 0 && p2["done"] == true
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
