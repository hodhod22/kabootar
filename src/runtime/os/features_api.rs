//! OS feature natives — boot, search, eco, packages, snapshots (safe increments).

use super::{KernelSubsystems, OsHandle};
use crate::docai::search as doc_search;
use crate::registry::install_package;
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

fn os_boot_ms_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(get_os(env)?.boot_ms() as i64))
}

fn os_search_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let query = str_arg(args, 0).unwrap_or_default().to_ascii_lowercase();
    let os = get_os(env)?;
    let mut hits = Vec::new();

    for dir in ["/", "/apps", "/system", "/data"] {
        if let Ok(entries) = os.list(dir) {
            for name in entries {
                let path = if dir == "/" {
                    format!("/{name}")
                } else {
                    format!("{dir}/{name}")
                };
                if query.is_empty() || name.to_ascii_lowercase().contains(&query) {
                    let mut m = HashMap::new();
                    m.insert("source".into(), Value::String("vfs".into()));
                    m.insert("title".into(), Value::String(name.clone()));
                    m.insert("path".into(), Value::String(path));
                    hits.push(Value::Object(m));
                }
            }
        }
    }

    if !query.is_empty() {
        for h in doc_search(&query, 8) {
            let mut m = HashMap::new();
            m.insert("source".into(), Value::String("docs".into()));
            m.insert("title".into(), Value::String(format!("{} — {}", h.path, h.heading)));
            m.insert("snippet".into(), Value::String(h.excerpt));
            hits.push(Value::Object(m));
        }
    }

    Ok(Value::Array(hits))
}

fn os_eco_mode_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let on = args.get(0).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(true);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.sauce.energy.set_eco_mode(on);
        if on {
            s.xcut.power.set_c_state(2);
        }
        Ok(Value::Bool(on))
    })
}

fn os_energy_battery_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let on_battery = args.get(0).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(true);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.sauce.energy.set_power_source(on_battery);
        Ok(Value::Bool(on_battery))
    })
}

fn os_snapshot_list_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    let list = os.vfs_snapshot_list()?;
    Ok(Value::Array(
        list.into_iter()
            .map(|s| Value::String(s))
            .collect(),
    ))
}

fn os_pkg_install_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let spec = str_arg(args, 0).ok_or("os_pkg_install(name@version)")?;
    let (name, ver) = spec
        .split_once('@')
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .unwrap_or((spec.clone(), "latest".into()));
    let base = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let reg = install_package(&name, &ver, &base).ok();
    let os = get_os(env)?;
    let path = format!("/apps/{name}.kv8");
    let bundle = format!(
        "---kml---\n<div id='app'><h1>{name}</h1></div>\n---css---\n#app {{ padding: 12px; }}\n---script---\n// pkg {name}@{ver}\n"
    );
    os.write(&path, bundle)?;
    let mut o = HashMap::new();
    o.insert("package".into(), Value::String(name));
    o.insert("version".into(), Value::String(ver));
    o.insert("path".into(), Value::String(path));
    if let Some(info) = reg {
        let mut reg_map = HashMap::new();
        reg_map.insert("name".into(), Value::String(info.name));
        reg_map.insert("version".into(), Value::String(info.version));
        o.insert("registry".into(), Value::Object(reg_map));
    }
    Ok(Value::Object(o))
}

fn os_privacy_telemetry_enable_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let enabled = args.get(0).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(false);
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        s.sauce.privacy.set_telemetry_enabled(enabled);
        Ok(Value::Bool(enabled))
    })
}

fn os_seamless_list_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::Array(
            s.sauce
                .seamless
                .list_paired()
                .into_iter()
                .map(|d| {
                    let mut m = HashMap::new();
                    m.insert("id".into(), Value::String(d.id));
                    m.insert("kind".into(), Value::String(d.kind));
                    m.insert("hz".into(), Value::Number(d.ultrasonic_hz as i64));
                    Value::Object(m)
                })
                .collect(),
        ))
    })
}

fn os_debug_suggest_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let context = str_arg(args, 0).unwrap_or_else(|| "crash".into());
    let os = get_os(env)?;
    with_subsys(&os, |s| {
        Ok(Value::String(
            s.sauce
                .ai
                .debug_suggest(&context)
                .unwrap_or_else(|| "no suggestion".into()),
        ))
    })
}

fn os_features_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os(env)?;
    let mut o = HashMap::new();
    o.insert("boot_ms".into(), Value::Number(os.boot_ms() as i64));
    with_subsys(&os, |s| {
        let (battery, _, deferred, _) = s.sauce.energy.stats();
        o.insert("battery".into(), Value::Bool(battery));
        o.insert("eco_deferred".into(), Value::Number(deferred as i64));
        o.insert(
            "telemetry".into(),
            Value::Bool(s.sauce.privacy.telemetry_enabled()),
        );
        Ok(())
    })?;
    o.insert(
        "snapshots".into(),
        Value::Number(os.vfs_snapshot_list()?.len() as i64),
    );
    Ok(Value::Object(o))
}

pub fn register_features_globals(env: &mut Environment) {
    env.set("os_boot_ms".into(), Value::NativeFunction(os_boot_ms_native));
    env.set("os_search".into(), Value::NativeFunction(os_search_native));
    env.set("os_eco_mode".into(), Value::NativeFunction(os_eco_mode_native));
    env.set(
        "os_energy_battery".into(),
        Value::NativeFunction(os_energy_battery_native),
    );
    env.set("os_snapshot_list".into(), Value::NativeFunction(os_snapshot_list_native));
    env.set("os_pkg_install".into(), Value::NativeFunction(os_pkg_install_native));
    env.set(
        "os_privacy_telemetry_enable".into(),
        Value::NativeFunction(os_privacy_telemetry_enable_native),
    );
    env.set(
        "os_seamless_list".into(),
        Value::NativeFunction(os_seamless_list_native),
    );
    env.set("os_debug_suggest".into(), Value::NativeFunction(os_debug_suggest_native));
    env.set("os_features_info".into(), Value::NativeFunction(os_features_info_native));
}
