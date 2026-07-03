//! Kabootar dual-layer platform tests — host vs native stack

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn platform_dual_layers_registered() {
    let info = eval("platform_info()");
    assert!(matches!(info, Value::Object(_)));
    let layer = eval("platform_layer()");
    assert!(matches!(layer, Value::String(s) if s == "hybrid"));
}

#[test]
fn host_layer_chrome_like_document() {
    let ua = eval("navigator.userAgent");
    assert!(matches!(ua, Value::String(s) if s.contains("Chrome")));
    let title = eval("document.title");
    assert!(matches!(title, Value::String(_)));
}

#[test]
fn kabootar_dom_live_api() {
    let v = eval(
        r#"
        let root = kdom_create("div");
        let child = kdom_append(root, kdom_text("Hej"));
        kdom_set_attr(child, "class", "app");
        kdom_render(child);
        "#,
    );
    assert!(matches!(v, Value::String(s) if s.contains("Hej")));
}

#[test]
fn kabootar_browser_navigation() {
    let v = eval(
        r#"
        kb_navigate("kabootar://apps/home");
        kb_location();
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "kabootar://apps/home"));
}

#[test]
fn kabootar_browser_mount_kdom() {
    let html = eval(
        r#"
        let ui = kml("<div><h1>Kabootar</h1></div>");
        kb_mount(ui);
        kb_render();
        "#,
    );
    assert!(matches!(html, Value::String(s) if s.contains("Kabootar")));
}

#[test]
fn kabootar_os_windows_and_processes() {
    let pid = eval("os_spawn(\"my-app\")");
    assert!(matches!(pid, Value::Number(n) if n >= 2));
    let wins = eval("os_window_create(\"Main\", 1024, 768)");
    assert!(matches!(wins, Value::Number(n) if n >= 1));
    let caps = eval("os_caps()");
    assert!(matches!(caps, Value::Array(_)));
}

#[test]
fn platform_switch_to_kabootar() {
    let layer = eval(
        r#"
        platform_use("kabootar");
        platform_layer();
        "#,
    );
    assert!(matches!(layer, Value::String(s) if s == "kabootar"));
}
