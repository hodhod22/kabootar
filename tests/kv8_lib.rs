//! Fast kv8 tests — lexer + parser only (~1–2 min).
//! Slow eval/dom: `cargo test --test kv8_lib_slow -- --test-threads=1`

use kabootar_lib::cli;
use kabootar_lib::value::Value;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

/// File smokes use host bytecode VM (same as `kabootar test`). Kab-VM probe can
/// cache a bad prefer-Kab result after a cold `.kabootar/cache` and return Null.
fn run_file_host(path: &str) -> Value {
    let prev = std::env::var("KABOOTAR_VM").ok();
    std::env::set_var("KABOOTAR_VM", "host");
    let v = cli::run_file(path);
    match prev {
        Some(p) => std::env::set_var("KABOOTAR_VM", p),
        None => std::env::remove_var("KABOOTAR_VM"),
    }
    v.expect("run_file_host")
}

#[test]
fn kv8_lexer_module_imports() {
    let code = r#"
import "kv8/lexer"
let toks = tokenize("let x = 1")
len(toks) >= 4 && tokenType(toks[0]) == "let" && tokenType(toks[2]) == "="
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_lexer_modern_operators() {
    let code = r#"
import "kv8/lexer"
let toks = tokenize("a ?? b; c?.d; x => y")
let ok = false
let i = 0
while i < len(toks) {
    let t = tokenType(toks[i])
    if t == "??" { ok = true }
    i = i + 1
}
ok
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_lexer_strict_equality() {
    let code = r#"
import "kv8/lexer"
let toks = tokenize("a === b !== c")
tokenType(toks[1]) == "===" && tokenType(toks[3]) == "!=="
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_lexer_smoke_example_runs() {
    let path = format!("{}/examples/kv8_lexer_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_lexer_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n > 10));
}

#[test]
fn kv8_parser_let_add() {
    let code = r#"
import "kv8/parser"
let ast = parseSource("let x = 1 + 2")
ast.kind == "Program" && ast.body[0].kind == "Let" && ast.body[0].init.kind == "Binary"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_parser_function_and_arrow() {
    let code = r#"
import "kv8/parser"
let f = parseSource("function id(x) { return x; }")
let a = parseSource("(y) => y + 1")
f.body[0].kind == "Function" && a.body[0].expr.kind == "Arrow"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_parser_smoke_example_runs() {
    let path = format!("{}/examples/kv8_parser_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_parser_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n >= 2));
}

#[test]
fn kv8_parser_while_loop() {
    let code = r#"
import "kv8/parser"
let ast = parseSource("while (x < 2) { x = x + 1; }")
ast.body[0].kind == "While" && ast.body[0].body.body[0].kind == "Assign"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_parser_class_and_async() {
    let code = r#"
import "kv8/parser"
let c = parseSource("class A { constructor(x) { return x } get() { return 1 } }")
let a = parseSource("async function f() { return 1 }")
c.body[0].kind == "Class" && c.body[0].sym == "A" && len(c.body[0].methods) == 2 && a.body[0].kind == "Function" && a.body[0]["async"] == true
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn k1c_kabootar_eval_path() {
    let code = r#"
import "kv8/eval"
evalSourceKab("let n = 0; while (n < 3) { n = n + 1; } n") == 3 && evalSourceKab("1 + 2") == 3
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn k1d_class_new_kab_eval() {
    // Class/new/this path in kv8/eval is mutual-rec heavy; Windows test threads
    // default to a small stack and overflow. Run on a larger stack (Value is !Send).
    let path = format!("{}/examples/kv8_k1d_class_new.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1d-class-new".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || matches!(run_file_host(&path), Value::Bool(true)))
        .expect("spawn k1d thread")
        .join()
        .expect("k1d thread join");
    assert!(ok);
}

#[test]
fn k1d_static_kab_eval() {
    let path = format!("{}/examples/kv8_k1d_static.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1d-static".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || matches!(run_file_host(&path), Value::Bool(true)))
        .expect("spawn k1d-static thread")
        .join()
        .expect("k1d-static join");
    assert!(ok);
}

#[test]
fn k1d_super_kab_eval() {
    let path = format!("{}/examples/kv8_k1d_super.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1d-super".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || matches!(run_file_host(&path), Value::Bool(true)))
        .expect("spawn k1d-super thread")
        .join()
        .expect("k1d-super join");
    assert!(ok);
}

#[test]
fn k1e_extends_kab_eval() {
    let path = format!("{}/examples/kv8_k1e_extends.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1e-extends".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || matches!(run_file_host(&path), Value::Bool(true)))
        .expect("spawn k1e thread")
        .join()
        .expect("k1e thread join");
    assert!(ok);
}

#[test]
fn k1e_eval_source_prefers_kab() {
    let code = r#"
import "kv8/eval"
evalSource("1 + 2") == 3 && evalSource("class A { a() { return 1 } } class B extends A { } let x = new B(); x.a()") == 1
"#;
    let ok = std::thread::Builder::new()
        .name("k1e-eval-source".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let mut env = kabootar_lib::evaluator::create_global_env();
            let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
            matches!(v, Value::Bool(true))
        })
        .expect("spawn k1e evalSource thread")
        .join()
        .expect("k1e evalSource join");
    assert!(ok);
}

#[test]
fn k1f_async_promise_kab_eval() {
    let path = format!("{}/examples/kv8_k1f_async.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1f-async".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_k1f_async.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn k1f thread")
        .join()
        .expect("k1f thread join");
    assert!(ok);
}

#[test]
fn k1g_promise_then_microtask() {
    let path = format!("{}/examples/kv8_k1g_promise_then.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("k1g-promise-then".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_k1g_promise_then.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn k1g thread")
        .join()
        .expect("k1g thread join");
    assert!(ok);
}

#[test]
fn h4_prefer_kab_eval() {
    let path = format!("{}/examples/kv8_h4_prefer_kab.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h4-prefer-kab".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_h4_prefer_kab.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h4 thread")
        .join()
        .expect("h4 thread join");
    assert!(ok);
}

#[test]
fn h6_arrow_kab_eval() {
    let path = format!("{}/examples/kv8_h6_arrow.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h6-arrow".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_h6_arrow.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h6 thread")
        .join()
        .expect("h6 thread join");
    assert!(ok);
}

#[test]
fn h6_builtins_kab_eval() {
    let path = format!("{}/examples/kv8_h6_builtins.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h6-builtins".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_h6_builtins.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h6b thread")
        .join()
        .expect("h6b thread join");
    assert!(ok);
}

#[test]
fn h6_events_timers_kab_eval() {
    let path = format!("{}/examples/kv8_h6_events_timers.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h6-events".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_h6_events_timers.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h6e thread")
        .join()
        .expect("h6e thread join");
    assert!(ok);
}

#[test]
fn h6a_parity_kab_eval() {
    let path = format!("{}/examples/kv8_h6a_parity.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h6a-parity".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/kv8_h6a_parity.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h6a parity thread")
        .join()
        .expect("h6a parity thread join");
    assert!(ok);
}
