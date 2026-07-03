//! Kv8 runtime context — isolate-like document + CSS + JS-scope state.

use crate::runtime::kabootar_dom::{assign_ids, DomNode, KabootarDocument};
use crate::runtime::kstyle::{compute_style, parse_stylesheet, ComputedStyle, Stylesheet};
use super::ast::Kv8Param;
use super::promise::{Kv8Microtask, SharedKv8Promise};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Shared mutable environment for a `function` / arrow closure.
pub type Kv8ClosureEnv = Arc<Mutex<HashMap<String, Kv8Value>>>;

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
        closure: Kv8ClosureEnv,
    },
    /// Arrow `=>` — bytecode cached on first call
    Arrow {
        params: Vec<Kv8Param>,
        body: Box<super::ast::Expr>,
        closure: Kv8ClosureEnv,
    },
    Promise(SharedKv8Promise),
    /// `async function` body
    AsyncFun {
        params: Vec<Kv8Param>,
        body: Vec<super::ast::Stmt>,
        prototype: HashMap<String, Kv8Value>,
        closure: Kv8ClosureEnv,
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
    /// Latest hoisted `function` value per name (last factory pop wins; used by finalize).
    pub hoist_latest: HashMap<String, Kv8Value>,
    /// Set after [`Self::finalize_module_hoists`] — enables live hoist scope resolution.
    pub hoists_finalized: bool,
    /// Closure variable names for the active call.
    pub closure_assign_stack: Vec<HashSet<String>>,
    /// Live closure cells for the active call (per-function, not global).
    pub closure_env_stack: Vec<Kv8ClosureEnv>,
    /// Mutable `globalThis` / `self` singleton for UMD exports.
    pub global_this: Option<Kv8Value>,
    /// Op counter for infinite-loop detection (reset per eval_script call).
    pub op_count: u64,
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
    pub stmts: Arc<Vec<super::ast::Stmt>>,
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
            op_count: 0,
            modules: HashMap::new(),
            export_default: None,
            export_bindings: HashMap::new(),
            module_bindings: HashMap::new(),
            hoist_latest: HashMap::new(),
            hoists_finalized: false,
            closure_assign_stack: Vec::new(),
            closure_env_stack: Vec::new(),
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


impl Kv8ContextInner {
    pub fn scope_current_mut(&mut self) -> &mut HashMap<String, Kv8Value> {
        self.scope_stack
            .last_mut()
            .expect("kv8 scope stack must not be empty")
    }

    /// Returns true if the name is already bound in the current (innermost) frame.
    pub fn scope_current_has(&self, name: &str) -> bool {
        self.scope_stack.last().map_or(false, |f| f.contains_key(name))
    }

    fn finalized_hoist(&self, name: &str) -> Option<Kv8Value> {
        if !self.hoists_finalized {
            return None;
        }
        self.hoist_latest.get(name).and_then(|v| {
            if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    pub fn scope_get(&self, name: &str) -> Option<Kv8Value> {
        for frame in self.scope_stack.iter().rev() {
            if let Some(v) = frame.get(name) {
                if matches!(v, Kv8Value::Undefined) {
                    if let Some(h) = self.finalized_hoist(name) {
                        return Some(h);
                    }
                }
                if self.closure_assign_active(name) {
                    if let Some(h) = self.finalized_hoist(name) {
                        if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                            return Some(h);
                        }
                    }
                    if matches!(v, Kv8Value::Undefined) {
                        if let Some(mb) = self.module_bindings.get(name) {
                            if matches!(mb, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                                return Some(mb.clone());
                            }
                        }
                    }
                }
                return Some(v.clone());
            }
        }
        if self.closure_assign_active(name) {
            if let Some(v) = self.scope_get_outer_lexical(name) {
                return Some(v);
            }
            // Prefer live module hoists over closure snapshots for functions.
            if let Some(v) = self.module_bindings.get(name) {
                if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                    return Some(v.clone());
                }
            }
            if let Some(env) = self.closure_env_stack.last() {
                if let Ok(map) = env.lock() {
                    if let Some(v) = map.get(name) {
                        return Some(v.clone());
                    }
                }
            }
        }
        self.module_bindings.get(name).cloned()
    }

    /// Scope lookup excluding the innermost frame (active call scope).
    /// Also skips variables that are declared as parameters in an outer frame —
    /// a parameter `factory` in Tm's call-frame must not shadow the `factory`
    /// captured in bm's own closure snapshot.
    pub fn scope_get_outer_lexical(&self, name: &str) -> Option<Kv8Value> {
        let skip_closure_frames = self.closure_assign_active(name);
        if self.scope_stack.len() > 1 {
            for frame in self.scope_stack[..self.scope_stack.len() - 1].iter().rev() {
                if skip_closure_frames
                    && matches!(frame.get("__is_closure_frame__"), Some(Kv8Value::Bool(true)))
                    && frame.contains_key(name)
                {
                    continue;
                }
                if let Some(v) = frame.get(name) {
                    let is_param = frame
                        .get("__params__")
                        .and_then(|p| if let Kv8Value::Str(s) = p { Some(s.as_str()) } else { None })
                        .map(|s| s.split(',').any(|p| p == name))
                        .unwrap_or(false);
                    if is_param {
                        continue;
                    }
                    return Some(v.clone());
                }
            }
        }
        None
    }

    pub fn scope_get_outer(&self, name: &str) -> Option<Kv8Value> {
        if let Some(v) = self.scope_get_outer_lexical(name) {
            return Some(v);
        }
        if self.closure_assign_active(name) {
            return None;
        }
        self.module_bindings.get(name).cloned()
    }

    pub fn capture_lexical_env(&self) -> HashMap<String, Kv8Value> {
        let mut env = HashMap::new();
        for frame in &self.scope_stack {
            // Collect the parameter names for this frame.
            let param_names: std::collections::HashSet<&str> = frame
                .get("__params__")
                .and_then(|v| if let Kv8Value::Str(s) = v { Some(s.as_str()) } else { None })
                .map(|s| s.split(',').filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            for (k, v) in frame {
                if k == "__params__" || k == "__hoisted_fns__" || k == "__is_closure_frame__" {
                    continue;
                }
                // Skip Fun/AsyncFun values UNLESS they are parameters.
                // Hoisted helpers referenced by inner closures use capture_lexical_env_for.
                if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. })
                    && !param_names.contains(k.as_str())
                {
                    continue;
                }
                env.insert(k.clone(), v.clone());
            }
        }
        env
    }

    /// Capture only names referenced by an inner function (transitive over hoisted helpers).
    pub fn capture_lexical_env_for(&self, names: &std::collections::HashSet<String>) -> HashMap<String, Kv8Value> {
        if names.is_empty() {
            return HashMap::new();
        }
        let needed = self.expand_closure_needed_names(names);
        let mut env = HashMap::new();
        for frame in &self.scope_stack {
            let param_names: std::collections::HashSet<&str> = frame
                .get("__params__")
                .and_then(|v| if let Kv8Value::Str(s) = v { Some(s.as_str()) } else { None })
                .map(|s| s.split(',').filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            for (k, v) in frame {
                if k == "__params__" || k == "__hoisted_fns__" || k == "__is_closure_frame__" {
                    continue;
                }
                if !needed.contains(k.as_str()) {
                    continue;
                }
                if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. })
                    && !param_names.contains(k.as_str())
                {
                    env.insert(k.clone(), v.clone());
                    continue;
                }
                env.insert(k.clone(), v.clone());
            }
        }
        env
    }

    /// Expand `seed` with free names of any hoisted/captured functions transitively.
    fn expand_closure_needed_names(
        &self,
        seed: &std::collections::HashSet<String>,
    ) -> std::collections::HashSet<String> {
        let mut needed = seed.clone();
        loop {
            let mut added = false;
            for frame in &self.scope_stack {
                for name in needed.clone() {
                    let Some(v) = frame.get(&name) else {
                        continue;
                    };
                    let free = match v {
                        Kv8Value::Fun { params, body, .. } => {
                            super::opt::free_names_for_function(params, body)
                        }
                        Kv8Value::AsyncFun { params, body, .. } => {
                            super::opt::free_names_for_function(params, body)
                        }
                        _ => continue,
                    };
                    for n in free {
                        if needed.insert(n) {
                            added = true;
                        }
                    }
                }
            }
            if !added {
                break;
            }
        }
        needed
    }

    pub fn scope_collapse_to_global(&mut self) {
        while self.scope_stack.len() > 1 {
            self.scope_pop_preserve();
        }
    }

    /// After a large factory bundle finishes, replace stale first-or-insert hoists in
    /// `module_bindings` with the final `Fun` values still present on scope frames.
   pub fn finalize_module_hoists(&mut self, names: &[&str]) {
    for &name in names {
        let latest = self
            .hoist_latest
            .get(name)
            .cloned()
            .or_else(|| {
                self.scope_stack.iter().rev().find_map(|frame| {
                    frame.get(name).and_then(|v| {
                        if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                            Some(v.clone())
                        } else {
                            None
                        }
                    })
                })
            })
            .or_else(|| {
                // 👇 NYTT: Läs från module_bindings
                self.module_bindings.get(name).and_then(|v| {
                    if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
            });
        if let Some(v) = latest {
            self.hoist_latest.insert(name.to_string(), v.clone());
            self.module_bindings.insert(name.to_string(), v);
        }
    }
    self.refresh_closure_hoist_snapshots(names);
    self.refresh_hoist_slots_in_scope(names);
    self.hoists_finalized = true;
}
    /// Replace stale `undefined`/early hoists on every scope frame with finalized bindings.
    fn refresh_hoist_slots_in_scope(&mut self, names: &[&str]) {
        for &name in names {
            let Some(v) = self
                .hoist_latest
                .get(name)
                .or_else(|| self.module_bindings.get(name))
                .cloned()
            else {
                continue;
            };
            if !matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                continue;
            }
            for frame in &mut self.scope_stack {
                if frame.contains_key(name) {
                    frame.insert(name.to_string(), v.clone());
                }
            }
        }
    }

    /// Patch captured hoist helpers inside closure envs so `mm` etc. see final `g2`/`Dl`.
    fn refresh_closure_hoist_snapshots(&mut self, names: &[&str]) {
        let live: HashMap<String, Kv8Value> = names
            .iter()
            .filter_map(|&n| {
                self.hoist_latest
                    .get(n)
                    .or_else(|| self.module_bindings.get(n))
                    .filter(|v| matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }))
                    .map(|v| (n.to_string(), v.clone()))
            })
            .collect();
        if live.is_empty() {
            return;
        }
        let patch = |val: &mut Kv8Value| {
            let closure = match val {
                Kv8Value::Fun { closure, .. } | Kv8Value::AsyncFun { closure, .. } => closure,
                _ => return,
            };
            if let Ok(mut map) = closure.lock() {
                for (k, v) in &live {
                    if map.contains_key(k) {
                        map.insert(k.clone(), v.clone());
                    }
                }
            }
        };
        let keys: Vec<String> = self.module_bindings.keys().cloned().collect();
        for k in keys {
            if let Some(v) = self.module_bindings.get_mut(&k) {
                patch(v);
            }
        }
        for frame in &mut self.scope_stack {
            for v in frame.values_mut() {
                patch(v);
            }
        }
    }

    pub fn publish_hoisted_fn(&mut self, name: &str, value: &Kv8Value) {
        if matches!(value, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
            self.hoist_latest.insert(name.to_string(), value.clone());
            self.module_bindings
                .entry(name.to_string())
                .or_insert_with(|| value.clone());
            for frame in &mut self.scope_stack {
                if matches!(
                    frame.get(name),
                    Some(Kv8Value::Undefined)
                        | Some(Kv8Value::Fun { .. })
                        | Some(Kv8Value::AsyncFun { .. })
                ) {
                    frame.insert(name.to_string(), value.clone());
                }
            }
        }
    }

   pub fn scope_pop_preserve(&mut self) {
    if self.scope_stack.len() <= 1 {
        return;
    }
    let is_module_level = self.scope_stack.len() == 2;
    let Some(frame) = self.scope_stack.pop() else {
        return;
    };

    let get_names = |key: &str| -> std::collections::HashSet<String> {
        frame.get(key)
            .and_then(|v| if let Kv8Value::Str(s) = v { Some(s.as_str()) } else { None })
            .map(|s| s.split(',').filter(|p| !p.is_empty()).map(|p| p.to_string()).collect())
            .unwrap_or_default()
    };

    // esbuild `At(factory)` runs the factory at depth>2; publish hoisted helpers
    // on every pop so createRoot/mm can still resolve g2, Dl, df, … via module_bindings.
    let hoisted_fn_names = get_names("__hoisted_fns__");

    // Always hoist important functions from all frames.
    let important_fns = ["mm", "g2", "Dl", "ih", "df", "Pc", "ui", "qi", "yh"];
    for (k, v) in &frame {
        if important_fns.contains(&k.as_str()) {
            if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                self.publish_hoisted_fn(k, v);
            }
        }
    }

    for (k, v) in &frame {
        if hoisted_fn_names.contains(k.as_str()) {
            if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
                self.publish_hoisted_fn(k, v);
            }
        }
    }

    if !is_module_level {
        return;
    }

    // Closure-call frames (e.g. an At-wrapper) must not promote their locals.
    if matches!(frame.get("__is_closure_frame__"), Some(Kv8Value::Bool(true))) {
        return;
    }

    let param_names = get_names("__params__");
    for (k, v) in &frame {
        if k == "__params__" || k == "__hoisted_fns__" || param_names.contains(k.as_str()) {
            continue;
        }
        if hoisted_fn_names.contains(k.as_str()) {
            continue;
        }
        if matches!(v, Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }) {
            self.publish_hoisted_fn(k, v);
        }
    }
}
    pub fn scope_resolve_mut(&mut self, name: &str) -> Option<&mut Kv8Value> {
        let skip_module = self.closure_assign_active(name);
        for frame in self.scope_stack.iter_mut().rev() {
            if frame.contains_key(name) {
                return frame.get_mut(name);
            }
        }
        if skip_module {
            return None;
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

    pub fn closure_call_enter(&mut self, env: Kv8ClosureEnv) {
        let names = env
            .lock()
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        self.closure_assign_stack.push(names);
        self.closure_env_stack.push(env);
    }

    pub fn closure_call_exit(&mut self) {
        self.closure_env_stack.pop();
        self.closure_assign_stack.pop();
    }

    pub fn closure_assign_push(&mut self, names: HashSet<String>) {
        self.closure_assign_stack.push(names);
    }

    pub fn closure_assign_pop(&mut self) {
        self.closure_assign_stack.pop();
    }

    pub fn closure_assign_active(&self, name: &str) -> bool {
        self.closure_assign_stack
            .last()
            .map(|s| s.contains(name))
            .unwrap_or(false)
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
        for stmt in frame.stmts.as_slice().iter().skip(frame.index.saturating_add(1)) {
            if let super::ast::Stmt::Function(n, params, body) = stmt {
                if n == name {
                    let closure = Arc::new(Mutex::new(self.capture_lexical_env()));
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
