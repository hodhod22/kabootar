//! GP4a — process-wide asset path watch / poll (mtime snapshots).

use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

fn watches() -> &'static Mutex<HashMap<String, SystemTime>> {
    static W: OnceLock<Mutex<HashMap<String, SystemTime>>> = OnceLock::new();
    W.get_or_init(|| Mutex::new(HashMap::new()))
}

fn mtime_of(path: &str) -> Result<SystemTime, String> {
    let meta = fs::metadata(path).map_err(|e| format!("asset_watch: {path}: {e}"))?;
    meta.modified()
        .map_err(|e| format!("asset_watch mtime: {path}: {e}"))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Record `path` and its current mtime. Re-watching refreshes the snapshot.
pub fn watch(path: &str) -> Result<(), String> {
    let key = normalize_path(path);
    let mt = mtime_of(&key)?;
    let mut map = watches()
        .lock()
        .map_err(|_| "asset_watch: lock poisoned".to_string())?;
    map.insert(key, mt);
    Ok(())
}

/// Return paths whose mtime changed since last watch/poll; update snapshots.
/// On `.kab` change, invalidate the compile file cache.
pub fn poll() -> Result<Vec<String>, String> {
    let mut map = watches()
        .lock()
        .map_err(|_| "asset_poll: lock poisoned".to_string())?;
    let mut changed = Vec::new();
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        let Ok(mt) = mtime_of(&key) else {
            continue;
        };
        let prev = map.get(&key).copied();
        if prev != Some(mt) {
            map.insert(key.clone(), mt);
            if key.ends_with(".kab") || key.ends_with(".kabootar") {
                crate::compile::invalidate_file_cache(&key);
                crate::modules::invalidate_module_export_cache(&key);
            }
            if key.ends_with(".wgsl") {
                let _ = crate::runtime::render::gpu3d::reload_wgsl_path(&key);
            }
            changed.push(key);
        }
    }
    Ok(changed)
}

pub fn reset_for_tests() {
    if let Ok(mut map) = watches().lock() {
        map.clear();
    }
}

pub fn asset_watch_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("asset_watch(path) expects string".into()),
    };
    watch(path)?;
    Ok(Value::Null)
}

pub fn asset_poll_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let changed = poll()?;
    Ok(Value::Array(
        changed.into_iter().map(Value::String).collect(),
    ))
}

/// True if `path` looks like a watched asset extension we care about.
#[allow(dead_code)]
pub fn is_asset_path(path: &str) -> bool {
    let p = Path::new(path);
    matches!(
        p.extension().and_then(|e| e.to_str()),
        Some("kab" | "kabootar" | "png" | "gltf" | "glb" | "wgsl" | "vert" | "frag")
    )
}
