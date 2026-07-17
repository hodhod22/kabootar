//! Kabootar language standout features — 20-point matrix

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::lang_preprocess;
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn lang_info_lists_twenty_features() {
    let list = eval("lang_info()");
    let Value::Array(items) = list else {
        panic!("expected array");
    };
    assert!(items.len() >= 20, "expected 20 features, got {}", items.len());
}

#[test]
fn html_macro_expands_to_kv8() {
    let out = lang_preprocess::expand_html_blocks(r#"html! { <p>Hi</p> }"#);
    assert!(out.contains("kv8_run_html"));
    let ctx = eval(r#"html! { <main>Kabootar</main> }"#);
    assert!(matches!(ctx, Value::Kv8Context(_)));
}

#[test]
fn channels_send_and_recv() {
    let out = eval(
        r#"
        let ch = channel_new(4);
        channel_send(ch, 42);
        channel_recv(ch);
        "#,
    );
    assert!(matches!(out, Value::Number(42)));
}

#[test]
fn actor_spawn_returns_mailbox() {
    let out = eval(r#"actor Worker { }"#);
    assert!(matches!(out, Value::Number(_) | Value::Object(_)));
    let handle = eval(r#"let a = actor_spawn("Counter"); a"#);
    let Value::Object(o) = handle else {
        panic!("expected actor object");
    };
    assert!(o.contains_key("mailbox"));
}

#[test]
fn comptime_assert_passes() {
    let ok = eval(r#"comptime_assert(true, "ok")"#);
    assert!(matches!(ok, Value::Bool(true)));
}

#[test]
fn lang_benchmark_runs_function() {
    let out = eval(
        r#"
        fn tick() { return 1 }
        lang_benchmark("tick", 50, tick);
        "#,
    );
    let Value::Object(o) = out else {
        panic!("expected bench object");
    };
    assert!(matches!(o.get("iterations"), Some(Value::Number(50))));
}

#[test]
fn effect_directives_stripped() {
    let src = "@pure\n@io\nlet x = 7";
    let body = lang_preprocess::strip_header_directives(src);
    assert!(!body.contains("@pure"));
    assert!(body.contains("let x = 7"));
}

#[test]
fn g2_array_member_push_on_expression() {
    let out = eval(
        r#"
        let a = [1];
        let n1 = a.push(2);
        let n2 = ([10]).push(20);
        a[1] == 2 && n1 == 2 && n2 == 2
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn lang_syscalls_zero_ffi() {
    let list = eval("lang_syscalls()");
    let Value::Array(items) = list else {
        panic!("expected syscall list");
    };
    assert!(!items.is_empty());
}

#[test]
fn persist_roundtrip_via_os() {
    let out = eval(
        r#"
        persist_save("/data/lang-test.json", { "v": 1 });
        persist_load("/data/lang-test.json");
        "#,
    );
    assert!(matches!(out, Value::String(s) if s.contains("v")));
}

#[test]
fn shader_compile_stub() {
    let out = eval(r#"shader_compile("vert", "void main(){}")"#);
    let Value::Object(o) = out else {
        panic!("expected shader object");
    };
    assert!(matches!(o.get("ok"), Some(Value::Bool(true))));
}
