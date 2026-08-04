//! GP2a — glTF 2.0 JSON subset (mesh + material + translation animation).

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

fn fixture_gltf() -> String {
    let path = format!(
        "{}/fixtures/game/triangle.gltf",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).expect("read triangle.gltf")
}

#[test]
fn gltf_load_fixture_via_kab() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    env.set("gltf_text".into(), Value::String(fixture_gltf()));
    let v = eval_source(
        r#"
        import "game/gltf"
        let g = loadGltfJson(gltf_text)
        let floatsOk = len(g["floats"]) == 9
        let idxOk = len(g["indices"]) == 3 && g["indices"][0] == 0 && g["indices"][2] == 2
        let colorOk = g["color"][0] == 1.0 && g["color"][1] == 0.25
        let animOk = len(g["animations"]) == 1 && len(g["animations"][0]["translations"]) == 6
        floatsOk && idxOk && colorOk && animOk
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gltf_load_json_native_direct() {
    let g = kabootar_lib::runtime::game::gltf::load_json(&fixture_gltf()).expect("load");
    let Value::Object(o) = g else {
        panic!("expected object");
    };
    let Value::Array(floats) = o.get("floats").expect("floats") else {
        panic!("floats");
    };
    assert_eq!(floats.len(), 9);
    let Value::Array(indices) = o.get("indices").expect("indices") else {
        panic!("indices");
    };
    assert_eq!(indices.len(), 3);
    let Value::Array(color) = o.get("color").expect("color") else {
        panic!("color");
    };
    assert!(matches!(color[0], Value::Float(f) if (f - 1.0).abs() < 1e-6));
    assert!(matches!(color[1], Value::Float(f) if (f - 0.25).abs() < 1e-6));
}
