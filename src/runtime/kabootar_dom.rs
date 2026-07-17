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
}

thread_local! {
    static MUTATION_RECORDS: RefCell<Vec<MutationRecord>> = RefCell::new(Vec::new());
}

pub fn record_child_list_mutation(parent_id: u64, added_id: u64) {
    MUTATION_RECORDS.with(|r| {
        r.borrow_mut().push(MutationRecord {
            kind: "childList".into(),
            target_id: parent_id,
            attribute_name: None,
            added_node_id: Some(added_id),
        });
    });
}

pub fn record_attribute_mutation(target_id: u64, attr: &str) {
    MUTATION_RECORDS.with(|r| {
        r.borrow_mut().push(MutationRecord {
            kind: "attributes".into(),
            target_id,
            attribute_name: Some(attr.to_string()),
            added_node_id: None,
        });
    });
}

fn take_mutation_records() -> Vec<MutationRecord> {
    MUTATION_RECORDS.with(|r| r.borrow_mut().drain(..).collect())
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
    Ok(Value::KabootarDom(DomNode::element(tag)))
}

fn kdom_append_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut parent = expect_dom(args, 0, "kdom_append()")?;
    let child = expect_dom(args, 1, "kdom_append()")?;
    let parent_id = parent.id;
    let child_id = child.id;
    parent.append(child);
    record_child_list_mutation(parent_id, child_id);
    Ok(Value::KabootarDom(parent))
}

fn kdom_set_attr_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut node = expect_dom(args, 0, "kdom_set_attr()")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_set_attr() expects key string".into()),
    };
    let val = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        Some(other) => {
            node.set_attr(key, &crate::value::format_value(other));
            record_attribute_mutation(node.id, key);
            return Ok(Value::KabootarDom(node));
        }
        None => "",
    };
    node.set_attr(key, val);
    record_attribute_mutation(node.id, key);
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
    Ok(Value::KabootarDom(DomNode::text_node(text)))
}

fn kdom_set_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut node = expect_dom(args, 0, "kdom_set_text()")?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return Err("kdom_set_text() expects text string".into()),
    };
    if node.tag == "#text" {
        node.text = Some(text);
    } else {
        node.children = vec![DomNode::text_node(text)];
    }
    Ok(Value::KabootarDom(node))
}

fn kdom_clear_children_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut node = expect_dom(args, 0, "kdom_clear_children()")?;
    node.children.clear();
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
    let mut node = expect_dom(args, 0, "kdom_on()")?;
    let event = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on() expects event name".into()),
    };
    let handler = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kdom_on() expects handler name".into()),
    };
    node.on(event, handler);
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
