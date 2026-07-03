//! Full kernel architecture — Parts 1-7 + cross-cutting systems

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn architecture_map_exposes_all_parts() {
    let arch = eval("os_architecture()");
    let Value::Object(m) = arch else {
        panic!("expected object");
    };
    for key in [
        "part1_microkernel",
        "part2_vmm",
        "part3_threads",
        "part4_drivers",
        "part5_journal",
        "part6_stack",
        "part7_shell",
        "xcut_security",
        "xcut_log",
        "xcut_power",
    ] {
        assert!(m.contains_key(key), "missing {key}");
    }
}

#[test]
fn ring0_ipc_and_scheduler() {
    let msg = eval(
        r#"
        os_ipc_send(1, 1, "ping");
        os_ipc_recv(1);
        "#,
    );
    let Value::Object(o) = msg else {
        panic!("expected ipc message");
    };
    assert!(matches!(o.get("from"), Some(Value::Number(1))));
    let tick = eval("os_sched_tick()");
    assert!(matches!(tick, Value::Object(_)));
    let sw = eval("os_context_switch(1, 2)");
    let Value::Object(sw_o) = sw else {
        panic!("expected context switch");
    };
    assert!(matches!(sw_o.get("from"), Some(Value::Number(1))));
}

#[test]
fn mmu_map_translate_and_stats() {
    let out = eval(
        r#"
        let phys = os_mm_map(1, 4096, 7);
        let virt = os_mm_translate(1, 4096);
        let stats = os_mm_stats();
        [phys, virt, stats[0]];
        "#,
    );
    let Value::Array(a) = out else {
        panic!("expected array");
    };
    assert!(matches!(a[0], Value::Number(n) if n > 0));
    assert!(matches!(a[1], Value::Number(n) if n > 0));
    assert!(matches!(a[2], Value::Number(n) if n >= 1));
}

#[test]
fn process_threads_signals_jobs() {
    let tid = eval("os_thread_spawn(1, \"worker\")");
    assert!(matches!(tid, Value::Number(n) if n > 0));
    let sig = eval("os_signal_send(1, 15)");
    assert!(matches!(sig, Value::Bool(true)));
    let job = eval("os_job_create(\"batch\", 50)");
    assert!(matches!(job, Value::Number(n) if n > 0));
}

#[test]
fn io_driver_framework_pnp_irq() {
    let drv = eval("os_driver_register(\"test-net\", \"1.0\")");
    assert!(matches!(drv, Value::Number(n) if n > 0));
    let name = eval("os_pnp_discover(\"usb\", \"046d\", \"c52b\")");
    assert!(matches!(name, Value::String(s) if s == "hid-generic"));
}

#[test]
fn fs_journal_and_netstack() {
    let seq = eval(
        r#"
        os_journal_append("/data.txt", "payload");
        os_journal_commit();
        "#,
    );
    assert!(matches!(seq, Value::Number(n) if n >= 1));
    let pkt = eval("os_netstack_send(\"tcp\", \"hello\")");
    assert!(matches!(pkt, Value::Array(a) if !a.is_empty()));
}

#[test]
fn ring3_shell_libc_and_crosscut() {
    let echo = eval("os_shell(\"echo kabootar\")");
    assert!(matches!(echo, Value::String(s) if s == "kabootar"));
    let fd = eval("os_libc_open(\"/dev/null\")");
    assert!(matches!(fd, Value::Number(n) if n >= 0));
    let _ = eval("os_watchdog_ping()");
    let sleep = eval("os_power_suspend()");
    assert!(matches!(sleep, Value::String(s) if s == "sleep"));
    let logs = eval("os_log_drain(8)");
    assert!(matches!(logs, Value::Array(_)));
}
