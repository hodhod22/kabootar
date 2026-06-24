//! PWA — manifests, service workers, offline cache.

use super::json_util::{extract_array_strings, extract_field};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct PwaManifest {
    pub name: String,
    pub short_name: String,
    pub start_url: String,
    pub display: String,
    pub theme_color: String,
    pub icons: Vec<String>,
}

#[derive(Clone)]
pub struct ServiceWorker {
    pub scope: String,
    pub script: String,
    pub cache: HashMap<String, String>,
}

static WORKERS: OnceLock<Mutex<HashMap<String, ServiceWorker>>> = OnceLock::new();
static OFFLINE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn workers() -> &'static Mutex<HashMap<String, ServiceWorker>> {
    WORKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn offline_cache() -> &'static Mutex<HashMap<String, String>> {
    OFFLINE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn parse_manifest(json: &str) -> Result<PwaManifest, String> {
    Ok(PwaManifest {
        name: extract_field(json, "name").unwrap_or_else(|| "Kabootar App".into()),
        short_name: extract_field(json, "short_name").unwrap_or_else(|| "App".into()),
        start_url: extract_field(json, "start_url").unwrap_or_else(|| "/".into()),
        display: extract_field(json, "display").unwrap_or_else(|| "standalone".into()),
        theme_color: extract_field(json, "theme_color").unwrap_or_else(|| "#1a1a2e".into()),
        icons: extract_array_strings(json, "icons"),
    })
}

pub fn install_to_os(
    manifest: &PwaManifest,
    os: &crate::runtime::os::OsHandle,
) -> Result<String, String> {
    let app_dir = format!("/apps/{}", slugify(&manifest.short_name));
    let _ = os.mkdir(&app_dir);
    let manifest_path = format!("{app_dir}/manifest.webmanifest");
    let kv8_path = format!("{app_dir}/app.kv8");
    let sw_path = format!("{app_dir}/sw.js");
    let bundle = format!(
        "---kml---\n<html><body><h1>{}</h1></body></html>\n---css---\nbody {{ background: {}; }}\n---script---\n// PWA shell\n",
        manifest.name, manifest.theme_color
    );
    let sw_script = format!(
        "// Service worker for {}\nself.addEventListener('fetch', (e) => {{ /* offline */ }});\n",
        manifest.name
    );
    os.write(&manifest_path, serde_manifest(manifest))?;
    os.write(&kv8_path, bundle.clone())?;
    os.write(&sw_path, sw_script.clone())?;
    register_worker(&manifest.start_url, &sw_script)?;
    cache_put(&manifest.start_url, &bundle)?;
    Ok(format!("kabootar://vfs{kv8_path}"))
}

pub fn register_worker(scope: &str, script: &str) -> Result<bool, String> {
    workers()
        .lock()
        .map_err(|_| "pwa worker lock".to_string())?
        .insert(
            scope.to_string(),
            ServiceWorker {
                scope: scope.to_string(),
                script: script.to_string(),
                cache: HashMap::new(),
            },
        );
    Ok(true)
}

pub fn cache_put(url: &str, body: &str) -> Result<bool, String> {
    offline_cache()
        .lock()
        .map_err(|_| "pwa cache lock".to_string())?
        .insert(url.to_string(), body.to_string());
    if let Ok(mut w) = workers().lock() {
        for worker in w.values_mut() {
            if url.starts_with(&worker.scope) || worker.scope == "/" {
                worker.cache.insert(url.to_string(), body.to_string());
            }
        }
    }
    Ok(true)
}

pub fn fetch_cached(url: &str) -> Option<String> {
    offline_cache().lock().ok()?.get(url).cloned()
}

pub fn list_workers() -> Vec<ServiceWorker> {
    workers()
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn serde_manifest(m: &PwaManifest) -> String {
    format!(
        r#"{{"name":"{}","short_name":"{}","start_url":"{}","display":"{}","theme_color":"{}"}}"#,
        m.name, m.short_name, m.start_url, m.display, m.theme_color
    )
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("api".into(), "PWA (Kabootar)".into());
    o.insert("install".into(), "os-vfs".into());
    o.insert("service_worker".into(), "true".into());
    o.insert("offline".into(), "true".into());
    o.insert("phase".into(), "v2.55".into());
    o.insert(
        "workers".into(),
        workers()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o.insert(
        "cache_entries".into(),
        offline_cache()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
