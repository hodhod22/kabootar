//! Kv8 self-hosting smoke tests — maps Web APIs for a minimal app without Chrome.
//!
//! Run: `cargo test kv8_smoke -- --nocapture`

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

fn eval_ok(code: &str) -> bool {
    let mut env = create_global_env();
    eval_source(code, &mut env).is_ok()
}

fn report_field<'a>(report: &'a Value, key: &str) -> &'a Value {
    let Value::Object(o) = report else {
        panic!("expected probe report object");
    };
    o.get(key).unwrap_or_else(|| panic!("missing {key} in probe report"))
}

/// APIs that must keep working — regression guard for Våg C1 DOM core.
const CORE_READY: &[&str] = &[
    "document.createElement",
    "document.appendChild",
    "document.querySelector",
    "element.appendChild",
    "element.textContent (write)",
    "element.style assignment",
    "function declaration",
    "arrow function",
    "for loop",
    "while loop",
    "break",
    "continue",
    "console.log",
];

/// P0 gaps we expect until Våg C1/C2 lands — React smoke path blockers.
const EXPECTED_P0_GAPS: &[&str] = &[];

/// Promise/async APIs implemented in Våg C2.
const PROMISE_READY: &[&str] = &["Promise", "async/await"];

/// Network APIs implemented in Våg C1.
const FETCH_READY: &[&str] = &["fetch"];

/// Timer APIs implemented in Våg C1.
const TIMER_READY: &[&str] = &["setTimeout"];

/// DOM + storage + frame APIs implemented in Våg C1.
const DOM_STORAGE_READY: &[&str] = &[
    "document.body",
    "localStorage.setItem",
    "document.getElementById",
    "element.innerHTML",
    "requestAnimationFrame",
    "document.querySelectorAll",
    "element.setAttribute",
    "element.removeChild",
    "element.firstChild",
];

/// Promise chaining implemented in Våg C1.
const PROMISE_CHAIN_READY: &[&str] = &["Promise.then"];

/// Event APIs implemented in Våg C1 — must stay ready.
const EVENT_READY: &[&str] = &[
    "element.addEventListener",
    "element.dispatchEvent",
];

#[test]
fn kv8_smoke_probe_returns_report() {
    let report = eval("kv8_self_hosting_probe()");
    let Value::Object(o) = &report else {
        panic!("expected object report");
    };
    assert!(matches!(o.get("engine"), Some(Value::String(s)) if s == "kv8"));
    assert!(matches!(o.get("ready_count"), Some(Value::Number(n)) if *n > 0));
    assert!(matches!(o.get("missing_count"), Some(Value::Number(n)) if *n > 0));
    assert!(matches!(o.get("p0_missing_count"), Some(Value::Number(n)) if *n == 0));
}

/// Kv8-only JS operators (Kabootar uses == / !=; Kv8 adds === / !== for React parity).
const KV8_JS_OPS_READY: &[&str] = &["=== strict equality", "!== strict inequality", "typeof"];

/// Kv8 JS parity for React esbuild bundle path.
const KV8_UMD_PARITY_READY: &[&str] = &[
    "function expression",
    "var declaration",
    "this",
    "Symbol.for",
    "Object.assign",
];

/// Våg 1 — expression-level JS parity for self-hosting / UMD.
const KV8_WAVE1_READY: &[&str] = &[
    "comma operator",
    "assignment expression",
    "void",
    "undefined",
    "throw",
];

/// Våg 2 — control flow + operators for React UMD parity.
const KV8_WAVE2_READY: &[&str] = &[
    "ternary",
    "switch/case",
    "for (C-style)",
    "for-in",
    "try/catch",
    "regex literal",
    "+=",
    "++",
];

/// Våg 3 — arrays for React UMD parity.
const KV8_WAVE3_READY: &[&str] = &[
    "array literal",
    "bracket index",
    "Array.isArray",
    "Array()",
];

/// Våg 4 — UMD patterns: member assign, arguments, defaults, shorthand, for-in.
const KV8_WAVE4_READY: &[&str] = &[
    "member assign",
    "arguments",
    "default param",
    "method shorthand",
    "for-in assign iterable",
];

/// Våg 5 — React 19 UMD parse: try/finally, unary -, new member, for/switch comma.
const KV8_WAVE5_READY: &[&str] = &[
    "try/finally",
    "unary minus",
    "new member expr",
    "for comma init",
    "switch comma disc",
];

/// Våg 6 — React 19 UMD eval: prototype, function scope, globalThis export.
const KV8_WAVE6_READY: &[&str] = &[
    "function prototype",
    "function scope",
];

/// Våg 7 — ReactDOM UMD parse path: bitwise, labels, spread, in/delete.
const KV8_WAVE7_READY: &[&str] = &[
    "bitwise and",
    "shift and ushr",
    "try/finally only",
    "label break",
    "do while",
    "in operator",
    "object spread",
];

/// Våg 8 — ReactDOM 19 UMD eval: member-assign order, string index.
const KV8_WAVE8_READY: &[&str] = &[
    "member assign lhs order",
    "string char index",
];

#[test]
fn kv8_smoke_core_dom_does_not_regress() {
    let report = eval("kv8_self_hosting_probe()");
    let ready = report_field(&report, "ready");
    let Value::Array(items) = ready else {
        panic!("expected ready array");
    };
    let ready_set: Vec<String> = items
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    for api in CORE_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "core API regressed: {api}"
        );
    }
    for api in EVENT_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "event API regressed: {api}"
        );
    }
    for api in TIMER_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "timer API regressed: {api}"
        );
    }
    for api in PROMISE_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "promise API regressed: {api}"
        );
    }
    for api in FETCH_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "fetch API regressed: {api}"
        );
    }
    for api in DOM_STORAGE_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "dom/storage API regressed: {api}"
        );
    }
    for api in PROMISE_CHAIN_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "promise chain API regressed: {api}"
        );
    }
    for api in KV8_JS_OPS_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 js op regressed: {api}"
        );
    }
    for api in KV8_UMD_PARITY_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 umd parity regressed: {api}"
        );
    }
    for api in KV8_WAVE1_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave1 regressed: {api}"
        );
    }
    for api in KV8_WAVE2_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave2 regressed: {api}"
        );
    }
    for api in KV8_WAVE3_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave3 regressed: {api}"
        );
    }
    for api in KV8_WAVE4_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave4 regressed: {api}"
        );
    }
    for api in KV8_WAVE5_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave5 regressed: {api}"
        );
    }
    for api in KV8_WAVE6_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave6 regressed: {api}"
        );
    }
    for api in KV8_WAVE7_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave7 regressed: {api}"
        );
    }
    for api in KV8_WAVE8_READY {
        assert!(
            ready_set.iter().any(|s| s == *api),
            "kv8 wave8 regressed: {api}"
        );
    }
}

#[test]
fn kv8_smoke_expected_p0_gaps_documented() {
    let report = eval("kv8_self_hosting_probe()");
    let p0_missing = report_field(&report, "p0_missing");
    let Value::Array(gaps) = p0_missing else {
        panic!("expected p0_missing array");
    };
    let gap_set: Vec<String> = gaps
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    for api in EXPECTED_P0_GAPS {
        assert!(
            gap_set.iter().any(|s| s == *api),
            "expected P0 gap missing from report: {api}"
        );
    }
}

#[test]
fn kv8_smoke_document_body_append_child() {
    let kids = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          document.body.appendChild(document.createElement('div'));
          document.body.appendChild(document.createElement('span'));
        ");
        let root = kv8_dom(ctx);
        root;
        "#,
    );
    let Value::KabootarDom(html) = kids else {
        panic!("expected html root");
    };
    let body = html.children.iter().find(|n| n.tag == "body").expect("body");
    assert_eq!(body.children.len(), 2);
    assert_eq!(body.children[0].tag, "div");
    assert_eq!(body.children[1].tag, "span");
}

#[test]
fn kv8_smoke_local_storage_roundtrip() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          localStorage.setItem('theme', 'dark');
          localStorage.getItem('theme');
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "dark"));
    let missing = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "localStorage.getItem('missing');");
        "#,
    );
    assert!(matches!(missing, Value::Null));
}

#[test]
fn kv8_smoke_inner_html_parses_markup() {
    let root = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let el = document.createElement('div');
          el.innerHTML = '<b>x</b>';
          document.body.appendChild(el);
        ");
        kv8_dom(ctx);
        "#,
    );
    let Value::KabootarDom(html) = root else {
        panic!("expected html root");
    };
    let body = html.children.iter().find(|n| n.tag == "body").expect("body");
    let wrapper = body.children.first().expect("wrapper div");
    assert_eq!(wrapper.tag, "div");
    let bold = wrapper.children.first().expect("bold");
    assert_eq!(bold.tag, "b");
    assert_eq!(bold.children[0].text.as_deref(), Some("x"));
}

#[test]
fn kv8_smoke_get_element_by_id_finds_node() {
    let tag = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let el = document.createElement('div');
          el.id = 'app';
          document.body.appendChild(el);
          let found = document.getElementById('app');
          found.tagName;
        ");
        "#,
    );
    assert!(matches!(tag, Value::String(s) if s == "div"));
}

#[test]
fn kv8_smoke_query_selector_all_finds_nodes() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          document.body.appendChild(document.createElement('div'));
          document.body.appendChild(document.createElement('div'));
          let all = document.querySelectorAll('div');
          return all.length;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(count) if count >= 2));
}

#[test]
fn kv8_smoke_set_attribute_roundtrip() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let el = document.createElement('div');
          el.setAttribute('data-x', '1');
          el.getAttribute('data-x');
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "1"));
}

#[test]
fn kv8_smoke_react_path_mounts_and_clicks() {
    let buttons = eval(
        r#"
        let ctx = kv8_create();
        kv8_react_smoke(ctx);
        "#,
    );
    assert!(matches!(buttons, Value::Number(1)));
    let label = eval(
        r#"
        let ctx = kv8_create();
        kv8_react_smoke(ctx);
        kv8_eval(ctx, "
          let btns = document.querySelectorAll('button');
          btns.forEach((btn) => { btn.dispatchEvent({ type: 'click' }); });
          let text = '';
          document.querySelectorAll('span').forEach((s) => { text = s.textContent; });
          return text;
        ");
        "#,
    );
    assert!(matches!(label, Value::String(s) if s == "Count: 1"));
}

#[test]
fn kv8_smoke_string_concat_number() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "'Count: ' + 3;");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "Count: 3"));
}

#[test]
fn kv8_smoke_while_loop_sums() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          let i = 0;
          while (i < 5) {
            s = s + i;
            i = i + 1;
          }
          return s;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(10)));
}

#[test]
fn kv8_smoke_break_exits_loop() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          let i = 0;
          while (i < 10) {
            if (i == 3) { break; }
            s = s + i;
            i = i + 1;
          }
          return s;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(3)));
}

#[test]
fn kv8_smoke_continue_skips_iteration() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          let i = 0;
          while (i < 5) {
            i = i + 1;
            if (i == 3) { continue; }
            s = s + i;
          }
          return s;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(12)));
}

#[test]
fn kv8_smoke_function_expression_iife() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = (function (x) { return x + 1; })(41);
          return n;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(42)));
}

#[test]
fn kv8_smoke_this_method_call() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { x: 7, get: function () { return this.x; } };
          return o.get();
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(7)));
}

#[test]
fn kv8_smoke_symbol_for_registry() {
    let t = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof Symbol.for('react.element');");
        "#,
    );
    assert!(matches!(t, Value::String(s) if s == "symbol"));
    let same = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "Symbol.for('k') === Symbol.for('k');");
        "#,
    );
    assert!(matches!(same, Value::Bool(true)));
}

#[test]
fn kv8_smoke_object_assign_and_has_own_property() {
    let ok = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { default: 1 };
          return Object.prototype.hasOwnProperty.call(o, 'default');
        ");
        "#,
    );
    assert!(matches!(ok, Value::Bool(true)));
    let a = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "Object.assign({}, { a: 2 }).a;");
        "#,
    );
    assert!(matches!(a, Value::Number(2)));
}

#[test]
fn kv8_smoke_wave1_comma_and_assign_expr() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let t = 0;
          let r = { exports: 0 };
          let x = (t || (t = 1, r.exports = 42, r.exports));
          return x;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(42)));
}

#[test]
fn kv8_smoke_wave1_void_and_undefined() {
    let t = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof undefined;");
        "#,
    );
    assert!(matches!(t, Value::String(s) if s == "undefined"));
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "void 0;");
        "#,
    );
    assert!(matches!(v, Value::Null));
}

#[test]
fn kv8_smoke_wave1_throw_propagates() {
    let err = {
        let mut env = kabootar_lib::evaluator::create_global_env();
        kabootar_lib::evaluator::eval_source(
            r#"
            let ctx = kv8_create();
            kv8_eval(ctx, "function f() { throw Error('boom'); } f();");
            "#,
            &mut env,
        )
    };
    assert!(err.is_err());
}

#[test]
fn kv8_smoke_wave2_ternary_and_switch() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "false ? 1 : 42;");
        "#,
    );
    assert!(matches!(n, Value::Number(42)));
    let s = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let c = 'x';
          let r = 0;
          switch (c) {
            case 'x': r = 7; break;
            default: r = 1;
          }
          return r;
        ");
        "#,
    );
    assert!(matches!(s, Value::Number(7)));
}

#[test]
fn kv8_smoke_wave2_for_and_compound() {
    let sum = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          for (var i = 0; i < 3; i++) { s = s + 1; }
          return s;
        ");
        "#,
    );
    assert!(matches!(sum, Value::Number(3)));
    let key = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { z: 1 };
          let k = '';
          for (var p in o) { k = p; }
          return k;
        ");
        "#,
    );
    assert!(matches!(key, Value::String(s) if s == "z"));
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let n = 1; n += 2; n;");
        "#,
    );
    assert!(matches!(n, Value::Number(3)));
}

#[test]
fn kv8_smoke_wave2_try_catch_and_regex() {
    let x = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let x = 0;
          try { throw Error('e'); } catch (err) { x = 1; }
          return x;
        ");
        "#,
    );
    assert!(matches!(x, Value::Number(1)));
    let t = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof /a/g;");
        "#,
    );
    assert!(matches!(t, Value::String(s) if s == "object"));
}

#[test]
fn kv8_smoke_wave3_arrays() {
    let len = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let a = [1, 2, 3]; a.length;");
        "#,
    );
    assert!(matches!(len, Value::Number(3)));
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let a = [10, 20]; a[1];");
        "#,
    );
    assert!(matches!(v, Value::Number(20)));
    let ok = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "Array.isArray([]);");
        "#,
    );
    assert!(matches!(ok, Value::Bool(true)));
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let r = [];
          r[0] = 5;
          return r.length;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(1)));
}

#[test]
fn kv8_smoke_wave4_umd_patterns() {
    let x = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let o = {}; (o).x = 7; return o.x;");
        "#,
    );
    assert!(matches!(x, Value::Number(7)));
    let a = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "function f() { return arguments[0]; } return f(42);");
        "#,
    );
    assert!(matches!(a, Value::Number(42)));
    let d = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "function f(x=9) { return x; } f();");
        "#,
    );
    assert!(matches!(d, Value::Number(9)));
    let m = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let o = { m() { return 3; } }; o.m();");
        "#,
    );
    assert!(matches!(m, Value::Number(3)));
    let k = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let src = { a: 1 }; let k = ''; for (r in i = src) { k = r; } return k;");
        "#,
    );
    assert!(matches!(k, Value::String(s) if s == "a"));
    let gt = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "(globalThis).React = 1; return globalThis.React;");
        "#,
    );
    assert!(matches!(gt, Value::Number(1)));
}

#[test]
fn kv8_smoke_wave5_umd_parse_patterns() {
    let minus = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let x = -1; return x === -1;");
        "#,
    );
    assert!(matches!(minus, Value::Bool(true)));
    let fin = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let x = 1; try { x = 2; } catch (e) {} finally { x = 3; } return x;");
        "#,
    );
    assert!(matches!(fin, Value::Number(3)));
    let info = eval("kv8_react_bundle_info()");
    let Value::Object(o) = info else {
        panic!("expected bundle info");
    };
    assert!(matches!(o.get("umd_kv8_parseable"), Some(Value::Bool(true))));
    assert!(matches!(o.get("umd_kv8_runnable"), Some(Value::Bool(true))));
    assert!(matches!(o.get("umd_has_react_export"), Some(Value::Bool(true))));
}

#[test]
#[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
fn kv8_smoke_wave6_umd_eval() {
    let version = eval(
        r#"
        let ctx = kv8_create();
        kv8_load_react_umd(ctx);
        kv8_eval(ctx, "return globalThis.React.version;");
        "#,
    );
    assert!(matches!(version, Value::String(s) if s.starts_with("19.")));
}

#[test]
fn kv8_smoke_wave7_reactdom_patterns() {
    let bitwise = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let x = 42 & 15; return x;");
        "#,
    );
    assert!(matches!(bitwise, Value::Number(10)));
    let spread = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let o = { ...{ a: 1 }, b: 2 }; return o.b;");
        "#,
    );
    assert!(matches!(spread, Value::Number(2)));
    let info = eval("kv8_react_bundle_info()");
    let Value::Object(o) = info else {
        panic!("expected bundle info");
    };
    assert!(matches!(
        o.get("react_dom_umd_bytes"),
        Some(Value::Number(n)) if *n > 100_000
    ));
}

#[test]
#[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
fn kv8_smoke_wave8_reactdom_umd_eval() {
    let info = eval("kv8_react_bundle_info()");
    let Value::Object(o) = info else {
        panic!("expected bundle info");
    };
    assert!(matches!(
        o.get("react_dom_umd_kv8_runnable"),
        Some(Value::Bool(true))
    ));
    assert!(matches!(
        o.get("react_dom_has_reactdom_export"),
        Some(Value::Bool(true))
    ));
    let create_root = eval(
        r#"
        let ctx = kv8_create();
        kv8_load_react_dom_umd(ctx);
        kv8_eval(ctx, "return typeof globalThis.ReactDOM.createRoot;");
        "#,
    );
    assert!(matches!(create_root, Value::String(s) if s == "function"));
}

#[test]
fn kv8_smoke_react_umd_parse_progress() {
    let info = eval("kv8_react_bundle_info()");
    let Value::Object(o) = info else {
        panic!("expected bundle info object");
    };
    assert!(
        matches!(o.get("shim_kv8_parseable"), Some(Value::Bool(true))),
        "shim must stay parseable"
    );
    assert!(
        matches!(o.get("app_kv8_parseable"), Some(Value::Bool(true))),
        "app must stay parseable"
    );
    assert!(
        matches!(o.get("umd_kv8_parseable"), Some(Value::Bool(true))),
        "react 19 UMD must parse"
    );
    assert!(
        matches!(o.get("umd_kv8_runnable"), Some(Value::Bool(true))),
        "react 19 UMD must eval"
    );
    assert!(
        o.get("umd_parse_error")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("")
            .is_empty(),
        "umd_parse_error should be empty when parseable"
    );
}

#[test]
fn kv8_smoke_strict_eq_and_neq() {
    let eq = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "1 === 1;");
        "#,
    );
    assert!(matches!(eq, Value::Bool(true)));
    let ne = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "'a' !== 'b';");
        "#,
    );
    assert!(matches!(ne, Value::Bool(true)));
    let mixed = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "1 === '1';");
        "#,
    );
    assert!(matches!(mixed, Value::Bool(false)));
    let loose = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "1 == '1';");
        "#,
    );
    assert!(matches!(loose, Value::Bool(true)));
}

#[test]
fn kv8_smoke_typeof_primitives() {
    let s = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof 'hello';");
        "#,
    );
    assert!(matches!(s, Value::String(t) if t == "string"));
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof 42;");
        "#,
    );
    assert!(matches!(n, Value::String(t) if t == "number"));
    let f = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "typeof document;");
        "#,
    );
    assert!(matches!(f, Value::String(t) if t == "object"));
}

#[test]
fn kv8_smoke_remove_child_clears_parent() {
    let kids = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let p = document.createElement('div');
          let c = document.createElement('span');
          p.appendChild(c);
          p.removeChild(c);
          document.body.appendChild(p);
        ");
        kv8_dom(ctx);
        "#,
    );
    let Value::KabootarDom(html) = kids else {
        panic!("expected html");
    };
    let body = html.children.iter().find(|n| n.tag == "body").expect("body");
    let wrapper = body.children.first().expect("wrapper");
    assert!(wrapper.children.is_empty());
}

#[test]
fn kv8_smoke_react_bundle_mounts_counter() {
    let info = eval("kv8_react_bundle_info()");
    let Value::Object(o) = &info else {
        panic!("expected bundle info");
    };
    assert!(matches!(o.get("shim_kv8_parseable"), Some(Value::Bool(true))));
    assert!(matches!(o.get("app_kv8_parseable"), Some(Value::Bool(true))));
    assert!(matches!(o.get("umd_kv8_parseable"), Some(Value::Bool(true))));
    assert!(matches!(
        o.get("react_version"),
        Some(Value::String(v)) if v.starts_with("19.")
    ));
    assert!(matches!(
        o.get("react_umd_source"),
        Some(Value::String(s)) if s == "esbuild"
    ));
    let buttons = eval(
        r#"
        let ctx = kv8_create();
        kv8_react_bundle_smoke(ctx);
        "#,
    );
    assert!(matches!(buttons, Value::Number(1)));
   let label = eval(
    r#"
    let ctx = kv8_create();
    kv8_react_bundle_smoke(ctx);
    kv8_eval(ctx, "
      let btns = document.querySelectorAll('button');
      btns.forEach((btn) => { btn.dispatchEvent({ type: 'click' }); });
      let text = '';
      document.querySelectorAll('button').forEach((b) => { text = b.textContent; });
      return text;
    ");
    "#,
);
assert!(matches!(label, Value::String(s) if s == "Count: 1"));
}

#[test]
fn kv8_smoke_request_animation_frame_fires_after_drain() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          requestAnimationFrame(() => { n = n + 1; });
          return n;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(0)));
    let after = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          requestAnimationFrame(() => { n = n + 1; });
          return n;
        ");
        kv8_drain_event_loop(ctx);
        kv8_eval(ctx, "return n;");
        "#,
    );
    assert!(matches!(after, Value::Number(1)));
}

#[test]
fn kv8_smoke_promise_then_runs_after_drain() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          Promise.resolve(5).then((x) => { n = x + 1; });
          return n;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(0)));
    let after = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          Promise.resolve(5).then((x) => { n = x + 1; });
          return n;
        ");
        kv8_drain_event_loop(ctx);
        kv8_eval(ctx, "return n;");
        "#,
    );
    assert!(matches!(after, Value::Number(6)));
}

#[test]
fn kv8_smoke_async_await_resolves() {
    let result = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          async function f() { return await Promise.resolve(1); }
          f();
        ");
        "#,
    );
    assert!(matches!(result, Value::String(s) if s == "<promise>"));
}

#[test]
fn kv8_smoke_fetch_returns_promise() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "fetch('http://127.0.0.1:9/');");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "<promise>"));
}

#[test]
fn kv8_smoke_settimeout_fires_after_drain() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          setTimeout(() => { n = n + 1; }, 0);
          return n;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(0)));
    let after = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          setTimeout(() => { n = n + 1; }, 0);
          return n;
        ");
        kv8_drain_timers(ctx);
        kv8_eval(ctx, "return n;");
        "#,
    );
    assert!(matches!(after, Value::Number(1)));
}

#[test]
fn kv8_smoke_click_listener_fires() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let n = 0;
          let el = document.createElement('button');
          el.addEventListener('click', () => { n = n + 1; });
          el.dispatchEvent({ type: 'click' });
          return n;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(1)));
}

#[test]
fn kv8_smoke_event_bubble_and_remove() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let seen = '';
          let parent = document.createElement('div');
          let child = document.createElement('span');
          parent.appendChild(child);
          document.body.appendChild(parent);
          function onParent(e) { seen = seen + 'p'; }
          function onChild(e) { seen = seen + 'c'; }
          parent.addEventListener('click', onParent);
          child.addEventListener('click', onChild);
          child.dispatchEvent(new Event('click', { bubbles: true }));
          child.removeEventListener('click', onChild);
          child.dispatchEvent(new Event('click', { bubbles: true }));
          return seen;
        ");
        "#,
    );
    assert!(
        matches!(n, Value::String(ref s) if s == "cpp"),
        "expected bubble then remove: {n:?}"
    );
}

#[test]
fn kv8_smoke_class_basic() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          class Animal {
            constructor(name) {
              this.name = name;
            }
            speak() {
              return this.name + ' makes a noise.';
            }
          }
          let a = new Animal('Cat');
          return a.speak();
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "Cat makes a noise."));
}

#[test]
fn kv8_smoke_class_extends() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          class Animal {
            constructor(name) {
              this.name = name;
            }
            speak() {
              return this.name + ' makes a noise.';
            }
          }
          class Dog extends Animal {
            speak() {
              return this.name + ' barks.';
            }
          }
          let d = new Dog('Rex');
          return d.speak();
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "Rex barks."));
}

#[test]
fn kv8_smoke_class_super_constructor() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          class Shape {
            constructor(color) {
              this.color = color;
            }
          }
          class Circle extends Shape {
            constructor(color, radius) {
              super(color);
              this.radius = radius;
            }
            area() {
              return this.radius * this.radius;
            }
          }
          let c = new Circle('red', 5);
          return c.color + ':' + c.area();
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "red:25"));
}

#[test]
fn kv8_smoke_class_static_method() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          class MathHelper {
            static square(x) {
              return x * x;
            }
          }
          return MathHelper.square(7);
        ");
        "#,
    );
    assert!(matches!(v, Value::Number(n) if n == 49));
}

#[test]
fn kv8_smoke_class_instance_fields_independent() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          class Counter {
            constructor() {
              this.count = 0;
            }
            inc() {
              this.count = this.count + 1;
            }
          }
          let a = new Counter();
          let b = new Counter();
          a.inc();
          a.inc();
          b.inc();
          return a.count + ':' + b.count;
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "2:1"));
}

#[test]
fn kv8_smoke_object_keys_values_entries() {
    let keys = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { a: 1, b: 2 };
          return Object.keys(o).length;
        ");
        "#,
    );
    assert!(matches!(keys, Value::Number(2)));

    let vals = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { x: 10, y: 20 };
          let sum = 0;
          Object.values(o).forEach((v) => { sum = sum + v; });
          return sum;
        ");
        "#,
    );
    assert!(matches!(vals, Value::Number(30)));

    let entries = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = { k: 'v' };
          let e = Object.entries(o);
          return e.length;
        ");
        "#,
    );
    assert!(matches!(entries, Value::Number(1)));
}

#[test]
fn kv8_smoke_object_assign_reads_obj_store() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let src = {};
          src.version = '19.2.7';
          let dst = Object.assign({}, src);
          return dst.version;
        ");
        "#,
    );
    assert!(matches!(v, Value::String(s) if s == "19.2.7"));
}

#[test]
fn kv8_smoke_object_from_entries() {
    let v = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let o = Object.fromEntries([['a', 1], ['b', 2]]);
          return o.a + o.b;
        ");
        "#,
    );
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn kv8_smoke_object_is() {
    let same = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "return Object.is(1, 1);");
        "#,
    );
    assert!(matches!(same, Value::Bool(true)));
    let diff = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "return Object.is(1, 2);");
        "#,
    );
    assert!(matches!(diff, Value::Bool(false)));
}

#[test]
fn kv8_smoke_minimum_app_shell_builds_dom() {
    let root = eval(
        r#"
        let ctx = kv8_create();
        kv8_minimum_app_shell(ctx);
        "#,
    );
    assert!(matches!(root, Value::KabootarDom(n) if n.tag == "div"));
    let painted = eval(
        r#"
        let ctx = kv8_create();
        kv8_minimum_app_shell(ctx);
        kv8_paint(ctx, 640, 480);
        "#,
    );
    let Value::Object(frame) = painted else {
        panic!("expected paint frame");
    };
    assert!(matches!(frame.get("width"), Some(Value::Number(640))));
}

#[test]
fn kv8_smoke_inventory_prints_gap_report() {
    let report = eval("kv8_self_hosting_probe()");
    let ready_n = match report_field(&report, "ready_count") {
        Value::Number(n) => *n,
        _ => 0,
    };
    let missing_n = match report_field(&report, "missing_count") {
        Value::Number(n) => *n,
        _ => 0,
    };
    let p0_n = match report_field(&report, "p0_missing_count") {
        Value::Number(n) => *n,
        _ => 0,
    };

    println!("\n=== KV8 Self-Hosting Smoke Report ===");
    println!("Ready: {ready_n}  Missing: {missing_n}  P0 blockers: {p0_n}");
    println!("\n--- Ready ---");
    if let Value::Array(items) = report_field(&report, "ready") {
        for v in items.iter() {
            if let Value::String(s) = v {
                println!("  [ok] {s}");
            }
        }
    }
    println!("\n--- P0 missing (React smoke path) ---");
    if let Value::Array(items) = report_field(&report, "p0_missing") {
        for v in items.iter() {
            if let Value::String(s) = v {
                println!("  [!!] {s}");
            }
        }
    }
    println!("=====================================\n");

    assert!(eval_ok("kv8_self_hosting_probe()"));
}
