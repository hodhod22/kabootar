//! P10a — host pipeline phase times (lexer → parse → emit → serialize → deserialize → VM).
//! Self-host totals belong in a later leaf log; this gate keeps the *shape* of the profile.

use std::time::Instant;

use kabootar_lib::bytecode::{deserialize, run_module, serialize, try_compile};
use kabootar_lib::evaluator::create_global_env;
use kabootar_lib::lexer::tokenize;
use kabootar_lib::parser::Parser;

const SRC: &str = r#"
let n = { "kind": "lit", "value": 1, "left": null, "right": null }
let s = 0
let i = 0
while i < 64 {
    s = s + n["value"]
    if n["kind"] == "lit" {
        s = s + 1
    }
    i = i + 1
}
s
"#;

fn ms(t0: Instant) -> f64 {
    t0.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn p10_host_pipeline_phases_complete() {
    let t_all = Instant::now();

    let t0 = Instant::now();
    let tokens = tokenize(SRC).expect("lex");
    let lex_ms = ms(t0);

    let t0 = Instant::now();
    let stmts = Parser::with_eof(tokens)
        .parse_program()
        .expect("parse");
    let parse_ms = ms(t0);

    let t0 = Instant::now();
    let module = try_compile(&stmts).expect("emit bytecode");
    let emit_ms = ms(t0);

    let t0 = Instant::now();
    let text = serialize(&module);
    let ser_ms = ms(t0);

    let t0 = Instant::now();
    let loaded = deserialize(&text).expect("deserialize");
    let deser_ms = ms(t0);

    let t0 = Instant::now();
    let mut env = create_global_env();
    let v = run_module(&loaded, &mut env).expect("vm");
    let vm_ms = ms(t0);

    let total_ms = ms(t_all);
    eprintln!(
        "P10 host pipeline ms: lex={lex_ms:.3} parse={parse_ms:.3} emit={emit_ms:.3} \
         serialize={ser_ms:.3} deserialize={deser_ms:.3} vm={vm_ms:.3} total={total_ms:.3} kbc_bytes={}",
        text.len()
    );

    match v {
        kabootar_lib::value::Value::Number(n) => assert!(n >= 64, "got {n}"),
        other => panic!("expected number, got {other:?}"),
    }
    assert!(
        total_ms < 2000.0,
        "host snippet pipeline should stay under 2s CI, got {total_ms:.1} ms"
    );
}

#[test]
fn p10_rust_compile_parser_session_core_leaf() {
    use kabootar_lib::compile::{compile_file_prefer_cached, CompilePrefer};
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("self_host")
        .join("parser_session_core.kab");
    let path_s = path.to_string_lossy().to_string();
    kabootar_lib::compile::invalidate_file_cache(&path_s);

    let t0 = Instant::now();
    let (_prog, backend) =
        compile_file_prefer_cached(&path_s, CompilePrefer::Rust).expect("compile leaf");
    let leaf_ms = ms(t0);
    eprintln!("P10 rust compile parser_session_core.kab: {leaf_ms:.1} ms backend={backend}");
    assert!(
        leaf_ms < 60_000.0,
        "parser_session_core rust compile should stay under 60s CI, got {leaf_ms:.1} ms"
    );
}
