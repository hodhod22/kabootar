//! Competitive strategy API — 9 "secret sauce" natives.

use super::sauce::{CompatPlatform, UpdateChannel};
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

fn str_arg(args: &[Value], i: usize) -> Option<String> {
    match args.get(i) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn os_sauce_map_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Object(
            s.sauce
                .strategy_map()
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        ))
    })
}

fn os_ai_prefetch_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    let targets = with_subsys(&os, |s| Ok(s.sauce.ai.prefetch_targets().to_vec()))?;
    for app in &targets {
        let _ = os.sched_enqueue(app);
    }
    Ok(Value::Array(
        targets
            .into_iter()
            .map(|a| Value::String(a))
            .collect(),
    ))
}

fn os_ai_record_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let app = str_arg(args, 0).ok_or("os_ai_record app")?;
    let hour = args.get(1).and_then(|v| match v {
        Value::Number(n) if *n >= 0 && *n < 24 => Some(*n as u8),
        _ => None,
    }).unwrap_or(8);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.sauce.ai.record_launch(&app, hour);
        Ok(Value::Null)
    })
}

fn os_ai_context_menu_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let app = str_arg(args, 0).ok_or("os_ai_context_menu app")?;
    let items: Vec<String> = match args.get(1) {
        Some(Value::Array(vals)) => vals
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => vec!["file".into(), "edit".into(), "view".into(), "help".into()],
    };
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Array(
            s.sauce
                .ai
                .contextual_menu(&app, &items)
                .into_iter()
                .map(Value::String)
                .collect(),
        ))
    })
}

fn os_setup_nfc_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let token = str_arg(args, 0).ok_or("os_setup_nfc token")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let p = s.sauce.setup.nfc_bump(&token)?;
        let mut o = HashMap::new();
        o.insert("wifi".into(), Value::String(p.wifi_ssid));
        o.insert("lang".into(), Value::String(p.language));
        o.insert("tz".into(), Value::String(p.timezone));
        o.insert("dark".into(), Value::Bool(p.dark_theme));
        Ok(Value::Object(o))
    })
}

fn os_recovery_restore_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    Ok(Value::Number(os.golden_restore()? as i64))
}

fn os_seamless_pair_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let hz = args.get(0).and_then(|v| match v {
        Value::Number(n) if *n > 0 => Some(*n as u32),
        _ => None,
    }).unwrap_or(19_000);
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::String(s.sauce.seamless.pair_ultrasonic(hz))))
}

fn os_seamless_clipboard_push_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let text = str_arg(args, 0).unwrap_or_default();
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.sauce.seamless.clipboard_push(&text);
        Ok(Value::Null)
    })
}

fn os_seamless_clipboard_poll_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(match s.sauce.seamless.clipboard_poll() {
            Some(t) => Value::String(t),
            None => Value::Null,
        })
    })
}

fn os_energy_schedule_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let job = str_arg(args, 0).unwrap_or_else(|| "indexer".into());
    let wall_only = args.get(1).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(true);
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::Bool(s.sauce.energy.schedule(&job, wall_only))))
}

fn os_haptic_danger_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = str_arg(args, 0).unwrap_or_else(|| "/system".into());
    let important = path.starts_with("/system");
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let fb = s.sauce.haptic.danger_feedback(&path, important);
        let mut o = HashMap::new();
        o.insert("glow".into(), Value::String(fb.glow));
        o.insert("vibrate".into(), Value::Number(fb.vibrate as i64));
        o.insert("blocked".into(), Value::Bool(fb.blocked));
        Ok(Value::Object(o))
    })
}

fn os_compat_run_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let platform = str_arg(args, 0).ok_or("os_compat_run platform")?;
    let syscall = str_arg(args, 1).ok_or("os_compat_run syscall")?;
    let plat = CompatPlatform::parse(&platform)
        .ok_or("platform: android|windows|linux32")?;
    let mut nargs = Vec::new();
    if let Some(Value::Array(vals)) = args.get(2) {
        for v in vals {
            if let Value::Number(n) = v {
                nargs.push(*n);
            }
        }
    }
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let out = s.sauce.compat.translate(plat, &syscall, &nargs)?;
        Ok(Value::Object(
            out.into_iter()
                .map(|(k, v)| (k, Value::Number(v)))
                .collect(),
        ))
    })
}

fn os_privacy_panic_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Bool({
            s.sauce.privacy.engage_privacy_switch();
            s.sauce.privacy.ram_locked()
        }))
    })
}

fn os_privacy_telemetry_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let cat = str_arg(args, 0).unwrap_or_else(|| "usage".into());
    let count = args.get(1).and_then(|v| match v {
        Value::Number(n) => Some(*n),
        _ => None,
    }).unwrap_or(1);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        let evt = s.sauce.privacy.submit_telemetry(&cat, count);
        let mut o = HashMap::new();
        o.insert("category".into(), Value::String(evt.category));
        o.insert("noisy_count".into(), Value::Number(evt.noisy_count));
        Ok(Value::Object(o))
    })
}

fn os_update_channel_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let ch = str_arg(args, 0).ok_or("os_update_channel name")?;
    let channel = UpdateChannel::parse(&ch).ok_or("channel: beta|stable|classic")?;
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::String(s.sauce.updates.switch_channel(channel))))
}

fn os_update_rollback_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let steps = args.get(0).and_then(|v| match v {
        Value::Number(n) if *n >= 0 => Some(*n as usize),
        _ => None,
    }).unwrap_or(1);
    let os = get_os(env)?;
    with_subsys(&os, |s| Ok(Value::String(s.sauce.updates.rollback(steps))))
}

fn os_sauce_honesty_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items: Vec<Value> = crate::runtime::reality::sauce_strategy_honesty()
        .into_iter()
        .map(|(id, tier, note)| {
            let mut m = HashMap::new();
            m.insert("strategy".into(), Value::String(id));
            m.insert("tier".into(), Value::String(tier));
            m.insert("reality".into(), Value::String(note));
            Value::Object(m)
        })
        .collect();
    Ok(Value::Array(items))
}

pub fn register_sauce_globals(env: &mut Environment) {
    env.set("os_sauce_map".into(), Value::NativeFunction(os_sauce_map_native));
    env.set("os_ai_prefetch".into(), Value::NativeFunction(os_ai_prefetch_native));
    env.set("os_ai_record".into(), Value::NativeFunction(os_ai_record_native));
    env.set("os_ai_context_menu".into(), Value::NativeFunction(os_ai_context_menu_native));
    env.set("os_setup_nfc".into(), Value::NativeFunction(os_setup_nfc_native));
    env.set("os_recovery_restore".into(), Value::NativeFunction(os_recovery_restore_native));
    env.set("os_seamless_pair".into(), Value::NativeFunction(os_seamless_pair_native));
    env.set(
        "os_seamless_clipboard_push".into(),
        Value::NativeFunction(os_seamless_clipboard_push_native),
    );
    env.set(
        "os_seamless_clipboard_poll".into(),
        Value::NativeFunction(os_seamless_clipboard_poll_native),
    );
    env.set("os_energy_schedule".into(), Value::NativeFunction(os_energy_schedule_native));
    env.set("os_haptic_danger".into(), Value::NativeFunction(os_haptic_danger_native));
    env.set("os_compat_run".into(), Value::NativeFunction(os_compat_run_native));
    env.set("os_privacy_panic".into(), Value::NativeFunction(os_privacy_panic_native));
    env.set(
        "os_privacy_telemetry".into(),
        Value::NativeFunction(os_privacy_telemetry_native),
    );
    env.set("os_update_channel".into(), Value::NativeFunction(os_update_channel_native));
    env.set("os_update_rollback".into(), Value::NativeFunction(os_update_rollback_native));
    env.set("os_sauce_honesty".into(), Value::NativeFunction(os_sauce_honesty_native));
}
