use crate::ast::Expr;
use crate::class::{ClassDef, ClassInstance, ClassRegistry, MethodDef, SharedClassInstance};
use crate::runtime::{
    db::DbConnection, http::HttpRouter, kabootar_browser::KabootarBrowser, kabootar_dom::DomNode,
    kv8::Kv8Context, os::OsHandle, security::{DeviceHandle, SecureBytes, SecurityHandle},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Shared promise state — clones of `Value::Promise` refer to the same cell.
pub type SharedPromise = Rc<RefCell<PromiseValue>>;

/// Body executed when a microtask runs.
#[derive(Debug, Clone)]
pub enum AsyncBody {
    Ast(Expr),
    Bytecode(std::rc::Rc<crate::bytecode::BytecodeFnDef>),
}

/// Bytecode function value with the defining environment for `LoadGlobal`.
#[derive(Clone)]
pub struct BytecodeFunction {
    pub def: std::rc::Rc<crate::bytecode::BytecodeFnDef>,
    pub closure: Environment,
}

impl std::fmt::Debug for BytecodeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BytecodeFunction({})", self.def.name)
    }
}

/// Async task scheduled on the microtask queue.
#[derive(Debug, Clone)]
pub struct Microtask {
    pub promise: SharedPromise,
    pub params: Vec<String>,
    pub body: AsyncBody,
    pub env: Environment,
    pub args: Vec<Value>,
    /// Pre-bound parameter values (default/rest expansion); when non-empty, used instead of zip.
    pub bindings: Vec<(String, Value)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeAt {
    Tick(u64),
    WallMs(u64),
}

#[derive(Debug, Clone)]
pub struct SleepWaiter {
    pub id: u64,
    pub promise: SharedPromise,
    pub wake: WakeAt,
    pub callback: Option<(Value, Vec<Value>)>,
    /// When set, the timer reschedules itself after each wake (`set_interval`).
    pub repeat_interval: Option<u64>,
    /// Whether `repeat_interval` is wall-clock milliseconds (`true`) or scheduler ticks (`false`).
    pub repeat_wall_ms: bool,
}

#[derive(Debug, Clone)]
pub struct TimerCallback {
    pub func: Value,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct IoTask {
    pub promise: SharedPromise,
    pub wake_at: u64,
    pub op: IoOp,
    pub abort_id: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum IoOp {
    OsRead(String),
    OsWrite { path: String, content: String },
    HttpRequest {
        method: String,
        path: String,
        body: String,
    },
    Sql { query: String, params: Vec<Value> },
    HttpFetch {
        method: String,
        url: String,
        body: String,
        headers: HashMap<String, String>,
        timeout_ms: u64,
    },
    StreamRead { stream_id: u64 },
    StreamReadAll { stream_id: u64 },
    StreamPipeTo { src_id: u64, dest_id: u64 },
    ReaderRead { reader_id: u64 },
    KvListenRead { stream_id: u64 },
    WorkerRecv { worker_id: u64 },
    WorkerPoll { timeout_ms: u64 },
}

#[derive(Debug)]
pub struct Scheduler {
    pub queue: RefCell<VecDeque<Microtask>>,
    pub ticks: RefCell<u64>,
    pub sleeps: RefCell<VecDeque<SleepWaiter>>,
    pub io_queue: RefCell<VecDeque<IoTask>>,
    pub timer_callbacks: RefCell<VecDeque<TimerCallback>>,
    pub microtask_callbacks: RefCell<VecDeque<TimerCallback>>,
    pub next_timer_id: RefCell<u64>,
    pub cancelled_timer_ids: RefCell<HashSet<u64>>,
    pub tls_trust: RefCell<crate::runtime::tls_trust::TlsTrust>,
    /// Global default for `http_fetch_async` when no per-request timeout is given (0 = none).
    pub http_fetch_timeout_ms: RefCell<u64>,
}

/// Promise lifecycle — `Pending` until the scheduler runs the enqueued microtask.
#[derive(Debug, Clone)]
pub enum PromiseValue {
    Pending,
    Resolved(Value),
}

/// Runtime value model for Kabootar.
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Number(i64),
    Float(f64),
    BigInt(num_bigint::BigInt),
    String(String),
    Bool(bool),
    /// Unique symbol id — metadata in `symbol` module registry.
    Symbol(u64),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Function {
        name: String,
        params: Vec<String>,
        defaults: Vec<Option<Expr>>,
        rest: Option<String>,
        body: Expr,
        env: Environment,
        public: bool,
        async_fn: bool,
    },
    Promise(SharedPromise),
    /// Callable `resolve` / `reject` from `promise_new(executor)`.
    PromiseSettler { ctrl_id: u64, reject: bool },
    NativeFunction(fn(&[Value], &mut Environment) -> Result<Value, String>),
    BytecodeFn(BytecodeFunction),
    #[cfg(target_arch = "wasm32")]
    BrowserDom(wasm_bindgen::prelude::JsValue),
    #[cfg(not(target_arch = "wasm32"))]
    BrowserDom,
    KabootarDom(DomNode),
    KabootarBrowser(KabootarBrowser),
    Kv8Context(Kv8Context),
    ClassInstance(crate::class::SharedClassInstance),
    BoundMethod(crate::class::SharedClassInstance, MethodDef),
    /// Native method bound to a receiver (`obj.method()` → prepend `obj` as first arg).
    BoundNative(Box<Value>, fn(&[Value], &mut Environment) -> Result<Value, String>),
    /// Enum type namespace, e.g. `Color` in `Color.Red`.
    EnumNamespace(String),
    /// Callable enum variant constructor, e.g. `Msg.Move`.
    EnumCtor {
        type_name: String,
        variant: String,
        arity: usize,
    },
    /// Enum variant value.
    EnumValue {
        type_name: String,
        variant: String,
        fields: Vec<Value>,
    },
    /// Python-style `range(start, end, step)` — end exclusive.
    Range {
        start: i64,
        end: i64,
        step: i64,
    },
    OsHandle(OsHandle),
    DbConnection(DbConnection),
    HttpResponse(crate::runtime::http::HttpResponse),
    HttpRouter(HttpRouter),
    SecureBytes(SecureBytes),
    SecurityHandle(SecurityHandle),
    DeviceHandle(DeviceHandle),
    Break,
    Continue,
    Fallthrough,
}

impl Value {
    pub fn is_undefined(&self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_nan(&self) -> bool {
        matches!(self, Value::Float(n) if n.is_nan())
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null | Value::Bool(false) => false,
            Value::Number(0) | Value::Float(0.0) => false,
            Value::BigInt(b) => b != &num_bigint::BigInt::from(0),
            Value::String(s) => !s.is_empty(),
            Value::Array(items) => !items.is_empty(),
            Value::Object(map) => !map.is_empty(),
            Value::Float(n) => !n.is_nan() && *n != 0.0,
            Value::Break | Value::Continue | Value::Fallthrough => false,
            Value::Promise(_) => true,
            _ => true,
        }
    }
}

struct EnvironmentInner {
    bindings: RefCell<HashMap<String, Value>>,
    immutable: RefCell<HashSet<String>>,
    parent: Option<Rc<EnvironmentInner>>,
    classes: ClassRegistry,
    exports: RefCell<HashSet<String>>,
    scheduler: Rc<Scheduler>,
    private_access_class: RefCell<Option<String>>,
}

/// Lexical environment — `Clone` snapshots the current frame; parent chain stays shared.
pub struct Environment {
    inner: Rc<EnvironmentInner>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::new(EnvironmentInner {
                bindings: RefCell::new(self.inner.bindings.borrow().clone()),
                immutable: RefCell::new(self.inner.immutable.borrow().clone()),
                parent: self.inner.parent.clone(),
                classes: self.inner.classes.clone(),
                exports: RefCell::new(self.inner.exports.borrow().clone()),
                scheduler: self.inner.scheduler.clone(),
                private_access_class: RefCell::new(self.inner.private_access_class.borrow().clone()),
            }),
        }
    }
}

impl Environment {
    /// Snapshot bindings for closure refresh without embedding other module functions.
    pub fn clone_excluding(&self, names: &[&str]) -> Self {
        let mut bindings = self.inner.bindings.borrow().clone();
        for name in names {
            bindings.remove(*name);
        }
        Self {
            inner: Rc::new(EnvironmentInner {
                bindings: RefCell::new(bindings),
                immutable: RefCell::new(self.inner.immutable.borrow().clone()),
                parent: self.inner.parent.clone(),
                classes: self.inner.classes.clone(),
                exports: RefCell::new(self.inner.exports.borrow().clone()),
                scheduler: self.inner.scheduler.clone(),
                private_access_class: RefCell::new(self.inner.private_access_class.borrow().clone()),
            }),
        }
    }
}

impl Clone for EnvironmentInner {
    fn clone(&self) -> Self {
        Self {
            bindings: RefCell::new(self.bindings.borrow().clone()),
            immutable: RefCell::new(self.immutable.borrow().clone()),
            parent: self.parent.clone(),
            classes: self.classes.clone(),
            exports: RefCell::new(self.exports.borrow().clone()),
            scheduler: self.scheduler.clone(),
            private_access_class: RefCell::new(self.private_access_class.borrow().clone()),
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(EnvironmentInner {
                bindings: RefCell::new(HashMap::new()),
                immutable: RefCell::new(HashSet::new()),
                parent: None,
                classes: ClassRegistry::default(),
                exports: RefCell::new(HashSet::new()),
                scheduler: Rc::new(Scheduler {
                    queue: RefCell::new(VecDeque::new()),
                    ticks: RefCell::new(0),
                    sleeps: RefCell::new(VecDeque::new()),
                    io_queue: RefCell::new(VecDeque::new()),
                    timer_callbacks: RefCell::new(VecDeque::new()),
                    microtask_callbacks: RefCell::new(VecDeque::new()),
                    next_timer_id: RefCell::new(1),
                    cancelled_timer_ids: RefCell::new(HashSet::new()),
                    tls_trust: RefCell::new(crate::runtime::tls_trust::TlsTrust::default()),
                    http_fetch_timeout_ms: RefCell::new(0),
                }),
                private_access_class: RefCell::new(None),
            }),
        }
    }

    pub fn set_private_scope(&self, class_name: Option<&str>) {
        *self.inner.private_access_class.borrow_mut() = class_name.map(str::to_string);
    }

    pub fn private_access_class(&self) -> Option<String> {
        self.inner.private_access_class.borrow().clone()
    }

    pub fn child(parent: Environment) -> Self {
        let scheduler = parent.inner.scheduler.clone();
        Self {
            inner: Rc::new(EnvironmentInner {
                bindings: RefCell::new(HashMap::new()),
                immutable: RefCell::new(HashSet::new()),
                parent: Some(parent.inner),
                classes: ClassRegistry::default(),
                exports: RefCell::new(HashSet::new()),
                scheduler,
                private_access_class: RefCell::new(None),
            }),
        }
    }

    /// Child frame sharing `parent`'s bindings via `Rc` (no deep clone of parent locals).
    pub fn child_from(parent: &Environment) -> Self {
        let scheduler = parent.inner.scheduler.clone();
        Self {
            inner: Rc::new(EnvironmentInner {
                bindings: RefCell::new(HashMap::new()),
                immutable: RefCell::new(HashSet::new()),
                parent: Some(Rc::clone(&parent.inner)),
                classes: ClassRegistry::default(),
                exports: RefCell::new(HashSet::new()),
                scheduler,
                private_access_class: RefCell::new(parent.inner.private_access_class.borrow().clone()),
            }),
        }
    }

    pub fn schedule_microtask(&self, task: Microtask) {
        self.inner
            .scheduler
            .queue
            .borrow_mut()
            .push_back(task);
    }

    pub fn pop_microtask(&self) -> Option<Microtask> {
        self.inner.scheduler.queue.borrow_mut().pop_front()
    }

    pub fn has_microtasks(&self) -> bool {
        !self.inner.scheduler.queue.borrow().is_empty()
    }

    pub fn current_tick(&self) -> u64 {
        *self.inner.scheduler.ticks.borrow()
    }

    pub fn schedule_sleep(&self, waiter: SleepWaiter) {
        self.inner.scheduler.sleeps.borrow_mut().push_back(waiter);
    }

    pub fn alloc_timer_id(&self) -> u64 {
        let mut next = self.inner.scheduler.next_timer_id.borrow_mut();
        let id = *next;
        *next += 1;
        id
    }

    pub fn cancel_timer(&self, id: u64) {
        self.inner
            .scheduler
            .cancelled_timer_ids
            .borrow_mut()
            .insert(id);
    }

    pub fn schedule_timer_callback(&self, func: Value, args: Vec<Value>) {
        self.inner
            .scheduler
            .timer_callbacks
            .borrow_mut()
            .push_back(TimerCallback { func, args });
    }

    pub fn pop_timer_callback(&self) -> Option<TimerCallback> {
        self.inner.scheduler.timer_callbacks.borrow_mut().pop_front()
    }

    pub fn schedule_microtask_callback(&self, func: Value, args: Vec<Value>) {
        self.inner
            .scheduler
            .microtask_callbacks
            .borrow_mut()
            .push_back(TimerCallback { func, args });
    }

    pub fn pop_microtask_callback(&self) -> Option<TimerCallback> {
        self.inner
            .scheduler
            .microtask_callbacks
            .borrow_mut()
            .pop_front()
    }

    pub fn has_microtask_callbacks(&self) -> bool {
        !self.inner.scheduler.microtask_callbacks.borrow().is_empty()
    }

    pub fn has_timer_callbacks(&self) -> bool {
        !self.inner.scheduler.timer_callbacks.borrow().is_empty()
    }

    pub fn has_pending_sleeps(&self) -> bool {
        !self.inner.scheduler.sleeps.borrow().is_empty()
    }

    pub fn has_pending_wall_sleeps(&self) -> bool {
        self.inner
            .scheduler
            .sleeps
            .borrow()
            .iter()
            .any(|w| matches!(w.wake, WakeAt::WallMs(_)))
    }

    /// Strong references held by scheduled async work (microtasks, timers, sleeps).
    pub(crate) fn gc_scheduler_roots(&self) -> Vec<Value> {
        let mut roots = Vec::new();
        let sched = &self.inner.scheduler;
        for task in sched.queue.borrow().iter() {
            roots.extend(task.args.iter().cloned());
            for (_, v) in &task.bindings {
                roots.push(v.clone());
            }
            for name in task.env.all_binding_names() {
                if let Some(v) = task.env.get(&name) {
                    roots.push(v);
                }
            }
        }
        for cb in sched.microtask_callbacks.borrow().iter() {
            roots.push(cb.func.clone());
            roots.extend(cb.args.iter().cloned());
        }
        for cb in sched.timer_callbacks.borrow().iter() {
            roots.push(cb.func.clone());
            roots.extend(cb.args.iter().cloned());
        }
        for waiter in sched.sleeps.borrow().iter() {
            if let Some((func, args)) = &waiter.callback {
                roots.push(func.clone());
                roots.extend(args.iter().cloned());
            }
        }
        roots
    }

    pub fn ms_until_wall_wake(&self) -> Option<u64> {
        let now = unix_ms_now();
        self.inner
            .scheduler
            .sleeps
            .borrow()
            .iter()
            .filter_map(|w| match w.wake {
                WakeAt::WallMs(ms) if ms > now => Some(ms - now),
                _ => None,
            })
            .min()
    }

    pub fn wake_ready_sleeps(&self) -> bool {
        {
            let mut ticks = self.inner.scheduler.ticks.borrow_mut();
            *ticks += 1;
        }
        let tick = self.current_tick();
        let now = unix_ms_now();
        let mut sleeps = self.inner.scheduler.sleeps.borrow_mut();
        let cancelled = self.inner.scheduler.cancelled_timer_ids.borrow();
        let mut woke = false;
        let mut remaining = VecDeque::new();
        while let Some(waiter) = sleeps.pop_front() {
            let ready = match waiter.wake {
                WakeAt::Tick(at) => tick >= at,
                WakeAt::WallMs(at) => now >= at,
            };
            if ready {
                if !cancelled.contains(&waiter.id) {
                    if waiter.repeat_interval.is_none() {
                        *waiter.promise.borrow_mut() = PromiseValue::Resolved(Value::Null);
                    }
                    if let Some((func, args)) = waiter.callback.clone() {
                        self.schedule_timer_callback(func, args);
                    }
                    if let Some(every) = waiter.repeat_interval {
                        if !cancelled.contains(&waiter.id) {
                            let next_wake = if waiter.repeat_wall_ms {
                                WakeAt::WallMs(now.saturating_add(every))
                            } else {
                                WakeAt::Tick(tick.saturating_add(every))
                            };
                            remaining.push_back(SleepWaiter {
                                id: waiter.id,
                                promise: waiter.promise.clone(),
                                wake: next_wake,
                                callback: waiter.callback.clone(),
                                repeat_interval: Some(every),
                                repeat_wall_ms: waiter.repeat_wall_ms,
                            });
                        }
                    }
                }
                woke = true;
            } else {
                remaining.push_back(waiter);
            }
        }
        *sleeps = remaining;
        woke
    }

    pub fn advance_tick_and_wake_sleeps(&self) -> bool {
        self.wake_ready_sleeps()
    }

    pub fn schedule_io(&self, task: IoTask) {
        self.inner.scheduler.io_queue.borrow_mut().push_back(task);
    }

    pub fn pop_ready_io(&self, tick: u64) -> Option<IoTask> {
        let mut q = self.inner.scheduler.io_queue.borrow_mut();
        if let Some(pos) = q.iter().position(|t| t.wake_at <= tick) {
            q.remove(pos)
        } else {
            None
        }
    }

    pub fn has_pending_io(&self) -> bool {
        !self.inner.scheduler.io_queue.borrow().is_empty()
    }

    pub fn cancel_io_by_abort_id(&self, id: u64, reason: Value) {
        let mut q = self.inner.scheduler.io_queue.borrow_mut();
        let cancelled: Vec<SharedPromise> = q
            .iter()
            .filter(|t| t.abort_id == Some(id))
            .map(|t| t.promise.clone())
            .collect();
        q.retain(|t| t.abort_id != Some(id));
        drop(q);
        for p in cancelled {
            *p.borrow_mut() = PromiseValue::Resolved(Value::Result(Err(Box::new(
                reason.clone(),
            ))));
        }
    }

    pub fn tls_trust_mut(&self) -> std::cell::RefMut<'_, crate::runtime::tls_trust::TlsTrust> {
        self.inner.scheduler.tls_trust.borrow_mut()
    }

    pub fn tls_trust(&self) -> crate::runtime::tls_trust::TlsTrust {
        self.inner.scheduler.tls_trust.borrow().clone()
    }

    pub fn http_fetch_timeout_ms(&self) -> u64 {
        *self.inner.scheduler.http_fetch_timeout_ms.borrow()
    }

    pub fn http_fetch_timeout_ms_mut(&self) -> std::cell::RefMut<'_, u64> {
        self.inner.scheduler.http_fetch_timeout_ms.borrow_mut()
    }

    pub fn mark_exported(&mut self, name: impl Into<String>) {
        self.inner.exports.borrow_mut().insert(name.into());
    }

    pub fn is_exported(&self, name: &str) -> bool {
        self.inner.exports.borrow().contains(name)
    }

    pub fn exported_names(&self) -> Vec<String> {
        self.inner.exports.borrow().iter().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let mut current = Some(self.inner.as_ref());
        while let Some(node) = current {
            if let Some(v) = node.bindings.borrow().get(name) {
                return Some(v.clone());
            }
            current = node.parent.as_ref().map(Rc::as_ref);
        }
        None
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.inner.bindings.borrow_mut().insert(name, value);
    }

    pub fn set_const(&mut self, name: String, value: Value) {
        self.inner.bindings.borrow_mut().insert(name.clone(), value);
        self.inner.immutable.borrow_mut().insert(name);
    }

    pub fn is_immutable(&self, name: &str) -> bool {
        let mut current = Some(self.inner.as_ref());
        while let Some(node) = current {
            if node.immutable.borrow().contains(name) {
                return true;
            }
            current = node.parent.as_ref().map(Rc::as_ref);
        }
        false
    }

    pub fn local_names(&self) -> Vec<String> {
        self.inner.bindings.borrow().keys().cloned().collect()
    }

    pub fn all_binding_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        let mut current = Some(self.inner.as_ref());
        while let Some(node) = current {
            for key in node.bindings.borrow().keys() {
                if seen.insert(key.clone()) {
                    names.push(key.clone());
                }
            }
            current = node.parent.as_ref().map(Rc::as_ref);
        }
        names
    }

    pub fn get_class(&self, name: &str) -> Option<ClassDef> {
        let mut current = Some(self.inner.as_ref());
        while let Some(node) = current {
            if let Some(def) = node.classes.get(name) {
                return Some(def.clone());
            }
            current = node.parent.as_ref().map(Rc::as_ref);
        }
        None
    }

    pub fn get_interface(&self, name: &str) -> Option<crate::class::InterfaceDef> {
        let mut current = Some(self.inner.as_ref());
        while let Some(node) = current {
            if let Some(def) = node.classes.interfaces.get(name) {
                return Some(def.clone());
            }
            current = node.parent.as_ref().map(Rc::as_ref);
        }
        None
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.is_immutable(name) {
            return Err(format!("Cannot assign to const `{}`", name));
        }
        let mut current = Some(self.inner.clone());
        while let Some(node) = current {
            if node.bindings.borrow().contains_key(name) {
                node.bindings
                    .borrow_mut()
                    .insert(name.to_string(), value);
                return Ok(());
            }
            current = node.parent.clone();
        }
        Err(format!("Undefined variable: {}", name))
    }

    pub fn classes(&self) -> &ClassRegistry {
        &self.inner.classes
    }

    pub fn classes_mut(&mut self) -> &mut ClassRegistry {
        &mut Rc::make_mut(&mut self.inner).classes
    }
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("bindings", &*self.inner.bindings.borrow())
            .field("has_parent", &self.inner.parent.is_some())
            .finish()
    }
}

pub fn format_value(val: &Value) -> String {
    match val {
        Value::Undefined => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Number(n) => n.to_string(),
        Value::BigInt(b) => crate::runtime::stdlib::bigint::format_bigint(b),
        Value::Float(n) if n.is_nan() => "NaN".to_string(),
        Value::Float(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Symbol(id) => crate::runtime::stdlib::symbol::format_symbol(*id),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(format_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}: {}", k, format_value(&map[k])))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Option(opt) => match opt {
            Some(v) => format_value(v),
            None => "None".to_string(),
        },
        Value::Result(res) => match res {
            Ok(v) => format!("Ok({})", format_value(v)),
            Err(e) => format!("Err({})", format_value(e)),
        },
        Value::Function { name, .. } => format!("<function {}>", name),
        Value::Promise(_) => "<promise>".to_string(),
        Value::PromiseSettler { reject, .. } => {
            if *reject {
                "<promise reject>".to_string()
            } else {
                "<promise resolve>".to_string()
            }
        }
        Value::NativeFunction(_) => "<native function>".to_string(),
        Value::BytecodeFn(f) => format!("<bytecode fn {}>", f.def.name),
        #[cfg(target_arch = "wasm32")]
        Value::BrowserDom(_) => "<browser DOM>".to_string(),
        #[cfg(not(target_arch = "wasm32"))]
        Value::BrowserDom => "<browser DOM>".to_string(),
        Value::KabootarDom(node) => format!("<kabootar-dom {}>", node.tag),
        Value::KabootarBrowser(_) => "<kabootar-browser>".to_string(),
        Value::Kv8Context(_) => "<kv8-context>".to_string(),
        Value::ClassInstance(inst) => {
            let name = inst
                .try_borrow()
                .map(|i| i.class_name.clone())
                .unwrap_or_else(|_| "<borrowed>".to_string());
            format!("<{} instance>", name)
        }
        Value::BoundMethod(inst, method) => {
            let name = inst
                .try_borrow()
                .map(|i| i.class_name.clone())
                .unwrap_or_else(|_| "<borrowed>".to_string());
            format!("<method {} on {}>", method.name, name)
        }
        Value::BoundNative(_, _) => "<bound native>".to_string(),
        Value::EnumNamespace(name) => format!("<enum {}>", name),
        Value::EnumCtor {
            type_name,
            variant,
            ..
        } => format!("<enum ctor {}.{}>", type_name, variant),
        Value::EnumValue {
            type_name,
            variant,
            fields,
        } => {
            if fields.is_empty() {
                format!("{}.{}", type_name, variant)
            } else {
                let parts: Vec<String> = fields.iter().map(format_value).collect();
                format!("{}.{}({})", type_name, variant, parts.join(", "))
            }
        }
        Value::OsHandle(os) => format!("<os {}>", os.name()),
        Value::DbConnection(db) => format!("<db {}>", db.name),
        Value::HttpResponse(res) => format!("<http {}>", res.status),
        Value::HttpRouter(_) => "<http router>".to_string(),
        Value::SecureBytes(_) => "<secure bytes>".to_string(),
        Value::SecurityHandle(_) => "<security>".to_string(),
        Value::DeviceHandle(h) => format!("<device {}>", h.device_id),
        Value::Range { start, end, step } => {
            if *step == 1 && *start == 0 {
                format!("range({})", end)
            } else if *step == 1 {
                format!("range({}, {})", start, end)
            } else {
                format!("range({}, {}, {})", start, end, step)
            }
        }
        Value::Break => "break".to_string(),
        Value::Continue => "continue".to_string(),
        Value::Fallthrough => "fallthrough".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_clone_is_shallow() {
        let mut env = Environment::new();
        env.set("x".into(), Value::Number(1));
        let cloned = env.clone();
        env.set("y".into(), Value::Number(2));
        assert!(matches!(cloned.get("x"), Some(Value::Number(1))));
        assert!(cloned.get("y").is_none());
    }

    #[test]
    fn child_sees_parent_bindings() {
        let mut parent = Environment::new();
        parent.set("a".into(), Value::Number(10));
        let child = Environment::child(parent);
        assert!(matches!(child.get("a"), Some(Value::Number(10))));
    }
}
