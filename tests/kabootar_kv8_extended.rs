//! Kv8 extended: JS-subset, kstyle blocks, VFS modules, JIT, browser bridge.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn kv8_jit_loop_accumulator_correct() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          for (let i = 0; i < 10; i = i + 1) { s = s + i; }
          return s;
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(45)));
}

#[test]
fn kv8_opt_caches_program_and_arrow() {
    let info = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "let add = (a, b) => a + b; return add(2, 3);");
        kv8_eval(ctx, "let add = (a, b) => a + b; return add(4, 5);");
        kv8_opt_info(ctx);
        "#,
    );
    let Value::Object(o) = info else {
        panic!("expected opt info");
    };
    assert!(matches!(o.get("program_cache"), Some(Value::Number(n)) if *n >= 1));
    assert!(matches!(o.get("arrow_cache"), Some(Value::Number(n)) if *n >= 1));
}

#[test]
fn kv8_info_reports_optimizations() {
    let info = eval("kv8_info()");
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("hot_path_predictor"), Some(Value::Bool(true))));
    assert!(matches!(o.get("ownership_gc"), Some(Value::String(s)) if s.contains("no-pause")));
}

#[test]
fn kv8_if_for_function_arrow() {
    let n = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          function double(x) { return x * 2; }
          let add = (a, b) => a + b;
          let sum = 0;
          for (let i = 0; i < 5; i = i + 1) {
            if (i < 3) { sum = sum + i; }
          }
          return double(add(sum, 1));
        ");
        "#,
    );
    assert!(matches!(n, Value::Number(8)));
}

#[test]
fn kv8_jit_records_hot_loops() {
    let hits = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "
          let s = 0;
          for (let i = 0; i < 20; i = i + 1) { s = s + 1; }
          return s;
        ");
        let info = kv8_jit_info(ctx);
        info["loop_hits"];
        "#,
    );
    assert!(matches!(hits, Value::Number(n) if n >= 8));
}

#[test]
fn kstyle_block_expands_to_css() {
    let css = eval(
        r#"
        kstyle {
          .app { color: #e8eaed; padding: 16px; }
          #title { font-size: 24px; }
        }
        kstyle_css();
        "#,
    );
    let Value::String(s) = css else {
        panic!("expected css string");
    };
    assert!(s.contains(".app"));
    assert!(s.contains("color"));
    assert!(s.contains("#title"));
}

#[test]
fn kv8_load_vfs_module() {
    let dom = eval(
        r#"
        os_write("/apps/demo.kv8", "
---kml---
<div id='app'><h1>Hello Kv8</h1></div>
---css---
#app { color: #00ff88; }
---script---
let h = document.querySelector('h1');
h.textContent = 'Loaded';
        ");
        let ctx = kv8_create();
        kv8_load_vfs(ctx, "/apps/demo.kv8");
        "#,
    );
    assert!(matches!(dom, Value::KabootarDom(n) if !n.children.is_empty()));
}

#[test]
fn kb_navigate_and_run_kv8_vfs() {
    let ok = eval(
        r#"
        os_write("/apps/page.kv8", "
---kml---
<html><body><p id='msg'>init</p></body></html>
---css---
#msg { color: blue; }
---script---
let p = document.querySelector('#msg');
p.textContent = 'from kv8';
        ");
        kb_navigate("kabootar://vfs/apps/page.kv8");
        kb_run_kv8();
        "#,
    );
    assert!(matches!(ok, Value::Bool(true)));
}
