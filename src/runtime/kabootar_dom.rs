//! Kabootar's native DOM (layer 2) — mirrors host DOM APIs but uses KML/Kabootar types.

use crate::kml::{parse_kml, render_kml};
use crate::runtime::kstyle::{parse_stylesheet, Stylesheet};
use crate::runtime::render::{frame_to_object, RenderEngine};
use crate::runtime::render::{layout_text, measure_text, text_layout_to_object, TextStyle, WhiteSpace};
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NODE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct MutationRecord {
    pub kind: String,
    pub target_id: u64,
    pub attribute_name: Option<String>,
    pub added_node_id: Option<u64>,
    pub removed_node_id: Option<u64>,
}

#[derive(Clone)]
struct MutationObserverEntry {
    id: u64,
    callback: Value,
    target_id: Option<u64>,
    child_list: bool,
    attributes: bool,
    connected: bool,
    pending: Vec<MutationRecord>,
}

static OBSERVER_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static MUTATION_RECORDS: RefCell<Vec<MutationRecord>> = RefCell::new(Vec::new());
    static MUTATION_OBSERVERS: RefCell<Vec<MutationObserverEntry>> = RefCell::new(Vec::new());
    /// Live Dom nodes by id — lets `.kab` store numeric ids instead of KabootarDom values.
    static LIVE_NODES: RefCell<HashMap<u64, DomNode>> = RefCell::new(HashMap::new());
    /// child id → parent id (for propagating child patches up to paintable parents).
    static LIVE_PARENTS: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

fn live_upsert(node: &DomNode) {
    LIVE_NODES.with(|m| {
        LIVE_PARENTS.with(|parents| {
            let mut map = m.borrow_mut();
            let mut parents = parents.borrow_mut();
            fn walk(
                map: &mut HashMap<u64, DomNode>,
                parents: &mut HashMap<u64, u64>,
                n: &DomNode,
                parent_id: Option<u64>,
            ) {
                if let Some(pid) = parent_id {
                    parents.insert(n.id, pid);
                }
                map.insert(n.id, n.clone());
                for child in &n.children {
                    walk(map, parents, child, Some(n.id));
                }
            }
            walk(&mut map, &mut parents, node, None);
        });
    });
}

/// After mutating a live child, refresh ancestor snapshots so paint(parent) sees new text/attrs.
fn live_propagate_to_ancestors(child_id: u64) {
    LIVE_NODES.with(|m| {
        LIVE_PARENTS.with(|parents| {
            let mut map = m.borrow_mut();
            let parents = parents.borrow();
            let mut current = child_id;
            while let Some(&pid) = parents.get(&current) {
                let Some(child) = map.get(&current).cloned() else {
                    break;
                };
                let Some(mut parent) = map.get(&pid).cloned() else {
                    break;
                };
                let mut replaced = false;
                for slot in &mut parent.children {
                    if slot.id == current {
                        *slot = child;
                        replaced = true;
                        break;
                    }
                }
                if !replaced {
                    break;
                }
                map.insert(pid, parent);
                current = pid;
            }
        });
    });
}

fn live_resolve(node: DomNode) -> DomNode {
    LIVE_NODES.with(|m| {
        m.borrow()
            .get(&node.id)
            .cloned()
            .unwrap_or(node)
    })
}

fn live_get(id: u64) -> Option<DomNode> {
    LIVE_NODES.with(|m| m.borrow().get(&id).cloned())
}


pub fn record_child_list_mutation(parent_id: u64, added_id: u64) {
    let record = MutationRecord {
        kind: "childList".into(),
        target_id: parent_id,
        attribute_name: None,
        added_node_id: Some(added_id),
        removed_node_id: None,
    };
    push_mutation_record(record);
}

pub fn record_child_removed_mutation(parent_id: u64, removed_id: u64) {
    let record = MutationRecord {
        kind: "childList".into(),
        target_id: parent_id,
        attribute_name: None,
        added_node_id: None,
        removed_node_id: Some(removed_id),
    };
    push_mutation_record(record);
}

pub fn record_attribute_mutation(target_id: u64, attr: &str) {
    let record = MutationRecord {
        kind: "attributes".into(),
        target_id,
        attribute_name: Some(attr.to_string()),
        added_node_id: None,
        removed_node_id: None,
    };
    push_mutation_record(record);
}

fn push_mutation_record(record: MutationRecord) {
    MUTATION_RECORDS.with(|r| {
        r.borrow_mut().push(record.clone());
    });
    MUTATION_OBSERVERS.with(|obs| {
        for entry in obs.borrow_mut().iter_mut() {
            if !entry.connected {
                continue;
            }
            let Some(target) = entry.target_id else {
                continue;
            };
            if target != record.target_id {
                continue;
            }
            let match_kind = match record.kind.as_str() {
                "childList" => entry.child_list,
                "attributes" => entry.attributes,
                _ => false,
            };
            if match_kind {
                entry.pending.push(record.clone());
            }
        }
    });
}

fn take_mutation_records() -> Vec<MutationRecord> {
    MUTATION_RECORDS.with(|r| r.borrow_mut().drain(..).collect())
}

fn deliver_mutation_observers(env: &mut Environment) -> Result<(), String> {
    let batches: Vec<(Value, Vec<MutationRecord>)> = MUTATION_OBSERVERS.with(|obs| {
        obs.borrow_mut()
            .iter_mut()
            .filter(|e| e.connected && !e.pending.is_empty())
            .map(|e| (e.callback.clone(), std::mem::take(&mut e.pending)))
            .collect()
    });
    for (callback, records) in batches {
        let arr = Value::Array(records.iter().map(mutation_record_to_value).collect());
        crate::bytecode::call_value(callback, vec![arr], &[], &[], &[], &[], env)?;
    }
    Ok(())
}

pub fn next_node_id() -> u64 {
    NODE_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn assign_ids(node: &mut DomNode) {
    if node.id == 0 {
        node.id = next_node_id();
    }
    for child in &mut node.children {
        assign_ids(child);
    }
}

#[derive(Debug, Clone)]
pub struct DomNode {
    pub id: u64,
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<DomNode>,
    pub text: Option<String>,
    pub listeners: HashMap<String, String>,
}

impl DomNode {
    pub fn element(tag: impl Into<String>) -> Self {
        Self {
            id: next_node_id(),
            tag: tag.into(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: None,
            listeners: HashMap::new(),
        }
    }

    pub fn text_node(text: impl Into<String>) -> Self {
        Self {
            id: next_node_id(),
            tag: "#text".into(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: Some(text.into()),
            listeners: HashMap::new(),
        }
    }

    pub fn set_attr(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    pub fn get_attr(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    pub fn append(&mut self, child: DomNode) {
        self.children.push(child);
    }

    pub fn on(&mut self, event: &str, handler: &str) {
        self.listeners.insert(event.to_string(), handler.to_string());
    }

    pub fn query_tag(&self, tag: &str) -> Option<&DomNode> {
        if self.tag == tag {
            return Some(self);
        }
        for c in &self.children {
            if let Some(found) = c.query_tag(tag) {
                return Some(found);
            }
        }
        None
    }

    pub fn query_id(&self, id: u64) -> Option<&DomNode> {
        if self.id == id {
            return Some(self);
        }
        for c in &self.children {
            if let Some(found) = c.query_id(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn matches_selector(&self, selector: &str) -> bool {
        let selector = selector.trim();
        if selector.is_empty() {
            return false;
        }
        // :not(inner) — optional base before :not
        if let Some(not_at) = selector.find(":not(") {
            let base = selector[..not_at].trim();
            let rest = &selector[not_at + 5..];
            let end = rest.find(')').unwrap_or(rest.len());
            let inner = rest[..end].trim();
            if self.matches_selector(inner) {
                return false;
            }
            if base.is_empty() {
                return true;
            }
            return self.matches_selector(base);
        }
        // Attribute: [name], [name=value], tag[name], tag[name=value]
        if let Some(bracket) = selector.find('[') {
            let prefix = selector[..bracket].trim();
            let rest = &selector[bracket..];
            if !rest.ends_with(']') {
                return false;
            }
            let inner = &rest[1..rest.len() - 1];
            let attr_ok = if let Some((name, raw_val)) = inner.split_once('=') {
                let want = raw_val.trim().trim_matches(|c| c == '"' || c == '\'');
                self.get_attr(name.trim()).is_some_and(|v| v == want)
            } else {
                self.get_attr(inner.trim()).is_some()
            };
            if !attr_ok {
                return false;
            }
            if prefix.is_empty() {
                return true;
            }
            return self.matches_selector_simple(prefix);
        }
        self.matches_selector_simple(selector)
    }

    fn matches_selector_simple(&self, selector: &str) -> bool {
        let selector = selector.trim();
        if selector.is_empty() {
            return false;
        }
        if let Some(id) = selector.strip_prefix('#') {
            return self.get_attr("id").is_some_and(|v| v == id);
        }
        if let Some(class) = selector.strip_prefix('.') {
            return self
                .get_attr("class")
                .is_some_and(|v| v.split_whitespace().any(|c| c == class));
        }
        if let Some((tag, rest)) = selector.split_once('#') {
            if self.tag != tag {
                return false;
            }
            return self.get_attr("id").is_some_and(|v| v == rest);
        }
        if let Some((tag, rest)) = selector.split_once('.') {
            if self.tag != tag {
                return false;
            }
            return self
                .get_attr("class")
                .is_some_and(|v| v.split_whitespace().any(|c| c == rest));
        }
        self.tag == selector
    }

    pub fn query_selector<'a>(&'a self, selector: &str) -> Option<&'a DomNode> {
        let selector = selector.trim();
        // Comma lists: first match wins.
        if selector.contains(',') {
            for part in selector.split(',') {
                if let Some(found) = self.query_selector(part.trim()) {
                    return Some(found);
                }
            }
            return None;
        }
        // Adjacent sibling: "div + span"
        if selector.contains('+') && !selector.contains('>') {
            let parts: Vec<&str> = selector
                .split('+')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() == 2 {
                return self.query_selector_adjacent(parts[0], parts[1]);
            }
        }
        // General sibling: "div ~ span"
        if selector.contains('~') && !selector.contains('>') && !selector.contains('+') {
            let parts: Vec<&str> = selector
                .split('~')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() == 2 {
                return self.query_selector_sibling(parts[0], parts[1]);
            }
        }
        // Child combinator: "div > span"
        if selector.contains('>') {
            let parts: Vec<&str> = selector
                .split('>')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            return self.query_selector_child(&parts);
        }
        let parts: Vec<&str> = selector.split_whitespace().filter(|s| !s.is_empty()).collect();
        match parts.as_slice() {
            [] => None,
            [one] => {
                if self.matches_selector(one) {
                    return Some(self);
                }
                for child in &self.children {
                    if let Some(found) = child.query_selector(one) {
                        return Some(found);
                    }
                }
                None
            }
            _ => self.query_selector_descendant(&parts),
        }
    }

    fn query_selector_adjacent<'a>(&'a self, left: &str, right: &str) -> Option<&'a DomNode> {
        for i in 1..self.children.len() {
            if self.children[i - 1].matches_selector(left) && self.children[i].matches_selector(right)
            {
                return Some(&self.children[i]);
            }
        }
        for child in &self.children {
            if let Some(found) = child.query_selector_adjacent(left, right) {
                return Some(found);
            }
        }
        None
    }

    fn query_selector_sibling<'a>(&'a self, left: &str, right: &str) -> Option<&'a DomNode> {
        let mut seen_left = false;
        for child in &self.children {
            if child.matches_selector(left) {
                seen_left = true;
            } else if seen_left && child.matches_selector(right) {
                return Some(child);
            }
        }
        for child in &self.children {
            if let Some(found) = child.query_selector_sibling(left, right) {
                return Some(found);
            }
        }
        None
    }

    fn query_selector_child<'a>(&'a self, parts: &[&str]) -> Option<&'a DomNode> {
        if parts.is_empty() {
            return None;
        }
        if parts.len() == 1 {
            return self.query_selector(parts[0]);
        }
        // Search for ancestor matching parts[0], then only direct children for the rest.
        if self.matches_selector(parts[0]) {
            for child in &self.children {
                if let Some(found) = child.query_selector_child_from(&parts[1..]) {
                    return Some(found);
                }
            }
        }
        for child in &self.children {
            if let Some(found) = child.query_selector_child(parts) {
                return Some(found);
            }
        }
        None
    }

    /// `self` must match parts[0]; only direct-child steps remain.
    fn query_selector_child_from<'a>(&'a self, parts: &[&str]) -> Option<&'a DomNode> {
        if parts.is_empty() {
            return Some(self);
        }
        if parts.len() == 1 {
            if self.matches_selector(parts[0]) {
                return Some(self);
            }
            return None;
        }
        if !self.matches_selector(parts[0]) {
            return None;
        }
        for child in &self.children {
            if let Some(found) = child.query_selector_child_from(&parts[1..]) {
                return Some(found);
            }
        }
        None
    }

    fn query_selector_descendant<'a>(&'a self, parts: &[&str]) -> Option<&'a DomNode> {
        if parts.len() == 1 {
            return self.query_selector(parts[0]);
        }
        if self.matches_selector(parts[0]) {
            for child in &self.children {
                if let Some(found) = child.query_selector_descendant(&parts[1..]) {
                    return Some(found);
                }
            }
        }
        for child in &self.children {
            if let Some(found) = child.query_selector_descendant(parts) {
                return Some(found);
            }
        }
        None
    }

    pub fn query_selector_all<'a>(&'a self, selector: &str) -> Vec<&'a DomNode> {
        let mut out = Vec::new();
        self.collect_selector_matches(selector, &mut out);
        out
    }

    fn collect_selector_matches<'a>(&'a self, selector: &str, out: &mut Vec<&'a DomNode>) {
        let selector = selector.trim();
        if selector.contains(',') {
            for part in selector.split(',') {
                self.collect_selector_matches(part.trim(), out);
            }
            return;
        }
        // Reuse query_selector walk for combinators; collect all via DFS for simple selectors.
        if selector.contains('>') || selector.contains('+') || selector.contains('~') {
            if let Some(found) = self.query_selector(selector) {
                if !out.iter().any(|n| n.id == found.id) {
                    out.push(found);
                }
            }
            for child in &self.children {
                child.collect_selector_matches(selector, out);
            }
            return;
        }
        let parts: Vec<&str> = selector.split_whitespace().filter(|s| !s.is_empty()).collect();
        if parts.len() <= 1 {
            let single = parts.first().copied().unwrap_or(selector);
            if self.matches_selector(single) {
                out.push(self);
            }
            for child in &self.children {
                child.collect_selector_matches(selector, out);
            }
            return;
        }
        if self.matches_selector(parts[0]) {
            for child in &self.children {
                child.collect_selector_descendant_matches(&parts[1..], out);
            }
        }
        for child in &self.children {
            child.collect_selector_matches(selector, out);
        }
    }

    fn collect_selector_descendant_matches<'a>(
        &'a self,
        parts: &[&str],
        out: &mut Vec<&'a DomNode>,
    ) {
        if parts.is_empty() {
            return;
        }
        if parts.len() == 1 {
            if self.matches_selector(parts[0]) {
                out.push(self);
            }
            for child in &self.children {
                child.collect_selector_descendant_matches(parts, out);
            }
            return;
        }
        if self.matches_selector(parts[0]) {
            for child in &self.children {
                child.collect_selector_descendant_matches(&parts[1..], out);
            }
        }
        for child in &self.children {
            child.collect_selector_descendant_matches(parts, out);
        }
    }

    /// Attach an event listener to the first node matching `tag` (mutates tree in place).
    pub fn listen_on_tag(&mut self, tag: &str, event: &str, handler: &str) -> bool {
        if self.tag == tag {
            self.on(event, handler);
            return true;
        }
        for child in &mut self.children {
            if child.listen_on_tag(tag, event, handler) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct KabootarDocument {
    pub root: DomNode,
}

impl KabootarDocument {
    pub fn new() -> Self {
        Self {
            root: DomNode::element("html"),
        }
    }
}

fn global_stylesheet(env: &Environment) -> Stylesheet {
    env.get("__kstyle")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_stylesheet(&s)),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn kabootar_dom_globals(env: &mut Environment) {
    let doc = KabootarDocument::new();
    env.set("kdom".to_string(), Value::KabootarDom(doc.root.clone()));
    env.set("__kstyle".to_string(), Value::String(String::new()));
    env.set("kml".to_string(), Value::NativeFunction(kml_native));
    env.set("kdom_render".to_string(), Value::NativeFunction(kdom_render_native));
    env.set("kdom_paint".to_string(), Value::NativeFunction(kdom_paint_native));
    env.set("kdom_create".to_string(), Value::NativeFunction(kdom_create_native));
    env.set("kdom_append".to_string(), Value::NativeFunction(kdom_append_native));
    env.set("kdom_set_attr".to_string(), Value::NativeFunction(kdom_set_attr_native));
    env.set("kdom_get_attr".to_string(), Value::NativeFunction(kdom_get_attr_native));
    env.set("kdom_text".to_string(), Value::NativeFunction(kdom_text_native));
    env.set("kdom_set_text".to_string(), Value::NativeFunction(kdom_set_text_native));
    env.set(
        "kdom_set_text_by_id".to_string(),
        Value::NativeFunction(kdom_set_text_by_id_native),
    );
    env.set(
        "kdom_set_attr_by_id".to_string(),
        Value::NativeFunction(kdom_set_attr_by_id_native),
    );
    env.set(
        "kdom_clear_children_by_id".to_string(),
        Value::NativeFunction(kdom_clear_children_by_id_native),
    );
    env.set(
        "kdom_append_text_by_id".to_string(),
        Value::NativeFunction(kdom_append_text_by_id_native),
    );
    env.set(
        "kdom_append_by_id".to_string(),
        Value::NativeFunction(kdom_append_by_id_native),
    );
    env.set(
        "kdom_get_by_id".to_string(),
        Value::NativeFunction(kdom_get_by_id_native),
    );
    env.set(
        "kdom_on_by_id".to_string(),
        Value::NativeFunction(kdom_on_by_id_native),
    );
    env.set(
        "kdom_dispatch_by_id".to_string(),
        Value::NativeFunction(kdom_dispatch_by_id_native),
    );
    env.set(
        "kdom_child_id".to_string(),
        Value::NativeFunction(kdom_child_id_native),
    );
    env.set(
        "kdom_clear_children".to_string(),
        Value::NativeFunction(kdom_clear_children_native),
    );
    env.set("kdom_query".to_string(), Value::NativeFunction(kdom_query_native));
    env.set(
        "kdom_query_selector".to_string(),
        Value::NativeFunction(kdom_query_selector_native),
    );
    env.set(
        "kdom_query_selector_all".to_string(),
        Value::NativeFunction(kdom_query_selector_all_native),
    );
    env.set("kdom_query_id".to_string(), Value::NativeFunction(kdom_query_id_native));
    env.set("kdom_children".to_string(), Value::NativeFunction(kdom_children_native));
    env.set("kdom_on".to_string(), Value::NativeFunction(kdom_on_native));
    env.set("kdom_listen".to_string(), Value::NativeFunction(kdom_listen_native));
    env.set("kdom_id".to_string(), Value::NativeFunction(kdom_id_native));
    env.set("kdom_dispatch".to_string(), Value::NativeFunction(kdom_dispatch_native));
    env.set(
        "kdom_mutation_records".to_string(),
        Value::NativeFunction(kdom_mutation_records_native),
    );
    env.set(
        "kdom_mutation_clear".to_string(),
        Value::NativeFunction(kdom_mutation_clear_native),
    );
    env.set(
        "kdom_mo_new".to_string(),
        Value::NativeFunction(kdom_mo_new_native),
    );
    env.set(
        "kdom_mo_observe".to_string(),
        Value::NativeFunction(kdom_mo_observe_native),
    );
    env.set(
        "kdom_mo_disconnect".to_string(),
        Value::NativeFunction(kdom_mo_disconnect_native),
    );
    env.set(
        "kdom_mo_take_records".to_string(),
        Value::NativeFunction(kdom_mo_take_records_native),
    );
    env.set(
        "kdom_mo_deliver".to_string(),
        Value::NativeFunction(kdom_mo_deliver_native),
    );
    env.set(
        "MutationObserver".to_string(),
        Value::NativeFunction(mutation_observer_ctor_native),
    );
    env.set("kstyle_parse".to_string(), Value::NativeFunction(kstyle_parse_native));
    env.set("ktext_layout".to_string(), Value::NativeFunction(ktext_layout_native));
    env.set("ktext_measure".to_string(), Value::NativeFunction(ktext_measure_native));
    env.set("kdom_layer".to_string(), Value::String("kabootar".into()));
}

fn expect_dom(args: &[Value], i: usize, name: &str) -> Result<DomNode, String> {
    match args.get(i) {
        Some(Value::KabootarDom(n)) => Ok(n.clone()),
        _ => Err(format!("{name} expects a Kabootar DOM node")),
    }
}

fn kml_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let source = args.first().ok_or("kml() expects 1 argument")?;
    let Value::String(s) = source else {
        return Err("kml() expects a string".into());
    };
    let mut node = parse_kml(s)?;
    assign_ids(&mut node);
    Ok(Value::KabootarDom(node))
}

fn kdom_render_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_render()")?;
    Ok(Value::String(render_kml(&node)))
}

fn kdom_paint_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_paint()")?;
    live_upsert(&node);
    let node = live_resolve(node);
    let w = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            _ => None,
        })
        .unwrap_or(1280.0);
    let h = args
        .get(2)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            _ => None,
        })
        .unwrap_or(720.0);
    let mut engine = RenderEngine::with_viewport(w, h);
    engine.set_stylesheet(global_stylesheet(env));
    let frame = engine.compose(&node);
    crate::runtime::frame_buffer::publish_frame(frame.clone());
    Ok(Value::Object(frame_to_object(&frame)))
}

fn kstyle_parse_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let css = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kstyle_parse() expects CSS string".into()),
    };
    let sheet = parse_stylesheet(&css);
    let rules = sheet.rules.len();
    env.set("__kstyle".into(), Value::String(css));
    Ok(Value::Number(rules as i64))
}

fn ktext_layout_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ktext_layout() expects text string".into()),
    };
    let width = args.get(1).and_then(|v| match v {
        Value::Number(n) => Some(*n as f32),
        Value::Float(f) => Some(*f as f32),
        _ => None,
    });
    let font_size = args
        .get(2)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f32),
            Value::Float(f) => Some(*f as f32),
            _ => None,
        })
        .unwrap_or(16.0);
    let mut style = TextStyle {
        font_size,
        line_height: 1.25,
        max_width: width,
        white_space: WhiteSpace::Normal,
        color: 0xffe8eaed,
    };
    if let Some(Value::String(ws)) = args.get(3) {
        style.white_space = match ws.as_str() {
            "nowrap" => WhiteSpace::Nowrap,
            "pre-wrap" => WhiteSpace::PreWrap,
            _ => WhiteSpace::Normal,
        };
    }
    let layout = layout_text(text, &style);
    Ok(Value::Object(text_layout_to_object(&layout)))
}

fn ktext_measure_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ktext_measure() expects text string".into()),
    };
    let font_size = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f32),
            Value::Float(f) => Some(*f as f32),
            _ => None,
        })
        .unwrap_or(16.0);
    let style = TextStyle {
        font_size,
        line_height: 1.25,
        max_width: None,
        white_space: WhiteSpace::Normal,
        color: 0xffe8eaed,
    };
    let (w, h) = measure_text(text, &style);
    Ok(Value::Array(vec![Value::Float(w as f64), Value::Float(h as f64)]))
}

fn kdom_create_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tag = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kdom_create() expects a tag string".into()),
    };
    let node = DomNode::element(tag);
    live_upsert(&node);
    Ok(Value::KabootarDom(node))
}

fn kdom_append_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let parent_arg = expect_dom(args, 0, "kdom_append()")?;
    let child_arg = expect_dom(args, 1, "kdom_append()")?;
    let mut parent = live_resolve(parent_arg);
    let child = live_resolve(child_arg);
    let parent_id = parent.id;
    let child_id = child.id;
    parent.append(child);
    record_child_list_mutation(parent_id, child_id);
    live_upsert(&parent);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(parent))
}

fn kdom_set_attr_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut node = live_resolve(expect_dom(args, 0, "kdom_set_attr()")?);
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_set_attr() expects key string".into()),
    };
    let val = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        Some(other) => {
            node.set_attr(key, &crate::value::format_value(other));
            record_attribute_mutation(node.id, key);
            live_upsert(&node);
            deliver_mutation_observers(env)?;
            return Ok(Value::KabootarDom(node));
        }
        None => "",
    };
    node.set_attr(key, val);
    record_attribute_mutation(node.id, key);
    live_upsert(&node);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(node))
}

fn kdom_get_attr_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_get_attr()")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_get_attr() expects key string".into()),
    };
    Ok(match node.get_attr(key) {
        Some(v) => Value::String(v.into()),
        None => Value::Null,
    })
}

fn kdom_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kdom_text() expects a string".into()),
    };
    let node = DomNode::text_node(text);
    live_upsert(&node);
    Ok(Value::KabootarDom(node))
}

fn kdom_set_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut node = live_resolve(expect_dom(args, 0, "kdom_set_text()")?);
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Err("kdom_set_text() expects text string".into()),
    };
    if node.tag == "#text" {
        node.text = Some(text);
    } else {
        let child = DomNode::text_node(text);
        live_upsert(&child);
        node.children = vec![child];
    }
    live_upsert(&node);
    Ok(Value::KabootarDom(node))
}

fn kdom_set_text_by_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_set_text_by_id() expects numeric id".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Err("kdom_set_text_by_id() expects text".into()),
    };
    let mut node = live_get(id).ok_or_else(|| format!("kdom_set_text_by_id: unknown id {id}"))?;
    if node.tag == "#text" {
        node.text = Some(text);
    } else {
        let child = DomNode::text_node(text);
        live_upsert(&child);
        node.children = vec![child];
    }
    live_upsert(&node);
    live_propagate_to_ancestors(node.id);
    Ok(Value::KabootarDom(node))
}

fn kdom_set_attr_by_id_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_set_attr_by_id() expects numeric id".into()),
    };
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_set_attr_by_id() expects key string".into()),
    };
    let mut node = live_get(id).ok_or_else(|| format!("kdom_set_attr_by_id: unknown id {id}"))?;
    let val = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => crate::value::format_value(other),
        None => String::new(),
    };
    node.set_attr(key, &val);
    record_attribute_mutation(node.id, key);
    live_upsert(&node);
    live_propagate_to_ancestors(node.id);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(node))
}

fn kdom_clear_children_by_id_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_clear_children_by_id() expects numeric id".into()),
    };
    let mut node = live_get(id).ok_or_else(|| format!("kdom_clear_children_by_id: unknown id {id}"))?;
    for child in &node.children {
        record_child_removed_mutation(node.id, child.id);
    }
    node.children.clear();
    live_upsert(&node);
    live_propagate_to_ancestors(node.id);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(node))
}

fn kdom_append_text_by_id_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_append_text_by_id() expects numeric id".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Err("kdom_append_text_by_id() expects text".into()),
    };
    let mut node = live_get(id).ok_or_else(|| format!("kdom_append_text_by_id: unknown id {id}"))?;
    let child = DomNode::text_node(text);
    let child_id = child.id;
    live_upsert(&child);
    node.append(child);
    record_child_list_mutation(node.id, child_id);
    live_upsert(&node);
    live_propagate_to_ancestors(node.id);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(node))
}

fn kdom_append_by_id_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_append_by_id() expects numeric id".into()),
    };
    let child_arg = expect_dom(args, 1, "kdom_append_by_id()")?;
    let mut parent = live_get(id).ok_or_else(|| format!("kdom_append_by_id: unknown id {id}"))?;
    let child = live_resolve(child_arg);
    let parent_id = parent.id;
    let child_id = child.id;
    parent.append(child);
    record_child_list_mutation(parent_id, child_id);
    live_upsert(&parent);
    live_propagate_to_ancestors(parent.id);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(parent))
}

fn kdom_get_by_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_get_by_id() expects numeric id".into()),
    };
    Ok(match live_get(id) {
        Some(node) => Value::KabootarDom(node),
        None => Value::Null,
    })
}

fn kdom_on_by_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_on_by_id() expects numeric id".into()),
    };
    let event = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on_by_id() expects event name".into()),
    };
    let handler = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on_by_id() expects handler name".into()),
    };
    let mut node = live_get(id).ok_or_else(|| format!("kdom_on_by_id: unknown id {id}"))?;
    node.on(event, handler);
    live_upsert(&node);
    live_propagate_to_ancestors(node.id);
    Ok(Value::KabootarDom(node))
}

fn kdom_dispatch_by_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_dispatch_by_id() expects numeric id".into()),
    };
    let event = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_dispatch_by_id() expects event name".into()),
    };
    let node = live_get(id).ok_or_else(|| format!("kdom_dispatch_by_id: unknown id {id}"))?;
    if let Some(handler) = node.listeners.get(event) {
        crate::runtime::events::enqueue(crate::runtime::events::KabootarEvent {
            node_id: node.id,
            event_type: event.to_string(),
            handler: handler.clone(),
            x: 0.0,
            y: 0.0,
        });
        Ok(Value::String(handler.clone()))
    } else {
        Ok(Value::Null)
    }
}

fn kdom_child_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_child_id() expects numeric id".into()),
    };
    let idx = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let node = live_get(id).ok_or_else(|| format!("kdom_child_id: unknown id {id}"))?;
    Ok(match node.children.get(idx) {
        Some(child) => Value::Number(child.id as i64),
        None => Value::Null,
    })
}

fn kdom_clear_children_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut node = live_resolve(expect_dom(args, 0, "kdom_clear_children()")?);
    for child in &node.children {
        record_child_removed_mutation(node.id, child.id);
    }
    node.children.clear();
    live_upsert(&node);
    deliver_mutation_observers(env)?;
    Ok(Value::KabootarDom(node))
}

fn kdom_query_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_query()")?;
    let tag = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_query() expects tag string".into()),
    };
    Ok(match node.query_tag(tag) {
        Some(found) => Value::KabootarDom(found.clone()),
        None => Value::Null,
    })
}

fn kdom_query_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_query_id()")?;
    let id = match args.get(1) {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("kdom_query_id() expects numeric id".into()),
    };
    Ok(match node.query_id(id) {
        Some(found) => Value::KabootarDom(found.clone()),
        None => Value::Null,
    })
}

fn kdom_children_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_children()")?;
    Ok(Value::Array(
        node.children
            .iter()
            .cloned()
            .map(Value::KabootarDom)
            .collect(),
    ))
}

fn kdom_on_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut node = live_resolve(expect_dom(args, 0, "kdom_on()")?);
    let event = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on() expects event name".into()),
    };
    let handler = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on() expects handler name".into()),
    };
    node.on(event, handler);
    live_upsert(&node);
    Ok(Value::KabootarDom(node))
}

fn kdom_listen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut root = expect_dom(args, 0, "kdom_listen()")?;
    let tag = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_listen() expects tag string".into()),
    };
    let event = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_listen() expects event name".into()),
    };
    let handler = match args.get(3) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_listen() expects handler name".into()),
    };
    if !root.listen_on_tag(tag, event, handler) {
        return Err(format!("kdom_listen(): no node with tag '{tag}'"));
    }
    Ok(Value::KabootarDom(root))
}

fn kdom_id_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_id()")?;
    Ok(Value::Number(node.id as i64))
}

fn kdom_dispatch_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_dispatch()")?;
    let event = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_dispatch() expects event name".into()),
    };
    if let Some(handler) = node.listeners.get(event) {
        crate::runtime::events::enqueue(crate::runtime::events::KabootarEvent {
            node_id: node.id,
            event_type: event.to_string(),
            handler: handler.clone(),
            x: 0.0,
            y: 0.0,
        });
        Ok(Value::String(handler.clone()))
    } else {
        Ok(Value::Null)
    }
}

fn kdom_query_selector_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_query_selector()")?;
    let selector = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_query_selector() expects CSS selector string".into()),
    };
    Ok(match node.query_selector(selector) {
        Some(found) => Value::KabootarDom(found.clone()),
        None => Value::Null,
    })
}

fn kdom_query_selector_all_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = expect_dom(args, 0, "kdom_query_selector_all()")?;
    let selector = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_query_selector_all() expects CSS selector string".into()),
    };
    Ok(Value::Array(
        node.query_selector_all(selector)
            .into_iter()
            .cloned()
            .map(Value::KabootarDom)
            .collect(),
    ))
}

fn mutation_record_to_value(record: &MutationRecord) -> Value {
    let mut map = HashMap::new();
    map.insert("type".into(), Value::String(record.kind.clone()));
    map.insert("targetId".into(), Value::Number(record.target_id as i64));
    if let Some(attr) = &record.attribute_name {
        map.insert("attributeName".into(), Value::String(attr.clone()));
    }
    if let Some(id) = record.added_node_id {
        map.insert("addedNodeId".into(), Value::Number(id as i64));
    }
    if let Some(id) = record.removed_node_id {
        map.insert("removedNodeId".into(), Value::Number(id as i64));
    }
    Value::Object(map)
}

fn kdom_mutation_records_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Array(
        take_mutation_records()
            .iter()
            .map(mutation_record_to_value)
            .collect(),
    ))
}

fn kdom_mutation_clear_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = take_mutation_records();
    Ok(Value::Undefined)
}

fn observer_id_from(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64),
        Value::Object(o) => match o.get("id") {
            Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
            _ => Err("MutationObserver missing id".into()),
        },
        _ => Err("expected MutationObserver or id".into()),
    }
}

fn target_id_from(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64),
        Value::KabootarDom(n) => Ok(n.id),
        _ => Err("observe() expects DOM node or id".into()),
    }
}

fn kdom_mo_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let callback = args
        .first()
        .cloned()
        .ok_or("kdom_mo_new() expects callback")?;
    match &callback {
        Value::Function { .. } | Value::BytecodeFn(_) | Value::NativeFunction(_) => {}
        _ => return Err("kdom_mo_new() expects a function callback".into()),
    }
    let id = OBSERVER_ID.fetch_add(1, Ordering::SeqCst);
    MUTATION_OBSERVERS.with(|obs| {
        obs.borrow_mut().push(MutationObserverEntry {
            id,
            callback,
            target_id: None,
            child_list: true,
            attributes: false,
            connected: false,
            pending: Vec::new(),
        });
    });
    Ok(Value::Number(id as i64))
}

fn kdom_mo_observe_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let oid = observer_id_from(args.first().ok_or("kdom_mo_observe expects observer")?)?;
    let tid = target_id_from(args.get(1).ok_or("kdom_mo_observe expects target")?)?;
    let (child_list, attributes) = match args.get(2) {
        Some(Value::Object(o)) => {
            let child_list = match o.get("childList") {
                Some(Value::Bool(b)) => *b,
                _ => true,
            };
            let attributes = match o.get("attributes") {
                Some(Value::Bool(b)) => *b,
                _ => false,
            };
            (child_list, attributes)
        }
        _ => (true, false),
    };
    let found = MUTATION_OBSERVERS.with(|obs| {
        for entry in obs.borrow_mut().iter_mut() {
            if entry.id == oid {
                entry.target_id = Some(tid);
                entry.child_list = child_list;
                entry.attributes = attributes;
                entry.connected = true;
                return true;
            }
        }
        false
    });
    if !found {
        return Err(format!("unknown MutationObserver id {oid}"));
    }
    Ok(Value::Undefined)
}

fn kdom_mo_disconnect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let oid = observer_id_from(args.first().ok_or("kdom_mo_disconnect expects observer")?)?;
    MUTATION_OBSERVERS.with(|obs| {
        for entry in obs.borrow_mut().iter_mut() {
            if entry.id == oid {
                entry.connected = false;
                entry.target_id = None;
                entry.pending.clear();
            }
        }
    });
    Ok(Value::Undefined)
}

fn kdom_mo_take_records_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let oid = observer_id_from(args.first().ok_or("kdom_mo_take_records expects observer")?)?;
    let records = MUTATION_OBSERVERS.with(|obs| {
        for entry in obs.borrow_mut().iter_mut() {
            if entry.id == oid {
                return std::mem::take(&mut entry.pending);
            }
        }
        Vec::new()
    });
    Ok(Value::Array(
        records.iter().map(mutation_record_to_value).collect(),
    ))
}

fn kdom_mo_deliver_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    deliver_mutation_observers(env)?;
    Ok(Value::Undefined)
}

fn mutation_observer_ctor_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match kdom_mo_new_native(args, env)? {
        Value::Number(n) => n,
        _ => return Err("MutationObserver internal id error".into()),
    };
    let mut o = HashMap::new();
    o.insert("__kab_mo".into(), Value::Bool(true));
    o.insert("id".into(), Value::Number(id));
    o.insert("observe".into(), Value::NativeFunction(kdom_mo_observe_native));
    o.insert(
        "disconnect".into(),
        Value::NativeFunction(kdom_mo_disconnect_native),
    );
    o.insert(
        "takeRecords".into(),
        Value::NativeFunction(kdom_mo_take_records_native),
    );
    Ok(Value::Object(o))
}
