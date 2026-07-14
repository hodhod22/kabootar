//! lib/kdom + lib/kstyle — Kabootar-language wrappers over native kDOM/KSS.

use kabootar_lib::cli;
use kabootar_lib::value::Value;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

#[test]
fn kdom_lib_smoke_example_runs() {
    let path = format!("{}/examples/kdom_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kdom_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n > 0));
}

#[test]
fn kdom_document_module_imports() {
    let code = r#"
import "kdom/document"
let n = el("span")
attr(n, "class", "x")
kdom_id(n)
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Number(n) if n > 0));
}

#[test]
fn kstyle_theme_apply_dark() {
    let code = r#"
import "kstyle/theme"
let n = applyDark()
len(kstyle_css()) > 10
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kdom_paint_styled_via_lib() {
    let code = r#"
import "kdom/document"
import "kstyle/parser"
parse("h1 { color: #8ab4f8; font-size: 24px; }")
let ui = domExtra("kml", "<html><body><h1>Lib</h1></body></html>", "")
let frame = paint(ui, 800, 600, "")
frame["html"] != undefined
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kstyle_kabootar_parse_sheet() {
    let code = r#"
import "kstyle/parse"
let rules = parseSheet("body { color: red; } .card { padding: 8px; }")
len(rules) == 2 && len(rules[0]["items"]) == 1 && rules[1]["selector"] == ".card"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kstyle_kabootar_parse_matches_native_count() {
    let code = r#"
import "kstyle/parse"
import "kstyle/parser"
let css = "body { color: red; } .card { padding: 8px; } h1 { font-size: 24px; }"
let kab = ruleCount(css)
let native = kstyle_parse(css)
kab == native
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kstyle_selectors_kabootar() {
    let code = r##"
import "kstyle/selectors"
matches("body", "body", "", "")
    && matches(".x", "div", "a x b", "")
    && matches("#main", "span", "", "main")
"##;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn kstyle_parse_smoke_example_runs() {
    let path = format!("{}/examples/kstyle_parse_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kstyle_parse_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n >= 2));
}
