//! GP4a — asset_watch / asset_poll mtime hot reload.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;
use std::thread;
use std::time::Duration;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

#[test]
fn asset_poll_sees_mtime_change() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();

    let dir = std::env::temp_dir().join(format!(
        "kabootar_gp4a_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    let path = dir.join("watched.kab");
    let path_str = path.to_string_lossy().replace('\\', "/");
    std::fs::write(&path, "// v1\n").expect("write");

    let mut env = create_global_env();
    env.set("watch_path".into(), Value::String(path_str.clone()));

    let first = eval_source(
        r#"
        import "game/hot"
        watch(watch_path)
        let empty = poll()
        len(empty) == 0
        "#,
        &mut env,
    )
    .expect("eval watch");
    assert!(matches!(first, Value::Bool(true)), "got {first:?}");

    // Cross coarse FS mtime resolution (esp. Windows).
    thread::sleep(Duration::from_millis(1100));
    std::fs::write(&path, "// v2 changed\n").expect("rewrite");

    let second = eval_source(
        r#"
        import "game/hot"
        let changed = poll()
        len(changed) == 1
        "#,
        &mut env,
    )
    .expect("eval poll");
    assert!(matches!(second, Value::Bool(true)), "got {second:?}");

    let _ = std::fs::remove_dir_all(&dir);
}
