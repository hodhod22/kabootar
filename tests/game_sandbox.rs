//! Spelbyggare / sandlåda — Play + Edit + Learn (GP ∩ editor ∩ STEM).

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
fn sandbox_play_edit_learn() {
    let v = eval(
        r#"
        import "science"
        import "science/mechanics"
        import "game/sandbox"
        os_mkdir("/sandbox")
        os_mkdir("/scenes")
        let world = createPuzzle(defaultParams())
        world = autoSolveTowardGoal(world, 300)
        let playOk = isCleared(world) && world["lesson"]["title"] != null
        let ed = puzzleToEditor(world)
        ed = editorPlaceBody(ed, "ball", 55.0, 88.0)
        ed = editorPlaceBody(ed, "goal", 190.0, 75.0)
        let saved = savePuzzleScene(ed, "/scenes/force_puzzle.kscene")
        let loaded = loadPuzzleScene("/scenes/force_puzzle.kscene", defaultParams())
        let editOk = saved["result"]["ok"] && loaded["ok"] && loaded["world"]["body"]["x"] == 55.0
        world = loaded["world"]
        world = applyLiveParams(world, { "k": 48.0, "m": 1.2, "g": 11.0 })
        world = attachParamsWatch(world, "/sandbox/params.json")
        let p0 = readParamsFile("/sandbox/params.json")
        os_write("/sandbox/params.json", json_stringify({ "k": 70.0, "m": 1.2, "g": 11.0, "drag": 0.2, "rest": 0.0 }))
        world = applyLiveParams(world, readParamsFile("/sandbox/params.json"))
        ed = stampLearnReload(ed, "/sandbox/params.json")
        let lesson = buildLesson(world)
        let f = explainForces(world["params"], world["body"], world["anchor"])
        let learnOk = p0["k"] == 48.0 && world["params"]["k"] == 70.0 && lesson["code"] != null && f["total"]["data"][0] != null && ed["hotReload"]["learnPath"] == "/sandbox/params.json"
        playOk && editOk && learnOk
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sandbox_host_params_hot_reload_watch() {
    let dir = std::env::temp_dir();
    let path = dir.join("kab_sandbox_host_params.json");
    let path_s = path.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &path,
        r#"{"k":40.0,"m":1.0,"g":9.81,"drag":0.15,"rest":0.0}"#,
    )
    .expect("write params");
    let v = eval(&format!(
        r#"
        import "game/sandbox"
        let w = enableParamsHotReload("{path_s}")
        w["ok"] == true && w["path"] == "{path_s}"
        "#
    ));
    let _ = std::fs::remove_file(&path);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sandbox_session_canvas_modes_levels() {
    let v = eval(
        r#"
        import "science"
        import "game/sandbox"
        os_mkdir("/sandbox")
        os_mkdir("/scenes")
        let pack = defaultLevelPack()
        saveLevelPack("/sandbox/levels.json", pack)
        let lp = loadLevelPack("/sandbox/levels.json")
        let session = createSession(240, 160, lp["pack"])
        session = runSessionFrames(session, 8, [
            { "type": "down", "x": 170.0, "y": 80.0 },
            { "type": "move", "x": 180.0, "y": 75.0 },
            { "type": "up", "x": 180.0, "y": 75.0 }
        ])
        let canvasOk = session["frames"] >= 8
        session = enterEdit(session)
        session["editor"] = editorPlaceBody(session["editor"], "ball", 42.0, 85.0)
        session = applyEditorAndPlay(session)
        let editOk = session["mode"] == "play" && session["world"]["body"]["x"] == 42.0
        session["world"] = clearLevel(session["world"])
        session = enterLearnMode(session)
        let k0 = session["world"]["params"]["k"]
        session = setLearnParam(session, "k", 10.0)
        let learnOk = session["learnUi"]["k"] == k0 + 10.0 && session["learnUi"]["formula"] != null
        session = nextLevel(session)
        let levelOk = session["world"]["levelId"] == "spring_gap" && len(session["world"]["walls"]) == 2
        canvasOk && editOk && learnOk && levelOk && lp["ok"]
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn mechanics_spring_and_integrate() {
    let v = eval(
        r#"
        import "science"
        import "science/mechanics"
        let p = defaultParams()
        let s = springForce(p["k"], 10.0, 0.0, 0.0, 0.0)
        let body = { "x": 0.0, "y": 0.0, "vx": 0.0, "vy": 0.0, "m": 1.0 }
        body = integrateBody(body, 0.0, p["m"] * p["g"], 0.1)
        s["fx"] < 0.0 && body["vy"] > 0.0 && body["y"] > 0.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
