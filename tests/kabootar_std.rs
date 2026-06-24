//! Standard library — JSON, Map/Set, array/string APIs, regex, type checks.

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::modules::import_module;
use kabootar::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn json_roundtrip() {
    let out = eval(
        r#"
        let raw = `{"x":1,"tags":["a","b"]}`
        let o = json_parse(raw)
        json_stringify(o)
        "#,
    );
    let Value::String(s) = out else {
        panic!("expected string");
    };
    assert!(s.contains("\"x\":1"));
    assert!(s.contains("\"tags\""));
}

#[test]
fn reduce_sum() {
    let out = eval(
        r#"
        fn add(acc, n) { return acc + n }
        reduce([1, 2, 3, 4], add, 0)
        "#,
    );
    assert!(matches!(out, Value::Number(10)));
}

#[test]
fn find_and_slice() {
    let out = eval(
        r#"
        let xs = [10, 20, 30, 40]
        fn gt25(x) { return x > 25 }
        let hit = find(xs, gt25)
        slice(xs, 1, 3)
        "#,
    );
    assert!(matches!(
        out,
        Value::Array(items) if items.len() == 2
            && matches!(&items[0], Value::Number(20))
            && matches!(&items[1], Value::Number(30))
    ));
}

#[test]
fn map_and_set_collections() {
    let out = eval(
        r#"
        let m = map_new();
        map_set(m, "port", 8080);
        let p = map_get(m, "port");
        let s = set_new();
        set_add(s, "kab");
        set_add(s, "kab");
        let n = set_size(s);
        p + n;
        "#,
    );
    assert!(matches!(out, Value::Number(8081)));
}

#[test]
fn map_values_and_set_delete() {
    let out = eval(
        r#"
        let m = map_new();
        map_set(m, "a", 10);
        map_set(m, "b", 20);
        let vs = map_values(m);
        let s = set_new();
        set_add(s, "x");
        set_add(s, "y");
        set_delete(s, "x");
        len(vs) + set_size(s);
        "#,
    );
    assert!(matches!(out, Value::Number(3)));
}

#[test]
fn typeof_map_set() {
    let out = eval(
        r#"
        typeof(map_new()) + "|" + typeof(set_new());
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "map|set"));
}

#[test]
fn regex_and_strings() {
    let out = eval(
        r#"
        let ok = regex_test("^kab.*tar$", "kabootar")
        let t = trim("  hi  ")
        ok && t == "hi"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn entries_values_objects() {
    let out = eval(
        r#"
        let o = { a: 1, b: 2 };
        len(entries(o)) + len(values(o));
        "#,
    );
    assert!(matches!(out, Value::Number(4)));
}

#[test]
fn type_assert_passes() {
    let out = eval(r#"type_assert("ok", "string")"#);
    assert!(matches!(out, Value::String(s) if s == "ok"));
}

#[test]
fn std_info_lists_capabilities() {
    let out = eval("std_info()");
    let Value::Object(info) = out else {
        panic!("expected object");
    };
    let caps = info.get("capabilities").expect("capabilities");
    let Value::Array(items) = caps else {
        panic!("expected array");
    };
    assert!(items.len() >= 10);
}

#[test]
fn math_and_parse() {
    let out = eval(
        r#"
        floor(3.9) + parse_int("42") + max(1, 5, 3)
        "#,
    );
    assert!(matches!(out, Value::Number(50)));
}

#[test]
fn sort_and_join() {
    let out = eval(
        r#"
        let xs = [3, 1, 2]
        join(sort(xs), "-")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "1-2-3"));
}

#[test]
fn object_assign_and_has_key() {
    let out = eval(
        r#"
        let a = { x: 1 }
        let b = assign(a, { y: 2 })
        has_key(b, "y") && b.x == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn import_std_module() {
    let mut env = create_global_env();
    import_module("std", &mut env).unwrap();
    let out = eval_source(r#"stringify({ ok: true })"#, &mut env).unwrap();
    let Value::String(s) = out else {
        panic!("expected string");
    };
    assert!(s.contains("\"ok\":true"));
}
