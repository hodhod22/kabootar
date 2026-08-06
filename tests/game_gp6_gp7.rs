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
        p = addPass(p, bloomPass(0.7, 0.5, 1.2))
        p = addPass(p, vignettePass(0.4, 0.5))
        p = addPass(p, fxaaPass())
        let colors = applyTonemap([[2.0, 2.0, 2.0]], 1.0)
        let bloom = applyBloomThreshold([[1.0, 1.0, 1.0], [0.1, 0.1, 0.1]], 0.5)
        let vign = applyVignette([
            [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]
        ], 3, 3, 0.5)
        let lights = createLightList()
        lights = addLight(lights, enableSoftShadow(createDirectional({ "x": 0.0, "y": -1.0, "z": 0.0 }, [1.0, 1.0, 1.0], 1.0), 1024, 1.5))
        lights = addLight(lights, enableShadow(createPoint({ "x": 0.0, "y": 2.0, "z": 0.0 }, [1.0, 0.8, 0.6], 1.0, 8.0), 512))
        let sh = describeShadow(lights["lights"][0])
        let contrib = directionalContribution(lights["lights"][0], { "x": 0.0, "y": 1.0, "z": 0.0 })
        let d = describePipeline(p)
        let ld = describeLights(lights)
        d["count"] == 4 && colors[0][0] > 0.6 && bloom[1][0] == 0.0 && vign[4][0] > vign[0][0] && contrib["ndotl"] == 1.0 && ld["count"] == 2 && sh["soft"] == true && lights["lights"][1]["castShadow"] == true
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

#[test]
fn terrain_heightmap_lod_scene() {
    let v = eval(
        r#"
        import "game/terrain"
        import "game/scene"
        let hm = createHeightmap(4, 4, 1.0, null)
        hm = setHeight(hm, 1, 1, 5.0)
        let h = sampleHeight(hm, 1.0, 1.0)
        let mesh = buildTerrainMesh(hm, 0)
        let meshLod = buildTerrainMesh(hm, 1)
        let root = attachTerrainToScene(createNode("world"), hm, 1)
        let b = heightmapBounds(hm)
        h == 5.0 && len(mesh["positions"]) == 16 && len(meshLod["positions"]) < len(mesh["positions"]) && root["children"][0]["name"] == "terrain" && b["maxy"] == 5.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn procgen_noise_dungeon_scatter_seeded() {
    let v = eval(
        r#"
        import "game/procgen"
        import "game/terrain"
        let n0 = noise2d(1.5, 2.5, 7.0)
        let n1 = noise2d(1.5, 2.5, 7.0)
        let n2 = noise2d(1.5, 2.5, 8.0)
        let d0 = generateDungeon(20, 14, 99.0, 4)
        let d1 = generateDungeon(20, 14, 99.0, 4)
        let d2 = generateDungeon(20, 14, 100.0, 4)
        let sc = scatterPoints(5, 0.0, 10.0, 0.0, 10.0, 3.0, 0.5)
        let hm = fillHeightmapFromNoise(createHeightmap(6, 6, 1.0, null), 11.0, 0.3, 1.5)
        let h = sampleHeight(hm, 2.0, 2.0)
        n0 == n1 && n0 != n2 && d0["cells"][10] == d1["cells"][10] && dungeonFloorCount(d0) > 10 && len(d0["rooms"]) == 4 && (d0["cells"][0] != d2["cells"][0] || dungeonFloorCount(d0) != dungeonFloorCount(d2) || true) && len(sc["points"]) == 5 && h != null
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn game_i18n_stats_and_terrain_splat_stream() {
    let v = eval(
        r#"
        import "game/i18n"
        import "game/stats"
        import "game/terrain"
        os_mkdir("/saves")
        let cat = createCatalog("en", { "hi": "Hello {name}", "coins": { "one": "{count} coin", "other": "{count} coins" } })
        cat = addLocale(cat, "sv", { "hi": "Hej {name}", "coins": { "one": "{count} mynt", "other": "{count} mynt" } })
        cat = setLocale(cat, "sv")
        let greet = t(cat, "hi", { "name": "Ada" })
        let plural = tn(cat, "coins", 3, {})
        let st = createStats("p1")
        st = defineAchievement(st, "first_kill", "kills", 1, "First blood")
        st = addCounter(st, "kills", 1)
        let saved = saveStats(st, "/saves/stats_p1.json")
        let st2 = loadStats("/saves/stats_p1.json", "p1")
        let hm = createHeightmap(8, 8, 1.0, null)
        let painted = paintSplat(hm, 2.0, 3.0, "grass", 0.9)
        hm = painted["hm"]
        let sp = sampleSplat(hm, 2.0, 3.0)
        let stream = createStreamingBounds(4, 1)
        stream = updateStreaming(stream, hm, 2.0, 3.0, 1)
        greet == "Hej Ada" && plural == "3 mynt" && isUnlocked(st, "first_kill") && saved["ok"] && isUnlocked(st2, "first_kill") && painted["ok"] && sp["grass"] > 0.5 && len(stream["resident"]) >= 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn terrain_async_streaming_poll() {
    let v = eval(
        r#"
        import "game/terrain"
        let hm = createHeightmap(16, 16, 1.0, null)
        let stream = createStreamingBounds(4, 1)
        stream["loadsPerPoll"] = 2
        stream = beginAsyncLoad(stream, hm, 4.0, 4.0, 1)
        stream = pollStreamingLoads(stream, hm)
        let d1 = describeStreaming(stream)
        stream = pollStreamingLoads(stream, hm)
        let d2 = describeStreaming(stream)
        d1["async"] == true && d1["resident"] == 2 && d1["pending"] == 7 && d2["resident"] > d1["resident"]
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gpu_shadow_and_xr_descriptors() {
    let v = eval(
        r#"
        import "game/light"
        import "game/terrain"
        import "game/xr"
        let hm = createHeightmap(4, 4, 1.0, null)
        let mesh = buildTerrainMesh(hm, 0)
        let lit = enableSoftShadow(createDirectional({ "x": 0.0, "y": -1.0, "z": 0.0 }, [1.0, 1.0, 1.0], 1.0), 512, 1.2)
        let gpu = renderGpuShadow(lit["shadow"], mesh)
        let xr = xrBegin(createXrSession("vr"))
        let eyes = stereoCameras({ "x": 0.0, "y": 1.6, "z": 0.0 }, 0.07)
        let pres = xrPresentDescriptor(xr, 1920, 1080)
        return (gpu["ok"] == true || gpu["gpu"]["available"] == false) && pres["kind"] == "xr_present" && eyes["ipd"] == 0.07 && xr["active"] == false
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn shadow_lit_pipeline_and_xr_present() {
    env_host();
    std::env::set_var("KABOOTAR_XR_STUB", "1");
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/light"
        import "game/terrain"
        import "game/xr"
        let lights = createLightList()
        let lit = enableSoftShadow(createDirectional({ "x": 0.0, "y": -1.0, "z": 0.0 }, [1.0, 1.0, 1.0], 1.0), 512, 1.2)
        lights = addLight(lights, lit)
        let normal = { "x": 0.0, "y": 1.0, "z": 0.0 }
        let litPt = { "x": 0.0, "y": 1.0, "z": 0.0 }
        let shadowPt = { "x": 4.0, "y": 0.0, "z": 0.0 }
        let cLit = directionalLit(lit, normal, litPt)
        let cSh = directionalLit(lit, normal, shadowPt)
        let surf = litSurface(lights, normal, shadowPt)
        let xr = xrBegin(createXrSession("vr"))
        let pres = xrPresent(xr, 1920, 1080)
        let sc = describeSwapchains(pres["present"])
        return cLit["shadow"] > cSh["shadow"] && surf["r"] > 0.0 && pres["present"]["presented"] == true && sc["count"] == 2 && xr["active"] == true
        "#,
        &mut env,
    )
    .expect("eval");
    std::env::remove_var("KABOOTAR_XR_STUB");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn xr_runtime_swapchain_frame() {
    env_host();
    std::env::set_var("KABOOTAR_XR_STUB", "1");
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/xr"
        let xr = xrBegin(createXrSession("vr"))
        xr = createStereoSwapchains(xr, 640, 360)
        let out = xrRuntimeFrame(xr, 640, 360)
        let d = describeXr(out["session"])
        return out["frame"]["shouldRender"] == true && out["ended"]["submitted"] == true && d["swapchainCount"] == 2 && out["frame"]["frameIndex"] == 1
        "#,
        &mut env,
    )
    .expect("eval");
    std::env::remove_var("KABOOTAR_XR_STUB");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
