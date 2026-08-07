//! Property descriptors — `Object.defineProperty`, getters/setters, flags.

use crate::bytecode::call_value;
use crate::runtime::stdlib::object::{check_can_delete, check_can_set, is_extensible};
use crate::runtime::stdlib::symbol::symbol_value;
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub const DESCRIPTORS_KEY: &str = "__kab_descriptors";
pub const SYM_PROPS_KEY: &str = "__kab_sym_props";
pub const SYM_DESCRIPTORS_KEY: &str = "__kab_sym_descriptors";

#[derive(Debug, Clone)]
pub struct PropertyDescriptor {
    pub value: Option<Value>,
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
    pub get: Option<Value>,
    pub set: Option<Value>,
}

impl Default for PropertyDescriptor {
    fn default() -> Self {
        Self {
            value: None,
            writable: true,
            enumerable: true,
            configurable: true,
            get: None,
            set: None,
        }
    }
}

fn is_internal_key(key: &str) -> bool {
    key.starts_with("__kab_")
}

fn descriptors_table(map: &HashMap<String, Value>) -> Option<&HashMap<String, Value>> {
    match map.get(DESCRIPTORS_KEY) {
        Some(Value::Object(d)) => Some(d),
        _ => None,
    }
}

fn descriptors_table_mut(map: &mut HashMap<String, Value>) -> &mut HashMap<String, Value> {
    if !map.contains_key(DESCRIPTORS_KEY) {
        map.insert(DESCRIPTORS_KEY.into(), Value::from_object(HashMap::new()));
    }
    match map.get_mut(DESCRIPTORS_KEY) {
        Some(Value::Object(ref mut d)) => Rc::make_mut(d),
        _ => unreachable!("descriptors slot must be object"),
    }
}

fn bool_field(desc: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match desc.get(key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

fn parse_stored_descriptor(desc: &HashMap<String, Value>) -> PropertyDescriptor {
    let has_get = desc.contains_key("get");
    let has_set = desc.contains_key("set");
    let accessor = has_get || has_set;
    PropertyDescriptor {
        value: if accessor {
            None
        } else {
            desc.get("value").cloned()
        },
        writable: bool_field(desc, "writable", true),
        enumerable: bool_field(desc, "enumerable", true),
        configurable: bool_field(desc, "configurable", true),
        get: desc.get("get").cloned().filter(|v| !v.is_undefined()),
        set: desc.get("set").cloned().filter(|v| !v.is_undefined()),
    }
}

fn stored_descriptor(map: &HashMap<String, Value>, key: &str) -> Option<PropertyDescriptor> {
    descriptors_table(map)
        .and_then(|d| d.get(key))
        .and_then(|v| match v {
            Value::Object(fields) => Some(parse_stored_descriptor(fields)),
            _ => None,
        })
}

pub fn default_descriptor_for_value(value: Value) -> PropertyDescriptor {
    PropertyDescriptor {
        value: Some(value),
        ..PropertyDescriptor::default()
    }
}

pub fn effective_descriptor(map: &HashMap<String, Value>, key: &str) -> Option<PropertyDescriptor> {
    if let Some(d) = stored_descriptor(map, key) {
        return Some(d);
    }
    if map.contains_key(key) && !is_internal_key(key) {
        return Some(default_descriptor_for_value(map.get(key)?.clone()));
    }
    None
}

pub fn has_own_property(map: &HashMap<String, Value>, key: &str) -> bool {
    if is_internal_key(key) {
        return false;
    }
    map.contains_key(key)
        || descriptors_table(map)
            .is_some_and(|d| d.contains_key(key))
}

pub fn own_property_keys(map: &HashMap<String, Value>) -> Vec<String> {
    let mut keys = HashSet::new();
    for k in map.keys() {
        if !is_internal_key(k) {
            keys.insert(k.clone());
        }
    }
    if let Some(d) = descriptors_table(map) {
        keys.extend(d.keys().cloned());
    }
    let mut out: Vec<_> = keys.into_iter().collect();
    out.sort();
    out
}

pub fn enumerable_own_keys(map: &HashMap<String, Value>) -> Vec<String> {
    own_property_keys(map)
        .into_iter()
        .filter(|k| {
            effective_descriptor(map, k)
                .map(|d| d.enumerable)
                .unwrap_or(false)
        })
        .collect()
}

pub fn descriptor_to_value(desc: &PropertyDescriptor) -> Value {
    let mut m = HashMap::new();
    if let Some(get) = &desc.get {
        m.insert("get".into(), get.clone());
    } else {
        m.insert("get".into(), Value::Undefined);
    }
    if let Some(set) = &desc.set {
        m.insert("set".into(), set.clone());
    } else {
        m.insert("set".into(), Value::Undefined);
    }
    if desc.get.is_some() || desc.set.is_some() {
        m.insert("value".into(), Value::Undefined);
        m.insert("writable".into(), Value::Bool(false));
    } else {
        m.insert(
            "value".into(),
            desc.value.clone().unwrap_or(Value::Undefined),
        );
        m.insert("writable".into(), Value::Bool(desc.writable));
    }
    m.insert("enumerable".into(), Value::Bool(desc.enumerable));
    m.insert("configurable".into(), Value::Bool(desc.configurable));
    Value::from_object(m)
}

pub fn is_descriptor_object(v: &Value) -> bool {
    let Value::Object(m) = v else {
        return false;
    };
    m.contains_key("writable")
        || m.contains_key("enumerable")
        || m.contains_key("configurable")
        || m.contains_key("get")
        || m.contains_key("set")
        || m.contains_key("value")
}

pub fn parse_descriptor_input(v: &Value) -> Result<PropertyDescriptor, String> {
    let Value::Object(m) = v else {
        return Err("property descriptor must be an object".into());
    };
    let has_get = m.contains_key("get");
    let has_set = m.contains_key("set");
    let has_value = m.contains_key("value");
    if (has_get || has_set) && has_value {
        return Err("Invalid property descriptor: cannot have both value and accessor".into());
    }
    let get = m.get("get").cloned().filter(|v| !v.is_undefined());
    let set = m.get("set").cloned().filter(|v| !v.is_undefined());
    if let Some(g) = &get {
        if !g.is_callable() {
            return Err("property descriptor get must be callable".into());
        }
    }
    if let Some(s) = &set {
        if !s.is_callable() {
            return Err("property descriptor set must be callable".into());
        }
    }
    let has_value = m.contains_key("value");
    let accessor = has_get || has_set;
    Ok(PropertyDescriptor {
        value: if accessor {
            None
        } else if has_value {
            Some(m.get("value").cloned().unwrap_or(Value::Undefined))
        } else {
            None
        },
        writable: bool_field(m, "writable", !accessor),
        enumerable: bool_field(m, "enumerable", false),
        configurable: bool_field(m, "configurable", false),
        get,
        set,
    })
}

pub(crate) fn is_callable_value(v: &Value) -> bool {
    matches!(
        v,
        Value::Function { .. }
            | Value::NativeFunction(_)
            | Value::BytecodeFn(_)
            | Value::BoundMethod(_, _)
            | Value::BoundNative(_, _)
    )
}

pub fn define_property(
    map: &mut HashMap<String, Value>,
    key: &str,
    desc: PropertyDescriptor,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    if is_internal_key(key) {
        return Err("cannot define internal property".into());
    }
    let exists = has_own_property(map, key);
    let old = effective_descriptor(map, key);
    if exists {
        let configurable = old.as_ref().map(|d| d.configurable).unwrap_or(true);
        if !configurable {
            return Err("Cannot redefine non-configurable property".into());
        }
    } else {
        check_can_set(map, key)?;
        if !is_extensible(map) {
            return Err("Cannot add property to non-extensible object".into());
        }
    }

    let accessor = desc.get.is_some() || desc.set.is_some();
    let mut stored = HashMap::new();
    if let Some(get) = desc.get {
        stored.insert("get".into(), get);
    }
    if let Some(set) = desc.set {
        stored.insert("set".into(), set);
    }
    if accessor {
        map.remove(key);
    } else if let Some(value) = desc.value {
        map.insert(key.to_string(), value);
    }
    if !accessor {
        stored.insert(
            "value".into(),
            map.get(key).cloned().unwrap_or(Value::Undefined),
        );
        stored.insert("writable".into(), Value::Bool(desc.writable));
    }

    stored.insert("enumerable".into(), Value::Bool(desc.enumerable));
    stored.insert("configurable".into(), Value::Bool(desc.configurable));

    descriptors_table_mut(map).insert(key.to_string(), Value::from_object(stored));

    let _ = (receiver, env);
    Ok(())
}

pub fn get_own_property(
    map: &HashMap<String, Value>,
    key: &str,
    receiver: &Value,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    if !has_own_property(map, key) {
        return Ok(None);
    }
    let desc = effective_descriptor(map, key).unwrap_or_default();
    if let Some(get) = desc.get {
        return Ok(Some(call_value(
            get,
            vec![receiver.clone()],
            &[],
            &[],
            &[],
            &[],
            env,
        )?));
    }
    Ok(map.get(key).cloned())
}

pub fn set_own_property(
    map: &mut HashMap<String, Value>,
    key: &str,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    if is_internal_key(key) {
        return Err("cannot set internal property".into());
    }
    check_can_set(map, key)?;
    if let Some(desc) = effective_descriptor(map, key) {
        if let Some(set) = desc.set {
            call_value(
                set,
                vec![receiver.clone(), value],
                &[],
                &[],
                &[],
                &[],
                env,
            )?;
            return Ok(());
        }
        if desc.get.is_some() && desc.set.is_none() {
            return Err(format!("Cannot set read-only property \"{key}\""));
        }
        if !desc.writable {
            return Err(format!("Cannot assign to read-only property \"{key}\""));
        }
    } else if !is_extensible(map) {
        return Err("Cannot add property to non-extensible object".into());
    }
    map.insert(key.to_string(), value);
    Ok(())
}

pub fn delete_own_property(map: &mut HashMap<String, Value>, key: &str) -> Result<bool, String> {
    if !has_own_property(map, key) {
        return Ok(false);
    }
    check_can_delete(map)?;
    if let Some(desc) = effective_descriptor(map, key) {
        if !desc.configurable {
            return Ok(false);
        }
    }
    map.remove(key);
    if let Some(d) = descriptors_table_mut(map).remove(key) {
        let _ = d;
    }
    Ok(true)
}

pub fn get_own_property_descriptor_value(
    map: &HashMap<String, Value>,
    key: &str,
) -> Value {
    match effective_descriptor(map, key) {
        Some(mut d) => {
            if d.value.is_none() && d.get.is_none() && d.set.is_none() {
                d.value = map.get(key).cloned();
            }
            descriptor_to_value(&d)
        }
        None => Value::Undefined,
    }
}

#[derive(Debug, Clone)]
pub enum PropertyKey {
    String(String),
    Symbol(u64),
}

pub fn property_key_from_value(v: &Value) -> Result<PropertyKey, String> {
    match v {
        Value::String(s) => Ok(PropertyKey::String(s.clone())),
        Value::Symbol(id) => Ok(PropertyKey::Symbol(*id)),
        other => Err(format!("property key must be string or symbol, got {:?}", other)),
    }
}

fn sym_key(id: u64) -> String {
    id.to_string()
}

fn sym_props_table(map: &HashMap<String, Value>) -> Option<&HashMap<String, Value>> {
    match map.get(SYM_PROPS_KEY) {
        Some(Value::Object(d)) => Some(d),
        _ => None,
    }
}

fn sym_props_table_mut(map: &mut HashMap<String, Value>) -> &mut HashMap<String, Value> {
    if !map.contains_key(SYM_PROPS_KEY) {
        map.insert(SYM_PROPS_KEY.into(), Value::from_object(HashMap::new()));
    }
    match map.get_mut(SYM_PROPS_KEY) {
        Some(Value::Object(ref mut d)) => Rc::make_mut(d),
        _ => unreachable!("symbol props slot must be object"),
    }
}

fn sym_descriptors_table(map: &HashMap<String, Value>) -> Option<&HashMap<String, Value>> {
    match map.get(SYM_DESCRIPTORS_KEY) {
        Some(Value::Object(d)) => Some(d),
        _ => None,
    }
}

fn sym_descriptors_table_mut(map: &mut HashMap<String, Value>) -> &mut HashMap<String, Value> {
    if !map.contains_key(SYM_DESCRIPTORS_KEY) {
        map.insert(SYM_DESCRIPTORS_KEY.into(), Value::from_object(HashMap::new()));
    }
    match map.get_mut(SYM_DESCRIPTORS_KEY) {
        Some(Value::Object(ref mut d)) => Rc::make_mut(d),
        _ => unreachable!("symbol descriptor slot must be object"),
    }
}

fn stored_symbol_descriptor(map: &HashMap<String, Value>, sym_id: u64) -> Option<PropertyDescriptor> {
    sym_descriptors_table(map)
        .and_then(|d| d.get(&sym_key(sym_id)))
        .and_then(|v| match v {
            Value::Object(fields) => Some(parse_stored_descriptor(fields)),
            _ => None,
        })
}

pub fn effective_symbol_descriptor(map: &HashMap<String, Value>, sym_id: u64) -> Option<PropertyDescriptor> {
    if let Some(d) = stored_symbol_descriptor(map, sym_id) {
        return Some(d);
    }
    sym_props_table(map)
        .and_then(|p| p.get(&sym_key(sym_id)))
        .map(|v| default_descriptor_for_value(v.clone()))
}

pub fn has_own_symbol(map: &HashMap<String, Value>, sym_id: u64) -> bool {
    sym_props_table(map)
        .is_some_and(|p| p.contains_key(&sym_key(sym_id)))
        || sym_descriptors_table(map)
            .is_some_and(|d| d.contains_key(&sym_key(sym_id)))
}

pub fn has_own_property_key(map: &HashMap<String, Value>, key: &PropertyKey) -> bool {
    match key {
        PropertyKey::String(s) => has_own_property(map, s),
        PropertyKey::Symbol(id) => has_own_symbol(map, *id),
    }
}

pub fn own_symbol_ids(map: &HashMap<String, Value>) -> Vec<u64> {
    let mut ids = HashSet::new();
    if let Some(p) = sym_props_table(map) {
        ids.extend(p.keys().filter_map(|k| k.parse::<u64>().ok()));
    }
    if let Some(d) = sym_descriptors_table(map) {
        ids.extend(d.keys().filter_map(|k| k.parse::<u64>().ok()));
    }
    let mut out: Vec<_> = ids.into_iter().collect();
    out.sort_unstable();
    out
}

pub fn enumerable_own_symbol_ids(map: &HashMap<String, Value>) -> Vec<u64> {
    own_symbol_ids(map)
        .into_iter()
        .filter(|id| {
            effective_symbol_descriptor(map, *id)
                .map(|d| d.enumerable)
                .unwrap_or(false)
        })
        .collect()
}

pub fn get_own_property_symbols(map: &HashMap<String, Value>) -> Vec<Value> {
    own_symbol_ids(map)
        .into_iter()
        .map(symbol_value)
        .collect()
}

pub fn get_own_property_key(
    map: &HashMap<String, Value>,
    key: &PropertyKey,
    receiver: &Value,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    match key {
        PropertyKey::String(s) => get_own_property(map, s, receiver, env),
        PropertyKey::Symbol(id) => get_own_symbol(map, *id, receiver, env),
    }
}

pub fn get_own_symbol(
    map: &HashMap<String, Value>,
    sym_id: u64,
    receiver: &Value,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    if !has_own_symbol(map, sym_id) {
        return Ok(None);
    }
    let desc = effective_symbol_descriptor(map, sym_id).unwrap_or_default();
    if let Some(get) = desc.get {
        return Ok(Some(call_value(
            get,
            vec![receiver.clone()],
            &[],
            &[],
            &[],
            &[],
            env,
        )?));
    }
    Ok(sym_props_table(map).and_then(|p| p.get(&sym_key(sym_id)).cloned()))
}

pub fn set_own_property_key(
    map: &mut HashMap<String, Value>,
    key: &PropertyKey,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    match key {
        PropertyKey::String(s) => set_own_property(map, s, value, receiver, env),
        PropertyKey::Symbol(id) => set_own_symbol(map, *id, value, receiver, env),
    }
}

pub fn set_own_symbol(
    map: &mut HashMap<String, Value>,
    sym_id: u64,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    check_can_set(map, &sym_key(sym_id))?;
    if let Some(desc) = effective_symbol_descriptor(map, sym_id) {
        if let Some(set) = desc.set {
            call_value(
                set,
                vec![receiver.clone(), value],
                &[],
                &[],
                &[],
                &[],
                env,
            )?;
            return Ok(());
        }
        if desc.get.is_some() && desc.set.is_none() {
            return Err("Cannot set read-only symbol property".into());
        }
        if !desc.writable {
            return Err("Cannot assign to read-only symbol property".into());
        }
    } else if !is_extensible(map) {
        return Err("Cannot add property to non-extensible object".into());
    }
    sym_props_table_mut(map).insert(sym_key(sym_id), value);
    Ok(())
}

pub fn delete_own_property_key(map: &mut HashMap<String, Value>, key: &PropertyKey) -> Result<bool, String> {
    match key {
        PropertyKey::String(s) => delete_own_property(map, s),
        PropertyKey::Symbol(id) => delete_own_symbol(map, *id),
    }
}

pub fn delete_own_symbol(map: &mut HashMap<String, Value>, sym_id: u64) -> Result<bool, String> {
    if !has_own_symbol(map, sym_id) {
        return Ok(false);
    }
    check_can_delete(map)?;
    if let Some(desc) = effective_symbol_descriptor(map, sym_id) {
        if !desc.configurable {
            return Ok(false);
        }
    }
    sym_props_table_mut(map).remove(&sym_key(sym_id));
    sym_descriptors_table_mut(map).remove(&sym_key(sym_id));
    Ok(true)
}

pub fn define_property_key(
    map: &mut HashMap<String, Value>,
    key: PropertyKey,
    desc: PropertyDescriptor,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    match key {
        PropertyKey::String(s) => define_property(map, &s, desc, receiver, env),
        PropertyKey::Symbol(id) => define_symbol_property(map, id, desc, receiver, env),
    }
}

pub fn define_symbol_property(
    map: &mut HashMap<String, Value>,
    sym_id: u64,
    desc: PropertyDescriptor,
    receiver: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    let exists = has_own_symbol(map, sym_id);
    let old = effective_symbol_descriptor(map, sym_id);
    if exists {
        let configurable = old.as_ref().map(|d| d.configurable).unwrap_or(true);
        if !configurable {
            return Err("Cannot redefine non-configurable symbol property".into());
        }
    } else if !is_extensible(map) {
        return Err("Cannot add property to non-extensible object".into());
    }

    let accessor = desc.get.is_some() || desc.set.is_some();
    let mut stored = HashMap::new();
    if let Some(get) = desc.get {
        stored.insert("get".into(), get);
    }
    if let Some(set) = desc.set {
        stored.insert("set".into(), set);
    }
    if accessor {
        sym_props_table_mut(map).remove(&sym_key(sym_id));
    } else if let Some(value) = desc.value {
        sym_props_table_mut(map).insert(sym_key(sym_id), value);
    }
    if !accessor {
        stored.insert(
            "value".into(),
            sym_props_table(map)
                .and_then(|p| p.get(&sym_key(sym_id)))
                .cloned()
                .unwrap_or(Value::Undefined),
        );
        stored.insert("writable".into(), Value::Bool(desc.writable));
    }
    stored.insert("enumerable".into(), Value::Bool(desc.enumerable));
    stored.insert("configurable".into(), Value::Bool(desc.configurable));
    sym_descriptors_table_mut(map)
        .insert(sym_key(sym_id), Value::from_object(stored));
    let _ = (receiver, env);
    Ok(())
}

pub fn get_own_property_descriptor_key(map: &HashMap<String, Value>, key: &PropertyKey) -> Value {
    match key {
        PropertyKey::String(s) => get_own_property_descriptor_value(map, s),
        PropertyKey::Symbol(id) => match effective_symbol_descriptor(map, *id) {
            Some(mut d) => {
                if d.value.is_none() && d.get.is_none() && d.set.is_none() {
                    d.value = sym_props_table(map).and_then(|p| p.get(&sym_key(*id)).cloned());
                }
                descriptor_to_value(&d)
            }
            None => Value::Undefined,
        },
    }
}

// Extend Value with is_callable helper inline
trait CallableValue {
    fn is_callable(&self) -> bool;
}

impl CallableValue for Value {
    fn is_callable(&self) -> bool {
        is_callable_value(self)
    }
}
