//! ECMAScript `Symbol` — unique property keys and well-known symbols.

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const WELL_KNOWN_END: u64 = 32;
static NEXT_SYMBOL_ID: AtomicU64 = AtomicU64::new(WELL_KNOWN_END + 1);

#[derive(Debug, Clone)]
struct SymbolMeta {
    description: String,
    registered: Option<String>,
}

thread_local! {
    static REGISTRY: RefCell<HashMap<u64, SymbolMeta>> = RefCell::new(HashMap::new());
    static GLOBAL_FOR: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

fn with_registry<F, T>(f: F) -> T
where
    F: FnOnce(&mut HashMap<u64, SymbolMeta>) -> T,
{
    REGISTRY.with(|r| f(&mut r.borrow_mut()))
}

fn register_meta(id: u64, description: String, registered: Option<String>) {
    with_registry(|m| {
        m.insert(
            id,
            SymbolMeta {
                description,
                registered,
            },
        );
    });
}

fn ensure_well_known() {
    with_registry(|m| {
        if !m.is_empty() {
            return;
        }
        let names = [
            (1, "Symbol.iterator"),
            (2, "Symbol.asyncIterator"),
            (3, "Symbol.hasInstance"),
            (4, "Symbol.toStringTag"),
            (5, "Symbol.species"),
            (6, "Symbol.toPrimitive"),
            (7, "Symbol.isConcatSpreadable"),
            (8, "Symbol.unscopables"),
            (9, "Symbol.match"),
            (10, "Symbol.replace"),
            (11, "Symbol.search"),
            (12, "Symbol.split"),
            (13, "Symbol.dispose"),
        ];
        for (id, name) in names {
            m.insert(
                id,
                SymbolMeta {
                    description: name.to_string(),
                    registered: None,
                },
            );
        }
    });
}

pub fn symbol_id(v: &Value) -> Option<u64> {
    match v {
        Value::Symbol(id) => Some(*id),
        _ => None,
    }
}

pub fn symbol_value(id: u64) -> Value {
    ensure_well_known();
    Value::Symbol(id)
}

pub fn symbol_description(id: u64) -> String {
    ensure_well_known();
    with_registry(|m| {
        m.get(&id)
            .map(|meta| meta.description.clone())
            .unwrap_or_else(|| "".into())
    })
}

pub fn format_symbol(id: u64) -> String {
    let desc = symbol_description(id);
    if desc.is_empty() {
        "Symbol()".into()
    } else {
        format!("Symbol({desc})")
    }
}

pub fn symbol_new(description: Option<String>) -> Value {
    ensure_well_known();
    let id = NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed);
    let desc = description.unwrap_or_default();
    register_meta(id, desc.clone(), None);
    Value::Symbol(id)
}

pub fn symbol_for(key: &str) -> Value {
    ensure_well_known();
    GLOBAL_FOR.with(|g| {
        let mut guard = g.borrow_mut();
        if let Some(&id) = guard.get(key) {
            return Value::Symbol(id);
        }
        let id = NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed);
        register_meta(id, key.to_string(), Some(key.to_string()));
        guard.insert(key.to_string(), id);
        Value::Symbol(id)
    })
}

pub fn symbol_key_for(v: &Value) -> Value {
    let Some(id) = symbol_id(v) else {
        return Value::Undefined;
    };
    ensure_well_known();
    let key = with_registry(|m| m.get(&id).and_then(|meta| meta.registered.clone()));
    match key {
        Some(k) => Value::String(k),
        None => Value::Undefined,
    }
}

pub fn well_known(id: u64) -> Value {
    ensure_well_known();
    Value::Symbol(id)
}

pub fn is_symbol_ctor_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get("__kab_symbol_ctor"), Some(Value::Bool(true))),
        _ => false,
    }
}

fn symbol_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Undefined) | None => None,
        other => {
            return Err(format!(
                "Symbol() description must be string, got {:?}",
                other
            ))
        }
    };
    Ok(symbol_new(desc))
}

fn symbol_for_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("Symbol.for(key) expects string".into()),
    };
    Ok(symbol_for(key))
}

fn symbol_key_for_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Symbol.keyFor(sym)")?;
    Ok(symbol_key_for(v))
}

fn insert_native(
    map: &mut HashMap<String, Value>,
    name: &str,
    func: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    map.insert(name.into(), Value::NativeFunction(func));
}

pub fn build_symbol_namespace() -> Value {
    ensure_well_known();
    let mut m = HashMap::new();
    m.insert("__kab_symbol_ctor".into(), Value::Bool(true));
    m.insert("iterator".into(), well_known(1));
    m.insert("asyncIterator".into(), well_known(2));
    m.insert("hasInstance".into(), well_known(3));
    m.insert("toStringTag".into(), well_known(4));
    m.insert("species".into(), well_known(5));
    m.insert("toPrimitive".into(), well_known(6));
    m.insert("isConcatSpreadable".into(), well_known(7));
    m.insert("unscopables".into(), well_known(8));
    m.insert("match".into(), well_known(9));
    m.insert("replace".into(), well_known(10));
    m.insert("search".into(), well_known(11));
    m.insert("split".into(), well_known(12));
    m.insert("dispose".into(), well_known(13));
    insert_native(&mut m, "for", symbol_for_native);
    insert_native(&mut m, "keyFor", symbol_key_for_native);
    Value::from_object(m)
}

pub fn try_symbol_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_symbol_ctor_object(callee) {
        Some(symbol_ctor_native(args, env))
    } else {
        None
    }
}

pub fn register_symbol(env: &mut Environment) {
    env.set("Symbol".to_string(), build_symbol_namespace());
}
