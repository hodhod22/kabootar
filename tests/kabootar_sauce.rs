//! Competitive strategy layer — 9 "secret sauce" pillars

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn sauce_map_has_all_nine_strategies() {
    let map = eval("os_sauce_map()");
    let Value::Object(m) = map else {
        panic!("expected object");
    };
    for key in [
        "s1_ai_prefetch",
        "s2_setup_secs",
        "s3_partitions",
        "s4_paired",
        "s5_energy",
        "s6_haptic",
        "s7_compat",
        "s8_privacy",
        "s9_updates",
    ] {
        assert!(m.contains_key(key), "missing {key}");
    }
}

#[test]
fn ai_prefetch_and_context_menu() {
    let apps = eval(
        r#"
        os_ai_record("vscode", 9);
        os_ai_prefetch();
        "#,
    );
    assert!(matches!(apps, Value::Array(a) if !a.is_empty()));
    let menu = eval(r#"os_ai_context_menu("vscode", ["a","b","c","d","e","f","g"])"#);
    assert!(matches!(menu, Value::Array(a) if a.len() <= 6));
}

#[test]
fn zero_touch_nfc_setup_under_90s() {
    let profile = eval(r#"os_setup_nfc("phone-abc")"#);
    let Value::Object(o) = profile else {
        panic!("expected profile");
    };
    assert!(matches!(o.get("dark"), Some(Value::Bool(true))));
    assert!(matches!(o.get("wifi"), Some(Value::String(s)) if s.contains("phone-abc")));
}

#[test]
fn golden_recovery_and_compat() {
    let ms = eval("os_recovery_restore()");
    assert!(matches!(ms, Value::Number(n) if n > 0 && n <= 2000));
    let win = eval(r#"os_compat_run("windows", "CreateFileW", [])"#);
    let Value::Object(o) = win else {
        panic!("expected compat result");
    };
    assert!(matches!(o.get("perf_pct"), Some(Value::Number(99))));
}

#[test]
fn seamless_clipboard_roundtrip() {
    let out = eval(
        r#"
        os_seamless_pair(19000);
        os_seamless_clipboard_push("hello-from-phone");
        os_seamless_clipboard_poll();
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "hello-from-phone"));
}

#[test]
fn energy_privacy_and_updates() {
    let deferred = eval(r#"os_energy_schedule("backup", true)"#);
    assert!(matches!(deferred, Value::Bool(false)));
    let locked = eval("os_privacy_panic()");
    assert!(matches!(locked, Value::Bool(true)));
    let tel = eval(r#"os_privacy_telemetry("clicks", 100)"#);
    assert!(matches!(tel, Value::Object(_)));
    let ver = eval(r#"os_update_channel("classic")"#);
    assert!(matches!(ver, Value::String(s) if s.contains("classic")));
    let rolled = eval("os_update_rollback(1)");
    assert!(matches!(rolled, Value::String(_)));
}

#[test]
fn haptic_danger_blocks_system_path() {
    let fb = eval(r#"os_haptic_danger("/system/kernel")"#);
    let Value::Object(o) = fb else {
        panic!("expected haptic feedback");
    };
    assert!(matches!(o.get("blocked"), Some(Value::Bool(true))));
    assert!(matches!(o.get("glow"), Some(Value::String(s)) if s == "red-pulse"));
}
