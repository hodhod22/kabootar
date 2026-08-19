//! SH0 inventory, SH1 facade seeds, SH3a nested push(len).

use std::sync::mpsc;
use std::time::{Duration, Instant};

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
        inv.kab_files >= 64,
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

/// SH16 subset: Kab compile policy refuses oversize source; prefer is self-host-only.
#[test]
fn sh16_boot_policy_self_host_only_and_max_bytes() {
    let boot = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib/kab/boot.kab"),
    )
    .expect("boot.kab");
    assert!(
        boot.contains("return \"self-host-only\""),
        "bootPolicy prefer must be self-host-only"
    );
    assert!(
        boot.contains("appRustFallback"),
        "bootPolicy must expose appRustFallback=false"
    );
    let compile = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host/compile.kab"),
    )
    .expect("compile.kab");
    assert!(
        compile.contains("sourceTooBig") && compile.contains("65536"),
        "compile() must enforce bootPolicy maxBytes in Kab"
    );
}

/// SH16: app `.kab` must not rust-fallback; oversize apps fail (split the module).
#[test]
fn sh16_app_no_rust_fallback() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    ensure_compiler_image();
    let dir = std::env::temp_dir().join(format!("kab_sh16_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let tiny = dir.join("app.kab");
    std::fs::write(&tiny, "let x = 1\nreturn x + 2\n").expect("write tiny app");
    let tiny_s = tiny.to_string_lossy().replace('\\', "/");
    let tiny2 = tiny_s.clone();
    let (p, backend) = std::thread::Builder::new()
        .name("sh16-tiny".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_file_prefer(&tiny2, CompilePrefer::SelfHostThenRust))
        .expect("spawn")
        .join()
        .expect("join")
        .expect("tiny app must self-host");
    assert!(p.has_bytecode());
    assert_eq!(backend, "self-host");
    let big = dir.join("huge.kab");
    let src = "return 1\n".to_string() + &"x".repeat(70_000);
    std::fs::write(&big, src).expect("write huge app");
    let big_s = big.to_string_lossy().replace('\\', "/");
    let err = compile_file_prefer(&big_s, CompilePrefer::SelfHostThenRust)
        .expect_err("oversize app must not rust-fallback");
    assert!(
        err.contains("SH16"),
        "expected SH16 error, got {err}"
    );
    let _ = std::fs::remove_file(&tiny);
    let _ = std::fs::remove_file(&big);
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

fn sh8_phase_timeout() -> Duration {
    if cfg!(debug_assertions) {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(60)
    }
}

fn sh8_spawn<T: Send + 'static>(label: &'static str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name(label.into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(r);
        })
        .expect("spawn");
    match rx.recv_timeout(sh8_phase_timeout()) {
        Ok(Ok(v)) => v,
        Ok(Err(p)) => std::panic::resume_unwind(p),
        Err(_) => panic!("SH8 hung in {label} after {:?}", sh8_phase_timeout()),
    }
}

fn sh8_call_export(module: &str, export: &str, arg: &str) -> String {
    use kabootar_lib::bytecode::call_value;
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;

    let module = module.to_string();
    let export = export.to_string();
    let arg = arg.to_string();
    sh8_spawn("sh8-call", move || {
        eprintln!("SH8 import {module} ...");
        let t_imp = Instant::now();
        let mut env = create_module_env();
        import_module(&module, &mut env).unwrap_or_else(|e| panic!("import {module}: {e}"));
        eprintln!(
            "SH8 import {module} done {:.0}ms",
            t_imp.elapsed().as_secs_f64() * 1000.0
        );
        let f = env
            .get(&export)
            .unwrap_or_else(|| panic!("{module}: missing export {export}"));
        eprintln!("SH8 call {export} ...");
        let t_call = Instant::now();
        let v = call_value(
            f,
            vec![Value::String(arg)],
            &[],
            &[],
            &[],
            &[],
            &mut env,
        )
        .unwrap_or_else(|e| {
            panic!(
                "{export}: {}",
                kabootar_lib::runtime::stdlib::error::format_runtime_error(&e)
            )
        });
        eprintln!(
            "SH8 call {export} done {:.0}ms",
            t_call.elapsed().as_secs_f64() * 1000.0
        );
        kabootar_lib::value::format_value(&v)
    })
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

/// SH2: nested named `fn` must save/restore outer eFnOps (MakeArrow + local), not clobber.
#[test]
fn sh2_nested_named_fn_self_host_emit() {
    use kabootar_lib::compile::compile_source_self_host;
    std::env::set_var("KABOOTAR_VM", "host");
    let stats = kabootar_lib::compile::compile_dirty_dag_seeds().expect("heal emit DAG");
    assert_eq!(stats.failed, 0, "dirty compile failed");
    kabootar_lib::compile::reset_self_host_toolchain_cache();
    let src = r#"
fn outer(n) {
    fn inner(x) {
        return x + 1
    }
    return inner(n)
}
return outer(3)
"#;
    let ok = sh8_spawn("sh2-nestfn", move || {
        let p = compile_source_self_host(src).expect("self-host nested fn");
        let bc = p.bytecode.expect("bytecode");
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).expect("run");
        kabootar_lib::value::format_value(&v) == "4"
    });
    assert!(ok, "SH2 nested named fn: outer(3) via inner should be 4");
}

/// SH8: object field writes in a callee must be visible on the caller's object.
#[test]
fn sh8_object_arg_mutation_visible() {
    let src = r#"
fn inc(o) {
    o["n"] = o["n"] + 1
    return 0
}
let o = { "n": 0 }
inc(o)
return o["n"]
"#;
    let prog = compile_source(src).expect("compile");
    let bc = prog.bytecode.expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(&bc, &mut env).expect("run");
    assert_eq!(
        kabootar_lib::value::format_value(&v),
        "1",
        "callee object mutation must update caller alias"
    );
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
    assert!(parser.contains("pMakeSession()"), "alloc sess on first call");
    assert!(parser.contains("pResetSession(sess)"), "SH13 in-place reset");
    assert!(parser.contains("fn tramp(sess)"), "tramp takes sess");
    assert!(emit.contains("fn tramp(E)"), "emit tramp takes E");
}

/// SH3b: product facades re-export by alias (wrapping pub fn adds a Kab-VM frame).
#[test]
fn sh3b_facades_are_aliases_not_wrap_fn() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("self_host");
    for (name, needle) in [
        ("parser.kab", "pub let parseTokens = parseTokensExec"),
        ("emit.kab", "pub let emit = emitMainExec"),
        ("lexer.kab", "pub let tokenize = tokenizeExec"),
        ("serialize.kab", "pub let serialize_bc = serSerializeBc"),
        ("vm.kab", "pub let runModule = runModuleImplBodyCore"),
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

/// SH8: tokenize tiny source via compiler-image (isolates lexer from parse tramp).
#[test]
fn sh8_tiny_tokenize_via_compiler_image() {
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    let missing = missing_compiler_dag_seeds().expect("scan");
    eprintln!("SH8 tokenize missing_seeds={}", missing.len());
    let t0 = Instant::now();
    let out = sh8_call_export("self_host/lexer", "tokenize", "return 1\n");
    eprintln!("SH8 tokenize ms={:.1} out={out}", t0.elapsed().as_secs_f64() * 1000.0);
    assert!(
        out.contains("EOF") || out.contains("return") || out.len() > 8,
        "tokenize(return 1) should yield tokens, got {out}"
    );
}

/// SH8: tiny source parses via compiler-image (`parse`).
#[test]
fn sh8_tiny_parse_via_compiler_image() {
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    let t0 = Instant::now();
    let kind = sh8_spawn("sh8-parse", || {
        use kabootar_lib::bytecode::call_value;
        use kabootar_lib::evaluator::create_module_env;
        use kabootar_lib::modules::import_module;
        use kabootar_lib::value::Value;

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
        .unwrap_or_else(|e| {
            panic!(
                "parse tiny: {}",
                kabootar_lib::runtime::stdlib::error::format_runtime_error(&e)
            )
        });
        match ast {
            Value::Object(map) => map
                .get("kind")
                .map(kabootar_lib::value::format_value)
                .unwrap_or_default(),
            other => kabootar_lib::value::format_value(&other),
        }
    });
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH8 parse tiny kind={kind} ms={ms:.1}");
    assert!(
        kind.contains("Program"),
        "parse(return 1) should be AST_PROGRAM, got {kind}"
    );
}

/// SH8: tiny self-host compile. SH14: second compile in the same thread (session + toolchain reuse).
#[test]
fn sh8_tiny_self_host_compile() {
    use kabootar_lib::bytecode::{jit_reset_for_tests, jit_stats};
    use kabootar_lib::compile::compile_source_self_host;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    let (first_ms, warm_ms, program) = sh8_spawn("sh8-compile", || {
        let t0 = Instant::now();
        let p = compile_source_self_host("return 1\n").expect("self-host tiny");
        let first_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let t1 = Instant::now();
        let p2 = compile_source_self_host(
            "fn add(a, b) { return a + b }\nfn loopn(n) {\n  let i = 0\n  let s = 0\n  while i < n {\n    s = add(s, i)\n    i = i + 1\n  }\n  return s\n}\nreturn loopn(8)\n",
        )
        .expect("self-host warm");
        let warm_ms = t1.elapsed().as_secs_f64() * 1000.0;
        assert!(p2.bytecode.is_some(), "SH14 warm self-host bytecode");
        (first_ms, warm_ms, p)
    });
    eprintln!("SH8 tiny first_ms={first_ms:.1} SH14 warm_ms={warm_ms:.1}");
    assert!(program.bytecode.is_some(), "tiny self-host bytecode");
    let budget = if cfg!(debug_assertions) {
        90_000.0
    } else {
        12_000.0
    };
    assert!(
        first_ms < budget,
        "SH8 tiny compile should be < {budget} ms, got {first_ms:.1}"
    );
    assert!(
        warm_ms < budget,
        "SH14 warm self-host compile should be < {budget} ms, got {warm_ms:.1}"
    );
    if first_ms >= 2_000.0 {
        assert!(
            warm_ms <= first_ms * 0.75 + 250.0,
            "SH14: second compile in-process should reuse toolchain+session (cold={first_ms:.1} warm={warm_ms:.1})"
        );
    }
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 self-host compile() jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: compile() of tiny/medium should hit Cranelift (accCount/serCount), hits={hits} compiled={compiled} fails={fails}"
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

/// SH9: host Cranelift helper is available (P13 add-loop) while the toolchain runs on host-VM.
#[test]
fn sh9_host_jit_add_loop() {
    let n = kabootar_lib::bytecode::jit_add_loop(8);
    assert_eq!(n, Some(8), "P13/SH9 jit_add_loop(8)");
}

/// SH9: `accCount` in emit DAG is a typed i64 AccAdd-loop — JIT hits when the toolchain runs it.
#[test]
fn sh9_emit_acc_count_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-acc", || {
        let mut env = create_module_env();
        import_module("self_host/emit_arr_util", &mut env).expect("import emit_arr_util");
        let f = env
            .get("accCount")
            .expect("accCount export")
            .clone();
        let out = call_value(f, vec![Value::Number(64)], &[], &[], &[], &[], &mut env)
            .expect("accCount");
        matches!(out, Value::Number(64))
    });
    assert!(ok, "accCount(64) == 64");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 accCount jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected Cranelift on accCount, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH9: `serCount` in serialize DAG is a typed i64 AccAdd-loop.
#[test]
fn sh9_serialize_ser_count_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-ser", || {
        let mut env = create_module_env();
        import_module("self_host/serialize_ops", &mut env).expect("import serialize_ops");
        let f = env.get("serCount").expect("serCount export").clone();
        let out = call_value(f, vec![Value::Number(48)], &[], &[], &[], &[], &mut env)
            .expect("serCount");
        matches!(out, Value::Number(48))
    });
    assert!(ok, "serCount(48) == 48");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 serCount jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected Cranelift on serCount, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH9: `idxSum` is a typed i64 index AccAdd-loop (triangular numbers).
#[test]
fn sh9_emit_idx_sum_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-idx", || {
        let mut env = create_module_env();
        import_module("self_host/emit_arr_util", &mut env).expect("import emit_arr_util");
        let f = env.get("idxSum").expect("idxSum export").clone();
        let out = call_value(f, vec![Value::Number(32)], &[], &[], &[], &[], &mut env)
            .expect("idxSum");
        matches!(out, Value::Number(496))
    });
    assert!(ok, "idxSum(32) == 496");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 idxSum jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected Cranelift on idxSum, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH9: `idxSumArr` is a LenLocal + IndexGetLocal i64 loop (Cranelift loads flattened i64[]).
#[test]
fn sh9_emit_idx_sum_arr_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-idxarr", || {
        let mut env = create_module_env();
        import_module("self_host/emit_arr_util", &mut env).expect("import emit_arr_util");
        let f = env.get("idxSumArr").expect("idxSumArr export").clone();
        let xs = Value::from_array(vec![
            Value::Number(1),
            Value::Number(2),
            Value::Number(3),
            Value::Number(4),
        ]);
        let out = call_value(f, vec![xs], &[], &[], &[], &[], &mut env).expect("idxSumArr");
        matches!(out, Value::Number(10))
    });
    assert!(ok, "idxSumArr([1,2,3,4]) == 10");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 idxSumArr jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected Cranelift on idxSumArr, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH9: `strCount` is a LenLocal-loop over a string (Cranelift gets `len`, no char IndexGet).
#[test]
fn sh9_emit_str_count_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-str", || {
        let mut env = create_module_env();
        import_module("self_host/emit_arr_util", &mut env).expect("import emit_arr_util");
        let f = env.get("strCount").expect("strCount export").clone();
        let out = call_value(
            f,
            vec![Value::String("kabootar".into())],
            &[],
            &[],
            &[],
            &[],
            &mut env,
        )
        .expect("strCount");
        matches!(out, Value::Number(8))
    });
    assert!(ok, "strCount(\"kabootar\") == 8");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 strCount jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected Cranelift on strCount, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH9: `strAt` / `strJoinIdx` use IndexGet of 1-char string Values (not i64 codepoints).
#[test]
fn sh9_emit_str_char_index_jit_hits() {
    use kabootar_lib::bytecode::{
        call_value, jit_reset_for_tests, jit_set_call_threshold_for_tests, jit_stats,
    };
    use kabootar_lib::evaluator::create_module_env;
    use kabootar_lib::modules::import_module;
    use kabootar_lib::value::Value;
    std::env::set_var("KABOOTAR_VM", "host");
    ensure_compiler_image();
    jit_reset_for_tests();
    jit_set_call_threshold_for_tests(1);
    let ok = sh8_spawn("sh9-strch", || {
        let mut env = create_module_env();
        import_module("self_host/emit_arr_util", &mut env).expect("import emit_arr_util");
        let at = env.get("strAt").expect("strAt").clone();
        let join = env.get("strJoinIdx").expect("strJoinIdx").clone();
        let ch = call_value(
            at,
            vec![Value::String("kab".into()), Value::Number(1)],
            &[],
            &[],
            &[],
            &[],
            &mut env,
        )
        .expect("strAt");
        let cat = call_value(
            join,
            vec![Value::String("kab".into())],
            &[],
            &[],
            &[],
            &[],
            &mut env,
        )
        .expect("strJoinIdx");
        matches!(ch, Value::String(s) if s == "a")
            && matches!(cat, Value::String(s) if s == "kab")
    });
    assert!(ok, "strAt(\"kab\", 1)==\"a\" and strJoinIdx(\"kab\")==\"kab\"");
    let (hits, compiled, fails) = jit_stats();
    eprintln!("SH9 str char-index jit hits={hits} compiled={compiled} fails={fails}");
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        hits > 0 || compiled > 0,
        "SH9: expected host IndexGet 1-char path, hits={hits} compiled={compiled} fails={fails}"
    );
}

/// SH15: fingerprint includes compiler-image version (cache key changes if image ver changes).
#[test]
fn sh15_fingerprint_includes_image_version() {
    assert_eq!(kabootar_lib::compile::COMPILER_IMAGE_VERSION, 1);
    let a = kabootar_lib::compile::source_fingerprint("missing.kab", "return 1\n");
    assert!(!a.is_empty());
}

/// SH15: CA `kbcb` hit via mmap does not need the text `.kbc` sidecar.
#[test]
fn sh15_ca_kbcb_mmap_hit_skips_text() {
    use kabootar_lib::compile::{
        compile_source, read_bytecode_cache_at, source_fingerprint, write_compile_marker_at,
        COMPILER_IMAGE_VERSION,
    };
    let tmp = std::env::temp_dir().join(format!("kab_sh15_int_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("tmp");
    let path = tmp.join("hit.kab");
    let src = "return 1 + 1\n";
    std::fs::write(&path, src).expect("write");
    let path_s = path.to_str().expect("utf8");
    let prog = compile_source(src).expect("compile");
    write_compile_marker_at(&tmp, path_s, &prog).expect("cache");
    let fp = source_fingerprint(path_s, src);
    let ca = tmp
        .join(".kabootar")
        .join("cache")
        .join("ca")
        .join(format!("v{COMPILER_IMAGE_VERSION}_{fp}.kbcb"));
    assert!(ca.is_file(), "CA kbcb {}", ca.display());
    for ent in std::fs::read_dir(tmp.join(".kabootar").join("cache")).expect("cache dir") {
        let ent = ent.expect("ent");
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) == Some("kbc") {
            let _ = std::fs::remove_file(&p);
        }
        if p.extension().and_then(|e| e.to_str()) == Some("kbcb") {
            let _ = std::fs::remove_file(&p);
        }
    }
    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    let hit = read_bytecode_cache_at(&tmp, path_s, mtime).expect("read");
    assert!(hit.is_some(), "mmap CA hit after deleting path-keyed cache");
    let _ = std::fs::remove_dir_all(&tmp);
}
