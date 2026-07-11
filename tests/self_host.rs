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

fn tokenize_via_interpreter(src: &str) -> kabootar_lib::value::Value {
    let probe_src = format!(
        "import \"self_host/lexer\"\nreturn tokenize({})",
        kab_string_literal(src)
    );
    let probe_path = self_host_path("_tokenize_helper_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write tokenize helper probe");
    let v = kabootar_lib::cli::run_file(&probe_path).expect("tokenize helper should run");
    let _ = std::fs::remove_file(&probe_path);
    v
}

fn parse_via_interpreter(src: &str) -> kabootar_lib::value::Value {
    let probe_src = format!(
        "import \"self_host/parse\"\nreturn parse({})",
        kab_string_literal(src)
    );
    let probe_path = self_host_path("_parse_helper_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write parse helper probe");
    let v = kabootar_lib::cli::run_file(&probe_path).expect("parse helper should run");
    let _ = std::fs::remove_file(&probe_path);
    v
}

fn ast_kind<'a>(v: &'a kabootar_lib::value::Value) -> Option<&'a str> {
    let kabootar_lib::value::Value::Object(map) = v else {
        return None;
    };
    map.get("kind").and_then(|k| match k {
        kabootar_lib::value::Value::String(s) => Some(s.as_str()),
        _ => None,
    })
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
fn self_host_bracket_index_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let snippet = "let KEYWORDS = { \"if\": \"KW_IF\" }\nfn f(id) { return KEYWORDS[id] }\nreturn f";
    let probe_src = format!(
        "import \"self_host/compile\"\nreturn compile({})",
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_bracket_index_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write bracket index probe");
    let v = kabootar_lib::cli::run_file(&probe_path)
        .expect("bracket index probe should run");
    let _ = std::fs::remove_file(&probe_path);
    let Value::String(text) = v else {
        panic!("bracket index probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize bracket index snippet .kbc");
    let mut env = create_global_env();
    run_module(&module, &mut env).expect("run bracket index snippet");
    let f = env.get("f").expect("f export");
    let hit = call_value(
        f,
        vec![Value::String("if".into())],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("KEYWORDS[id] with string index");
    assert_eq!(
        kabootar_lib::value::format_value(&hit),
        "KW_IF",
        "bracket index should read object map"
    );
}

#[test]
fn self_host_undefined_eq_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let snippet = "let KEYWORDS = { \"if\": \"KW_IF\" }\nfn f(id) {\n  let kw = KEYWORDS[id]\n  if kw == undefined {\n    return \"miss\"\n  }\n  return \"hit\"\n}\nreturn f";
    let probe_src = format!(
        "import \"self_host/compile\"\nreturn compile({})",
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_undefined_eq_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write undefined eq probe");
    let v = kabootar_lib::cli::run_file(&probe_path)
        .expect("undefined eq probe should run");
    let _ = std::fs::remove_file(&probe_path);
    let Value::String(text) = v else {
        panic!("undefined eq probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize undefined eq snippet .kbc");
    let mut env = create_global_env();
    run_module(&module, &mut env).expect("run undefined eq snippet");
    let f = env.get("f").expect("f export");
    let miss = call_value(
        f.clone(),
        vec![Value::String("ab".into())],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("missing key should compare equal to undefined");
    assert_eq!(
        kabootar_lib::value::format_value(&miss),
        "miss",
        "KEYWORDS[\"ab\"] == undefined must be true"
    );
    let hit = call_value(
        f,
        vec![Value::String("if".into())],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("present key should not match undefined");
    assert_eq!(
        kabootar_lib::value::format_value(&hit),
        "hit",
        "KEYWORDS[\"if\"] == undefined must be false"
    );
}

/// Fast regression: self-hosted compiled EOF loop must stop before extra parseStmt.
#[test]
fn self_host_parser_eof_loop_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;

    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let out_file = format!("{}/_parser_eof_loop_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&out_file);
    let snippet = "let pPos = 0\nlet pToks = []\nlet pDone = 0\nlet pTok = null\nlet pBody = []\nlet pVal = null\nfn peek() {\n  if pPos >= len(pToks) {\n    return { type: \"EOF\", value: null, line: 1, column: 1 }\n  }\n  return pToks[pPos]\n}\nfn bump() {\n  pTok = peek()\n  if pPos < len(pToks) {\n    pPos = pPos + 1\n  }\n  return pTok\n}\nfn parseStmt() {\n  if peek().type == \"EOF\" {\n    throw \"parseStmt EOF\"\n  }\n  while peek().type != \"EOF\" && pPos < len(pToks) {\n    bump()\n  }\n  return 1\n}\npub fn countStmts(tokens) {\n  pToks = tokens\n  pPos = 0\n  pBody = []\n  pDone = 0\n  while pDone == 0 {\n    if pPos >= len(pToks) {\n      pDone = 1\n    }\n    if pDone == 0 {\n      pTok = peek()\n      if pTok.type == \"EOF\" {\n        pDone = 1\n      }\n    }\n    if pDone == 0 {\n      pVal = parseStmt()\n      if pVal != null {\n        pBody = push(pBody, pVal)\n      }\n    }\n  }\n  return len(pBody)\n}\nreturn countStmts";
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nwrite_text_file(\"/proj/_parser_eof_loop_out.kbc\", compile({}))\nreturn 1",
        kab_string_literal(&manifest),
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_parser_eof_loop_probe_gen.kab");
    std::fs::write(&probe_path, probe).expect("write parser eof loop probe");
    run_kabootar_file_subprocess(&probe_path).expect("compile parser eof loop snippet via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let text = std::fs::read_to_string(&out_file).expect("read parser eof loop .kbc");
    let _ = std::fs::remove_file(&out_file);
    let module = deserialize(&text).expect("deserialize parser eof loop snippet .kbc");
    let mut env = create_global_env();
    run_module(&module, &mut env).expect("run parser eof loop snippet");
    let count_leading = env.get("countStmts").expect("countStmts export");
    let tokens = tokenize_via_interpreter("let x = 1");
    let n = call_value(
        count_leading,
        vec![tokens],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("countStmts should not call parseStmt at EOF");
    assert_eq!(
        kabootar_lib::value::format_value(&n),
        "1",
        "let x = 1 => one stmt before EOF"
    );
}

/// Fast regression: chained `+` in fn bodies must not clobber nested emitExpr (throw/debug strings).
#[test]
fn self_host_emit_binary_concat_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;

    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let out_file = format!("{}/_emit_binary_concat_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&out_file);
    let snippet = "pub fn concatPos(n) { return \"pos \" + (\"\" + n) }\nreturn concatPos";
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nwrite_text_file(\"/proj/_emit_binary_concat_out.kbc\", compile({}))\nreturn 1",
        kab_string_literal(&manifest),
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_emit_binary_concat_probe_gen.kab");
    std::fs::write(&probe_path, probe).expect("write emit binary concat probe");
    run_kabootar_file_subprocess(&probe_path).expect("compile binary concat snippet via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let text = std::fs::read_to_string(&out_file).expect("read binary concat .kbc");
    let _ = std::fs::remove_file(&out_file);
    let module = deserialize(&text).expect("deserialize binary concat snippet .kbc");
    let mut env = create_global_env();
    run_module(&module, &mut env).expect("run binary concat snippet");
    let concat_pos = env.get("concatPos").expect("concatPos export");
    let out = call_value(
        concat_pos,
        vec![kabootar_lib::value::Value::Number(4)],
        &[],
        &[],
        &[],
        &[],
        &mut env,
    )
    .expect("concatPos should not add objects");
    assert_eq!(
        kabootar_lib::value::format_value(&out),
        "pos 4",
        "chained + in fn body"
    );
}

/// Fast regression: `arr[i] = rhs` must save rhs before emitExpr clobbers `eNode`.
#[test]
fn self_host_emit_index_assign_compile_and_run() {
    use kabootar_lib::value::Value;

    kabootar_lib::cli::run_file(&self_host_path("_emit_index_assign_only_probe.kab"))
        .expect("emit(a[i]=1) must not read rhs from clobbered eNode");

    let v = kabootar_lib::cli::run_file(&self_host_path("_emit_index_assign_compile_probe.kab"))
        .expect("compile(index assign jump-patch snippet) should succeed");
    let Value::Number(n) = v else {
        panic!("index assign compile probe should return .kbc length");
    };
    assert!(n > 100, "index assign snippet should produce non-trivial .kbc");
}

#[test]
fn self_host_nested_break_scope_compile_and_run() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let snippet = "let x = 0\nwhile true {\n  while true {\n    break\n  }\n  x = 1\n  break\n}\nreturn x";
    let probe_src = format!(
        "import \"self_host/compile\"\nreturn compile({})",
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_nested_break_scope_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write nested break scope probe");
    let v = kabootar_lib::cli::run_file(&probe_path)
        .expect("nested break scope probe should run");
    let _ = std::fs::remove_file(&probe_path);
    let Value::String(text) = v else {
        panic!("nested break probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize nested break snippet .kbc");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run nested break snippet");
    assert_eq!(
        kabootar_lib::value::format_value(&result),
        "1",
        "inner break must not break outer loop"
    );
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

#[test]
fn self_host_parser_full_compile_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_parser_full_compile.kab"))
        .expect("self_host/test_parser_full_compile.kab should pass");
}

#[test]
fn self_host_emit_full_compile_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_emit_full_compile.kab"))
        .expect("self_host/test_emit_full_compile.kab should pass");
}

#[test]
#[ignore = "slow (~2-3h): self-hosted compile(emit.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_emit_full_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::runtime::stdlib::error::format_runtime_error;
    use kabootar_lib::value::Value;

    let probe_path = self_host_path("_emit_full_probe_gen.kab");
    let src_copy = format!("{}/_emit_full_src.kab", env!("CARGO_MANIFEST_DIR"));
    let out_file = format!("{}/_emit_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    std::fs::copy(self_host_path("emit.kab"), &src_copy).expect("copy emit.kab for compile probe");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nlet kbc = compile(read_text_file(\"/proj/_emit_full_src.kab\"))\nwrite_text_file(\"/proj/_emit_full_out.kbc\", kbc)\nreturn len(kbc)",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write generated emit full compile probe");

    run_kabootar_file_subprocess(&probe_path).expect("kabootar compile(emit.kab) via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let kbc = std::fs::read_to_string(&out_file).expect("read compiled emit .kbc output");
    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "emit .kbc should have bytecode header"
    );
    let module = deserialize(&kbc).expect("deserialize compiled emit.kab");
    assert!(!module.functions.is_empty(), "emit should emit functions");
    let mut run_env = create_global_env();
    run_module(&module, &mut run_env).expect("run compiled emit module");
    let emit_fn = run_env.get("emit").expect("compiled emit should export emit");
    let ast = parse_via_interpreter("let x = 1");
    assert_eq!(ast_kind(&ast), Some("Program"), "parse let x = 1 root kind");
    let bc = match call_value(
        emit_fn,
        vec![ast],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    ) {
        Ok(v) => v,
        Err(e) => {
            let msg = format_runtime_error(&e);
            panic!("emit(parse(\"let x = 1\")) threw: {msg}");
        }
    };
    let Value::Object(ir) = bc else {
        panic!("emit should return IR object, got {bc:?}");
    };
    let Value::Array(globals) = ir.get("globals").expect("emit IR globals") else {
        panic!("emit globals should be array");
    };
    assert_eq!(globals.len(), 1, "let x = 1 => one global");
    assert_eq!(
        match &globals[0] {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        },
        Some("x"),
        "let global name"
    );
    let Value::Array(ops) = ir.get("ops").expect("emit IR ops") else {
        panic!("emit ops should be array");
    };
    assert!(ops.len() >= 4, "let x = 1 should emit several main ops");
    let Value::Object(last_op) = ops.last().expect("emit ops non-empty") else {
        panic!("emit op should be object");
    };
    assert_eq!(
        last_op.get("op").and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }),
        Some("halt"),
        "main ops should end with halt"
    );
    let has_store_global = ops.iter().any(|op| {
        let Value::Object(obj) = op else {
            return false;
        };
        matches!(obj.get("op"), Some(Value::String(s)) if s == "store_global")
    });
    assert!(has_store_global, "let x = 1 should emit store_global");
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
}

#[test]
#[ignore = "slow (~2h): self-hosted compile(parser.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_parser_full_compile_and_run() {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;
    use kabootar_lib::runtime::stdlib::error::format_runtime_error;

    let probe_path = self_host_path("_parser_full_probe_gen.kab");
    let src_copy = format!("{}/_parser_full_src.kab", env!("CARGO_MANIFEST_DIR"));
    let out_file = format!("{}/_parser_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    std::fs::copy(self_host_path("parser.kab"), &src_copy).expect("copy parser.kab for compile probe");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nlet kbc = compile(read_text_file(\"/proj/_parser_full_src.kab\"))\nwrite_text_file(\"/proj/_parser_full_out.kbc\", kbc)\nreturn len(kbc)",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write generated parser full compile probe");

    run_kabootar_file_subprocess(&probe_path).expect("kabootar compile(parser.kab) via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let kbc = std::fs::read_to_string(&out_file).expect("read compiled parser .kbc output");
    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "parser .kbc should have bytecode header"
    );
    let module = deserialize(&kbc).expect("deserialize compiled parser.kab");
    assert!(!module.functions.is_empty(), "parser should emit functions");
    let mut run_env = create_global_env();
    run_module(&module, &mut run_env).expect("run compiled parser module");
    let parse_tokens = run_env
        .get("parseTokens")
        .expect("compiled parser should export parseTokens");
    let tokens = tokenize_via_interpreter("let x = 1");
    let ast = match call_value(
        parse_tokens,
        vec![tokens],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    ) {
        Ok(v) => v,
        Err(e) => {
            let msg = format_runtime_error(&e);
            panic!("parseTokens(tokenize(\"let x = 1\")) threw: {msg}");
        }
    };
    assert_eq!(ast_kind(&ast), Some("Program"), "parseTokens root kind");
    let Value::Object(root) = ast else {
        panic!("parseTokens should return object AST");
    };
    let Value::Array(body) = root.get("body").expect("program body") else {
        panic!("program body should be array");
    };
    assert_eq!(body.len(), 1, "let x = 1 => one stmt");
    assert_eq!(ast_kind(&body[0]), Some("LetStmt"), "first stmt should be let");
    let Value::Object(let_stmt) = &body[0] else {
        panic!("let stmt should be object");
    };
    assert_eq!(
        let_stmt.get("sym").and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }),
        Some("x"),
        "let sym"
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
    let probe_path = self_host_path("_emit_globals_run_probe_gen.kab");
    let out_file = format!("{}/_emit_globals_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&out_file);
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nwrite_text_file(\"/proj/_emit_globals_out.kbc\", compile(read_text_file(\"/proj/self_host/_emit_globals_mini.kab\")))\nreturn 1",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write emit globals probe");
    run_kabootar_file_subprocess(&probe_path)
        .expect("compile _emit_globals_mini.kab via subprocess");
    let _ = std::fs::remove_file(&probe_path);
    let text = std::fs::read_to_string(&out_file).expect("read compiled mini lexer .kbc");
    let _ = std::fs::remove_file(&out_file);
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
    assert_eq!(items.len(), 2, "tokenize should return [ident, eof]");
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

/// Profile: deserialize / run_module / emit(parse) after compile(emit.kab).
/// Run: cargo test --test self_host self_host_emit_profile_run_phases -- --ignored --nocapture
#[test]
#[ignore = "profile: requires _emit_full_out.kbc from emit full compile"]
fn self_host_emit_profile_run_phases() {
    use std::io::Write;
    use std::time::Instant;

    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    fn profile_step(msg: &str) {
        eprintln!("PROFILE step {msg}");
        let _ = std::io::stderr().flush();
    }

    let out_file = format!("{}/_emit_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    profile_step("read_kbc_start");
    let kbc = std::fs::read_to_string(&out_file)
        .unwrap_or_else(|e| panic!("read {out_file}: {e} (run emit full compile first)"));
    profile_step(&format!("read_kbc_done bytes={}", kbc.len()));

    let t0 = Instant::now();
    profile_step("deserialize_start");
    let module = deserialize(&kbc).expect("deserialize compiled emit.kab");
    let t1 = Instant::now();
    profile_step(&format!(
        "deserialize_done ms={} fn_count={}",
        (t1 - t0).as_millis(),
        module.functions.len()
    ));

    let mut run_env = create_global_env();
    profile_step("run_module_start");
    run_module(&module, &mut run_env).expect("run compiled emit module");
    let t2 = Instant::now();
    profile_step(&format!("run_module_done ms={}", (t2 - t1).as_millis()));

    let emit_fn = run_env.get("emit").expect("compiled emit should export emit");
    profile_step("parse_ast_start");
    let ast = parse_via_interpreter("let x = 1");
    let t2b = Instant::now();
    profile_step(&format!("parse_ast_done ms={}", (t2b - t2).as_millis()));

    profile_step("emit_call_start");
    let bc = call_value(
        emit_fn,
        vec![ast],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    )
    .expect("emit(parse(\"let x = 1\"))");
    let t3 = Instant::now();
    profile_step(&format!("emit_call_done ms={}", (t3 - t2b).as_millis()));

    let Value::Object(ir) = bc else {
        panic!("emit should return IR object");
    };
    let ops_len = ir
        .get("ops")
        .and_then(|v| match v {
            Value::Array(a) => Some(a.len()),
            _ => None,
        })
        .unwrap_or(0);

    eprintln!("PROFILE run.deserialize_ms {}", (t1 - t0).as_millis());
    eprintln!("PROFILE run.run_module_ms {}", (t2 - t1).as_millis());
    eprintln!("PROFILE run.parse_ast_ms {}", (t2b - t2).as_millis());
    eprintln!("PROFILE run.emit_call_ms {}", (t3 - t2b).as_millis());
    eprintln!("PROFILE run.total_ms {}", (t3 - t0).as_millis());
    eprintln!("PROFILE meta.fn_count {}", module.functions.len());
    eprintln!("PROFILE meta.emit_ops {}", ops_len);
}

/// Profile: self-hosted parse/emit/serialize on a tiny snippet (sanity check for profiler).
#[test]
fn self_host_profile_phases_smoke() {
    let snippet = "let x = 1\nreturn x";
    let probe = format!(
        "import \"self_host/parse\"\nimport \"self_host/emit\"\nimport \"self_host/serialize\"\n\
         let src = {}\nlet t0 = date_now_ms()\nlet ast = parse(src)\nlet t1 = date_now_ms()\n\
         let ir = emit(ast)\nlet t2 = date_now_ms()\nlet kbc = serialize_bc(ir)\nlet t3 = date_now_ms()\n\
         if t3 < t0 {{ throw \"bad clock\" }}\nreturn len(kbc)",
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_profile_phases_smoke_gen.kab");
    std::fs::write(&probe_path, probe).expect("write profile smoke probe");
    let v = kabootar_lib::cli::run_file(&probe_path).expect("profile phases smoke");
    let _ = std::fs::remove_file(&probe_path);
    let kabootar_lib::value::Value::Number(n) = v else {
        panic!("profile smoke should return kbc length");
    };
    assert!(n > 0, "profile smoke kbc len");
}

/// Self-host compile must not emit push(stack, len(x)) — use pushLen helper pattern.
#[test]
fn self_host_push_len_compile_and_run() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;

    let snippet = "let eLenScratch = 0\nfn pushLen(stack, arr) {\n  eLenScratch = len(arr)\n  return push(stack, eLenScratch)\n}\nlet s = []\nlet p = { \"body\": [1] }\ns = pushLen(s, p[\"body\"])\nreturn s[0]";
    let probe_src = format!(
        "import \"self_host/compile\"\nreturn compile({})",
        kab_string_literal(snippet)
    );
    let probe_path = self_host_path("_push_len_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write pushLen compile probe");
    let v = kabootar_lib::cli::run_file(&probe_path)
        .expect("pushLen compile probe should run");
    let _ = std::fs::remove_file(&probe_path);
    let kabootar_lib::value::Value::String(text) = v else {
        panic!("pushLen probe should return .kbc text");
    };
    let module = deserialize(&text).expect("deserialize pushLen snippet");
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run pushLen snippet");
    assert_eq!(
        kabootar_lib::value::format_value(&result),
        "1",
        "pushLen should store body length"
    );
}

/// Rust-compiled emit (import) on let x = 1 — should finish; compare to self-compiled .kbc profile test.
#[test]
fn self_host_emit_rust_bytecode_let_probe() {
    let v = kabootar_lib::cli::run_file(&self_host_path("_emit_interpreted_let_probe.kab"))
        .expect("rust-bytecode emit(parse let x=1) should complete");
    let kabootar_lib::value::Value::Number(n) = v else {
        panic!("emit let probe should return op count");
    };
    assert!(n >= 3, "let x = 1 should emit at least const/store/halt path, got {n} ops");
}

