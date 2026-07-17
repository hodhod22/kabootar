//! PWA — manifests, service workers, offline cache, fetch events (C8).

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
    /// True when the SW script registers a `fetch` listener (or via `pwa_on_fetch`).
    pub fetch_listener: bool,
    /// Fetch strategy: `cache-first` (default) | `offline-only` | `network-stub`.
    pub fetch_strategy: String,
}

#[derive(Clone, Debug)]
pub struct FetchEventResult {
    pub url: String,
    pub scope: String,
    pub handled: bool,
    pub from_cache: bool,
    pub status: i64,
    pub body: String,
}

static WORKERS: OnceLock<Mutex<HashMap<String, ServiceWorker>>> = OnceLock::new();
static OFFLINE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static LAST_FETCH: OnceLock<Mutex<Option<FetchEventResult>>> = OnceLock::new();

fn workers() -> &'static Mutex<HashMap<String, ServiceWorker>> {
    WORKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn offline_cache() -> &'static Mutex<HashMap<String, String>> {
    OFFLINE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn last_fetch() -> &'static Mutex<Option<FetchEventResult>> {
    LAST_FETCH.get_or_init(|| Mutex::new(None))
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

fn script_has_fetch_listener(script: &str) -> bool {
    let lower = script.to_ascii_lowercase();
    lower.contains("addeventlistener('fetch'")
        || lower.contains("addeventlistener(\"fetch\"")
        || lower.contains("addeventlistener(`fetch`")
        || lower.contains(".onfetch")
        || lower.contains("on('fetch'")
}

pub fn register_worker(scope: &str, script: &str) -> Result<bool, String> {
    let fetch_listener = script_has_fetch_listener(script);
    workers()
        .lock()
        .map_err(|_| "pwa worker lock".to_string())?
        .insert(
            scope.to_string(),
            ServiceWorker {
                scope: scope.to_string(),
                script: script.to_string(),
                cache: HashMap::new(),
                fetch_listener,
                fetch_strategy: "cache-first".into(),
            },
        );
    Ok(true)
}

/// Enable/override fetch handling for a registered worker scope.
pub fn on_fetch(scope: &str, strategy: &str) -> Result<bool, String> {
    let strategy = match strategy {
        "cache-first" | "offline-only" | "network-stub" => strategy.to_string(),
        other => {
            return Err(format!(
                "pwa_on_fetch: unknown strategy '{other}' (cache-first|offline-only|network-stub)"
            ))
        }
    };
    let mut guard = workers()
        .lock()
        .map_err(|_| "pwa worker lock".to_string())?;
    let worker = guard
        .get_mut(scope)
        .ok_or_else(|| format!("pwa_on_fetch: no worker for scope '{scope}'"))?;
    worker.fetch_listener = true;
    worker.fetch_strategy = strategy;
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

/// Dispatch a FetchEvent to the longest matching service worker scope (C8).
pub fn dispatch_fetch(url: &str) -> Result<FetchEventResult, String> {
    let workers_guard = workers()
        .lock()
        .map_err(|_| "pwa worker lock".to_string())?;
    let mut best: Option<&ServiceWorker> = None;
    for worker in workers_guard.values() {
        if url.starts_with(&worker.scope) || worker.scope == "/" {
            let better = match best {
                None => true,
                Some(cur) => worker.scope.len() > cur.scope.len(),
            };
            if better {
                best = Some(worker);
            }
        }
    }
    let Some(worker) = best else {
        let result = FetchEventResult {
            url: url.to_string(),
            scope: String::new(),
            handled: false,
            from_cache: false,
            status: 503,
            body: String::new(),
        };
        *last_fetch().lock().map_err(|_| "pwa last_fetch lock")? = Some(result.clone());
        return Ok(result);
    };

    if !worker.fetch_listener {
        let result = FetchEventResult {
            url: url.to_string(),
            scope: worker.scope.clone(),
            handled: false,
            from_cache: false,
            status: 0,
            body: String::new(),
        };
        *last_fetch().lock().map_err(|_| "pwa last_fetch lock")? = Some(result.clone());
        return Ok(result);
    }

    let scope = worker.scope.clone();
    let strategy = worker.fetch_strategy.clone();
    let sw_hit = worker.cache.get(url).cloned();
    drop(workers_guard);

    let offline_hit = offline_cache()
        .lock()
        .ok()
        .and_then(|c| c.get(url).cloned());

    let result = match strategy.as_str() {
        "network-stub" => FetchEventResult {
            url: url.to_string(),
            scope,
            handled: true,
            from_cache: false,
            status: 200,
            body: format!("network-stub:{url}"),
        },
        "offline-only" => match offline_hit.or(sw_hit) {
            Some(body) => FetchEventResult {
                url: url.to_string(),
                scope,
                handled: true,
                from_cache: true,
                status: 200,
                body,
            },
            None => FetchEventResult {
                url: url.to_string(),
                scope,
                handled: true,
                from_cache: false,
                status: 504,
                body: "offline-miss".into(),
            },
        },
        // cache-first
        _ => match sw_hit.or(offline_hit) {
            Some(body) => FetchEventResult {
                url: url.to_string(),
                scope,
                handled: true,
                from_cache: true,
                status: 200,
                body,
            },
            None => FetchEventResult {
                url: url.to_string(),
                scope,
                handled: true,
                from_cache: false,
                status: 404,
                body: "fetch-miss".into(),
            },
        },
    };

    *last_fetch().lock().map_err(|_| "pwa last_fetch lock")? = Some(result.clone());
    Ok(result)
}

pub fn last_fetch_event() -> Option<FetchEventResult> {
    last_fetch().lock().ok()?.clone()
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
    o.insert("fetch_events".into(), "true".into());
    o.insert("phase".into(), "C8".into());
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
