//! GP3c — grid A* navigation.

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
fn nav_astar_finds_path_around_wall() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/nav"
        let g = createGrid(5, 3, null)
        g = setBlocked(g, 1, 0, 1)
        g = setBlocked(g, 1, 1, 1)
        let path = astar(g, 0, 0, 2, 0)
        path != null && pathCost(path) >= 4 && path[0]["x"] == 0 && path[len(path)-1]["x"] == 2
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn nav_astar_blocked_goal_is_null() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "game/nav"
        let g = createGrid(3, 3, null)
        g = setBlocked(g, 2, 2, 1)
        astar(g, 0, 0, 2, 2) == null
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
