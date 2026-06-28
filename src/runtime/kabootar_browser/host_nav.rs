//! Multi-OS navigation — Kabootar VFS, host filesystem, and HTTP.

use crate::runtime::kabootar_dom::assign_ids;
use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kv8::{parse_kv8_module, Kv8Module};
use crate::runtime::os::OsHandle;
use crate::runtime::kstyle::Stylesheet;
use crate::kml::parse_kml;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserOsMode {
    Auto,
    Kabootar,
    Host,
}

impl BrowserOsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Kabootar => "kabootar",
            Self::Host => "host",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "kabootar" | "k" => Some(Self::Kabootar),
            "host" | "native" => Some(Self::Host),
            _ => None,
        }
    }
}

pub fn host_os_name() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        return "wasm";
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
    {
        "windows"
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
    {
        "linux"
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
    {
        "macos"
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(target_os = "windows"),
        not(target_os = "linux"),
        not(target_os = "macos")
    ))]
    {
        "unknown"
    }
}

pub(crate) struct LoadedPage {
    pub document: DomNode,
    pub kv8_script: Option<String>,
    pub kv8_css: Option<String>,
    pub kv8_parsed_stylesheet: Option<Stylesheet>,
    pub source: String,
}

pub fn os_info_map(os: Option<&OsHandle>, mode: BrowserOsMode) -> HashMap<String, crate::value::Value> {
    let mut m = HashMap::new();
    m.insert("mode".into(), crate::value::Value::String(mode.as_str().into()));
    m.insert(
        "host_os".into(),
        crate::value::Value::String(host_os_name().into()),
    );
    m.insert(
        "kabootar_os".into(),
        crate::value::Value::Bool(os.is_some()),
    );
    if let Some(handle) = os {
        if let Ok(mounts) = handle.list_mounts() {
            m.insert(
                "mounts".into(),
                crate::value::Value::Array(
                    mounts
                        .into_iter()
                        .map(|(vfs, host)| {
                            let mut o = HashMap::new();
                            o.insert("vfs".into(), crate::value::Value::String(vfs));
                            o.insert("host".into(), crate::value::Value::String(host));
                            crate::value::Value::Object(o)
                        })
                        .collect(),
                ),
            );
        }
    }
    m.insert(
        "schemes".into(),
        crate::value::Value::Array(vec![
            crate::value::Value::String("kabootar://vfs/...".into()),
            crate::value::Value::String("file://...".into()),
            crate::value::Value::String("host://...".into()),
            crate::value::Value::String("http(s)://...".into()),
        ]),
    );
    m
}

pub fn load_page(
    url: &str,
    os: Option<&OsHandle>,
    mode: BrowserOsMode,
    fallback_home: fn(&str) -> DomNode,
) -> LoadedPage {
    let effective = effective_mode(url, mode);
    if let Ok(page) = try_load(url, os, effective) {
        return page;
    }
    if effective != BrowserOsMode::Auto {
        if let Ok(page) = try_load(url, os, BrowserOsMode::Auto) {
            return page;
        }
    }
    LoadedPage {
        document: fallback_home(url),
        kv8_script: None,
        kv8_css: None,
        kv8_parsed_stylesheet: None,
        source: "fallback".into(),
    }
}

fn effective_mode(url: &str, mode: BrowserOsMode) -> BrowserOsMode {
    match mode {
        BrowserOsMode::Auto => {
            if url.starts_with("kabootar://") {
                BrowserOsMode::Kabootar
            } else if url.starts_with("file://")
                || url.starts_with("host://")
                || url.starts_with("http://")
                || url.starts_with("https://")
            {
                BrowserOsMode::Host
            } else {
                BrowserOsMode::Kabootar
            }
        }
        other => other,
    }
}

fn try_load(url: &str, os: Option<&OsHandle>, mode: BrowserOsMode) -> Result<LoadedPage, String> {
    let (content, source) = fetch_content(url, os, mode)?;
    parse_content(&content, url, source)
}

fn normalize_vfs_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    if path.is_empty() || path == "/" {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn fetch_content(
    url: &str,
    os: Option<&OsHandle>,
    mode: BrowserOsMode,
) -> Result<(String, String), String> {
    if url == "kabootar://home" || url == "kabootar://new" {
        return Err("virtual home".into());
    }

    if let Some(path) = url.strip_prefix("kabootar://vfs") {
        if mode == BrowserOsMode::Host {
            return Err("kabootar vfs disabled in host mode".into());
        }
        let handle = os.ok_or("Kabootar OS not available")?;
        let vfs_path = normalize_vfs_path(path);
        let content = handle.read(&vfs_path)?;
        return Ok((content, "kabootar-vfs".into()));
    }

    if let Some(path) = url.strip_prefix("file://") {
        return read_host_path(path, "file");
    }
    if let Some(path) = url.strip_prefix("host://") {
        return read_host_path(path, "host");
    }

    if url.starts_with("http://") || url.starts_with("https://") {
        return fetch_http(url);
    }

    if mode == BrowserOsMode::Kabootar {
        if let Some(handle) = os {
            if url.starts_with('/') {
                if let Ok(content) = handle.read(url) {
                    return Ok((content, "kabootar-vfs".into()));
                }
            }
        }
    }

    Err(format!("unsupported url for mode {}: {url}", mode.as_str()))
}

fn read_host_path(path: &str, source: &str) -> Result<(String, String), String> {
    let path = decode_file_url(path);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("host read {path}: {e}"))?;
        return Ok((content, source.into()));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err("host file access requires native runtime (use kabootar://vfs or http)".into())
    }
}

fn decode_file_url(path: &str) -> String {
    let path = path.trim_start_matches('/');
    #[cfg(windows)]
    {
        if path.len() >= 2 {
            let bytes: Vec<char> = path.chars().collect();
            if bytes[1] == ':' {
                return path.replace('/', "\\");
            }
        }
    }
    if path.is_empty() {
        ".".into()
    } else {
        path.replace('\\', "/")
    }
}

fn fetch_http(url: &str) -> Result<(String, String), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let resp = crate::runtime::net::http_fetch_default("GET", url, "")?;
        if resp.status >= 400 {
            return Err(format!("HTTP {} for {url}", resp.status));
        }
        return Ok((resp.body, "http".into()));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        Err("sync HTTP in browser requires native; use http_fetch_async in host layer".into())
    }
}

fn parse_content(content: &str, url: &str, source: String) -> Result<LoadedPage, String> {
    let kv8_script = None;
    let kv8_css = None;
    let kv8_parsed_stylesheet = None;

    if url.ends_with(".kv8") || content.contains("---kml---") {
        if let Ok(module) = parse_kv8_module(content) {
            return page_from_kv8(module, source);
        }
    }

    if let Ok(mut node) = parse_kml(content) {
        assign_ids(&mut node);
        return Ok(LoadedPage {
            document: node,
            kv8_script,
            kv8_css,
            kv8_parsed_stylesheet,
            source,
        });
    }

    if content.trim_start().starts_with('<') {
        if let Ok(mut node) = parse_kml(&format!(
            "<html><body style=\"padding:16px;\"><pre>{}</pre></body></html>",
            xml_escape(content)
        )) {
            assign_ids(&mut node);
            return Ok(LoadedPage {
                document: node,
                kv8_script,
                kv8_css,
                kv8_parsed_stylesheet,
                source,
            });
        }
    }

    if let Ok(mut node) = parse_kml(&format!(
        "<html><body style=\"padding:16px;background:#292a2d;color:#e8eaed;\"><h1>{}</h1><pre>{}</pre></body></html>",
        title_from_url(url),
        xml_escape(content)
    )) {
        assign_ids(&mut node);
        Ok(LoadedPage {
            document: node,
            kv8_script,
            kv8_css,
            kv8_parsed_stylesheet,
            source,
        })
    } else {
        Err("failed to parse page content".into())
    }
}

fn page_from_kv8(module: Kv8Module, source: String) -> Result<LoadedPage, String> {
    let kv8_script = Some(module.script.clone());
    let kv8_css = Some(module.css.clone());
    let kv8_parsed_stylesheet = Some(crate::runtime::kstyle::parse_stylesheet(&module.css));
    let mut node = parse_kml(&module.kml)?;
    assign_ids(&mut node);
    Ok(LoadedPage {
        document: node,
        kv8_script,
        kv8_css,
        kv8_parsed_stylesheet,
        source,
    })
}

fn title_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_from_str() {
        assert_eq!(BrowserOsMode::from_str("host"), Some(BrowserOsMode::Host));
        assert_eq!(BrowserOsMode::from_str("auto"), Some(BrowserOsMode::Auto));
    }

    #[test]
    fn effective_mode_picks_host_for_file() {
        assert_eq!(
            effective_mode("file:///tmp/x.html", BrowserOsMode::Auto),
            BrowserOsMode::Host
        );
    }
}
