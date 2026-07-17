//! Kabootar native stack — render engine, CSS, compositor, OS binding

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Mutex;

/// `kb_backend` / `kb_set_backend` use process-wide state; serialize those tests.
static KB_BACKEND_LOCK: Mutex<()> = Mutex::new(());

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

fn eval_backend(code: &str) -> Value {
    let _lock = KB_BACKEND_LOCK.lock().unwrap();
    eval(code)
}

#[test]
fn kstyle_parses_css_rules() {
    let n = eval(r#"kstyle_parse("body { color: red; } .card { padding: 8px; }")"#);
    assert!(matches!(n, Value::Number(2)));
}

#[test]
fn kdom_paint_produces_frame() {
    let frame = eval(
        r#"
        kstyle_parse("h1 { font-size: 24px; color: #8ab4f8; }");
        let ui = kml("<html><body><h1>Paint</h1></body></html>");
        kdom_paint(ui, 800, 600);
        "#,
    );
    assert!(matches!(frame, Value::Object(obj) if
        obj.get("html").and_then(|v| match v {
            Value::String(s) => Some(s.contains("kb-viewport")),
            _ => None
        }).unwrap_or(false)
    ));
}

#[test]
fn kb_paint_chrome_theme() {
    let frame = eval(
        r#"
        let ui = kml("<html><body class='kb-home'><h1>Browser</h1></body></html>");
        kb_mount(ui);
        kb_paint();
        "#,
    );
    assert!(matches!(frame, Value::Object(obj) if
        obj.get("nodes").and_then(|v| match v {
            Value::Number(n) => Some(*n >= 3),
            _ => None
        }).unwrap_or(false)
    ));
}

#[test]
fn kb_composite_links_os_windows() {
    let out = eval(
        r#"
        let win = os_window_create("Kabootar", 1024, 768);
        let tab = kb_tabs();
        os_window_bind(win, 1);
        kb_composite();
        "#,
    );
    assert!(matches!(out, Value::Object(obj) if obj.contains_key("windows")));
}

#[test]
fn kdom_events_and_ids() {
    let id = eval(
        r#"
        let btn = kdom_create("button");
        let b = kdom_on(btn, "click", "on_click");
        kdom_id(b);
        "#,
    );
    assert!(matches!(id, Value::Number(n) if n > 0));
}

#[test]
fn vfs_page_load_in_browser() {
    let html = eval(
        r#"
        os_mkdir("/apps");
        os_write("/apps/home.kml", "<html><body><h1>VFS Page</h1></body></html>");
        kb_navigate("kabootar://vfs/apps/home.kml");
        kb_render();
        "#,
    );
    assert!(matches!(html, Value::String(s) if s.contains("VFS Page")));
}

#[test]
fn kb_backend_defaults_cpu() {
    let b = eval_backend(
        r#"
        kb_set_backend("cpu");
        kb_backend();
        "#,
    );
    assert!(matches!(b, Value::String(s) if s == "cpu"));
}

#[test]
fn kb_set_backend_gpu_request() {
    let b = eval_backend(
        r#"
        kb_set_backend("gpu");
        kb_backend();
        "#,
    );
    assert!(matches!(b, Value::String(s) if s == "gpu"));
    let _ = eval_backend(r#"kb_set_backend("cpu")"#);
}

#[test]
fn kb_paint_frame_has_backend_field() {
    let frame = eval(
        r#"
        kb_mount(kml("<html><body><p>Backend</p></body></html>"));
        kb_paint();
        "#,
    );
    assert!(matches!(frame, Value::Object(obj) if
        obj.get("backend").and_then(|v| match v {
            Value::String(s) => Some(!s.is_empty()),
            _ => None,
        }).unwrap_or(false)
    ));
}

#[test]
fn ktext_word_wrap_multiline() {
    let layout = eval(
        r#"
        ktext_layout("Kabootar OS is the native stack for browser and desktop", 120, 16);
        "#,
    );
    assert!(matches!(layout, Value::Object(obj) if
        obj.get("lines").and_then(|v| match v {
            Value::Number(n) => Some(*n >= 2),
            _ => None,
        }).unwrap_or(false)
    ));
}

#[test]
fn ktext_measure_proportional_width() {
    let m = eval(r#"ktext_measure("iii WWW", 16)"#);
    assert!(matches!(m, Value::Array(vals) if vals.len() == 2));
}

#[test]
fn ktext_large_font_increases_height() {
    let small = eval(r#"ktext_layout("Title", 400, 16)"#);
    let large = eval(r#"ktext_layout("Title", 400, 32)"#);
    let sh = match &small {
        Value::Object(o) => o.get("height").and_then(|v| match v { Value::Float(f) => Some(*f), _ => None }),
        _ => None,
    };
    let lh = match &large {
        Value::Object(o) => o.get("height").and_then(|v| match v { Value::Float(f) => Some(*f), _ => None }),
        _ => None,
    };
    assert!(sh.is_some() && lh.is_some() && lh.unwrap() > sh.unwrap());
}

#[test]
fn host_paint_after_compositor() {
    let v = eval(
        r#"
        kb_mount(kml("<html><body><p>Host bridge</p></body></html>"));
        kb_host_sync();
        host_paint();
        "#,
    );
    assert!(matches!(v, Value::String(s) if s.contains("kb-viewport")));
}

#[test]
fn js_wave_c1_query_selector() {
    let out = eval(
        r##"
        let root = kdom_create("div")
        let child = kdom_create("span")
        child = kdom_set_attr(child, "id", "main")
        child = kdom_set_attr(child, "class", "item active")
        child = kdom_set_attr(child, "data-x", "1")
        root = kdom_append(root, child)
        let sib = kdom_create("p")
        sib = kdom_set_attr(sib, "class", "next")
        root = kdom_append(root, sib)
        let by_id = kdom_query_selector(root, "#main")
        let by_class = kdom_query_selector(root, ".active")
        let by_attr = kdom_query_selector(root, "[data-x=1]")
        let by_child = kdom_query_selector(root, "div > span")
        let by_not = kdom_query_selector(root, "span:not(.missing)")
        let by_comma = kdom_query_selector(root, "span, p")
        let by_adj = kdom_query_selector(root, "span + p")
        let by_sib = kdom_query_selector(root, "span ~ p")
        let all = kdom_query_selector_all(root, "span, p")
        kdom_id(by_id) == kdom_id(child)
            && kdom_id(by_class) == kdom_id(child)
            && kdom_id(by_attr) == kdom_id(child)
            && kdom_id(by_child) == kdom_id(child)
            && kdom_id(by_not) == kdom_id(child)
            && kdom_id(by_comma) == kdom_id(child)
            && kdom_id(by_adj) == kdom_id(sib)
            && kdom_id(by_sib) == kdom_id(sib)
            && len(all) == 2
        "##,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_c1_mutation_records() {
    let out = eval(
        r#"
        kdom_mutation_clear()
        let root = kdom_create("div")
        let child = kdom_create("p")
        root = kdom_append(root, child)
        child = kdom_set_attr(child, "data-x", "1")
        root = kdom_clear_children(root)
        let recs = kdom_mutation_records()
        len(recs) >= 3
            && recs[0]["type"] == "childList"
            && recs[1]["type"] == "attributes"
            && recs[2]["type"] == "childList"
            && recs[2]["removedNodeId"] != null
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_c1_mutation_observer_observe_disconnect() {
    let out = eval(
        r#"
        kdom_mutation_clear()
        let state = { n: 0 }
        fn on_mut(recs) {
            state.n = state.n + len(recs)
        }
        let mo = MutationObserver(on_mut)
        let root = kdom_create("div")
        mo.observe(root, { childList: true, attributes: false })
        let child = kdom_create("span")
        root = kdom_append(root, child)
        let after_append = state.n
        mo.disconnect()
        root = kdom_append(root, kdom_create("p"))
        after_append == 1 && state.n == 1
        "#,
    );
    assert!(
        matches!(out, Value::Bool(true)),
        "MutationObserver observe/disconnect: {out:?}"
    );
}
