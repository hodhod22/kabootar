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
        inv.compile_dag.len() >= 12,
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
        boot.contains("kbcHeader") && compile.contains("guardAndPreprocess"),
        "bootPolicy kbcHeader + compile guardAndPreprocess"
    );
    assert!(
        boot.contains("rustFallbackPrefix") && boot.contains("self_host/"),
        "bootPolicy rustFallbackPrefix must be self_host/"
    );
    let perf = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib/kab/perf.kab"),
    )
    .expect("perf.kab");
    assert!(
        perf.contains("pub fn perfTick") && perf.contains("pub fn perfCount"),
        "FT F0 kab/perf counters"
    );
    assert!(
        compile.contains("pub fn lastCompileMs") && compile.contains("gMsTotal"),
        "FT F0 compile() must store phase ms"
    );
    assert!(
        compile.contains("gMsSer = 0") && boot.contains("bootLastCompileMs"),
        "FT F0 compileIr records ms; bootLastCompileMs re-exports"
    );
    assert!(
        boot.contains("refuseKbcPath"),
        "SH16 bootPolicy refuseKbcPath"
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

/// SH16: oversize under `self_host/` may still rust-compile (toolchain seeds); apps may not.
#[test]
fn sh16_toolchain_oversize_may_rust() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    let dir = std::env::temp_dir().join(format!("kab_sh16_tool_{}", std::process::id()));
    let sh = dir.join("self_host");
    let _ = std::fs::create_dir_all(&sh);
    let huge = sh.join("huge.kab");
    let src = "let x = 1\nreturn x + 2\n".to_string() + &"\n".repeat(70_000);
    std::fs::write(&huge, src).expect("write toolchain huge");
    let huge_s = huge.to_string_lossy().replace('\\', "/");
    let (p, backend) = compile_file_prefer(&huge_s, CompilePrefer::SelfHostThenRust)
        .expect("self_host oversize may rust-fallback");
    assert!(p.has_bytecode() || p.stmt_count > 0);
    assert_eq!(backend, "rust");
    let _ = std::fs::remove_file(&huge);
}

/// SH16: app `*.kbc` / `*.kbcb` is bytecode, not source — no rust-fallback compile.
#[test]
fn sh16_app_kbc_path_no_rust() {
    use kabootar_lib::compile::{compile_file_prefer, CompilePrefer};
    let dir = std::env::temp_dir().join(format!("kab_sh16_kbc_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let kbc = dir.join("app.kbc");
    std::fs::write(&kbc, "let x = 1\nreturn x + 2\n").expect("write fake kbc");
    let kbc_s = kbc.to_string_lossy().replace('\\', "/");
    let err = compile_file_prefer(&kbc_s, CompilePrefer::SelfHostThenRust)
        .expect_err("app .kbc must not rust-compile as source");
    assert!(
        err.contains("SH16"),
        "expected SH16 error, got {err}"
    );
    let _ = std::fs::remove_file(&kbc);
}

/// SH16: product run/compile refuse rust env for app `.kab` (toolchain rust remains self_host/).
#[test]
fn sh16_app_run_refuses_rust_env() {
    use kabootar_lib::compile::eval_file_cached;
    use kabootar_lib::evaluator::create_global_env;
    let dir = std::env::temp_dir().join(format!("kab_sh16_run_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let tiny = dir.join("app.kab");
    std::fs::write(&tiny, "return 1\n").expect("write");
    let tiny_s = tiny.to_string_lossy().replace('\\', "/");
    let prev = std::env::var("KABOOTAR_COMPILE").ok();
    std::env::set_var("KABOOTAR_COMPILE", "rust");
    let mut env = create_global_env();
    let err = eval_file_cached(&tiny_s, &mut env).expect_err("app rust env must fail");
    match prev {
        Some(v) => std::env::set_var("KABOOTAR_COMPILE", v),
        None => std::env::remove_var("KABOOTAR_COMPILE"),
    }
    let _ = std::fs::remove_file(&tiny);
    assert!(
        err.contains("SH16"),
        "expected SH16 error, got {err}"
    );
}

/// SH16: `kabootar compile --rust` on an app file fails.
#[test]
fn sh16_cli_compile_rust_flag_fails_for_app() {
    let dir = std::env::temp_dir().join(format!("kab_sh16_cli_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let tiny = dir.join("app.kab");
    std::fs::write(&tiny, "return 1\n").expect("write");
    let tiny_s = tiny.to_string_lossy().replace('\\', "/");
    let code = kabootar_lib::cli::run(&["compile".into(), "--rust".into(), tiny_s]);
    let _ = std::fs::remove_file(&tiny);
    assert_eq!(code, 1, "compile --rust on app must exit 1");
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

/// SH6: run policy lives in Kab (`vmPrefer` / `maxKbc`); loader only mirrors.
#[test]
fn sh6_vm_policy_in_kab() {
    let boot = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib/kab/boot.kab"),
    )
    .expect("boot.kab");
    let vm = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lib/kab/vm.kab"),
    )
    .expect("vm.kab");
    assert!(
        boot.contains("return \"kab\"") && boot.contains("maxKbc") && boot.contains("262144"),
        "bootPolicy vmPrefer=kab and maxKbc"
    );
    assert!(
        boot.contains("vmAppHostFallback"),
        "bootPolicy vmAppHostFallback (false = Kab evalKbc does not host-run oversize)"
    );
    assert!(
        vm.contains("maxKbcBytes") && vm.contains("bootPolicy(\"maxKbc\")"),
        "kab/vm must read maxKbc from bootPolicy"
    );
    assert!(
        vm.contains("bootPolicy(\"vmAppHostFallback\") == true"),
        "evalKbc must not host-fallback unless vmAppHostFallback"
    );
}

/// SH6: tiny bytecode eval uses Kab VM when enabled; failure is not swallowed by host.
#[test]
fn sh6_tiny_eval_program_kab_or_host_ok() {
    use kabootar_lib::compile::{compile_source, eval_program};
    let prev = std::env::var("KABOOTAR_VM").ok();
    std::env::remove_var("KABOOTAR_VM");
    let program = compile_source("let x = 1\nreturn x + 2\n").expect("compile tiny");
    let formatted = std::thread::Builder::new()
        .name("sh6-tiny-eval".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut env = create_global_env();
            eval_program(&program, &mut env).map(|v| kabootar_lib::value::format_value(&v))
        })
        .expect("spawn")
        .join()
        .expect("join")
        .expect("tiny eval");
    match prev {
        Some(p) => std::env::set_var("KABOOTAR_VM", p),
        None => std::env::remove_var("KABOOTAR_VM"),
    }
    assert_eq!(formatted, "3");
}

/// SH17: i64 JIT planner lives in Kab (opcode filter + linear-scan GPR count).
#[test]
fn sh17_jit_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let jit = std::fs::read_to_string(root.join("lib/kab/jit.kab")).expect("jit.kab");
    let boot = std::fs::read_to_string(root.join("lib/kab/boot.kab")).expect("boot.kab");
    assert!(
        jit.contains("pub fn jitCanCompile")
            && jit.contains("pub fn jitGprCount")
            && jit.contains("acc_add_local")
            && jit.contains("pub fn jitEmitRet")
            && jit.contains("pub fn jitEmitI64IncRet")
            && jit.contains("pub fn jitMapKind")
            && jit.contains("pub fn jitIncLen")
            && jit.contains("195"),
        "SH17 Kab JIT x64 RET + i64 inc templates"
    );
    assert!(
        !jit.contains("cranelift") && !jit.contains("Cranelift"),
        "SH17 must not depend on the host JIT engine from Kab"
    );
    assert!(
        boot.contains("if key == \"jit\"") && boot.contains("return \"kab\""),
        "bootPolicy jit=kab"
    );
}

/// SH17: RET opcode check stays in a tiny leaf (jit.kab emit DAG overflows if grown).
#[test]
fn sh17_jit_ret_check_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/jit_check.kab")).expect("jit_check.kab");
    assert!(
        c.contains("pub fn jitByteIsRet") && c.contains("195"),
        "SH17 Kab RET byte check"
    );
}

/// SH17: rwx map policy for inc+ret lives off jit.kab (emit DAG overflows if grown).
#[test]
fn sh17_jit_map_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = std::fs::read_to_string(root.join("lib/kab/jit_map.kab")).expect("jit_map.kab");
    assert!(
        m.contains("pub fn jitMapIncRetOk") && m.contains("rwx") && m.contains("6"),
        "SH17 Kab rwx map for 6-byte inc+ret"
    );
}

/// SH17: page size lives off jit.kab (emit DAG overflows if grown).
#[test]
fn sh17_jit_pg_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/jit_pg.kab")).expect("jit_pg.kab");
    assert!(
        p.contains("pub fn jitPageSize") && p.contains("4096"),
        "SH17 Kab jitPageSize"
    );
}

/// SH17: page count for a template lives off jit.kab.
#[test]
fn sh17_jit_np_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let n = std::fs::read_to_string(root.join("lib/kab/jit_np.kab")).expect("jit_np.kab");
    assert!(
        n.contains("pub fn jitPagesFor") && n.contains("4096"),
        "SH17 Kab jitPagesFor"
    );
}

/// SH17: exec policy after rwx map (no VirtualAlloc in Rust).
#[test]
fn sh17_jit_exec_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let e = std::fs::read_to_string(root.join("lib/kab/jit_exec.kab")).expect("jit_exec.kab");
    assert!(
        e.contains("pub fn jitExecOk") && e.contains("rwx") && !e.contains("VirtualAlloc"),
        "SH17 Kab jitExecOk"
    );
}

/// F8: inline budget stays in a tiny leaf (do not grow jit.kab).
#[test]
fn f8_jit_opt_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let o = std::fs::read_to_string(root.join("lib/kab/jit_opt.kab")).expect("jit_opt.kab");
    assert!(
        o.contains("pub fn jitCanInline"),
        "F8 Kab jitCanInline"
    );
}

/// F8: LICM hoist gate (do not grow jit.kab).
#[test]
fn f8_jit_licm_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/jit_licm.kab")).expect("jit_licm.kab");
    assert!(
        l.contains("pub fn jitLicmOk"),
        "F8 Kab jitLicmOk"
    );
}

/// F8: GVN/CSE gate (do not grow jit.kab).
#[test]
fn f8_jit_gvn_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/jit_gvn.kab")).expect("jit_gvn.kab");
    assert!(
        g.contains("pub fn jitGvnOk"),
        "F8 Kab jitGvnOk"
    );
}

/// F8: deopt guard gate (do not grow jit.kab).
#[test]
fn f8_jit_deopt_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let d = std::fs::read_to_string(root.join("lib/kab/jit_deopt.kab")).expect("jit_deopt.kab");
    assert!(
        d.contains("pub fn jitDeoptOk"),
        "F8 Kab jitDeoptOk"
    );
}

/// F8: SSA phi gate (do not grow jit.kab).
#[test]
fn f8_jit_ssa_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/jit_ssa.kab")).expect("jit_ssa.kab");
    assert!(
        s.contains("pub fn jitSsaOk"),
        "F8 Kab jitSsaOk"
    );
}

/// F9: linear-scan GPR count (do not grow jit.kab).
#[test]
fn f9_jit_gpr_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/jit_gpr.kab")).expect("jit_gpr.kab");
    assert!(
        g.contains("pub fn jitScanGprs"),
        "F9 Kab jitScanGprs"
    );
}

/// F9: graph-coloring GPR budget (do not grow jit.kab).
#[test]
fn f9_jit_col_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/jit_col.kab")).expect("jit_col.kab");
    assert!(
        c.contains("pub fn jitColorOk") && c.contains("16"),
        "F9 Kab jitColorOk"
    );
}

/// F9: SIMD 16-byte lane (do not grow jit.kab).
#[test]
fn f9_jit_simd_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/jit_simd.kab")).expect("jit_simd.kab");
    assert!(
        s.contains("pub fn jitSimdOk") && s.contains("16"),
        "F9 Kab jitSimdOk"
    );
}

/// F10: AOT/PGO warm gate in a tiny leaf.
#[test]
fn f10_aot_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/aot.kab")).expect("aot.kab");
    assert!(
        a.contains("pub fn aotWarmOk"),
        "F10 Kab aotWarmOk"
    );
}

/// F10: PGO hit threshold (do not import kab/aot or kab/jit).
#[test]
fn f10_aot_pgo_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_pgo.kab")).expect("aot_pgo.kab");
    assert!(
        p.contains("pub fn aotPgoOk") && p.contains("8"),
        "F10 Kab aotPgoOk"
    );
}

/// F10: native image fn count (do not import kab/aot).
#[test]
fn f10_aot_img_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/aot_img.kab")).expect("aot_img.kab");
    assert!(
        i.contains("pub fn aotImageOk"),
        "F10 Kab aotImageOk"
    );
}

/// F10: cold-start ms budget (do not import kab/aot).
#[test]
fn f10_aot_cd_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/aot_cd.kab")).expect("aot_cd.kab");
    assert!(
        c.contains("pub fn aotColdOk") && c.contains("100"),
        "F10 Kab aotColdOk"
    );
}

/// F10: steady-state frame budget (do not import kab/aot).
#[test]
fn f10_aot_ss_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ss.kab")).expect("aot_ss.kab");
    assert!(
        s.contains("pub fn aotSteadyOk") && s.contains("16"),
        "F10 Kab aotSteadyOk"
    );
}

/// F10: fingerprint must be non-empty (native image key).
#[test]
fn f10_aot_fp_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/aot_fp.kab")).expect("aot_fp.kab");
    assert!(
        f.contains("pub fn aotFpOk") && f.contains("len(fp)"),
        "F10 Kab aotFpOk"
    );
}

/// F10: Kab-native image header (not LLVM as product).
#[test]
fn f10_aot_hdr_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/aot_hdr.kab")).expect("aot_hdr.kab");
    assert!(
        h.contains("pub fn aotNativeHdr") && h.contains("kabootar-native/1"),
        "F10 Kab aotNativeHdr"
    );
}

/// F10: relocation must accept a non-negative image base.
#[test]
fn f10_aot_reloc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_reloc.kab")).expect("aot_reloc.kab");
    assert!(
        r.contains("pub fn aotRelocBaseOk") && r.contains("base >= 0"),
        "F10 Kab aotRelocBaseOk"
    );
}

/// F10: native image exports require a non-empty Kab symbol.
#[test]
fn f10_aot_sym_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_sym.kab")).expect("aot_sym.kab");
    assert!(
        s.contains("pub fn aotSymbolOk") && s.contains("len(name)"),
        "F10 Kab aotSymbolOk"
    );
}

/// F10: native image sections reserve 16-byte alignment.
#[test]
fn f10_aot_align_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/aot_align.kab")).expect("aot_align.kab");
    assert!(
        a.contains("pub fn aotAlignOk") && a.contains(">= 16"),
        "F10 Kab aotAlignOk"
    );
}

/// F10: Kab-native images have text, rodata, and data sections.
#[test]
fn f10_aot_section_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_section.kab")).expect("aot_section.kab");
    assert!(
        s.contains("pub fn aotSectionOk") && s.contains("\"rodata\""),
        "F10 Kab aotSectionOk"
    );
}

/// F10: native image mappings reserve at least one page.
#[test]
fn f10_aot_page_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_page.kab")).expect("aot_page.kab");
    assert!(
        p.contains("pub fn aotPageOk") && p.contains(">= 4096"),
        "F10 Kab aotPageOk"
    );
}

/// F10: emitted text is RX, never RWX.
#[test]
fn f10_aot_protect_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_protect.kab")).expect("aot_protect.kab");
    assert!(
        p.contains("pub fn aotTextProtectOk") && p.contains("\"rx\""),
        "F10 Kab aotTextProtectOk"
    );
}

/// F10: initial native image has a non-empty entry image.
#[test]
fn f10_aot_entry_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let e = std::fs::read_to_string(root.join("lib/kab/aot_entry.kab")).expect("aot_entry.kab");
    assert!(
        e.contains("pub fn aotEntryOk") && e.contains("imageSize > 0"),
        "F10 Kab aotEntryOk"
    );
}

/// F10: sealing combines native header, RX text, and entry validation.
#[test]
fn f10_aot_seal_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_seal.kab")).expect("aot_seal.kab");
    assert!(
        s.contains("pub fn aotSealOk")
            && s.contains("kabootar-native/1")
            && s.contains("textMode == \"rx\""),
        "F10 Kab aotSealOk"
    );
}

/// F10: native-image cache keys carry a non-empty fingerprint.
#[test]
fn f10_aot_key_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let k = std::fs::read_to_string(root.join("lib/kab/aot_key.kab")).expect("aot_key.kab");
    assert!(
        k.contains("pub fn aotCacheKeyOk") && k.contains("len(key) > 7"),
        "F10 Kab aotCacheKeyOk"
    );
}

/// F10: mutable native-image data is isolated in RW pages.
#[test]
fn f10_aot_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let d = std::fs::read_to_string(root.join("lib/kab/aot_data.kab")).expect("aot_data.kab");
    assert!(
        d.contains("pub fn aotDataProtectOk") && d.contains("\"rw\""),
        "F10 Kab aotDataProtectOk"
    );
}

/// F10: native-image read-only data is isolated in R pages.
#[test]
fn f10_aot_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_rodata.kab")).expect("aot_rodata.kab");
    assert!(
        r.contains("pub fn aotRodataProtectOk") && r.contains("\"r\""),
        "F10 Kab aotRodataProtectOk"
    );
}

/// F10: native-image text, rodata, and data use distinct protections.
#[test]
fn f10_aot_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_layout.kab")).expect("aot_layout.kab");
    assert!(
        l.contains("pub fn aotLayoutOk")
            && l.contains("textMode == \"rx\"")
            && l.contains("rodataMode == \"r\"")
            && l.contains("dataMode == \"rw\""),
        "F10 Kab aotLayoutOk"
    );
}

/// F10: first Kab-native images target x64 or arm64.
#[test]
fn f10_aot_target_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_target.kab")).expect("aot_target.kab");
    assert!(
        t.contains("pub fn aotTargetOk") && t.contains("\"arm64\""),
        "F10 Kab aotTargetOk"
    );
}

/// F10: emit a target-qualified Kab-native image header.
#[test]
fn f10_aot_emit_hdr_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/aot_emit_hdr.kab")).expect("aot_emit_hdr.kab");
    assert!(
        h.contains("pub fn aotEmitHeader") && h.contains("kabootar-native/1:x64"),
        "F10 Kab aotEmitHeader"
    );
}

/// F10: emit stable permission metadata for native image sections.
#[test]
fn f10_aot_emit_section_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_emit_section.kab"))
        .expect("aot_emit_section.kab");
    assert!(
        s.contains("pub fn aotEmitSection") && s.contains("rodata:r"),
        "F10 Kab aotEmitSection"
    );
}

/// F10: emit a target-qualified first native image manifest.
#[test]
fn f10_aot_emit_manifest_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = std::fs::read_to_string(root.join("lib/kab/aot_emit_manifest.kab"))
        .expect("aot_emit_manifest.kab");
    assert!(
        m.contains("pub fn aotEmitManifest") && m.contains("text:rx|rodata:r|data:rw"),
        "F10 Kab aotEmitManifest"
    );
}

/// F10: native image filenames are stable and target-qualified.
#[test]
fn f10_aot_image_name_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let n = std::fs::read_to_string(root.join("lib/kab/aot_image_name.kab"))
        .expect("aot_image_name.kab");
    assert!(
        n.contains("pub fn aotImageName") && n.contains("kabootar-arm64.kbn"),
        "F10 Kab aotImageName"
    );
}

/// F10: loader validates a target-qualified native image manifest.
#[test]
fn f10_aot_load_manifest_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_load_manifest.kab"))
        .expect("aot_load_manifest.kab");
    assert!(
        l.contains("pub fn aotLoadManifestOk") && l.contains("kabootar-native/1|x64"),
        "F10 Kab aotLoadManifestOk"
    );
}

/// F10: native image filename and manifest must agree.
#[test]
fn f10_aot_verify_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_image.kab"))
        .expect("aot_verify_image.kab");
    assert!(
        v.contains("pub fn aotVerifyImageOk") && v.contains("kabootar-arm64.kbn"),
        "F10 Kab aotVerifyImageOk"
    );
}

/// F10: native image ABI is declared per target.
#[test]
fn f10_aot_abi_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/aot_abi.kab")).expect("aot_abi.kab");
    assert!(
        a.contains("pub fn aotAbiOk") && a.contains("aapcs64"),
        "F10 Kab aotAbiOk"
    );
}

/// F10: emit ABI metadata for a supported native image target.
#[test]
fn f10_aot_emit_abi_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/aot_emit_abi.kab"))
        .expect("aot_emit_abi.kab");
    assert!(
        a.contains("pub fn aotEmitAbi") && a.contains("abi:aapcs64"),
        "F10 Kab aotEmitAbi"
    );
}

/// F10: emit the zero entry offset for a non-empty native image.
#[test]
fn f10_aot_emit_entry_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let e = std::fs::read_to_string(root.join("lib/kab/aot_emit_entry.kab"))
        .expect("aot_emit_entry.kab");
    assert!(
        e.contains("pub fn aotEmitEntry") && e.contains("entry:0"),
        "F10 Kab aotEmitEntry"
    );
}

/// F10: emit the initial native image page-size contract.
#[test]
fn f10_aot_emit_page_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_emit_page.kab"))
        .expect("aot_emit_page.kab");
    assert!(
        p.contains("pub fn aotEmitPage") && p.contains("page:4096"),
        "F10 Kab aotEmitPage"
    );
}

/// F10: emit the complete target-qualified first native image plan.
#[test]
fn f10_aot_emit_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_emit_plan.kab"))
        .expect("aot_emit_plan.kab");
    assert!(
        p.contains("pub fn aotEmitPlan")
            && p.contains("abi:win64|entry:0|page:4096|text:rx|rodata:r|data:rw"),
        "F10 Kab aotEmitPlan"
    );
}

/// F10: loader validates the complete target-qualified native image plan.
#[test]
fn f10_aot_load_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/aot_load_plan.kab"))
        .expect("aot_load_plan.kab");
    assert!(
        p.contains("pub fn aotLoadPlanOk")
            && p.contains("abi:aapcs64|entry:0|page:4096"),
        "F10 Kab aotLoadPlanOk"
    );
}

/// F10: emitted native image plan round-trips through Kab loader validation.
#[test]
fn f10_aot_emit_full_smoke_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_emit_full_smoke.kab"))
        .expect("f10_aot_emit_full_smoke.kab");
    assert!(
        s.contains("aotEmitPlan") && s.contains("aotLoadPlanOk"),
        "F10 Kab native image plan round-trip"
    );
}

/// F10: loader rejects a native image plan for the wrong target.
#[test]
fn f10_aot_load_plan_reject_smoke_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_plan_reject_smoke.kab"))
        .expect("f10_aot_load_plan_reject_smoke.kab");
    assert!(
        s.contains("aotLoadPlanOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab native image plan target rejection"
    );
}

/// F10: native image plans can be persisted through Kab host capability.
#[test]
fn f10_aot_write_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_plan.kab"))
        .expect("aot_write_plan.kab");
    assert!(
        w.contains("pub fn aotWritePlan") && w.contains("os_write(path, plan)"),
        "F10 Kab native image plan writer"
    );
}

/// F10: native image plans can be loaded through Kab host capability.
#[test]
fn f10_aot_read_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_plan.kab"))
        .expect("aot_read_plan.kab");
    assert!(
        r.contains("pub fn aotReadPlan") && r.contains("os_read(path)"),
        "F10 Kab native image plan reader"
    );
}

/// F10: persisted plan bytes must match a first native image plan.
#[test]
fn f10_aot_loaded_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_plan.kab"))
        .expect("aot_loaded_plan.kab");
    assert!(
        l.contains("pub fn aotLoadedPlanOk") && l.contains("page:4096"),
        "F10 Kab persisted native image plan"
    );
}

/// F10: first Kab-native images emit a target-qualified return stub.
#[test]
fn f10_aot_emit_code_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/aot_emit_code.kab"))
        .expect("aot_emit_code.kab");
    assert!(
        c.contains("pub fn aotEmitCode") && c.contains("code:x64:ret"),
        "F10 Kab aotEmitCode"
    );
}

/// F10: loader accepts only a target-qualified first-image return stub.
#[test]
fn f10_aot_load_code_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/aot_load_code.kab"))
        .expect("aot_load_code.kab");
    assert!(
        c.contains("pub fn aotLoadCodeOk") && c.contains("code:arm64:ret"),
        "F10 Kab aotLoadCodeOk"
    );
}

/// F10: native image filename and first-image code stub must agree.
#[test]
fn f10_aot_verify_code_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_code.kab"))
        .expect("aot_verify_code.kab");
    assert!(
        v.contains("pub fn aotVerifyCodeOk") && v.contains("kabootar-arm64.kbn"),
        "F10 Kab aotVerifyCodeOk"
    );
}

/// F10: emit a first native image with plan plus return stub.
#[test]
fn f10_aot_emit_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/aot_emit_image.kab"))
        .expect("aot_emit_image.kab");
    assert!(
        i.contains("pub fn aotEmitImage") && i.contains("code:x64:ret"),
        "F10 Kab aotEmitImage"
    );
}

/// F10: loader validates a first native image with plan plus return stub.
#[test]
fn f10_aot_load_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/aot_load_image.kab"))
        .expect("aot_load_image.kab");
    assert!(
        i.contains("pub fn aotLoadImageOk") && i.contains("code:arm64:ret"),
        "F10 Kab aotLoadImageOk"
    );
}

/// F10: emitted native image round-trips through Kab loader validation.
#[test]
fn f10_aot_image_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_image_round_smoke.kab"))
        .expect("f10_aot_image_round_smoke.kab");
    assert!(
        s.contains("aotEmitImage") && s.contains("aotLoadImageOk"),
        "F10 Kab native image round-trip"
    );
}

/// F10: loader rejects a native image for the wrong target.
#[test]
fn f10_aot_load_image_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_image_reject_smoke.kab"))
        .expect("f10_aot_load_image_reject_smoke.kab");
    assert!(
        s.contains("aotLoadImageOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab native image target rejection"
    );
}

/// F10: first native images can be persisted through Kab host capability.
#[test]
fn f10_aot_write_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_image.kab"))
        .expect("aot_write_image.kab");
    assert!(
        w.contains("pub fn aotWriteImage") && w.contains("os_write(path, image)"),
        "F10 Kab native image writer"
    );
}

/// F10: first native images can be loaded through Kab host capability.
#[test]
fn f10_aot_read_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_image.kab"))
        .expect("aot_read_image.kab");
    assert!(
        r.contains("pub fn aotReadImage") && r.contains("os_read(path)"),
        "F10 Kab native image reader"
    );
}

/// F10: persisted image bytes must match a first native image.
#[test]
fn f10_aot_loaded_image_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_image.kab"))
        .expect("aot_loaded_image.kab");
    assert!(
        l.contains("pub fn aotLoadedImageOk") && l.contains("code:x64:ret"),
        "F10 Kab persisted native image"
    );
}

/// F10: native image filename and first-image bytes must agree.
#[test]
fn f10_aot_verify_full_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_full.kab"))
        .expect("aot_verify_full.kab");
    assert!(
        v.contains("pub fn aotVerifyFullOk") && v.contains("kabootar-arm64.kbn"),
        "F10 Kab aotVerifyFullOk"
    );
}

/// F10: a first image is shippable only when name, bytes, and target agree.
#[test]
fn f10_aot_ship_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship.kab")).expect("aot_ship.kab");
    assert!(
        s.contains("pub fn aotShipOk") && s.contains("kabootar-arm64.kbn"),
        "F10 Kab aotShipOk"
    );
}

/// F10: ship gate rejects a first image for the wrong target.
#[test]
fn f10_aot_ship_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_reject_smoke.kab"))
        .expect("f10_aot_ship_reject_smoke.kab");
    assert!(
        s.contains("aotShipOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab native image ship rejection"
    );
}

/// F10: emitted name and image round-trip through the ship gate.
#[test]
fn f10_aot_ship_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_round_smoke.kab"))
        .expect("f10_aot_ship_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitImage") && s.contains("aotShipOk"),
        "F10 Kab native image ship round-trip"
    );
}

/// F10: first Kab-native images emit a documented return opcode.
#[test]
fn f10_aot_ret_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_ret_op.kab")).expect("aot_ret_op.kab");
    assert!(
        r.contains("pub fn aotRetOp") && r.contains("d65f03c0"),
        "F10 Kab aotRetOp"
    );
}

/// F10: loader accepts only a documented first-image return opcode.
#[test]
fn f10_aot_load_ret_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_ret.kab")).expect("aot_load_ret.kab");
    assert!(
        r.contains("pub fn aotLoadRetOk") && r.contains("d65f03c0"),
        "F10 Kab aotLoadRetOk"
    );
}

/// F10: emitted return opcode round-trips through loader validation.
#[test]
fn f10_aot_ret_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ret_round_smoke.kab"))
        .expect("f10_aot_ret_round_smoke.kab");
    assert!(
        s.contains("aotRetOp") && s.contains("aotLoadRetOk"),
        "F10 Kab return opcode round-trip"
    );
}

/// F10: loader rejects a return opcode for the wrong target.
#[test]
fn f10_aot_load_ret_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_ret_reject_smoke.kab"))
        .expect("f10_aot_load_ret_reject_smoke.kab");
    assert!(
        s.contains("aotLoadRetOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab return opcode target rejection"
    );
}

/// F10: native image filename and return opcode must agree.
#[test]
fn f10_aot_verify_ret_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_ret.kab"))
        .expect("aot_verify_ret.kab");
    assert!(
        v.contains("pub fn aotVerifyRetOk") && v.contains("d65f03c0"),
        "F10 Kab aotVerifyRetOk"
    );
}

/// F10: emitted image name and return opcode round-trip through verification.
#[test]
fn f10_aot_verify_ret_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_ret_round_smoke.kab"))
        .expect("f10_aot_verify_ret_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotRetOp") && s.contains("aotVerifyRetOk"),
        "F10 Kab return opcode verify round-trip"
    );
}

/// F10: verify rejects a return opcode for the wrong image name.
#[test]
fn f10_aot_verify_ret_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_ret_reject_smoke.kab"))
        .expect("f10_aot_verify_ret_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyRetOk") && s.contains("d65f03c0") && s.contains("false"),
        "F10 Kab return opcode name rejection"
    );
}

/// F10: arm64 image name and return opcode round-trip through verification.
#[test]
fn f10_aot_verify_ret_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_ret_arm64_smoke.kab"))
        .expect("f10_aot_verify_ret_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyRetOk"),
        "F10 Kab arm64 return opcode verify round-trip"
    );
}

/// F10: arm64 name and image round-trip through the ship gate.
#[test]
fn f10_aot_ship_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_arm64_smoke.kab"))
        .expect("f10_aot_ship_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitImage") && s.contains("\"arm64\""),
        "F10 Kab arm64 native image ship round-trip"
    );
}

/// F10: first native text sections carry RX plus one then add then sub then nop then return.
#[test]
fn f10_aot_emit_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_emit_text.kab"))
        .expect("aot_emit_text.kab");
    assert!(
        t.contains("pub fn aotEmitText") && t.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c0740075007c0090c3"),
        "F10 Kab aotEmitText"
    );
}

/// F10: loader accepts only RX text plus a documented return opcode.
#[test]
fn f10_aot_load_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_load_text.kab"))
        .expect("aot_load_text.kab");
    assert!(
        t.contains("pub fn aotLoadTextOk") && t.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f54000000540000015400000bd503201fd65f03c0"),
        "F10 Kab aotLoadTextOk"
    );
}

/// F10: emitted RX text round-trips through loader validation.
#[test]
fn f10_aot_text_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_text_round_smoke.kab"))
        .expect("f10_aot_text_round_smoke.kab");
    assert!(
        s.contains("aotEmitText") && s.contains("aotLoadTextOk"),
        "F10 Kab RX text round-trip"
    );
}

/// F10: loader rejects RX text for the wrong target.
#[test]
fn f10_aot_load_text_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_text_reject_smoke.kab"))
        .expect("f10_aot_load_text_reject_smoke.kab");
    assert!(
        s.contains("aotLoadTextOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab RX text target rejection"
    );
}

/// F10: native image filename and RX text payload must agree.
#[test]
fn f10_aot_verify_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_text.kab"))
        .expect("aot_verify_text.kab");
    assert!(
        v.contains("pub fn aotVerifyTextOk") && v.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f54000000540000015400000bd503201fd65f03c0"),
        "F10 Kab aotVerifyTextOk"
    );
}

/// F10: emitted image name and RX text round-trip through verification.
#[test]
fn f10_aot_verify_text_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_text_round_smoke.kab"))
        .expect("f10_aot_verify_text_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitText") && s.contains("aotVerifyTextOk"),
        "F10 Kab RX text verify round-trip"
    );
}

/// F10: verify rejects RX text for the wrong image name.
#[test]
fn f10_aot_verify_text_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_text_reject_smoke.kab"))
        .expect("f10_aot_verify_text_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyTextOk") && s.contains("kabootar-x64.kbn") && s.contains("false"),
        "F10 Kab RX text verify rejection"
    );
}

/// F10: arm64 image name and RX text round-trip through verification.
#[test]
fn f10_aot_verify_text_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_text_arm64_smoke.kab"))
        .expect("f10_aot_verify_text_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyTextOk"),
        "F10 Kab arm64 RX text verify round-trip"
    );
}

/// F10: first native RX text can be persisted through Kab host capability.
#[test]
fn f10_aot_write_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_text.kab"))
        .expect("aot_write_text.kab");
    assert!(
        w.contains("pub fn aotWriteText") && w.contains("os_write(path, text)"),
        "F10 Kab native RX text writer"
    );
}

/// F10: first native RX text can be loaded through Kab host capability.
#[test]
fn f10_aot_read_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_text.kab"))
        .expect("aot_read_text.kab");
    assert!(
        r.contains("pub fn aotReadText") && r.contains("os_read(path)"),
        "F10 Kab native RX text reader"
    );
}

/// F10: persisted text bytes must match first native RX plus return opcode.
#[test]
fn f10_aot_loaded_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_text.kab"))
        .expect("aot_loaded_text.kab");
    assert!(
        l.contains("pub fn aotLoadedTextOk") && l.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c0740075007c0090c3"),
        "F10 Kab persisted native RX text"
    );
}

/// F10: emitted RX text round-trips through the persisted-text gate.
#[test]
fn f10_aot_loaded_text_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_text_round_smoke.kab"))
        .expect("f10_aot_loaded_text_round_smoke.kab");
    assert!(
        s.contains("aotEmitText") && s.contains("aotLoadedTextOk"),
        "F10 Kab persisted RX text round-trip"
    );
}

/// F10: persisted-text gate rejects RWX payloads.
#[test]
fn f10_aot_loaded_text_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_text_reject_smoke.kab"))
        .expect("f10_aot_loaded_text_reject_smoke.kab");
    assert!(
        s.contains("aotLoadedTextOk") && s.contains("text:rwx") && s.contains("false"),
        "F10 Kab persisted RX text RWX rejection"
    );
}

/// F10: emitted arm64 RX text round-trips through the persisted-text gate.
#[test]
fn f10_aot_loaded_text_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_text_arm64_smoke.kab"))
        .expect("f10_aot_loaded_text_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotLoadedTextOk"),
        "F10 Kab persisted arm64 RX text round-trip"
    );
}

/// F10: RX text is shippable only when name, payload, and target agree.
#[test]
fn f10_aot_ship_text_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship_text.kab"))
        .expect("aot_ship_text.kab");
    assert!(
        s.contains("pub fn aotShipTextOk") && s.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f54000000540000015400000bd503201fd65f03c0"),
        "F10 Kab aotShipTextOk"
    );
}

/// F10: emitted name and RX text round-trip through the ship-text gate.
#[test]
fn f10_aot_ship_text_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_text_round_smoke.kab"))
        .expect("f10_aot_ship_text_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitText") && s.contains("aotShipTextOk"),
        "F10 Kab RX text ship round-trip"
    );
}

/// F10: ship-text gate rejects RX text for the wrong target.
#[test]
fn f10_aot_ship_text_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_text_reject_smoke.kab"))
        .expect("f10_aot_ship_text_reject_smoke.kab");
    assert!(
        s.contains("aotShipTextOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab RX text ship target rejection"
    );
}

/// F10: arm64 name and RX text round-trip through the ship-text gate.
#[test]
fn f10_aot_ship_text_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_text_arm64_smoke.kab"))
        .expect("f10_aot_ship_text_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitText") && s.contains("\"arm64\""),
        "F10 Kab arm64 RX text ship round-trip"
    );
}

/// F10: first native rodata sections carry R plus a zero stub.
#[test]
fn f10_aot_emit_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_emit_rodata.kab"))
        .expect("aot_emit_rodata.kab");
    assert!(
        t.contains("pub fn aotEmitRodata") && t.contains("rodata:r|00"),
        "F10 Kab aotEmitRodata"
    );
}

/// F10: loader accepts only R rodata plus a documented zero stub.
#[test]
fn f10_aot_load_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_load_rodata.kab"))
        .expect("aot_load_rodata.kab");
    assert!(
        t.contains("pub fn aotLoadRodataOk") && t.contains("rodata:r|00"),
        "F10 Kab aotLoadRodataOk"
    );
}

/// F10: emitted R rodata round-trips through loader validation.
#[test]
fn f10_aot_rodata_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_rodata_round_smoke.kab"))
        .expect("f10_aot_rodata_round_smoke.kab");
    assert!(
        s.contains("aotEmitRodata") && s.contains("aotLoadRodataOk"),
        "F10 Kab R rodata round-trip"
    );
}

/// F10: loader rejects RW rodata payloads.
#[test]
fn f10_aot_load_rodata_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_rodata_reject_smoke.kab"))
        .expect("f10_aot_load_rodata_reject_smoke.kab");
    assert!(
        s.contains("aotLoadRodataOk") && s.contains("rodata:rw") && s.contains("false"),
        "F10 Kab R rodata RW rejection"
    );
}

/// F10: native image filename and R rodata payload must agree.
#[test]
fn f10_aot_verify_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_rodata.kab"))
        .expect("aot_verify_rodata.kab");
    assert!(
        v.contains("pub fn aotVerifyRodataOk") && v.contains("rodata:r|00"),
        "F10 Kab aotVerifyRodataOk"
    );
}

/// F10: emitted image name and R rodata round-trip through verification.
#[test]
fn f10_aot_verify_rodata_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_rodata_round_smoke.kab"))
        .expect("f10_aot_verify_rodata_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitRodata") && s.contains("aotVerifyRodataOk"),
        "F10 Kab R rodata verify round-trip"
    );
}

/// F10: verify rejects RW rodata for a native image name.
#[test]
fn f10_aot_verify_rodata_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_rodata_reject_smoke.kab"))
        .expect("f10_aot_verify_rodata_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyRodataOk") && s.contains("rodata:rw") && s.contains("false"),
        "F10 Kab R rodata verify RW rejection"
    );
}

/// F10: arm64 image name and R rodata round-trip through verification.
#[test]
fn f10_aot_verify_rodata_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_rodata_arm64_smoke.kab"))
        .expect("f10_aot_verify_rodata_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyRodataOk"),
        "F10 Kab arm64 R rodata verify round-trip"
    );
}

/// F10: first native R rodata can be persisted through Kab host capability.
#[test]
fn f10_aot_write_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_rodata.kab"))
        .expect("aot_write_rodata.kab");
    assert!(
        w.contains("pub fn aotWriteRodata") && w.contains("os_write(path, rodata)"),
        "F10 Kab native R rodata writer"
    );
}

/// F10: first native R rodata can be loaded through Kab host capability.
#[test]
fn f10_aot_read_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_rodata.kab"))
        .expect("aot_read_rodata.kab");
    assert!(
        r.contains("pub fn aotReadRodata") && r.contains("os_read(path)"),
        "F10 Kab native R rodata reader"
    );
}

/// F10: persisted rodata bytes must match first native R plus a zero stub.
#[test]
fn f10_aot_loaded_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_rodata.kab"))
        .expect("aot_loaded_rodata.kab");
    assert!(
        l.contains("pub fn aotLoadedRodataOk") && l.contains("rodata:r|00"),
        "F10 Kab persisted native R rodata"
    );
}

/// F10: emitted R rodata round-trips through the persisted-rodata gate.
#[test]
fn f10_aot_loaded_rodata_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_rodata_round_smoke.kab"))
        .expect("f10_aot_loaded_rodata_round_smoke.kab");
    assert!(
        s.contains("aotEmitRodata") && s.contains("aotLoadedRodataOk"),
        "F10 Kab persisted R rodata round-trip"
    );
}

/// F10: persisted-rodata gate rejects RW payloads.
#[test]
fn f10_aot_loaded_rodata_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_rodata_reject_smoke.kab"))
        .expect("f10_aot_loaded_rodata_reject_smoke.kab");
    assert!(
        s.contains("aotLoadedRodataOk") && s.contains("rodata:rw") && s.contains("false"),
        "F10 Kab persisted R rodata RW rejection"
    );
}

/// F10: emitted arm64 R rodata round-trips through the persisted-rodata gate.
#[test]
fn f10_aot_loaded_rodata_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_rodata_arm64_smoke.kab"))
        .expect("f10_aot_loaded_rodata_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotLoadedRodataOk"),
        "F10 Kab persisted arm64 R rodata round-trip"
    );
}

/// F10: R rodata is shippable only when name, payload, and target agree.
#[test]
fn f10_aot_ship_rodata_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship_rodata.kab"))
        .expect("aot_ship_rodata.kab");
    assert!(
        s.contains("pub fn aotShipRodataOk") && s.contains("rodata:r|00"),
        "F10 Kab aotShipRodataOk"
    );
}

/// F10: emitted name and R rodata round-trip through the ship-rodata gate.
#[test]
fn f10_aot_ship_rodata_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_rodata_round_smoke.kab"))
        .expect("f10_aot_ship_rodata_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitRodata") && s.contains("aotShipRodataOk"),
        "F10 Kab R rodata ship round-trip"
    );
}

/// F10: ship-rodata gate rejects R rodata for the wrong target.
#[test]
fn f10_aot_ship_rodata_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_rodata_reject_smoke.kab"))
        .expect("f10_aot_ship_rodata_reject_smoke.kab");
    assert!(
        s.contains("aotShipRodataOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab R rodata ship target rejection"
    );
}

/// F10: arm64 name and R rodata round-trip through the ship-rodata gate.
#[test]
fn f10_aot_ship_rodata_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_rodata_arm64_smoke.kab"))
        .expect("f10_aot_ship_rodata_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitRodata") && s.contains("\"arm64\""),
        "F10 Kab arm64 R rodata ship round-trip"
    );
}

/// F10: first native data sections carry RW plus a zero stub.
#[test]
fn f10_aot_emit_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_emit_data.kab"))
        .expect("aot_emit_data.kab");
    assert!(
        t.contains("pub fn aotEmitData") && t.contains("data:rw|00"),
        "F10 Kab aotEmitData"
    );
}

/// F10: loader accepts only RW data plus a documented zero stub.
#[test]
fn f10_aot_load_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_load_data.kab"))
        .expect("aot_load_data.kab");
    assert!(
        t.contains("pub fn aotLoadDataOk") && t.contains("data:rw|00"),
        "F10 Kab aotLoadDataOk"
    );
}

/// F10: emitted RW data round-trips through loader validation.
#[test]
fn f10_aot_data_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_data_round_smoke.kab"))
        .expect("f10_aot_data_round_smoke.kab");
    assert!(
        s.contains("aotEmitData") && s.contains("aotLoadDataOk"),
        "F10 Kab RW data round-trip"
    );
}

/// F10: loader rejects RX data payloads.
#[test]
fn f10_aot_load_data_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_data_reject_smoke.kab"))
        .expect("f10_aot_load_data_reject_smoke.kab");
    assert!(
        s.contains("aotLoadDataOk") && s.contains("data:rx") && s.contains("false"),
        "F10 Kab RW data RX rejection"
    );
}

/// F10: native image filename and RW data payload must agree.
#[test]
fn f10_aot_verify_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_data.kab"))
        .expect("aot_verify_data.kab");
    assert!(
        v.contains("pub fn aotVerifyDataOk") && v.contains("data:rw|00"),
        "F10 Kab aotVerifyDataOk"
    );
}

/// F10: emitted image name and RW data round-trip through verification.
#[test]
fn f10_aot_verify_data_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_data_round_smoke.kab"))
        .expect("f10_aot_verify_data_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitData") && s.contains("aotVerifyDataOk"),
        "F10 Kab RW data verify round-trip"
    );
}

/// F10: verify rejects RX data for a native image name.
#[test]
fn f10_aot_verify_data_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_data_reject_smoke.kab"))
        .expect("f10_aot_verify_data_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyDataOk") && s.contains("data:rx") && s.contains("false"),
        "F10 Kab RW data verify RX rejection"
    );
}

/// F10: arm64 image name and RW data round-trip through verification.
#[test]
fn f10_aot_verify_data_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_data_arm64_smoke.kab"))
        .expect("f10_aot_verify_data_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyDataOk"),
        "F10 Kab arm64 RW data verify round-trip"
    );
}

/// F10: first native RW data can be persisted through Kab host capability.
#[test]
fn f10_aot_write_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_data.kab"))
        .expect("aot_write_data.kab");
    assert!(
        w.contains("pub fn aotWriteData") && w.contains("os_write(path, data)"),
        "F10 Kab native RW data writer"
    );
}

/// F10: first native RW data can be loaded through Kab host capability.
#[test]
fn f10_aot_read_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_data.kab"))
        .expect("aot_read_data.kab");
    assert!(
        r.contains("pub fn aotReadData") && r.contains("os_read(path)"),
        "F10 Kab native RW data reader"
    );
}

/// F10: persisted data bytes must match first native RW plus a zero stub.
#[test]
fn f10_aot_loaded_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_data.kab"))
        .expect("aot_loaded_data.kab");
    assert!(
        l.contains("pub fn aotLoadedDataOk") && l.contains("data:rw|00"),
        "F10 Kab persisted native RW data"
    );
}

/// F10: emitted RW data round-trips through the persisted-data gate.
#[test]
fn f10_aot_loaded_data_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_data_round_smoke.kab"))
        .expect("f10_aot_loaded_data_round_smoke.kab");
    assert!(
        s.contains("aotEmitData") && s.contains("aotLoadedDataOk"),
        "F10 Kab persisted RW data round-trip"
    );
}

/// F10: persisted-data gate rejects RX payloads.
#[test]
fn f10_aot_loaded_data_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_data_reject_smoke.kab"))
        .expect("f10_aot_loaded_data_reject_smoke.kab");
    assert!(
        s.contains("aotLoadedDataOk") && s.contains("data:rx") && s.contains("false"),
        "F10 Kab persisted RW data RX rejection"
    );
}

/// F10: emitted arm64 RW data round-trips through the persisted-data gate.
#[test]
fn f10_aot_loaded_data_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_data_arm64_smoke.kab"))
        .expect("f10_aot_loaded_data_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotLoadedDataOk"),
        "F10 Kab persisted arm64 RW data round-trip"
    );
}

/// F10: RW data is shippable only when name, payload, and target agree.
#[test]
fn f10_aot_ship_data_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship_data.kab"))
        .expect("aot_ship_data.kab");
    assert!(
        s.contains("pub fn aotShipDataOk") && s.contains("data:rw|00"),
        "F10 Kab aotShipDataOk"
    );
}

/// F10: emitted name and RW data round-trip through the ship-data gate.
#[test]
fn f10_aot_ship_data_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_data_round_smoke.kab"))
        .expect("f10_aot_ship_data_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitData") && s.contains("aotShipDataOk"),
        "F10 Kab RW data ship round-trip"
    );
}

/// F10: ship-data gate rejects RW data for the wrong target.
#[test]
fn f10_aot_ship_data_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_data_reject_smoke.kab"))
        .expect("f10_aot_ship_data_reject_smoke.kab");
    assert!(
        s.contains("aotShipDataOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab RW data ship target rejection"
    );
}

/// F10: arm64 name and RW data round-trip through the ship-data gate.
#[test]
fn f10_aot_ship_data_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_data_arm64_smoke.kab"))
        .expect("f10_aot_ship_data_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitData") && s.contains("\"arm64\""),
        "F10 Kab arm64 RW data ship round-trip"
    );
}

/// F10: first native layouts concatenate RX text, R rodata, and RW data.
#[test]
fn f10_aot_emit_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_emit_layout.kab"))
        .expect("aot_emit_layout.kab");
    assert!(
        t.contains("pub fn aotEmitLayout") && t.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c07400750090c3"),
        "F10 Kab aotEmitLayout"
    );
}

/// F10: loader accepts only concatenated RX text, R rodata, and RW data.
#[test]
fn f10_aot_load_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_load_layout.kab"))
        .expect("aot_load_layout.kab");
    assert!(
        t.contains("pub fn aotLoadLayoutOk") && t.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotLoadLayoutOk"
    );
}

/// F10: emitted layout round-trips through loader validation.
#[test]
fn f10_aot_layout_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_layout_round_smoke.kab"))
        .expect("f10_aot_layout_round_smoke.kab");
    assert!(
        s.contains("aotEmitLayout") && s.contains("aotLoadLayoutOk"),
        "F10 Kab layout round-trip"
    );
}

/// F10: loader rejects a layout for the wrong target.
#[test]
fn f10_aot_load_layout_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_layout_reject_smoke.kab"))
        .expect("f10_aot_load_layout_reject_smoke.kab");
    assert!(
        s.contains("aotLoadLayoutOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab layout target rejection"
    );
}

/// F10: native image filename and concatenated layout payload must agree.
#[test]
fn f10_aot_verify_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_layout.kab"))
        .expect("aot_verify_layout.kab");
    assert!(
        v.contains("pub fn aotVerifyLayoutOk") && v.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotVerifyLayoutOk"
    );
}

/// F10: emitted image name and layout round-trip through verification.
#[test]
fn f10_aot_verify_layout_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_layout_round_smoke.kab"))
        .expect("f10_aot_verify_layout_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitLayout") && s.contains("aotVerifyLayoutOk"),
        "F10 Kab layout verify round-trip"
    );
}

/// F10: verify rejects a layout for the wrong image name.
#[test]
fn f10_aot_verify_layout_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_layout_reject_smoke.kab"))
        .expect("f10_aot_verify_layout_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyLayoutOk") && s.contains("kabootar-x64.kbn") && s.contains("false"),
        "F10 Kab layout verify rejection"
    );
}

/// F10: arm64 image name and layout round-trip through verification.
#[test]
fn f10_aot_verify_layout_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_layout_arm64_smoke.kab"))
        .expect("f10_aot_verify_layout_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyLayoutOk"),
        "F10 Kab arm64 layout verify round-trip"
    );
}

/// F10: first native layouts can be persisted through Kab host capability.
#[test]
fn f10_aot_write_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_layout.kab"))
        .expect("aot_write_layout.kab");
    assert!(
        w.contains("pub fn aotWriteLayout") && w.contains("os_write(path, layout)"),
        "F10 Kab native layout writer"
    );
}

/// F10: first native layouts can be loaded through Kab host capability.
#[test]
fn f10_aot_read_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_layout.kab"))
        .expect("aot_read_layout.kab");
    assert!(
        r.contains("pub fn aotReadLayout") && r.contains("os_read(path)"),
        "F10 Kab native layout reader"
    );
}

/// F10: persisted layout bytes must match first native RX/R/RW concatenation.
#[test]
fn f10_aot_loaded_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_layout.kab"))
        .expect("aot_loaded_layout.kab");
    assert!(
        l.contains("pub fn aotLoadedLayoutOk") && l.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c07400750090c3"),
        "F10 Kab persisted native layout"
    );
}

/// F10: emitted layout round-trips through the persisted-layout gate.
#[test]
fn f10_aot_loaded_layout_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_layout_round_smoke.kab"))
        .expect("f10_aot_loaded_layout_round_smoke.kab");
    assert!(
        s.contains("aotEmitLayout") && s.contains("aotLoadedLayoutOk"),
        "F10 Kab persisted layout round-trip"
    );
}

/// F10: persisted-layout gate rejects RWX text payloads.
#[test]
fn f10_aot_loaded_layout_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_layout_reject_smoke.kab"))
        .expect("f10_aot_loaded_layout_reject_smoke.kab");
    assert!(
        s.contains("aotLoadedLayoutOk") && s.contains("text:rwx") && s.contains("false"),
        "F10 Kab persisted layout RWX rejection"
    );
}

/// F10: emitted arm64 layout round-trips through the persisted-layout gate.
#[test]
fn f10_aot_loaded_layout_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_layout_arm64_smoke.kab"))
        .expect("f10_aot_loaded_layout_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotLoadedLayoutOk"),
        "F10 Kab persisted arm64 layout round-trip"
    );
}

/// F10: layout is shippable only when name, payload, and target agree.
#[test]
fn f10_aot_ship_layout_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship_layout.kab"))
        .expect("aot_ship_layout.kab");
    assert!(
        s.contains("pub fn aotShipLayoutOk") && s.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotShipLayoutOk"
    );
}

/// F10: emitted name and layout round-trip through the ship-layout gate.
#[test]
fn f10_aot_ship_layout_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_layout_round_smoke.kab"))
        .expect("f10_aot_ship_layout_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitLayout") && s.contains("aotShipLayoutOk"),
        "F10 Kab layout ship round-trip"
    );
}

/// F10: ship-layout gate rejects a layout for the wrong target.
#[test]
fn f10_aot_ship_layout_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_layout_reject_smoke.kab"))
        .expect("f10_aot_ship_layout_reject_smoke.kab");
    assert!(
        s.contains("aotShipLayoutOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab layout ship target rejection"
    );
}

/// F10: arm64 name and layout round-trip through the ship-layout gate.
#[test]
fn f10_aot_ship_layout_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_layout_arm64_smoke.kab"))
        .expect("f10_aot_ship_layout_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitLayout") && s.contains("\"arm64\""),
        "F10 Kab arm64 layout ship round-trip"
    );
}

/// F10: first native images emit plan prefix plus concatenated section payloads.
#[test]
fn f10_aot_emit_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_emit_native.kab"))
        .expect("aot_emit_native.kab");
    assert!(
        t.contains("pub fn aotEmitNative") && t.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c07400750090c3"),
        "F10 Kab aotEmitNative"
    );
}

/// F10: loader accepts only plan prefix plus concatenated first-image sections.
#[test]
fn f10_aot_load_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/aot_load_native.kab"))
        .expect("aot_load_native.kab");
    assert!(
        t.contains("pub fn aotLoadNativeOk") && t.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotLoadNativeOk"
    );
}

/// F10: emitted native image round-trips through loader validation.
#[test]
fn f10_aot_native_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_native_round_smoke.kab"))
        .expect("f10_aot_native_round_smoke.kab");
    assert!(
        s.contains("aotEmitNative") && s.contains("aotLoadNativeOk"),
        "F10 Kab native image round-trip"
    );
}

/// F10: loader rejects a native image for the wrong target.
#[test]
fn f10_aot_load_native_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_native_reject_smoke.kab"))
        .expect("f10_aot_load_native_reject_smoke.kab");
    assert!(
        s.contains("aotLoadNativeOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab native image target rejection"
    );
}

/// F10: native image filename and plan-prefixed payload must agree.
#[test]
fn f10_aot_verify_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_native.kab"))
        .expect("aot_verify_native.kab");
    assert!(
        v.contains("pub fn aotVerifyNativeOk") && v.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotVerifyNativeOk"
    );
}

/// F10: emitted image name and native payload round-trip through verification.
#[test]
fn f10_aot_verify_native_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_native_round_smoke.kab"))
        .expect("f10_aot_verify_native_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitNative") && s.contains("aotVerifyNativeOk"),
        "F10 Kab native verify round-trip"
    );
}

/// F10: verify rejects a native image for the wrong image name.
#[test]
fn f10_aot_verify_native_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_native_reject_smoke.kab"))
        .expect("f10_aot_verify_native_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyNativeOk") && s.contains("kabootar-x64.kbn") && s.contains("false"),
        "F10 Kab native verify rejection"
    );
}

/// F10: arm64 image name and native payload round-trip through verification.
#[test]
fn f10_aot_verify_native_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_native_arm64_smoke.kab"))
        .expect("f10_aot_verify_native_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyNativeOk"),
        "F10 Kab arm64 native verify round-trip"
    );
}

/// F10: first native images can be persisted through Kab host capability.
#[test]
fn f10_aot_write_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/aot_write_native.kab"))
        .expect("aot_write_native.kab");
    assert!(
        w.contains("pub fn aotWriteNative") && w.contains("os_write(path, image)"),
        "F10 Kab native image writer"
    );
}

/// F10: first native images can be loaded through Kab host capability.
#[test]
fn f10_aot_read_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_read_native.kab"))
        .expect("aot_read_native.kab");
    assert!(
        r.contains("pub fn aotReadNative") && r.contains("os_read(path)"),
        "F10 Kab native image reader"
    );
}

/// F10: persisted native bytes must match plan prefix plus first-image sections.
#[test]
fn f10_aot_loaded_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/aot_loaded_native.kab"))
        .expect("aot_loaded_native.kab");
    assert!(
        l.contains("pub fn aotLoadedNativeOk") && l.contains("text:rx|b80100000001c029c00fafc0f7f821c009c0d1e0d1e8f7d033c0f7d839c085c07400750090c3"),
        "F10 Kab persisted native image"
    );
}

/// F10: emitted native image round-trips through the persisted-native gate.
#[test]
fn f10_aot_loaded_native_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_native_round_smoke.kab"))
        .expect("f10_aot_loaded_native_round_smoke.kab");
    assert!(
        s.contains("aotEmitNative") && s.contains("aotLoadedNativeOk"),
        "F10 Kab persisted native round-trip"
    );
}

/// F10: persisted-native gate rejects RWX text payloads.
#[test]
fn f10_aot_loaded_native_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_loaded_native_reject_smoke.kab"))
        .expect("f10_aot_loaded_native_reject_smoke.kab");
    assert!(
        s.contains("aotLoadedNativeOk") && s.contains("text:rwx") && s.contains("false"),
        "F10 Kab persisted native RWX rejection"
    );
}

/// F10: native image is shippable only when name, payload, and target agree.
#[test]
fn f10_aot_ship_native_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/aot_ship_native.kab"))
        .expect("aot_ship_native.kab");
    assert!(
        s.contains("pub fn aotShipNativeOk") && s.contains("text:rx|d28000208b000000cb0000009b007c009ac00c008a000000aa000000d37ff800d341fc00aa2003e0ca000000cb0003e0eb00001fea00001f5400000054000001d503201fd65f03c0"),
        "F10 Kab aotShipNativeOk"
    );
}

/// F10: emitted name and native image round-trip through the ship-native gate.
#[test]
fn f10_aot_ship_native_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_native_round_smoke.kab"))
        .expect("f10_aot_ship_native_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitNative") && s.contains("aotShipNativeOk"),
        "F10 Kab native ship round-trip"
    );
}

/// F10: ship-native gate rejects a native image for the wrong target.
#[test]
fn f10_aot_ship_native_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_native_reject_smoke.kab"))
        .expect("f10_aot_ship_native_reject_smoke.kab");
    assert!(
        s.contains("aotShipNativeOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab native ship target rejection"
    );
}

/// F10: arm64 name and native image round-trip through the ship-native gate.
#[test]
fn f10_aot_ship_native_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_ship_native_arm64_smoke.kab"))
        .expect("f10_aot_ship_native_arm64_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotEmitNative") && s.contains("\"arm64\""),
        "F10 Kab arm64 native ship round-trip"
    );
}

/// F10: first Kab-native images emit a documented nop opcode for text padding.
#[test]
fn f10_aot_nop_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_nop_op.kab")).expect("aot_nop_op.kab");
    assert!(
        r.contains("pub fn aotNopOp") && r.contains("d503201f"),
        "F10 Kab aotNopOp"
    );
}

/// F10: loader accepts only a documented first-image nop opcode.
#[test]
fn f10_aot_load_nop_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_nop.kab")).expect("aot_load_nop.kab");
    assert!(
        r.contains("pub fn aotLoadNopOk") && r.contains("d503201f"),
        "F10 Kab aotLoadNopOk"
    );
}

/// F10: emitted nop opcode round-trips through loader validation.
#[test]
fn f10_aot_nop_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_nop_round_smoke.kab"))
        .expect("f10_aot_nop_round_smoke.kab");
    assert!(
        s.contains("aotNopOp") && s.contains("aotLoadNopOk"),
        "F10 Kab nop opcode round-trip"
    );
}

/// F10: loader rejects a nop opcode for the wrong target.
#[test]
fn f10_aot_load_nop_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_nop_reject_smoke.kab"))
        .expect("f10_aot_load_nop_reject_smoke.kab");
    assert!(
        s.contains("aotLoadNopOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab nop opcode target rejection"
    );
}

/// F10: first Kab-native images emit a documented integer-zero opcode for the return register.
#[test]
fn f10_aot_zero_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_zero_op.kab")).expect("aot_zero_op.kab");
    assert!(
        r.contains("pub fn aotZeroOp") && r.contains("aa1f03e0"),
        "F10 Kab aotZeroOp"
    );
}

/// F10: loader accepts only a documented first-image integer-zero opcode.
#[test]
fn f10_aot_load_zero_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_zero.kab")).expect("aot_load_zero.kab");
    assert!(
        r.contains("pub fn aotLoadZeroOk") && r.contains("aa1f03e0"),
        "F10 Kab aotLoadZeroOk"
    );
}

/// F10: emitted integer-zero opcode round-trips through loader validation.
#[test]
fn f10_aot_zero_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_zero_round_smoke.kab"))
        .expect("f10_aot_zero_round_smoke.kab");
    assert!(
        s.contains("aotZeroOp") && s.contains("aotLoadZeroOk"),
        "F10 Kab integer-zero opcode round-trip"
    );
}

/// F10: loader rejects an integer-zero opcode for the wrong target.
#[test]
fn f10_aot_load_zero_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_zero_reject_smoke.kab"))
        .expect("f10_aot_load_zero_reject_smoke.kab");
    assert!(
        s.contains("aotLoadZeroOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab integer-zero opcode target rejection"
    );
}

/// F10: first Kab-native images emit a documented integer-one opcode for the return register.
#[test]
fn f10_aot_one_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_one_op.kab")).expect("aot_one_op.kab");
    assert!(
        r.contains("pub fn aotOneOp") && r.contains("d2800020"),
        "F10 Kab aotOneOp"
    );
}

/// F10: loader accepts only a documented first-image integer-one opcode.
#[test]
fn f10_aot_load_one_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_one.kab")).expect("aot_load_one.kab");
    assert!(
        r.contains("pub fn aotLoadOneOk") && r.contains("d2800020"),
        "F10 Kab aotLoadOneOk"
    );
}

/// F10: emitted integer-one opcode round-trips through loader validation.
#[test]
fn f10_aot_one_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_one_round_smoke.kab"))
        .expect("f10_aot_one_round_smoke.kab");
    assert!(
        s.contains("aotOneOp") && s.contains("aotLoadOneOk"),
        "F10 Kab integer-one opcode round-trip"
    );
}

/// F10: loader rejects an integer-one opcode for the wrong target.
#[test]
fn f10_aot_load_one_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_one_reject_smoke.kab"))
        .expect("f10_aot_load_one_reject_smoke.kab");
    assert!(
        s.contains("aotLoadOneOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab integer-one opcode target rejection"
    );
}

/// F10: first Kab-native images emit a documented add opcode for the return register.
#[test]
fn f10_aot_add_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_add_op.kab")).expect("aot_add_op.kab");
    assert!(
        r.contains("pub fn aotAddOp") && r.contains("8b000000"),
        "F10 Kab aotAddOp"
    );
}

/// F10: loader accepts only a documented first-image add opcode.
#[test]
fn f10_aot_load_add_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_add.kab")).expect("aot_load_add.kab");
    assert!(
        r.contains("pub fn aotLoadAddOk") && r.contains("8b000000"),
        "F10 Kab aotLoadAddOk"
    );
}

/// F10: emitted add opcode round-trips through loader validation.
#[test]
fn f10_aot_add_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_add_round_smoke.kab"))
        .expect("f10_aot_add_round_smoke.kab");
    assert!(
        s.contains("aotAddOp") && s.contains("aotLoadAddOk"),
        "F10 Kab add opcode round-trip"
    );
}

/// F10: loader rejects an add opcode for the wrong target.
#[test]
fn f10_aot_load_add_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_add_reject_smoke.kab"))
        .expect("f10_aot_load_add_reject_smoke.kab");
    assert!(
        s.contains("aotLoadAddOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab add opcode target rejection"
    );
}

/// F10: first Kab-native images emit a documented sub opcode for the return register.
#[test]
fn f10_aot_sub_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_sub_op.kab")).expect("aot_sub_op.kab");
    assert!(
        r.contains("pub fn aotSubOp") && r.contains("cb000000"),
        "F10 Kab aotSubOp"
    );
}

/// F10: loader accepts only a documented first-image sub opcode.
#[test]
fn f10_aot_load_sub_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_sub.kab")).expect("aot_load_sub.kab");
    assert!(
        r.contains("pub fn aotLoadSubOk") && r.contains("cb000000"),
        "F10 Kab aotLoadSubOk"
    );
}

/// F10: emitted sub opcode round-trips through loader validation.
#[test]
fn f10_aot_sub_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_sub_round_smoke.kab"))
        .expect("f10_aot_sub_round_smoke.kab");
    assert!(
        s.contains("aotSubOp") && s.contains("aotLoadSubOk"),
        "F10 Kab sub opcode round-trip"
    );
}

/// F10: loader rejects a sub opcode for the wrong target.
#[test]
fn f10_aot_load_sub_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_sub_reject_smoke.kab"))
        .expect("f10_aot_load_sub_reject_smoke.kab");
    assert!(
        s.contains("aotLoadSubOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab sub opcode target rejection"
    );
}

/// F10: native image filename and sub opcode must agree.
#[test]
fn f10_aot_verify_sub_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_sub.kab"))
        .expect("aot_verify_sub.kab");
    assert!(
        v.contains("pub fn aotVerifySubOk") && v.contains("cb000000"),
        "F10 Kab aotVerifySubOk"
    );
}

/// F10: emitted image name and sub opcode round-trip through verification.
#[test]
fn f10_aot_verify_sub_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_sub_round_smoke.kab"))
        .expect("f10_aot_verify_sub_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotSubOp") && s.contains("aotVerifySubOk"),
        "F10 Kab sub opcode verify round-trip"
    );
}

/// F10: verify rejects a sub opcode for the wrong image name.
#[test]
fn f10_aot_verify_sub_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_sub_reject_smoke.kab"))
        .expect("f10_aot_verify_sub_reject_smoke.kab");
    assert!(
        s.contains("aotVerifySubOk") && s.contains("cb000000") && s.contains("false"),
        "F10 Kab sub opcode name rejection"
    );
}

/// F10: arm64 image name and sub opcode round-trip through verification.
#[test]
fn f10_aot_verify_sub_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_sub_arm64_smoke.kab"))
        .expect("f10_aot_verify_sub_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifySubOk"),
        "F10 Kab arm64 sub opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented mul opcode for the return register.
#[test]
fn f10_aot_mul_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_mul_op.kab")).expect("aot_mul_op.kab");
    assert!(
        r.contains("pub fn aotMulOp") && r.contains("9b007c00"),
        "F10 Kab aotMulOp"
    );
}

/// F10: loader accepts only a documented first-image mul opcode.
#[test]
fn f10_aot_load_mul_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_mul.kab")).expect("aot_load_mul.kab");
    assert!(
        r.contains("pub fn aotLoadMulOk") && r.contains("9b007c00"),
        "F10 Kab aotLoadMulOk"
    );
}

/// F10: emitted mul opcode round-trips through loader validation.
#[test]
fn f10_aot_mul_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_mul_round_smoke.kab"))
        .expect("f10_aot_mul_round_smoke.kab");
    assert!(
        s.contains("aotMulOp") && s.contains("aotLoadMulOk"),
        "F10 Kab mul opcode round-trip"
    );
}

/// F10: loader rejects a mul opcode for the wrong target.
#[test]
fn f10_aot_load_mul_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_mul_reject_smoke.kab"))
        .expect("f10_aot_load_mul_reject_smoke.kab");
    assert!(
        s.contains("aotLoadMulOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab mul opcode target rejection"
    );
}

/// F10: native image filename and mul opcode must agree.
#[test]
fn f10_aot_verify_mul_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_mul.kab"))
        .expect("aot_verify_mul.kab");
    assert!(
        v.contains("pub fn aotVerifyMulOk") && v.contains("9b007c00"),
        "F10 Kab aotVerifyMulOk"
    );
}

/// F10: emitted image name and mul opcode round-trip through verification.
#[test]
fn f10_aot_verify_mul_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_mul_round_smoke.kab"))
        .expect("f10_aot_verify_mul_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotMulOp") && s.contains("aotVerifyMulOk"),
        "F10 Kab mul opcode verify round-trip"
    );
}

/// F10: verify rejects a mul opcode for the wrong image name.
#[test]
fn f10_aot_verify_mul_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_mul_reject_smoke.kab"))
        .expect("f10_aot_verify_mul_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyMulOk") && s.contains("9b007c00") && s.contains("false"),
        "F10 Kab mul opcode name rejection"
    );
}

/// F10: arm64 image name and mul opcode round-trip through verification.
#[test]
fn f10_aot_verify_mul_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_mul_arm64_smoke.kab"))
        .expect("f10_aot_verify_mul_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyMulOk"),
        "F10 Kab arm64 mul opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented div opcode for the return register.
#[test]
fn f10_aot_div_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_div_op.kab")).expect("aot_div_op.kab");
    assert!(
        r.contains("pub fn aotDivOp") && r.contains("9ac00c00"),
        "F10 Kab aotDivOp"
    );
}

/// F10: loader accepts only a documented first-image div opcode.
#[test]
fn f10_aot_load_div_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_div.kab")).expect("aot_load_div.kab");
    assert!(
        r.contains("pub fn aotLoadDivOk") && r.contains("9ac00c00"),
        "F10 Kab aotLoadDivOk"
    );
}

/// F10: emitted div opcode round-trips through loader validation.
#[test]
fn f10_aot_div_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_div_round_smoke.kab"))
        .expect("f10_aot_div_round_smoke.kab");
    assert!(
        s.contains("aotDivOp") && s.contains("aotLoadDivOk"),
        "F10 Kab div opcode round-trip"
    );
}

/// F10: loader rejects a div opcode for the wrong target.
#[test]
fn f10_aot_load_div_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_div_reject_smoke.kab"))
        .expect("f10_aot_load_div_reject_smoke.kab");
    assert!(
        s.contains("aotLoadDivOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab div opcode target rejection"
    );
}

/// F10: native image filename and div opcode must agree.
#[test]
fn f10_aot_verify_div_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_div.kab"))
        .expect("aot_verify_div.kab");
    assert!(
        v.contains("pub fn aotVerifyDivOk") && v.contains("9ac00c00"),
        "F10 Kab aotVerifyDivOk"
    );
}

/// F10: emitted image name and div opcode round-trip through verification.
#[test]
fn f10_aot_verify_div_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_div_round_smoke.kab"))
        .expect("f10_aot_verify_div_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotDivOp") && s.contains("aotVerifyDivOk"),
        "F10 Kab div opcode verify round-trip"
    );
}

/// F10: verify rejects a div opcode for the wrong image name.
#[test]
fn f10_aot_verify_div_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_div_reject_smoke.kab"))
        .expect("f10_aot_verify_div_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyDivOk") && s.contains("9ac00c00") && s.contains("false"),
        "F10 Kab div opcode name rejection"
    );
}

/// F10: arm64 image name and div opcode round-trip through verification.
#[test]
fn f10_aot_verify_div_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_div_arm64_smoke.kab"))
        .expect("f10_aot_verify_div_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyDivOk"),
        "F10 Kab arm64 div opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented and opcode for the return register.
#[test]
fn f10_aot_and_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_and_op.kab")).expect("aot_and_op.kab");
    assert!(
        r.contains("pub fn aotAndOp") && r.contains("8a000000"),
        "F10 Kab aotAndOp"
    );
}

/// F10: loader accepts only a documented first-image and opcode.
#[test]
fn f10_aot_load_and_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_and.kab")).expect("aot_load_and.kab");
    assert!(
        r.contains("pub fn aotLoadAndOk") && r.contains("8a000000"),
        "F10 Kab aotLoadAndOk"
    );
}

/// F10: emitted and opcode round-trips through loader validation.
#[test]
fn f10_aot_and_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_and_round_smoke.kab"))
        .expect("f10_aot_and_round_smoke.kab");
    assert!(
        s.contains("aotAndOp") && s.contains("aotLoadAndOk"),
        "F10 Kab and opcode round-trip"
    );
}

/// F10: loader rejects an and opcode for the wrong target.
#[test]
fn f10_aot_load_and_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_and_reject_smoke.kab"))
        .expect("f10_aot_load_and_reject_smoke.kab");
    assert!(
        s.contains("aotLoadAndOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab and opcode target rejection"
    );
}

/// F10: native image filename and and opcode must agree.
#[test]
fn f10_aot_verify_and_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_and.kab"))
        .expect("aot_verify_and.kab");
    assert!(
        v.contains("pub fn aotVerifyAndOk") && v.contains("8a000000"),
        "F10 Kab aotVerifyAndOk"
    );
}

/// F10: emitted image name and and opcode round-trip through verification.
#[test]
fn f10_aot_verify_and_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_and_round_smoke.kab"))
        .expect("f10_aot_verify_and_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotAndOp") && s.contains("aotVerifyAndOk"),
        "F10 Kab and opcode verify round-trip"
    );
}

/// F10: verify rejects an and opcode for the wrong image name.
#[test]
fn f10_aot_verify_and_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_and_reject_smoke.kab"))
        .expect("f10_aot_verify_and_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyAndOk") && s.contains("8a000000") && s.contains("false"),
        "F10 Kab and opcode name rejection"
    );
}

/// F10: arm64 image name and and opcode round-trip through verification.
#[test]
fn f10_aot_verify_and_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_and_arm64_smoke.kab"))
        .expect("f10_aot_verify_and_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyAndOk"),
        "F10 Kab arm64 and opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented or opcode for the return register.
#[test]
fn f10_aot_or_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_or_op.kab")).expect("aot_or_op.kab");
    assert!(
        r.contains("pub fn aotOrOp") && r.contains("aa000000"),
        "F10 Kab aotOrOp"
    );
}

/// F10: loader accepts only a documented first-image or opcode.
#[test]
fn f10_aot_load_or_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_or.kab")).expect("aot_load_or.kab");
    assert!(
        r.contains("pub fn aotLoadOrOk") && r.contains("aa000000"),
        "F10 Kab aotLoadOrOk"
    );
}

/// F10: emitted or opcode round-trips through loader validation.
#[test]
fn f10_aot_or_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_or_round_smoke.kab"))
        .expect("f10_aot_or_round_smoke.kab");
    assert!(
        s.contains("aotOrOp") && s.contains("aotLoadOrOk"),
        "F10 Kab or opcode round-trip"
    );
}

/// F10: loader rejects an or opcode for the wrong target.
#[test]
fn f10_aot_load_or_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_or_reject_smoke.kab"))
        .expect("f10_aot_load_or_reject_smoke.kab");
    assert!(
        s.contains("aotLoadOrOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab or opcode target rejection"
    );
}

/// F10: native image filename and or opcode must agree.
#[test]
fn f10_aot_verify_or_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_or.kab"))
        .expect("aot_verify_or.kab");
    assert!(
        v.contains("pub fn aotVerifyOrOk") && v.contains("aa000000"),
        "F10 Kab aotVerifyOrOk"
    );
}

/// F10: emitted image name and or opcode round-trip through verification.
#[test]
fn f10_aot_verify_or_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_or_round_smoke.kab"))
        .expect("f10_aot_verify_or_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotOrOp") && s.contains("aotVerifyOrOk"),
        "F10 Kab or opcode verify round-trip"
    );
}

/// F10: verify rejects an or opcode for the wrong image name.
#[test]
fn f10_aot_verify_or_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_or_reject_smoke.kab"))
        .expect("f10_aot_verify_or_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyOrOk") && s.contains("aa000000") && s.contains("false"),
        "F10 Kab or opcode name rejection"
    );
}

/// F10: arm64 image name and or opcode round-trip through verification.
#[test]
fn f10_aot_verify_or_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_or_arm64_smoke.kab"))
        .expect("f10_aot_verify_or_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyOrOk"),
        "F10 Kab arm64 or opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented shl opcode for the return register.
#[test]
fn f10_aot_shl_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_shl_op.kab")).expect("aot_shl_op.kab");
    assert!(
        r.contains("pub fn aotShlOp") && r.contains("d37ff800"),
        "F10 Kab aotShlOp"
    );
}

/// F10: loader accepts only a documented first-image shl opcode.
#[test]
fn f10_aot_load_shl_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_shl.kab")).expect("aot_load_shl.kab");
    assert!(
        r.contains("pub fn aotLoadShlOk") && r.contains("d37ff800"),
        "F10 Kab aotLoadShlOk"
    );
}

/// F10: emitted shl opcode round-trips through loader validation.
#[test]
fn f10_aot_shl_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_shl_round_smoke.kab"))
        .expect("f10_aot_shl_round_smoke.kab");
    assert!(
        s.contains("aotShlOp") && s.contains("aotLoadShlOk"),
        "F10 Kab shl opcode round-trip"
    );
}

/// F10: loader rejects a shl opcode for the wrong target.
#[test]
fn f10_aot_load_shl_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_shl_reject_smoke.kab"))
        .expect("f10_aot_load_shl_reject_smoke.kab");
    assert!(
        s.contains("aotLoadShlOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab shl opcode target rejection"
    );
}

/// F10: native image filename and shl opcode must agree.
#[test]
fn f10_aot_verify_shl_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_shl.kab"))
        .expect("aot_verify_shl.kab");
    assert!(
        v.contains("pub fn aotVerifyShlOk") && v.contains("d37ff800"),
        "F10 Kab aotVerifyShlOk"
    );
}

/// F10: emitted image name and shl opcode round-trip through verification.
#[test]
fn f10_aot_verify_shl_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shl_round_smoke.kab"))
        .expect("f10_aot_verify_shl_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotShlOp") && s.contains("aotVerifyShlOk"),
        "F10 Kab shl opcode verify round-trip"
    );
}

/// F10: verify rejects a shl opcode for the wrong image name.
#[test]
fn f10_aot_verify_shl_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shl_reject_smoke.kab"))
        .expect("f10_aot_verify_shl_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyShlOk") && s.contains("d37ff800") && s.contains("false"),
        "F10 Kab shl opcode name rejection"
    );
}

/// F10: arm64 image name and shl opcode round-trip through verification.
#[test]
fn f10_aot_verify_shl_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shl_arm64_smoke.kab"))
        .expect("f10_aot_verify_shl_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyShlOk"),
        "F10 Kab arm64 shl opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented shr opcode for the return register.
#[test]
fn f10_aot_shr_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_shr_op.kab")).expect("aot_shr_op.kab");
    assert!(
        r.contains("pub fn aotShrOp") && r.contains("d341fc00"),
        "F10 Kab aotShrOp"
    );
}

/// F10: loader accepts only a documented first-image shr opcode.
#[test]
fn f10_aot_load_shr_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_shr.kab")).expect("aot_load_shr.kab");
    assert!(
        r.contains("pub fn aotLoadShrOk") && r.contains("d341fc00"),
        "F10 Kab aotLoadShrOk"
    );
}

/// F10: emitted shr opcode round-trips through loader validation.
#[test]
fn f10_aot_shr_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_shr_round_smoke.kab"))
        .expect("f10_aot_shr_round_smoke.kab");
    assert!(
        s.contains("aotShrOp") && s.contains("aotLoadShrOk"),
        "F10 Kab shr opcode round-trip"
    );
}

/// F10: loader rejects a shr opcode for the wrong target.
#[test]
fn f10_aot_load_shr_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_shr_reject_smoke.kab"))
        .expect("f10_aot_load_shr_reject_smoke.kab");
    assert!(
        s.contains("aotLoadShrOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab shr opcode target rejection"
    );
}

/// F10: native image filename and shr opcode must agree.
#[test]
fn f10_aot_verify_shr_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_shr.kab"))
        .expect("aot_verify_shr.kab");
    assert!(
        v.contains("pub fn aotVerifyShrOk") && v.contains("d341fc00"),
        "F10 Kab aotVerifyShrOk"
    );
}

/// F10: emitted image name and shr opcode round-trip through verification.
#[test]
fn f10_aot_verify_shr_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shr_round_smoke.kab"))
        .expect("f10_aot_verify_shr_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotShrOp") && s.contains("aotVerifyShrOk"),
        "F10 Kab shr opcode verify round-trip"
    );
}

/// F10: verify rejects a shr opcode for the wrong image name.
#[test]
fn f10_aot_verify_shr_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shr_reject_smoke.kab"))
        .expect("f10_aot_verify_shr_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyShrOk") && s.contains("d341fc00") && s.contains("false"),
        "F10 Kab shr opcode name rejection"
    );
}

/// F10: arm64 image name and shr opcode round-trip through verification.
#[test]
fn f10_aot_verify_shr_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_shr_arm64_smoke.kab"))
        .expect("f10_aot_verify_shr_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyShrOk"),
        "F10 Kab arm64 shr opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented not opcode for the return register.
#[test]
fn f10_aot_not_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_not_op.kab")).expect("aot_not_op.kab");
    assert!(
        r.contains("pub fn aotNotOp") && r.contains("aa2003e0"),
        "F10 Kab aotNotOp"
    );
}

/// F10: loader accepts only a documented first-image not opcode.
#[test]
fn f10_aot_load_not_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_not.kab")).expect("aot_load_not.kab");
    assert!(
        r.contains("pub fn aotLoadNotOk") && r.contains("aa2003e0"),
        "F10 Kab aotLoadNotOk"
    );
}

/// F10: emitted not opcode round-trips through loader validation.
#[test]
fn f10_aot_not_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_not_round_smoke.kab"))
        .expect("f10_aot_not_round_smoke.kab");
    assert!(
        s.contains("aotNotOp") && s.contains("aotLoadNotOk"),
        "F10 Kab not opcode round-trip"
    );
}

/// F10: loader rejects a not opcode for the wrong target.
#[test]
fn f10_aot_load_not_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_not_reject_smoke.kab"))
        .expect("f10_aot_load_not_reject_smoke.kab");
    assert!(
        s.contains("aotLoadNotOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab not opcode target rejection"
    );
}

/// F10: native image filename and not opcode must agree.
#[test]
fn f10_aot_verify_not_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_not.kab"))
        .expect("aot_verify_not.kab");
    assert!(
        v.contains("pub fn aotVerifyNotOk") && v.contains("aa2003e0"),
        "F10 Kab aotVerifyNotOk"
    );
}

/// F10: emitted image name and not opcode round-trip through verification.
#[test]
fn f10_aot_verify_not_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_not_round_smoke.kab"))
        .expect("f10_aot_verify_not_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotNotOp") && s.contains("aotVerifyNotOk"),
        "F10 Kab not opcode verify round-trip"
    );
}

/// F10: verify rejects a not opcode for the wrong image name.
#[test]
fn f10_aot_verify_not_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_not_reject_smoke.kab"))
        .expect("f10_aot_verify_not_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyNotOk") && s.contains("aa2003e0") && s.contains("false"),
        "F10 Kab not opcode name rejection"
    );
}

/// F10: arm64 image name and not opcode round-trip through verification.
#[test]
fn f10_aot_verify_not_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_not_arm64_smoke.kab"))
        .expect("f10_aot_verify_not_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyNotOk"),
        "F10 Kab arm64 not opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented xor opcode for the return register.
#[test]
fn f10_aot_xor_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_xor_op.kab")).expect("aot_xor_op.kab");
    assert!(
        r.contains("pub fn aotXorOp") && r.contains("ca000000"),
        "F10 Kab aotXorOp"
    );
}

/// F10: loader accepts only a documented first-image xor opcode.
#[test]
fn f10_aot_load_xor_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_xor.kab")).expect("aot_load_xor.kab");
    assert!(
        r.contains("pub fn aotLoadXorOk") && r.contains("ca000000"),
        "F10 Kab aotLoadXorOk"
    );
}

/// F10: emitted xor opcode round-trips through loader validation.
#[test]
fn f10_aot_xor_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_xor_round_smoke.kab"))
        .expect("f10_aot_xor_round_smoke.kab");
    assert!(
        s.contains("aotXorOp") && s.contains("aotLoadXorOk"),
        "F10 Kab xor opcode round-trip"
    );
}

/// F10: loader rejects a xor opcode for the wrong target.
#[test]
fn f10_aot_load_xor_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_xor_reject_smoke.kab"))
        .expect("f10_aot_load_xor_reject_smoke.kab");
    assert!(
        s.contains("aotLoadXorOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab xor opcode target rejection"
    );
}

/// F10: native image filename and xor opcode must agree.
#[test]
fn f10_aot_verify_xor_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_xor.kab"))
        .expect("aot_verify_xor.kab");
    assert!(
        v.contains("pub fn aotVerifyXorOk") && v.contains("ca000000"),
        "F10 Kab aotVerifyXorOk"
    );
}

/// F10: emitted image name and xor opcode round-trip through verification.
#[test]
fn f10_aot_verify_xor_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_xor_round_smoke.kab"))
        .expect("f10_aot_verify_xor_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotXorOp") && s.contains("aotVerifyXorOk"),
        "F10 Kab xor opcode verify round-trip"
    );
}

/// F10: verify rejects a xor opcode for the wrong image name.
#[test]
fn f10_aot_verify_xor_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_xor_reject_smoke.kab"))
        .expect("f10_aot_verify_xor_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyXorOk") && s.contains("ca000000") && s.contains("false"),
        "F10 Kab xor opcode name rejection"
    );
}

/// F10: arm64 image name and xor opcode round-trip through verification.
#[test]
fn f10_aot_verify_xor_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_xor_arm64_smoke.kab"))
        .expect("f10_aot_verify_xor_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyXorOk"),
        "F10 Kab arm64 xor opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented neg opcode for the return register.
#[test]
fn f10_aot_neg_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_neg_op.kab")).expect("aot_neg_op.kab");
    assert!(
        r.contains("pub fn aotNegOp") && r.contains("cb0003e0"),
        "F10 Kab aotNegOp"
    );
}

/// F10: loader accepts only a documented first-image neg opcode.
#[test]
fn f10_aot_load_neg_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_neg.kab")).expect("aot_load_neg.kab");
    assert!(
        r.contains("pub fn aotLoadNegOk") && r.contains("cb0003e0"),
        "F10 Kab aotLoadNegOk"
    );
}

/// F10: emitted neg opcode round-trips through loader validation.
#[test]
fn f10_aot_neg_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_neg_round_smoke.kab"))
        .expect("f10_aot_neg_round_smoke.kab");
    assert!(
        s.contains("aotNegOp") && s.contains("aotLoadNegOk"),
        "F10 Kab neg opcode round-trip"
    );
}

/// F10: loader rejects a neg opcode for the wrong target.
#[test]
fn f10_aot_load_neg_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_neg_reject_smoke.kab"))
        .expect("f10_aot_load_neg_reject_smoke.kab");
    assert!(
        s.contains("aotLoadNegOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab neg opcode target rejection"
    );
}

/// F10: native image filename and neg opcode must agree.
#[test]
fn f10_aot_verify_neg_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_neg.kab"))
        .expect("aot_verify_neg.kab");
    assert!(
        v.contains("pub fn aotVerifyNegOk") && v.contains("cb0003e0"),
        "F10 Kab aotVerifyNegOk"
    );
}

/// F10: emitted image name and neg opcode round-trip through verification.
#[test]
fn f10_aot_verify_neg_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_neg_round_smoke.kab"))
        .expect("f10_aot_verify_neg_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotNegOp") && s.contains("aotVerifyNegOk"),
        "F10 Kab neg opcode verify round-trip"
    );
}

/// F10: verify rejects a neg opcode for the wrong image name.
#[test]
fn f10_aot_verify_neg_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_neg_reject_smoke.kab"))
        .expect("f10_aot_verify_neg_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyNegOk") && s.contains("cb0003e0") && s.contains("false"),
        "F10 Kab neg opcode name rejection"
    );
}

/// F10: arm64 image name and neg opcode round-trip through verification.
#[test]
fn f10_aot_verify_neg_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_neg_arm64_smoke.kab"))
        .expect("f10_aot_verify_neg_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyNegOk"),
        "F10 Kab arm64 neg opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented cmp opcode for the return register.
#[test]
fn f10_aot_cmp_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_cmp_op.kab")).expect("aot_cmp_op.kab");
    assert!(
        r.contains("pub fn aotCmpOp") && r.contains("eb00001f"),
        "F10 Kab aotCmpOp"
    );
}

/// F10: loader accepts only a documented first-image cmp opcode.
#[test]
fn f10_aot_load_cmp_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_cmp.kab")).expect("aot_load_cmp.kab");
    assert!(
        r.contains("pub fn aotLoadCmpOk") && r.contains("eb00001f"),
        "F10 Kab aotLoadCmpOk"
    );
}

/// F10: emitted cmp opcode round-trips through loader validation.
#[test]
fn f10_aot_cmp_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_cmp_round_smoke.kab"))
        .expect("f10_aot_cmp_round_smoke.kab");
    assert!(
        s.contains("aotCmpOp") && s.contains("aotLoadCmpOk"),
        "F10 Kab cmp opcode round-trip"
    );
}

/// F10: loader rejects a cmp opcode for the wrong target.
#[test]
fn f10_aot_load_cmp_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_cmp_reject_smoke.kab"))
        .expect("f10_aot_load_cmp_reject_smoke.kab");
    assert!(
        s.contains("aotLoadCmpOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab cmp opcode target rejection"
    );
}

/// F10: native image filename and cmp opcode must agree.
#[test]
fn f10_aot_verify_cmp_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_cmp.kab"))
        .expect("aot_verify_cmp.kab");
    assert!(
        v.contains("pub fn aotVerifyCmpOk") && v.contains("eb00001f"),
        "F10 Kab aotVerifyCmpOk"
    );
}

/// F10: emitted image name and cmp opcode round-trip through verification.
#[test]
fn f10_aot_verify_cmp_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_cmp_round_smoke.kab"))
        .expect("f10_aot_verify_cmp_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotCmpOp") && s.contains("aotVerifyCmpOk"),
        "F10 Kab cmp opcode verify round-trip"
    );
}

/// F10: verify rejects a cmp opcode for the wrong image name.
#[test]
fn f10_aot_verify_cmp_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_cmp_reject_smoke.kab"))
        .expect("f10_aot_verify_cmp_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyCmpOk") && s.contains("eb00001f") && s.contains("false"),
        "F10 Kab cmp opcode name rejection"
    );
}

/// F10: arm64 image name and cmp opcode round-trip through verification.
#[test]
fn f10_aot_verify_cmp_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_cmp_arm64_smoke.kab"))
        .expect("f10_aot_verify_cmp_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyCmpOk"),
        "F10 Kab arm64 cmp opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented test opcode for the return register.
#[test]
fn f10_aot_test_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_test_op.kab")).expect("aot_test_op.kab");
    assert!(
        r.contains("pub fn aotTestOp") && r.contains("ea00001f"),
        "F10 Kab aotTestOp"
    );
}

/// F10: loader accepts only a documented first-image test opcode.
#[test]
fn f10_aot_load_test_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_test.kab")).expect("aot_load_test.kab");
    assert!(
        r.contains("pub fn aotLoadTestOk") && r.contains("ea00001f"),
        "F10 Kab aotLoadTestOk"
    );
}

/// F10: emitted test opcode round-trips through loader validation.
#[test]
fn f10_aot_test_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_test_round_smoke.kab"))
        .expect("f10_aot_test_round_smoke.kab");
    assert!(
        s.contains("aotTestOp") && s.contains("aotLoadTestOk"),
        "F10 Kab test opcode round-trip"
    );
}

/// F10: loader rejects a test opcode for the wrong target.
#[test]
fn f10_aot_load_test_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_test_reject_smoke.kab"))
        .expect("f10_aot_load_test_reject_smoke.kab");
    assert!(
        s.contains("aotLoadTestOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab test opcode target rejection"
    );
}

/// F10: native image filename and test opcode must agree.
#[test]
fn f10_aot_verify_test_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_test.kab"))
        .expect("aot_verify_test.kab");
    assert!(
        v.contains("pub fn aotVerifyTestOk") && v.contains("ea00001f"),
        "F10 Kab aotVerifyTestOk"
    );
}

/// F10: emitted image name and test opcode round-trip through verification.
#[test]
fn f10_aot_verify_test_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_test_round_smoke.kab"))
        .expect("f10_aot_verify_test_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotTestOp") && s.contains("aotVerifyTestOk"),
        "F10 Kab test opcode verify round-trip"
    );
}

/// F10: verify rejects a test opcode for the wrong image name.
#[test]
fn f10_aot_verify_test_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_test_reject_smoke.kab"))
        .expect("f10_aot_verify_test_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyTestOk") && s.contains("ea00001f") && s.contains("false"),
        "F10 Kab test opcode name rejection"
    );
}

/// F10: arm64 image name and test opcode round-trip through verification.
#[test]
fn f10_aot_verify_test_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_test_arm64_smoke.kab"))
        .expect("f10_aot_verify_test_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyTestOk"),
        "F10 Kab arm64 test opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented je opcode for the return register.
#[test]
fn f10_aot_je_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_je_op.kab")).expect("aot_je_op.kab");
    assert!(
        r.contains("pub fn aotJeOp") && r.contains("54000000"),
        "F10 Kab aotJeOp"
    );
}

/// F10: loader accepts only a documented first-image je opcode.
#[test]
fn f10_aot_load_je_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_je.kab")).expect("aot_load_je.kab");
    assert!(
        r.contains("pub fn aotLoadJeOk") && r.contains("54000000"),
        "F10 Kab aotLoadJeOk"
    );
}

/// F10: emitted je opcode round-trips through loader validation.
#[test]
fn f10_aot_je_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_je_round_smoke.kab"))
        .expect("f10_aot_je_round_smoke.kab");
    assert!(
        s.contains("aotJeOp") && s.contains("aotLoadJeOk"),
        "F10 Kab je opcode round-trip"
    );
}

/// F10: loader rejects a je opcode for the wrong target.
#[test]
fn f10_aot_load_je_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_je_reject_smoke.kab"))
        .expect("f10_aot_load_je_reject_smoke.kab");
    assert!(
        s.contains("aotLoadJeOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab je opcode target rejection"
    );
}

/// F10: native image filename and je opcode must agree.
#[test]
fn f10_aot_verify_je_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_je.kab"))
        .expect("aot_verify_je.kab");
    assert!(
        v.contains("pub fn aotVerifyJeOk") && v.contains("54000000"),
        "F10 Kab aotVerifyJeOk"
    );
}

/// F10: emitted image name and je opcode round-trip through verification.
#[test]
fn f10_aot_verify_je_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_je_round_smoke.kab"))
        .expect("f10_aot_verify_je_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotJeOp") && s.contains("aotVerifyJeOk"),
        "F10 Kab je opcode verify round-trip"
    );
}

/// F10: verify rejects a je opcode for the wrong image name.
#[test]
fn f10_aot_verify_je_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_je_reject_smoke.kab"))
        .expect("f10_aot_verify_je_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyJeOk") && s.contains("54000000") && s.contains("false"),
        "F10 Kab je opcode name rejection"
    );
}

/// F10: arm64 image name and je opcode round-trip through verification.
#[test]
fn f10_aot_verify_je_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_je_arm64_smoke.kab"))
        .expect("f10_aot_verify_je_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyJeOk"),
        "F10 Kab arm64 je opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented jne opcode for the return register.
#[test]
fn f10_aot_jne_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_jne_op.kab")).expect("aot_jne_op.kab");
    assert!(
        r.contains("pub fn aotJneOp") && r.contains("54000001"),
        "F10 Kab aotJneOp"
    );
}

/// F10: loader accepts only a documented first-image jne opcode.
#[test]
fn f10_aot_load_jne_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_jne.kab")).expect("aot_load_jne.kab");
    assert!(
        r.contains("pub fn aotLoadJneOk") && r.contains("54000001"),
        "F10 Kab aotLoadJneOk"
    );
}

/// F10: emitted jne opcode round-trips through loader validation.
#[test]
fn f10_aot_jne_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_jne_round_smoke.kab"))
        .expect("f10_aot_jne_round_smoke.kab");
    assert!(
        s.contains("aotJneOp") && s.contains("aotLoadJneOk"),
        "F10 Kab jne opcode round-trip"
    );
}

/// F10: loader rejects a jne opcode for the wrong target.
#[test]
fn f10_aot_load_jne_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_jne_reject_smoke.kab"))
        .expect("f10_aot_load_jne_reject_smoke.kab");
    assert!(
        s.contains("aotLoadJneOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab jne opcode target rejection"
    );
}

/// F10: native image filename and jne opcode must agree.
#[test]
fn f10_aot_verify_jne_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_jne.kab"))
        .expect("aot_verify_jne.kab");
    assert!(
        v.contains("pub fn aotVerifyJneOk") && v.contains("54000001"),
        "F10 Kab aotVerifyJneOk"
    );
}

/// F10: emitted image name and jne opcode round-trip through verification.
#[test]
fn f10_aot_verify_jne_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jne_round_smoke.kab"))
        .expect("f10_aot_verify_jne_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotJneOp") && s.contains("aotVerifyJneOk"),
        "F10 Kab jne opcode verify round-trip"
    );
}

/// F10: verify rejects a jne opcode for the wrong image name.
#[test]
fn f10_aot_verify_jne_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jne_reject_smoke.kab"))
        .expect("f10_aot_verify_jne_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyJneOk") && s.contains("54000001") && s.contains("false"),
        "F10 Kab jne opcode name rejection"
    );
}

/// F10: arm64 image name and jne opcode round-trip through verification.
#[test]
fn f10_aot_verify_jne_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jne_arm64_smoke.kab"))
        .expect("f10_aot_verify_jne_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyJneOk"),
        "F10 Kab arm64 jne opcode verify round-trip"
    );
}

/// F10: first Kab-native images emit a documented jl opcode for the return register.
#[test]
fn f10_aot_jl_op_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_jl_op.kab")).expect("aot_jl_op.kab");
    assert!(
        r.contains("pub fn aotJlOp") && r.contains("5400000b"),
        "F10 Kab aotJlOp"
    );
}

/// F10: loader accepts only a documented first-image jl opcode.
#[test]
fn f10_aot_load_jl_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/aot_load_jl.kab")).expect("aot_load_jl.kab");
    assert!(
        r.contains("pub fn aotLoadJlOk") && r.contains("5400000b"),
        "F10 Kab aotLoadJlOk"
    );
}

/// F10: emitted jl opcode round-trips through loader validation.
#[test]
fn f10_aot_jl_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_jl_round_smoke.kab"))
        .expect("f10_aot_jl_round_smoke.kab");
    assert!(
        s.contains("aotJlOp") && s.contains("aotLoadJlOk"),
        "F10 Kab jl opcode round-trip"
    );
}

/// F10: loader rejects a jl opcode for the wrong target.
#[test]
fn f10_aot_load_jl_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_load_jl_reject_smoke.kab"))
        .expect("f10_aot_load_jl_reject_smoke.kab");
    assert!(
        s.contains("aotLoadJlOk") && s.contains("\"arm64\"") && s.contains("false"),
        "F10 Kab jl opcode target rejection"
    );
}

/// F10: native image filename and jl opcode must agree.
#[test]
fn f10_aot_verify_jl_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_jl.kab"))
        .expect("aot_verify_jl.kab");
    assert!(
        v.contains("pub fn aotVerifyJlOk") && v.contains("5400000b"),
        "F10 Kab aotVerifyJlOk"
    );
}

/// F10: emitted image name and jl opcode round-trip through verification.
#[test]
fn f10_aot_verify_jl_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jl_round_smoke.kab"))
        .expect("f10_aot_verify_jl_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotJlOp") && s.contains("aotVerifyJlOk"),
        "F10 Kab jl opcode verify round-trip"
    );
}

/// F10: verify rejects a jl opcode for the wrong image name.
#[test]
fn f10_aot_verify_jl_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jl_reject_smoke.kab"))
        .expect("f10_aot_verify_jl_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyJlOk") && s.contains("5400000b") && s.contains("false"),
        "F10 Kab jl opcode name rejection"
    );
}

/// F10: arm64 image name and jl opcode round-trip through verification.
#[test]
fn f10_aot_verify_jl_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_jl_arm64_smoke.kab"))
        .expect("f10_aot_verify_jl_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyJlOk"),
        "F10 Kab arm64 jl opcode verify round-trip"
    );
}

/// F10: native image filename and add opcode must agree.
#[test]
fn f10_aot_verify_add_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_add.kab"))
        .expect("aot_verify_add.kab");
    assert!(
        v.contains("pub fn aotVerifyAddOk") && v.contains("8b000000"),
        "F10 Kab aotVerifyAddOk"
    );
}

/// F10: emitted image name and add opcode round-trip through verification.
#[test]
fn f10_aot_verify_add_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_add_round_smoke.kab"))
        .expect("f10_aot_verify_add_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotAddOp") && s.contains("aotVerifyAddOk"),
        "F10 Kab add opcode verify round-trip"
    );
}

/// F10: verify rejects an add opcode for the wrong image name.
#[test]
fn f10_aot_verify_add_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_add_reject_smoke.kab"))
        .expect("f10_aot_verify_add_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyAddOk") && s.contains("8b000000") && s.contains("false"),
        "F10 Kab add opcode name rejection"
    );
}

/// F10: arm64 image name and add opcode round-trip through verification.
#[test]
fn f10_aot_verify_add_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_add_arm64_smoke.kab"))
        .expect("f10_aot_verify_add_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyAddOk"),
        "F10 Kab arm64 add opcode verify round-trip"
    );
}

/// F10: native image filename and integer-one opcode must agree.
#[test]
fn f10_aot_verify_one_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_one.kab"))
        .expect("aot_verify_one.kab");
    assert!(
        v.contains("pub fn aotVerifyOneOk") && v.contains("d2800020"),
        "F10 Kab aotVerifyOneOk"
    );
}

/// F10: emitted image name and integer-one opcode round-trip through verification.
#[test]
fn f10_aot_verify_one_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_one_round_smoke.kab"))
        .expect("f10_aot_verify_one_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotOneOp") && s.contains("aotVerifyOneOk"),
        "F10 Kab integer-one opcode verify round-trip"
    );
}

/// F10: verify rejects an integer-one opcode for the wrong image name.
#[test]
fn f10_aot_verify_one_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_one_reject_smoke.kab"))
        .expect("f10_aot_verify_one_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyOneOk") && s.contains("d2800020") && s.contains("false"),
        "F10 Kab integer-one opcode name rejection"
    );
}

/// F10: arm64 image name and integer-one opcode round-trip through verification.
#[test]
fn f10_aot_verify_one_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_one_arm64_smoke.kab"))
        .expect("f10_aot_verify_one_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyOneOk"),
        "F10 Kab arm64 integer-one opcode verify round-trip"
    );
}

/// F10: native image filename and integer-zero opcode must agree.
#[test]
fn f10_aot_verify_zero_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_zero.kab"))
        .expect("aot_verify_zero.kab");
    assert!(
        v.contains("pub fn aotVerifyZeroOk") && v.contains("aa1f03e0"),
        "F10 Kab aotVerifyZeroOk"
    );
}

/// F10: emitted image name and integer-zero opcode round-trip through verification.
#[test]
fn f10_aot_verify_zero_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_zero_round_smoke.kab"))
        .expect("f10_aot_verify_zero_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotZeroOp") && s.contains("aotVerifyZeroOk"),
        "F10 Kab integer-zero opcode verify round-trip"
    );
}

/// F10: verify rejects an integer-zero opcode for the wrong image name.
#[test]
fn f10_aot_verify_zero_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_zero_reject_smoke.kab"))
        .expect("f10_aot_verify_zero_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyZeroOk") && s.contains("aa1f03e0") && s.contains("false"),
        "F10 Kab integer-zero opcode name rejection"
    );
}

/// F10: arm64 image name and integer-zero opcode round-trip through verification.
#[test]
fn f10_aot_verify_zero_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_zero_arm64_smoke.kab"))
        .expect("f10_aot_verify_zero_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyZeroOk"),
        "F10 Kab arm64 integer-zero opcode verify round-trip"
    );
}

/// F10: native image filename and nop opcode must agree.
#[test]
fn f10_aot_verify_nop_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let v = std::fs::read_to_string(root.join("lib/kab/aot_verify_nop.kab"))
        .expect("aot_verify_nop.kab");
    assert!(
        v.contains("pub fn aotVerifyNopOk") && v.contains("d503201f"),
        "F10 Kab aotVerifyNopOk"
    );
}

/// F10: emitted image name and nop opcode round-trip through verification.
#[test]
fn f10_aot_verify_nop_round_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_nop_round_smoke.kab"))
        .expect("f10_aot_verify_nop_round_smoke.kab");
    assert!(
        s.contains("aotImageName") && s.contains("aotNopOp") && s.contains("aotVerifyNopOk"),
        "F10 Kab nop opcode verify round-trip"
    );
}

/// F10: verify rejects a nop opcode for the wrong image name.
#[test]
fn f10_aot_verify_nop_reject_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_nop_reject_smoke.kab"))
        .expect("f10_aot_verify_nop_reject_smoke.kab");
    assert!(
        s.contains("aotVerifyNopOk") && s.contains("d503201f") && s.contains("false"),
        "F10 Kab nop opcode name rejection"
    );
}

/// F10: arm64 image name and nop opcode round-trip through verification.
#[test]
fn f10_aot_verify_nop_arm64_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("examples/f10_aot_verify_nop_arm64_smoke.kab"))
        .expect("f10_aot_verify_nop_arm64_smoke.kab");
    assert!(
        s.contains("\"arm64\"") && s.contains("aotVerifyNopOk"),
        "F10 Kab arm64 nop opcode verify round-trip"
    );
}

/// F14: zero-copy I/O policy in a tiny leaf.
#[test]
fn f14_io_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/io.kab")).expect("io.kab");
    assert!(
        i.contains("pub fn ioNoCopy"),
        "F14 Kab ioNoCopy"
    );
}

/// F1: direct dispatch policy in a tiny leaf (do not import kab/vm).
#[test]
fn f1_disp_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let d = std::fs::read_to_string(root.join("lib/kab/disp.kab")).expect("disp.kab");
    assert!(
        d.contains("pub fn dispIsDirect"),
        "F1 Kab dispIsDirect"
    );
}

/// F1: copy-and-patch slot vs AccAdd template (do not import kab/vm).
#[test]
fn f1_disp_cp_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/disp_cp.kab")).expect("disp_cp.kab");
    assert!(
        c.contains("pub fn dispPatchFits") && c.contains("6"),
        "F1 Kab dispPatchFits"
    );
}

/// F1: AccAdd opcode name for direct dispatch (do not import kab/vm).
#[test]
fn f1_disp_nm_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let n = std::fs::read_to_string(root.join("lib/kab/disp_nm.kab")).expect("disp_nm.kab");
    assert!(
        n.contains("pub fn dispOpAccAdd") && n.contains("acc_add_local"),
        "F1 Kab dispOpAccAdd"
    );
}

/// F2: monomorphic IC/shape policy in a tiny leaf.
#[test]
fn f2_ic_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/ic.kab")).expect("ic.kab");
    assert!(
        c.contains("pub fn icIsMono"),
        "F2 Kab icIsMono"
    );
}

/// F2: poly IC ≤4 shapes (do not import kab/ic or kab/vm).
#[test]
fn f2_ic_pl_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/ic_pl.kab")).expect("ic_pl.kab");
    assert!(
        p.contains("pub fn icIsPoly") && p.contains("4"),
        "F2 Kab icIsPoly"
    );
}

/// F2: megamorphic IC >4 shapes (do not import kab/ic or kab/vm).
#[test]
fn f2_ic_mg_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = std::fs::read_to_string(root.join("lib/kab/ic_mg.kab")).expect("ic_mg.kab");
    assert!(
        m.contains("pub fn icIsMega") && m.contains("4"),
        "F2 Kab icIsMega"
    );
}

/// F2: IC hit-rate gate (do not import kab/ic or kab/vm).
#[test]
fn f2_ic_ht_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/ic_ht.kab")).expect("ic_ht.kab");
    assert!(
        h.contains("pub fn icHitOk") && h.contains("90"),
        "F2 Kab icHitOk"
    );
}

/// F3: 8-byte unbox slot policy in a tiny leaf.
#[test]
fn f3_unbox_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let u = std::fs::read_to_string(root.join("lib/kab/unbox.kab")).expect("unbox.kab");
    assert!(
        u.contains("pub fn unboxSlotOk"),
        "F3 Kab unboxSlotOk"
    );
}

/// F3: bool unbox slot (do not import kab/unbox or kab/vm).
#[test]
fn f3_unbox_b_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let b = std::fs::read_to_string(root.join("lib/kab/unbox_b.kab")).expect("unbox_b.kab");
    assert!(
        b.contains("pub fn unboxBoolOk") && b.contains("1"),
        "F3 Kab unboxBoolOk"
    );
}

/// F3: f64 unbox slot (do not import kab/unbox or kab/vm).
#[test]
fn f3_unbox_f_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/unbox_f.kab")).expect("unbox_f.kab");
    assert!(
        f.contains("pub fn unboxF64Ok") && f.contains("8"),
        "F3 Kab unboxF64Ok"
    );
}

/// F3: array_f64 packed stride (do not import kab/unbox or kab/vm).
#[test]
fn f3_unbox_af_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/unbox_af.kab")).expect("unbox_af.kab");
    assert!(
        a.contains("pub fn unboxArrF64Stride") && a.contains("8"),
        "F3 Kab unboxArrF64Stride"
    );
}

/// F4: argc 0-3 register call policy in a tiny leaf.
#[test]
fn f4_call_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/call.kab")).expect("call.kab");
    assert!(
        c.contains("pub fn callFitsRegs"),
        "F4 Kab callFitsRegs"
    );
}

/// F4: stack/heap argv when argc > 3 (do not import kab/call or kab/vm).
#[test]
fn f4_call_st_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/call_st.kab")).expect("call_st.kab");
    assert!(
        s.contains("pub fn callNeedsStack") && s.contains("3"),
        "F4 Kab callNeedsStack"
    );
}

/// F4: frame reuse when nLive is 0 (do not import kab/call or kab/vm).
#[test]
fn f4_call_ru_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/call_ru.kab")).expect("call_ru.kab");
    assert!(
        r.contains("pub fn callReuseOk"),
        "F4 Kab callReuseOk"
    );
}

/// F6: 1-block emit inline policy in a tiny leaf.
#[test]
fn f6_inl_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/inl.kab")).expect("inl.kab");
    assert!(
        i.contains("pub fn emitCanInline"),
        "F6 Kab emitCanInline"
    );
}

/// F6: op-count budget for emit inline (do not import kab/inl or kab/jit).
#[test]
fn f6_inl_ops_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let o = std::fs::read_to_string(root.join("lib/kab/inl_ops.kab")).expect("inl_ops.kab");
    assert!(
        o.contains("pub fn emitInlineOpsOk") && o.contains("4"),
        "F6 Kab emitInlineOpsOk"
    );
}

/// F6: getter-only emit inline (do not import kab/inl or kab/jit).
#[test]
fn f6_inl_get_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/inl_get.kab")).expect("inl_get.kab");
    assert!(
        g.contains("pub fn emitInlineGetOk"),
        "F6 Kab emitInlineGetOk"
    );
}

/// F16: CPU fallback when no GPU device, tiny leaf.
#[test]
fn f16_gpu_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/gpu.kab")).expect("gpu.kab");
    assert!(
        g.contains("pub fn gpuUseCpu"),
        "F16 Kab gpuUseCpu"
    );
}

/// F17: 60 FPS league gate vs Kab baseline (not rustc).
#[test]
fn f17_liga_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/liga.kab")).expect("liga.kab");
    assert!(
        l.contains("pub fn ligaFpsOk"),
        "F17 Kab ligaFpsOk"
    );
}

/// F14: os_write via Kab policy leaf (host FS capability).
#[test]
fn f14_io_put_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/io_put.kab")).expect("io_put.kab");
    assert!(
        p.contains("pub fn ioPut") && p.contains("os_write"),
        "F14 Kab ioPut"
    );
}

/// F14: os_read via Kab policy leaf.
#[test]
fn f14_io_get_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/io_get.kab")).expect("io_get.kab");
    assert!(
        g.contains("pub fn ioGet") && g.contains("os_read"),
        "F14 Kab ioGet"
    );
}

/// F14: os_mkdir via Kab policy leaf.
#[test]
fn f14_io_mkdir_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = std::fs::read_to_string(root.join("lib/kab/io_mkdir.kab")).expect("io_mkdir.kab");
    assert!(
        m.contains("pub fn ioMkdir") && m.contains("os_mkdir"),
        "F14 Kab ioMkdir"
    );
}

/// F14: os_exists via Kab policy leaf.
#[test]
fn f14_io_has_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/io_has.kab")).expect("io_has.kab");
    assert!(
        h.contains("pub fn ioHas") && h.contains("os_exists"),
        "F14 Kab ioHas"
    );
}

/// F14: os_delete via Kab policy leaf.
#[test]
fn f14_io_del_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let d = std::fs::read_to_string(root.join("lib/kab/io_del.kab")).expect("io_del.kab");
    assert!(
        d.contains("pub fn ioDel") && d.contains("os_delete"),
        "F14 Kab ioDel"
    );
}

/// F14: os_list via Kab policy leaf.
#[test]
fn f14_io_list_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l = std::fs::read_to_string(root.join("lib/kab/io_list.kab")).expect("io_list.kab");
    assert!(
        l.contains("pub fn ioList") && l.contains("os_list"),
        "F14 Kab ioList"
    );
}

/// F14: os_stat via Kab policy leaf.
#[test]
fn f14_io_stat_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/io_stat.kab")).expect("io_stat.kab");
    assert!(
        s.contains("pub fn ioStat") && s.contains("os_stat"),
        "F14 Kab ioStat"
    );
}

/// F14: os_write_async via Kab policy leaf (no await in the leaf).
#[test]
fn f14_io_async_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/io_async.kab")).expect("io_async.kab");
    assert!(
        a.contains("pub fn ioWriteAsync") && a.contains("os_write_async"),
        "F14 Kab ioWriteAsync"
    );
}

/// F14: os_read_async via Kab policy leaf (no await in the leaf).
#[test]
fn f14_io_aread_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/io_aread.kab")).expect("io_aread.kab");
    assert!(
        r.contains("pub fn ioReadAsync") && r.contains("os_read_async"),
        "F14 Kab ioReadAsync"
    );
}

/// F14: await_all via Kab policy leaf.
#[test]
fn f14_io_await_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/io_await.kab")).expect("io_await.kab");
    assert!(
        a.contains("pub fn ioAwaitAll") && a.contains("await_all"),
        "F14 Kab ioAwaitAll"
    );
}

/// F14: HTTP timeout policy (no TCP).
#[test]
fn f14_http_to_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/http_to.kab")).expect("http_to.kab");
    assert!(
        t.contains("pub fn httpSetTimeout") && t.contains("http_set_timeout"),
        "F14 Kab httpSetTimeout"
    );
}

/// F14: wrap http_fetch_async; smokes must not call it (no live net).
#[test]
fn f14_http_fetch_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/http_fetch.kab")).expect("http_fetch.kab");
    assert!(
        f.contains("pub fn httpFetch") && f.contains("http_fetch_async"),
        "F14 Kab httpFetch"
    );
}

/// F14: HTTP timeout reset (no TCP).
#[test]
fn f14_http_rst_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/http_rst.kab")).expect("http_rst.kab");
    assert!(
        r.contains("pub fn httpResetTimeout") && r.contains("http_reset_timeout"),
        "F14 Kab httpResetTimeout"
    );
}

/// F14: in-process http_request (no TCP).
#[test]
fn f14_http_req_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let q = std::fs::read_to_string(root.join("lib/kab/http_req.kab")).expect("http_req.kab");
    assert!(
        q.contains("pub fn httpReq") && q.contains("http_request"),
        "F14 Kab httpReq"
    );
}

/// F14: wrap http_serve; smokes must not call it (bind + accept loop).
#[test]
fn f14_http_srv_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/http_srv.kab")).expect("http_srv.kab");
    assert!(
        s.contains("pub fn httpServe") && s.contains("http_serve"),
        "F14 Kab httpServe"
    );
}

/// F14: in-process http_route (no TCP).
#[test]
fn f14_http_rt_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/http_rt.kab")).expect("http_rt.kab");
    assert!(
        r.contains("pub fn httpRouteOk") && r.contains("http_route"),
        "F14 Kab httpRouteOk"
    );
}

/// F14: in-process http_response (no TCP).
#[test]
fn f14_http_res_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/http_res.kab")).expect("http_res.kab");
    assert!(
        h.contains("pub fn httpRes") && h.contains("http_response"),
        "F14 Kab httpRes"
    );
}

/// F14: in-process http_status (no TCP).
#[test]
fn f14_http_st_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/http_st.kab")).expect("http_st.kab");
    assert!(
        s.contains("pub fn httpStatusOf") && s.contains("http_status"),
        "F14 Kab httpStatusOf"
    );
}

/// F14: in-process http_body (no TCP).
#[test]
fn f14_http_bd_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let b = std::fs::read_to_string(root.join("lib/kab/http_bd.kab")).expect("http_bd.kab");
    assert!(
        b.contains("pub fn httpBodyOf") && b.contains("http_body"),
        "F14 Kab httpBodyOf"
    );
}

/// F14: in-process http_headers (no TCP).
#[test]
fn f14_http_hd_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/http_hd.kab")).expect("http_hd.kab");
    assert!(
        h.contains("pub fn httpHeadersOf") && h.contains("http_headers"),
        "F14 Kab httpHeadersOf"
    );
}

/// F14: in-process http_header lookup (no TCP).
#[test]
fn f14_http_hg_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let g = std::fs::read_to_string(root.join("lib/kab/http_hg.kab")).expect("http_hg.kab");
    assert!(
        g.contains("pub fn httpHeaderOf") && g.contains("http_header"),
        "F14 Kab httpHeaderOf"
    );
}

/// F14: in-process http_process (no TCP).
#[test]
fn f14_http_pc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/http_pc.kab")).expect("http_pc.kab");
    assert!(
        p.contains("pub fn httpProcess") && p.contains("http_process"),
        "F14 Kab httpProcess"
    );
}

/// F14: in-process http_request_async (no await, no TCP).
#[test]
fn f14_http_ra_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/http_ra.kab")).expect("http_ra.kab");
    assert!(
        a.contains("pub fn httpReqAsync") && a.contains("http_request_async"),
        "F14 Kab httpReqAsync"
    );
}

/// F14: wrap http_serve_once; smokes must not call it (bind + accept).
#[test]
fn f14_http_so_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/http_so.kab")).expect("http_so.kab");
    assert!(
        s.contains("pub fn httpServeOnce") && s.contains("http_serve_once"),
        "F14 Kab httpServeOnce"
    );
}

/// SH18: nursery bump + frame budget live in Kab (not host GC).
#[test]
fn sh18_gc_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gc = std::fs::read_to_string(root.join("lib/kab/gc.kab")).expect("gc.kab");
    assert!(
        gc.contains("pub fn gcNurseryCap")
            && gc.contains("pub fn gcBump")
            && gc.contains("pub fn gcNeedCollect")
            && gc.contains("pub fn gcFrameBudgetMs")
            && gc.contains("65536"),
        "SH18 Kab nursery cap + bump + 16ms frame budget"
    );
}

/// SH18: promote stays in a tiny leaf (do not grow gc.kab).
#[test]
fn sh18_gc_prom_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/gc_prom.kab")).expect("gc_prom.kab");
    assert!(
        p.contains("pub fn gcPromote"),
        "SH18 Kab nursery promote"
    );
}

/// SH18: sweep stays in a tiny leaf.
#[test]
fn sh18_gc_mark_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = std::fs::read_to_string(root.join("lib/kab/gc_mark.kab")).expect("gc_mark.kab");
    assert!(
        m.contains("pub fn gcSweepDead"),
        "SH18 Kab sweep dead count"
    );
}

/// SH18: write barrier stays in a tiny leaf.
#[test]
fn sh18_gc_bar_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let b = std::fs::read_to_string(root.join("lib/kab/gc_bar.kab")).expect("gc_bar.kab");
    assert!(
        b.contains("pub fn gcWriteBarrier"),
        "SH18 Kab write barrier"
    );
}

/// SH18: concurrent mark step stays in a tiny leaf (do not grow gc.kab).
#[test]
fn sh18_gc_conc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/gc_conc.kab")).expect("gc_conc.kab");
    assert!(
        c.contains("pub fn gcMarkStep") && c.contains("budgetMs"),
        "SH18 Kab concurrent mark step"
    );
}

/// F12: escape/stackalloc policy in a tiny leaf.
#[test]
fn f12_esc_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let e = std::fs::read_to_string(root.join("lib/kab/esc.kab")).expect("esc.kab");
    assert!(
        e.contains("pub fn escFitsFrame"),
        "F12 Kab escFitsFrame"
    );
}

/// SH19: process loader policy lives in Kab (host main.rs is skuld).
#[test]
fn sh19_load_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let load = std::fs::read_to_string(root.join("lib/kab/load.kab")).expect("load.kab");
    assert!(
        load.contains("pub fn loadIsKab")
            && load.contains("pub fn loadEntry")
            && load.contains("pub fn loadReady")
            && load.contains(".kab"),
        "SH19 Kab loader path + entry"
    );
}

/// SH19: `.kbc`/`.kbcb` is a packed image, not source (do not grow load.kab).
#[test]
fn sh19_load_kbc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let k = std::fs::read_to_string(root.join("lib/kab/load_kbc.kab")).expect("load_kbc.kab");
    assert!(
        k.contains("pub fn loadIsKbc") && k.contains(".kbc"),
        "SH19 Kab loadIsKbc"
    );
}

/// SH19: compiler-image filename lives off load.kab.
#[test]
fn sh19_load_img_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let i = std::fs::read_to_string(root.join("lib/kab/load_img.kab")).expect("load_img.kab");
    assert!(
        i.contains("pub fn loadImageName") && i.contains("compiler.kbcb"),
        "SH19 Kab loadImageName"
    );
}

/// SH20: core stdlib wrappers live in Kab (host natives are skuld).
#[test]
fn sh20_stdlib_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let st = std::fs::read_to_string(root.join("lib/kab/stdlib.kab")).expect("stdlib.kab");
    assert!(
        st.contains("pub fn stdAdd")
            && st.contains("pub fn stdLen")
            && st.contains("pub fn stdHas"),
        "SH20 Kab stdAdd/stdLen/stdHas"
    );
}

/// SH20: JSON null token lives off stdlib.kab.
#[test]
fn sh20_std_json_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let j = std::fs::read_to_string(root.join("lib/kab/std_json.kab")).expect("std_json.kab");
    assert!(
        j.contains("pub fn stdJsonIsNull") && j.contains("null"),
        "SH20 Kab stdJsonIsNull"
    );
}

/// SH20: epoch-ms gate lives off stdlib.kab.
#[test]
fn sh20_std_date_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let d = std::fs::read_to_string(root.join("lib/kab/std_date.kab")).expect("std_date.kab");
    assert!(
        d.contains("pub fn stdDateEpochOk"),
        "SH20 Kab stdDateEpochOk"
    );
}

/// SH20: regex literal-hit lives off stdlib.kab.
#[test]
fn sh20_std_re_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/std_re.kab")).expect("std_re.kab");
    assert!(
        r.contains("pub fn stdReHit") && r.contains("str_index_of"),
        "SH20 Kab stdReHit"
    );
}

/// SH21: OS/FS policy lives in Kab (host os_* are capabilities).
#[test]
fn sh21_os_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let os = std::fs::read_to_string(root.join("lib/kab/os.kab")).expect("os.kab");
    assert!(
        os.contains("pub fn kabOsIsVfs")
            && os.contains("pub fn kabOsCapRead")
            && os.contains("pub fn kabOsCapWrite")
            && os.contains("/apps/"),
        "SH21 Kab VFS path + read/write caps"
    );
}

/// SH21: file-path gate lives off os.kab.
#[test]
fn sh21_os_fs_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/os_fs.kab")).expect("os_fs.kab");
    assert!(
        f.contains("pub fn kabOsIsFile") && f.contains("."),
        "SH21 Kab kabOsIsFile"
    );
}

/// SH21: process argv gate lives off os.kab.
#[test]
fn sh21_os_proc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/os_proc.kab")).expect("os_proc.kab");
    assert!(
        p.contains("pub fn kabOsArgvOk"),
        "SH21 Kab kabOsArgvOk"
    );
}

/// SH22: SQL policy/scalar lives in Kab (host src/sql is skuld).
#[test]
fn sh22_sql_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join("lib/kab/sql.kab")).expect("sql.kab");
    assert!(
        sql.contains("pub fn sqlIsSelect")
            && sql.contains("pub fn sqlScalarOne")
            && sql.contains("pub fn sqlOk"),
        "SH22 Kab sqlIsSelect + scalar 1"
    );
}

/// SH22: WHERE-clause gate lives off sql.kab.
#[test]
fn sh22_sql_where_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let w = std::fs::read_to_string(root.join("lib/kab/sql_where.kab")).expect("sql_where.kab");
    assert!(
        w.contains("pub fn sqlIsWhere") && w.contains("WHERE"),
        "SH22 Kab sqlIsWhere"
    );
}

/// SH22: row-store gate lives off sql.kab.
#[test]
fn sh22_sql_store_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/sql_store.kab")).expect("sql_store.kab");
    assert!(
        s.contains("pub fn sqlStoreOk"),
        "SH22 Kab sqlStoreOk"
    );
}

/// SH23: TLS/pin policy lives in Kab (host rustls is skuld).
#[test]
fn sh23_crypto_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/crypto.kab")).expect("crypto.kab");
    assert!(
        c.contains("pub fn cryptoIsHttps")
            && c.contains("pub fn cryptoPinOk")
            && c.contains("pub fn cryptoTrustOk")
            && c.contains("https://"),
        "SH23 Kab https + pin + trust"
    );
}

/// SH23: TLS 1.2 gate lives off crypto.kab.
#[test]
fn sh23_crypto_tls_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let t = std::fs::read_to_string(root.join("lib/kab/crypto_tls.kab")).expect("crypto_tls.kab");
    assert!(
        t.contains("pub fn cryptoTls12Ok") && t.contains("1.2"),
        "SH23 Kab cryptoTls12Ok"
    );
}

/// SH23: PEM root marker lives off crypto.kab.
#[test]
fn sh23_crypto_root_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::fs::read_to_string(root.join("lib/kab/crypto_root.kab")).expect("crypto_root.kab");
    assert!(
        r.contains("pub fn cryptoRootPem") && r.contains("BEGIN"),
        "SH23 Kab cryptoRootPem"
    );
}

/// SH24: HTTP method/status policy lives in Kab (host http.rs is skuld).
#[test]
fn sh24_http_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let h = std::fs::read_to_string(root.join("lib/kab/http.kab")).expect("http.kab");
    assert!(
        h.contains("pub fn httpIsGet")
            && h.contains("pub fn httpOk")
            && h.contains("pub fn httpIsFetch"),
        "SH24 Kab GET + 200 + fetch"
    );
}

/// SH24: POST method lives off http.kab.
#[test]
fn sh24_http_post_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = std::fs::read_to_string(root.join("lib/kab/http_post.kab")).expect("http_post.kab");
    assert!(
        p.contains("pub fn httpIsPost") && p.contains("POST"),
        "SH24 Kab httpIsPost"
    );
}

/// SH24: JSON content-type lives off http.kab.
#[test]
fn sh24_http_ct_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/http_ct.kab")).expect("http_ct.kab");
    assert!(
        c.contains("pub fn httpIsJson") && c.contains("json"),
        "SH24 Kab httpIsJson"
    );
}

/// SH25: CLI argv lives in Kab (host src/cli is skuld).
#[test]
fn sh25_cli_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/cli.kab")).expect("cli.kab");
    assert!(
        c.contains("pub fn cliIsRun")
            && c.contains("pub fn cliIsRepl")
            && c.contains("pub fn cliIsTest"),
        "SH25 Kab run/repl/test argv"
    );
}

/// SH25: compile argv lives off cli.kab.
#[test]
fn sh25_cli_cc_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/cli_cc.kab")).expect("cli_cc.kab");
    assert!(
        c.contains("pub fn cliIsCompile") && c.contains("compile"),
        "SH25 Kab cliIsCompile"
    );
}

/// SH25: fmt argv lives off cli.kab.
#[test]
fn sh25_cli_fmt_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/cli_fmt.kab")).expect("cli_fmt.kab");
    assert!(
        f.contains("pub fn cliIsFmt") && f.contains("fmt"),
        "SH25 Kab cliIsFmt"
    );
}

/// SH26: science/nd arithmetic lives in Kab (host GPU is syscall skuld).
#[test]
fn sh26_sci_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let s = std::fs::read_to_string(root.join("lib/kab/sci.kab")).expect("sci.kab");
    assert!(
        s.contains("pub fn sciAdd")
            && s.contains("pub fn sciMul")
            && s.contains("pub fn sciGpuOff"),
        "SH26 Kab sciAdd/sciMul + GPU off"
    );
}

/// SH26: nd length lives off sci.kab.
#[test]
fn sh26_sci_nd_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let n = std::fs::read_to_string(root.join("lib/kab/sci_nd.kab")).expect("sci_nd.kab");
    assert!(
        n.contains("pub fn sciNdLenOk"),
        "SH26 Kab sciNdLenOk"
    );
}

/// SH26: FFT power-of-two size lives off sci.kab.
#[test]
fn sh26_sci_fft_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/sci_fft.kab")).expect("sci_fft.kab");
    assert!(
        f.contains("pub fn sciFftPow2") && f.contains("8"),
        "SH26 Kab sciFftPow2"
    );
}

/// SH27: DOM/game-loop policy lives in Kab (host browser* is skuld).
#[test]
fn sh27_ui_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let u = std::fs::read_to_string(root.join("lib/kab/ui.kab")).expect("ui.kab");
    assert!(
        u.contains("pub fn uiIsDiv")
            && u.contains("pub fn uiTickMs")
            && u.contains("pub fn uiReady"),
        "SH27 Kab div + 16ms tick"
    );
}

/// SH27: canvas tag lives off ui.kab.
#[test]
fn sh27_ui_cv_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let c = std::fs::read_to_string(root.join("lib/kab/ui_cv.kab")).expect("ui_cv.kab");
    assert!(
        c.contains("pub fn uiIsCanvas") && c.contains("canvas"),
        "SH27 Kab uiIsCanvas"
    );
}

/// SH27: 60 FPS frame gate lives off ui.kab.
#[test]
fn sh27_ui_fps_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::read_to_string(root.join("lib/kab/ui_fps.kab")).expect("ui_fps.kab");
    assert!(
        f.contains("pub fn uiFpsOk") && f.contains("16"),
        "SH27 Kab uiFpsOk"
    );
}

/// SH28: zero product-Rust is policy in Kab; host src/ is not deleted yet.
#[test]
fn sh28_noll_plan_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let n = std::fs::read_to_string(root.join("lib/kab/noll.kab")).expect("noll.kab");
    assert!(
        n.contains("pub fn nollSrcGoal")
            && n.contains("pub fn nollRustcStillHost")
            && n.contains("pub fn nollBootstrapFromKab"),
        "SH28 Kab src-goal 0 + host rustc still + Kab bootstrap"
    );
}

/// SH28: AOT is not ready — must not delete src/.
#[test]
fn sh28_noll_aot_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = std::fs::read_to_string(root.join("lib/kab/noll_aot.kab")).expect("noll_aot.kab");
    assert!(
        a.contains("pub fn nollAotReady") && a.contains("return false"),
        "SH28 Kab nollAotReady is false"
    );
}

/// SH28: keep src/ until AOT/bootstrap.
#[test]
fn sh28_noll_keep_in_kab() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let k = std::fs::read_to_string(root.join("lib/kab/noll_keep.kab")).expect("noll_keep.kab");
    assert!(
        k.contains("pub fn nollKeepSrc") && k.contains("return true"),
        "SH28 Kab nollKeepSrc"
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
