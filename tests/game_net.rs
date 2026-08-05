//! GP3d — multiplayer tick/snapshot hooks.

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
fn net_snapshot_roundtrip() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/net"
        import "game/ecs"
        let session = createSession("local")
        let w = createWorld()
        let id = spawn(w)
        w = add(w, id, "pos", { x: 3, y: 4 })
        session = sendTick(session, w)
        let out = drainOutgoing(session)
        let remote = createSession("local")
        remote = receiveRemote(remote, out[0])
        let empty = createWorld()
        let applied = pollRemote(remote, empty)
        let got = get(applied["world"], id, "pos")
        session["tick"] == 1 && len(out) == 1 && got["x"] == 3 && got["y"] == 4
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
