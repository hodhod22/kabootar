//! Self-hosted Kabootar compiler smoke tests (lexer + parser).

fn self_host_path(name: &str) -> String {
    format!("{}/self_host/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn self_host_lexer_suite() {
    kabootar::cli::run_file(&self_host_path("test_lexer.kab"))
        .expect("self_host/test_lexer.kab should pass");
}

#[test]
fn self_host_parser_suite() {
    kabootar::cli::run_file(&self_host_path("test_parser.kab"))
        .expect("self_host/test_parser.kab should pass");
}

#[test]
fn self_host_emit_suite() {
    kabootar::cli::run_file(&self_host_path("test_emit.kab"))
        .expect("self_host/test_emit.kab should pass");
}

#[test]
fn self_host_serialize_suite() {
    kabootar::cli::run_file(&self_host_path("test_serialize.kab"))
        .expect("self_host/test_serialize.kab should pass");
}

#[test]
fn self_host_serialize_compiles() {
    let path = self_host_path("serialize.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("serialize.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_emit_compiles() {
    let path = self_host_path("emit.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("emit.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_sample_runs() {
    use kabootar::value::format_value;

    let v = kabootar::cli::run_file(&self_host_path("sample.kab"))
        .expect("sample.kab should run under Rust interpreter");
    assert_eq!(format_value(&v), "42");
}

#[test]
fn self_host_bootstrap_smoke() {
    kabootar::cli::run_file(&self_host_path("test_bootstrap.kab"))
        .expect("self_host/test_bootstrap.kab should pass");
}

#[test]
fn self_host_bootstrap_compile_and_run() {
    use kabootar::bytecode::{deserialize, run_module};
    use kabootar::evaluator::create_global_env;
    use kabootar::value::{Value, format_value};

    let compile_path = self_host_path("compile.kab");
    let (_, bytecode) = kabootar::cli::compile_file_report(&compile_path)
        .expect("kabootar compile self_host/compile.kab should succeed");
    assert!(bytecode, "compile.kab should produce bytecode cache");

    let v = kabootar::cli::run_file(&self_host_path("bootstrap_probe.kab"))
        .expect("bootstrap_probe.kab should run");
    let Value::String(text) = v else {
        panic!("bootstrap_probe should return .kbc text from self-hosted compile");
    };
    let module = deserialize(&text).expect("deserialize bootstrap .kbc");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run self-hosted compiled sample.kab");
    assert_eq!(
        format_value(&result),
        "42",
        "sample.kab (n=10; return n+32) should return 42, got {}",
        format_value(&result)
    );
}

#[test]
fn self_host_compile_suite() {
    kabootar::cli::run_file(&self_host_path("test_compile.kab"))
        .expect("self_host/test_compile.kab should pass");
}

#[test]
fn self_host_compile_compiles() {
    let path = self_host_path("compile.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("compile.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_parse_facade_suite() {
    kabootar::cli::run_file(&self_host_path("test_parse_facade.kab"))
        .expect("self_host/test_parse_facade.kab should pass");
}

#[test]
fn self_host_parse_facade_smoke() {
    kabootar::cli::run_file(&self_host_path("test_tiny.kab"))
        .expect("self_host/test_tiny.kab should pass");
    kabootar::cli::run_file(&self_host_path("roundtrip_probe.kab"))
        .expect("self_host/roundtrip_probe.kab should pass");
}

#[test]
fn self_host_parse_facade_compiles() {
    let path = self_host_path("parse.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("parse.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_lexer_compiles() {
    let path = self_host_path("lexer.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("lexer.kab should compile");
    assert!(n > 0);
    assert!(bytecode, "lexer.kab should emit bytecode");
}

#[test]
fn self_host_parser_compiles() {
    let path = self_host_path("parser.kab");
    let (n, bytecode) = kabootar::cli::compile_file_report(&path)
        .expect("parser.kab should compile");
    assert!(n > 0);
    assert!(bytecode, "parser.kab should emit bytecode");
}

#[test]
fn self_host_kbc_roundtrip_main() {
    use kabootar::bytecode::deserialize;
    use kabootar::value::Value;

    let v = kabootar::cli::run_file(&self_host_path("roundtrip_main_probe.kab"))
        .expect("roundtrip_main_probe.kab should run");
    let Value::String(text) = v else {
        panic!("roundtrip_probe should return serialized .kbc text");
    };
    let module = deserialize(&text).expect("Rust deserialize should accept self-hosted .kbc");
    assert_eq!(module.globals, vec!["x".to_string()]);
    assert_eq!(module.constants.len(), 1);
    assert!(module.main_code.len() >= 4);
    assert!(module.functions.is_empty());
}

#[test]
fn self_host_kbc_roundtrip_fn() {
    use kabootar::bytecode::deserialize;
    use kabootar::value::Value;

    let v = kabootar::cli::run_file(&self_host_path("roundtrip_fn_probe.kab"))
        .expect("roundtrip_fn_probe.kab should run");
    let Value::String(text) = v else {
        panic!("roundtrip_fn_probe should return serialized .kbc text");
    };
    let module = deserialize(&text).expect("deserialize fn .kbc from self-host");
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "add");
    assert_eq!(module.functions[0].params, vec!["a".to_string(), "b".to_string()]);
    assert!(!module.functions[0].code.is_empty());
    assert_eq!(module.main_code.last(), Some(&kabootar::bytecode::Opcode::Halt));
}

#[test]
fn self_host_kbc_run_fn_call() {
    use kabootar::bytecode::{deserialize, run_module};
    use kabootar::evaluator::create_global_env;
    use kabootar::value::{Value, format_value};

    let v = kabootar::cli::run_file(&self_host_path("roundtrip_call_probe.kab"))
        .expect("roundtrip_call_probe.kab should run");
    let Value::String(text) = v else {
        panic!("roundtrip_call_probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize call probe");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run self-hosted fn call bytecode");
    assert_eq!(
        format_value(&result),
        "3",
        "add(1,2) should return 3, got {}",
        format_value(&result)
    );
}
