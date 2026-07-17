//! S2 — `kabootar compile` prefers self_host/compile.kab for app sources.

use kabootar_lib::bytecode::{compile_source, run_module};
use kabootar_lib::cli::compile_file_report_with;
use kabootar_lib::compile::{compile_file_prefer, compile_source_self_host, CompilePrefer};
use kabootar_lib::evaluator::create_global_env;
use kabootar_lib::value::Value;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn tmp_kab(name: &str, source: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kab_s2_{name}_{nanos}.kab"));
    fs::write(&path, source).expect("write tmp kab");
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn self_host_compile_tiny_program_runs() {
    let source = "let x = 1\nreturn x + 2\n";
    let program = compile_source_self_host(source).expect("self-host compile");
    assert!(program.has_bytecode(), "expected bytecode from self-host");
    let mut env = create_global_env();
    let v = run_module(program.bytecode.as_ref().unwrap(), &mut env).expect("run");
    assert!(matches!(v, Value::Number(3)), "got {v:?}");
}

#[test]
fn self_host_and_rust_agree_on_tiny_program() {
    let source = "fn add(a, b) {\n    return a + b\n}\nreturn add(10, 32)\n";
    let sh = compile_source_self_host(source).expect("self-host");
    let rust = compile_source(source).expect("rust");
    let mut env_a = create_global_env();
    let mut env_b = create_global_env();
    let va = run_module(sh.bytecode.as_ref().unwrap(), &mut env_a).unwrap();
    let vb = run_module(rust.bytecode.as_ref().unwrap(), &mut env_b).unwrap();
    assert_eq!(format!("{va:?}"), format!("{vb:?}"));
}

#[test]
fn compile_prefer_self_host_labels_backend() {
    let path = tmp_kab("prefer", "return 7\n");
    let (program, backend) =
        compile_file_prefer(&path, CompilePrefer::SelfHostThenRust).expect("prefer");
    let _ = fs::remove_file(&path);
    assert!(program.has_bytecode());
    assert_eq!(backend, "self-host");
}

#[test]
fn compile_prefer_rust_force() {
    let path = tmp_kab("rust", "return 8\n");
    let (program, backend) = compile_file_prefer(&path, CompilePrefer::Rust).expect("rust");
    let _ = fs::remove_file(&path);
    assert!(program.has_bytecode());
    assert_eq!(backend, "rust");
}

#[test]
fn compile_file_report_with_self_host() {
    let path = tmp_kab("report", "return 9\n");
    let (n, bytecode, backend) =
        compile_file_report_with(&path, CompilePrefer::SelfHostOnly).expect("report");
    let _ = fs::remove_file(&path);
    assert!(n >= 1);
    assert!(bytecode);
    assert_eq!(backend, "self-host");
}

#[test]
fn self_host_path_falls_back_to_rust() {
    // Compiling the compiler via self-host is skipped → rust fallback.
    let (program, backend) =
        compile_file_prefer("self_host/compile.kab", CompilePrefer::SelfHostThenRust)
            .expect("fallback");
    assert!(program.has_bytecode() || program.stmt_count > 0 || backend == "rust");
    assert_eq!(backend, "rust");
}
