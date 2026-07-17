//! Browser extensions — manifests, content scripts, permissions (C8).

use super::json_util::{extract_array_strings, extract_field};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const KNOWN_PERMISSIONS: &[&str] = &[
    "tabs",
    "storage",
    "activeTab",
    "scripting",
    "alarms",
    "notifications",
];

#[derive(Clone)]
pub struct Extension {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
    pub enabled: bool,
    pub entry: String,
    pub content_scripts: Vec<String>,
    pub storage: HashMap<String, String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static EXT: OnceLock<Mutex<HashMap<u64, Extension>>> = OnceLock::new();

fn ext_store() -> &'static Mutex<HashMap<u64, Extension>> {
    EXT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn normalize_permission(p: &str) -> String {
    p.trim().to_string()
}

pub fn is_known_permission(perm: &str) -> bool {
    KNOWN_PERMISSIONS.iter().any(|k| *k == perm)
}

pub fn parse_manifest(json: &str) -> Result<Extension, String> {
    let name = extract_field(json, "name").unwrap_or_else(|| "extension".to_string());
    let version = extract_field(json, "version").unwrap_or_else(|| "1.0.0".to_string());
    let entry = extract_field(json, "entry").unwrap_or_else(|| "background.kv8".to_string());
    let permissions: Vec<String> = extract_array_strings(json, "permissions")
        .into_iter()
        .map(|p| normalize_permission(&p))
        .filter(|p| !p.is_empty())
        .collect();
    let content_scripts = extract_array_strings(json, "content_scripts");

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let ext = Extension {
        id,
        name,
        version,
        permissions,
        enabled: true,
        entry,
        content_scripts,
        storage: HashMap::new(),
    };
    ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?
        .insert(id, ext.clone());
    Ok(ext)
}

pub fn list_extensions() -> Vec<Extension> {
    ext_store()
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

pub fn enabled_content_scripts() -> Vec<String> {
    ext_store()
        .lock()
        .map(|m| {
            m.values()
                .filter(|e| e.enabled)
                .flat_map(|e| e.content_scripts.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Run enabled extension content scripts in a Kv8 context after navigation.
pub fn inject_on_navigate(url: &str, ctx: &crate::runtime::kv8::Kv8Context) {
    let scripts: Vec<String> = ext_store()
        .lock()
        .map(|m| {
            m.values()
                .filter(|e| e.enabled && !e.content_scripts.is_empty())
                .flat_map(|e| e.content_scripts.clone())
                .map(|s| format!("// ext inject @ {url}\n{s}"))
                .collect()
        })
        .unwrap_or_default();
    for script in scripts {
        let _ = crate::runtime::kv8::eval_script(ctx, &script);
    }
}

pub fn set_enabled(id: u64, enabled: bool) -> Result<bool, String> {
    let mut guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get_mut(&id).ok_or("extension not found")?;
    ext.enabled = enabled;
    Ok(ext.enabled)
}

pub fn has_permission(id: u64, perm: &str) -> Result<bool, String> {
    let guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get(&id).ok_or("extension not found")?;
    Ok(ext.permissions.iter().any(|p| p == perm))
}

pub fn request_permission(id: u64, perm: &str) -> Result<bool, String> {
    let perm = normalize_permission(perm);
    if !is_known_permission(&perm) {
        return Err(format!("ext_request_permission: unknown permission '{perm}'"));
    }
    let mut guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get_mut(&id).ok_or("extension not found")?;
    if !ext.permissions.iter().any(|p| p == &perm) {
        ext.permissions.push(perm);
    }
    Ok(true)
}

pub fn revoke_permission(id: u64, perm: &str) -> Result<bool, String> {
    let mut guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get_mut(&id).ok_or("extension not found")?;
    let before = ext.permissions.len();
    ext.permissions.retain(|p| p != perm);
    Ok(ext.permissions.len() < before)
}

pub fn require_permission(id: u64, perm: &str) -> Result<(), String> {
    if has_permission(id, perm)? {
        Ok(())
    } else {
        Err(format!(
            "extension {id} missing permission '{perm}'"
        ))
    }
}

pub fn list_permissions(id: u64) -> Result<Vec<String>, String> {
    let guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get(&id).ok_or("extension not found")?;
    Ok(ext.permissions.clone())
}

/// `chrome.storage`-style get — requires `storage` permission.
pub fn storage_get(id: u64, key: &str) -> Result<Option<String>, String> {
    require_permission(id, "storage")?;
    let guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get(&id).ok_or("extension not found")?;
    Ok(ext.storage.get(key).cloned())
}

/// `chrome.storage`-style set — requires `storage` permission.
pub fn storage_set(id: u64, key: &str, value: &str) -> Result<bool, String> {
    require_permission(id, "storage")?;
    let mut guard = ext_store()
        .lock()
        .map_err(|_| "extensions lock poisoned".to_string())?;
    let ext = guard.get_mut(&id).ok_or("extension not found")?;
    ext.storage.insert(key.to_string(), value.to_string());
    Ok(true)
}

/// Stub tabs query — requires `tabs` permission.
pub fn tabs_query(id: u64) -> Result<Vec<String>, String> {
    require_permission(id, "tabs")?;
    Ok(vec!["kabootar://active".into()])
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("api".into(), "Kabootar Extensions".into());
    o.insert("manifest".into(), "json".into());
    o.insert("content_scripts".into(), "true".into());
    o.insert("permissions".into(), "true".into());
    o.insert("phase".into(), "C8".into());
    o.insert(
        "known_permissions".into(),
        KNOWN_PERMISSIONS.join(","),
    );
    o.insert(
        "installed".into(),
        ext_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
