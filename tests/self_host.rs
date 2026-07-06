//! Self-hosted Kabootar compiler smoke tests (lexer + parser).

fn self_host_path(name: &str) -> String {
    format!("{}/self_host/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn kab_string_literal(s: &str) -> String {
    let mut out = String::from('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn kabootar_bin() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let debug = exe.parent().expect("deps").parent().expect("profile dir");
    if cfg!(windows) {
        debug.join("kabootar.exe")
    } else {
        debug.join("kabootar")
    }
}

fn run_kabootar_file_subprocess(path: &str) -> Result<(), String> {
    let bin = kabootar_bin();
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new(&bin)
        .current_dir(manifest)
        .arg(path)
        .output()
        .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "kabootar {} failed ({})\nstdout: {stdout}\nstderr: {stderr}",
        path,
        output.status
    ))
}

#[test]
fn self_host_lexer_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_lexer.kab"))
        .expect("self_host/test_lexer.kab should pass");
}

#[test]
fn self_host_parser_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_parser.kab"))
        .expect("self_host/test_parser.kab should pass");
}

#[test]
fn self_host_emit_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_emit.kab"))
        .expect("self_host/test_emit.kab should pass");
}

#[test]
fn self_host_serialize_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_serialize.kab"))
        .expect("self_host/test_serialize.kab should pass");
}

#[test]
fn self_host_serialize_compiles() {
    let path = self_host_path("serialize.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("serialize.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_emit_compiles() {
    let path = self_host_path("emit.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("emit.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_sample_runs() {
    use kabootar_lib::value::format_value;

    let v = kabootar_lib::cli::run_file(&self_host_path("sample.kab"))
        .expect("sample.kab should run under Rust interpreter");
    assert_eq!(format_value(&v), "42");
}

#[test]
fn self_host_subset_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_subset.kab"))
        .expect("self_host/test_subset.kab should pass");
}

#[test]
fn self_host_larger_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_larger.kab"))
        .expect("self_host/test_larger.kab should pass");
}

#[test]
fn self_host_mini_module_runs() {
    use kabootar_lib::value::format_value;

    let v = kabootar_lib::cli::run_file(&self_host_path("mini_module.kab"))
        .expect("mini_module.kab should run");
    assert_eq!(format_value(&v), "true");
}

#[test]
fn self_host_larger_compile_and_run() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::{Value, format_value};

    let v = kabootar_lib::cli::run_file(&self_host_path("larger_probe.kab"))
        .expect("larger_probe.kab should run");
    let Value::String(text) = v else {
        panic!("larger_probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize larger .kbc");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run self-hosted mini module");
    assert_eq!(format_value(&result), "true");
}

#[test]
fn self_host_m7_subset_suite() {
    // In-process run overflows test harness stack; kabootar.exe has 16 MiB (build.rs).
    run_kabootar_file_subprocess(&self_host_path("test_m7.kab"))
        .expect("self_host/test_m7.kab should pass");
}

#[test]
fn self_host_lexer_compile_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_lexer_compile.kab"))
        .expect("self_host/test_lexer_compile.kab should pass");
}

#[test]
fn self_host_lexer_compile_and_run() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::{Value, format_value};

    let v = kabootar_lib::cli::run_file(&self_host_path("lexer_compile_probe.kab"))
        .expect("lexer_compile_probe.kab should run");
    let Value::String(text) = v else {
        panic!("lexer_compile_probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize lexer .kbc");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run lexer-like loop bytecode");
    assert_eq!(format_value(&result), "2");
}

#[test]
fn self_host_lexer_full_compile_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_lexer_full_compile.kab"))
        .expect("self_host/test_lexer_full_compile.kab should pass");
}

#[test]
#[ignore = "slow (~15 min): self-hosted compile(lexer.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_lexer_full_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let probe_path = self_host_path("_lexer_full_probe_gen.kab");
    let src_copy = format!("{}/_lexer_full_src.kab", env!("CARGO_MANIFEST_DIR"));
    let out_file = format!("{}/_lexer_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    std::fs::copy(self_host_path("lexer.kab"), &src_copy).expect("copy lexer.kab for compile probe");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nlet kbc = compile(read_text_file(\"/proj/_lexer_full_src.kab\"))\nwrite_text_file(\"/proj/_lexer_full_out.kbc\", kbc)\nreturn len(kbc)",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write generated lexer full compile probe");

    run_kabootar_file_subprocess(&probe_path).expect("kabootar compile(lexer.kab) via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let kbc = std::fs::read_to_string(&out_file).expect("read compiled lexer .kbc output");
    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "lexer .kbc should have bytecode header"
    );
    let module = deserialize(&kbc).expect("deserialize compiled lexer.kab");
    assert!(!module.functions.is_empty(), "lexer should emit functions");
    let mut run_env = create_global_env();
    run_module(&module, &mut run_env).expect("run compiled lexer module");
    let tokenize = run_env
        .get("tokenize")
        .expect("compiled lexer should export tokenize");
    let toks = call_value(
        tokenize,
        vec![Value::String("ab".into())],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    )
    .expect("tokenize(\"ab\")");
    let Value::Array(items) = toks else {
        panic!("tokenize should return array, got {toks:?}");
    };
    assert_eq!(items.len(), 2, "tokenize(\"ab\") => [ident, eof]");
    let Value::Object(first) = &items[0] else {
        panic!("expected token object, got {:?}", items[0]);
    };
    assert_eq!(
        first.get("type").and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }),
        Some("Identifier")
    );
    assert_eq!(
        first.get("value").and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }),
        Some("ab")
    );
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
}

/// Fast regression: per-fn `load_global` indices must use module `globals`, not a
/// separate per-function table (otherwise `lxScan()` resolves to the wrong fn).
#[test]
fn self_host_emit_unified_globals_member_access() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nreturn compile(read_text_file(\"/proj/self_host/_emit_globals_mini.kab\"))",
        kab_string_literal(&manifest)
    );
    let probe_path = self_host_path("_emit_globals_run_probe_gen.kab");
    std::fs::write(&probe_path, probe).expect("write emit globals probe");
    let v = kabootar_lib::cli::run_file(&probe_path)
        .expect("compile _emit_globals_mini.kab via self-hosted pipeline");
    let _ = std::fs::remove_file(&probe_path);
    let Value::String(text) = v else {
        panic!("probe should return .kbc text, got {v:?}");
    };
    let module = deserialize(&text).expect("deserialize mini lexer .kbc");
    let mut env = create_global_env();
    run_module(&module, &mut env).expect("run mini lexer module");
    let tokenize = env.get("tokenize").expect("tokenize export");
    let toks = call_value(
        tokenize,
        vec![Value::String("ab".into())],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("tokenize should not fail on tok.type member access");
    let Value::Array(items) = toks else {
        panic!("tokenize should return array, got {toks:?}");
    };
    assert_eq!(items.len(), 1);
    let Value::Object(first) = &items[0] else {
        panic!("expected token object");
    };
    assert_eq!(
        first.get("type").and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }),
        Some("Identifier")
    );
}

#[test]
fn self_host_bootstrap_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_bootstrap.kab"))
        .expect("self_host/test_bootstrap.kab should pass");
}

#[test]
fn self_host_bootstrap_compile_and_run() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::{Value, format_value};

    let compile_path = self_host_path("compile.kab");
    let (_, bytecode) = kabootar_lib::cli::compile_file_report(&compile_path)
        .expect("kabootar compile self_host/compile.kab should succeed");
    assert!(bytecode, "compile.kab should produce bytecode cache");

    let v = kabootar_lib::cli::run_file(&self_host_path("bootstrap_probe.kab"))
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
    kabootar_lib::cli::run_file(&self_host_path("test_compile.kab"))
        .expect("self_host/test_compile.kab should pass");
}

#[test]
fn self_host_compile_compiles() {
    let path = self_host_path("compile.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("compile.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_parse_facade_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_parse_facade.kab"))
        .expect("self_host/test_parse_facade.kab should pass");
}

#[test]
fn self_host_parse_facade_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_tiny.kab"))
        .expect("self_host/test_tiny.kab should pass");
    kabootar_lib::cli::run_file(&self_host_path("roundtrip_probe.kab"))
        .expect("self_host/roundtrip_probe.kab should pass");
}

#[test]
fn self_host_parse_facade_compiles() {
    let path = self_host_path("parse.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("parse.kab should compile");
    assert!(n > 0);
    assert!(bytecode);
}

#[test]
fn self_host_lexer_compiles() {
    let path = self_host_path("lexer.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("lexer.kab should compile");
    assert!(n > 0);
    assert!(bytecode, "lexer.kab should emit bytecode");
}

#[test]
fn self_host_parser_compiles() {
    let path = self_host_path("parser.kab");
    let (n, bytecode) = kabootar_lib::cli::compile_file_report(&path)
        .expect("parser.kab should compile");
    assert!(n > 0);
    assert!(bytecode, "parser.kab should emit bytecode");
}

#[test]
fn self_host_kbc_roundtrip_main() {
    use kabootar_lib::bytecode::deserialize;
    use kabootar_lib::value::Value;

    let v = kabootar_lib::cli::run_file(&self_host_path("roundtrip_main_probe.kab"))
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
    use kabootar_lib::bytecode::deserialize;
    use kabootar_lib::value::Value;

    let v = kabootar_lib::cli::run_file(&self_host_path("roundtrip_fn_probe.kab"))
        .expect("roundtrip_fn_probe.kab should run");
    let Value::String(text) = v else {
        panic!("roundtrip_fn_probe should return serialized .kbc text");
    };
    let module = deserialize(&text).expect("deserialize fn .kbc from self-host");
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "add");
    assert_eq!(module.functions[0].params, vec!["a".to_string(), "b".to_string()]);
    assert!(!module.functions[0].code.is_empty());
    assert_eq!(module.main_code.last(), Some(&kabootar_lib::bytecode::Opcode::Halt));
}

#[test]
fn self_host_kbc_run_fn_call() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::{Value, format_value};

    let v = kabootar_lib::cli::run_file(&self_host_path("roundtrip_call_probe.kab"))
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
