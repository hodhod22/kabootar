//! Explicit resource management — `using`, `Symbol.dispose`, `close()`.

use crate::runtime::stdlib::descriptor::{get_own_property_key, property_key_from_value};
use crate::runtime::stdlib::symbol;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DISP_ID: AtomicU64 = AtomicU64::new(1);

const DISPOSED_IDS_KEY: &str = "__kab_disp_id";

thread_local! {
    static DISPOSED_IDS: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
    static DISPOSABLES: RefCell<Vec<Value>> = RefCell::new(Vec::new());
}

pub fn symbol_dispose_id() -> u64 {
    13
}

pub fn dispose_resource(resource: &Value, env: &mut Environment) {
    let Value::Object(map) = resource else {
        return;
    };
    if disposable_disposed_value(resource) {
        return;
    }

    let dispose_sym = symbol::well_known(symbol_dispose_id());
    if let Ok(pk) = property_key_from_value(&dispose_sym) {
    if let Ok(Some(dispose_fn)) = get_own_property_key(map, &pk, resource, env) {
        if is_callable(&dispose_fn) {
            let _ = crate::bytecode::call_value(dispose_fn, vec![], &[], &[], &[], &[], env);
                mark_disposable_object(resource);
                return;
            }
        }
    }

    if let Some(dispose_fn) = map.get("dispose") {
        if is_callable(dispose_fn) {
            let _ = crate::bytecode::call_value(dispose_fn.clone(), vec![], &[], &[], &[], &[], env);
            mark_disposable_object(resource);
            return;
        }
    }

    if let Some(close_fn) = map.get("close") {
        if is_callable(close_fn) {
            let _ = crate::bytecode::call_value(close_fn.clone(), vec![], &[], &[], &[], &[], env);
            mark_disposable_object(resource);
        }
    }
}

fn is_callable(v: &Value) -> bool {
    matches!(
        v,
        Value::Function { .. } | Value::NativeFunction(_) | Value::BytecodeFn(_)
    )
}

fn disposable_id(resource: &Value) -> Option<u64> {
    match resource {
        Value::Object(map) => match map.get(DISPOSED_IDS_KEY) {
            Some(Value::Number(id)) if *id >= 0 => Some(*id as u64),
            _ => None,
        },
        _ => None,
    }
}

pub fn mark_disposable_object(resource: &Value) {
    if let Some(id) = disposable_id(resource) {
        DISPOSED_IDS.with(|s| {
            s.borrow_mut().insert(id);
        });
    }
}

pub fn disposable_disposed_value(resource: &Value) -> bool {
    disposable_id(resource)
        .is_some_and(|id| DISPOSED_IDS.with(|s| s.borrow().contains(&id)))
}

pub fn disposable_depth() -> usize {
    DISPOSABLES.with(|d| d.borrow().len())
}

pub fn push_disposable(resource: Value) {
    DISPOSABLES.with(|d| d.borrow_mut().push(resource));
}

pub fn dispose_since(depth: usize, env: &mut Environment) {
    DISPOSABLES.with(|d| {
        let mut guard = d.borrow_mut();
        while guard.len() > depth {
            let resource = guard.pop().unwrap();
            drop(guard);
            dispose_resource(&resource, env);
            guard = d.borrow_mut();
        }
    });
}

fn disposable_new_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = NEXT_DISP_ID.fetch_add(1, Ordering::Relaxed);
    let mut map = HashMap::new();
    map.insert(DISPOSED_IDS_KEY.into(), Value::Number(id as i64));
    map.insert(
        "dispose".into(),
        Value::NativeFunction(disposable_dispose_native),
    );
    Ok(Value::from_object(map))
}

fn disposable_dispose_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Undefined)
}

fn disposable_disposed_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let resource = args.first().ok_or("disposable_disposed(resource)")?;
    Ok(Value::Bool(disposable_disposed_value(resource)))
}

pub fn register_disposable(env: &mut Environment) {
    env.set(
        "disposable_new".to_string(),
        Value::NativeFunction(disposable_new_native),
    );
    env.set(
        "disposable_disposed".to_string(),
        Value::NativeFunction(disposable_disposed_native),
    );
}
