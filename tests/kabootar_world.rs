//! Kabootar World — end-to-end: OS → browser → raster → events → persist

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::{Environment, Value};
use std::path::PathBuf;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

fn eval_in(env: &mut Environment, code: &str) -> Value {
    eval_source(code, env).unwrap()
}

fn vfs_snapshot_path() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target-local");
    p.push("kabootar-world-vfs.kvf");
    p.to_string_lossy().into_owned()
}

#[test]
fn kabootar_world_os_browser_raster() {
    let out = eval(
        r##"
        platform_use("kabootar");
        let pid = os_spawn("browser");
        let win = os_window_create("World", 800, 600);
        os_window_bind(win, 1);
        os_display_register(win, "World Desktop", 800, 600);
        let mem = os_mem_alloc(4096, "framebuffer");
        let stats = os_mem_stats();
        os_sched_enqueue("paint-loop");
        let ui = kdom_listen(kml("<html><body><h1>Kabootar World</h1><button>Go</button></body></html>"), "button", "click", "on_go");
        kb_mount(ui);
        kb_viewport(800, 600);
        kb_paint();
        kb_pixels();
        "##,
    );
    assert!(matches!(out, Value::Object(obj) if
        obj.get("bytes").and_then(|b| match b {
            Value::Number(n) => Some(*n > 0),
            _ => None,
        }).unwrap_or(false)
    ));
}

#[test]
fn kabootar_world_click_events() {
    let events = eval(
        r#"
        platform_use("kabootar");
        let ui = kdom_listen(kml("<html><body style='padding:0;margin:0'><button style='width:200px;height:40px'>Tap</button></body></html>"), "button", "click", "handle_tap");
        kb_mount(ui);
        kb_viewport(400, 200);
        kb_paint();
        kb_click(100, 20);
        kb_poll_events();
        "#,
    );
    assert!(matches!(events, Value::Array(list) if
        list.len() == 1 &&
        matches!(&list[0], Value::Object(e) if
            e.get("handler").and_then(|v| match v {
                Value::String(s) => Some(s == "handle_tap"),
                _ => None,
            }).unwrap_or(false)
        )
    ));
}

#[test]
fn kabootar_world_vfs_persist_roundtrip() {
    let path = vfs_snapshot_path();
    let _ = std::fs::remove_file(&path);
    let mut env = create_global_env();
    eval_in(
        &mut env,
        &format!(
            r##"
            os_mkdir("/world");
            os_write("/world/data.txt", "kabootar-world");
            os_vfs_save("{path}");
            "##
        ),
    );
    let read1 = eval_in(&mut env, r#"os_read("/world/data.txt")"#);
    assert!(matches!(read1, Value::String(s) if s == "kabootar-world"));

    eval_in(
        &mut env,
        &format!(
            r##"
            os_write("/world/data.txt", "stale");
            os_vfs_load("{path}");
            "##
        ),
    );
    let read2 = eval_in(&mut env, r#"os_read("/world/data.txt")"#);
    assert!(matches!(read2, Value::String(s) if s == "kabootar-world"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn kabootar_world_syscall_bridge() {
    let info = eval(r#"os_syscall("info")"#);
    assert!(matches!(info, Value::String(s) if s.contains("kabootar-kernel")));

    let spawn = eval(r#"os_syscall("spawn", "worker")"#);
    assert!(matches!(spawn, Value::Number(n) if n > 0));
}

#[cfg(feature = "gpu")]
#[test]
fn kabootar_world_gpu_upload_when_available() {
    let frame = eval(
        r##"
        kb_set_backend("gpu");
        kb_mount(kml("<html><body><h1>GPU</h1></body></html>"));
        kb_viewport(320, 240);
        kb_paint();
        "##,
    );
    assert!(matches!(frame, Value::Object(obj) if
        obj.get("backend").and_then(|v| match v {
            Value::String(s) => Some(s == "gpu" || s == "cpu"),
            _ => None,
        }).unwrap_or(false)
    ));
    let _ = eval(r#"kb_set_backend("cpu")"#);
}
