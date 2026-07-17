//! DevTools — Console, Element Inspector, Debugger, Network, Profiler, Live edit (C9).

use crate::runtime::kabootar_browser::KabootarBrowser;
use crate::runtime::kabootar_dom::DomNode;
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use super::json_util::json_string;

const MAX_LOGS: usize = 1024;
const MAX_NETWORK: usize = 512;
const MAX_PROFILE_MARKS: usize = 1024;

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

#[derive(Clone, Debug)]
pub struct NetworkEntry {
    pub id: u64,
    pub method: String,
    pub url: String,
    pub status: i64,
    pub size: i64,
    pub duration_ms: f64,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct ProfileMark {
    pub name: String,
    pub t_ms: f64,
}

#[derive(Clone, Debug)]
pub struct ProfileMeasure {
    pub name: String,
    pub duration_ms: f64,
    pub start: String,
    pub end: String,
}

struct ProfileSession {
    label: String,
    started: Instant,
    marks: Vec<ProfileMark>,
    measures: Vec<ProfileMeasure>,
}

static LOGS: std::sync::OnceLock<Mutex<VecDeque<ConsoleEntry>>> = std::sync::OnceLock::new();
static BREAKPOINTS: std::sync::OnceLock<Mutex<HashSet<(String, u32)>>> = std::sync::OnceLock::new();
static SOURCE_MAPS: std::sync::OnceLock<Mutex<HashMap<String, String>>> = std::sync::OnceLock::new();
static NETWORK: std::sync::OnceLock<Mutex<VecDeque<NetworkEntry>>> = std::sync::OnceLock::new();
static NETWORK_NEXT: std::sync::OnceLock<Mutex<u64>> = std::sync::OnceLock::new();
static PROFILE: std::sync::OnceLock<Mutex<Option<ProfileSession>>> = std::sync::OnceLock::new();
static PROFILE_LAST: std::sync::OnceLock<
    Mutex<Option<(String, Vec<ProfileMark>, Vec<ProfileMeasure>, f64)>>,
> = std::sync::OnceLock::new();

fn log_store() -> &'static Mutex<VecDeque<ConsoleEntry>> {
    LOGS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn bp_store() -> &'static Mutex<HashSet<(String, u32)>> {
    BREAKPOINTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn map_store() -> &'static Mutex<HashMap<String, String>> {
    SOURCE_MAPS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn network_store() -> &'static Mutex<VecDeque<NetworkEntry>> {
    NETWORK.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn network_next() -> &'static Mutex<u64> {
    NETWORK_NEXT.get_or_init(|| Mutex::new(1))
}

fn profile_store() -> &'static Mutex<Option<ProfileSession>> {
    PROFILE.get_or_init(|| Mutex::new(None))
}

fn profile_last() -> &'static Mutex<Option<(String, Vec<ProfileMark>, Vec<ProfileMeasure>, f64)>> {
    PROFILE_LAST.get_or_init(|| Mutex::new(None))
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
    o.insert("attrs".into(), Value::Object(attrs));
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

fn network_json() -> String {
    let entries: Vec<String> = network_dump()
        .into_iter()
        .map(|e| {
            format!(
                "{{\"id\":{},\"method\":{},\"url\":{},\"status\":{},\"size\":{},\"duration_ms\":{},\"source\":{}}}",
                e.id,
                json_string(&e.method),
                json_string(&e.url),
                e.status,
                e.size,
                e.duration_ms,
                json_string(&e.source)
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

fn profiler_json() -> String {
    let active = profile_store()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.label.clone()));
    let last = profile_last().lock().ok().and_then(|g| g.clone());
    let (label, marks, measures, total_ms) = match last {
        Some((l, m, meas, t)) => (l, m, meas, t),
        None => (active.unwrap_or_default(), Vec::new(), Vec::new(), 0.0),
    };
    let marks_j: Vec<String> = marks
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"t_ms\":{}}}",
                json_string(&m.name),
                m.t_ms
            )
        })
        .collect();
    let meas_j: Vec<String> = measures
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"duration_ms\":{},\"start\":{},\"end\":{}}}",
                json_string(&m.name),
                m.duration_ms,
                json_string(&m.start),
                json_string(&m.end)
            )
        })
        .collect();
    let is_active = profile_store()
        .lock()
        .ok()
        .map(|g| g.is_some())
        .unwrap_or(false);
    format!(
        "{{\"label\":{},\"active\":{},\"total_ms\":{},\"marks\":[{}],\"measures\":[{}]}}",
        json_string(&label),
        if is_active { "true" } else { "false" },
        total_ms,
        marks_j.join(","),
        meas_j.join(",")
    )
}

/// JSON snapshot for `kabootar-shell.html` DevTools dock.
pub fn shell_snapshot_json(doc: Option<&DomNode>) -> String {
    let tree = doc.map(dom_tree_json).unwrap_or_else(|| "null".into());
    format!(
        "{{\"tree\":{tree},\"console\":{},\"breakpoints\":{},\"network\":{},\"profiler\":{}}}",
        console_json(),
        breakpoints_json(),
        network_json(),
        profiler_json()
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

// --- C9: Network panel ---

pub fn network_record(
    method: &str,
    url: &str,
    status: i64,
    size: i64,
    duration_ms: f64,
    source: &str,
) -> Result<NetworkEntry, String> {
    let id = {
        let mut n = network_next()
            .lock()
            .map_err(|_| "devtools network id lock".to_string())?;
        let id = *n;
        *n += 1;
        id
    };
    let entry = NetworkEntry {
        id,
        method: method.to_string(),
        url: url.to_string(),
        status,
        size,
        duration_ms,
        source: source.to_string(),
    };
    let mut q = network_store()
        .lock()
        .map_err(|_| "devtools network lock".to_string())?;
    if q.len() >= MAX_NETWORK {
        q.pop_front();
    }
    q.push_back(entry.clone());
    Ok(entry)
}

pub fn network_dump() -> Vec<NetworkEntry> {
    network_store()
        .lock()
        .map(|q| q.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn network_clear() -> bool {
    network_store()
        .lock()
        .map(|mut q| {
            q.clear();
            true
        })
        .unwrap_or(false)
}

// --- C9: Profiler ---

pub fn profile_start(label: &str) -> Result<bool, String> {
    let mut guard = profile_store()
        .lock()
        .map_err(|_| "devtools profile lock".to_string())?;
    *guard = Some(ProfileSession {
        label: if label.is_empty() {
            "default".into()
        } else {
            label.into()
        },
        started: Instant::now(),
        marks: Vec::new(),
        measures: Vec::new(),
    });
    Ok(true)
}

pub fn profile_mark(name: &str) -> Result<f64, String> {
    let mut guard = profile_store()
        .lock()
        .map_err(|_| "devtools profile lock".to_string())?;
    let session = guard
        .as_mut()
        .ok_or("devtools_profile_mark: no active profile (call devtools_profile_start)")?;
    let t_ms = session.started.elapsed().as_secs_f64() * 1000.0;
    if session.marks.len() >= MAX_PROFILE_MARKS {
        session.marks.remove(0);
    }
    session.marks.push(ProfileMark {
        name: name.to_string(),
        t_ms,
    });
    Ok(t_ms)
}

pub fn profile_measure(name: &str, start_mark: &str, end_mark: &str) -> Result<f64, String> {
    let mut guard = profile_store()
        .lock()
        .map_err(|_| "devtools profile lock".to_string())?;
    let session = guard
        .as_mut()
        .ok_or("devtools_profile_measure: no active profile")?;
    let start_t = session
        .marks
        .iter()
        .rev()
        .find(|m| m.name == start_mark)
        .map(|m| m.t_ms)
        .ok_or_else(|| format!("devtools_profile_measure: unknown start mark '{start_mark}'"))?;
    let end_t = if end_mark.is_empty() {
        session.started.elapsed().as_secs_f64() * 1000.0
    } else {
        session
            .marks
            .iter()
            .rev()
            .find(|m| m.name == end_mark)
            .map(|m| m.t_ms)
            .ok_or_else(|| format!("devtools_profile_measure: unknown end mark '{end_mark}'"))?
    };
    let duration_ms = (end_t - start_t).max(0.0);
    session.measures.push(ProfileMeasure {
        name: name.to_string(),
        duration_ms,
        start: start_mark.to_string(),
        end: if end_mark.is_empty() {
            "now".into()
        } else {
            end_mark.to_string()
        },
    });
    Ok(duration_ms)
}

pub fn profile_stop() -> Result<HashMap<String, Value>, String> {
    let session = {
        let mut guard = profile_store()
            .lock()
            .map_err(|_| "devtools profile lock".to_string())?;
        guard
            .take()
            .ok_or("devtools_profile_stop: no active profile")?
    };
    let total_ms = session.started.elapsed().as_secs_f64() * 1000.0;
    if let Ok(mut last) = profile_last().lock() {
        *last = Some((
            session.label.clone(),
            session.marks.clone(),
            session.measures.clone(),
            total_ms,
        ));
    }
    let mut o = HashMap::new();
    o.insert("label".into(), Value::String(session.label));
    o.insert("total_ms".into(), Value::Float(total_ms));
    o.insert(
        "marks".into(),
        Value::Array(
            session
                .marks
                .into_iter()
                .map(|m| {
                    let mut x = HashMap::new();
                    x.insert("name".into(), Value::String(m.name));
                    x.insert("t_ms".into(), Value::Float(m.t_ms));
                    Value::Object(x)
                })
                .collect(),
        ),
    );
    o.insert(
        "measures".into(),
        Value::Array(
            session
                .measures
                .into_iter()
                .map(|m| {
                    let mut x = HashMap::new();
                    x.insert("name".into(), Value::String(m.name));
                    x.insert("duration_ms".into(), Value::Float(m.duration_ms));
                    x.insert("start".into(), Value::String(m.start));
                    x.insert("end".into(), Value::String(m.end));
                    Value::Object(x)
                })
                .collect(),
        ),
    );
    Ok(o)
}

pub fn profile_dump() -> HashMap<String, Value> {
    if let Ok(guard) = profile_last().lock() {
        if let Some((label, marks, measures, total_ms)) = guard.as_ref() {
            let mut o = HashMap::new();
            o.insert("label".into(), Value::String(label.clone()));
            o.insert("total_ms".into(), Value::Float(*total_ms));
            o.insert(
                "marks".into(),
                Value::Array(
                    marks
                        .iter()
                        .map(|m| {
                            let mut x = HashMap::new();
                            x.insert("name".into(), Value::String(m.name.clone()));
                            x.insert("t_ms".into(), Value::Float(m.t_ms));
                            Value::Object(x)
                        })
                        .collect(),
                ),
            );
            o.insert(
                "measures".into(),
                Value::Array(
                    measures
                        .iter()
                        .map(|m| {
                            let mut x = HashMap::new();
                            x.insert("name".into(), Value::String(m.name.clone()));
                            x.insert("duration_ms".into(), Value::Float(m.duration_ms));
                            x.insert("start".into(), Value::String(m.start.clone()));
                            x.insert("end".into(), Value::String(m.end.clone()));
                            Value::Object(x)
                        })
                        .collect(),
                ),
            );
            return o;
        }
    }
    HashMap::new()
}

// --- C9: Live edit ---

pub fn live_edit_text(id: u64, text: &str) -> Result<bool, String> {
    crate::runtime::kabootar_dom::devtools_live_set_text(id, text)?;
    Ok(true)
}

pub fn live_edit_attr(id: u64, key: &str, value: &str) -> Result<bool, String> {
    crate::runtime::kabootar_dom::devtools_live_set_attr(id, key, value)?;
    Ok(true)
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("console".into(), "true".into());
    o.insert("inspector".into(), "true".into());
    o.insert("debugger".into(), "true".into());
    o.insert("network".into(), "true".into());
    o.insert("profiler".into(), "true".into());
    o.insert("live_edit".into(), "true".into());
    o.insert("kv8_hook".into(), "console.log".into());
    o.insert("phase".into(), "C9".into());
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
    o.insert(
        "network_count".into(),
        network_store()
            .lock()
            .map(|q| q.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
