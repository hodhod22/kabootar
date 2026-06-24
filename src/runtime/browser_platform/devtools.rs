//! DevTools — Console, Element Inspector, Debugger for Kabootar Browser.

use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kabootar_browser::KabootarBrowser;
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use super::json_util::json_string;

const MAX_LOGS: usize = 1024;

#[derive(Clone)]
pub struct ConsoleEntry {
    pub level: String,
    pub message: String,
    pub source: String,
}

#[derive(Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
}

static LOGS: std::sync::OnceLock<Mutex<VecDeque<ConsoleEntry>>> = std::sync::OnceLock::new();
static BREAKPOINTS: std::sync::OnceLock<Mutex<HashSet<(String, u32)>>> = std::sync::OnceLock::new();
static SOURCE_MAPS: std::sync::OnceLock<Mutex<HashMap<String, String>>> = std::sync::OnceLock::new();

fn log_store() -> &'static Mutex<VecDeque<ConsoleEntry>> {
    LOGS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn bp_store() -> &'static Mutex<HashSet<(String, u32)>> {
    BREAKPOINTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn map_store() -> &'static Mutex<HashMap<String, String>> {
    SOURCE_MAPS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn console_log(level: &str, message: &str, source: &str) {
    if let Ok(mut q) = log_store().lock() {
        if q.len() >= MAX_LOGS {
            q.pop_front();
        }
        q.push_back(ConsoleEntry {
            level: level.into(),
            message: message.into(),
            source: source.into(),
        });
    }
}

/// Called from Kv8 when `console.log(...)` is invoked.
pub fn kv8_console_log(args: &[String]) {
    let msg = args.join(" ");
    console_log("log", &msg, "kv8");
}

pub fn console_dump() -> Vec<ConsoleEntry> {
    log_store()
        .lock()
        .map(|q| q.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn console_clear() {
    if let Ok(mut q) = log_store().lock() {
        q.clear();
    }
}

pub fn inspect_node(node: &DomNode) -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("id".into(), node.id.to_string());
    o.insert("tag".into(), node.tag.clone());
    o.insert("text".into(), node.text.clone().unwrap_or_default());
    o.insert("children".into(), node.children.len().to_string());
    let attrs: Vec<String> = node
        .attributes
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    o.insert("attributes".into(), attrs.join(", "));
    o.insert("computed".into(), "kss-pending".into());
    o
}

/// Recursive DOM tree for Elements panel and `devtools_dom_tree()`.
pub fn dom_tree_value(node: &DomNode) -> Value {
    let mut attrs = HashMap::new();
    for (k, v) in &node.attributes {
        attrs.insert(k.clone(), Value::String(v.clone()));
    }
    let mut o = HashMap::new();
    o.insert("id".into(), Value::Number(node.id as i64));
    o.insert("tag".into(), Value::String(node.tag.clone()));
    if let Some(text) = &node.text {
        o.insert("text".into(), Value::String(text.clone()));
    }
    o.insert(
        "attrs".into(),
        Value::Object(attrs),
    );
    o.insert(
        "children".into(),
        Value::Array(node.children.iter().map(dom_tree_value).collect()),
    );
    Value::Object(o)
}

fn dom_tree_json(node: &DomNode) -> String {
    let mut parts = vec![
        format!("\"id\":{}", node.id),
        format!("\"tag\":{}", json_string(&node.tag)),
    ];
    if let Some(text) = &node.text {
        parts.push(format!("\"text\":{}", json_string(text)));
    }
    let attrs: Vec<String> = node
        .attributes
        .iter()
        .map(|(k, v)| format!("{}:{}", json_string(k), json_string(v)))
        .collect();
    parts.push(format!("\"attrs\":{{{}}}", attrs.join(",")));
    let children: Vec<String> = node.children.iter().map(dom_tree_json).collect();
    parts.push(format!("\"children\":[{}]", children.join(",")));
    format!("{{{}}}", parts.join(","))
}

fn console_json() -> String {
    let entries: Vec<String> = console_dump()
        .into_iter()
        .map(|e| {
            format!(
                "{{\"level\":{},\"message\":{},\"source\":{}}}",
                json_string(&e.level),
                json_string(&e.message),
                json_string(&e.source)
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

fn breakpoints_json() -> String {
    let entries: Vec<String> = debugger_breakpoints()
        .into_iter()
        .map(|b| {
            format!(
                "{{\"file\":{},\"line\":{}}}",
                json_string(&b.file),
                b.line
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

/// JSON snapshot for `kabootar-shell.html` DevTools dock.
pub fn shell_snapshot_json(doc: Option<&DomNode>) -> String {
    let tree = doc.map(dom_tree_json).unwrap_or_else(|| "null".into());
    format!(
        "{{\"tree\":{tree},\"console\":{},\"breakpoints\":{}}}",
        console_json(),
        breakpoints_json()
    )
}

pub fn shell_snapshot_from_env(env: &Environment) -> String {
    let doc = env.get("kbrowser").and_then(|v| match v {
        Value::KabootarBrowser(b) => b.active_document().ok(),
        _ => None,
    });
    shell_snapshot_json(doc.as_ref())
}

pub fn shell_snapshot_from_browser(browser: &KabootarBrowser) -> String {
    let doc = browser.active_document().ok();
    shell_snapshot_json(doc.as_ref())
}

pub fn debugger_breakpoint_set(file: &str, line: u32) -> bool {
    bp_store()
        .lock()
        .map(|mut s| s.insert((file.to_string(), line)))
        .unwrap_or(false)
}

pub fn debugger_breakpoint_clear(file: &str, line: u32) -> bool {
    bp_store()
        .lock()
        .map(|mut s| s.remove(&(file.to_string(), line)))
        .unwrap_or(false)
}

pub fn debugger_breakpoints() -> Vec<Breakpoint> {
    bp_store()
        .lock()
        .map(|s| {
            s.iter()
                .map(|(f, l)| Breakpoint {
                    file: f.clone(),
                    line: *l,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn debugger_should_break(file: &str, line: u32) -> bool {
    bp_store()
        .lock()
        .map(|s| s.contains(&(file.to_string(), line)))
        .unwrap_or(false)
}

pub fn source_map_register(generated: &str, original: &str) {
    if let Ok(mut m) = map_store().lock() {
        m.insert(generated.to_string(), original.to_string());
    }
}

pub fn source_map_resolve(generated: &str) -> Option<String> {
    map_store().lock().ok()?.get(generated).cloned()
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("console".into(), "true".into());
    o.insert("inspector".into(), "true".into());
    o.insert("debugger".into(), "true".into());
    o.insert("kv8_hook".into(), "console.log".into());
    o.insert("phase".into(), "v2.56".into());
    o.insert("elements_ui".into(), "kabootar-shell.html".into());
    o.insert(
        "log_count".into(),
        log_store()
            .lock()
            .map(|q| q.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o.insert(
        "breakpoints".into(),
        bp_store()
            .lock()
            .map(|s| s.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
