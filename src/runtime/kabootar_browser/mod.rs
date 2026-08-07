//! Kabootar Browser — Chrome-inspired viewport over Kabootar DOM (layer 2).
//!
//! Host Chrome maps to `document`/`window`; Kabootar apps use `kbrowser` + `kb_*`.
//! Navigation supports Kabootar OS VFS, host `file://`, and `http(s)://` (native).

mod host_nav;

use crate::runtime::events::{self, hit_test};
use crate::runtime::os::hotplug;
use crate::runtime::frame_buffer;
use crate::runtime::render::RenderLayer;
use crate::runtime::kabootar_dom::{assign_ids, DomNode};
use crate::runtime::kv8::Kv8Context;
use crate::runtime::kstyle::{parse_stylesheet, Stylesheet};
use crate::runtime::os::OsHandle;
use host_nav::{host_os_name, load_page, BrowserOsMode, os_info_map};
use crate::runtime::render::{frame_to_object, gpu_info_map, RenderEngine};
use crate::runtime::render::{active_backend, set_backend, RenderBackend};
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct BrowserTab {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub document: DomNode,
    // H6c delete-gate: tab/history session lives in Kab (`kbrowser/nav`).
    // Rust tab is render state only (document + url for load/paint).
    pub kv8_script: Option<String>,
    pub kv8_css: Option<String>,
    pub kv8_parsed_stylesheet: Option<Stylesheet>,
}

impl BrowserTab {
    fn new(id: u64, url: &str) -> Self {
        Self {
            id,
            title: "New Tab".into(),
            url: url.to_string(),
            document: default_home_document(url),
            kv8_script: None,
            kv8_css: None,
            kv8_parsed_stylesheet: None,
        }
    }

    fn navigate(&mut self, url: &str, os: Option<&OsHandle>, mode: BrowserOsMode) {
        self.url = url.to_string();
        self.reload_document(os, mode);
        self.title = title_from_url(url);
    }

    fn reload_document(&mut self, os: Option<&OsHandle>, mode: BrowserOsMode) {
        let page = load_page(&self.url, os, mode, default_home_document);
        self.document = page.document;
        self.kv8_script = page.kv8_script;
        self.kv8_css = page.kv8_css;
        self.kv8_parsed_stylesheet = page.kv8_parsed_stylesheet;
    }
}

fn default_home_document(url: &str) -> DomNode {
    let mut h1 = DomNode::element("h1");
    h1.append(DomNode::text_node("Kabootar Browser"));
    let mut p = DomNode::element("p");
    p.append(DomNode::text_node(&format!("Welcome — {url}")));
    let mut body = DomNode::element("body");
    body.set_attr("class", "kb-home");
    body.set_attr("style", "padding:24px;background:#292a2d;color:#e8eaed;");
    body.append(h1);
    body.append(p);
    let mut root = DomNode::element("html");
    root.append(body);
    assign_ids(&mut root);
    root
}

fn title_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

#[derive(Debug, Clone)]
pub struct KabootarBrowser {
    inner: Arc<Mutex<BrowserInner>>,
}

#[derive(Debug)]
struct BrowserInner {
    tabs: Vec<BrowserTab>,
    active: usize,
    next_id: u64,
    user_agent: String,
    viewport_w: f64,
    viewport_h: f64,
    device_pixel_ratio: f64,
    orientation: String,
    safe_top: f64,
    safe_right: f64,
    safe_bottom: f64,
    safe_left: f64,
    stylesheet: String,
    last_layers: Vec<RenderLayer>,
    os_mode: BrowserOsMode,
}

impl KabootarBrowser {
    pub fn new() -> Self {
        let mut inner = BrowserInner {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            user_agent: format!(
                "KabootarBrowser/{} (KHTML, like Chrome) Kabootar/{}",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_VERSION")
            ),
            viewport_w: 1280.0,
            viewport_h: 720.0,
            device_pixel_ratio: 1.0,
            orientation: "landscape".into(),
            safe_top: 0.0,
            safe_right: 0.0,
            safe_bottom: 0.0,
            safe_left: 0.0,
            stylesheet: default_chrome_theme_css(),
            last_layers: Vec::new(),
            os_mode: BrowserOsMode::Auto,
        };
        inner.tabs.push(BrowserTab::new(1, "kabootar://home"));
        inner.next_id = 2;
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    fn with_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut BrowserInner) -> Result<T, String>,
    {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| "Kabootar browser lock poisoned".to_string())?;
        f(&mut g)
    }

    pub fn navigate(&self, url: &str, os: Option<&OsHandle>) -> Result<(), String> {
        self.with_mut(|inner| {
            let mode = inner.os_mode;
            let tab = inner.tabs.get_mut(inner.active).ok_or("No active tab")?;
            tab.navigate(url, os, mode);
            Ok(())
        })
    }

    pub fn set_os_mode(&self, mode: BrowserOsMode) -> Result<(), String> {
        self.with_mut(|inner| {
            inner.os_mode = mode;
            Ok(())
        })
    }

    pub fn os_mode(&self) -> Result<BrowserOsMode, String> {
        self.with_mut(|inner| Ok(inner.os_mode))
    }

    pub fn reload(&self, os: Option<&OsHandle>) -> Result<(), String> {
        self.with_mut(|inner| {
            let mode = inner.os_mode;
            let tab = inner.tabs.get_mut(inner.active).ok_or("No active tab")?;
            tab.reload_document(os, mode);
            Ok(())
        })
    }

    pub fn location(&self) -> Result<String, String> {
        self.with_mut(|inner| {
            inner
                .tabs
                .get(inner.active)
                .map(|t| t.url.clone())
                .ok_or_else(|| "No active tab".into())
        })
    }

    pub fn active_document(&self) -> Result<DomNode, String> {
        self.with_mut(|inner| {
            inner
                .tabs
                .get(inner.active)
                .map(|t| t.document.clone())
                .ok_or_else(|| "No active tab".into())
        })
    }

    pub fn set_document(&self, node: DomNode) -> Result<(), String> {
        self.with_mut(|inner| {
            let tab = inner.tabs.get_mut(inner.active).ok_or("No active tab")?;
            tab.document = node;
            Ok(())
        })
    }

    pub fn run_kv8_script(&self, _os: Option<&OsHandle>) -> Result<Value, String> {
        self.with_mut(|inner| {
            let tab = inner.tabs.get_mut(inner.active).ok_or("No active tab")?;
            let script = tab.kv8_script.clone().unwrap_or_default();
            if script.is_empty() {
                return Ok(Value::Null);
            }
            let ctx = Kv8Context::default();
            ctx.with_mut(|c| {
                c.document.root = tab.document.clone();
                if let Some(css) = &tab.kv8_css {
                    c.css_text = css.clone();
                    c.stylesheet = parse_stylesheet(css);
                }
                Ok(())
            })?;
            crate::runtime::kv8::eval_script(&ctx, &script)?;
            crate::runtime::browser_platform::inject_on_navigate(&tab.url, &ctx);
            tab.document = ctx.root_dom()?;
            Ok(Value::Bool(true))
        })
    }

    pub fn paint(&self, os: Option<&OsHandle>) -> Result<HashMap<String, Value>, String> {
        self.with_mut(|inner| {
            let tab = inner.tabs.get(inner.active).ok_or("No active tab")?;
            let mut engine = RenderEngine::with_viewport(inner.viewport_w, inner.viewport_h);
            let mut sheet = parse_stylesheet(&inner.stylesheet);
            if let Some(extra) = &tab.kv8_parsed_stylesheet {
                sheet.rules.extend(extra.rules.clone());
            } else if let Some(extra) = &tab.kv8_css {
                let parsed = parse_stylesheet(extra);
                sheet.rules.extend(parsed.rules);
            }
            engine.set_stylesheet(sheet);
            let frame = engine.compose(&tab.document);
            inner.last_layers = frame.layers.clone();
            frame_buffer::publish_frame(frame.clone());
            if let Some(handle) = os {
                if let Ok(wins) = handle.window_list() {
                    for w in wins {
                        if w.browser_tab_id == Some(tab.id) {
                            let _ = handle.display_present(w.id, frame.pixels_rgba.len());
                        }
                    }
                }
            }
            Ok(frame_to_object(&frame))
        })
    }

    pub fn set_viewport(&self, w: f64, h: f64) -> Result<(), String> {
        self.set_viewport_ex(w, h, None, None).map(|_| ())
    }

    pub fn set_viewport_ex(
        &self,
        w: f64,
        h: f64,
        dpr: Option<f64>,
        orientation: Option<&str>,
    ) -> Result<HashMap<String, Value>, String> {
        self.with_mut(|inner| {
            inner.viewport_w = w;
            inner.viewport_h = h;
            if let Some(d) = dpr {
                if d > 0.0 {
                    inner.device_pixel_ratio = d;
                }
            }
            if let Some(o) = orientation {
                if o == "portrait" || o == "landscape" {
                    inner.orientation = o.into();
                }
            } else {
                inner.orientation = if h >= w {
                    "portrait".into()
                } else {
                    "landscape".into()
                };
            }
            Ok(viewport_map(inner))
        })
    }

    pub fn viewport_info(&self) -> Result<HashMap<String, Value>, String> {
        self.with_mut(|inner| Ok(viewport_map(inner)))
    }

    pub fn set_safe_area(&self, top: f64, right: f64, bottom: f64, left: f64) -> Result<HashMap<String, Value>, String> {
        self.with_mut(|inner| {
            inner.safe_top = top.max(0.0);
            inner.safe_right = right.max(0.0);
            inner.safe_bottom = bottom.max(0.0);
            inner.safe_left = left.max(0.0);
            Ok(safe_area_map(inner))
        })
    }

    pub fn safe_area_info(&self) -> Result<HashMap<String, Value>, String> {
        self.with_mut(|inner| Ok(safe_area_map(inner)))
    }

    pub fn set_theme_css(&self, css: &str) -> Result<(), String> {
        self.with_mut(|inner| {
            inner.stylesheet = css.to_string();
            Ok(())
        })
    }

    pub fn active_tab_id(&self) -> Result<u64, String> {
        self.with_mut(|inner| {
            inner
                .tabs
                .get(inner.active)
                .map(|t| t.id)
                .ok_or_else(|| "No active tab".into())
        })
    }

    pub fn user_agent(&self) -> Result<String, String> {
        self.with_mut(|inner| Ok(inner.user_agent.clone()))
    }

    pub fn click_at(&self, x: f64, y: f64) -> Result<Option<String>, String> {
        self.dispatch_pointer(x, y, "click")
    }

    pub fn touch_at(&self, x: f64, y: f64, phase: &str) -> Result<Option<String>, String> {
        let event_type = match phase {
            "start" | "touchstart" => "touchstart",
            "move" | "touchmove" => "touchmove",
            "end" | "touchend" | "" => "touchend",
            other => other,
        };
        self.dispatch_pointer(x, y, event_type)
    }

    fn dispatch_pointer(&self, x: f64, y: f64, event_type: &str) -> Result<Option<String>, String> {
        self.with_mut(|inner| {
            let node_id = hit_test(&inner.last_layers, x, y);
            let Some(id) = node_id else {
                return Ok(None);
            };
            // Prefer live registry + parent bubble so post-mount `kdom_on` works.
            let Some((target_id, handler)) =
                crate::runtime::kabootar_dom::resolve_listener(id, event_type)
            else {
                return Ok(None);
            };
            events::enqueue(events::KabootarEvent {
                node_id: target_id,
                event_type: event_type.into(),
                handler: handler.clone(),
                x,
                y,
            });
            Ok(Some(handler))
        })
    }
}

fn viewport_map(inner: &BrowserInner) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("width".into(), Value::Float(inner.viewport_w));
    m.insert("height".into(), Value::Float(inner.viewport_h));
    m.insert("dpr".into(), Value::Float(inner.device_pixel_ratio));
    m.insert("orientation".into(), Value::String(inner.orientation.clone()));
    m
}

fn safe_area_map(inner: &BrowserInner) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("top".into(), Value::Float(inner.safe_top));
    m.insert("right".into(), Value::Float(inner.safe_right));
    m.insert("bottom".into(), Value::Float(inner.safe_bottom));
    m.insert("left".into(), Value::Float(inner.safe_left));
    m
}

fn default_chrome_theme_css() -> String {
    r#"
body { display: flex; flex-direction: column; background: #292a2d; color: #e8eaed; padding: 16px; }
h1 { font-size: 28px; font-weight: 600; color: #8ab4f8; margin: 8px 0; }
h2 { font-size: 22px; color: #e8eaed; }
p { font-size: 16px; line-height: 1.5; }
.kb-home { display: flex; flex-direction: column; gap: 12px; }
button { background: #8ab4f8; color: #202124; padding: 8px 16px; border-radius: 8px; }
.card { background: #35363a; padding: 16px; border-radius: 12px; margin: 8px 0; }
"#
    .to_string()
}

fn get_browser(env: &Environment) -> Result<KabootarBrowser, String> {
    let v = env.get("kbrowser").ok_or("kbrowser not available")?;
    let Value::KabootarBrowser(b) = v else {
        return Err("kbrowser handle expected".into());
    };
    Ok(b)
}

fn get_os_opt(env: &Environment) -> Option<OsHandle> {
    env.get("os").and_then(|v| match v {
        Value::OsHandle(h) => Some(h),
        _ => None,
    })
}

fn kb_navigate_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let url = expect_str(args, 0, "kb_navigate()")?;
    get_browser(env)?.navigate(&url, get_os_opt(env).as_ref())?;
    Ok(Value::Null)
}

fn kb_run_kv8_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_browser(env)?.run_kv8_script(get_os_opt(env).as_ref())
}

fn kb_location_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(get_browser(env)?.location()?))
}

fn kb_render_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let frame = get_browser(env)?.paint(get_os_opt(env).as_ref())?;
    Ok(match frame.get("html") {
        Some(Value::String(s)) => Value::String(s.clone()),
        _ => Value::String(String::new()),
    })
}

fn kb_paint_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::from_object(
        get_browser(env)?.paint(get_os_opt(env).as_ref())?,
    ))
}

fn kb_composite_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let browser = get_browser(env)?;
    let os = get_os_opt(env);
    let frame = browser.paint(os.as_ref())?;
    let tab_id = browser.active_tab_id()?;
    let mut out = frame;
    out.insert("tab".into(), Value::Number(tab_id as i64));
    out.insert("url".into(), Value::String(browser.location()?));
    out.insert("os_mode".into(), Value::String(browser.os_mode()?.as_str().into()));
    out.insert("host_os".into(), Value::String(host_os_name().into()));
    if let Some(os) = os {
        let windows = os.window_list()?;
        out.insert(
            "windows".into(), Value::from_array(
                windows
                    .into_iter()
                    .map(|w| {
                        let mut m = HashMap::new();
                        m.insert("id".into(), Value::Number(w.id as i64));
                        m.insert("title".into(), Value::String(w.title));
                        m.insert("width".into(), Value::Number(w.width));
                        m.insert("height".into(), Value::Number(w.height));
                        m.insert("focused".into(), Value::Bool(w.focused));
                        if let Some(tid) = w.browser_tab_id {
                            m.insert("tab".into(), Value::Number(tid as i64));
                        }
                        Value::from_object(m)
                    })
                    .collect(),
            ),
        );
    }
    Ok(Value::from_object(out))
}

fn kb_host_sync_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let frame = get_browser(env)?.paint(get_os_opt(env).as_ref())?;
    if let Some(Value::String(html)) = frame.get("html") {
        env.set("__host_frame".into(), Value::String(html.clone()));
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(false))
}

fn kb_viewport_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let w = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(1280.0);
    let h = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(720.0);
    let dpr = args.get(2).and_then(|v| match v {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    });
    let orientation = args.get(3).and_then(|v| match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    });
    Ok(Value::from_object(
        get_browser(env)?.set_viewport_ex(w, h, dpr, orientation)?,
    ))
}

fn kb_safe_area_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::from_object(get_browser(env)?.safe_area_info()?));
    }
    let num = |i: usize| {
        args.get(i).and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
    };
    Ok(Value::from_object(get_browser(env)?.set_safe_area(
        num(0).unwrap_or(0.0),
        num(1).unwrap_or(0.0),
        num(2).unwrap_or(0.0),
        num(3).unwrap_or(0.0),
    )?))
}

fn kb_theme_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let css = expect_str(args, 0, "kb_theme()")?;
    get_browser(env)?.set_theme_css(&css)?;
    Ok(Value::Null)
}

fn kb_reload_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let os = get_os_opt(env);
    get_browser(env)?.reload(os.as_ref())?;
    Ok(Value::Bool(true))
}

fn kb_os_mode_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(get_browser(env)?.os_mode()?.as_str().into()))
}

fn kb_set_os_mode_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mode = expect_str(args, 0, "kb_set_os_mode()")?;
    let parsed = BrowserOsMode::from_str(&mode)
        .ok_or("kb_set_os_mode expects auto, kabootar, or host")?;
    get_browser(env)?.set_os_mode(parsed)?;
    Ok(Value::String(parsed.as_str().into()))
}

fn kb_os_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let browser = get_browser(env)?;
    Ok(Value::from_object(os_info_map(
        get_os_opt(env).as_ref(),
        browser.os_mode()?,
    )))
}

fn kb_sync_platform_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let layer = env
        .get("__platform")
        .and_then(|v| match v {
            Value::Object(m) => m.get("active").and_then(|a| match a {
                Value::String(s) => Some(s.clone()),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or_else(|| "hybrid".into());
    let mode = match layer.as_str() {
        "host" => BrowserOsMode::Host,
        "kabootar" => BrowserOsMode::Kabootar,
        _ => BrowserOsMode::Auto,
    };
    get_browser(env)?.set_os_mode(mode)?;
    let mut m = HashMap::new();
    m.insert("mode".into(), Value::String(mode.as_str().into()));
    m.insert("layer".into(), Value::String(layer));
    m.insert(
        "host_os".into(),
        Value::String(host_nav::host_os_name().into()),
    );
    m.insert(
        "schemes".into(), Value::from_array(vec![
            Value::String("kabootar://".into()),
            Value::String("file://".into()),
            Value::String("http://".into()),
            Value::String("https://".into()),
        ]),
    );
    Ok(Value::from_object(m))
}

fn kb_mount_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let node = args.first().ok_or("kb_mount() expects a DOM node")?;
    let Value::KabootarDom(dom) = node else {
        return Err("kb_mount() expects a Kabootar DOM node".into());
    };
    crate::runtime::kabootar_dom::live_register_tree(dom);
    get_browser(env)?.set_document(dom.clone())?;
    Ok(Value::Null)
}

fn kb_user_agent_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(get_browser(env)?.user_agent()?))
}

fn kb_click_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = args.first().and_then(|v| match v { Value::Number(n) => Some(*n as f64), Value::Float(f) => Some(*f), _ => None }).unwrap_or(0.0);
    let y = args.get(1).and_then(|v| match v { Value::Number(n) => Some(*n as f64), Value::Float(f) => Some(*f), _ => None }).unwrap_or(0.0);
    Ok(match get_browser(env)?.click_at(x, y)? {
        Some(h) => Value::String(h),
        None => Value::Null,
    })
}

fn kb_touch_at_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let x = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    let y = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    let phase = args
        .get(2)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("end");
    Ok(match get_browser(env)?.touch_at(x, y, phase)? {
        Some(h) => Value::String(h),
        None => Value::Null,
    })
}

fn kb_poll_events_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let events = events::drain_events();
    Ok(Value::from_array(
        events
            .into_iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("node".into(), Value::Number(e.node_id as i64));
                m.insert("type".into(), Value::String(e.event_type));
                m.insert("handler".into(), Value::String(e.handler));
                m.insert("x".into(), Value::Float(e.x));
                m.insert("y".into(), Value::Float(e.y));
                Value::from_object(m)
            })
            .collect(),
    ))
}

fn kb_poll_hotplug_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::from_array(
        hotplug::drain()
            .into_iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("action".into(), Value::String(e.action));
                m.insert("device_id".into(), Value::String(e.device_id));
                m.insert("kind".into(), Value::String(e.kind));
                m.insert("name".into(), Value::String(e.name));
                m.insert("vendor".into(), Value::String(e.vendor));
                Value::from_object(m)
            })
            .collect(),
    ))
}

fn kb_pixels_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(match frame_buffer::last_frame_pixels() {
        Some((w, h, px)) => {
            let mut m = HashMap::new();
            m.insert("width".into(), Value::Number(w));
            m.insert("height".into(), Value::Number(h));
            m.insert("bytes".into(), Value::Number(px.len() as i64));
            if let Some(frame) = frame_buffer::last_frame() {
                m.insert("backend".into(), Value::String(frame.backend));
                if let Some(gpu) = frame.gpu_handle {
                    m.insert("gpu".into(), Value::Number(gpu as i64));
                }
            }
            Value::from_object(m)
        }
        None => Value::Null,
    })
}

fn kb_backend_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(active_backend().as_str().into()))
}

fn kb_set_backend_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = expect_str(args, 0, "kb_set_backend")?;
    let backend = RenderBackend::from_str(&name).ok_or_else(|| format!("unknown backend: {name}"))?;
    set_backend(backend);
    Ok(Value::String(active_backend().as_str().into()))
}

fn kb_gpu_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::from_object(
        gpu_info_map()
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect(),
    ))
}

fn expect_str(args: &[Value], i: usize, name: &str) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects a string")),
    }
}

pub fn kabootar_browser_globals(env: &mut Environment) {
    env.set("kbrowser".into(), Value::KabootarBrowser(KabootarBrowser::new()));
    env.set("kb_navigate".into(), Value::NativeFunction(kb_navigate_native));
    env.set("kb_run_kv8".into(), Value::NativeFunction(kb_run_kv8_native));
    env.set("kb_location".into(), Value::NativeFunction(kb_location_native));
    env.set("kb_render".into(), Value::NativeFunction(kb_render_native));
    env.set("kb_paint".into(), Value::NativeFunction(kb_paint_native));
    env.set("kb_composite".into(), Value::NativeFunction(kb_composite_native));
    env.set("kb_host_sync".into(), Value::NativeFunction(kb_host_sync_native));
    env.set("kb_viewport".into(), Value::NativeFunction(kb_viewport_native));
    env.set("kb_safe_area".into(), Value::NativeFunction(kb_safe_area_native));
    env.set("kb_theme".into(), Value::NativeFunction(kb_theme_native));
    env.set("kb_reload".into(), Value::NativeFunction(kb_reload_native));
    env.set("kb_os_mode".into(), Value::NativeFunction(kb_os_mode_native));
    env.set("kb_set_os_mode".into(), Value::NativeFunction(kb_set_os_mode_native));
    env.set("kb_os_info".into(), Value::NativeFunction(kb_os_info_native));
    env.set("kb_sync_platform".into(), Value::NativeFunction(kb_sync_platform_native));
    env.set("kb_mount".into(), Value::NativeFunction(kb_mount_native));
    env.set("kb_user_agent".into(), Value::NativeFunction(kb_user_agent_native));
    env.set("kb_click".into(), Value::NativeFunction(kb_click_native));
    env.set("kb_touch_at".into(), Value::NativeFunction(kb_touch_at_native));
    env.set("kb_poll_events".into(), Value::NativeFunction(kb_poll_events_native));
    env.set("kb_poll_hotplug".into(), Value::NativeFunction(kb_poll_hotplug_native));
    env.set("kb_pixels".into(), Value::NativeFunction(kb_pixels_native));
    env.set("kb_backend".into(), Value::NativeFunction(kb_backend_native));
    env.set("kb_set_backend".into(), Value::NativeFunction(kb_set_backend_native));
    env.set("kb_gpu_info".into(), Value::NativeFunction(kb_gpu_info_native));
}
