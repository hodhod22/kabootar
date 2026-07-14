//! lib/kv8 — Kabootar-language Kv8 lexer (G9 start).

use kabootar_lib::cli;
use kabootar_lib::value::Value;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
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
fn kv8_eval_let_and_add() {
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
    let path = format!("{}/examples/kv8_eval_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_eval_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n == 7));
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
fn kv8_eval_while_and_member() {
    let code = r#"
import "kv8/eval"
import "kv8/parser"
evalSource("let n = 0; while (n < 3) { n = n + 1; } n") == 3 &&
evalProgram(parseSource("cfg.mode"), { "cfg": { "mode": "ui" } }) == "ui"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kv8_dom_ui_pipeline() {
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
    let path = format!("{}/examples/kv8_dom_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kv8_dom_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)));
}
