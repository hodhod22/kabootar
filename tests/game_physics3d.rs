//! GP3b — ray-AABB + character step.

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
fn ray_aabb_hit_and_miss() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/physics"
        let box = { minx: -1.0, miny: -1.0, minz: -1.0, maxx: 1.0, maxy: 1.0, maxz: 1.0 }
        let hit = rayAabb({ x: 0.0, y: 0.0, z: -5.0 }, { x: 0.0, y: 0.0, z: 1.0 }, box)
        let miss = rayAabb({ x: 10.0, y: 0.0, z: -5.0 }, { x: 0.0, y: 0.0, z: 1.0 }, box)
        hit["hit"] && hit["t"] == 4.0 && miss["hit"] == false
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn character_step_grounds() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/physics"
        let ground = [{ minx: -10.0, miny: 0.0, minz: -10.0, maxx: 10.0, maxy: 0.1, maxz: 10.0 }]
        let c = { x: 0.0, y: 2.0, z: 0.0, radius: 0.3, height: 5.0, vy: 0.0 }
        let n = characterStep(c, 0.0, 0.0, 0.1, -20.0, ground)
        n["grounded"] == true && n["y"] < 0.2
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn character_drive_wish_and_transform_sync() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/physics"
        let ground = [{ minx: -10.0, miny: 0.0, minz: -10.0, maxx: 10.0, maxy: 0.1, maxz: 10.0 }]
        let c = createPhysicsCharacter(0.0, 0.2, 0.0, 0.3, 5.0)
        c = characterDrive(c, 1.0, 0.0, 10.0, 0.1, -20.0, ground, false)
        let t = { "x": 0.0, "y": 0.0, "z": 0.0 }
        t = syncTransformFromCharacter(t, c)
        c["grounded"] == true && t["x"] > 0.0 && t["x"] == c["x"] && t["z"] == c["z"]
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
