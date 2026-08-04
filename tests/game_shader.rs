//! GP0e — WGSL pipeline cache + hot reload.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::{format_value, Value};
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

#[cfg(feature = "gpu")]
#[test]
fn wgsl_load_from_file_and_hot_reload() {
    test_runtime_env();
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    let avail = format_value(&eval_source(r#"webgl_info()["gpu3d"]"#, &mut env).unwrap());
    if avail == "cpu-fallback" {
        return;
    }

    let dir = std::env::temp_dir().join(format!("kab_wgsl_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("solid.wgsl");
    let fixture = format!("{}/fixtures/game/solid.wgsl", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&fixture).expect("fixture");
    std::fs::write(&path, &src).unwrap();
    let path_s = path.to_string_lossy().replace('\\', "/");

    env.set("wgsl_path".into(), Value::String(path_s.clone()));
    let first = eval_source(
        r#"
        import "game/shader"
        let info = loadSolidFromFile(wgsl_path)
        info["cache_hit"]
        "#,
        &mut env,
    )
    .expect("load1");
    assert!(matches!(first, Value::String(ref s) if s == "false"), "first load should miss: {first:?}");

    let second = eval_source(
        r#"
        import "game/shader"
        let info = loadSolidFromFile(wgsl_path)
        info["cache_hit"]
        "#,
        &mut env,
    )
    .expect("load2");
    assert!(matches!(second, Value::String(ref s) if s == "true"), "second load should hit: {second:?}");

    // Change content so hash differs.
    let mutated = src.replace("0.95", "0.90");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&path, mutated).unwrap();

    let reloaded = eval_source(
        r#"
        import "game/shader"
        let changed = pollReload()
        let info = info()
        len(changed) >= 1 && info["reload_count"] != "0"
        "#,
        &mut env,
    )
    .expect("reload");
    assert!(matches!(reloaded, Value::Bool(true)), "got {reloaded:?}");

    let _ = std::fs::remove_dir_all(&dir);
}
