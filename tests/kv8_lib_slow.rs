//! Slow kv8 tests — eval + dom (heavy `import "kv8/eval"` chain).
//! Run: `cargo test --test kv8_lib_slow -- --test-threads=1`

use kabootar_lib::cli;
use kabootar_lib::compile;
use kabootar_lib::value::Value;
use std::sync::Once;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

static KV8_CACHE_ONCE: Once = Once::new();

fn ensure_kv8_eval_cache_fresh() {
    KV8_CACHE_ONCE.call_once(|| {
        let base = manifest_dir();
        for rel in [
            "lib/kv8/eval.kab",
            "lib/kv8/dom.kab",
            "lib/kv8/parser.kab",
            "lib/kv8/host.kab",
        ] {
            let path = format!("{base}/{rel}");
            compile::invalidate_file_cache(&path);
            kabootar_lib::modules::invalidate_module_export_cache(&path);
        }
    });
}

#[test]
fn kv8_eval_let_and_add() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let x = 1 + 2; x") == 3 && evalSource("function add(a, b) { return a + b; } add(3, 4)") == 7
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_eval_smoke_example_runs() {
    ensure_kv8_eval_cache_fresh();
    let path = format!("{}/examples/kv8_eval_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_eval_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n == 7));
}

#[test]
fn kv8_eval_member() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSourceWith("cfg.mode", { "cfg": { "mode": "ui" } }) == "ui"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_eval_while_inline_example_runs() {
    ensure_kv8_eval_cache_fresh();
    let path = format!("{}/examples/kv8_eval_while_inline.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_eval_while_inline.kab should run");
    assert!(matches!(result, Value::Number(n) if n == 0));
}

#[test]
fn kv8_eval_while_and_member() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let n = 0; while (n < 3) { n = n + 1; } n") == 3 &&
evalSourceWith("cfg.mode", { "cfg": { "mode": "ui" } }) == "ui"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_dom_ui_pipeline() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/dom"
let frame = evalUi("let r = el(\"div\"); let t = text(\"Hi\"); r = attach(r, t, true); paint(r, 240, 120, \"\");")
frame != null && frame["html"] != undefined
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_dom_smoke_example_runs() {
    ensure_kv8_eval_cache_fresh();
    let path = format!("{}/examples/kv8_dom_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_dom_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)));
}
