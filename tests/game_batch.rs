//! GP1f — sprite batch / tilemap quads.

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
fn build_sprite_quads_and_tilemap() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/batch"
        let uv0 = { u0: 0.0, v0: 0.0, u1: 0.5, v1: 1.0 }
        let uv1 = { u0: 0.5, v0: 0.0, u1: 1.0, v1: 1.0 }
        let quads = buildSpriteQuads([
            { x: 0.0, y: 0.0, w: 1.0, h: 1.0, uv: uv0 }
        ])
        let sprites = buildTilemapSprites([[0, 1], [-1, 0]], [uv0, uv1], 2.0, 2.0)
        len(quads["floats"]) == 20 && quads["count"] == 6 && len(sprites) == 3
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[cfg(feature = "gpu")]
#[test]
fn sprite_batch_draws_on_gpu_when_available() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let avail = eval_source(r#"webgl_info()["gpu3d"]"#, &mut env).expect("info");
    if matches!(&avail, Value::String(s) if s == "cpu-fallback") {
        return;
    }
    let path = format!("{}/fixtures/game/px.png", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).expect("px.png");
    env.set(
        "png_bytes".into(),
        Value::Array(bytes.iter().map(|b| Value::Number(*b as i64)).collect()),
    );
    let v = eval_source(
        r#"
        import "game/atlas"
        import "game/batch"
        let img = image_decode_png(png_bytes)
        let atlas = bakeAtlas([{ w: img["width"], h: img["height"], rgba: img["rgba"] }])
        let gl = webgl_create(32, 32)
        gl.lookAt(0, 0, 3, 0, 0, 0, 0, 1, 0)
        gl.uniform4f(0, 1.0, 1.0, 1.0, 1.0)
        let batch = createSpriteBatch(gl, atlas, [
            { x: -0.5, y: -0.5, w: 1.0, h: 1.0, uv: atlas["uvs"][0] }
        ])
        drawSpriteBatch(batch)
        webgl_info()["gpu3d_last"]
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::String(ref s) if s == "wgpu"), "got {v:?}");
}
