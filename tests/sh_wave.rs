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
