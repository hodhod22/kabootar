//! lib/os — Kabootar-language OS wrappers (VFS, mount, process, kernel).

use kabootar_lib::cli;
use kabootar_lib::value::Value;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

#[test]
fn os_vfs_read_write_roundtrip() {
    let code = r#"
import "os/vfs"
mkdir("/data")
write("/data/ping.txt", "pong")
read("/data/ping.txt") == "pong" && exists("/data/ping.txt")
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_vfs_stat_and_list() {
    let code = r#"
import "os/vfs"
mkdir("/box")
write("/box/a.txt", "A")
write("/box/b.txt", "B")
let st = stat("/box/a.txt")
st.kind == "file" && st.size == 1 && len(list("/box")) >= 2
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_process_spawn_and_list() {
    let code = r#"
import "os/process"
let pid = spawn("worker")
let procs = list()
pid > 0 && len(procs) >= 1
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_kernel_info_and_caps() {
    let code = r#"
import "os/kernel"
let k = info()
let c = caps()
len(k) > 3 && len(c) > 0
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_smoke_example_runs() {
    let path = format!("{}/examples/os_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/os_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n >= 10));
}

#[test]
fn os_async_read_roundtrip() {
    let code = r#"
import "os/vfs"
import "os/async"
mkdir("/adata")
write("/adata/x.txt", "hello")
async fn load() {
    return await readAsync("/adata/x.txt")
}
await load() == "hello"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_async_parallel_reads() {
    let code = r#"
import "os/vfs"
import "os/async"
mkdir("/p")
write("/p/a.txt", "A")
write("/p/b.txt", "B")
let xs = awaitAll([readPromise("/p/a.txt"), readPromise("/p/b.txt")])
xs[0] + xs[1] == "AB"
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_async_smoke_example_runs() {
    let path = format!("{}/examples/os_async_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/os_async_smoke.kab should run");
    assert!(matches!(result, Value::Number(n) if n == 2));
}

#[test]
fn os_sched_enqueue_and_tick() {
    let code = r#"
import "os/sched"
enqueue("paint")
enqueue("net")
let t = tick()
let y = schedYield()
t != null && y != null
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn os_k3_vfs_smoke_example_runs() {
    let path = format!("{}/examples/os_k3_vfs_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/os_k3_vfs_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)));
}

#[test]
fn os_h6d_policy_smoke() {
    let path = format!("{}/examples/h6d_os_policy_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("h6d-os".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            matches!(
                cli::run_file(&path).expect("examples/h6d_os_policy_smoke.kab should run"),
                Value::Bool(true)
            )
        })
        .expect("spawn h6d thread")
        .join()
        .expect("h6d thread join");
    assert!(ok);
}
