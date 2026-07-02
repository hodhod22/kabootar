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
fn self_host_parse_facade_smoke() {
    kabootar::cli::run_file(&self_host_path("test_tiny.kab"))
        .expect("self_host/test_tiny.kab should pass");
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
