//! Kv8 engine + OS integration tests

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn kv8_info_and_create() {
    let info = eval("kv8_info()");
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("engine"), Some(Value::String(s)) if s == "kv8"));
    let ctx = eval("kv8_create()");
    assert!(matches!(ctx, Value::Kv8Context(_)));
}

#[test]
fn kv8_dom_script_create_element() {
    let dom = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let el = document.createElement('div'); document.appendChild(el);");
        kv8_dom(ctx);
        "#,
    );
    assert!(matches!(dom, Value::KabootarDom(n) if !n.children.is_empty()));
}

#[test]
fn kv8_css_and_computed_style() {
    let attr = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let el = document.createElement('div');
          el.style.color = '#ff0000';
          document.appendChild(el);
        ");
        let root = kv8_dom(ctx);
        let kids = kdom_children(root);
        kdom_get_attr(kids[0], "style:color");
        "#,
    );
    assert!(matches!(attr, Value::String(s) if s.contains("ff0000")));
}

#[test]
fn kv8_js_style_assignment() {
    let _ = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let el = document.createElement('button');
          el.style.color = '#00ff00';
          document.appendChild(el);
        ");
        "#,
    );
}

#[test]
fn os_write_triggers_journal_and_prefetch() {
    let prefetch = eval(
        r#"
        os_mkdir("/apps/spotify");
        os_write("/apps/spotify/config.json", "{}");
        os_ai_prefetch();
        "#,
    );
    let Value::Array(apps) = prefetch else {
        panic!("expected array");
    };
    assert!(apps.iter().any(|v| matches!(v, Value::String(s) if s.contains("spotify"))));
}

#[test]
fn os_spawn_registers_thread_and_ai() {
    let threads = eval(
        r#"
        os_spawn("mail-client");
        os_architecture()["part3_threads"];
        "#,
    );
    assert!(matches!(threads, Value::String(s) if s.parse::<u32>().unwrap_or(0) >= 1));
}

#[test]
fn golden_restore_resets_os_partition() {
    let out = eval(
        r#"
        os_write("/system/hack.txt", "bad");
        os_recovery_restore();
        os_read("/system/README");
        "#,
    );
    assert!(matches!(out, Value::String(s) if s.contains("golden")));
}
