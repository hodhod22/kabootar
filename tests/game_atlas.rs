//! GP2b — PNG decode + row atlas bake.

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

fn fixture_png_bytes() -> Vec<u8> {
    let path = format!("{}/fixtures/game/px.png", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).expect("read px.png")
}

fn bytes_to_kab_array(bytes: &[u8]) -> Value {
    Value::Array(bytes.iter().map(|b| Value::Number(*b as i64)).collect())
}

#[test]
fn image_decode_png_2x2() {
    let decoded = kabootar_lib::runtime::game::image_png::decode_png(&fixture_png_bytes())
        .expect("decode");
    let Value::Object(o) = decoded else {
        panic!("expected object");
    };
    assert!(matches!(o.get("width"), Some(Value::Number(2))));
    assert!(matches!(o.get("height"), Some(Value::Number(2))));
    let Value::Array(rgba) = o.get("rgba").expect("rgba") else {
        panic!("rgba");
    };
    assert_eq!(rgba.len(), 16);
    // Top-left pixel red
    assert!(matches!(rgba[0], Value::Number(255)));
    assert!(matches!(rgba[1], Value::Number(0)));
    assert!(matches!(rgba[2], Value::Number(0)));
}

#[test]
fn atlas_bake_row_from_png() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    env.set("png_bytes".into(), bytes_to_kab_array(&fixture_png_bytes()));
    let v = eval_source(
        r#"
        import "game/atlas"
        let img = image_decode_png(png_bytes)
        let atlas = bakeAtlas([
            { w: img["width"], h: img["height"], rgba: img["rgba"] },
            { w: img["width"], h: img["height"], rgba: img["rgba"] }
        ])
        let sizeOk = atlas["width"] == 4 && atlas["height"] == 2
        let uv0 = atlas["uvs"][0]
        let uv1 = atlas["uvs"][1]
        let uvOk = uv0["u0"] == 0.0 && uv0["u1"] == 0.5 && uv1["u0"] == 0.5 && uv1["u1"] == 1.0
        let rgbaOk = len(atlas["rgba"]) == 4 * 2 * 4
        sizeOk && uvOk && rgbaOk
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
