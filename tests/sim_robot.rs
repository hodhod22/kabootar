//! Sim / robotics MVP — arm twin, joints, ODE step, sensors.

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
fn sim_arm3_fk_sensors_twin() {
    let v = eval(
        r#"
        import "science"
        import "sim"
        import "sim/robot"
        import "game/editor"
        os_mkdir("/sim")
        let arm = createArm3(defaultArmParams())
        let ee0 = endEffector(arm)
        let x0 = ee0["x"]
        arm = setArmTargets(arm, 0.7, 0.5, -0.3)
        arm = simulateArm(arm, 1.0 / 60.0, 100)
        let ee1 = endEffector(arm)
        let moved = (ee1["x"] - x0) * (ee1["x"] - x0) + ee1["y"] * ee1["y"] + ee1["z"] * ee1["z"] > 0.01
        let enc = readEncoders(arm)
        let imu = readImu(arm)
        let root = worldToEditor(arm)
        let ed = createEditor(root)
        ed = refresh(ed)
        let lesson = buildTwinLesson(arm)
        let ik = inverseKinematics(arm, 1.2, 0.5, 0.0)
        let w2c = resolveGroundContact(arm, 0.0)
        moved && len(enc) == 3 && imu["kind"] == "imu" && len(ed["hierarchy"]) >= 4 && lesson["dof"] == 3 && ik["ok"] == true && w2c["contacts"]["kind"] == "ground"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sim_teleop_joint_ik_learn() {
    let v = eval(
        r#"
        import "science"
        import "sim"
        import "sim/robot"
        import "sim/teleop"
        import "game/editor"
        os_mkdir("/sim")
        let tele = bindArmEditor(createArm3(defaultArmParams()), "/sim/tele_test.json")
        tele = selectLink(tele, "link1")
        tele = teleopSetJoint(tele, "j1", 0.9, 80)
        let q = findJoint(tele["world"], "j1")["q"]
        let h0 = len(tele["editor"]["hierarchy"])
        tele = enterIkMode(tele)
        tele = teleopPlaceEe(tele, 1.1, 0.35, 0.0, 70)
        let ee = endEffector(tele["world"])
        tele = setLearnParam(tele, "kp", 8.0)
        tele = setLearnJoint(tele, "j2", 0.2, 30)
        tele = teleopStep(tele, 1.0 / 60.0, 10)
        q > 0.35 && h0 >= 4 && ee["x"] != null && tele["world"]["params"]["kp"] >= 40.0 && tele["learnUi"]["kind"] == "teleop_learn" && tele["mode"] == "learn" && tele["ticks"] > 0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sim_hinge_slider_rk4() {
    let v = eval(
        r#"
        import "sim"
        let w = createWorld(defaultSimParams())
        w = applyLiveParams(w, { "solver": "rk4", "kp": 50.0, "kd": 6.0 })
        w = addBody(w, createFixedBase("base", 0.0, 0.0, 0.0))
        w = addBody(w, createBody("link", 1.0, 1.0, 0.0, 0.0))
        w = addJoint(w, createHinge("h1", "base", "link", "z", 1.0, 0.4))
        w = setJointTarget(w, "h1", 1.0)
        w = stepN(w, 1.0 / 60.0, 80)
        let q = findJoint(w, "h1")["q"]
        let w2 = createWorld(defaultSimParams())
        w2 = addBody(w2, createFixedBase("rail", 0.0, 0.0, 0.0))
        w2 = addBody(w2, createBody("cart", 1.0, 0.0, 0.0, 0.0))
        w2 = addJoint(w2, createSlider("s1", "rail", "cart", "x", 2.0, 1.0))
        w2 = setJointTarget(w2, "s1", 1.5)
        w2 = stepN(w2, 1.0 / 60.0, 100)
        q > 0.4 && findBody(w2, "cart")["x"] > 0.8
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
