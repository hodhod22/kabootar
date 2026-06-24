//! `AbortController` — cancel in-flight `fetch` / async IO.

use crate::value::{Environment, PromiseValue, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ABORT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct AbortState {
    aborted: bool,
    reason: Value,
}

thread_local! {
    static ABORT_STATES: RefCell<HashMap<u64, AbortState>> = RefCell::new(HashMap::new());
}

pub fn alloc_abort_signal() -> u64 {
    let id = NEXT_ABORT_ID.fetch_add(1, Ordering::Relaxed);
    ABORT_STATES.with(|s| {
        s.borrow_mut().insert(
            id,
            AbortState {
                aborted: false,
                reason: Value::Undefined,
            },
        );
    });
    id
}

pub fn signal_object(id: u64) -> Value {
    let state = ABORT_STATES.with(|s| {
        s.borrow()
            .get(&id)
            .cloned()
            .unwrap_or(AbortState {
                aborted: false,
                reason: Value::Undefined,
            })
    });
    let mut obj = HashMap::new();
    obj.insert("__kab_abort".into(), Value::Bool(true));
    obj.insert("__kab_abort_id".into(), Value::Number(id as i64));
    obj.insert("aborted".into(), Value::Bool(state.aborted));
    obj.insert("reason".into(), state.reason);
    Value::Object(obj)
}

pub fn signal_id(v: &Value) -> Option<u64> {
    let Value::Object(map) = v else {
        return None;
    };
    if !matches!(map.get("__kab_abort"), Some(Value::Bool(true))) {
        return None;
    }
    match map.get("__kab_abort_id") {
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    }
}

pub fn is_aborted(id: u64) -> bool {
    ABORT_STATES.with(|s| s.borrow().get(&id).is_some_and(|st| st.aborted))
}

pub fn abort_reason(id: u64) -> Value {
    ABORT_STATES.with(|s| {
        s.borrow()
            .get(&id)
            .map(|st| st.reason.clone())
            .unwrap_or(Value::Undefined)
    })
}

pub fn abort_signal(id: u64, reason: Value, env: &Environment) {
    let reason_for_io = reason.clone();
    ABORT_STATES.with(|s| {
        if let Some(st) = s.borrow_mut().get_mut(&id) {
            st.aborted = true;
            st.reason = reason;
        }
    });
    env.cancel_io_by_abort_id(id, reason_for_io);
}

pub fn rejected_abort_promise(reason: Value) -> Value {
    Value::Promise(Rc::new(RefCell::new(PromiseValue::Resolved(
        Value::Result(Err(Box::new(reason))),
    ))))
}

fn abort_controller_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = alloc_abort_signal();
    let mut ctrl = HashMap::new();
    ctrl.insert("__kab_abort_ctrl".into(), Value::Bool(true));
    ctrl.insert("__kab_abort_id".into(), Value::Number(id as i64));
    ctrl.insert("signal".into(), signal_object(id));
    Ok(Value::Object(ctrl))
}

fn abort_controller_abort_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let ctrl = args.first().ok_or("abort_controller_abort(ctrl, reason?)")?;
    let Value::Object(map) = ctrl else {
        return Err("abort_controller_abort() expects controller object".into());
    };
    let id = match map.get("__kab_abort_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid abort controller".into()),
    };
    let reason = args.get(1).cloned().unwrap_or(Value::String("AbortError".into()));
    abort_signal(id, reason.clone(), env);
    let mut out = map.clone();
    out.insert("signal".into(), signal_object(id));
    Ok(Value::Object(out))
}

fn abort_signal_aborted_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sig = args.first().ok_or("abort_signal_aborted(signal)")?;
    let Some(id) = signal_id(sig) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(is_aborted(id)))
}

pub fn register_abort(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("abort_controller_new", abort_controller_new_native),
        ("abort_controller_abort", abort_controller_abort_native),
        ("abort_signal_aborted", abort_signal_aborted_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
