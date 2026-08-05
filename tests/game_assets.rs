//! GP2d — asset database (VFS + host).

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
fn assets_vfs_text_and_host_png() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let png = format!("{}/fixtures/game/px.png", env!("CARGO_MANIFEST_DIR")).replace('\\', "/");
    env.set("png_host".into(), Value::String(png));
    let v = eval_source(
        r#"
        import "game/assets"
        os_mkdir("/game")
        os_write("/game/note.txt", "hello-asset")
        let db = createDb("/game")
        db = registerVfs(db, "note", "/game/note.txt", "txt")
        db = registerHost(db, "px", png_host, "png")
        let text = loadText(db, "note")
        let img = loadPng(db, "px")
        text == "hello-asset" && img["width"] == 2 && resolve(db, "px") == png_host
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
