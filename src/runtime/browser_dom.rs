//! Bridge to the host browser (layer 1) — Chrome-like APIs + WASM web_sys bridge.

use crate::value::{Environment, Value};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

fn host_document_object() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert(
        "querySelector".into(),
        Value::NativeFunction(document_query_native),
    );
    m.insert(
        "createElement".into(),
        Value::NativeFunction(document_create_native),
    );
    m.insert(
        "getElementById".into(),
        Value::NativeFunction(document_get_by_id_native),
    );
    m.insert("title".into(), Value::String(host_document_title()));
    m.insert("layer".into(), Value::String("host".into()));
    m
}

fn host_document_title() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_document() {
            return doc.title();
        }
    }
    "Kabootar Host".into()
}

#[cfg(target_arch = "wasm32")]
fn web_document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

fn host_window_object() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    let (href, inner_w, inner_h) = host_window_metrics();
    m.insert(
        "location".into(),
        Value::Object({
            let mut loc = HashMap::new();
            loc.insert("href".into(), Value::String(href));
            loc.insert("protocol".into(), Value::String("kabootar:".into()));
            loc
        }),
    );
    m.insert("innerWidth".into(), Value::Number(inner_w));
    m.insert("innerHeight".into(), Value::Number(inner_h));
    m.insert("fetch".into(), Value::NativeFunction(window_fetch_native));
    m.insert("layer".into(), Value::String("host".into()));
    m
}

fn host_window_metrics() -> (String, i64, i64) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            let href = win.location().ok().and_then(|l| l.href().ok()).unwrap_or_else(|| "kabootar://host".into());
            let w = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1280.0) as i64;
            let h = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(720.0) as i64;
            return (href, w, h);
        }
    }
    ("kabootar://host".into(), 1280, 720)
}

fn host_navigator_object() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert(
        "userAgent".into(),
        Value::String(format!(
            "Mozilla/5.0 (Kabootar Host) Chrome/120.0 Kabootar/{}",
            env!("CARGO_PKG_VERSION")
        )),
    );
    m.insert("platform".into(), Value::String(host_platform_name().into()));
    m.insert("language".into(), Value::String("sv-SE".into()));
    m.insert("onLine".into(), Value::Bool(true));
    m.insert("layer".into(), Value::String("host".into()));
    m
}

fn host_platform_name() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        return "WASM";
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
    {
        "Win32"
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
    {
        "Linux"
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
    {
        "MacIntel"
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(target_os = "windows"),
        not(target_os = "linux"),
        not(target_os = "macos")
    ))]
    {
        "Native"
    }
}

fn document_query_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let selector = expect_str(args, 0, "document.querySelector")?;
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_document() {
            if let Ok(Some(el)) = doc.query_selector(&selector) {
                let mut out = HashMap::new();
                out.insert("tag".into(), Value::String(el.tag_name()));
                out.insert("layer".into(), Value::String("host".into()));
                return Ok(Value::Object(out));
            }
        }
    }
    let mut el = HashMap::new();
    el.insert("tag".into(), Value::String(selector));
    el.insert("layer".into(), Value::String("host".into()));
    Ok(Value::Object(el))
}

fn document_create_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tag = expect_str(args, 0, "document.createElement")?;
    if tag.eq_ignore_ascii_case("canvas") {
        return crate::runtime::browser_platform::canvas_host::create_element(300, 150);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_document() {
            if let Ok(el) = doc.create_element(&tag) {
                let mut out = HashMap::new();
                out.insert("tag".into(), Value::String(el.tag_name()));
                out.insert("layer".into(), Value::String("host".into()));
                return Ok(Value::Object(out));
            }
        }
    }
    let mut el = HashMap::new();
    el.insert("tag".into(), Value::String(tag));
    el.insert("innerHTML".into(), Value::String(String::new()));
    el.insert("layer".into(), Value::String("host".into()));
    Ok(Value::Object(el))
}

fn document_get_by_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_str(args, 0, "document.getElementById")?;
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_document() {
            if let Ok(Some(el)) = doc.get_element_by_id(&id) {
                let mut out = HashMap::new();
                out.insert("id".into(), Value::String(id));
                out.insert("tag".into(), Value::String(el.tag_name()));
                out.insert("layer".into(), Value::String("host".into()));
                return Ok(Value::Object(out));
            }
        }
    }
    let mut el = HashMap::new();
    el.insert("id".into(), Value::String(id));
    el.insert("layer".into(), Value::String("host".into()));
    Ok(Value::Object(el))
}

fn window_fetch_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let url = expect_str(args, 0, "window.fetch")?;
    if let Some(fetch) = env.get("http_fetch_async") {
        if let Value::NativeFunction(f) = fetch {
            return f(args, env);
        }
    }
    Ok(Value::String(format!("fetch:{url}")))
}

fn expect_str(args: &[Value], i: usize, name: &str) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects a string")),
    }
}

pub fn browser_globals(env: &mut Environment) {
    env.set("document".into(), Value::Object(host_document_object()));
    env.set("window".into(), Value::Object(host_window_object()));
    env.set("navigator".into(), Value::Object(host_navigator_object()));
    env.set("host_paint".into(), Value::NativeFunction(host_paint_native));
    env.set("host_frame".into(), Value::NativeFunction(host_frame_native));
    env.set("host_mount".into(), Value::NativeFunction(host_mount_native));
    env.set("host_layer".into(), Value::String("host".into()));
}

fn host_paint_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(match crate::runtime::frame_buffer::last_frame_html() {
        Some(html) => Value::String(html),
        None => Value::Null,
    })
}

fn host_frame_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(match crate::runtime::frame_buffer::last_frame_text() {
        Some(text) => Value::String(text),
        None => Value::Null,
    })
}

fn host_mount_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let html = crate::runtime::frame_buffer::last_frame_html().ok_or("no frame to mount")?;
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(doc) = web_document() {
            if let Ok(el) = doc.get_element_by_id("kb-host-root") {
                if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                    html_el.set_inner_html(&extract_body(&html));
                    return Ok(Value::Bool(true));
                }
            }
            if let Ok(body) = doc.body() {
                if let Some(html_el) = body.dyn_ref::<web_sys::HtmlElement>() {
                    html_el.set_inner_html(&extract_body(&html));
                    return Ok(Value::Bool(true));
                }
            }
        }
    }
    Ok(Value::String(html))
}

fn extract_body(html: &str) -> String {
    if let Some(start) = html.find("<body") {
        if let Some(gt) = html[start..].find('>') {
            let content_start = start + gt + 1;
            if let Some(end) = html[content_start..].find("</body>") {
                return html[content_start..content_start + end].to_string();
            }
        }
    }
    html.to_string()
}
