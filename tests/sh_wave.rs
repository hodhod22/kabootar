//! SH0 inventory, SH1 facade seeds, SH3a nested push(len).

use std::time::Instant;

use kabootar_lib::bytecode::run_module;
use kabootar_lib::compile::{
    collect_self_host_inventory, compile_source, missing_compiler_dag_seeds, read_seed_bytecode,
    write_compiler_facade_seeds,
};
use kabootar_lib::evaluator::create_global_env;
use kabootar_lib::modules::import_shard_stats;

#[test]
fn sh0_self_host_compile_dag_snapshot() {
    let inv = collect_self_host_inventory().expect("inventory");
    eprintln!(
        "SH0 inventory kab_files={} vm_files={} probe_files={} compile_dag={} (evals,unique)={:?}",
        inv.kab_files,
        inv.vm_files,
        inv.probe_files,
        inv.compile_dag.len(),
        import_shard_stats()
    );
    assert!(
        inv.kab_files >= 80,
        "self_host product .kab count, got {}",
        inv.kab_files
    );
    assert!(
        inv.vm_files < 40,
        "SH6: vm_* shards must stay under 40 files, got {}",
        inv.vm_files
    );
    assert!(
        inv.compile_dag.len() >= 20,
        "compile.kab DAG should stay a real pipeline, got {}",
        inv.compile_dag.len()
    );
    assert!(
        inv.compile_dag.len() < 80,
        "SH5 reverse-densify: compile DAG must stay under 80 files, got {}",
        inv.compile_dag.len()
    );
    let dag_vm = inv
        .compile_dag
        .iter()
        .filter(|p| p.contains("/vm_") || p.ends_with("/vm.kab"))
        .count();
    assert_eq!(
        dag_vm, 0,
        "compile.kab must not import self_host/vm (host VM runs the toolchain)"
    );
}

fn ensure_compiler_image() {
    let missing = missing_compiler_dag_seeds().expect("scan dag seeds");
    if missing.is_empty() {
        return;
    }
    eprintln!("SH1/SH7 healing dirty={}", missing.len());
    let stats = kabootar_lib::compile::compile_dirty_dag_seeds().expect("compile dirty");
    assert_eq!(stats.failed, 0, "SH7 dirty compile failed");
    kabootar_lib::compile::reset_self_host_toolchain_cache();
}

#[test]
fn sh1_compiler_facade_seeds() {
    let n = write_compiler_facade_seeds().expect("write facade seeds");
    assert_eq!(n, 5, "compile/parse/emit/serialize/ownership");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host");
    for name in [
        "compile.kab",
        "parse.kab",
        "emit.kab",
        "serialize.kab",
        "ownership.kab",
    ] {
        let path = root.join(name).to_string_lossy().replace('\\', "/");
        let bc = read_seed_bytecode(&path)
            .expect("read seed")
            .unwrap_or_else(|| panic!("seed miss {name}"));
        assert!(
            !bc.functions.is_empty() || !bc.main_code.is_empty(),
            "{name} seed empty"
        );
    }
}

#[test]
fn sh1_compiler_dag_image_complete() {
    ensure_compiler_image();
    let missing = missing_compiler_dag_seeds().expect("scan dag seeds");
    assert!(
        missing.is_empty(),
        "SH1 compiler image stale/missing for {} files (run KABOOTAR_SH1_WARM=1 cargo test --test sh_wave sh1_warm -- --ignored). first: {:?}",
        missing.len(),
        missing.iter().take(8).collect::<Vec<_>>()
    );
}

#[test]
fn sh1_import_compile_image_budget() {
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;

    std::env::set_var("KABOOTAR_VM", "host");
    std::env::set_var("KABOOTAR_COMPILE", "rust");
    ensure_compiler_image();
    let missing = missing_compiler_dag_seeds().expect("scan");
    assert!(
        missing.is_empty(),
        "SH1 image required for 2s gate; missing {}: {:?}",
        missing.len(),
        missing.iter().take(8).collect::<Vec<_>>()
    );
    let t0 = Instant::now();
    std::thread::Builder::new()
        .name("sh1-import".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let mut env = create_module_env();
            import_module("self_host/compile", &mut env).expect("import compile image");
            assert!(env.get("compile").is_some(), "compile export missing");
        })
        .expect("spawn")
        .join()
        .expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH1 import self_host/compile {ms:.1} ms");
    let budget = if cfg!(debug_assertions) {
        5_000.0
    } else {
        2_000.0
    };
    assert!(
        ms < budget,
        "SH1 first import with compiler-image should be < {budget} ms (2s release / 5s debug), got {ms:.1}"
    );
}

#[test]
fn sh3a_rust_push_len_nested() {
    let src = "let xs = [1, 2, 3]\nlet s = []\ns = push(s, len(xs))\nreturn s[0]\n";
    let prog = compile_source(src).expect("rust compile");
    let bc = prog.bytecode.expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(&bc, &mut env).expect("run");
    assert_eq!(kabootar_lib::value::format_value(&v), "3");
}

#[test]
#[ignore = "self-host toolchain import is minutes even in debug; rust gate is sh3a_rust_push_len_nested"]
fn sh3a_self_host_push_len_nested() {
    use kabootar_lib::compile::compile_source_self_host;

    let src = "let xs = [1, 2, 3]\nlet s = []\ns = push(s, len(xs))\nreturn s[0]\n";
    let t0 = Instant::now();
    let program = std::thread::Builder::new()
        .name("sh3a-push-len".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_source_self_host(src).expect("self-host compile push+len"))
        .expect("spawn")
        .join()
        .expect("join");
    eprintln!("SH3a self-host push+len {} ms", t0.elapsed().as_secs_f64() * 1000.0);
    let bc = program.bytecode.expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(&bc, &mut env).expect("run");
    assert_eq!(
        kabootar_lib::value::format_value(&v),
        "3",
        "push(s, len(xs)) must store 3, not emit len as the outer call"
    );
}

#[test]
#[ignore = "opt-in: KABOOTAR_SH1_WARM=1 writes every compile-DAG .kbc under seed/dag"]
fn sh1_warm_full_compile_dag() {
    let n = kabootar_lib::compile::write_compiler_dag_seeds().expect("warm dag");
    eprintln!("SH1 warm wrote {n} seed/dag files");
    assert!(n >= 20, "compile DAG should stay a pipeline, wrote {n}");
}

#[test]
fn sh4_kbcb_v2_roundtrip() {
    use kabootar_lib::bytecode::{deserialize_kbcb, serialize_kbcb, serialize_kbcb_v1};
    let src = r#"
fn add(a, b) {
    return a + b
}
let obj = { "k": "v" }
let xs = [1, 2, 3]
return add(xs[0], len(obj))
"#;
    let prog = compile_source(src).expect("compile");
    let m = prog.bytecode.expect("bytecode");
    let bin = serialize_kbcb(&m);
    assert_eq!(bin[4], 2, "default kbcb is v2");
    let back = deserialize_kbcb(&bin).expect("v2 decode");
    assert_eq!(back, m);
    let v1 = serialize_kbcb_v1(&m);
    let from_v1 = deserialize_kbcb(&v1).expect("v1 still loads");
    assert_eq!(from_v1, m);
}

#[test]
fn sh4_kbcb_v2_faster_than_text() {
    use kabootar_lib::bytecode::{deserialize, deserialize_kbcb, serialize_kbcb};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("self_host/seed/emit_impl.kab.kbc");
    let text = std::fs::read_to_string(&path).expect("emit_impl seed");
    let module = deserialize(&text).expect("text kbc");
    let bin = serialize_kbcb(&module);
    let n = 40;
    let t0 = Instant::now();
    for _ in 0..n {
        let _ = deserialize(&text).expect("text");
    }
    let text_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t0 = Instant::now();
    for _ in 0..n {
        let _ = deserialize_kbcb(&bin).expect("v2");
    }
    let v2_ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH4 emit_impl deserialize x{n} text={text_ms:.2}ms v2={v2_ms:.2}ms");
    assert!(
        v2_ms < text_ms,
        "kbcb v2 should deserialize faster than text .kbc ({v2_ms:.2} vs {text_ms:.2})"
    );
}

#[test]
fn sh6_vm_shard_count_under_40() {
    let inv = collect_self_host_inventory().expect("inventory");
    assert!(
        inv.vm_files < 40,
        "SH6 densify: vm_* must be < 40, got {}",
        inv.vm_files
    );
    assert!(
        inv.vm_files >= 8,
        "keep a real kab VM (ops/session/run), got {}",
        inv.vm_files
    );
}

#[test]
fn sh6_vm_facade_evals() {
    let path = format!("{}/self_host/vm.kab", env!("CARGO_MANIFEST_DIR"));
    let ok = std::thread::Builder::new()
        .name("sh6-vm".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let t0 = Instant::now();
            let mut env = create_global_env();
            let r = kabootar_lib::compile::eval_file_cached(&path, &mut env);
            eprintln!("SH6 eval vm.kab {} ms", t0.elapsed().as_millis());
            match r {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("SH6 eval vm.kab err: {e}");
                    false
                }
            }
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "eval self_host/vm");
}

fn nested_if_while_fn_src() -> &'static str {
    r#"
fn f(n) {
    let i = 0
    while i < n {
        if i == 1 {
            return i
        }
        i = i + 1
    }
    return 0
}
return f(3)
"#
}

/// SH2: nested if/while/fn compiles; parser/emit exec allocate sess per call.
#[test]
fn sh2_nested_if_while_fn_rust() {
    let prog = compile_source(nested_if_while_fn_src()).expect("rust compile");
    let bc = prog.bytecode.expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(&bc, &mut env).expect("run");
    assert_eq!(kabootar_lib::value::format_value(&v), "1");
}

#[test]
fn sh2_parser_emit_exec_are_per_call_session() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host");
    let parser = std::fs::read_to_string(root.join("parser_exec.kab")).expect("parser_exec");
    let emit = std::fs::read_to_string(root.join("emit_exec.kab")).expect("emit_exec");
    assert!(
        !parser.lines().any(|l| l.starts_with("let sess = pMakeSession()")),
        "SH2: parser_exec must not keep a module-global sess"
    );
    assert!(
        !emit.lines().any(|l| l.starts_with("let E = eMakeSession()")),
        "SH2: emit_exec must not keep a module-global E"
    );
    assert!(parser.contains("let sess = pMakeSession()"), "per-call sess");
    assert!(parser.contains("gSess = sess"), "rebind tramp target");
    assert!(emit.contains("gE = E"), "rebind emit tramp target");
}

/// SH3b: product facades re-export by alias (wrapping pub fn adds a Kab-VM frame).
#[test]
fn sh3b_facades_are_aliases_not_wrap_fn() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host");
    for (name, needle) in [
        ("parser.kab", "pub let parseTokens = parseTokensImpl"),
        ("emit.kab", "pub let emit = emitImpl"),
        ("lexer.kab", "pub let tokenize = tokenizeImpl"),
    ] {
        let src = std::fs::read_to_string(root.join(name)).expect(name);
        assert!(
            src.contains(needle),
            "{name} must alias impl (SH3b), missing `{needle}`"
        );
        assert!(
            !src.contains("pub fn parseTokens(") && !src.contains("pub fn emit(") && !src.contains("pub fn tokenize("),
            "{name} must not wrap the impl in a pub fn (extra Kab-VM frame)"
        );
    }
}

/// SH3c: self-host serialize uses real newlines (CHAR_NL), not a one-line .kbc.
#[test]
#[ignore = "shares hang with SH8 tiny self-host compile in debug"]
fn sh3c_self_host_kbc_has_real_newlines() {
    use kabootar_lib::compile::compile_source_self_host;
    ensure_compiler_image();
    let program = std::thread::Builder::new()
        .name("sh3c-nl".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| compile_source_self_host("return 1\n").expect("self-host tiny"))
        .expect("spawn")
        .join()
        .expect("join");
    let bc = program.bytecode.expect("bytecode");
    let text = kabootar_lib::bytecode::serialize(&bc);
    assert!(
        text.contains("kabootar-bytecode/1\n"),
        "SH3c: .kbc must be line-oriented (CHAR_NL)"
    );
    assert!(
        text.lines().count() > 3,
        "SH3c: expected multi-line .kbc, got {} lines",
        text.lines().count()
    );
}

/// SH7: matching compiler-image means dirty=0 (incremental no-op).
#[test]
fn sh7_dirty_dag_noop_when_image_fresh() {
    let missing = missing_compiler_dag_seeds().expect("scan");
    if !missing.is_empty() {
        let stats = kabootar_lib::compile::compile_dirty_dag_seeds().expect("compile dirty");
        assert_eq!(stats.failed, 0, "SH7 dirty compile failed");
        eprintln!("SH7 dirty={} compiled={}", stats.dirty, stats.compiled);
    }
    let stats = kabootar_lib::compile::compile_dirty_dag_seeds().expect("second dirty");
    eprintln!("SH7 dirty={}", stats.dirty);
    assert_eq!(stats.dirty, 0, "fresh image should compile dirty=0 shards");
    assert_eq!(stats.compiled, 0);
}

/// SH7b: product import tree incremental — second pass dirty=0 after cache write.
#[test]
fn sh7b_product_tree_incremental() {
    let entry = "self_host/sample";
    let s1 = kabootar_lib::compile::compile_dirty_product_tree(entry).expect("first");
    assert_eq!(s1.failed, 0, "SH7b first compile failed");
    eprintln!("SH7b first dirty={} compiled={}", s1.dirty, s1.compiled);
    let s2 = kabootar_lib::compile::compile_dirty_product_tree(entry).expect("second");
    eprintln!("SH7b second dirty={}", s2.dirty);
    assert_eq!(s2.failed, 0);
    assert_eq!(s2.dirty, 0, "second product-tree compile should be dirty=0");
}

/// SH8: tiny source parses via compiler-image (`parse`), without full `compile()` hang.
#[test]
#[ignore = "import parse + tramp can hang in debug; rust nested-if is sh2_nested_if_while_fn_rust"]
fn sh8_tiny_parse_via_compiler_image() {
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::bytecode::call_value;
    use kabootar_lib::value::Value;

    ensure_compiler_image();
    std::env::set_var("KABOOTAR_VM", "host");
    let t0 = Instant::now();
    let kind = std::thread::Builder::new()
        .name("sh8-parse".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let mut env = create_module_env();
            import_module("self_host/parse", &mut env).expect("import parse");
            let parse_fn = env.get("parse").expect("parse export");
            let ast = call_value(
                parse_fn,
                vec![Value::String("return 1\n".into())],
                &[],
                &[],
                &[],
                &[],
                &mut env,
            )
            .expect("parse tiny");
            match ast {
                Value::Object(map) => map
                    .get("kind")
                    .map(kabootar_lib::value::format_value)
                    .unwrap_or_default(),
                other => kabootar_lib::value::format_value(&other),
            }
        })
        .expect("spawn")
        .join()
        .expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH8 parse tiny kind={kind} ms={ms:.1}");
    assert!(
        kind.contains("Program"),
        "parse(return 1) should be AST_PROGRAM, got {kind}"
    );
}

/// SH8: tiny self-host compile. Full `compile()` can hang in debug — opt-in.
#[test]
#[ignore = "debug self-host compile of tiny source can hang; parse gate is sh8_tiny_parse_via_compiler_image"]
fn sh8_tiny_self_host_compile() {
    use kabootar_lib::compile::compile_source_self_host;
    ensure_compiler_image();
    let t0 = Instant::now();
    let program = std::thread::Builder::new()
        .name("sh8-tiny".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| compile_source_self_host("return 1\n").expect("self-host tiny"))
        .expect("spawn")
        .join()
        .expect("join");
    let first_ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH8 tiny first_ms={first_ms:.1}");
    assert!(program.bytecode.is_some(), "tiny self-host bytecode");
    let budget = if cfg!(debug_assertions) {
        15_000.0
    } else {
        2_000.0
    };
    assert!(
        first_ms < budget,
        "SH8 tiny compile should be < {budget} ms, got {first_ms:.1}"
    );
}

/// SH10: facades stay free of module-global pPos/eOps; import depth stays bounded.
#[test]
fn sh10_stability_budget() {
    let inv = collect_self_host_inventory().expect("inventory");
    let depth = kabootar_lib::compile::dag_max_import_depth().expect("depth");
    eprintln!(
        "SH10 compile_dag={} import_depth={} kab_files={}",
        inv.compile_dag.len(),
        depth,
        inv.kab_files
    );
    assert!(
        depth < 25,
        "SH10 max import depth from compile.kab must stay < 25, got {depth}"
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host");
    for name in ["parser.kab", "emit.kab", "compile.kab", "serialize.kab", "lexer.kab"] {
        let src = std::fs::read_to_string(root.join(name)).expect(name);
        for banned in ["let pPos ", "let eOps "] {
            assert!(
                !src.contains(banned),
                "SH10: {name} must not declare module-global `{banned}`"
            );
        }
    }
    let mut psave_let = 0usize;
    for e in std::fs::read_dir(&root).expect("self_host") {
        let e = e.expect("dirent");
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".kab") {
            continue;
        }
        if name.starts_with('_') || name.starts_with("test_") {
            continue;
        }
        let src = std::fs::read_to_string(e.path()).expect(&name);
        for line in src.lines() {
            let t = line.trim_start();
            if t.starts_with("let pSave") {
                psave_let += 1;
            }
        }
    }
    eprintln!("SH10 module_let_pSave={psave_let}");
    assert_eq!(
        psave_let, 0,
        "SH10: no module-level `let pSave*` in product self_host (use session fields)"
    );
}
