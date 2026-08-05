//! GP4b/GP7 — editor hierarchy + inspector + scene/game view + DnD + GP6 ui/anim/particles.

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
fn editor_hierarchy_and_inspector() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/scene"
        import "game/editor"
        let root = createNode("root")
        root = setLocal(root, 10, 0, 0)
        let child = createNode("child")
        child = setLocal(child, 1, 2, 3)
        root = addChild(root, child)
        let ed = createEditor(root)
        ed = refresh(ed)
        ed = selectNode(ed, root["children"][0])
        ed = inspectSetLocal(ed, 5, 6, 7)
        len(ed["hierarchy"]) == 2 && ed["hierarchy"][1]["name"] == "child" && ed["inspector"]["x"] == 5 && ed["inspector"]["world"]["x"] == 15
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn editor_mvp_scene_game_dnd_live() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/game_editor"
        let ed = bootEditor("world")
        ed = sceneOrbit(ed, 1.0, -2.0)
        ed = sceneZoom(ed, 0.5)
        ed = play(ed)
        ed = stepGame(ed, 0.016)
        ed = dragStart(ed, { "kind": "asset", "name": "crate", "x": 3, "y": 0, "z": 0 })
        ed = dragDropOnNode(ed, ed["root"])
        ed = liveSet(ed, "layer", 2)
        let lay = editorLayout(ed)
        ed["sceneView"]["camX"] == 1.0 && ed["gameView"]["playing"] == true && ed["gameView"]["time"] > 0.0 && lay["sceneView"]["mode"] == "scene" && ed["selected"]["name"] == "crate" && ed["selected"]["layer"] == 2 && ed["toolbarUi"] != null
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gp6_ui_anim_particles() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/ui"
        import "game/anim"
        import "game/particles"
        let p = createPanel("hud", 0, 0, 200, 40)
        p = addWidget(p, createButton("a", "A", 0, 0, 40, 20))
        p = layoutRow(p, 4)
        let clip = createClip("move", [{ "t": 0.0, "x": 0, "y": 0, "z": 0 }, { "t": 1.0, "x": 10, "y": 0, "z": 0 }])
        let s = sampleClip(clip, 0.5)
        let em = createEmitter(0, 0, 0, 10)
        em = emitBurst(em, 2)
        em = stepEmitter(em, 0.1)
        p["children"][0]["x"] == 0 && s["x"] == 5 && len(em["particles"]) >= 2
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
