//! Extended kernel architecture API — Parts 1-7 + cross-cutting natives.

use super::{KernelSubsystems, OsHandle};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn get_os(env: &Environment) -> Result<OsHandle, String> {
    let os = env.get("os").ok_or("OS handle not available")?;
    let Value::OsHandle(handle) = os else {
        return Err("OS handle not available".into());
    };
    Ok(handle)
}

fn with_subsys<F, T>(os: &OsHandle, f: F) -> Result<T, String>
where
    F: FnOnce(&mut KernelSubsystems) -> Result<T, String>,
{
    let mut g = os
        .subsys
        .lock()
        .map_err(|_| "kernel subsystems lock poisoned".to_string())?;
    f(&mut g)
}

fn os_architecture_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Object(
            s.architecture_map()
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        ))
    })
}

fn os_kcore_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Object(
            s.kcore
                .info()
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        ))
    })
}

fn os_ipc_send_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let from = num_arg(args, 0, "os_ipc_send from")?;
    let to = num_arg(args, 1, "os_ipc_send to")?;
    let payload = bytes_arg(args, 2).unwrap_or_default();
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.kcore.ipc_send(from, to, payload)?;
        Ok(Value::Null)
    })
}

fn os_ipc_recv_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let ep = num_arg(args, 0, "os_ipc_recv endpoint")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| match s.kcore.ipc_recv(ep) {
        Some(m) => {
            let mut o = HashMap::new();
            o.insert("from".into(), Value::Number(m.from as i64));
            o.insert("to".into(), Value::Number(m.to as i64));
            o.insert(
                "payload".into(),
                Value::Array(m.payload.into_iter().map(|b| Value::Number(b as i64)).collect()),
            );
            Ok(Value::Object(o))
        }
        None => Ok(Value::Null),
    })
}

fn os_sched_tick_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| match s.kcore.tick() {
        Some(t) => {
            let mut o = HashMap::new();
            o.insert("tid".into(), Value::Number(t.tid as i64));
            o.insert("pid".into(), Value::Number(t.pid as i64));
            o.insert("name".into(), Value::String(t.name));
            o.insert("vruntime".into(), Value::Number(t.vruntime as i64));
            Ok(Value::Object(o))
        }
        None => Ok(Value::Null),
    })
}

fn os_sched_yield_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| match s.kcore.yield_running() {
        Some(t) => {
            let mut o = HashMap::new();
            o.insert("tid".into(), Value::Number(t.tid as i64));
            o.insert("pid".into(), Value::Number(t.pid as i64));
            o.insert("name".into(), Value::String(t.name));
            o.insert("vruntime".into(), Value::Number(t.vruntime as i64));
            Ok(Value::Object(o))
        }
        None => Ok(Value::Null),
    })
}

fn os_context_switch_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let from = num_arg(args, 0, "os_context_switch from")?;
    let to = num_arg(args, 1, "os_context_switch to")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let sw = s.kcore.context_switch(from, to);
        let mut o = HashMap::new();
        o.insert("from".into(), Value::Number(sw.from as i64));
        o.insert("to".into(), Value::Number(sw.to as i64));
        o.insert("sp".into(), Value::Number(sw.saved_sp as i64));
        o.insert("elapsed_ns".into(), Value::Number(sw.elapsed_ns as i64));
        Ok(Value::Object(o))
    })
}

fn os_mm_map_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_mm_map pid")?;
    let virt = num_arg(args, 1, "os_mm_map virt")?;
    let perms = args.get(2).and_then(|v| match v {
        Value::Number(n) => Some(*n as u8),
        _ => None,
    }).unwrap_or(7);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let phys = s.mm.vmm.alloc_phys();
        s.mm.map_page(pid, virt, phys, perms)?;
        Ok(Value::Number(phys as i64))
    })
}

fn os_mm_translate_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_mm_translate pid")?;
    let virt = num_arg(args, 1, "os_mm_translate virt")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.mm.translate(pid, virt)? as i64)))
}

fn os_mm_stats_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let (mapped, swapped, faults, alloc) = s.mm.stats();
        Ok(Value::Array(vec![
            Value::Number(mapped as i64),
            Value::Number(swapped as i64),
            Value::Number(faults as i64),
            Value::Number(alloc as i64),
        ]))
    })
}

fn os_mm_fault_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_mm_fault pid")?;
    let virt = num_arg(args, 1, "os_mm_fault virt")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.mm.fault(pid, virt)? as i64)))
}

fn os_mm_mmap_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_mm_mmap pid")?;
    let virt = num_arg(args, 1, "os_mm_mmap virt")?;
    let len = num_arg(args, 2, "os_mm_mmap len")?;
    let perms = args
        .get(3)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as u8),
            _ => None,
        })
        .unwrap_or(7);
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.mm.mmap(pid, virt, len, perms)? as i64)))
}

fn os_mm_cow_share_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let src = num_arg(args, 0, "os_mm_cow_share src")?;
    let dst = num_arg(args, 1, "os_mm_cow_share dst")?;
    let virt = num_arg(args, 2, "os_mm_cow_share virt")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.mm.cow_share(src, dst, virt)? as i64)))
}

fn os_mm_cow_break_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_mm_cow_break pid")?;
    let virt = num_arg(args, 1, "os_mm_cow_break virt")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.mm.cow_break(pid, virt)? as i64)))
}

fn os_thread_spawn_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_thread_spawn pid")?;
    let name = str_arg(args, 1).unwrap_or_else(|| "worker".into());
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.proc2.spawn_thread(pid, &name) as i64)))
}

fn os_signal_send_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = num_arg(args, 0, "os_signal_send pid")?;
    let sig = args.get(1).and_then(|v| match v {
        Value::Number(n) => super::proc2::Signal::from_num(*n as i32),
        _ => None,
    }).ok_or("os_signal_send expects signal number")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Bool(s.proc2.send_signal(pid, sig))))
}

fn os_job_create_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = str_arg(args, 0).unwrap_or_else(|| "job".into());
    let quota = args.get(1).and_then(|v| match v {
        Value::Number(n) => Some(*n as u32),
        _ => None,
    }).unwrap_or(100);
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.proc2.create_job(&name, quota) as i64)))
}

fn os_driver_register_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = str_arg(args, 0).ok_or("os_driver_register name")?;
    let ver = str_arg(args, 1).unwrap_or_else(|| "1.0".into());
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.iosys.register_driver(&name, &ver) as i64)))
}

fn os_driver_unregister_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = num_arg(args, 0, "os_driver_unregister id")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Bool(s.iosys.unregister_driver(id as u64))))
}

fn os_pnp_discover_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let bus = str_arg(args, 0).ok_or("os_pnp_discover bus")?;
    let vid = str_arg(args, 1).ok_or("os_pnp_discover vid")?;
    let pid = str_arg(args, 2).ok_or("os_pnp_discover pid")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::String(
            s.iosys.discover_device(&bus, &vid, &pid).unwrap_or_default(),
        ))
    })
}

fn os_irq_poll_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| match s.iosys.irq.poll() {
        Some(irq) => {
            let mut o = HashMap::new();
            o.insert("irq".into(), Value::Number(irq.irq as i64));
            o.insert("device".into(), Value::String(irq.device));
            Ok(Value::Object(o))
        }
        None => Ok(Value::Null),
    })
}

fn os_journal_append_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = str_arg(args, 0).ok_or("os_journal_append path")?;
    let data = str_arg(args, 1).unwrap_or_default();
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.fsys.journal.append(&path, &data)? as i64)))
}

fn os_journal_commit_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.fsys.journal.commit() as i64)))
}

fn os_journal_replay_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Array(
            s.fsys
                .journal
                .replay()
                .into_iter()
                .map(|e| {
                    let mut o = HashMap::new();
                    o.insert("seq".into(), Value::Number(e.seq as i64));
                    o.insert("path".into(), Value::String(e.path));
                    o.insert("bytes".into(), Value::Number(e.bytes as i64));
                    o.insert("payload".into(), Value::String(e.payload));
                    Value::Object(o)
                })
                .collect(),
        ))
    })
}

fn os_journal_checkpoint_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.fsys.journal.checkpoint() as i64)))
}

fn os_acl_grant_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let subject = str_arg(args, 0).ok_or("os_acl_grant(subject, object, right)")?;
    let object = str_arg(args, 1).ok_or("os_acl_grant(subject, object, right)")?;
    let right = str_arg(args, 2).ok_or("os_acl_grant(subject, object, right)")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.xcut.security.grant_acl(&subject, &object, &right);
        Ok(Value::Bool(true))
    })
}

fn os_acl_check_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let subject = str_arg(args, 0).ok_or("os_acl_check(subject, object, right)")?;
    let object = str_arg(args, 1).ok_or("os_acl_check(subject, object, right)")?;
    let right = str_arg(args, 2).ok_or("os_acl_check(subject, object, right)")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let ok = s.xcut.security.check_acl(&subject, &object, &right);
        s.xcut.security.audit(0, &format!("acl:{right}:{object}"), ok);
        Ok(Value::Bool(ok))
    })
}

fn os_acl_revoke_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let subject = str_arg(args, 0).ok_or("os_acl_revoke(subject, object)")?;
    let object = str_arg(args, 1).ok_or("os_acl_revoke(subject, object)")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Bool(s.xcut.security.revoke_acl(&subject, &object))))
}

fn os_netstack_send_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let proto = str_arg(args, 0).unwrap_or_else(|| "tcp".into());
    let payload = bytes_arg(args, 1).unwrap_or_default();
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let out = s.netstack.send_packet(&proto, &payload)?;
        Ok(Value::Array(out.into_iter().map(|b| Value::Number(b as i64)).collect()))
    })
}

fn os_shell_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let line = str_arg(args, 0).unwrap_or_default();
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::String(s.ring3.run_command(&line))))
}

fn os_libc_open_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = str_arg(args, 0).ok_or("os_libc_open path")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Number(s.ring3.libc.open(&path) as i64)))
}

fn os_log_drain_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let max = args.get(0).and_then(|v| match v {
        Value::Number(n) => Some(*n as usize),
        _ => None,
    }).unwrap_or(16);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Array(
            s.xcut
                .log
                .drain(max)
                .into_iter()
                .map(|e| {
                    let mut o = HashMap::new();
                    o.insert("level".into(), Value::String(e.level));
                    o.insert("message".into(), Value::String(e.message));
                    o.insert("ts".into(), Value::Number(e.timestamp as i64));
                    Value::Object(o)
                })
                .collect(),
        ))
    })
}

fn os_watchdog_ping_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.xcut.error.watchdog.ping();
        Ok(Value::Null)
    })
}

fn os_power_suspend_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.xcut.power.suspend();
        Ok(Value::String("sleep".into()))
    })
}

fn num_arg(args: &[Value], i: usize, name: &str) -> Result<u64, String> {
    match args.get(i) {
        Some(Value::Number(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err(format!("{name} expects number")),
    }
}

fn str_arg(args: &[Value], i: usize) -> Option<String> {
    match args.get(i) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn bytes_arg(args: &[Value], i: usize) -> Option<Vec<u8>> {
    match args.get(i) {
        Some(Value::String(s)) => Some(s.as_bytes().to_vec()),
        Some(Value::Array(vals)) => vals
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n as u8),
                _ => Err(()),
            })
            .collect::<Result<Vec<_>, _>>()
            .ok(),
        _ => None,
    }
}

pub fn register_architecture_globals(env: &mut Environment) {
    env.set("os_architecture".into(), Value::NativeFunction(os_architecture_native));
    env.set("os_kcore_info".into(), Value::NativeFunction(os_kcore_info_native));
    env.set("os_ipc_send".into(), Value::NativeFunction(os_ipc_send_native));
    env.set("os_ipc_recv".into(), Value::NativeFunction(os_ipc_recv_native));
    env.set("os_sched_tick".into(), Value::NativeFunction(os_sched_tick_native));
    env.set("os_sched_yield".into(), Value::NativeFunction(os_sched_yield_native));
    env.set("os_context_switch".into(), Value::NativeFunction(os_context_switch_native));
    env.set("os_mm_map".into(), Value::NativeFunction(os_mm_map_native));
    env.set("os_mm_translate".into(), Value::NativeFunction(os_mm_translate_native));
    env.set("os_mm_stats".into(), Value::NativeFunction(os_mm_stats_native));
    env.set("os_mm_fault".into(), Value::NativeFunction(os_mm_fault_native));
    env.set("os_mm_mmap".into(), Value::NativeFunction(os_mm_mmap_native));
    env.set("os_mm_cow_share".into(), Value::NativeFunction(os_mm_cow_share_native));
    env.set("os_mm_cow_break".into(), Value::NativeFunction(os_mm_cow_break_native));
    env.set("os_thread_spawn".into(), Value::NativeFunction(os_thread_spawn_native));
    env.set("os_signal_send".into(), Value::NativeFunction(os_signal_send_native));
    env.set("os_job_create".into(), Value::NativeFunction(os_job_create_native));
    env.set("os_driver_register".into(), Value::NativeFunction(os_driver_register_native));
    env.set("os_driver_unregister".into(), Value::NativeFunction(os_driver_unregister_native));
    env.set("os_pnp_discover".into(), Value::NativeFunction(os_pnp_discover_native));
    env.set("os_irq_poll".into(), Value::NativeFunction(os_irq_poll_native));
    env.set("os_journal_append".into(), Value::NativeFunction(os_journal_append_native));
    env.set("os_journal_commit".into(), Value::NativeFunction(os_journal_commit_native));
    env.set("os_journal_replay".into(), Value::NativeFunction(os_journal_replay_native));
    env.set("os_journal_checkpoint".into(), Value::NativeFunction(os_journal_checkpoint_native));
    env.set("os_acl_grant".into(), Value::NativeFunction(os_acl_grant_native));
    env.set("os_acl_check".into(), Value::NativeFunction(os_acl_check_native));
    env.set("os_acl_revoke".into(), Value::NativeFunction(os_acl_revoke_native));
    env.set("os_netstack_send".into(), Value::NativeFunction(os_netstack_send_native));
    env.set("os_shell".into(), Value::NativeFunction(os_shell_native));
    env.set("os_libc_open".into(), Value::NativeFunction(os_libc_open_native));
    env.set("os_log_drain".into(), Value::NativeFunction(os_log_drain_native));
    env.set("os_watchdog_ping".into(), Value::NativeFunction(os_watchdog_ping_native));
    env.set("os_power_suspend".into(), Value::NativeFunction(os_power_suspend_native));
}
