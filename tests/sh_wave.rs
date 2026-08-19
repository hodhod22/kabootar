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
        inv.kab_files >= 200,
        "self_host product .kab count, got {}",
        inv.kab_files
    );
    assert!(
        inv.vm_files >= 50,
        "vm_* shards still dominate until SH5/SH6, got {}",
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
