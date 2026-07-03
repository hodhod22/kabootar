//! Honesty API — marketing vs reality tiers.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn reality_reports_not_self_hosting() {
    let out = eval("kabootar_reality()");
    let Value::Object(r) = out else {
        panic!("expected object");
    };
    assert!(matches!(r.get("self_hosting"), Some(Value::Bool(false))));
    assert!(matches!(
        r.get("compiler_host"),
        Some(Value::String(s)) if s == "rust"
    ));
}

#[test]
fn reality_lists_stdlib_as_native() {
    let out = eval(r#"feature_tier("stdlib")"#);
    let Value::Object(f) = out else {
        panic!("expected object");
    };
    assert!(matches!(f.get("tier"), Some(Value::String(s)) if s == "native"));
}

#[test]
fn reality_marks_compat_as_stub() {
    let out = eval(r#"feature_tier("os_sauce_compat")"#);
    let Value::Object(f) = out else {
        panic!("expected object");
    };
    assert!(matches!(f.get("tier"), Some(Value::String(s)) if s == "stub"));
}

#[test]
fn os_sauce_honesty_has_nine_strategies() {
    let out = eval("os_sauce_honesty()");
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    assert_eq!(items.len(), 9);
    let stub_count = items
        .iter()
        .filter(|v| {
            matches!(
                v,
                Value::Object(m) if matches!(m.get("tier"), Some(Value::String(s)) if s == "stub")
            )
        })
        .count();
    assert!(stub_count >= 1, "compat strategy should be stub");
}

#[test]
fn reality_includes_builtin_modules() {
    let out = eval("kabootar_reality()");
    let Value::Object(r) = out else {
        panic!("expected object");
    };
    let mods = r.get("builtin_modules").expect("builtin_modules");
    let Value::Array(items) = mods else {
        panic!("expected array");
    };
    assert!(items.iter().any(|v| matches!(v, Value::String(s) if s == "std")));
}
