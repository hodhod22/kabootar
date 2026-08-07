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

/// P2: playPcm accepts Uint8Array as little-endian i16 PCM.
#[test]
fn play_pcm_uint8_array_le_i16() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/audio"
        let bus = createBus("sfx")
        // 4 LE i16 samples as Uint8Array (8 bytes)
        let sab = array_buffer_new(8)
        let u8 = uint8_array_new(sab, 0, 8)
        uint8_array_set(u8, 0, 232)
        uint8_array_set(u8, 1, 3)
        uint8_array_set(u8, 2, 0)
        uint8_array_set(u8, 3, 0)
        uint8_array_set(u8, 4, 24)
        uint8_array_set(u8, 5, 252)
        uint8_array_set(u8, 6, 0)
        uint8_array_set(u8, 7, 16)
        let n = playPcm(bus, u8)
        let viaHelper = playPcm(bus, pcmToUint8([1000, 0, -1000, 4096]))
        n > 0 && viaHelper > 0
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
