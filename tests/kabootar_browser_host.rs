//! Kabootar Browser — multi-OS navigation (Kabootar VFS + host)

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn kb_os_info_reports_modes() {
    let info = eval("kb_os_info()");
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("mode"), Some(Value::String(_))));
    assert!(matches!(o.get("host_os"), Some(Value::String(_))));
    assert!(matches!(o.get("kabootar_os"), Some(Value::Bool(true))));
}

#[test]
fn vfs_and_host_mode_switch() {
    let html = eval(
        r#"
        os_write("/apps/page.kml", "<html><body><h1>VFS Page</h1></body></html>");
        kb_set_os_mode("kabootar");
        kb_navigate("kabootar://vfs/apps/page.kml");
        kb_render();
        "#,
    );
    assert!(matches!(html, Value::String(s) if s.contains("VFS Page")));

    let mode = eval(
        r#"
        kb_set_os_mode("host");
        kb_os_mode();
        "#,
    );
    assert!(matches!(mode, Value::String(s) if s == "host"));
}

#[test]
fn sync_platform_maps_hybrid_to_auto() {
    let mode = eval(
        r#"
        platform_use("hybrid");
        kb_sync_platform();
        kb_os_mode();
        "#,
    );
    assert!(matches!(mode, Value::String(s) if s == "auto"));
}

#[test]
fn host_file_url_loads_native_file() {
    let dir = std::env::temp_dir().join("kabootar-browser-test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("page.kml");
    std::fs::write(&path, "<html><body><h1>Host File</h1></body></html>").unwrap();
    let url = format!("file:///{}", path.display().to_string().replace('\\', "/"));
    let code = format!(
        r#"
        kb_set_os_mode("host");
        kb_navigate("{url}");
        kb_render();
        "#
    );
    let html = eval(&code);
    assert!(matches!(html, Value::String(s) if s.contains("Host File")));
}

#[test]
fn g7_mobile_viewport_touch_safe_area() {
    let out = eval(
        r#"
        let vp = kb_viewport(390, 844, 3, "portrait");
        let sa = kb_safe_area(47, 0, 34, 0);
        let sa2 = kb_safe_area();
        kb_paint();
        let touched = kb_touch_at(10, 10, "start");
        let ev = kb_poll_events();
        is_object(vp) && vp.width == 390 && vp.height == 844 && vp.dpr == 3
            && vp.orientation == "portrait"
            && is_object(sa) && sa.top == 47 && sa.bottom == 34
            && sa2.top == 47
            && is_array(ev)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}
