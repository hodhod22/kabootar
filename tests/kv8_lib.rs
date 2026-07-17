//! Fast kv8 tests — lexer + parser only (~1–2 min).
//! Slow eval/dom: `cargo test --test kv8_lib_slow -- --test-threads=1`

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
