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
fn d1_cfs_enqueue_tick_and_yield() {
    let out = eval(
        r#"
        let a = os_sched_enqueue("paint");
        let b = os_sched_enqueue("net");
        let t1 = os_sched_tick();
        let y = os_sched_yield();
        let t2 = os_sched_tick();
        is_number(a) && is_number(b) && a > 0 && b > 0
            && is_object(t1) && is_object(y) && is_object(t2)
            && is_string(t1.name) && is_number(y.vruntime)
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
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
fn d2_mmu_fault_mmap_and_cow() {
    let out = eval(
        r#"
        let before = os_mm_stats();
        let faulted = os_mm_fault(1, 131072);
        let mid = os_mm_stats();
        let base = os_mm_mmap(1, 196608, 8192, 7);
        let p1 = os_mm_translate(1, 196608);
        let p2 = os_mm_translate(1, 200704);
        let shared = os_mm_cow_share(1, 2, 196608);
        let broken = os_mm_cow_break(2, 196608);
        is_number(faulted) && mid[2] > before[2]
            && base == 196608 && p1 > 0 && p2 > 0 && p1 != p2
            && shared > 0 && broken > 0 && broken != shared
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
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
fn d3_journal_replay_checkpoint_and_acl() {
    let out = eval(
        r#"
        os_journal_append("/secure/a.txt", "hello");
        os_journal_append("/secure/b.txt", "world");
        let committed = os_journal_commit();
        let replay = os_journal_replay();
        let ck = os_journal_checkpoint();
        let after = os_journal_replay();
        os_acl_grant("uid:1", "/secure/secret", "read");
        let ok = os_acl_check("uid:1", "/secure/secret", "read");
        let deny = os_acl_check("uid:2", "/secure/secret", "read");
        os_acl_revoke("uid:1", "/secure/secret");
        let gone = os_acl_check("uid:1", "/secure/secret", "read");
        is_number(committed) && is_array(replay) && len(replay) >= 2
            && replay[0].payload == "hello" && ck >= 1
            && is_array(after) && len(after) == 0
            && ok == true && deny == false && gone == false
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
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
