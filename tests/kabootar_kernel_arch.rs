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
fn d1_irq_timer_preempt() {
    let out = eval(
        r#"
        os_sched_enqueue("ui");
        os_sched_enqueue("net");
        os_sched_tick();
        let before = os_kcore_info();
        let raised = os_irq_raise(0, "timer");
        let polled = os_irq_poll();
        let after = os_kcore_info();
        let forced = os_sched_preempt();
        is_object(raised) && raised.preempted == true && is_number(raised.tid)
            && is_object(polled) && polled.device == "timer" && polled.kind == "timer"
            && is_object(forced) && forced.forced == true
            && after.context_switches != before.context_switches
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
fn d2_os_mm_store_after_mmap() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 393216, 4096, 7);
        let tpl = [49, 192, 131, 192, 1, 195];
        let n = os_mm_store(1, 393216, tpl);
        base == 393216 && n == 6
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 393216, 4096, 7);
        let tpl = [49, 192, 131, 192, 1, 195];
        let n = os_mm_store(1, 393216, tpl);
        let r = os_mm_call(1, 393216);
        base == 393216 && n == 6 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop8_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 401408, 4096, 7);
        let tpl = [76, 8, 0, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 401408, tpl);
        let r = os_mm_call(1, 401408);
        base == 401408 && n == 8 && r == 8
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop16_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 409600, 4096, 7);
        let tpl = [76, 16, 0, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 409600, tpl);
        let r = os_mm_call(1, 409600);
        base == 409600 && n == 8 && r == 16
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_add_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 417792, 4096, 7);
        let tpl = [76, 16, 3, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 417792, tpl);
        let r = os_mm_call(1, 417792);
        base == 417792 && n == 8 && r == 19
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_sub_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 425984, 4096, 7);
        let tpl = [76, 16, 0, 3, 0, 0, 0, 195];
        let n = os_mm_store(1, 425984, tpl);
        let r = os_mm_call(1, 425984);
        base == 425984 && n == 8 && r == 13
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_mul_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 434176, 4096, 7);
        let tpl = [76, 16, 0, 0, 3, 0, 0, 195];
        let n = os_mm_store(1, 434176, tpl);
        let r = os_mm_call(1, 434176);
        base == 434176 && n == 8 && r == 48
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_div_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 442368, 4096, 7);
        let tpl = [76, 16, 0, 0, 0, 2, 0, 195];
        let n = os_mm_store(1, 442368, tpl);
        let r = os_mm_call(1, 442368);
        base == 442368 && n == 8 && r == 8
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_mod_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 450560, 4096, 7);
        let tpl = [76, 16, 0, 0, 0, 0, 5, 195];
        let n = os_mm_store(1, 450560, tpl);
        let r = os_mm_call(1, 450560);
        base == 450560 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_and_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 458752, 4096, 7);
        let tpl = [77, 12, 10, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 458752, tpl);
        let r = os_mm_call(1, 458752);
        base == 458752 && n == 8 && r == 8
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_or_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 466944, 4096, 7);
        let tpl = [78, 12, 10, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 466944, tpl);
        let r = os_mm_call(1, 466944);
        base == 466944 && n == 8 && r == 14
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_xor_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 475136, 4096, 7);
        let tpl = [79, 12, 10, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 475136, tpl);
        let r = os_mm_call(1, 475136);
        base == 475136 && n == 8 && r == 6
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_shl_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 483328, 4096, 7);
        let tpl = [80, 12, 1, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 483328, tpl);
        let r = os_mm_call(1, 483328);
        base == 483328 && n == 8 && r == 24
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_shr_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 491520, 4096, 7);
        let tpl = [81, 12, 1, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 491520, tpl);
        let r = os_mm_call(1, 491520);
        base == 491520 && n == 8 && r == 6
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_ushr_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 499712, 4096, 7);
        let tpl = [82, 12, 2, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 499712, tpl);
        let r = os_mm_call(1, 499712);
        base == 499712 && n == 8 && r == 3
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_not_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 507904, 4096, 7);
        let tpl = [83, 12, 0, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 507904, tpl);
        let r = os_mm_call(1, 507904);
        base == 507904 && n == 8 && r == ~12
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_neg_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 516096, 4096, 7);
        let tpl = [84, 12, 0, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 516096, tpl);
        let r = os_mm_call(1, 516096);
        base == 516096 && n == 8 && r == -12
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_eq_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 524288, 4096, 7);
        let tpl = [85, 12, 12, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 524288, tpl);
        let r = os_mm_call(1, 524288);
        base == 524288 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_ne_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 532480, 4096, 7);
        let tpl = [86, 12, 10, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 532480, tpl);
        let r = os_mm_call(1, 532480);
        base == 532480 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_lt_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 540672, 4096, 7);
        let tpl = [87, 10, 12, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 540672, tpl);
        let r = os_mm_call(1, 540672);
        base == 540672 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_gt_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 548864, 4096, 7);
        let tpl = [88, 12, 10, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 548864, tpl);
        let r = os_mm_call(1, 548864);
        base == 548864 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_le_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 557056, 4096, 7);
        let tpl = [89, 12, 12, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 557056, tpl);
        let r = os_mm_call(1, 557056);
        base == 557056 && n == 8 && r == 1
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}

#[test]
fn d2_os_mm_call_loop_ge_after_store() {
    let out = eval(
        r#"
        let base = os_mm_mmap(1, 565248, 4096, 7);
        let tpl = [90, 12, 12, 0, 0, 0, 0, 195];
        let n = os_mm_store(1, 565248, tpl);
        let r = os_mm_call(1, 565248);
        base == 565248 && n == 8 && r == 1
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
fn d4_netstack_nic_and_info() {
    let out = eval(
        r#"
        let n = os_hw_refresh();
        let ifaces = os_net_interfaces();
        let info = os_netstack_info();
        let host = os_host_info();
        let pkt = os_netstack_send("udp", "ping");
        is_number(n) && is_array(ifaces) && len(ifaces) >= 2
            && is_object(info) && is_string(info.backend) && is_string(info.packets)
            && is_object(host) && is_string(host.net_backend)
            && is_array(pkt) && len(pkt) > 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
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

#[test]
fn d5_monitors_vsync_acrylic() {
    let out = eval(
        r#"
        let mons = os_display_monitors();
        let vs = os_display_vsync("fifo");
        let win = os_window_create("Acrylic", 800, 600);
        os_display_register(win, "Acrylic", 800, 600);
        let layer = os_compositor_layer(win, 8, 0.8);
        let bytes = os_compositor_acrylic(layer);
        is_array(mons) && len(mons) >= 2 && mons[0].primary == true
            && vs == "fifo" && is_number(win) && is_number(layer) && layer > 0
            && is_number(bytes) && bytes > 0
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "got {out:?}");
}
