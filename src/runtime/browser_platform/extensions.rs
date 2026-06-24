//! Browser extensions — manifests + content script injection.

use super::json_util::{extract_array_strings, extract_field};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Clone)]
pub struct Extension {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
    pub enabled: bool,
    pub entry: String,
    pub content_scripts: Vec<String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static EXT: OnceLock<Mutex<HashMap<u64, Extension>>> = OnceLock::new();

fn ext_store() -> &'static Mutex<HashMap<u64, Extension>> {
    EXT.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn parse_manifest(json: &str) -> Result<Extension, String> {
    let name = extract_field(json, "name").unwrap_or_else(|| "extension".to_string());
    let version = extract_field(json, "version").unwrap_or_else(|| "1.0.0".to_string());
    let entry = extract_field(json, "entry").unwrap_or_else(|| "background.kv8".to_string());
    let permissions = extract_array_strings(json, "permissions");
    let content_scripts = extract_array_strings(json, "content_scripts");
    let permissions = if permissions.is_empty() && json.contains("permissions") {
        vec!["tabs".into(), "storage".into()]
    } else {
        permissions
    };

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let ext = Extension {
        id,
        name,
        version,
        permissions,
        enabled: true,
        entry,
        content_scripts,
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

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("api".into(), "Kabootar Extensions".into());
    o.insert("manifest".into(), "json".into());
    o.insert("content_scripts".into(), "true".into());
    o.insert("phase".into(), "v2.55".into());
    o.insert(
        "installed".into(),
        ext_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
