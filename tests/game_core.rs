//! game/core foundation — ECS/scene/render shims (no Bazi components here).

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

#[test]
fn game_core_shim_and_canonical_paths() {
    env_host();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/ecs"
        import "game/core/ecs"
        import "game/core"
        let w = createWorld()
        let id = spawn(w)
        w = add(w, id, "tag", { "v": 1 })
        let n = createNode("root")
        n = setLocal(n, 1, 2, 3)
        let p = worldPos(n)
        return get(w, id, "tag")["v"] == 1 && p["x"] == 1 && p["y"] == 2 && p["z"] == 3
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
