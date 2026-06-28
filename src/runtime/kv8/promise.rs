//! Kv8 Promise state + microtask queue.

use super::context::Kv8Value;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Kv8PromiseState {
    Pending,
    Fulfilled(Kv8Value),
    Rejected(String),
}

#[derive(Debug, Clone)]
pub struct Kv8ThenLink {
    pub child: SharedKv8Promise,
    pub on_fulfilled: Option<Kv8Value>,
    pub on_rejected: Option<Kv8Value>,
}

#[derive(Debug)]
pub struct Kv8PromiseInner {
    pub state: Kv8PromiseState,
    pub links: Vec<Kv8ThenLink>,
}

pub type SharedKv8Promise = Rc<RefCell<Kv8PromiseInner>>;

#[derive(Debug, Clone)]
pub struct Kv8Microtask {
    pub callback: Kv8Value,
    pub args: Vec<Kv8Value>,
}

pub fn new_pending_promise() -> SharedKv8Promise {
    Rc::new(RefCell::new(Kv8PromiseInner {
        state: Kv8PromiseState::Pending,
        links: Vec::new(),
    }))
}

pub fn promise_state(promise: &SharedKv8Promise) -> Kv8PromiseState {
    promise.borrow().state.clone()
}

pub fn take_then_links(promise: &SharedKv8Promise) -> Vec<Kv8ThenLink> {
    promise.borrow_mut().links.drain(..).collect()
}

pub fn push_then_link(promise: &SharedKv8Promise, link: Kv8ThenLink) {
    promise.borrow_mut().links.push(link);
}

pub fn fulfill_promise(promise: &SharedKv8Promise, value: Kv8Value) -> Option<Vec<Kv8ThenLink>> {
    let mut g = promise.borrow_mut();
    if !matches!(g.state, Kv8PromiseState::Pending) {
        return None;
    }
    g.state = Kv8PromiseState::Fulfilled(value);
    Some(g.links.drain(..).collect())
}

pub fn reject_promise(
    promise: &SharedKv8Promise,
    message: impl Into<String>,
) -> Option<Vec<Kv8ThenLink>> {
    let mut g = promise.borrow_mut();
    if !matches!(g.state, Kv8PromiseState::Pending) {
        return None;
    }
    g.state = Kv8PromiseState::Rejected(message.into());
    Some(g.links.drain(..).collect())
}

pub fn promise_resolved(value: Kv8Value) -> Kv8Value {
    let p = new_pending_promise();
    fulfill_promise(&p, value);
    Kv8Value::Promise(p)
}

pub fn promise_rejected(message: impl Into<String>) -> Kv8Value {
    let p = new_pending_promise();
    reject_promise(&p, message);
    Kv8Value::Promise(p)
}

pub fn kv8_http_fetch(url: &str) -> Result<Kv8Value, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let res = crate::runtime::net::http_fetch_default("GET", url, "")?;
        let mut m = std::collections::HashMap::new();
        m.insert("status".into(), Kv8Value::Num(res.status as f64));
        m.insert(
            "ok".into(),
            Kv8Value::Bool(res.status >= 200 && res.status < 300),
        );
        m.insert("body".into(), Kv8Value::Str(res.body));
        Ok(Kv8Value::Obj(m))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        Err("Kv8 fetch is not available on WASM yet".into())
    }
}
