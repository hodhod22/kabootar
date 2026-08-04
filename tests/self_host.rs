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

/// Pre pop()-refactor emit.kab compiled to bytecode with manual stack-copy loops in popStack.
/// Running emit() from such a .kbc appears to hang (hours of CPU on emit_call).
fn assert_fresh_emit_kbc(kbc: &str) {
    let stale_popstack = kbc.contains("fn 0 popStack\nfn_params 0 stack\nfn_locals 0 stack,newStack,i")
        && kbc.contains("fn_op 0 jump -20");
    if stale_popstack {
        panic!(
            "stale _emit_full_out.kbc (pre pop() refactor in emit.kab): emit_call will not finish in reasonable time.\n\
             Rebuild (~4h): python scripts/profile_emit_compile.py compile emit.kab\n\
             Or full M10 (compile+run): cargo test --test self_host self_host_emit_full_compile_and_run -- --ignored --test-threads=1"
        );
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

/// Compile lexer/parser/emit once per test run (re-compiling per section OOMs on Windows).
fn load_self_host_emit_programs() -> Result<Vec<kabootar_lib::compile::CompiledProgram>, String> {
    use kabootar_lib::compile::compile_file_cached;

    let mut out = Vec::with_capacity(3);
    for mod_file in ["lexer.kab", "parser.kab", "emit.kab"] {
        let path = self_host_path(mod_file);
        let program = compile_file_cached(&path)?;
        if !program.has_bytecode() {
            return Err(format!("{mod_file} must compile to bytecode"));
        }
        out.push(program);
    }
    Ok(out)
}

/// Load precompiled self_host modules into `env`.
fn preload_self_host_emit_deps(
    env: &mut kabootar_lib::value::Environment,
    programs: &[kabootar_lib::compile::CompiledProgram],
) -> Result<(), String> {
    use kabootar_lib::compile::eval_program;

    for program in programs {
        eval_program(program, env)?;
    }
    Ok(())
}

/// Run one emit test section with fresh env + preloaded bytecode modules.
fn run_emit_section(
    title: &str,
    body: &str,
    programs: &[kabootar_lib::compile::CompiledProgram],
) -> Result<(), String> {
    use kabootar_lib::evaluator::{create_global_env, eval_source};

    let header = r#"import "self_host/emit_defs"
import "self_host/ast_defs"
let passed = 0
let fail = 0
let tI = 0
let tHas = 0
fn assert_eq(a, b, msg) {
    if a == b {
        passed = passed + 1
    } else {
        fail = fail + 1
        println("FAIL: " + msg + " -- expected " + json_stringify(b) + ", got " + json_stringify(a))
    }
}
fn assert_true(v, msg) {
    if v {
        passed = passed + 1
    } else {
        fail = fail + 1
        println("FAIL: " + msg)
    }
}
"#;
    let mut env = create_global_env();
    preload_self_host_emit_deps(&mut env, programs)?;
    let src = format!(
        "{header}{body}\nif fail > 0 {{ throw \"EMIT FAIL: {title}\" }}\n"
    );
    eval_source(&src, &mut env).map_err(|e| format!("{title}: {e}"))?;
    Ok(())
}

/// Run one emit section in a fresh kabootar subprocess (frees heap between sections).
fn run_emit_section_subprocess(title: &str, body: &str) -> Result<(), String> {
    let header = r#"import "self_host/lexer"
import "self_host/parser"
import "self_host/emit"
import "self_host/emit_defs"
import "self_host/ast_defs"
let passed = 0
let fail = 0
let tI = 0
let tHas = 0
fn assert_eq(a, b, msg) {
    if a == b {
        passed = passed + 1
    } else {
        fail = fail + 1
        println("FAIL: " + msg + " -- expected " + json_stringify(b) + ", got " + json_stringify(a))
    }
}
fn assert_true(v, msg) {
    if v {
        passed = passed + 1
    } else {
        fail = fail + 1
        println("FAIL: " + msg)
    }
}
"#;
    let probe = format!(
        "{header}{body}\nif fail > 0 {{ throw \"EMIT FAIL: {title}\" }}\nreturn 1\n"
    );
    let safe: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let path = self_host_path(&format!("_emit_section_{safe}.kab"));
    std::fs::write(&path, probe).map_err(|e| format!("write section probe {title}: {e}"))?;
    let result = run_kabootar_file_subprocess(&path);
    let _ = std::fs::remove_file(&path);
    result.map_err(|e| format!("{title}: {e}"))
}

/// Run `test_emit.kab` sections (subprocess per section; emit.kab must stay <=7 top-level fn).
fn run_emit_test_suite() -> Result<(), String> {
    let sections = emit_sections_from_test_file()?;
    eprintln!(
        "emit suite: {} sections, one kabootar subprocess each",
        sections.len()
    );
    assert!(
        sections.len() >= 20,
        "expected >= 20 emit sections in test_emit.kab, got {}",
        sections.len()
    );
    for (title, body) in sections {
        eprintln!("emit section: {title}");
        run_emit_section_subprocess(&title, &body)?;
    }
    Ok(())
}

/// Rust compile + run_module only (no emit call).
#[test]
fn self_host_emit_rust_run_module_smoke() {
    use kabootar_lib::bytecode::run_module;
    use kabootar_lib::compile::compile_file;
    use kabootar_lib::evaluator::create_global_env;

    let emit_path = self_host_path("emit.kab");
    let program = compile_file(&emit_path).expect("compile emit.kab");
    let bytecode = program
        .bytecode
        .as_ref()
        .expect("emit.kab should produce bytecode");
    eprintln!(
        "emit module: main_ops={} fn_count={} main_locals={}",
        bytecode.main_code.len(),
        bytecode.functions.len(),
        bytecode.main_locals.len()
    );
    let mut env = create_global_env();
    run_module(bytecode, &mut env).expect("run_module emit.kab");
    assert!(env.get("emit").is_some(), "emit export after run_module");
}

#[test]
fn self_host_lexer_rust_run_module_smoke() {
    use kabootar_lib::bytecode::run_module;
    use kabootar_lib::compile::compile_file;
    use kabootar_lib::evaluator::create_global_env;

    let path = self_host_path("lexer.kab");
    let program = compile_file(&path).expect("compile lexer.kab");
    let bytecode = program.bytecode.as_ref().expect("lexer bytecode");
    eprintln!(
        "lexer module: main_ops={} fn_count={} main_locals={}",
        bytecode.main_code.len(),
        bytecode.functions.len(),
        bytecode.main_locals.len()
    );
    let mut env = create_global_env();
    run_module(bytecode, &mut env).expect("run_module lexer.kab");
    assert!(env.get("tokenize").is_some());
}

#[test]
fn self_host_parser_rust_run_module_smoke() {
    use kabootar_lib::bytecode::run_module;
    use kabootar_lib::compile::compile_file;
    use kabootar_lib::evaluator::create_global_env;

    let path = self_host_path("parser.kab");
    let program = compile_file(&path).expect("compile parser.kab");
    let bytecode = program.bytecode.as_ref().expect("parser bytecode");
    eprintln!(
        "parser module: main_ops={} fn_count={} main_locals={}",
        bytecode.main_code.len(),
        bytecode.functions.len(),
        bytecode.main_locals.len()
    );
    let mut env = create_global_env();
    run_module(bytecode, &mut env).expect("run_module parser.kab");
    assert!(env.get("parseTokens").is_some());
}

/// Rust compile + run_module + emit(parse) without kabootar CLI (fast regression for Windows OOM).
#[test]
fn self_host_emit_rust_compile_run_smoke() {
    use kabootar_lib::bytecode::{call_value, run_module};
    use kabootar_lib::compile::compile_file;
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::Value;

    let emit_path = self_host_path("emit.kab");
    let program = compile_file(&emit_path).expect("compile emit.kab");
    let bytecode = program
        .bytecode
        .as_ref()
        .expect("emit.kab should produce bytecode");
    let mut env = create_global_env();
    run_module(bytecode, &mut env).expect("run_module emit.kab");
    let emit_fn = env.get("emit").expect("emit export");
    let ast = parse_via_interpreter("let x = 1");
    let bc = call_value(emit_fn, vec![ast], &[], &[], &[], &[], &mut env)
        .expect("emit(parse let x=1)");
    let Value::Object(ir) = bc else {
        panic!("emit should return object");
    };
    let Value::Array(globals) = ir.get("globals").expect("globals") else {
        panic!("globals array");
    };
    assert_eq!(globals.len(), 1);
}

/// Split `test_emit.kab` into sections at `// --` markers (section count sanity check).
fn emit_sections_from_test_file() -> Result<Vec<(String, String)>, String> {
    let content = std::fs::read_to_string(self_host_path("test_emit.kab"))
        .map_err(|e| format!("read test_emit.kab: {e}"))?;
    let mut out = Vec::new();
    let mut title: Option<String> = None;
    let mut body = String::new();
    for line in content.lines() {
        if line.starts_with("// --") {
            if let Some(t) = title.take() {
                if !t.contains("Report") && !body.trim().is_empty() {
                    out.push((t, body.clone()));
                }
                body.clear();
            }
            title = Some(line.trim().to_string());
            continue;
        }
        if line.starts_with("import ")
            || line.starts_with("let passed")
            || line.starts_with("let fail")
            || line.starts_with("let tI")
            || line.starts_with("let tHas")
            || line.starts_with("fn assert_")
            || line.starts_with("// Test suite")
            || line.starts_with("// CI:")
            || line.starts_with("// Full suite")
        {
            continue;
        }
        if title.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(t) = title {
        if !t.contains("Report") && !body.trim().is_empty() {
            out.push((t, body));
        }
    }
    Ok(out)
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
fn self_host_ownership_suite() {
    kabootar_lib::cli::run_file(&self_host_path("test_ownership.kab"))
        .expect("self_host/test_ownership.kab should pass");
}

/// Compile lexer + parser once (interpreting parser.kab OOMs on Windows).
fn load_self_host_parser_programs() -> Result<Vec<kabootar_lib::compile::CompiledProgram>, String> {
    use kabootar_lib::compile::compile_file_cached;

    let mut out = Vec::with_capacity(2);
    for mod_file in ["lexer.kab", "parser.kab"] {
        let path = self_host_path(mod_file);
        let program = compile_file_cached(&path)?;
        if !program.has_bytecode() {
            return Err(format!("{mod_file} must compile to bytecode"));
        }
        out.push(program);
    }
    Ok(out)
}

fn preload_self_host_parser_deps(
    env: &mut kabootar_lib::value::Environment,
    programs: &[kabootar_lib::compile::CompiledProgram],
) -> Result<(), String> {
    use kabootar_lib::compile::eval_program;

    for program in programs {
        eval_program(program, env)?;
    }
    Ok(())
}

fn run_parser_test_suite() -> Result<(), String> {
    use kabootar_lib::compile::{compile_file_cached, eval_program};
    use kabootar_lib::evaluator::create_global_env;

    let programs = load_self_host_parser_programs()?;
    let mut env = create_global_env();
    preload_self_host_parser_deps(&mut env, &programs)?;
    eval_program(&compile_file_cached(&self_host_path("test_parser.kab"))?, &mut env)?;
    Ok(())
}

#[test]
fn self_host_parser_suite() {
    run_parser_test_suite().expect("self_host/test_parser.kab should pass");
}

#[test]
#[ignore = "slow (~6m): first emit section (tokenize+parse+emit)"]
fn self_host_emit_first_section_smoke() {
    let programs = load_self_host_emit_programs().expect("compile emit deps");
    let sections = emit_sections_from_test_file().expect("parse test_emit.kab sections");
    let (title, body) = sections.first().expect("at least one emit section");
    run_emit_section(title, body, &programs).expect("first emit section should pass");
}

/// First three emit sections (let/add/if) - ~20 min regression without full suite runtime.
#[test]
#[ignore = "slow (~20m): first 3 emit sections"]
fn self_host_emit_core_sections_smoke() {
    let programs = load_self_host_emit_programs().expect("compile emit deps");
    let sections = emit_sections_from_test_file().expect("parse test_emit.kab sections");
    for (title, body) in sections.into_iter().take(3) {
        run_emit_section(&title, &body, &programs).expect("emit core section should pass");
    }
}

#[test]
#[ignore = "slow (~2h on Windows): all emit sections; see self_host_emit_first_section_smoke"]
fn self_host_emit_suite() {
    run_emit_test_suite().expect("self_host/test_emit.kab should pass");
}

#[test]
fn self_host_emit_section_count() {
    let sections = emit_sections_from_test_file().expect("parse test_emit.kab sections");
    assert!(
        sections.len() >= 28,
        "expected >= 28 emit sections, got {}",
        sections.len()
    );
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

/// Interpreter serialize must emit line-oriented .kbc (CHAR_NL, not literal "\\n").
#[test]
fn self_host_serialize_interpreter_deserializes() {
    use kabootar_lib::bytecode::deserialize;
    use kabootar_lib::value::Value;

    let probe_src = r#"import "self_host/parse"
import "self_host/emit"
import "self_host/serialize"
return serialize_bc(emit(parse("let x = 1; return x")))"#;
    let probe_path = self_host_path("_serialize_interp_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write serialize interpreter probe");
    let v = kabootar_lib::cli::run_file(&probe_path).expect("serialize interpreter probe");
    let _ = std::fs::remove_file(&probe_path);
    let Value::String(text) = v else {
        panic!("serialize_bc should return string, got {v:?}");
    };
    assert!(
        text.contains("kabootar-bytecode/1\n"),
        "serialized .kbc must use real newlines after header"
    );
    let module = deserialize(&text).expect("Rust deserialize interpreter serialize output");
    assert_eq!(module.globals, vec!["x".to_string()]);
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

fn assert_emit_kbc_runs_let_x_1(kbc: &str) {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::runtime::stdlib::error::format_runtime_error;
    use kabootar_lib::value::Value;

    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "emit .kbc should have bytecode header"
    );
    assert_fresh_emit_kbc(kbc);
    let module = deserialize(kbc).expect("deserialize compiled emit.kab");
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
    assert!(
        ops.len() >= 3,
        "let x = 1 should emit const/store_global/halt, got {} ops",
        ops.len()
    );
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
}

#[test]
#[ignore = "slow (~2-3h): self-hosted compile(emit.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_emit_full_compile_and_run() {
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
    assert_emit_kbc_runs_let_x_1(&kbc);
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
}

fn emit_via_interpreter(src: &str) -> kabootar_lib::value::Value {
    let probe_src = format!(
        "import \"self_host/parse\"\nimport \"self_host/emit\"\nreturn emit(parse({}))",
        kab_string_literal(src)
    );
    let probe_path = self_host_path("_emit_helper_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write emit helper probe");
    let v = kabootar_lib::cli::run_file(&probe_path).expect("emit helper should run");
    let _ = std::fs::remove_file(&probe_path);
    v
}

fn assert_fresh_serialize_kbc(kbc: &str) {
    if kbc.starts_with("kabootar-bytecode/1constants=")
        || kbc.starts_with("kabootar-bytecode/1const")
    {
        panic!(
            "stale _serialize_full_out.kbc (pre CHAR_NL fix in serialize.kab): .kbc is one line, deserialize will fail.\n\
             Rebuild (~50m): python scripts/profile_emit_compile.py compile serialize.kab\n\
             Or full M11: cargo test --test self_host self_host_serialize_full_compile_and_run -- --ignored --test-threads=1"
        );
    }
    assert!(
        kbc.contains("kabootar-bytecode/1\n"),
        "serialize .kbc must use real newlines (CHAR_NL)"
    );
}

fn assert_serialize_kbc_roundtrips_let_x_1(kbc: &str) {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::runtime::stdlib::error::format_runtime_error;
    use kabootar_lib::value::{Value, format_value};

    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "serialize .kbc should have bytecode header"
    );
    assert_fresh_serialize_kbc(kbc);
    let module = deserialize(kbc).expect("deserialize compiled serialize.kab");
    assert!(
        !module.functions.is_empty(),
        "serialize should emit helper functions"
    );
    let mut run_env = create_global_env();
    run_module(&module, &mut run_env).expect("run compiled serialize module");
    let serialize_bc = run_env
        .get("serialize_bc")
        .expect("compiled serialize should export serialize_bc");
    let ir = emit_via_interpreter("let x = 1; return x");
    let text = match call_value(
        serialize_bc,
        vec![ir],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    ) {
        Ok(v) => v,
        Err(e) => {
            let msg = format_runtime_error(&e);
            panic!("serialize_bc(emit(parse(...))) threw: {msg}");
        }
    };
    let Value::String(kbc_text) = text else {
        panic!("serialize_bc should return .kbc text, got {text:?}");
    };
    assert!(
        kbc_text.starts_with("kabootar-bytecode/1"),
        "serialized output should have bytecode header"
    );
    assert!(
        kbc_text.contains("global 0 x"),
        "let x = 1 should serialize global x"
    );
    let roundtrip = deserialize(&kbc_text).expect("deserialize self-serialized .kbc");
    let mut rt_env = create_global_env();
    let result = run_module(&roundtrip, &mut rt_env).expect("run self-serialized module");
    assert_eq!(
        format_value(&result),
        "1",
        "let x = 1; return x via self-hosted serialize"
    );
}

#[test]
fn self_host_serialize_full_compile_smoke() {
    kabootar_lib::cli::run_file(&self_host_path("test_serialize_full_compile.kab"))
        .expect("self_host/test_serialize_full_compile.kab should pass");
}

#[test]
#[ignore = "slow (~30-90m): self-hosted compile(serialize.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_serialize_full_compile_and_run() {
    let probe_path = self_host_path("_serialize_full_probe_gen.kab");
    let src_copy = format!("{}/_serialize_full_src.kab", env!("CARGO_MANIFEST_DIR"));
    let out_file = format!("{}/_serialize_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    std::fs::copy(self_host_path("serialize.kab"), &src_copy)
        .expect("copy serialize.kab for compile probe");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nlet kbc = compile(read_text_file(\"/proj/_serialize_full_src.kab\"))\nwrite_text_file(\"/proj/_serialize_full_out.kbc\", kbc)\nreturn len(kbc)",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write generated serialize full compile probe");

    run_kabootar_file_subprocess(&probe_path)
        .expect("kabootar compile(serialize.kab) via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let kbc = std::fs::read_to_string(&out_file).expect("read compiled serialize .kbc output");
    assert_serialize_kbc_roundtrips_let_x_1(&kbc);
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
}

/// Run phase only: reuse _serialize_full_out.kbc from a prior compile(serialize.kab).
#[test]
#[ignore = "requires _serialize_full_out.kbc from serialize full compile"]
fn self_host_serialize_kbc_run_only() {
    let out_file = format!("{}/_serialize_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let kbc = std::fs::read_to_string(&out_file).unwrap_or_else(|e| {
        panic!("read {out_file}: {e} (run serialize full compile first)")
    });
    assert_serialize_kbc_roundtrips_let_x_1(&kbc);
}

/// Run phase only: reuse _emit_full_out.kbc from a prior compile(emit.kab).
#[test]
#[ignore = "requires _emit_full_out.kbc from emit full compile"]
fn self_host_emit_kbc_run_only() {
    let out_file = format!("{}/_emit_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let kbc = std::fs::read_to_string(&out_file)
        .unwrap_or_else(|e| panic!("read {out_file}: {e} (run emit full compile first)"));
    assert_emit_kbc_runs_let_x_1(&kbc);
}

/// compile.kab body uses nested calls; callee must not clobber (pCalleeStack + eCalleeStack).
#[test]
fn self_host_emit_nested_call_compile_facade() {
    let src = std::fs::read_to_string(self_host_path("compile.kab")).expect("read compile.kab");
    let probe_src = format!(
        "import \"self_host/parse\"\nimport \"self_host/emit\"\nlet ir = emit(parse({}))\nif len(ir[\"globals\"]) < 4 {{ throw \"expected 4 globals\" }}\nreturn 0",
        kab_string_literal(&src)
    );
    let probe_path = self_host_path("_emit_nested_call_probe_gen.kab");
    std::fs::write(&probe_path, probe_src).expect("write nested call emit probe");
    run_kabootar_file_subprocess(&probe_path).expect("nested call emit probe");
    let _ = std::fs::remove_file(&probe_path);
}

fn assert_compiled_compile_runs_sample(kbc: &str) {
    use kabootar_lib::bytecode::{call_value, deserialize, run_module};
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::runtime::stdlib::error::format_runtime_error;
    use kabootar_lib::value::{Value, format_value};

    assert!(
        kbc.starts_with("kabootar-bytecode/1"),
        "compile .kbc should have bytecode header"
    );
    let module = deserialize(kbc).expect("deserialize compiled compile.kab");
    assert!(
        !module.functions.is_empty(),
        "compile.kab should emit compile function body"
    );
    let mut run_env = create_global_env();
    run_module(&module, &mut run_env).expect("run compiled compile module");
    let compile_fn = run_env
        .get("compile")
        .expect("compiled compile.kab should export compile");
    let sample_src = "let n = 10\nreturn n + 32";
    let text = match call_value(
        compile_fn,
        vec![Value::String(sample_src.into())],
        &[],
        &[],
        &[],
        &[],
        &mut run_env,
    ) {
        Ok(v) => v,
        Err(e) => {
            let msg = format_runtime_error(&e);
            panic!("compile(sample) threw: {msg}");
        }
    };
    let Value::String(sample_kbc) = text else {
        panic!("compile should return .kbc text, got {text:?}");
    };
    assert!(
        sample_kbc.starts_with("kabootar-bytecode/1"),
        "compile(sample) should produce bytecode header"
    );
    let sample_mod = deserialize(&sample_kbc).expect("deserialize compile(sample) output");
    let mut sample_env = create_global_env();
    let result = run_module(&sample_mod, &mut sample_env).expect("run compile(sample) output");
    assert_eq!(
        format_value(&result),
        "42",
        "true bootstrap: compile(sample) should return 42"
    );
}

#[test]
fn self_host_compile_full_compile_smoke() {
    run_kabootar_file_subprocess(&self_host_path("test_compile_full_compile.kab"))
        .expect("self_host/test_compile_full_compile.kab should pass");
}

#[test]
#[ignore = "slow (~1-3h): self-hosted compile(compile.kab); run: cargo test --test self_host -- --ignored"]
fn self_host_compile_full_compile_and_run() {
    let probe_path = self_host_path("_compile_full_probe_gen.kab");
    let src_copy = format!("{}/_compile_full_src.kab", env!("CARGO_MANIFEST_DIR"));
    let out_file = format!("{}/_compile_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    std::fs::copy(self_host_path("compile.kab"), &src_copy)
        .expect("copy compile.kab for compile probe");
    let probe = format!(
        "import \"self_host/compile\"\nos_mount(\"/proj\", {})\nlet kbc = compile(read_text_file(\"/proj/_compile_full_src.kab\"))\nwrite_text_file(\"/proj/_compile_full_out.kbc\", kbc)\nreturn len(kbc)",
        kab_string_literal(&manifest)
    );
    std::fs::write(&probe_path, probe).expect("write generated compile full compile probe");

    run_kabootar_file_subprocess(&probe_path).expect("kabootar compile(compile.kab) via subprocess");
    let _ = std::fs::remove_file(&probe_path);

    let kbc = std::fs::read_to_string(&out_file).expect("read compiled compile .kbc output");
    assert_compiled_compile_runs_sample(&kbc);
    let _ = std::fs::remove_file(&src_copy);
    let _ = std::fs::remove_file(&out_file);
}

/// Run phase only: reuse _compile_full_out.kbc from a prior compile(compile.kab).
#[test]
#[ignore = "requires _compile_full_out.kbc from compile full compile"]
fn self_host_compile_kbc_run_only() {
    let out_file = format!("{}/_compile_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let kbc = std::fs::read_to_string(&out_file).unwrap_or_else(|e| {
        panic!("read {out_file}: {e} (run compile full compile first)")
    });
    assert_compiled_compile_runs_sample(&kbc);
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

/// S3 gate alias — same bootstrap path as `self_host_bootstrap_compile_and_run`.
#[test]
fn s3_self_host_bootstrap_gate() {
    self_host_bootstrap_compile_and_run();
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

/// Fail fast if _emit_full_out.kbc on disk predates emit.kab pop() refactor.
#[test]
fn self_host_emit_kbc_freshness_guard() {
    let out_file = format!("{}/_emit_full_out.kbc", env!("CARGO_MANIFEST_DIR"));
    let Ok(kbc) = std::fs::read_to_string(&out_file) else {
        return; // no artifact yet
    };
    assert_fresh_emit_kbc(&kbc);
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
    assert_fresh_emit_kbc(&kbc);
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

#[test]
fn h6e_boot_policy_smoke() {
    let path = format!(
        "{}/examples/h6e_boot_policy_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-boot".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e boot smoke should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn h6e thread")
        .join()
        .expect("h6e thread join");
    assert!(ok);
}

#[test]
fn h6e_run_selfhost_probe() {
    let path = format!(
        "{}/examples/h6e_run_selfhost_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-run-probe".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e run selfhost probe should run"),
                kabootar_lib::value::Value::Number(42)
            )
        })
        .expect("spawn h6e run probe thread")
        .join()
        .expect("h6e run probe thread join");
    assert!(ok);
}

#[test]
fn h6e_vm_smoke() {
    let path = format!(
        "{}/examples/h6e_vm_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-vm".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e vm smoke should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn h6e vm thread")
        .join()
        .expect("h6e vm thread join");
    assert!(ok);
}

#[test]
fn h6e_kab_vm_smoke() {
    let path = format!(
        "{}/examples/h6e_kab_vm_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-kab-vm".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e kab vm smoke should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn h6e kab vm thread")
        .join()
        .expect("h6e kab vm thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_probe() {
    let path = format!(
        "{}/self_host/vm_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-probe".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm probe thread")
        .join()
        .expect("vm probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_native_probe() {
    let path = format!(
        "{}/self_host/vm_native_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-native".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_native_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm native probe thread")
        .join()
        .expect("vm native probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_class_probe() {
    let path = format!(
        "{}/self_host/vm_class_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-class".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_class_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm class probe thread")
        .join()
        .expect("vm class probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_arith_probe() {
    let path = format!(
        "{}/self_host/vm_arith_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-arith".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_arith_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm arith probe thread")
        .join()
        .expect("vm arith probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_hostops_probe() {
    let path = format!(
        "{}/self_host/vm_hostops_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-hostops".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_hostops_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm hostops probe thread")
        .join()
        .expect("vm hostops probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_adv_probe() {
    let path = format!(
        "{}/self_host/vm_adv_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-adv".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_adv_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm adv probe thread")
        .join()
        .expect("vm adv probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_lang_probe() {
    let path = format!(
        "{}/self_host/vm_lang_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-lang".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_lang_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm lang probe thread")
        .join()
        .expect("vm lang probe thread join");
    assert!(ok);
}

#[test]
fn self_host_vm_import_probe() {
    let path = format!(
        "{}/self_host/vm_import_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("vm-import".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("vm_import_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn vm import probe thread")
        .join()
        .expect("vm import probe thread join");
    assert!(ok);
}

/// H6e: heavy leaf shards stay skipped (see should_attempt_self_host); every facade
/// above them (emit.kab, parser.kab, serialize_impl.kab, vm_run.kab, …) is
/// attemptable — see `self_host_vm_cores_not_in_skip_list` and the
/// `*_facade_full_compile` tests below.
#[test]
fn self_host_heavy_cores_still_skipped() {
    use kabootar_lib::compile::compile_file_self_host;
    for name in [
        "emit_impl.kab",
        "parser_impl.kab",
        "lexer_impl.kab",
        "serialize_body.kab",
        "vm_run_body.kab",
    ] {
        let path = format!("{}/self_host/{name}", env!("CARGO_MANIFEST_DIR"));
        let err = compile_file_self_host(&path).unwrap_err();
        assert!(
            err.contains("skipped"),
            "{name} should stay skipped, got: {err}"
        );
    }
}

/// Gate only: deserialize/vm/H6e facades must not hit the skip list (compile.kab
/// was always tiny; emit/parser/lexer/serialize/vm_impl plus the sharded
/// serialize_impl/vm_run are now thin facades over the skip-listed leaf shards —
/// see `*_facade_full_compile` tests below for the actual CI-fast self-host
/// compile+run gate).
#[test]
fn self_host_vm_cores_not_in_skip_list() {
    use kabootar_lib::compile::self_host_is_attemptable;
    for name in [
        "deserialize.kab",
        "vm.kab",
        "compile.kab",
        "emit.kab",
        "parser.kab",
        "lexer.kab",
        "serialize.kab",
        "vm_impl.kab",
        "serialize_impl.kab",
        "vm_run.kab",
    ] {
        let path = format!("{}/self_host/{name}", env!("CARGO_MANIFEST_DIR"));
        assert!(
            self_host_is_attemptable(&path),
            "{name} should be self-host attemptable"
        );
    }
}

/// H6e: each heavy core's thin facade must self-host-compile in CI-fast time
/// (<10s) — the facade's own source is a two-line import + `pub let` alias, so
/// `compile self_host/X.kab --self-host` never touches the skip-listed `_impl`
/// body (which is loaded by the Rust module loader when the compiled bytecode
/// is later run/imported).
fn assert_facade_self_host_compile_fast(name: &str) {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    use std::time::Instant;

    let path = format!("{}/self_host/{name}", env!("CARGO_MANIFEST_DIR"));
    let path2 = path.clone();
    let name2 = name.to_string();
    let (backend, has_bc, elapsed_ms) = std::thread::Builder::new()
        .name("sh-facade".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let t0 = Instant::now();
            let (program, backend) =
                compile_file_prefer(&path2, CompilePrefer::SelfHostOnly).expect("compile");
            (backend, program.has_bytecode(), t0.elapsed().as_millis())
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(has_bc, "{name2} facade should produce bytecode");
    assert_eq!(backend, "self-host", "{name2} facade should self-host-compile");
    assert!(
        elapsed_ms < 10_000,
        "{name2} facade self-host compile took {elapsed_ms}ms, expected <10s"
    );
}

#[test]
fn self_host_serialize_facade_full_compile() {
    assert_facade_self_host_compile_fast("serialize.kab");
}

#[test]
fn self_host_lexer_facade_full_compile() {
    assert_facade_self_host_compile_fast("lexer.kab");
}

#[test]
fn self_host_parser_facade_full_compile() {
    assert_facade_self_host_compile_fast("parser.kab");
}

#[test]
fn self_host_emit_facade_full_compile() {
    assert_facade_self_host_compile_fast("emit.kab");
}

#[test]
fn self_host_vm_impl_facade_full_compile() {
    assert_facade_self_host_compile_fast("vm_impl.kab");
}

#[test]
fn self_host_serialize_impl_facade_full_compile() {
    assert_facade_self_host_compile_fast("serialize_impl.kab");
}

#[test]
fn self_host_vm_run_facade_full_compile() {
    assert_facade_self_host_compile_fast("vm_run.kab");
}

#[test]
fn self_host_compile_facade_full_compile() {
    assert_facade_self_host_compile_fast("compile.kab");
}

/// H6e: nested calls must restore `eArgN`/`eArgs` (module globals) so
/// `str_slice(s, 2, len(s))` emits `call 3`, not a clobbered `call 1`.
#[test]
fn self_host_emit_nested_call_argn_restore() {
    use kabootar_lib::bytecode::{deserialize, run_module};
    use kabootar_lib::compile::compile_source_self_host;
    use kabootar_lib::evaluator::create_global_env;
    use kabootar_lib::value::format_value;

    let src = "let s = \"abcdefgh\"\nreturn str_slice(s, 2, len(s))";
    let program = std::thread::Builder::new()
        .name("sh-nested-argn".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_source_self_host(src).expect("self-host compile nested call"))
        .expect("spawn")
        .join()
        .expect("join");
    let bc = program.bytecode.expect("bytecode");
    let kbc = kabootar_lib::bytecode::serialize(&bc);
    assert!(
        kbc.contains("call 3"),
        "expected call 3 for str_slice arity, kbc snippet:\n{}",
        kbc.chars().take(400).collect::<String>()
    );
    let module = deserialize(&kbc).expect("deserialize");
    let mut env = create_global_env();
    let v = run_module(&module, &mut env).expect("run");
    assert_eq!(format_value(&v), "cdefgh");
}

/// H6e delete-gate: under kab-only, a skip-listed leaf without `.kbc` cache must
/// not fall through to a live Rust compile.
#[test]
fn h6e_skip_listed_kab_only_delete_gate() {
    use kabootar_lib::compile::{
        compile_file_prefer_cached, self_host_is_skip_listed, CompilePrefer,
    };

    let path = format!(
        "{}/self_host/serialize_body.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    assert!(
        self_host_is_skip_listed(&path),
        "serialize_body.kab must stay skip-listed"
    );
    // Ensure no on-disk cache for this leaf.
    kabootar_lib::compile::invalidate_file_cache(&path);
    if let Ok(base) = std::env::current_dir() {
        let marker = kabootar_lib::compile::cache_path_for(&base, &path);
        let _ = std::fs::remove_file(marker);
    }

    let prev = std::env::var("KABOOTAR_VM").ok();
    std::env::set_var("KABOOTAR_VM", "kab-only");
    let err = compile_file_prefer_cached(&path, CompilePrefer::SelfHostThenRust).unwrap_err();
    match prev {
        Some(v) => std::env::set_var("KABOOTAR_VM", v),
        None => std::env::remove_var("KABOOTAR_VM"),
    }
    assert!(
        err.contains("skip-listed") && err.contains("Rust compile"),
        "expected kab-only skip-list delete-gate, got: {err}"
    );
}

/// H6e: module load path prefers self-host for attemptable facades (same as run_file).
#[test]
fn h6e_load_program_prefers_self_host_facade() {
    use kabootar_lib::compile::{compile_file_prefer_cached, CompilePrefer};

    let path = format!("{}/self_host/serialize.kab", env!("CARGO_MANIFEST_DIR"));
    kabootar_lib::compile::invalidate_file_cache(&path);
    if let Ok(base) = std::env::current_dir() {
        let marker = kabootar_lib::compile::cache_path_for(&base, &path);
        let _ = std::fs::remove_file(marker);
    }
    let path2 = path.clone();
    let (backend, has_bc) = std::thread::Builder::new()
        .name("h6e-load-pref".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let (program, backend) =
                compile_file_prefer_cached(&path2, CompilePrefer::SelfHostThenRust)
                    .expect("prefer compile serialize facade");
            (backend, program.has_bytecode())
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(has_bc);
    assert_eq!(
        backend, "self-host",
        "serialize.kab facade should self-host via prefer path"
    );
}

/// H6e stricter kab-only: self-host-compile a tiny snippet via the (now CI-fast)
/// facade pipeline, then run the resulting bytecode under KABOOTAR_VM=kab-only
/// (no Rust host `run_module` fallback allowed).
#[test]
fn h6e_kab_only_selfhost_compile_run() {
    use kabootar_lib::bytecode::deserialize;
    use kabootar_lib::compile::{compile_source_self_host, CompilePrefer};
    use kabootar_lib::value::{format_value, Value};

    let _ = CompilePrefer::SelfHostOnly; // documents intent; compile_source_self_host is always self-host
    let src = "let n = 10\nreturn n + 32";
    let program = std::thread::Builder::new()
        .name("h6e-kab-only-sh".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_source_self_host(src).expect("self-host compile tiny snippet"))
        .expect("spawn")
        .join()
        .expect("join");
    assert!(program.has_bytecode(), "tiny snippet should self-host-compile to bytecode");
    let bytecode = program.bytecode.expect("bytecode");
    let kbc = kabootar_lib::bytecode::serialize(&bytecode);
    let module = deserialize(&kbc).expect("deserialize self-host .kbc");

    let prev = std::env::var("KABOOTAR_VM").ok();
    std::env::set_var("KABOOTAR_VM", "kab-only");
    // `Value` (Rc-based) is not Send — format to a String inside the thread before
    // crossing the join boundary.
    let result: Result<String, String> = std::thread::Builder::new()
        .name("h6e-kab-only-run".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::evaluator::create_global_env;
            let mut env = create_global_env();
            kabootar_lib::compile::eval_program(
                &kabootar_lib::compile::CompiledProgram {
                    stmts: Vec::new(),
                    bytecode: Some(module.clone()),
                    stmt_count: 2,
                    memory_mode: module.memory_mode,
                },
                &mut env,
            )
            .map(|v| format_value(&v))
        })
        .expect("spawn")
        .join()
        .expect("join");
    match prev {
        Some(v) => std::env::set_var("KABOOTAR_VM", v),
        None => std::env::remove_var("KABOOTAR_VM"),
    }
    let formatted = result.expect("run self-host-compiled snippet under kab-only");
    assert_eq!(
        formatted, "42",
        "let n = 10; return n + 32 under kab-only via self-host facade pipeline"
    );
    let _: Option<Value> = None;
}

#[test]
fn self_host_deserialize_full_compile() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    let path = format!("{}/self_host/deserialize.kab", env!("CARGO_MANIFEST_DIR"));
    let path2 = path.clone();
    let (backend, has_bc) = std::thread::Builder::new()
        .name("sh-deser".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let (program, backend) =
                compile_file_prefer(&path2, CompilePrefer::SelfHostOnly).expect("compile");
            (backend, program.has_bytecode())
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(has_bc);
    assert_eq!(backend, "self-host");
}

/// Else-if chains, AccAdd peephole, and bitwise ops must self-host-parse/emit.
#[test]
fn self_host_elseif_accadd_bitops_compile() {
    use kabootar_lib::compile::compile_source_self_host;
    let src = r#"
fn f(x) {
  if x == 1 { return 10 }
  else if x == 2 { return 20 }
  else if x == 3 { return 30 }
  else { return 0 }
}
fn acc(n) {
  let x = 0
  x = x + n
  x = x + 1
  return x
}
fn bits() {
  return (1 & 3) | (2 << 1) | (~0 + 1)
}
"#;
    let program = std::thread::Builder::new()
        .name("sh-elseif-bits".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_source_self_host(src).expect("self-host compile"))
        .expect("spawn")
        .join()
        .expect("join");
    assert!(program.has_bytecode());
}

#[test]
fn self_host_vm_full_compile() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    let path = format!("{}/self_host/vm.kab", env!("CARGO_MANIFEST_DIR"));
    let path2 = path.clone();
    let (backend, has_bc) = std::thread::Builder::new()
        .name("sh-vm".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let (program, backend) =
                compile_file_prefer(&path2, CompilePrefer::SelfHostOnly).expect("compile");
            (backend, program.has_bytecode())
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(has_bc);
    assert_eq!(backend, "self-host");
}

/// Verified self-host compile path for an attemptable core (unescape probe).
#[test]
fn self_host_unescape_probe_full_compile() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    let path = format!(
        "{}/self_host/unescape_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let path2 = path.clone();
    let (backend, has_bc) = std::thread::Builder::new()
        .name("sh-unescape-compile".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let (program, backend) =
                compile_file_prefer(&path2, CompilePrefer::SelfHostThenRust).expect("compile");
            (backend, program.has_bytecode())
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(has_bc);
    assert_eq!(backend, "self-host");
}

#[test]
fn h6e_kab_vm_delete_gate() {
    let path = format!(
        "{}/examples/h6e_kab_vm_delete_gate.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-delete".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e kab vm delete gate should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn h6e delete gate thread")
        .join()
        .expect("h6e delete gate thread join");
    assert!(ok);
}

#[test]
fn h6e_kab_only_gate() {
    let path = format!(
        "{}/examples/h6e_kab_only_gate.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-kab-only".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("h6e kab-only gate should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn h6e kab-only gate thread")
        .join()
        .expect("h6e kab-only gate thread join");
    assert!(ok);
}

/// Process delete-gate: outer `.kbc` must run on Kab VM (no host fallback).
#[test]
fn h6e_kab_only_process_import() {
    let prev = std::env::var("KABOOTAR_VM").ok();
    std::env::set_var("KABOOTAR_VM", "kab-only");
    let path = format!(
        "{}/self_host/vm_import_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("h6e-kab-only-import".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("kab-only import probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn kab-only import thread")
        .join()
        .expect("kab-only import thread join");
    match prev {
        Some(v) => std::env::set_var("KABOOTAR_VM", v),
        None => std::env::remove_var("KABOOTAR_VM"),
    }
    assert!(ok);
}

#[test]
fn self_host_unescape_probe() {
    let path = format!(
        "{}/self_host/unescape_probe.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    let ok = std::thread::Builder::new()
        .name("unescape".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                kabootar_lib::cli::run_file(&path).expect("unescape_probe should run"),
                kabootar_lib::value::Value::Bool(true)
            )
        })
        .expect("spawn unescape probe thread")
        .join()
        .expect("unescape probe thread join");
    assert!(ok);
}

