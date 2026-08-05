//! GP4b — editor hierarchy + inspector.

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
