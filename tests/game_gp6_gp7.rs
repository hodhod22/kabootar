//! GP6i/GP7f save + scene I/O; GP6b physics3; GP6f/g postfx/light; GP7g editor UX.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn env_host() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    env_host();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn save_scene_roundtrip_and_slots() {
    let v = eval(
        r#"
        import "game/scene"
        import "game/save"
        os_mkdir("/scenes")
        os_mkdir("/saves")
        let root = createNode("world")
        let a = createNode("a")
        a = setLocal(a, 1, 2, 3)
        a = setLayer(a, 4)
        root = addChild(root, a)
        let wr = writeScene("/scenes/t.kscene", root, { "tag": "t" })
        let rd = readScene("/scenes/t.kscene")
        let slot = createSlot("slot1", 1)
        let sv = saveState(slot, { "score": 42 })
        let ld = loadState(sv["slot"])
        wr["ok"] && rd["ok"] && rd["root"]["name"] == "world" && rd["root"]["children"][0]["x"] == 1 && rd["root"]["children"][0]["layer"] == 4 && ld["state"]["score"] == 42
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn physics3_rigidbody_and_constraints() {
    let v = eval(
        r#"
        import "game/physics3"
        let w = createWorld3(-20.0)
        let ground = { "minx": -10.0, "miny": -0.1, "minz": -10.0, "maxx": 10.0, "maxy": 0.0, "maxz": 10.0 }
        w = addStatic(w, ground)
        let b = createRigidBody(0.0, 2.0, 0.0, 1.0)
        b = setCollider(b, createBoxCollider(0.5, 0.5, 0.5))
        w = addBody(w, b)
        let s = createSphereCollider(0.4)
        let b2 = setCollider(createRigidBody(2.0, 3.0, 0.0, 1.0), s)
        w = addBody(w, b2)
        w = addDistanceConstraint(w, 0, 1, 2.0)
        let i = 0
        while i < 30 {
            w = stepWorld(w, 0.016)
            i = i + 1
        }
        let hit = raycastStatics(w, { "x": 0.0, "y": 5.0, "z": 0.0 }, { "x": 0.0, "y": -1.0, "z": 0.0 })
        w["bodies"][0]["y"] < 1.0 && hit["hit"] == true && len(w["constraints"]) == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn postfx_and_light_descriptors() {
    let v = eval(
        r#"
        import "game/postfx"
        import "game/light"
        let p = createPipeline()
        p = addPass(p, tonemapPass(1.5))
        p = addPass(p, bloomPass(0.7, 0.5))
        p = addPass(p, fxaaPass())
        let colors = applyTonemap([[2.0, 2.0, 2.0]], 1.0)
        let bloom = applyBloomThreshold([[1.0, 1.0, 1.0], [0.1, 0.1, 0.1]], 0.5)
        let lights = createLightList()
        lights = addLight(lights, createDirectional({ "x": 0.0, "y": -1.0, "z": 0.0 }, [1.0, 1.0, 1.0], 1.0))
        lights = addLight(lights, enableShadow(createPoint({ "x": 0.0, "y": 2.0, "z": 0.0 }, [1.0, 0.8, 0.6], 1.0, 8.0), 512))
        let contrib = directionalContribution(lights["lights"][0], { "x": 0.0, "y": 1.0, "z": 0.0 })
        let d = describePipeline(p)
        let ld = describeLights(lights)
        d["count"] == 3 && colors[0][0] > 0.6 && bloom[1][0] == 0.0 && contrib["ndotl"] == 1.0 && ld["count"] == 2 && lights["lights"][1]["castShadow"] == true
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn editor_save_undo_multiselect_kos() {
    let v = eval(
        r#"
        import "game/game_editor"
        os_mkdir("/scenes")
        let ed = bootEditor("world")
        let hero = createNode("hero")
        hero = setLocal(hero, 1, 0, 0)
        ed = pushUndo(ed, snapshotRoot(ed))
        ed["root"] = addChild(ed["root"], hero)
        ed = selectNode(ed, hero)
        let crateN = createNode("crate")
        ed["root"] = addChild(ed["root"], crateN)
        ed = selectAdd(ed, crateN)
        ed = play(ed)
        ed = stepGame(ed, 0.1)
        let saved = saveScene(ed, "/scenes/ed.kscene")
        ed = saved["editor"]
        ed = inspectSetLocal(ed, 5, 0, 0)
        ed = undo(ed)
        let sc = handleShortcut(ed, "Ctrl+Y")
        ed = sc["editor"]
        ed = selectNode(ed, ed["root"]["children"][0])
        let kos = kosEditorLayout(ed)
        let gate = deleteGateStatus(ed)
        let names = multiSelectNames(ed)
        ed["scenePath"] == "/scenes/ed.kscene" && len(names) >= 1 && kos["kind"] == "kos-editor" && gate["create"] && gate["play"] && gate["save"] && ed["postfx"] != null && ed["lights"] != null
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
