//! JS parity batch 2 — language operators and stdlib.

use kabootar_lib::evaluator::{create_global_env, drain_all_microtasks, eval_source};
use kabootar_lib::value::{PromiseValue, Value};

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

fn eval_binding_after_drain(code: &str, name: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap();
    drain_all_microtasks(&mut env).unwrap();
    env.get(name).unwrap_or(Value::Undefined)
}

fn eval_async(code: &str) -> Value {
    let mut env = create_global_env();
    let v = eval_source(code, &mut env).unwrap();
    drain_all_microtasks(&mut env).unwrap();
    v
}

#[test]
fn pow_operator() {
    let out = eval("2 ** 3");
    assert!(matches!(out, Value::Number(8)));
    let out = eval("2 ** 3 ** 2");
    assert!(matches!(out, Value::Number(512)));
}

#[test]
fn nullish_coalesce() {
    let out = eval("null ?? 42");
    assert!(matches!(out, Value::Number(42)));
    let out = eval("undefined ?? 7");
    assert!(matches!(out, Value::Number(7)));
    let out = eval("0 ?? 9");
    assert!(matches!(out, Value::Number(0) | Value::Float(0.0)));
}

#[test]
fn do_while_runs_at_least_once() {
    let out = eval(
        r#"
        let n = 0
        do { n = n + 1 } while (false)
        n
        "#,
    );
    assert!(matches!(out, Value::Number(1)));
}

#[test]
fn pad_and_repeat() {
    let out = eval(r#"pad_start("7", 3, "0")"#);
    assert!(matches!(out, Value::String(s) if s == "007"));
    let out = eval(r#"repeat("ab", 3)"#);
    assert!(matches!(out, Value::String(s) if s == "ababab"));
}

#[test]
fn flat_map_flattens_one_level() {
    let out = eval(
        r#"
        fn dbl(x) { return [x, x] }
        flat_map([1, 2], dbl)
        "#,
    );
    assert!(matches!(
        out,
        Value::Array(items) if items.len() == 4
            && matches!(&items[0], Value::Number(1))
            && matches!(&items[3], Value::Number(2))
    ));
}

#[test]
fn array_is_array_alias() {
    let out = eval("array_is_array([1])");
    assert!(matches!(out, Value::Bool(true)));
    let out = eval("array_is_array(1)");
    assert!(matches!(out, Value::Bool(false)));
}

#[test]
fn encode_decode_uri_roundtrip() {
    let out = eval(r#"decode_uri(encode_uri("a b/c?d"))"#);
    assert!(matches!(out, Value::String(s) if s == "a b/c?d"));
    let out = eval(r#"decode_uri_component(encode_uri_component("a=b&c"))"#);
    assert!(matches!(out, Value::String(s) if s == "a=b&c"));
}

#[test]
fn date_format_fields() {
    let out = eval("date_format(0)");
    let Value::Object(map) = out else {
        panic!("expected object");
    };
    assert!(matches!(map.get("year"), Some(Value::Number(1970))));
    assert!(matches!(map.get("month"), Some(Value::Number(1))));
    assert!(matches!(map.get("day"), Some(Value::Number(1))));
}

#[test]
fn sleep_ms_resolves_promise() {
    let out = eval_async(
        r#"
        async fn main() {
            await sleep_ms(2)
            return 99
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(99))));
}

#[test]
fn set_timeout_runs_callback() {
    let out = eval_async(
        r#"
        let state = { n: 0 }
        async fn main() {
            fn bump() { state.n = state.n + 1 }
            set_timeout(bump, 1)
            await sleep_ms(1)
            return state.n
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(1))));
}

#[test]
fn for_in_yields_indices() {
    let out = eval(
        r#"
        let xs = [100, 200]
        let sum = 0
        for i in xs { sum = sum + xs[i] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(300)));
}

#[test]
fn for_const_of_reads_values() {
    let out = eval(
        r#"
        let hit = 0
        for const x of [1, 2] { hit = hit + x }
        hit
        "#,
    );
    assert!(matches!(out, Value::Number(3)));
}

#[test]
fn set_interval_fires_repeatedly() {
    let out = eval_async(
        r#"
        let state = { n: 0 }
        async fn main() {
            fn bump() { state.n = state.n + 1 }
            let id = set_interval_ticks(bump, 1)
            await sleep_ticks(3)
            clear_interval(id)
            return state.n
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let resolved = p.borrow().clone();
    match resolved {
        PromiseValue::Resolved(Value::Number(n)) => assert!(n >= 2),
        other => panic!("expected resolved count >= 2, got {other:?}"),
    }
}

#[test]
fn object_values_entries_aliases() {
    let out = eval(r#"object_values({ a: 1, b: 2 })"#);
    assert!(matches!(out, Value::Array(items) if items.len() == 2));
    let out = eval(r#"object_entries({ a: 1 })"#);
    assert!(matches!(out, Value::Array(items) if items.len() == 1));
}

#[test]
fn for_of_iterates_values() {
    let out = eval(
        r#"
        let sum = 0
        for x of [1, 2, 3] { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn switch_statement() {
    let out = eval(
        r#"
        let n = 2
        switch (n) {
            case 1: { 10 }
            case 2: { 20 }
            default: { 0 }
        }
        "#,
    );
    assert!(matches!(out, Value::Number(20)));
}

#[test]
fn object_keys_alias_and_array_keys() {
    let out = eval(r#"object_keys({ a: 1, b: 2 })"#);
    assert!(matches!(out, Value::Array(items) if items.len() == 2));
    let out = eval(r#"object_keys([10, 20])"#);
    assert!(matches!(
        out,
        Value::Array(items) if items.len() == 2
            && matches!(&items[0], Value::String(s) if s == "0")
    ));
}

#[test]
fn string_and_array_batch3() {
    let out = eval(r#"char_at("abc", 1)"#);
    assert!(matches!(out, Value::String(s) if s == "b"));
    let out = eval(r#"str_includes("hello", "ell")"#);
    assert!(matches!(out, Value::Bool(true)));
    let out = eval(r#"last_index_of([1, 2, 1], 1)"#);
    assert!(matches!(out, Value::Number(2)));
}

#[test]
fn bitwise_operators() {
    let out = eval("5 & 3");
    assert!(matches!(out, Value::Number(1)));
    let out = eval("5 | 2");
    assert!(matches!(out, Value::Number(7)));
    let out = eval("5 ^ 3");
    assert!(matches!(out, Value::Number(6)));
    let out = eval("~0");
    assert!(matches!(out, Value::Number(-1)));
    let out = eval("1 << 3");
    assert!(matches!(out, Value::Number(8)));
    let out = eval("8 >> 2");
    assert!(matches!(out, Value::Number(2)));
}

#[test]
fn unsigned_right_shift() {
    let out = eval("8 >>> 2");
    assert!(matches!(out, Value::Number(2)));
    // 0xFFFFFFFF >>> 1 — same bit pattern as (-1) >>> 1 in JS
    let out = eval("4294967295 >>> 1");
    assert!(matches!(out, Value::Number(2147483647)));
}

#[test]
fn switch_fallthrough() {
    let out = eval(
        r#"
        let n = 1
        switch (n) {
            case 1: {
                let x = 10
                fallthrough
            }
            case 2: { 20 }
            default: { 0 }
        }
        "#,
    );
    assert!(matches!(out, Value::Number(20)));
}

#[test]
fn remaining_stdlib_batch() {
    let out = eval(r#"str_slice("hello", 1, 4)"#);
    assert!(matches!(out, Value::String(s) if s == "ell"));
    let out = eval(r#"string_includes("kabootar", "boot")"#);
    assert!(matches!(out, Value::Bool(true)));
    let out = eval(r#"at([10, 20, 30], -1)"#);
    assert!(matches!(out, Value::Number(30)));
    let out = eval(r#"fill([1, 2, 3], 0, 1, 3)"#);
    assert!(matches!(
        out,
        Value::Array(items) if items.len() == 3
            && matches!(&items[0], Value::Number(1))
            && matches!(&items[1], Value::Number(0))
            && matches!(&items[2], Value::Number(0))
    ));
    let out = eval(r#"copy_within([1, 2, 3, 4], 0, 2)"#);
    assert!(matches!(
        out,
        Value::Array(items) if matches!(&items[0], Value::Number(3))
            && matches!(&items[1], Value::Number(4))
    ));
    let out = eval(r#"to_spliced([1, 2, 3], 1, 1, 9)"#);
    assert!(matches!(
        out,
        Value::Array(items) if items.len() == 3
            && matches!(&items[0], Value::Number(1))
            && matches!(&items[1], Value::Number(9))
            && matches!(&items[2], Value::Number(3))
    ));
    let out = eval("to_fixed(1.2345, 2)");
    assert!(matches!(out, Value::String(s) if s == "1.23"));
    let out = eval("date_iso(0)");
    assert!(matches!(out, Value::String(s) if s == "1970-01-01T00:00:00.000Z"));
    let out = eval(r#"object_assign({ a: 1 }, { b: 2 })"#);
    let Value::Object(map) = out else { panic!("expected object") };
    assert!(map.contains_key("a") && map.contains_key("b"));
    let out = eval(r#"object_has_own({ x: 1 }, "x")"#);
    assert!(matches!(out, Value::Bool(true)));
    let out = eval(r#"regex_search("foo", "xfooy")"#);
    assert!(matches!(out, Value::Number(1)));
}

#[test]
fn js_parity_batch5() {
    let out = eval(r#"array_from("abc")"#);
    assert!(matches!(out, Value::Array(items) if items.len() == 3));
    let out = eval(r#"array_with([1, 2, 3], 1, 9)"#);
    assert!(matches!(
        out,
        Value::Array(items) if matches!(&items[1], Value::Number(9))
    ));
    let out = eval(r#"object_from_entries([["a", 1], ["b", 2]])"#);
    let Value::Object(map) = out else { panic!("expected object") };
    assert!(map.contains_key("a"));
    let out = eval(r#"structured_clone({ x: [1, 2] })"#);
    let Value::Object(map) = out else { panic!("expected object") };
    let Value::Array(inner) = map.get("x").unwrap() else { panic!("expected array") };
    assert_eq!(inner.len(), 2);
    assert!(matches!(eval("is_integer(5)"), Value::Bool(true)));
    let out = eval("is_integer(5.0)");
    assert!(matches!(out, Value::Bool(true)));
    let out = eval("sin(0)");
    assert!(matches!(out, Value::Number(0) | Value::Float(0.0)));
    let out = eval("char_code_at(\"A\", 0)");
    assert!(matches!(out, Value::Number(65)));
    let out = eval(r#"from_char_code(65, 66)"#);
    assert!(matches!(out, Value::String(s) if s == "AB"));
    let out = eval(r#"regex_replace_all("a", "banana", "o")"#);
    assert!(matches!(out, Value::String(s) if s == "bonono"));
    let out = eval(r#"json_stringify({ a: 1 }, 2)"#);
    assert!(matches!(out, Value::String(s) if s.contains("\n")));
    let out = eval(
        r#"
        switch (2) {
            case 1: case 2: { 42 }
            default: { 0 }
        }
        "#,
    );
    assert!(matches!(out, Value::Number(42)));
}

#[test]
fn js_parity_batch6() {
    let out = eval("atan(1)");
    assert!(matches!(out, Value::Float(f) if (f - std::f64::consts::FRAC_PI_4).abs() < 1e-10));
    let out = eval("atan2(1, 1)");
    assert!(matches!(out, Value::Float(f) if (f - std::f64::consts::FRAC_PI_4).abs() < 1e-10));
    let out = eval("asin(0)");
    assert!(matches!(out, Value::Number(0) | Value::Float(0.0)));
    let out = eval("acos(1)");
    assert!(matches!(out, Value::Number(0) | Value::Float(0.0)));

    let out = eval(
        r#"
        let m = map_new();
        map_set(m, "a", 1);
        map_set(m, "b", 2);
        let vs = map_values(m);
        map_size(m) + len(vs);
        "#,
    );
    assert!(matches!(out, Value::Number(4)));

    let out = eval(
        r#"
        let m = map_new();
        map_set(m, "x", 10);
        map_has(m, "x") && !map_delete(m, "y") && map_delete(m, "x") && map_size(m);
        "#,
    );
    assert!(matches!(out, Value::Number(0)));

    let out = eval(
        r#"
        let s = set_new();
        set_add(s, 1);
        set_add(s, 2);
        set_add(s, 1);
        let n = set_size(s);
        set_delete(s, 1);
        n + set_size(s);
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let m = map_new();
        map_set(m, "k", 5);
        map_for_each(m, (v, k) => { null });
        at(map_values(m), 0);
        "#,
    );
    assert!(matches!(out, Value::Number(5)));

    let out = eval(
        r#"
        let s = set_new();
        set_add(s, 10);
        set_add(s, 20);
        set_for_each(s, (v) => { null });
        len(set_values(s));
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    use kabootar_lib::bytecode::can_compile;
    assert!(can_compile("5 & 3"));
    assert!(can_compile("4294967295 >>> 1"));
    assert!(can_compile(
        r#"
        switch (1) {
            case 1: {
                fallthrough
            }
            default: { 2 }
        }
        "#,
    ));
    assert!(can_compile("do { 1 } while (false)"));
}

#[test]
fn js_parity_batch7() {
    let out = eval("is_promise(promise_resolve(42))");
    assert!(matches!(out, Value::Bool(true)));

    let out = eval_async(
        r#"
        async fn main() {
            let xs = await promise_all([promise_resolve(1), 2, 3])
            return xs[0] + xs[1] + xs[2]
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));

    let out = eval_async(
        r#"
        async fn main() {
            let xs = await promise_all_settled([promise_resolve(10), promise_reject("nope")])
            let ok = xs[0].status == "fulfilled" && xs[0].value == 10
            let bad = xs[1].status == "rejected" && xs[1].reason == "nope"
            return ok && bad
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Bool(true))));

    let out = eval_async(
        r#"
        async fn main() {
            return await promise_any([promise_reject("x"), promise_resolve(7)])
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(7))));

    let out = eval_async(
        r#"
        async fn main() {
            return await promise_then(promise_resolve(5), (n) => n * 2)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(10))));

    let out = eval(
        r#"
        let m = map_from_entries([["port", 8080], ["host", "local"]])
        map_get(m, "port")
        "#,
    );
    assert!(matches!(out, Value::Number(8080)));

    assert!(matches!(eval("is_safe_integer(9007199254740991)"), Value::Bool(true)));
    assert!(matches!(eval("fmod(7, 3)"), Value::Number(1)));
}

#[test]
fn js_parity_batch8() {
    let out = eval(
        r#"
        reduce_right([1, 2, 3], (acc, x) => acc - x, 10)
        "#,
    );
    assert!(matches!(out, Value::Number(4)));

    let out = eval("to_reversed([1, 2, 3])[0]");
    assert!(matches!(out, Value::Number(3)));

    let out = eval("to_sorted([3, 1, 2])[0]");
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        let a = set_new()
        set_add(a, 1)
        set_add(a, 2)
        let b = set_new()
        set_add(b, 2)
        set_add(b, 3)
        set_size(set_union(a, b))
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let a = set_new()
        set_add(a, 1)
        set_add(a, 2)
        let b = set_new()
        set_add(b, 2)
        set_add(b, 3)
        set_has(set_intersection(a, b), 2)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let a = set_new()
        set_add(a, 1)
        set_add(a, 2)
        let b = set_new()
        set_add(b, 2)
        set_size(set_difference(a, b))
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    assert!(matches!(eval("imul(2, 3)"), Value::Number(6)));
    assert!(matches!(eval("clz32(1)"), Value::Number(31)));
    // G4 Math rest
    match eval("f16round(1.5)") {
        Value::Float(f) => assert!((f - 1.5).abs() < 1e-12),
        Value::Number(n) => assert_eq!(n, 1), // num_out may fold 1.5? unlikely; 1.5 stays float
        other => panic!("f16round(1.5)={other:?}"),
    }
    match eval("f16round(1.337)") {
        Value::Float(f) => assert!((f - 1.3369140625).abs() < 1e-9, "f16round(1.337)={f}"),
        Value::Number(n) => assert!((n as f64 - 1.3369140625).abs() < 1e-9),
        other => panic!("f16round(1.337)={other:?}"),
    }
    match eval("sumPrecise([10000000000000000.0, 1.0, -10000000000000000.0])") {
        Value::Float(f) => assert!((f - 1.0).abs() < 1e-9, "sumPrecise={f}"),
        Value::Number(n) => assert_eq!(n, 1, "sumPrecise number={n}"),
        other => panic!("sumPrecise={other:?}"),
    }
    assert!(matches!(
        eval("sumPrecise([])"),
        Value::Float(f) if f == 0.0 && f.is_sign_negative()
    ));
    assert!(matches!(eval("number_to_string(42)"), Value::String(s) if s == "42"));
    assert!(matches!(
        eval(r#"string_split("a,b", ",")[1]"#),
        Value::String(s) if s == "b"
    ));
}

#[test]
fn js_parity_batch9() {
    let out = eval("array_flat([[1, 2], 3])[0]");
    assert!(matches!(out, Value::Number(1)));

    let out = eval("array_includes([1, 2, 3], 2)");
    assert!(matches!(out, Value::Bool(true)));

    let out = eval("array_find_index([10, 20, 30], (x) => x == 20)");
    assert!(matches!(out, Value::Number(1)));

    let out = eval(r#"object_has({ a: 1 }, "a")"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(r#"object_is(NaN, NaN)"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let o = { x: 1 }
        let copy = object_delete(o, "x")
        object_has(copy, "x")
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let big = set_new()
        set_add(big, 1)
        set_add(big, 2)
        set_add(big, 3)
        let small = set_new()
        set_add(small, 2)
        set_is_superset(big, small)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let a = set_new()
        set_add(a, 1)
        let b = set_new()
        set_add(b, 2)
        set_is_disjoint(a, b)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_parity_batch10() {
    let out = eval("array_map([1, 2, 3], (x) => x * 2)[1]");
    assert!(matches!(out, Value::Number(4)));

    let out = eval("array_of(1, 2, 3)[2]");
    assert!(matches!(out, Value::Number(3)));

    let out = eval(r#"code_point_at("A", 0)"#);
    assert!(matches!(out, Value::Number(65)));

    let out = eval(
        r#"
        let a = set_new()
        set_add(a, 1)
        let b = set_new()
        set_add(b, 2)
        set_size(set_symmetric_difference(a, b))
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval(r#"{ x: 1 }?.x"#);
    assert!(matches!(out, Value::Number(1)));

    let out = eval(r#"null?.missing"#);
    assert!(matches!(out, Value::Undefined));

    let out = eval(
        r#"
        let o = { z: 1 }
        delete o.z
        object_has(o, "z")
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let d = date_new(0)
        date_get_full_year(d)
        "#,
    );
    assert!(matches!(out, Value::Number(1970)));

    let out = eval(r#"object_is_frozen(object_freeze({ a: 1 }))"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval("log1p(0)");
    assert!(matches!(out, Value::Number(0)));
}

#[test]
fn js_parity_batch11() {
    let out = eval(
        r#"
        let g = object_group_by([1.2, 2.1, 1.5], (x) => floor(x))
        len(g["1"])
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval(
        r#"
        let m = map_group_by(["a", "ab", "b"], (s) => string_slice(s, 0, 1))
        map_size(m)
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval(
        r#"
        let pr = promise_with_resolvers()
        pr.resolve(9)
        pr.promise
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(9))));

    let out = eval(
        r#"
        let s = set_new()
        set_add(s, 10)
        set_add(s, 20)
        let sum = 0
        for x of s { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(30)));

    let out = eval(
        r#"
        let m = map_new()
        map_set(m, "k", 7)
        let got = 0
        for pair of m { got = pair[1] }
        got
        "#,
    );
    assert!(matches!(out, Value::Number(7)));

    let out = eval(
        r#"
        let ac = abort_controller_new()
        abort_controller_abort(ac, "nope")
        ac.signal.aborted
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_parity_batch12() {
    let out = eval(
        r#"
        try {
            throw error_new("boom")
        } catch (e) {
            error_message(e)
        }
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "boom"));

    let out = eval(
        r#"
        try {
            throw type_error("bad type")
        } catch (e) {
            e.name
        }
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "TypeError"));

    let out = eval(r#"is_error(error_new("x"))"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn boom() { throw error_new("nope") }
        promise_try(boom)
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    match p.borrow().clone() {
        PromiseValue::Resolved(Value::Result(Err(reason))) => {
            assert!(matches!(*reason, Value::Object(_)));
        }
        other => panic!("expected rejected promise, got {:?}", other),
    }

    let out = eval(
        r#"
        let g = group_by([1, 2, 3], (x) => x % 2)
        len(g["1"])
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval("len(iterator_from([10, 20, 30]))");
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let u = url_new("https://example.com/path?foo=bar&baz=1#frag")
        u.hostname
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "example.com"));

    let out = eval(
        r#"
        let u = url_new("https://example.com/?x=1")
        usp_get(u.searchParams, "x")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "1"));

    let out = eval(
        r#"
        let usp = url_search_params_new("a=1&b=2")
        usp_set(usp, "c", "3")
        usp_to_string(usp)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s.contains("c=3")));
}

#[test]
fn js_parity_batch13() {
    let out = eval(
        r#"
        /* block comment */
        let n = 0
        try {
            n = 1
        } catch (e) {
            n = 2
        } finally {
            n = n + 10
        }
        n
        "#,
    );
    assert!(matches!(out, Value::Number(11)));

    let out = eval(
        r#"
        let caught = false
        try {
            throw error_new("x")
        } catch (e) {
            caught = true
        } finally {
            caught = true
        }
        caught
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn sum(a, b = 10, ...rest) {
            let total = a + b
            for x of rest { total = total + x }
            return total
        }
        sum(1, 2, 3, 4)
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        fn only_default(x = 5) { x }
        only_default()
        "#,
    );
    assert!(matches!(out, Value::Number(5)));

    let out = eval(r#"len(text_encode("hi"))"#);
    assert!(matches!(out, Value::Number(2)));

    let out = eval(
        r#"
        let bytes = text_encode("ab")
        text_decode(bytes)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "ab"));

    let out = eval(r#"len(globalThis())"#);
    assert!(matches!(out, Value::Number(n) if n > 10));

    let out = eval(
        r#"
        let sum = 0
        for x of iterator_filter([1, 2, 3, 4], (x) => x % 2 == 0) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        let sum = 0
        for x of iterator_map([2, 3], (x) => x * 2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(r#"regex_escape("a.b")"#);
    assert!(matches!(out, Value::String(s) if s == "a\\.b"));

    let out = eval(
        r#"
        try {
            throw reference_error("bad ref")
        } catch (e) {
            e.name
        }
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "ReferenceError"));
}

#[test]
fn js_parity_promises_complete() {
    let out = eval(
        r#"
        fn go(res, rej) { res(21) }
        promise_new(go)
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(21))));

    let out = eval(
        r#"
        promise_new((resolve, reject) => { resolve(99) })
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(99))));

    let out = eval_async(
        r#"
        async fn main() {
            fn go(res, rej) {
                set_timeout(() => { res(9) }, 1)
            }
            return await promise_new(go)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(9))));

    let err = {
        let mut env = create_global_env();
        eval_source(
            r#"
            async fn main() {
                return await promise_reject("boom")
            }
            main()
            "#,
            &mut env,
        )
    };
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("boom"));

    let out = eval_async(
        r#"
        async fn main() {
            return await promise_catch(promise_reject("e"), (r) => "ok-" + r)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        p.borrow().clone(),
        PromiseValue::Resolved(Value::String(s)) if s == "ok-e"
    ));

    let out = eval_async(
        r#"
        async fn main() {
            let p = promise_finally(promise_resolve(1), () => { null })
            return await p
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(1))));

    let out = eval_async(
        r#"
        async fn main() {
            fn go(res, rej) {
                let p1 = promise_resolve(2)
                res(p1)
            }
            return await promise_then(promise_new(go), (n) => n + 3)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(5))));

    let out = eval_async(
        r#"
        async fn main() {
            return await promise_all([1, promise_resolve(2)])
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        p.borrow().clone(),
        PromiseValue::Resolved(Value::Array(items)) if items.len() == 2
    ));
}

#[test]
fn object_descriptors_phase2() {
    let out = eval(
        r#"
        let o = {}
        Object.defineProperty(o, "x", { value: 10, writable: false, enumerable: true })
        o.x
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        let o = { a: 1, b: 2, hidden: 3 }
        Object.defineProperty(o, "hidden", { value: 3, enumerable: false })
        len(Object.keys(o))
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval(
        r#"
        let o = { a: 1, hidden: 2 }
        Object.defineProperty(o, "hidden", { value: 2, enumerable: false })
        len(Object.getOwnPropertyNames(o))
        "#,
    );
    assert!(matches!(out, Value::Number(2)));

    let out = eval(
        r#"
        let o = {}
        Object.defineProperty(o, "n", {
            get: () => 99,
            enumerable: true,
            configurable: true
        })
        o.n
        "#,
    );
    assert!(matches!(out, Value::Number(99)));

    let out = eval(
        r#"
        let o = {}
        let desc = Object.getOwnPropertyDescriptor(
            Object.defineProperty(o, "k", { value: 7, writable: true }),
            "k"
        )
        desc.value
        "#,
    );
    assert!(matches!(out, Value::Number(7)));

    let out = eval(
        r#"
        let o = Object.defineProperty({ z: 1 }, "z", { writable: false })
        o.z
        "#,
    );
    assert!(matches!(out, Value::Number(1)));
}

#[test]
fn object_namespace_phase1() {
    let out = eval(r#"Object.assign({ a: 1 }, { b: 2 })["b"]"#);
    assert!(matches!(out, Value::Number(2)));

    let out = eval(r#"len(Object.getOwnPropertyNames({ z: 1, a: 2 }))"#);
    assert!(matches!(out, Value::Number(2)));

    let out = eval(r#"Object.isExtensible({})"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"Object.isExtensible(Object.preventExtensions({}))"#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let proto = { inherited: 42 }
        let o = Object.create(proto)
        o.inherited
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        let p = { x: 1 }
        let o = Object.create(p)
        object_get_parent(o)["x"]
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(r#"Object.hasOwn(Object.create({ a: 1 }), "a")"#);
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(r#"Object.is(NaN, NaN)"#);
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn symbol_phase3() {
    let out = eval(r#"typeof(Symbol("x"))"#);
    assert!(matches!(out, Value::String(s) if s == "symbol"));

    let out = eval(
        r#"
        let a = Symbol("a")
        let b = Symbol("a")
        a == b
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let s = Symbol("key")
        let o = {}
        o[s] = 42
        o[s]
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        let o = {}
        o[Symbol.for("shared")] = 7
        Object.getOwnPropertySymbols(o).length
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(r#"Symbol.for("k") == Symbol.for("k")"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(r#"Symbol.iterator != Symbol.asyncIterator"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn iter() { return [1, 2, 3] }
        let sum = 0
        let o = {}
        o[Symbol.iterator] = iter
        for x of o { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn proxy_reflect_phase4() {
    let out = eval(
        r#"
        let target = { x: 10, y: 20 }
        let p = Proxy(target, {})
        p.x + p.y
        "#,
    );
    assert!(matches!(out, Value::Number(30)));

    let out = eval(
        r#"
        let target = { msg: "hi" }
        fn get_trap(t, k, r) { return "got:" + k }
        let p = Proxy(target, { get: get_trap })
        p.msg
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "got:msg"));

    let out = eval(
        r#"
        let target = { n: 0 }
        let p = Proxy(target, {})
        p.n = 7
        p.n
        "#,
    );
    assert!(matches!(out, Value::Number(7)));

    let out = eval(
        r#"
        let target = { a: 1 }
        let p = Proxy(target, {})
        Reflect.has(p, "a") && Reflect.get(p, "a") == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let target = {}
        fn has_trap(t, k) { return k == "magic" }
        let p = Proxy(target, { has: has_trap })
        "magic" in p && !("other" in p)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let target = { a: 1, b: 2 }
        fn keys_trap(t) { return ["z"] }
        let p = Proxy(target, { ownKeys: keys_trap })
        Reflect.ownKeys(p).length
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        fn add(a, b) { return a + b }
        let p = Proxy(add, {})
        p(2, 3)
        "#,
    );
    assert!(matches!(out, Value::Number(5)));
}

#[test]
fn weakref_finreg_phase5() {
    let out = eval(
        r#"
        let o = { x: 42 }
        let w = WeakRef(o)
        w.deref().x
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        let o = { n: 1 }
        let w = WeakRef(o)
        o = null
        typeof(w.deref())
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "undefined"));

    let out = eval_binding_after_drain(
        r#"
        let fr = FinalizationRegistry((v) => { return v })
        let o = {}
        fr.register(o, "marker")
        o = null
        "#,
        "o",
    );
    assert!(matches!(out, Value::Null));

    let out = eval_binding_after_drain(
        r#"
        let held = undefined
        let fr = FinalizationRegistry((v) => { held = v })
        let o = {}
        fr.register(o, "marker")
        o = null
        "#,
        "held",
    );
    assert!(matches!(out, Value::String(s) if s == "marker"));

    let out = eval_binding_after_drain(
        r#"
        let fr = FinalizationRegistry((v) => { return v })
        let o = {}
        fr.register(o, 99, o)
        fr.unregister(o)
        let cleared = false
        let o2 = {}
        fr.register(o2, 1)
        o2 = null
        cleared
        "#,
        "cleared",
    );
    assert!(matches!(out, Value::Bool(false)));
}

#[test]
fn iterator_protocol_phase6() {
    let out = eval(
        r#"
        fn endNext() {
          return { value: null, done: true }
        }
        fn mkEnd() {
          let it = {}
          it.next = endNext
          return it
        }
        let o = {}
        o[Symbol.iterator] = mkEnd
        let n = 0
        for x of o { n = n + 1 }
        n
        "#,
    );
    assert!(matches!(out, Value::Number(0)));

    let out = eval(
        r#"
        fn oneNext() {
          return { value: 42, done: true }
        }
        fn mkOne() {
          let it = {}
          it.next = oneNext
          return it
        }
        let o = {}
        o[Symbol.iterator] = mkOne
        let sum = 0
        for x of o { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(0)));

    let out = eval(
        r#"
        fn iter() { return [4, 5, 6] }
        let o = {}
        o[Symbol.iterator] = iter
        let n = 0
        for x of o { n = n + 1 }
        n
        "#,
    );
    assert!(matches!(out, Value::Number(3)));
}

#[test]
fn iterator_param_object_writeback() {
    let out = eval(
        r#"
        fn stepNext(it) {
          it.n = it.n + 1
          return { value: it.n, done: false }
        }
        let it = { n: 0 }
        stepNext(it)
        it.n
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        fn stepNext(it) {
          return { value: it.n, done: false }
        }
        typeof(stepNext({ n: 1 }))
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "object"));
}

#[test]
fn iterator_protocol_param_state() {
    let out = eval(
        r#"
        fn stepNext(it) {
          if (it.n < 3) {
            it.n = it.n + 1
            return { value: it.n, done: false }
          }
          return { value: null, done: true }
        }
        fn mkIter() {
          let it = { n: 0 }
          it.next = stepNext
          return it
        }
        let o = {}
        o[Symbol.iterator] = mkIter
        let sum = 0
        for x of o { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn iterator_protocol_closure_counter() {
    let out = eval(
        r#"
        let i = 0
        fn stepNext() {
          if (i < 3) {
            i = i + 1
            return { value: i, done: false }
          }
          return { value: null, done: true }
        }
        fn mkIter() {
          let it = {}
          it.next = stepNext
          return it
        }
        let o = {}
        o[Symbol.iterator] = mkIter
        let sum = 0
        for x of o { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn iterator_protocol_phase7_builtins() {
    let out = eval(
        r#"
        let sum = 0
        for x of [1, 2, 3] { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        let it = [10, 20][Symbol.iterator]()
        let r = it.next()
        r.value
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        let s = ""
        for c of "ab" { s = s + c }
        s
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "ab"));

    let out = eval(
        r#"
        let m = map_new()
        map_set(m, "a", 1)
        map_set(m, "b", 2)
        let sum = 0
        for pair of m { sum = sum + pair[1] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let st = set_new()
        set_add(st, 5)
        set_add(st, 7)
        let sum = 0
        for v of st { sum = sum + v }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(12)));
}

#[test]
fn iterator_protocol_phase8() {
    let out = eval(
        r#"
        let it = {
          n: 0,
          next() {
            if (self.n < 3) {
              self.n = self.n + 1
              return { value: self.n, done: false }
            }
            return { value: null, done: true }
          }
        }
        let sum = 0
        for x of it { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval("len(iterator_from([10, 20, 30]))");
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let sum = 0
        for x of range(1, 4) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn iterator_protocol_phase9_generators() {
    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
          yield 3
        }
        let sum = 0
        for x of gen() { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        fn* gen() {
          yield 10
          return 99
        }
        let g = gen()
        let a = g.next()
        let b = g.next()
        a.value + b.value
        "#,
    );
    assert!(matches!(out, Value::Number(109)));

    let out = eval(
        r#"
        fn* counter() {
          let n = 0
          while (n < 3) {
            n = n + 1
            yield n
          }
        }
        let sum = 0
        for x of counter() { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));
}

#[test]
fn iterator_protocol_phase10_async() {
    let out = eval_async(
        r#"
        async fn main() {
          let sum = 0
          for await x of [1, 2, 3] { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));

    let out = eval_async(
        r#"
        async fn main() {
          fn* gen() {
            yield 10
            yield 20
          }
          let sum = 0
          for await x of gen() { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(30))));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            return {
              n: 0,
              next() {
                if (self.n < 3) {
                  self.n = self.n + 1
                  return promise_resolve({ value: self.n, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let sum = 0
          for await x of o { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));
}

#[test]
fn iterator_protocol_phase11_closure_and_async_gen() {
    let out = eval(
        r#"
        fn mk() {
          let i = 0
          return {
            next() {
              if (i < 3) {
                i = i + 1
                return { value: i, done: false }
              }
              return { value: null, done: true }
            }
          }
        }
        let sum = 0
        for x of mk() { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        fn mk() {
          let i = 0
          return {
            next() {
              if (i < 3) {
                i = i + 1
                return { value: i, done: false }
              }
              return { value: null, done: true }
            }
          }
        }
        let o = {}
        o[Symbol.iterator] = mk
        let sum = 0
        for x of o { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 3) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let sum = 0
          for await x of o { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            yield 10
            yield 20
          }
          let sum = 0
          for await x of gen() { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(30))));
}

#[test]
fn nested_fn_let_i_scoping_in_same_block() {
    let out = eval(
        r#"
        fn mkLeft() {
          let i = 0
          return {
            next() {
              if (i < 2) {
                i = i + 1
                return { value: i, done: false }
              }
              return { value: null, done: true }
            }
          }
        }
        fn mkRight() {
          let i = 9
          return {
            next() {
              if (i < 11) {
                i = i + 1
                return { value: i, done: false }
              }
              return { value: null, done: true }
            }
          }
        }
        let left = mkLeft()
        let right = mkRight()
        [
          [left.next().value, right.next().value],
          [left.next().value, right.next().value]
        ]
        "#,
    );
    let Value::Array(pairs) = out else {
        panic!("expected pairs array, got {:?}", out);
    };
    assert_eq!(pairs.len(), 2);
    let Value::Array(p0) = &pairs[0] else {
        panic!("expected pair");
    };
    assert!(matches!(p0[0], Value::Number(1)));
    assert!(matches!(p0[1], Value::Number(10)));
    let Value::Array(p1) = &pairs[1] else {
        panic!("expected pair");
    };
    assert!(matches!(p1[0], Value::Number(2)));
    assert!(matches!(p1[1], Value::Number(11)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkLeft() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          fn mkRight() {
            let i = 9
            return {
              next() {
                if (i < 11) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let a = {}
          a[Symbol.asyncIterator] = mkLeft
          let b = {}
          b[Symbol.asyncIterator] = mkRight
          return await AsyncIterator.zip(a, b).toArray()
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected zip pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 2);
    let Value::Array(z0) = &pairs[0] else {
        panic!("expected zip pair");
    };
    assert!(matches!(z0[0], Value::Number(1)));
    assert!(matches!(z0[1], Value::Number(10)));
}

#[test]
fn iterator_protocol_phase12_lazy() {
    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
          yield 3
        }
        let sum = 0
        for x of iterator_map(gen(), (n) => n * 10) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(60)));

    let out = eval(
        r#"
        let sum = 0
        for x of iterator_take(range(1, 100), 3) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval_async(
        r#"
        async fn main() {
          fn* gen() {
            yield 1
            yield 2
          }
          let sum = 0
          for await x of gen() { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(3))));
}

#[test]
fn iterator_protocol_phase13_lazy_skip_chain() {
    let out = eval(
        r#"
        for_of_items(iterator_skip(range(1, 5), 2))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(3)));
    assert!(matches!(items[1], Value::Number(4)));

    let out = eval(
        r#"
        let sum = 0
        for x of iterator_skip(range(1, 5), 2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(7)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
          yield 3
        }
        let sum = 0
        for x of iterator_skip(gen(), 2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let sum = 0
        for x of iterator_chain(range(1, 3), range(10, 12)) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(24)));

    let out = eval(
        r#"
        fn* gen(start) {
          yield start
          yield start + 1
        }
        let sum = 0
        for x of iterator_chain(gen(1), gen(10)) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(24)));
}

#[test]
fn iterator_protocol_phase14_lazy_zip_enumerate_map() {
    let out = eval(
        r#"
        for_of_items(iterator_zip(range(1, 4), range(10, 13)))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(&items[0], Value::Array(pair) if pair.len() == 2
        && matches!(pair[0], Value::Number(1)) && matches!(pair[1], Value::Number(10))));

    let out = eval(
        r#"
        let sum = 0
        for pair of iterator_zip(range(1, 3), range(100, 102)) { sum = sum + pair[0] + pair[1] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(204)));

    let out = eval(
        r#"
        for_of_items(iterator_enumerate([100, 200]))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], Value::Array(pair) if pair.len() == 2
        && matches!(pair[0], Value::Number(0)) && matches!(pair[1], Value::Number(100))));

    let out = eval(
        r#"
        fn* gen() {
          yield 10
          yield 20
        }
        let sum = 0
        for pair of iterator_enumerate(gen()) { sum = sum + pair[0] + pair[1] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(31)));

    let out = eval(
        r#"
        let m = map_new()
        map_set(m, "a", 1)
        map_set(m, "b", 2)
        let sum = 0
        for pair of m { sum = sum + pair[1] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        let it = iterator_from(range(1, 4))
        let r = it.next()
        r.value
        "#,
    );
    assert!(matches!(out, Value::Number(1)));
}

#[test]
fn iterator_protocol_phase15_classes_async() {
    let out = eval(
        r#"
        let sum = 0
        for x of Iterator.from(range(1, 5)).map((n) => n * 2).take(2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        Iterator.is(Iterator.from([1]))
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let sum = 0
        for pair of Iterator.zip(range(1, 3), range(10, 12)) { sum = sum + pair[0] + pair[1] }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(24)));

    let out = eval_async(
        r#"
        async fn main() {
          fn* gen() {
            yield 1
            yield 2
            yield 3
          }
          let sum = 0
          for await x of AsyncIterator.from(gen()).map((n) => n * 10).take(2) { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(30))));

    let out = eval_async(
        r#"
        async fn main() {
          let sum = 0
          for await x of AsyncIterator.from([1, 2, 3]) { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));
}

#[test]
fn iterator_protocol_phase16_flatmap_async_statics() {
    let out = eval(
        r#"
        for_of_items(iterator_flat_map(range(1, 3), (n) => [n, n * 10]))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(10)));
    assert!(matches!(items[2], Value::Number(2)));
    assert!(matches!(items[3], Value::Number(20)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
        }
        let sum = 0
        for x of iterator_flat_map(gen(), (n) => range(n, n + 2)) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(8)));

    let out = eval(
        r#"
        let sum = 0
        for x of Iterator.from(range(1, 3)).flatMap((n) => [n, n + 100]) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(206)));

    let out = eval(
        r#"
        let sum = 0
        for x of Iterator.flatMap(range(1, 3), (n) => n * 2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval_async(
        r#"
        async fn main() {
          fn* gen() {
            yield 1
            yield 2
          }
          let sum = 0
          for await x of AsyncIterator.flatMap(gen(), (n) => [n, n * 10]).take(3) { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(13))));

    let out = eval_async(
        r#"
        async fn main() {
          let sum = 0
          for await x of AsyncIterator.from([1, 2]).flatMap((n) => range(n, n + 2)) { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(8))));
}

#[test]
fn iterator_protocol_phase17_dropwhile_terminals_return() {
    let out = eval(
        r#"
        for_of_items(iterator_drop_while(range(1, 6), (n) => n < 3))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Number(3)));
    assert!(matches!(items[1], Value::Number(4)));
    assert!(matches!(items[2], Value::Number(5)));

    let out = eval(
        r#"
        Iterator.from(range(1, 4)).dropWhile((n) => n < 2).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(2)));
    assert!(matches!(items[1], Value::Number(3)));

    let out = eval(
        r#"
        Iterator.from(range(1, 5)).reduce((a, b) => a + b, 0)
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        Iterator.from(range(1, 5)).some((n) => n > 3)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        Iterator.from(range(1, 5)).every((n) => n < 10)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
          yield 3
        }
        let g = gen()
        g.next()
        let r = g.return(99)
        r.done
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
          yield 3
        }
        let g = gen()
        g.next()
        let r = g.return(99)
        r.value
        "#,
    );
    assert!(matches!(out, Value::Number(99)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
        }
        let g = gen()
        g.next()
        g.return(99)
        g.next().done
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval_async(
        r#"
        async fn main() {
          let items = await AsyncIterator.from(range(1, 6)).dropWhile((n) => n < 4).toArray()
          let sum = 0
          for x of items { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(9))));

    let out = eval_async(
        r#"
        async fn main() {
          return await AsyncIterator.from(range(1, 5)).reduce((a, b) => a + b, 0)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(10))));
}

#[test]
fn iterator_protocol_phase18_foreach_find_throw() {
    let out = eval(
        r#"
        let sum = 0
        iterator_for_each(range(1, 4), (n) => { sum = sum + n })
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        Iterator.from(range(1, 10)).forEach((n) => { })
        1
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        iterator_find(range(1, 10), (n) => n > 5)
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        Iterator.from(range(1, 10)).find((n) => n > 5)
        "#,
    );
    assert!(matches!(out, Value::Number(6)));

    let out = eval(
        r#"
        Iterator.from(range(1, 3)).find((n) => n > 10)
        "#,
    );
    assert!(matches!(out, Value::Undefined));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
        }
        let g = gen()
        g.next()
        let r = g.throw(99)
        r.value
        "#,
    );
    assert!(matches!(out, Value::Number(99)));

    let out = eval(
        r#"
        let it = Iterator.from(range(1, 10))
        it.next()
        let r = it.throw(42)
        r.done
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval_async(
        r#"
        async fn main() {
          let found = await AsyncIterator.from(range(1, 10)).find((n) => n > 7)
          return found
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(8))));

    let out = eval_async(
        r#"
        async fn main() {
          let sum = 0
          await AsyncIterator.from(range(1, 4)).forEach((n) => { sum = sum + n })
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));
}

#[test]
fn iterator_protocol_phase19_findindex_includes() {
    let out = eval(
        r#"
        iterator_find_index(range(1, 10), (n) => n > 5)
        "#,
    );
    assert!(matches!(out, Value::Number(5)));

    let out = eval(
        r#"
        Iterator.from(range(1, 10)).findIndex((n) => n > 5)
        "#,
    );
    assert!(matches!(out, Value::Number(5)));

    let out = eval(
        r#"
        Iterator.from(range(1, 3)).findIndex((n) => n > 10)
        "#,
    );
    assert!(matches!(out, Value::Number(-1)));

    let out = eval(
        r#"
        iterator_includes(range(1, 5), 3)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        Iterator.from(range(1, 5)).includes(99)
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        Iterator.from([1, 2, null]).includes(null)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval_async(
        r#"
        async fn main() {
          return await AsyncIterator.from(range(1, 10)).findIndex((n) => n > 7)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(7))));

    let out = eval_async(
        r#"
        async fn main() {
          return await AsyncIterator.from(range(1, 6)).includes(5)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Bool(true))));
}

#[test]
fn iterator_protocol_phase20_generator_throw_resume() {
    let out = eval(
        r#"
        fn* gen() {
          try {
            yield 1
            yield 2
          } catch (e) {
            yield e * 10
          }
          yield 3
        }
        let g = gen()
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        fn* gen() {
          try {
            yield 1
            yield 2
          } catch (e) {
            yield e * 10
          }
          yield 3
        }
        let g = gen()
        g.next()
        g.throw(99).value
        "#,
    );
    assert!(matches!(out, Value::Number(990)));

    let out = eval(
        r#"
        fn* gen() {
          try {
            yield 1
            yield 2
          } catch (e) {
            yield e * 10
          }
          yield 3
        }
        let g = gen()
        g.next()
        g.throw(99)
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(3)));

    let out = eval(
        r#"
        fn* gen() {
          let x = yield 1
          return x
        }
        let g = gen()
        g.next()
        g.next(42).value
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
        }
        let g = gen()
        g.next()
        g.throw(99).value
        "#,
    );
    assert!(matches!(out, Value::Number(99)));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
        }
        let g = gen()
        g.next()
        g.return(7).value
        "#,
    );
    assert!(matches!(out, Value::Number(7)));
}

#[test]
fn iterator_protocol_phase21_async_lazy_flatmap_depth() {
    let out = eval(
        r#"
        let sum = 0
        for x of Iterator.from([1]).flatMap((n) => [[n, [n + 10]]], 2) { sum = sum + x }
        sum
        "#,
    );
    assert!(matches!(out, Value::Number(12)));

    let out = eval(
        r#"
        for_of_items(iterator_flat_map(range(1, 3), (n) => [[n, n + 10]], 2))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array from materialized flatMap, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(11)));
    assert!(matches!(items[2], Value::Number(2)));
    assert!(matches!(items[3], Value::Number(12)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 3) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let items = await AsyncIterator.from(o).map((n) => n * 2).take(2).toArray()
          return items
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(2)));
    assert!(matches!(items[1], Value::Number(4)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let sum = 0
          for await x of AsyncIterator.from(o).flatMap((n) => [[n, n + 10]], 2) { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(26))));
}

#[test]
fn iterator_protocol_phase22_takewhile_async_lazy_zip() {
    let out = eval(
        r#"
        for_of_items(iterator_take_while(range(1, 10), (n) => n < 4))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
    assert!(matches!(items[2], Value::Number(3)));

    let out = eval(
        r#"
        Iterator.from(range(1, 10)).dropWhile((n) => n < 3).takeWhile((n) => n < 6).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Number(3)));
    assert!(matches!(items[1], Value::Number(4)));
    assert!(matches!(items[2], Value::Number(5)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 4) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let items = await AsyncIterator.from(o).takeWhile((n) => n < 3).toArray()
          return items
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkLeft() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          fn mkRight() {
            let i = 9
            return {
              next() {
                if (i < 11) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let a = {}
          a[Symbol.asyncIterator] = mkLeft
          let pairs = await AsyncIterator.enumerate(a).toArray()
          return pairs
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array of pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 2);
    let Value::Array(p0) = &pairs[0] else {
        panic!("expected pair array");
    };
    assert!(matches!(p0[0], Value::Number(0)));
    assert!(matches!(p0[1], Value::Number(1)));
    let Value::Array(p1) = &pairs[1] else {
        panic!("expected pair array");
    };
    assert!(matches!(p1[0], Value::Number(1)));
    assert!(matches!(p1[1], Value::Number(2)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkLeft() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          fn mkRight() {
            let i = 9
            return {
              next() {
                if (i < 11) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let a = {}
          a[Symbol.asyncIterator] = mkLeft
          let b = {}
          b[Symbol.asyncIterator] = mkRight
          let pairs = await AsyncIterator.zip(a, b).toArray()
          return pairs
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected zip pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 2, "async native lazy zip pairs: {:?}", pairs);
    let Value::Array(p0) = &pairs[0] else {
        panic!("expected pair array");
    };
    assert!(matches!(p0[0], Value::Number(1)));
    assert!(matches!(p0[1], Value::Number(10)));
    let Value::Array(p1) = &pairs[1] else {
        panic!("expected pair array");
    };
    assert!(matches!(p1[0], Value::Number(2)));
    assert!(matches!(p1[1], Value::Number(11)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 3) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          return await array_from_async(o)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
    assert!(matches!(items[2], Value::Number(3)));
}

#[test]
fn iterator_protocol_phase23_pairwise_accumulate() {
    let out = eval(
        r#"
        for_of_items(iterator_pairwise(range(1, 5)))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    let Value::Array(p0) = &items[0] else {
        panic!("expected pair");
    };
    assert!(matches!(p0[0], Value::Number(1)));
    assert!(matches!(p0[1], Value::Number(2)));
    let Value::Array(p2) = &items[2] else {
        panic!("expected pair");
    };
    assert!(matches!(p2[0], Value::Number(3)));
    assert!(matches!(p2[1], Value::Number(4)));

    let out = eval(
        r#"
        for_of_items(iterator_accumulate(range(1, 5), (a, b) => a + b))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(3)));
    assert!(matches!(items[2], Value::Number(6)));
    assert!(matches!(items[3], Value::Number(10)));

    let out = eval(
        r#"
        for_of_items(iterator_accumulate(range(1, 5), (a, b) => a + b, 0))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 5);
    assert!(matches!(items[0], Value::Number(0)));
    assert!(matches!(items[4], Value::Number(10)));

    let out = eval(
        r#"
        Iterator.from(range(1, 5)).pairwise().map((pair) => pair[0] * pair[1]).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Number(2)));
    assert!(matches!(items[1], Value::Number(6)));
    assert!(matches!(items[2], Value::Number(12)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 4) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let pairs = await AsyncIterator.pairwise(o).toArray()
          return pairs
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 3);
    let Value::Array(p0) = &pairs[0] else {
        panic!("expected pair");
    };
    assert!(matches!(p0[0], Value::Number(1)));
    assert!(matches!(p0[1], Value::Number(2)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 4) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let totals = await AsyncIterator.from(o).accumulate((a, b) => a + b).toArray()
          return totals
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let totals = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected totals, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(totals.len(), 4);
    assert!(matches!(totals[0], Value::Number(1)));
    assert!(matches!(totals[3], Value::Number(10)));
}

#[test]
fn iterator_protocol_phase24_async_generator_throw_resume() {
    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            try {
              yield 1
              yield 2
            } catch (e) {
              yield e * 10
            }
            yield 3
          }
          let g = gen()
          await g.next()
          let r = await g.throw(99)
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(990))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            try {
              yield 1
              yield 2
            } catch (e) {
              yield e * 10
            }
            yield 3
          }
          let g = gen()
          await g.next()
          await g.throw(99)
          let r = await g.next()
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(3))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            yield 1
            yield 2
          }
          let g = gen()
          await g.next()
          let r = await g.throw(99)
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(99))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            try {
              yield 1
            } catch (e) {
              yield e * 10
            }
            yield 2
          }
          let g = gen()
          let it = AsyncIterator.from(g)
          await it.next()
          let r = await it.throw(99)
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(990))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            try {
              yield 1
            } catch (e) {
              yield e * 10
            }
            yield 2
          }
          let g = gen()
          await g.next()
          await g.throw(99)
          let sum = 0
          for await x of g { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(2))
    ));
}

#[test]
fn iterator_protocol_phase25_from_async_instance_zip_enumerate_chain() {
    let out = eval(
        r#"
        Iterator.from(range(1, 4)).enumerate().toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 3);
    let Value::Array(p0) = &items[0] else {
        panic!("expected pair");
    };
    assert!(matches!(p0[0], Value::Number(0)));
    assert!(matches!(p0[1], Value::Number(1)));

    let out = eval(
        r#"
        Iterator.from(range(1, 3)).zip(range(10, 12)).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    let Value::Array(z0) = &items[0] else {
        panic!("expected zip pair");
    };
    assert!(matches!(z0[0], Value::Number(1)));
    assert!(matches!(z0[1], Value::Number(10)));

    let out = eval(
        r#"
        Iterator.from(range(1, 3)).chain(range(10, 12)).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
    assert!(matches!(items[2], Value::Number(10)));
    assert!(matches!(items[3], Value::Number(11)));

    let out = eval(
        r#"
        Iterator.from(range(1, 3)).map((n) => n * 10).zip(Iterator.from(range(1, 3))).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    let Value::Array(z0) = &items[0] else {
        panic!("expected zip pair");
    };
    assert!(matches!(z0[0], Value::Number(10)));
    assert!(matches!(z0[1], Value::Number(1)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let items = await Iterator.fromAsync(o).toArray()
          return items
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let pairs = await Iterator.fromAsync(o).enumerate().toArray()
          return pairs
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 2);
    let Value::Array(p0) = &pairs[0] else {
        panic!("expected pair");
    };
    assert!(matches!(p0[0], Value::Number(0)));
    assert!(matches!(p0[1], Value::Number(1)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkLeft() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          fn mkRight() {
            let i = 9
            return {
              next() {
                if (i < 11) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let a = {}
          a[Symbol.asyncIterator] = mkLeft
          let b = {}
          b[Symbol.asyncIterator] = mkRight
          let left = Iterator.fromAsync(a)
          let zipped = await left.zip(b).toArray()
          return zipped
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let pairs = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected zip pairs, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(pairs.len(), 2);
    let Value::Array(z0) = &pairs[0] else {
        panic!("expected zip pair");
    };
    assert!(matches!(z0[0], Value::Number(1)));
    assert!(matches!(z0[1], Value::Number(10)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 2) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          let chained = await Iterator.fromAsync(o).chain([10, 11]).toArray()
          return chained
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected chained array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(2)));
    assert!(matches!(items[2], Value::Number(10)));
    assert!(matches!(items[3], Value::Number(11)));
}

#[test]
fn iterator_protocol_phase26_yield_star() {
    let out = eval(
        r#"
        fn* deleg() {
          yield* [1, 2, 3]
          yield 4
        }
        for_of_items(iterator_from(deleg()))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[3], Value::Number(4)));

    let out = eval(
        r#"
        fn* inner() {
          yield 10
          yield 20
        }
        fn* outer() {
          yield 1
          yield* inner()
          yield 2
        }
        for_of_items(iterator_from(outer()))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(10)));
    assert!(matches!(items[2], Value::Number(20)));
    assert!(matches!(items[3], Value::Number(2)));

    let out = eval(
        r#"
        fn* gen() {
          yield* range(1, 4)
          yield 99
        }
        Iterator.from(gen()).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[2], Value::Number(3)));
    assert!(matches!(items[3], Value::Number(99)));

    let out = eval(
        r#"
        fn* gen() {
          yield* []
          yield 7
        }
        Iterator.from(gen()).toArray()
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0], Value::Number(7)));

    let out = eval(
        r#"
        fn* inner() {
          let x = yield 10
          yield x
        }
        fn* outer() {
          yield* inner()
        }
        let g = outer()
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        fn* inner() {
          let x = yield 10
          yield x
        }
        fn* outer() {
          yield* inner()
        }
        let g = outer()
        g.next()
        g.next(42).value
        "#,
    );
    assert!(matches!(out, Value::Number(42)));
}

#[test]
fn iterator_protocol_phase27_async_yield_star() {
    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            let x = await promise_resolve(10)
            yield x
            yield x + 1
          }
          return await array_from_async(gen())
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(10)));
    assert!(matches!(items[1], Value::Number(11)));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            yield* [1, 2, 3]
            yield 4
          }
          return await array_from_async(gen())
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[3], Value::Number(4)));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* inner() {
            yield 10
            yield 20
          }
          async fn* outer() {
            yield 1
            yield* inner()
            yield 2
          }
          return await array_from_async(outer())
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(10)));
    assert!(matches!(items[2], Value::Number(20)));
    assert!(matches!(items[3], Value::Number(2)));

    let out = eval_async(
        r#"
        async fn main() {
          fn mkAsyncIter() {
            let i = 0
            return {
              next() {
                if (i < 3) {
                  i = i + 1
                  return promise_resolve({ value: i, done: false })
                }
                return promise_resolve({ value: null, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mkAsyncIter
          async fn* gen() {
            yield* o
            yield 99
          }
          return await array_from_async(gen())
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[2], Value::Number(3)));
    assert!(matches!(items[3], Value::Number(99)));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* inner() {
            let x = yield 10
            yield x
          }
          async fn* outer() {
            yield* inner()
          }
          let g = outer()
          await g.next()
          let r = await g.next(42)
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(42))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* gen() {
            yield* range(1, 4)
            yield 99
          }
          let sum = 0
          for await x of gen() { sum = sum + x }
          return sum
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(105))
    ));
}

#[test]
fn iterator_protocol_phase28_yield_star_return_throw() {
    let out = eval(
        r#"
        fn* inner() {
          yield 1
          return 99
        }
        fn* outer() {
          let x = yield* inner()
          yield x
        }
        for_of_items(iterator_from(outer()))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(99)));

    let out = eval(
        r#"
        fn* inner() {
          yield 10
          yield 20
        }
        fn* outer() {
          yield* inner()
          yield 99
        }
        let g = outer()
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        fn* inner() {
          yield 10
          yield 20
        }
        fn* outer() {
          yield* inner()
          yield 99
        }
        let g = outer()
        g.next()
        g.return(42).value
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        fn* inner() {
          yield 10
          yield 20
        }
        fn* outer() {
          yield* inner()
          yield 99
        }
        let g = outer()
        g.next()
        g.return(42).done
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn* inner() {
          try {
            yield 1
            yield 2
          } catch (e) {
            yield e
          }
        }
        fn* outer() {
          yield* inner()
        }
        let g = outer()
        g.next()
        g.throw("kab").value
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "kab"));

    let out = eval(
        r#"
        fn* inner() {
          return 88
        }
        fn* outer() {
          let x = yield* inner()
          return x + 1
        }
        let g = outer()
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(89)));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* inner() {
            yield 1
            return 99
          }
          async fn* outer() {
            let x = yield* inner()
            yield x
          }
          return await array_from_async(outer())
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    let items = match &*p.borrow() {
        PromiseValue::Resolved(Value::Array(items)) => items.clone(),
        PromiseValue::Resolved(v) => panic!("expected array, got {:?}", v),
        _ => panic!("expected resolved promise"),
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(items[0], Value::Number(1)));
    assert!(matches!(items[1], Value::Number(99)));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* inner() {
            yield 10
            yield 20
          }
          async fn* outer() {
            yield* inner()
            yield 99
          }
          let g = outer()
          await g.next()
          let r = await g.return(42)
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(42))
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* inner() {
            try {
              yield 1
              yield 2
            } catch (e) {
              yield e
            }
          }
          async fn* outer() {
            yield* inner()
          }
          let g = outer()
          await g.next()
          let r = await g.throw("kab")
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::String(s)) if s == "kab"
    ));
}

#[test]
fn iterator_protocol_phase29_abrupt_close_and_symbol_iterator() {
    let out = eval(
        r#"
        let flag = ""
        fn mk() {
          return {
            next() {
              return { value: 1, done: false }
            },
            return(v) {
              flag = "closed"
              return { value: v, done: true }
            }
          }
        }
        for x of mk() {
          break
        }
        flag
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "closed"));

    let out = eval(
        r#"
        fn* gen() {
          yield 1
          yield 2
        }
        let g = gen()
        for x of g {
          if (x == 1) { break }
        }
        g.next().done
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn grab(iter) {
          for x of iter {
            return x
          }
          return null
        }
        fn* gen() {
          yield 1
          yield 2
        }
        grab(gen())
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        fn* g() {
          yield 7
        }
        let gen = g()
        let it = gen[Symbol.iterator]()
        it.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(7)));

    let out = eval(
        r#"
        fn* outer() {
          let v = yield* src()
          if v != null { yield v }
        }
        fn src() {
          let n = 0
          return {
            next() {
              if (n < 1) {
                n = n + 1
                return { value: 10, done: false }
              }
              return { value: null, done: true }
            },
            return(x) {
              return { value: x, done: true }
            }
          }
        }
        for_of_items(iterator_from(outer()))
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0], Value::Number(10)));

    let out = eval_async(
        r#"
        async fn main() {
          let flag = ""
          fn mk() {
            return {
              next() {
                return promise_resolve({ value: 1, done: false })
              },
              return(v) {
                flag = "closed"
                return promise_resolve({ value: v, done: true })
              }
            }
          }
          let o = {}
          o[Symbol.asyncIterator] = mk
          for await x of o { break }
          return flag
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::String(s)) if s == "closed"
    ));

    let out = eval_async(
        r#"
        async fn main() {
          async fn* g() {
            yield 9
          }
          let gen = g()
          let it = gen[Symbol.asyncIterator]()
          let r = await it.next()
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(9))
    ));
}

#[test]
fn iterator_protocol_phase30_iterator_self_and_raise_close() {
    let out = eval(
        r#"
        let it = iterator_map(range(1, 4), (n) => n)
        let it2 = it[Symbol.iterator]()
        it2.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(1)));

    let out = eval(
        r#"
        let flag = ""
        fn mk() {
          return {
            next() {
              return { value: 1, done: false }
            },
            return(v) {
              flag = "closed"
              return { value: v, done: true }
            }
          }
        }
        let msg = ""
        try {
          for x of mk() {
            raise "kab"
          }
        } catch (e) {
          msg = e
        }
        [flag, msg]
        "#,
    );
    let Value::Array(items) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert!(matches!(&items[0], Value::String(s) if s == "closed"));
    assert!(matches!(&items[1], Value::String(s) if s == "kab"));

    let out = eval(
        r#"
        fn* outer() {
          let v = yield* src()
          if v != null { yield v }
        }
        fn src() {
          let n = 0
          return {
            next() {
              if (n < 1) {
                n = n + 1
                return { value: 10, done: false }
              }
              return { value: null, done: true }
            },
            throw(e) {
              return { value: e, done: true }
            }
          }
        }
        let g = outer()
        g.next().value
        "#,
    );
    assert!(matches!(out, Value::Number(10)));

    let out = eval(
        r#"
        fn* outer() {
          let v = yield* src()
          if v != null { yield v }
        }
        fn src() {
          let n = 0
          return {
            next() {
              if (n < 1) {
                n = n + 1
                return { value: 10, done: false }
              }
              return { value: null, done: true }
            },
            throw(e) {
              return { value: e, done: true }
            }
          }
        }
        let g = outer()
        g.next()
        g.throw("kab").value
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "kab"));

    let out = eval_async(
        r#"
        async fn main() {
          let lazy = AsyncIterator.from(range(1, 4)).map((n) => n)
          let it2 = lazy[Symbol.asyncIterator]()
          let r = await it2.next()
          return r.value
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise");
    };
    assert!(matches!(
        &*p.borrow(),
        PromiseValue::Resolved(Value::Number(1))
    ));
}

#[test]
fn js_wave_a1_btoa_atob() {
    let out = eval(r#"btoa("kab")"#);
    assert!(matches!(out, Value::String(s) if s == "a2Fi"));

    let out = eval(r#"atob("a2Fi")"#);
    assert!(matches!(out, Value::String(s) if s == "kab"));

    let out = eval(r#"atob("SGk=")"#);
    assert!(matches!(out, Value::String(s) if s == "Hi"));
}

#[test]
fn js_wave_a1_performance_now() {
    let out = eval(r#"performance.now()"#);
    match out {
        Value::Float(ms) => assert!(ms >= 0.0),
        Value::Number(n) => assert!(n >= 0),
        other => panic!("expected number, got {other:?}"),
    }

    let out = eval(
        r#"
        let t0 = performance.now()
        let t1 = performance.now()
        t1 >= t0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a1_crypto_get_random_values() {
    let out = eval(r#"crypto.getRandomValues([0, 0, 0, 0])[0]"#);
    match out {
        Value::Number(n) => assert!((0..=255).contains(&n)),
        other => panic!("expected byte 0..255, got {other:?}"),
    }

    let out = eval(
        r#"
        let sab = sab_new(16)
        let u8 = uint8_array_new(sab, 0, 8)
        crypto.getRandomValues(u8)
        uint8_array_get(u8, 0)
        "#,
    );
    match out {
        Value::Number(n) => assert!((0..=255).contains(&n)),
        other => panic!("expected byte 0..255, got {other:?}"),
    }
}

#[test]
fn js_wave_a2_bigint() {
    assert!(matches!(eval(r#"1n + 2n == 3n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"10n - 3n == 7n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"6n * 7n == 42n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"10n / 3n == 3n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"10n % 3n == 1n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"2n ** 10n == 1024n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"-42n == -42n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"BigInt("99") == 99n"#), Value::Bool(true)));
    assert!(matches!(eval(r#"typeof(1n)"#), Value::String(s) if s == "bigint"));
    assert!(matches!(eval(r#"1n == 1"#), Value::Bool(true)));
}

#[test]
fn js_wave_a3_private_class_fields() {
    let out = eval(
        r#"
        class Counter {
            #n: number = 0
            fn inc() { self.#n = self.#n + 1 }
            fn read() { return self.#n }
        }
        let c = Counter()
        c.inc()
        c.inc()
        c.read()
        "#,
    );
    assert!(
        matches!(out, Value::Number(2)),
        "expected Number(2), got {:?}",
        out
    );

    let out = eval(
        r#"
        class Vault {
            #secret: number = 42
            fn #peek() { return self.#secret }
            fn reveal() { return self.#peek() }
        }
        Vault().reveal()
        "#,
    );
    assert!(matches!(out, Value::Number(42)));
}

#[test]
fn js_wave_a4_regexp_flags_dotall_lookbehind() {
    let out = eval(
        r#"
        let nl = from_char_code(10)
        let re = regexp_new("a.b", "s")
        re.flags == "s" && re.dotAll && regexp_test(re, "a" + nl + "b")
        "#,
    );
    assert!(
        matches!(out, Value::Bool(true)),
        "expected Bool(true), got {:?}",
        out
    );

    let out = eval(r#"regex_test("/a.b/s", "a" + from_char_code(10) + "b")"#);
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(r#"regex_match("(?<=@)host", "user@host")"#);
    let Value::Array(groups) = out else {
        panic!("expected array, got {:?}", out);
    };
    assert!(matches!(&groups[0], Value::String(s) if s == "host"));

    let out = eval(
        r#"
        let re = regexp_new("foo", "iu")
        re.ignoreCase && re.unicode && regexp_test(re, "FOO")
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let re = regexp_new("bar")
        is_regexp(re) && re.source == "bar" && re.flags == ""
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a5_date_utc_local_timezone() {
    let out = eval(r#"date_to_iso_string(date_new(0))"#);
    assert!(matches!(out, Value::String(s) if s == "1970-01-01T00:00:00.000Z"));

    let out = eval(
        r#"
        let d = date_new(0)
        date_get_utc_full_year(d) == 1970
            && date_get_utc_month(d) == 0
            && date_get_utc_date(d) == 1
            && date_get_utc_hours(d) == 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        date_set_timezone_offset(-60)
        let d = date_new(0)
        date_get_hours(d) == 1 && date_get_timezone_offset() == -60
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let _ = eval("date_set_timezone_offset(0)");

    let out = eval(r#"date_parse("1970-01-01T00:00:00.000Z")"#);
    assert!(matches!(out, Value::Number(0)));

    let out = eval(
        r#"
        let d = date_new(0)
        is_date(d) && date_value_of(d) == 0 && Date_now() >= 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a6_typed_arrays() {
    let out = eval(
        r#"
        let buf = array_buffer_new(16)
        let f64 = float64_array_new(buf, 0, 2)
        float64_array_set(f64, 0, 3.5)
        float64_array_set(f64, 1, 2.25)
        float64_array_get(f64, 0) + float64_array_get(f64, 1)
        "#,
    );
    assert!(matches!(out, Value::Float(f) if (f - 5.75).abs() < 0.001));

    let out = eval(
        r#"
        let buf = array_buffer_new(16)
        let dv = data_view_new(buf, 0, 16)
        data_view_set_float64(dv, 0, 42.5)
        data_view_get_float64(dv, 0)
        "#,
    );
    assert!(matches!(out, Value::Float(f) if (f - 42.5).abs() < 0.001));
}

#[test]
fn js_wave_a7_proxy_reflect() {
    let out = eval(
        r#"
        let target = { x: 1 }
        fn get_trap(t, k, r) { return t[k] + 1 }
        let p = Proxy(target, { get: get_trap })
        is_proxy(p) && p.x == 2 && Reflect.isProxy(p)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        fn Maker(x) { return { value: x } }
        Reflect.construct(Maker, [99], Maker).value
        "#,
    );
    assert!(matches!(out, Value::Number(99)));
}

#[test]
fn js_wave_a8_weak_collections() {
    let out = eval(
        r#"
        let wm = weak_map_new()
        let key = { id: 1 }
        weak_map_set(wm, key, 42)
        weak_map_get(wm, key) == 42 && weak_map_has(wm, key)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let ws = weak_set_new()
        let item = { tag: "x" }
        weak_set_add(ws, item)
        weak_set_has(ws, item)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a9_using_disposes_on_block_exit() {
    let out = eval(
        r#"
        let r = disposable_new()
        {
            using x = r
        }
        disposable_disposed(r)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a9_using_close_fallback() {
    let out = eval(
        r#"
        let closed = false
        fn do_close() { closed = true }
        let r = { close: do_close }
        {
            using x = r
        }
        closed
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a9_import_meta_defaults() {
    let out = eval(
        r#"
        let m = import.meta
        m.url == "kabootar:///main" && m.path == "main"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a9_dynamic_import_builtin_math() {
    let out = eval_async(
        r#"
        async fn main() {
            let ns = await import("math")
            return ns.add(2, 3)
        }
        main()
        "#,
    );
    let Value::Promise(p) = out else {
        panic!("expected promise from async main");
    };
    assert!(matches!(
        *p.borrow(),
        PromiseValue::Resolved(Value::Number(5))
    ));
}

#[test]
fn js_wave_a10_intl_number_format() {
    let out = eval(
        r#"
        let nf = Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 2 })
        nf.format(1234.5)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "$1,234.50"));

    let out = eval(
        r#"
        let nf = Intl.NumberFormat("en-US", { style: "percent", maximumFractionDigits: 0 })
        nf.format(0.25)
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "25%"));
}

#[test]
fn js_wave_a10_intl_date_time_format() {
    let out = eval(
        r#"
        let d = date_new(0)
        let dtf = Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeStyle: "medium" })
        dtf.format(d).length > 8
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a11_temporal_plain_date_instant() {
    let out = eval(
        r#"
        let d = Temporal.PlainDate.from({ year: 2024, month: 6, day: 20 })
        temporal_plain_date_to_string(d) == "2024-06-20"
            && temporal_plain_date_add(d, { days: 10 }).day == 30
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let i = Temporal.Instant.from(0)
        temporal_instant_epoch_ms(i) == 0
            && temporal_instant_to_string(i) == "1970-01-01T00:00:00.000Z"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a12_error_cause_and_stack() {
    let out = eval(
        r#"
        let root = error_new("root")
        let wrapped = error_new("wrapped", { cause: root })
        error_message(wrapped) == "wrapped" && is_error(error_cause(wrapped))
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let e = null
        try {
            throw error_new("boom")
        } catch (err) {
            e = err
        }
        let s = error_stack(e)
        s.length > 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}

#[test]
fn js_wave_a12_generator_throw_single_yield_in_try() {
    let out = eval(
        r#"
        fn* gen() {
            try {
                yield 1
            } catch (e) {
                yield e * 10
            }
            yield 3
        }
        let g = gen()
        g.next()
        g.throw(7).value
        "#,
    );
    assert!(matches!(out, Value::Number(70)));
}

fn eval_fails(code: &str) -> bool {
    let mut env = create_global_env();
    eval_source(code, &mut env).is_err()
}

#[test]
fn js_wave_a3_private_field_external_access_fails() {
    assert!(eval_fails(
        r#"
        class Box {
            #v: number = 1
            fn init() {}
        }
        let b = Box()
        b.#v
        "#,
    ));
}

#[test]
fn object_symbol_phase4_1() {
    let out = eval(
        r#"
        let s = Symbol("copy")
        let src = {}
        src[s] = 42
        let dest = {}
        Object.assign(dest, src)
        dest[s]
        "#,
    );
    assert!(matches!(out, Value::Number(42)));

    let out = eval(
        r#"
        let s = Symbol("hidden")
        let o = {}
        o[s] = 1
        s in o
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let s = Symbol("nope")
        let o = {}
        s in o
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let p = { x: 1 }
        let o = Object.create(p)
        "x" in o
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let s = Symbol("tag")
        let o = {}
        o[s] = 9
        Reflect.has(o, s)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));

    let out = eval(
        r#"
        let s = Symbol("gone")
        let o = {}
        o[s] = 1
        Object.deleteProperty(o, s)
        s in o
        "#,
    );
    assert!(matches!(out, Value::Bool(false)));

    let out = eval(
        r#"
        let s = Symbol("via-proxy")
        let target = {}
        target[s] = 5
        let p = Proxy(target, {})
        s in p && Reflect.get(p, s) == 5
        "#,
    );
    assert!(matches!(out, Value::Bool(true)));
}
