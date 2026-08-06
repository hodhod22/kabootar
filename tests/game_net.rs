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

#[test]
fn net_lobby_interest_prediction() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/net"
        let lobby = createLobby("arena", 4)
        lobby = joinLobby(lobby, "p1")
        lobby = joinLobby(lobby, "p2")
        let ready = lobbyReady(lobby)
        let mm = matchmake([], "p3", "quick", 2)
        let interest = createInterest(5.0)
        interest = setInterestCenter(interest, 0.0, 0.0, 0.0)
        let near = filterByInterest(interest, [
            { "x": 1.0, "y": 0.0, "z": 0.0 },
            { "x": 20.0, "y": 0.0, "z": 0.0 }
        ])
        let pred = predictMove({ "x": 0.0, "y": 0.0, "vx": 1.0, "vy": 0.0 }, { "ax": 0.0, "ay": 0.0 }, 1.0)
        let rec = reconcile(pred, { "x": 0.0, "y": 0.0, "vx": 0.0, "vy": 0.0 }, 0.1)
        ready && len(lobby["players"]) == 2 && mm["created"] == true && len(near) == 1 && pred["x"] == 1.0 && rec["snapped"] == true
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
