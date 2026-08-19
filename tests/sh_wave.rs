//! SH0 inventory, SH1 facade seeds, SH3a nested push(len).

use std::time::Instant;

use kabootar_lib::bytecode::run_module;
use kabootar_lib::compile::{
    collect_self_host_inventory, compile_source, read_seed_bytecode, write_compiler_facade_seeds,
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
        inv.compile_dag.len() >= 40,
        "compile.kab DAG should be dozens of shards, got {}",
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
    assert!(n >= 40, "compile DAG should be dozens of modules, wrote {n}");
}
