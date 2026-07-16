//! Slow kv8 tests — eval + dom (heavy `import "kv8/eval"` chain).
//! Run: `cargo test --test kv8_lib_slow -- --test-threads=1`

use kabootar_lib::cli;
use kabootar_lib::compile;
use kabootar_lib::value::Value;
use std::sync::Once;
use std::time::Instant;

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
            "lib/kv8/react.kab",
            "lib/kv8/parser.kab",
            "lib/kv8/host.kab",
            "lib/kv8/defs.kab",
            "lib/kv8/lexer.kab",
        ] {
            let path = format!("{base}/{rel}");
            compile::invalidate_file_cache(&path);
            kabootar_lib::modules::invalidate_module_export_cache(&path);
        }
    });
}

#[test]
fn kv8_eval_import_chain() {
    ensure_kv8_eval_cache_fresh();
    let mut env = kabootar_lib::evaluator::create_global_env();
    for module in ["kv8/defs", "kv8/host", "kv8/parser", "kv8/eval"] {
        let started = Instant::now();
        kabootar_lib::evaluator::eval_source(&format!("import \"{module}\""), &mut env)
            .unwrap_or_else(|error| panic!("{module} import failed: {error}"));
        eprintln!("{module} import completed in {:?}", started.elapsed());
    }
}

#[test]
fn kv8_eval_let_and_add() {
    ensure_kv8_eval_cache_fresh();
    let mut env = kabootar_lib::evaluator::create_global_env();
    let import_started = Instant::now();
    kabootar_lib::evaluator::eval_source("import \"kv8/eval\"", &mut env).unwrap();
    eprintln!("kv8/eval import completed in {:?}", import_started.elapsed());

    let basic_started = Instant::now();
    let basic = kabootar_lib::evaluator::eval_source(
        "evalSource(\"let x = 1 + 2; x\")",
        &mut env,
    )
    .unwrap();
    eprintln!("kv8 basic evalSource completed in {:?}", basic_started.elapsed());
    assert!(matches!(basic, Value::Number(n) if n == 3));

    let function_started = Instant::now();
    let function = kabootar_lib::evaluator::eval_source(
        "evalSource(\"function add(a, b) { return a + b; } add(3, 4)\")",
        &mut env,
    )
    .unwrap();
    eprintln!("kv8 function evalSource completed in {:?}", function_started.elapsed());
    assert!(matches!(function, Value::Number(n) if n == 7));
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

/// Fas 1.3 — ops already produced by kv8/parser (*, /, !=, <=, >=, ??).
#[test]
fn kv8_eval_ops_extended() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let x = 2 * 3 + 4 / 2; x") == 8 &&
evalSource("let a = 1; a != 2 && a !== 2 && a <= 1 && a >= 1") == true &&
evalSource("let u = undefined; let v = u ?? 9; v") == 9 &&
evalSource("let z = 0; z ?? 5") == 0
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

/// Short-circuit: RHS must not run when LHS decides the result.
#[test]
fn kv8_eval_short_circuit() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("false && missing") == false &&
evalSource("true || missing") == true &&
evalSource("0 ?? missing") == 0 &&
evalSource("null ?? 7") == 7
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

/// C-style for + try/catch/throw.
#[test]
fn kv8_eval_for_and_try() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let s = 0; for (let i = 0; i < 3; i = i + 1) { s = s + i; } s") == 3 &&
evalSource("try { throw \"boom\"; } catch (e) { e }") == "boom"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

/// break/continue in for and while loops.
#[test]
fn kv8_eval_break_continue() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let s = 0; for (let i = 0; i < 10; i = i + 1) { if (i == 3) { break; } s = s + i; } s") == 3 &&
evalSource("let s = 0; for (let i = 0; i < 5; i = i + 1) { if (i == 2) { continue; } s = s + i; } s") == 8 &&
evalSource("let n = 0; let s = 0; while (n < 5) { n = n + 1; if (n == 3) { continue; } s = s + n; } s") == 12
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

/// finally + for-in.
#[test]
fn kv8_eval_finally_and_for_in() {
    ensure_kv8_eval_cache_fresh();
    let code = r#"
import "kv8/eval"
evalSource("let f = 0; try { throw \"x\"; } catch (e) { f = 1; } finally { f = f + 10; } f") == 11 &&
evalSource("let n = 0; for (let k in { \"a\": 1, \"b\": 2 }) { if (k != \"__oid\") { n = n + 1; } } n") == 2
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

#[test]
fn kv8_react_smoke_example_runs() {
    ensure_kv8_eval_cache_fresh();
    let path = format!("{}/examples/kv8_react_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_react_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)));
}
