//! GP1 subset — lib/game scene / input / time (+ render smoke).

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // Host compiler + host bytecode VM so `pub` exports are marked
        // (Kab VM path currently skips export writeback for small .kbc).
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn game_scene_create_and_world_pos() {
    let v = eval(
        r#"
        import "game/scene"
        let root = createNode("root")
        root = setLocal(root, 10, 20, 30)
        let child = createNode("child")
        child = setLocal(child, 1, 2, 3)
        root = addChild(root, child)
        let w = worldPos(root)
        w["x"] == 10 && w["y"] == 20 && w["z"] == 30 && len(root["children"]) == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn game_input_action_pressed() {
    let v = eval(
        r#"
        import "game/input"
        let actions = createActions({ jump: ["Space"], left: ["ArrowLeft", "KeyA"] })
        input_key_down("Space")
        let jumpOk = actionPressed(actions, "jump")
        let leftOk = actionPressed(actions, "left")
        jumpOk && leftOk == false
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn game_time_fixed_tick() {
    let v = eval(
        r#"
        import "game/time"
        let state = createFixed(0.05)
        let hits = 0
        fn onFixed(step) { hits = hits + 1 }
        let n = fixedTick(state, dtSec(120), onFixed)
        n == 2 && hits == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn game_render_mesh_draw() {
    let v = eval(
        r#"
        import "game/render"
        let gl = webgl_create(32, 32)
        let mesh = createMesh(gl, [-0.5, -0.5, 0.5, -0.5, 0.0, 0.5])
        setColor(gl, 1.0, 0.0, 0.0, 1.0)
        drawMesh(mesh)
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
