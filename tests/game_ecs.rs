//! GP1e — thin ECS component store.

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
fn ecs_spawn_add_query() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/ecs"
        let w = createWorld()
        let a = spawn(w)
        let b = spawn(w)
        w = add(w, a, "pos", { x: 1, y: 2 })
        w = add(w, b, "pos", { x: 3, y: 4 })
        w = add(w, a, "hp", { v: 10 })
        let q = query(w, "pos")
        len(q) == 2 && get(w, a, "hp")["v"] == 10 && has(w, b, "hp") == false
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
