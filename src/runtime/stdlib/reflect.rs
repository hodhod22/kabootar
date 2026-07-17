//! ECMAScript `Reflect` — meta object operations (also used as proxy trap defaults).

use crate::runtime::stdlib::descriptor::is_callable_value;
use crate::bytecode::call_value;
use crate::runtime::stdlib::descriptor::{
    define_property_key, get_own_property_descriptor_key, get_own_property_symbols,
    has_own_property_key, own_property_keys, parse_descriptor_input, property_key_from_value,
    PropertyKey,
};
use crate::runtime::stdlib::object::{
    get_object_parent, is_extensible, object_oid, would_parent_cycle,
};
use crate::runtime::stdlib::opt::{get_member_value, is_nullish};
use crate::value::{Environment, Value};
use std::collections::HashMap;

pub(crate) fn get_internal(
    target: &Value,
    key: &Value,
    _receiver: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    match key {
        Value::String(field) => get_member_value(target, field, env),
        idx => crate::ops::read_index(target, idx, env),
    }
}

pub(crate) fn set_internal(
    target: &mut Value,
    key: &Value,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let pk = property_key_from_value(key)?;
    match target {
        Value::Object(map) => match crate::runtime::stdlib::descriptor::set_own_property_key(
            map, &pk, value, receiver, env,
        ) {
            Ok(()) => Ok(true),
            Err(e) if is_set_failure(&e) => Ok(false),
            Err(e) => Err(e),
        },
        Value::Array(items) => {
            if let PropertyKey::String(s) = &pk {
                if let Ok(i) = s.parse::<usize>() {
                    if i < items.len() {
                        items[i] = value;
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn is_set_failure(msg: &str) -> bool {
    msg.contains("Cannot") || msg.contains("read-only") || msg.contains("non-extensible")
}

pub(crate) fn has_internal(
    target: &Value,
    key: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let pk = property_key_from_value(key)?;
    match target {
        Value::Object(map) => {
            if has_own_property_key(map, &pk) {
                return Ok(true);
            }
            let parent = get_object_parent(map);
            if !is_nullish(&parent) {
                return has_internal(&parent, key, env);
            }
            Ok(false)
        }
        Value::Array(items) => {
            if let PropertyKey::String(s) = &pk {
                if s == "length" {
                    return Ok(true);
                }
                if let Ok(i) = s.parse::<usize>() {
                    return Ok(i < items.len());
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

pub(crate) fn delete_property_internal(
    target: &mut Value,
    key: &Value,
) -> Result<bool, String> {
    let pk = property_key_from_value(key)?;
    match target {
        Value::Object(map) => crate::runtime::stdlib::descriptor::delete_own_property_key(map, &pk),
        _ => Ok(false),
    }
}

pub(crate) fn own_keys_internal(target: &Value) -> Result<Vec<Value>, String> {
    match target {
        Value::Object(map) => {
            let mut keys: Vec<Value> = own_property_keys(map)
                .into_iter()
                .map(Value::String)
                .collect();
            keys.extend(get_own_property_symbols(map));
            Ok(keys)
        }
        Value::Array(items) => Ok((0..items.len())
            .map(|i| Value::String(i.to_string()))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn get_own_property_descriptor_internal(
    target: &Value,
    key: &Value,
) -> Result<Value, String> {
    let pk = property_key_from_value(key)?;
    match target {
        Value::Object(map) => Ok(get_own_property_descriptor_key(map, &pk)),
        _ => Ok(Value::Undefined),
    }
}

pub(crate) fn define_property_internal(
    target: &mut Value,
    key: &Value,
    desc: &Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let pk = property_key_from_value(key)?;
    let Value::Object(map) = target else {
        return Ok(false);
    };
    let desc = if crate::runtime::stdlib::descriptor::is_descriptor_object(desc) {
        parse_descriptor_input(desc)?
    } else {
        crate::runtime::stdlib::descriptor::default_descriptor_for_value(desc.clone())
    };
    match define_property_key(map, pk, desc, receiver, env) {
        Ok(()) => Ok(true),
        Err(e) if e.contains("Cannot") => Ok(false),
        Err(e) => Err(e),
    }
}

pub(crate) fn get_parent_of_internal(target: &Value) -> Result<Value, String> {
    match target {
        Value::Object(map) => Ok(get_object_parent(map)),
        _ => Ok(Value::Null),
    }
}

pub(crate) fn set_parent_of_internal(
    target: &mut Value,
    parent: &Value,
) -> Result<bool, String> {
    if !matches!(parent, Value::Object(_) | Value::Null) {
        return Ok(false);
    }
    let Value::Object(mut map) = target.clone() else {
        return Ok(false);
    };
    let oid = object_oid(&mut map);
    if !matches!(parent, Value::Null) && would_parent_cycle(oid, parent) {
        return Ok(false);
    }
    if matches!(parent, Value::Null) {
        map.remove(crate::runtime::stdlib::object::OBJECT_PARENT_KEY);
        map.remove("__kab_proto");
    } else {
        map.remove("__kab_proto");
        map.insert(
            crate::runtime::stdlib::object::OBJECT_PARENT_KEY.into(),
            parent.clone(),
        );
    }
    *target = Value::Object(map);
    Ok(true)
}

pub(crate) fn is_extensible_internal(target: &Value) -> bool {
    match target {
        Value::Object(map) => is_extensible(map),
        _ => false,
    }
}

pub(crate) fn prevent_extensions_internal(target: &mut Value) -> Result<bool, String> {
    let Value::Object(mut map) = target.clone() else {
        return Ok(false);
    };
    map.insert("__kab_prevent_extensions".into(), Value::Bool(true));
    *target = Value::Object(map);
    Ok(true)
}

pub(crate) fn apply_internal(
    target: &Value,
    this_arg: Value,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    if !is_callable_value(target) {
        return Err("Reflect.apply target must be callable".into());
    }
    let args_with_this: Vec<Value> = std::iter::once(this_arg).chain(args.clone()).collect();
    call_value(target.clone(), args, &[], &[], &[], &[], env)
        .or_else(|_| call_value(target.clone(), args_with_this, &[], &[], &[], &[], env))
}

pub(crate) fn construct_internal(
    target: &Value,
    args: Vec<Value>,
    _new_target: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if !is_callable_value(target) {
        return Err("Reflect.construct target must be callable".into());
    }
    call_value(target.clone(), args, &[], &[], &[], &[], env)
}

/// Public entry: `Reflect.get` / property reads through proxy traps.
pub fn reflect_get(
    target: &Value,
    key: &Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_get(target, key, receiver, env)
    } else {
        get_internal(target, key, receiver, env)
    }
}

/// Public entry: `Reflect.set` / property writes through proxy traps.
pub fn reflect_set(
    target: &mut Value,
    key: &Value,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_set(target, key, value, receiver, env)
    } else {
        set_internal(target, key, value, receiver, env)
    }
}

pub fn reflect_has(target: &Value, key: &Value, env: &mut Environment) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_has(target, key, env)
    } else {
        has_internal(target, key, env)
    }
}

pub fn reflect_delete_property(
    target: &mut Value,
    key: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_delete_property(target, key, env)
    } else {
        delete_property_internal(target, key)
    }
}

pub fn reflect_own_keys(target: &Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_own_keys(target, env)
    } else {
        own_keys_internal(target)
    }
}

pub fn reflect_get_own_property_descriptor(
    target: &Value,
    key: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_get_own_property_descriptor(target, key, env)
    } else {
        get_own_property_descriptor_internal(target, key)
    }
}

pub fn reflect_define_property(
    target: &mut Value,
    key: &Value,
    desc: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_define_property(target, key, desc, env)
    } else {
        let receiver = target.clone();
        define_property_internal(target, key, desc, &receiver, env)
    }
}

pub fn reflect_get_parent_of(
    target: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_get_parent_of(target, env)
    } else {
        get_parent_of_internal(target)
    }
}

pub fn reflect_set_parent_of(
    target: &mut Value,
    parent: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_set_parent_of(target, parent, env)
    } else {
        set_parent_of_internal(target, parent)
    }
}

pub fn reflect_is_extensible(target: &Value, env: &mut Environment) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_is_extensible(target, env)
    } else {
        Ok(is_extensible_internal(target))
    }
}

pub fn reflect_prevent_extensions(
    target: &mut Value,
    env: &mut Environment,
) -> Result<bool, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_prevent_extensions(target, env)
    } else {
        prevent_extensions_internal(target)
    }
}

pub fn reflect_apply(
    target: &Value,
    this_arg: Value,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_apply(target, this_arg, args, env)
    } else {
        apply_internal(target, this_arg, args, env)
    }
}

pub fn reflect_construct(
    target: &Value,
    args: Vec<Value>,
    new_target: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(target) {
        crate::runtime::stdlib::proxy::trap_construct(target, args, new_target, env)
    } else {
        construct_internal(target, args, new_target, env)
    }
}

fn reflect_get_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.get(target, key, receiver?)")?;
    let key = args.get(1).ok_or("Reflect.get(target, key, receiver?)")?;
    let receiver = args.get(2).unwrap_or(target);
    reflect_get(target, key, receiver, env)
}

fn reflect_set_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.set(target, key, value, receiver?)")?;
    let key = args.get(1).ok_or("Reflect.set(target, key, value, receiver?)")?;
    let value = args
        .get(2)
        .cloned()
        .ok_or("Reflect.set(target, key, value, receiver?)")?;
    let receiver = args.get(3).unwrap_or(target);
    let mut target = target.clone();
    let ok = reflect_set(&mut target, key, value, receiver, env)?;
    Ok(Value::Bool(ok))
}

fn reflect_has_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.has(target, key)")?;
    let key = args.get(1).ok_or("Reflect.has(target, key)")?;
    Ok(Value::Bool(reflect_has(target, key, env)?))
}

fn reflect_delete_property_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.deleteProperty(target, key)")?;
    let key = args.get(1).ok_or("Reflect.deleteProperty(target, key)")?;
    let mut target = target.clone();
    Ok(Value::Bool(reflect_delete_property(&mut target, key, env)?))
}

fn reflect_own_keys_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.ownKeys(target)")?;
    Ok(Value::Array(reflect_own_keys(target, env)?))
}

fn reflect_get_own_property_descriptor_native(
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    let target = args
        .first()
        .ok_or("Reflect.getOwnPropertyDescriptor(target, key)")?;
    let key = args
        .get(1)
        .ok_or("Reflect.getOwnPropertyDescriptor(target, key)")?;
    reflect_get_own_property_descriptor(target, key, env)
}

fn reflect_define_property_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.defineProperty(target, key, desc)")?;
    let key = args.get(1).ok_or("Reflect.defineProperty(target, key, desc)")?;
    let desc = args.get(2).ok_or("Reflect.defineProperty(target, key, desc)")?;
    let mut target = target.clone();
    Ok(Value::Bool(reflect_define_property(
        &mut target, key, desc, env,
    )?))
}

fn reflect_get_parent_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.getParent(target)")?;
    reflect_get_parent_of(target, env)
}

fn reflect_set_parent_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.setParent(target, parent)")?;
    let parent = args.get(1).ok_or("Reflect.setParent(target, parent)")?;
    let mut target = target.clone();
    Ok(Value::Bool(reflect_set_parent_of(&mut target, parent, env)?))
}

fn reflect_is_extensible_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.isExtensible(target)")?;
    Ok(Value::Bool(reflect_is_extensible(target, env)?))
}

fn reflect_prevent_extensions_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.preventExtensions(target)")?;
    let mut target = target.clone();
    Ok(Value::Bool(reflect_prevent_extensions(&mut target, env)?))
}

fn reflect_apply_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.apply(target, thisArg, args)")?;
    let this_arg = args
        .get(1)
        .cloned()
        .unwrap_or(Value::Undefined);
    let arg_list = match args.get(2) {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Undefined) | None => Vec::new(),
        _ => return Err("Reflect.apply third argument must be array".into()),
    };
    reflect_apply(target, this_arg, arg_list, env)
}

fn reflect_construct_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Reflect.construct(target, args, newTarget?)")?;
    let arg_list = match args.get(1) {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Undefined) | None => Vec::new(),
        _ => return Err("Reflect.construct second argument must be array".into()),
    };
    let new_target = args.get(2).unwrap_or(target);
    reflect_construct(target, arg_list, new_target, env)
}

fn insert_native(
    map: &mut HashMap<String, Value>,
    name: &str,
    func: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    map.insert(name.into(), Value::NativeFunction(func));
}

fn reflect_is_proxy_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Reflect.isProxy(v)")?;
    Ok(Value::Bool(crate::runtime::stdlib::proxy::is_proxy(v)))
}

fn build_reflect_namespace() -> Value {
    let mut m = HashMap::new();
    insert_native(&mut m, "apply", reflect_apply_native);
    insert_native(&mut m, "construct", reflect_construct_native);
    insert_native(&mut m, "isProxy", reflect_is_proxy_native);
    insert_native(&mut m, "defineProperty", reflect_define_property_native);
    insert_native(&mut m, "deleteProperty", reflect_delete_property_native);
    insert_native(&mut m, "get", reflect_get_native);
    insert_native(&mut m, "getOwnPropertyDescriptor", reflect_get_own_property_descriptor_native);
    insert_native(&mut m, "getParent", reflect_get_parent_native);
    insert_native(&mut m, "has", reflect_has_native);
    insert_native(&mut m, "isExtensible", reflect_is_extensible_native);
    insert_native(&mut m, "ownKeys", reflect_own_keys_native);
    insert_native(&mut m, "preventExtensions", reflect_prevent_extensions_native);
    insert_native(&mut m, "set", reflect_set_native);
    insert_native(&mut m, "setParent", reflect_set_parent_native);
    Value::Object(m)
}

pub fn register_reflect(env: &mut Environment) {
    env.set("Reflect".to_string(), build_reflect_namespace());
}
