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
    let sync = eval(
        r#"
        platform_use("hybrid");
        kb_sync_platform();
        "#,
    );
    let Value::Object(o) = sync else {
        panic!("expected sync object, got {sync:?}");
    };
    assert!(matches!(o.get("mode"), Some(Value::String(s)) if s == "auto"));
    assert!(matches!(o.get("schemes"), Some(Value::Array(a)) if a.len() >= 3));
    let mode = eval("kb_os_mode()");
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

#[test]
fn g11_platform_classes_and_lib() {
    let out = eval(
        r#"
        platform_use("hybrid");
        let sync = kb_sync_platform();
        let info = kb_os_info();
        os_mkdir("/apps");
        os_write("/apps/g11.kml", "<html><body><h1>kOS</h1></body></html>");
        kb_set_os_mode("kabootar");
        kb_navigate("kabootar://vfs/apps/g11.kml");
        let html = kb_render();
        is_object(sync) && sync.mode == "auto" && len(sync.schemes) >= 3
            && is_object(info) && is_string(info.host_os)
            && is_string(html) && len(html) > 5
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn g7_mobile_shell_chrome_back_tabs() {
    let out = eval(
        r#"
        import "kbrowser/mobile_chrome"
        applyPhoneViewport();
        applySafeArea();
        mountChrome("kabootar://home");
        kb_paint();
        os_mkdir("/apps");
        os_write("/apps/a.kml", "<html><body><h1>A</h1></body></html>");
        os_write("/apps/b.kml", "<html><body><h1>B</h1></body></html>");
        kb_set_os_mode("kabootar");
        kb_navigate("kabootar://vfs/apps/a.kml");
        kb_navigate("kabootar://vfs/apps/b.kml");
        let went = goBack();
        let tabs = listTabs();
        went == true && is_array(tabs) && len(tabs) >= 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}
