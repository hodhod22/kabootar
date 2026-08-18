//! P10a — host pipeline phase times (lexer → parse → emit → serialize → deserialize → VM).
//! Self-host totals belong in a later leaf log; this gate keeps the *shape* of the profile.

use std::time::Instant;

use kabootar_lib::bytecode::{
    deserialize, deserialize_kbcb, run_module, serialize, serialize_kbcb, try_compile,
};
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

#[test]
fn p10_kbcb_roundtrip_and_load_times() {
    let tokens = tokenize(SRC).expect("lex");
    let stmts = Parser::with_eof(tokens)
        .parse_program()
        .expect("parse");
    let module = try_compile(&stmts).expect("emit");
    let text = serialize(&module);
    let bin = serialize_kbcb(&module);

    let t0 = Instant::now();
    let from_text = deserialize(&text).expect("text");
    let text_ms = ms(t0);

    let t0 = Instant::now();
    let from_bin = deserialize_kbcb(&bin).expect("kbcb");
    let bin_ms = ms(t0);

    eprintln!(
        "P10 kbcb vs text: text_bytes={} kbcb_bytes={} deserialize_text_ms={text_ms:.3} deserialize_kbcb_ms={bin_ms:.3}",
        text.len(),
        bin.len()
    );
    assert_eq!(from_text, from_bin, "kbcb payload must round-trip to the same module");
    assert!(bin.starts_with(b"KBCB"), "kbcb magic");
}

#[test]
fn p10_seed_kbc_load_vs_kbcb() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("self_host")
        .join("seed")
        .join("emit_impl.kab.kbc");
    let text = std::fs::read_to_string(&path).expect("read seed");
    let t0 = Instant::now();
    let module = deserialize(&text).expect("deserialize seed");
    let text_ms = ms(t0);
    let bin = serialize_kbcb(&module);
    let t0 = Instant::now();
    let loaded = deserialize_kbcb(&bin).expect("kbcb seed");
    let bin_ms = ms(t0);
    eprintln!(
        "P10 seed emit_impl.kab.kbc: text_bytes={} kbcb_bytes={} deser_text_ms={text_ms:.1} deser_kbcb_ms={bin_ms:.1} ops={}",
        text.len(),
        bin.len(),
        loaded.main_code.len()
    );
    assert_eq!(module.main_code.len(), loaded.main_code.len());
    assert!(
        text_ms < 30_000.0,
        "seed text deserialize should stay under 30s, got {text_ms:.1} ms"
    );
}

#[test]
fn p10_warm_self_host_subset_disk_cache() {
    use kabootar_lib::compile::{compile_file_prefer_cached, CompilePrefer};
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root
        .join("self_host")
        .join("parser_util_bump.kab")
        .to_string_lossy()
        .replace('\\', "/");
    kabootar_lib::compile::invalidate_file_cache(&path);
    let t0 = Instant::now();
    let (_p1, b1) = compile_file_prefer_cached(&path, CompilePrefer::Rust).expect("warm compile");
    let first_ms = ms(t0);
    kabootar_lib::compile::invalidate_memory_cache_for_tests(&path);
    let t0 = Instant::now();
    let (_p2, b2) = compile_file_prefer_cached(&path, CompilePrefer::Rust).expect("disk hit");
    let second_ms = ms(t0);
    eprintln!(
        "P10 self_host disk cache parser_util_bump: first={first_ms:.1}ms backend={b1} second={second_ms:.1}ms backend={b2}"
    );
    assert!(
        b2 == "disk-cache" || b2 == "cache" || b2 == "seed",
        "expected disk/memory cache on second rust compile, got {b2}"
    );
}

#[test]
#[ignore = "self-host toolchain import is minutes even in release; KABOOTAR_P10_PROFILE=1"]
fn p10_self_host_tiny_source_profile() {
    use kabootar_lib::compile::compile_source_self_host;
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_P10_PROFILE", "1");
        std::env::set_var("KABOOTAR_VM", "host");
    });

    let t0 = Instant::now();
    let prog = compile_source_self_host("return 1\n").expect("self-host tiny");
    let first_ms = ms(t0);
    let t0 = Instant::now();
    let prog2 = compile_source_self_host("return 2\n").expect("self-host tiny 2");
    let second_ms = ms(t0);
    eprintln!("P10 self-host tiny first_ms={first_ms:.1} second_ms={second_ms:.1}");
    assert!(
        prog.bytecode.is_some() && prog2.bytecode.is_some(),
        "self-host tiny should emit bytecode"
    );
    assert!(
        first_ms < 180_000.0,
        "self-host tiny (includes toolchain import) should stay under 180s CI, got {first_ms:.1} ms"
    );
    assert!(
        second_ms < first_ms || second_ms < 30_000.0,
        "second self-host compile should reuse toolchain env, first={first_ms:.1} second={second_ms:.1}"
    );
}
