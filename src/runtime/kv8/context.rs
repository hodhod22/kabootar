//! Kv8 runtime context — isolate-like document + CSS + JS-scope state.

use crate::runtime::kabootar_dom::{assign_ids, DomNode, KabootarDocument};
use crate::runtime::kstyle::{compute_style, parse_stylesheet, ComputedStyle, Stylesheet};
use super::ast::Kv8Param;
use super::promise::{Kv8Microtask, SharedKv8Promise};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Kv8Value {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Dom(DomNode),
    Obj(HashMap<String, Kv8Value>),
    /// Interpreted `function` / block body
    Fun {
        params: Vec<Kv8Param>,
        body: Vec<super::ast::Stmt>,
        prototype: HashMap<String, Kv8Value>,
        closure: HashMap<String, Kv8Value>,
    },
    /// Arrow `=>` — bytecode cached on first call
    Arrow {
        params: Vec<Kv8Param>,
        body: Box<super::ast::Expr>,
        closure: HashMap<String, Kv8Value>,
    },
    Promise(SharedKv8Promise),
    /// `async function` body
    AsyncFun {
        params: Vec<Kv8Param>,
        body: Vec<super::ast::Stmt>,
        prototype: HashMap<String, Kv8Value>,
        closure: HashMap<String, Kv8Value>,
    },
    /// `Symbol.for("key")` / well-known symbols
    Symbol {
        key: String,
        id: u64,
    },
}

impl Kv8Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null | Self::Bool(false) => false,
            Self::Num(n) => *n != 0.0,
            Self::Str(s) => !s.is_empty(),
            Self::Obj(m) => !m.is_empty(),
            Self::Dom(_) => true,
            Self::Bool(true) => true,
            Self::Fun { .. } | Self::Arrow { .. } | Self::AsyncFun { .. } => true,
            Self::Promise(_) | Self::Symbol { .. } => true,
        }
    }

    pub fn as_obj(&self) -> Option<&HashMap<String, Kv8Value>> {
        match self {
            Self::Obj(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Kv8ContextInner {
    pub document: KabootarDocument,
    pub stylesheet: Stylesheet,
    pub css_text: String,
    pub scope_stack: Vec<HashMap<String, Kv8Value>>,
    pub last_result: Kv8Value,
    pub jit: Option<super::jit::Kv8Jit>,
    pub opt: super::opt::Kv8OptState,
    /// DOM node id → event type → listener callbacks (Arrow/Fun).
    pub listeners: HashMap<u64, HashMap<String, Vec<Kv8Value>>>,
    pub timers: Vec<Kv8Timer>,
    pub next_timer_id: u64,
    pub cancelled_timer_ids: HashSet<u64>,
    pub microtasks: std::collections::VecDeque<Kv8Microtask>,
    pub in_async: u32,
    pub promise_handles: HashMap<u64, SharedKv8Promise>,
    pub next_promise_handle: u64,
    pub local_storage: HashMap<String, String>,
    pub raf_callbacks: Vec<Kv8Value>,
    pub next_raf_id: u64,
    pub cancelled_raf_ids: HashSet<u64>,
    /// Detached or latest snapshot of DOM nodes by id (for setAttribute before mount).
    pub dom_snapshots: HashMap<u64, DomNode>,
    /// Arbitrary JS expando properties on DOM nodes (`node[key] = val`).
    pub dom_expandos: HashMap<u64, HashMap<String, Kv8Value>>,
    /// Mutable plain JS objects keyed by id (`{__obj_id: n}`).
    pub obj_store: HashMap<u64, HashMap<String, Kv8Value>>,
    pub next_obj_id: u64,
    pub nodelists: HashMap<u64, Vec<DomNode>>,
    pub next_nodelist_id: u64,
    /// `this` binding stack for method / constructor calls.
    pub this_stack: Vec<Kv8Value>,
    /// `Symbol.for` registry (key → symbol).
    pub symbol_registry: HashMap<String, Kv8Value>,
    pub next_symbol_id: u64,
    /// Bindings from completed UMD/module function scopes (survive scope_pop).
    pub module_bindings: HashMap<String, Kv8Value>,
    /// Mutable `globalThis` / `self` singleton for UMD exports.
    pub global_this: Option<Kv8Value>,
    /// Registered ES modules (`import … from "name"`).
    pub modules: HashMap<String, Kv8Module>,
    /// `export default` from the current module evaluation.
    pub export_default: Option<Kv8Value>,
    /// `export { name }` bindings from the current module evaluation.
    pub export_bindings: HashMap<String, Kv8Value>,
    /// Synthetic `#document` node returned from `ownerDocument` / `getRootNode`.
    pub owner_document_node: Option<DomNode>,
    /// Recent call targets for eval error hints (UMD debugging).
    pub call_trace: Vec<String>,
    /// Active `run_stmts` frames for forward function-decl hoisting.
    pub exec_stmts_stack: Vec<ExecStmtsFrame>,
}

#[derive(Debug, Clone)]
pub struct ExecStmtsFrame {
    pub stmts: Vec<super::ast::Stmt>,
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Kv8Module {
    pub default_export: Option<Kv8Value>,
    pub named: HashMap<String, Kv8Value>,
}

fn new_kv8_document() -> KabootarDocument {
    let mut doc = KabootarDocument::new();
    let mut body = DomNode::element("body");
    assign_ids(&mut body);
    doc.root.append(body);
    doc
}

#[derive(Debug, Clone)]
pub struct Kv8Timer {
    pub id: u64,
    pub wake_ms: u64,
    pub callback: Kv8Value,
    pub repeat_ms: Option<u64>,
}

impl Default for Kv8ContextInner {
    fn default() -> Self {
        Self {
            document: new_kv8_document(),
            stylesheet: Stylesheet::default(),
            css_text: String::new(),
            scope_stack: vec![HashMap::new()],
            last_result: Kv8Value::Undefined,
            jit: Some(super::jit::Kv8Jit::default()),
            opt: super::opt::Kv8OptState::default(),
            listeners: HashMap::new(),
            timers: Vec::new(),
            next_timer_id: 1,
            cancelled_timer_ids: HashSet::new(),
            microtasks: std::collections::VecDeque::new(),
            in_async: 0,
            promise_handles: HashMap::new(),
            next_promise_handle: 1,
            local_storage: HashMap::new(),
            raf_callbacks: Vec::new(),
            next_raf_id: 1,
            cancelled_raf_ids: HashSet::new(),
            dom_snapshots: HashMap::new(),
            dom_expandos: HashMap::new(),
            obj_store: HashMap::new(),
            next_obj_id: 1,
            nodelists: HashMap::new(),
            next_nodelist_id: 1,
            this_stack: Vec::new(),
            symbol_registry: HashMap::new(),
            next_symbol_id: 1,
            global_this: None,
            modules: HashMap::new(),
            export_default: None,
            export_bindings: HashMap::new(),
            module_bindings: HashMap::new(),
            call_trace: Vec::new(),
            exec_stmts_stack: Vec::new(),
            owner_document_node: None,
        }
    }
}

#[derive(Clone)]
pub struct Kv8Context {
    inner: Arc<Mutex<Kv8ContextInner>>,
    eval_ops: Arc<AtomicU64>,
    /// `0` = no limit
    eval_ops_limit: Arc<AtomicU64>,
}

impl std::fmt::Debug for Kv8Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kv8Context")
            .field("eval_ops", &self.eval_ops.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Default for Kv8Context {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Kv8ContextInner::default())),
            eval_ops: Arc::new(AtomicU64::new(0)),
            eval_ops_limit: Arc::new(AtomicU64::new(0)),
        }
    }
}

fn plain_obj_id(map: &HashMap<String, Kv8Value>) -> Option<u64> {
    map.get("__obj_id")
        .and_then(|v| v.as_num())
        .map(|n| n as u64)
}

fn collect_closure_capture_names(
    v: &Kv8Value,
    names: &mut HashSet<String>,
    obj_store: &HashMap<u64, HashMap<String, Kv8Value>>,
) {
    match v {
        Kv8Value::Fun { closure, .. } | Kv8Value::AsyncFun { closure, .. } => {
            names.extend(closure.keys().cloned());
        }
        Kv8Value::Obj(map) => {
            if map.get("__native").and_then(|v| v.as_str()) == Some("bound.call") {
                if let Some(target) = map.get("__target") {
                    collect_closure_capture_names(target, names, obj_store);
                }
            }
            for val in map.values() {
                collect_closure_capture_names(val, names, obj_store);
            }
            if let Some(id) = plain_obj_id(map) {
                if let Some(store) = obj_store.get(&id) {
                    for (k, val) in store {
                        if k.starts_with("__desc__:") {
                            if let Kv8Value::Obj(desc) = val {
                                if let Some(getter) = desc.get("get") {
                                    collect_closure_capture_names(getter, names, obj_store);
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

impl Kv8ContextInner {
    pub fn scope_current_mut(&mut self) -> &mut HashMap<String, Kv8Value> {
        self.scope_stack
            .last_mut()
            .expect("kv8 scope stack must not be empty")
    }

    pub fn scope_get(&self, name: &str) -> Option<Kv8Value> {
        for frame in self.scope_stack.iter().rev() {
            if let Some(v) = frame.get(name) {
                return Some(v.clone());
            }
        }
        self.module_bindings.get(name).cloned()
    }

    /// Scope lookup excluding the innermost frame (active call scope).
    pub fn scope_get_outer(&self, name: &str) -> Option<Kv8Value> {
        if self.scope_stack.len() > 1 {
            for frame in self.scope_stack[..self.scope_stack.len() - 1].iter().rev() {
                if let Some(v) = frame.get(name) {
                    return Some(v.clone());
                }
            }
        }
        self.module_bindings.get(name).cloned()
    }

    pub fn capture_lexical_env(&self) -> HashMap<String, Kv8Value> {
        let mut env = HashMap::new();
        // Include all frames (incl. current) — same-block `var mn` must be visible in
        // `Nm = function(){ mn(...) }`. Skip `Fun` bodies (hoisted to module_bindings).
        for frame in &self.scope_stack {
            for (k, v) in frame {
                if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                    continue;
                }
                env.insert(k.clone(), v.clone());
            }
        }
        env
    }

    pub fn scope_pop_preserve(&mut self) {
        if self.scope_stack.len() <= 1 {
            return;
        }
        let Some(frame) = self.scope_stack.pop() else {
            return;
        };
        let mut closure_names = HashSet::new();
        for v in frame.values() {
            collect_closure_capture_names(v, &mut closure_names, &self.obj_store);
        }
        for (k, v) in frame {
            if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                self.module_bindings.entry(k).or_insert(v);
            } else if closure_names.contains(&k) {
                self.module_bindings.insert(k, v);
            }
        }
    }

    pub fn scope_resolve_mut(&mut self, name: &str) -> Option<&mut Kv8Value> {
        for frame in self.scope_stack.iter_mut().rev() {
            if frame.contains_key(name) {
                return frame.get_mut(name);
            }
        }
        self.module_bindings.get_mut(name)
    }

    /// True when `name` is bound in an outer scope frame or module bindings.
    pub fn lexical_binding_visible(&self, name: &str) -> bool {
        if self.scope_stack.len() > 1 {
            for frame in self.scope_stack[..self.scope_stack.len() - 1].iter().rev() {
                if frame.contains_key(name) {
                    return true;
                }
            }
        }
        self.module_bindings.contains_key(name)
    }

    pub fn scope_push(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    pub fn scope_pop(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
    }

    pub fn try_materialize_forward_fun(&mut self, name: &str) -> Option<Kv8Value> {
        let frame = self.exec_stmts_stack.last()?;
        for stmt in frame.stmts.iter().skip(frame.index.saturating_add(1)) {
            if let super::ast::Stmt::Function(n, params, body) = stmt {
                if n == name {
                    let closure = self.capture_lexical_env();
                    let fun = Kv8Value::Fun {
                        params: params.clone(),
                        body: body.clone(),
                        prototype: HashMap::new(),
                        closure,
                    };
                    self.scope_current_mut()
                        .insert(name.to_string(), fun.clone());
                    return Some(fun);
                }
            }
        }
        None
    }
}

impl Kv8Context {
    pub fn with_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Kv8ContextInner) -> Result<T, String>,
    {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| "kv8 context lock poisoned".to_string())?;
        f(&mut g)
    }

    pub fn with_read<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Kv8ContextInner) -> Result<T, String>,
    {
        let g = self
            .inner
            .lock()
            .map_err(|_| "kv8 context lock poisoned".to_string())?;
        f(&g)
    }

    pub fn reset_eval_ops(&self) {
        self.eval_ops.store(0, Ordering::Relaxed);
    }

    pub fn eval_ops_count(&self) -> u64 {
        self.eval_ops.load(Ordering::Relaxed)
    }

    /// Cap interpreter steps per `run_program` / `eval_script` (detect infinite loops).
    pub fn set_eval_ops_limit(&self, limit: Option<u64>) -> Result<(), String> {
        self.eval_ops_limit.store(limit.unwrap_or(0), Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn bump_eval_ops(&self) -> Result<(), String> {
        let n = self.eval_ops.fetch_add(1, Ordering::Relaxed) + 1;
        let limit = self.eval_ops_limit.load(Ordering::Relaxed);
        if limit != 0 && n > limit {
            return Err(format!(
                "Kv8 eval budget exceeded ({limit} ops) — bundle eval is very slow or stuck in a loop"
            ));
        }
        Ok(())
    }

    pub fn symbol_for(&self, key: &str) -> Result<Kv8Value, String> {
        self.with_mut(|inner| {
            if let Some(sym) = inner.symbol_registry.get(key) {
                return Ok(sym.clone());
            }
            let sym = Kv8Value::Symbol {
                key: key.to_string(),
                id: inner.next_symbol_id,
            };
            inner.next_symbol_id = inner.next_symbol_id.saturating_add(1);
            inner.symbol_registry.insert(key.to_string(), sym.clone());
            Ok(sym)
        })
    }

    pub fn well_known_symbol(&self, key: &str) -> Result<Kv8Value, String> {
        self.symbol_for(key)
    }

    fn ensure_dom_index(&self) -> Result<(), String> {
        self.with_mut(|ctx| {
            if ctx.opt.dom_index_dirty {
                super::opt::rebuild_dom_index(&ctx.document.root, &mut ctx.opt.dom_paths);
                ctx.opt.dom_index_dirty = false;
            }
            Ok(())
        })
    }

    fn mark_dom_dirty(&self) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.opt.dom_index_dirty = true;
            Ok(())
        })
    }

    pub fn set_stylesheet(&self, css: &str) -> Result<usize, String> {
        self.with_mut(|ctx| {
            if ctx.css_text == css {
                return Ok(ctx.stylesheet.rules.len());
            }
            ctx.css_text = css.to_string();
            ctx.stylesheet = parse_stylesheet(css);
            ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
            ctx.opt.style_cache.clear();
            Ok(ctx.stylesheet.rules.len())
        })
    }

    pub fn computed_style(&self, node: &DomNode) -> Result<ComputedStyle, String> {
        self.with_mut(|ctx| {
            if let Some(cached) = ctx.opt.style_cache.get(&node.id) {
                return Ok(cached.clone());
            }
            let style = compute_style(&node.tag, &node.attributes, &ctx.stylesheet);
            ctx.opt.style_cache.insert(node.id, style.clone());
            Ok(style)
        })
    }

    pub fn root_dom(&self) -> Result<DomNode, String> {
        self.with_mut(|ctx| Ok(ctx.document.root.clone()))
    }

    pub fn create_element(&self, tag: &str) -> Result<DomNode, String> {
        self.with_mut(|ctx| {
            let mut el = DomNode::element(tag);
            assign_ids(&mut el);
            ctx.dom_snapshots.insert(el.id, el.clone());
            Ok(el)
        })
    }

    pub fn owner_document_value(&self) -> Result<Kv8Value, String> {
        self.with_mut(|ctx| {
            if ctx.owner_document_node.is_none() {
                let mut doc = DomNode::element("#document");
                assign_ids(&mut doc);
                ctx.dom_snapshots.insert(doc.id, doc.clone());
                ctx.owner_document_node = Some(doc);
            }
            Ok(Kv8Value::Dom(ctx.owner_document_node.clone().unwrap()))
        })
    }

    pub fn snapshot_node(&self, node: DomNode) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.dom_snapshots.insert(node.id, node);
            Ok(())
        })
    }

    pub fn resolve_node(&self, id: u64) -> Result<Option<DomNode>, String> {
        if let Some(n) = self.find_dom_by_id(id)? {
            return Ok(Some(n));
        }
        self.with_mut(|ctx| Ok(ctx.dom_snapshots.get(&id).cloned()))
    }

    pub fn publish_node(&self, node: DomNode) -> Result<(), String> {
        let _ = self.replace_dom_node(node.clone())?;
        self.snapshot_node(node.clone())?;
        self.with_mut(|ctx| {
            for frame in &mut ctx.scope_stack {
                for v in frame.values_mut() {
                    if let Kv8Value::Dom(d) = v {
                        if d.id == node.id {
                            *d = node.clone();
                        }
                    }
                }
            }
            Ok(())
        })
    }

    pub fn store_nodelist(&self, nodes: Vec<DomNode>) -> Result<u64, String> {
        self.with_mut(|ctx| {
            let id = ctx.next_nodelist_id;
            ctx.next_nodelist_id = ctx.next_nodelist_id.saturating_add(1);
            ctx.nodelists.insert(id, nodes);
            Ok(id)
        })
    }

    pub fn nodelist_nodes(&self, id: u64) -> Result<Vec<DomNode>, String> {
        self.with_mut(|ctx| Ok(ctx.nodelists.get(&id).cloned().unwrap_or_default()))
    }

    pub fn append_child(&self, parent_id: u64, child: DomNode) -> Result<bool, String> {
        let tree_ok = self.with_mut(|ctx| {
            if let Some(p) = find_mut_by_id(&mut ctx.document.root, parent_id) {
                p.append(child.clone());
                return Ok(true);
            }
            if ctx.document.root.id == parent_id {
                ctx.document.root.append(child.clone());
                return Ok(true);
            }
            Ok(false)
        })?;
        if tree_ok {
            self.snapshot_node(child)?;
            self.mark_dom_dirty()?;
            return Ok(true);
        }
        self.with_mut(|ctx| {
            let Some(mut parent) = ctx.dom_snapshots.get(&parent_id).cloned() else {
                return Ok(false);
            };
            parent.append(child.clone());
            ctx.dom_snapshots.insert(parent_id, parent.clone());
            ctx.dom_snapshots.insert(child.id, child);
            for frame in &mut ctx.scope_stack {
                for v in frame.values_mut() {
                    if let Kv8Value::Dom(d) = v {
                        if d.id == parent_id {
                            *d = parent.clone();
                        }
                    }
                }
            }
            Ok(true)
        })
    }

    pub fn remove_child(&self, parent_id: u64, child_id: u64) -> Result<Option<DomNode>, String> {
        let removed = self.with_mut(|ctx| {
            if let Some(parent) = find_mut_by_id(&mut ctx.document.root, parent_id) {
                if let Some(idx) = parent.children.iter().position(|c| c.id == child_id) {
                    return Ok(Some(parent.children.remove(idx)));
                }
                return Ok(None);
            }
            if ctx.document.root.id == parent_id {
                if let Some(idx) = ctx.document.root.children.iter().position(|c| c.id == child_id) {
                    return Ok(Some(ctx.document.root.children.remove(idx)));
                }
                return Ok(None);
            }
            Ok(None)
        })?;
        if removed.is_some() {
            self.with_mut(|ctx| {
                ctx.dom_snapshots.remove(&child_id);
                Ok(())
            })?;
            self.mark_dom_dirty()?;
            return Ok(removed);
        }
        self.with_mut(|ctx| {
            let Some(mut parent) = ctx.dom_snapshots.get(&parent_id).cloned() else {
                return Ok(None);
            };
            let Some(idx) = parent.children.iter().position(|c| c.id == child_id) else {
                return Ok(None);
            };
            let child = parent.children.remove(idx);
            ctx.dom_snapshots.insert(parent_id, parent.clone());
            ctx.dom_snapshots.remove(&child_id);
            for frame in &mut ctx.scope_stack {
                for v in frame.values_mut() {
                    if let Kv8Value::Dom(d) = v {
                        if d.id == parent_id {
                            *d = parent.clone();
                        }
                    }
                }
            }
            Ok(Some(child))
        })
    }

    pub fn first_child(&self, node_id: u64) -> Result<Option<DomNode>, String> {
        if let Some(node) = self.resolve_node(node_id)? {
            return Ok(node.children.first().cloned());
        }
        Ok(None)
    }

    pub fn query_selector(&self, selector: &str) -> Result<Option<DomNode>, String> {
        self.ensure_dom_index()?;
        self.with_mut(|ctx| Ok(find_by_selector(&ctx.document.root, selector).cloned()))
    }

    pub fn query_selector_all(&self, selector: &str) -> Result<Vec<DomNode>, String> {
        self.ensure_dom_index()?;
        self.with_mut(|ctx| {
            Ok(ctx
                .document
                .root
                .query_selector_all(selector)
                .into_iter()
                .cloned()
                .collect())
        })
    }

    pub fn query_selector_all_from(&self, node_id: u64, selector: &str) -> Result<Vec<DomNode>, String> {
        self.ensure_dom_index()?;
        let root = self
            .resolve_node(node_id)?
            .ok_or_else(|| format!("querySelectorAll: node {node_id} not found"))?;
        Ok(root
            .query_selector_all(selector)
            .into_iter()
            .cloned()
            .collect())
    }

    pub fn get_element_by_id(&self, id: &str) -> Result<Option<DomNode>, String> {
        self.ensure_dom_index()?;
        self.with_mut(|ctx| Ok(find_by_attr(&ctx.document.root, "id", id).cloned()))
    }

    pub fn inner_html(&self, node_id: u64) -> Result<String, String> {
        self.ensure_dom_index()?;
        self.with_mut(|ctx| {
            let Some(node) = find_by_id(&ctx.document.root, node_id) else {
                return Ok(String::new());
            };
            Ok(serialize_inner_html(&node.children))
        })
    }

    pub fn set_inner_html(&self, node_id: u64, html: &str) -> Result<bool, String> {
        let mut node = self
            .find_dom_by_id(node_id)?
            .ok_or_else(|| format!("set_inner_html: node {node_id} not found"))?;
        node.children = parse_inner_html(html)?;
        self.replace_dom_node(node)
    }

    pub fn replace_dom_node(&self, node: DomNode) -> Result<bool, String> {
        self.ensure_dom_index()?;
        let id = node.id;
        let ok = self.with_mut(|ctx| {
            if let Some(path) = ctx.opt.dom_paths.get(&id).cloned() {
                if let Some(n) = super::opt::find_mut_by_path(&mut ctx.document.root, &path) {
                    *n = node;
                    return Ok(true);
                }
            }
            if let Some(n) = find_mut_by_id(&mut ctx.document.root, id) {
                *n = node;
                return Ok(true);
            }
            Ok(false)
        })?;
        if ok {
            self.mark_dom_dirty()?;
            self.with_mut(|ctx| {
                ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
                ctx.opt.style_cache.remove(&id);
                Ok(())
            })?;
        }
        Ok(ok)
    }

    pub fn parse_inner_html_fragment(&self, html: &str) -> Result<Vec<DomNode>, String> {
        parse_inner_html(html)
    }

    pub fn enqueue_microtask(&self, callback: Kv8Value, args: Vec<Kv8Value>) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.microtasks.push_back(Kv8Microtask { callback, args });
            Ok(())
        })
    }

    pub fn request_animation_frame(&self, callback: Kv8Value) -> Result<u64, String> {
        self.with_mut(|ctx| {
            let id = ctx.next_raf_id;
            ctx.next_raf_id = ctx.next_raf_id.saturating_add(1);
            ctx.raf_callbacks.push(callback);
            Ok(id)
        })
    }

    pub fn cancel_animation_frame(&self, id: u64) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.cancelled_raf_ids.insert(id);
            Ok(())
        })
    }

    pub fn take_raf_callbacks(&self) -> Result<Vec<Kv8Value>, String> {
        self.with_mut(|ctx| Ok(ctx.raf_callbacks.drain(..).collect()))
    }

    pub fn set_text_content(&self, id: u64, text: &str) -> Result<bool, String> {
        self.ensure_dom_index()?;
        let ok = self.mutate_node(id, |n| {
            n.text = Some(text.to_string());
        })?;
        if ok {
            self.with_mut(|ctx| {
                ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
                ctx.opt.style_cache.remove(&id);
                Ok(())
            })?;
        }
        Ok(ok)
    }

    pub fn set_attr(&self, id: u64, key: &str, value: &str) -> Result<bool, String> {
        self.ensure_dom_index()?;
        let ok = self.mutate_node(id, |n| {
            n.set_attr(key, value);
        })?;
        if ok {
            self.with_mut(|ctx| {
                ctx.opt.style_generation = ctx.opt.style_generation.wrapping_add(1);
                ctx.opt.style_cache.remove(&id);
                Ok(())
            })?;
        }
        Ok(ok)
    }

    pub fn add_event_listener(
        &self,
        node_id: u64,
        event_type: &str,
        listener: Kv8Value,
    ) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.listeners
                .entry(node_id)
                .or_default()
                .entry(event_type.to_string())
                .or_default()
                .push(listener);
            Ok(())
        })
    }

    pub fn listeners_for(
        &self,
        node_id: u64,
        event_type: &str,
    ) -> Result<Vec<Kv8Value>, String> {
        self.with_mut(|ctx| {
            Ok(ctx
                .listeners
                .get(&node_id)
                .and_then(|m| m.get(event_type))
                .cloned()
                .unwrap_or_default())
        })
    }

    pub fn find_dom_by_id(&self, id: u64) -> Result<Option<DomNode>, String> {
        self.with_mut(|ctx| Ok(find_by_id(&ctx.document.root, id).cloned()))
    }

    pub fn body_node(&self) -> Result<DomNode, String> {
        self.with_mut(|ctx| {
            ctx.document
                .root
                .children
                .iter()
                .find(|n| n.tag == "body")
                .cloned()
                .ok_or_else(|| "document.body missing".into())
        })
    }

    pub fn storage_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.local_storage.insert(key.to_string(), value.to_string());
            Ok(())
        })
    }

    pub fn storage_get(&self, key: &str) -> Result<Option<String>, String> {
        self.with_mut(|ctx| Ok(ctx.local_storage.get(key).cloned()))
    }

    pub fn storage_remove(&self, key: &str) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.local_storage.remove(key);
            Ok(())
        })
    }

    pub fn storage_clear(&self) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.local_storage.clear();
            Ok(())
        })
    }

    pub fn storage_key(&self, index: usize) -> Result<Option<String>, String> {
        self.with_mut(|ctx| {
            Ok(ctx
                .local_storage
                .keys()
                .nth(index)
                .cloned())
        })
    }

    pub fn schedule_timer(
        &self,
        callback: Kv8Value,
        delay_ms: u64,
        repeat_ms: Option<u64>,
    ) -> Result<u64, String> {
        let now = crate::value::unix_ms_now();
        self.with_mut(|ctx| {
            let id = ctx.next_timer_id;
            ctx.next_timer_id = ctx.next_timer_id.saturating_add(1);
            ctx.timers.push(Kv8Timer {
                id,
                wake_ms: now.saturating_add(delay_ms),
                callback,
                repeat_ms,
            });
            Ok(id)
        })
    }

    pub fn cancel_timer(&self, id: u64) -> Result<(), String> {
        self.with_mut(|ctx| {
            ctx.cancelled_timer_ids.insert(id);
            ctx.timers.retain(|t| t.id != id);
            Ok(())
        })
    }

    pub fn take_due_timers(&self) -> Result<Vec<Kv8Value>, String> {
        let now = crate::value::unix_ms_now();
        self.with_mut(|ctx| {
            let mut fired = Vec::new();
            let mut remaining = Vec::new();
            for t in ctx.timers.drain(..) {
                if ctx.cancelled_timer_ids.contains(&t.id) {
                    continue;
                }
                if t.wake_ms <= now {
                    fired.push((t.callback.clone(), t.repeat_ms, t.id));
                } else {
                    remaining.push(t);
                }
            }
            ctx.timers = remaining;
            for (callback, repeat_ms, id) in &fired {
                if let Some(repeat) = repeat_ms {
                    ctx.timers.push(Kv8Timer {
                        id: *id,
                        wake_ms: now.saturating_add(*repeat),
                        callback: callback.clone(),
                        repeat_ms: Some(*repeat),
                    });
                }
            }
            Ok(fired.into_iter().map(|(cb, _, _)| cb).collect())
        })
    }

    fn mutate_node<F>(&self, id: u64, f: F) -> Result<bool, String>
    where
        F: FnOnce(&mut DomNode),
    {
        self.with_mut(|ctx| {
            if let Some(path) = ctx.opt.dom_paths.get(&id).cloned() {
                if let Some(n) = super::opt::find_mut_by_path(&mut ctx.document.root, &path) {
                    f(n);
                    return Ok(true);
                }
            }
            if let Some(n) = find_mut_by_id(&mut ctx.document.root, id) {
                f(n);
                return Ok(true);
            }
            Ok(false)
        })
    }
}

fn find_mut_by_id<'a>(node: &'a mut DomNode, id: u64) -> Option<&'a mut DomNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_mut_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

fn find_by_id(node: &DomNode, id: u64) -> Option<&DomNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

fn find_by_selector<'a>(node: &'a DomNode, selector: &str) -> Option<&'a DomNode> {
    let sel = selector.trim();
    if sel.starts_with('#') {
        let id_attr = &sel[1..];
        return find_by_attr(node, "id", id_attr);
    }
    if sel.starts_with('.') {
        let cls = &sel[1..];
        return find_by_attr(node, "class", cls);
    }
    node.query_tag(sel)
}

fn find_by_attr<'a>(node: &'a DomNode, key: &str, value: &str) -> Option<&'a DomNode> {
    if node.get_attr(key) == Some(value) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_by_attr(child, key, value) {
            return Some(found);
        }
    }
    None
}

fn serialize_inner_html(children: &[DomNode]) -> String {
    children.iter().map(serialize_dom_fragment).collect()
}

fn serialize_dom_fragment(node: &DomNode) -> String {
    if node.tag == "#text" {
        return node.text.clone().unwrap_or_default();
    }
    let inner = serialize_inner_html(&node.children);
    format!("<{}>{inner}</{}>", node.tag, node.tag)
}

fn parse_inner_html(html: &str) -> Result<Vec<DomNode>, String> {
    Ok(parse_html_nodes(html.trim()))
}

fn parse_html_nodes(input: &str) -> Vec<DomNode> {
    let mut nodes = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(lt) = rest.find('<') {
            if lt > 0 {
                nodes.push(DomNode::text_node(&rest[..lt]));
            }
            rest = &rest[lt..];
            if !rest.starts_with('<') {
                break;
            }
            let Some(gt) = rest.find('>') else {
                break;
            };
            let tag_part = rest[1..gt].trim();
            rest = &rest[gt + 1..];
            if tag_part.is_empty() {
                continue;
            }
            if tag_part.starts_with('/') {
                break;
            }
            if tag_part.ends_with('/') {
                let tag = tag_part.trim_end_matches('/').split_whitespace().next().unwrap_or("");
                if !tag.is_empty() {
                    nodes.push(DomNode::element(tag));
                }
                continue;
            }
            let tag = tag_part.split_whitespace().next().unwrap_or(tag_part);
            let end_tag = format!("</{tag}>");
            if let Some(end_pos) = rest.find(&end_tag) {
                let inner = &rest[..end_pos];
                let mut el = DomNode::element(tag);
                el.children = parse_html_nodes(inner);
                assign_ids(&mut el);
                nodes.push(el);
                rest = &rest[end_pos + end_tag.len()..];
            } else {
                break;
            }
        } else {
            nodes.push(DomNode::text_node(rest));
            break;
        }
    }
    for node in &mut nodes {
        assign_ids(node);
    }
    nodes
}
