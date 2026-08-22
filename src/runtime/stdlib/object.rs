//! Object helpers — JS `Object.assign`, `in`, `delete` parity.

use crate::ast::{CallArg, Expr};
use crate::runtime::stdlib::descriptor::{
    default_descriptor_for_value, define_property_key, enumerable_own_keys,
    enumerable_own_symbol_ids, get_own_property_descriptor_key, get_own_property_symbols,
    get_own_property_key, has_own_property_key, is_descriptor_object,
    own_property_keys, parse_descriptor_input, property_key_from_value, set_own_property_key,
    PropertyKey,
};
use crate::value::{BytecodeFunction, Environment, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::rc::Rc;

static NEXT_OID: AtomicU64 = AtomicU64::new(1);

pub fn meta_truthy(map: &HashMap<String, Value>, key: &str) -> bool {
    map.get(key).is_some_and(|v| v.is_truthy())
}

pub fn object_oid(map: &mut HashMap<String, Value>) -> u64 {
    if let Some(Value::Number(n)) = map.get("__kab_oid") {
        return *n as u64;
    }
    let id = NEXT_OID.fetch_add(1, Ordering::Relaxed);
    map.insert("__kab_oid".into(), Value::Number(id as i64));
    id
}

pub fn object_oid_of(map: &HashMap<String, Value>) -> Option<u64> {
    match map.get("__kab_oid") {
        Some(Value::Number(n)) => Some(*n as u64),
        _ => None,
    }
}

/// True when an object has no user-visible fields (only internal `__kab_*` metadata).
pub fn object_is_pattern_empty(map: &HashMap<String, Value>) -> bool {
    map.keys().all(|k| k.starts_with("__kab_"))
}

/// When `obj.method` is read, bind `this` for plain-object methods.
pub fn bind_object_method(receiver: Value, method: Value) -> Value {
    match &method {
        Value::BoundNative(_, f) => Value::BoundNative(Box::new(receiver), *f),
        Value::BytecodeFn(_) => method,
        Value::Function { .. } => {
            let mut binding = HashMap::new();
            binding.insert("__kab_obj_method".into(), Value::Bool(true));
            binding.insert("__kab_method_recv".into(), receiver);
            binding.insert("__kab_method_fn".into(), method);
            Value::BoundNative(Box::new(Value::from_object(binding)), object_method_native)
        }
        _ => method,
    }
}

pub fn writeback_generator_by_oid(updated: &Value, env: &mut Environment) {
    let Value::Object(updated_map) = updated else {
        return;
    };
    let Some(oid) = object_oid_of(updated_map) else {
        return;
    };
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(live_map) = &live else {
            continue;
        };
        if object_oid_of(live_map) == Some(oid) {
            let mut merged = live.clone();
            if let Value::Object(dst) = &mut merged {
                for (k, v) in updated_map.iter() {
                    if k.starts_with("__kab_gen_") || k == "next" {
                        Rc::make_mut(dst).insert(k.clone(), v.clone());
                    }
                }
            }
            let _ = env.assign(&name, merged);
        }
    }
}

pub fn writeback_object_by_oid(updated: &Value, env: &mut Environment) {
    let Value::Object(updated_map) = updated else {
        return;
    };
    let Some(oid) = object_oid_of(updated_map) else {
        return;
    };
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(live_map) = &live else {
            continue;
        };
        if object_oid_of(live_map) == Some(oid) {
            let mut merged = live.clone();
            crate::runtime::closure_sync::merge_object_fields(updated, &mut merged);
            let _ = env.assign(&name, merged);
            break;
        }
    }
}

pub fn refresh_value_from_env_by_oid(v: &mut Value, env: &Environment) {
    let Value::Object(map) = v else {
        return;
    };
    let Some(oid) = object_oid_of(map) else {
        return;
    };
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(live_map) = &live else {
            continue;
        };
        if object_oid_of(live_map) == Some(oid) {
            *v = live.clone();
            return;
        }
    }
}

pub fn writeback_bytecode_fn_closure_on_receiver(receiver: &mut Value, func: &BytecodeFunction) {
    let Value::Object(ref mut map) = receiver else {
        return;
    };
    for v in Rc::make_mut(map).values_mut() {
        if let Value::BytecodeFn(stored) = v {
            if std::rc::Rc::ptr_eq(&stored.def, &func.def) {
                stored.closure = func.closure.clone();
            }
        }
    }
}

pub fn call_object_method(
    method: Value,
    args: Vec<Value>,
    mut receiver: Value,
    env: &mut Environment,
) -> Result<(Value, Value), String> {
    match method {
        Value::BytecodeFn(mut func) => {
            crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
            let mut call_env = Environment::child_from(&func.closure);
            call_env.set("this".into(), receiver.clone());
            let (result, _local_vals) = crate::bytecode::run_bytecode_fn_with_locals(
                func.def.as_ref(),
                args,
                &mut call_env,
            )?;
            if let Some(updated) = call_env.get("this") {
                crate::runtime::closure_sync::merge_object_fields(&updated, &mut receiver);
            }
            crate::runtime::closure_sync::sync_closure_writes(&func.closure, &call_env, env);
            crate::runtime::closure_sync::sync_bytecode_globals_to_root(&func, &call_env, env);
            writeback_bytecode_fn_closure_on_receiver(&mut receiver, &func);
            Ok((result, receiver))
        }
        Value::Function {
            params,
            defaults,
            rest,
            body,
            env: closure_env,
            async_fn,
            ..
        } => {
            if async_fn {
                return Err("async object methods not supported".into());
            }
            let closure = closure_env.clone();
            let mut call_env = Environment::child(closure_env);
            call_env.set("this".into(), receiver.clone());
            crate::evaluator::bind_call_params(&params, &defaults, &rest, &args, &mut call_env)?;
            let result = crate::evaluator::eval_expr(&body, &mut call_env)?;
            if let Some(updated) = call_env.get("this") {
                crate::runtime::closure_sync::merge_object_fields(&updated, &mut receiver);
            }
            crate::runtime::closure_sync::sync_closure_writes(&closure, &call_env, env);
            Ok((result, receiver))
        }
        other => {
            let result = crate::bytecode::call_value(other, args, &[], &[], &[], &[], env)?;
            Ok((result, receiver))
        }
    }
}

fn object_method_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let wrapper = args.first().ok_or("object method call")?;
    let Value::Object(m) = wrapper else {
        return Err("internal object method binding".into());
    };
    let recv = m
        .get("__kab_method_recv")
        .cloned()
        .ok_or("internal object method receiver")?;
    let method = m
        .get("__kab_method_fn")
        .cloned()
        .ok_or("internal object method fn")?;
    let user_args: Vec<Value> = args.get(1..).unwrap_or(&[]).to_vec();
    let (result, updated_recv) = call_object_method(method, user_args, recv, env)?;
    writeback_object_by_oid(&updated_recv, env);
    Ok(result)
}

pub fn is_extensible(map: &HashMap<String, Value>) -> bool {
    !meta_truthy(map, "__kab_prevent_extensions")
        && !meta_truthy(map, "__kab_sealed")
        && !meta_truthy(map, "__kab_frozen")
}

/// Internal object-parent link (Kabootar model — **not** JS prototype).
/// Public API: `Object.getParent` / `Object.setParent`, `Reflect.getParent` / `Reflect.setParent`.
pub const OBJECT_PARENT_KEY: &str = "__kab_parent";
/// Legacy key from older builds; still read, never written.
const OBJECT_PARENT_KEY_LEGACY: &str = "__kab_proto";

pub fn get_object_parent(map: &HashMap<String, Value>) -> Value {
    map.get(OBJECT_PARENT_KEY)
        .or_else(|| map.get(OBJECT_PARENT_KEY_LEGACY))
        .cloned()
        .unwrap_or(Value::Null)
}

fn set_object_parent_map(map: &mut HashMap<String, Value>, parent: Option<Value>) {
    map.remove(OBJECT_PARENT_KEY_LEGACY);
    match parent {
        Some(p) => {
            map.insert(OBJECT_PARENT_KEY.into(), p);
        }
        None => {
            map.remove(OBJECT_PARENT_KEY);
        }
    }
}

fn parent_is_object_or_null(v: &Value) -> bool {
    matches!(v, Value::Object(_) | Value::Null)
}

pub(crate) fn would_parent_cycle(obj_oid: u64, parent: &Value) -> bool {
    let mut current = parent.clone();
    for _ in 0..64 {
        let Value::Object(ref map) = current else {
            return false;
        };
        if object_oid_of(map) == Some(obj_oid) {
            return true;
        }
        current = get_object_parent(map);
        if matches!(current, Value::Null) {
            return false;
        }
    }
    true
}

pub fn own_property_names(map: &HashMap<String, Value>) -> Vec<String> {
    own_property_keys(map)
}

pub fn enumerable_property_names(map: &HashMap<String, Value>) -> Vec<String> {
    enumerable_own_keys(map)
}

fn is_object_mutator_method(method: &str) -> bool {
    matches!(
        method,
        "defineProperty"
            | "setParent"
            | "deleteProperty"
            | "freeze"
            | "seal"
            | "preventExtensions"
            | "assign"
    )
}

fn is_object_mutator_fn(name: &str) -> bool {
    matches!(
        name,
        "object_define_property"
            | "object_set_parent"
            | "object_delete_prop"
            | "object_delete"
            | "object_freeze"
            | "object_seal"
            | "object_prevent_extensions"
            | "object_assign"
            | "assign"
    )
}

/// When an object mutator returns a new object snapshot, write it back to the first-arg variable.
pub fn mutator_writeback_var(func: &Expr, args: &[CallArg]) -> Option<String> {
    let ok = match func {
        Expr::Member(obj, method, _) if is_object_mutator_method(method) => {
            matches!(obj.as_ref(), Expr::Variable(s) if s == "Object")
        }
        Expr::Variable(name) => is_object_mutator_fn(name),
        _ => false,
    };
    if !ok {
        return None;
    }
    match args.first()? {
        CallArg::Expr(Expr::Variable(name)) => Some(name.clone()),
        _ => None,
    }
}

pub fn try_mutator_writeback(
    func: &Expr,
    args: &[CallArg],
    result: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    if let Some(name) = mutator_writeback_var(func, args) {
        env.assign(&name, result.clone())?;
    }
    Ok(())
}

pub fn check_can_set(map: &HashMap<String, Value>, key: &str) -> Result<(), String> {
    if meta_truthy(map, "__kab_frozen") {
        return Err("Cannot modify frozen object".into());
    }
    let is_new = !map.contains_key(key);
    if is_new && (meta_truthy(map, "__kab_sealed") || meta_truthy(map, "__kab_prevent_extensions")) {
        return Err("Cannot add property to sealed/extensible-false object".into());
    }
    Ok(())
}

pub fn check_can_delete(map: &HashMap<String, Value>) -> Result<(), String> {
    if meta_truthy(map, "__kab_frozen") || meta_truthy(map, "__kab_sealed") {
        return Err("Cannot delete property".into());
    }
    Ok(())
}

fn mark_object(mut map: Rc<HashMap<String, Value>>, key: &str) -> Value {
    Rc::make_mut(&mut map).insert(key.to_string(), Value::Bool(true));
    Value::Object(map)
}

fn object_freeze_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let Value::Object(map) = args.first().ok_or("object_freeze(obj)")? else {
        return Err("object_freeze() expects object".into());
    };
    Ok(mark_object(map.clone(), "__kab_frozen"))
}

fn object_seal_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let Value::Object(map) = args.first().ok_or("object_seal(obj)")? else {
        return Err("object_seal() expects object".into());
    };
    Ok(mark_object(map.clone(), "__kab_sealed"))
}

fn object_prevent_extensions_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let Value::Object(map) = args.first().ok_or("object_prevent_extensions(obj)")? else {
        return Err("object_prevent_extensions() expects object".into());
    };
    Ok(mark_object(map.clone(), "__kab_prevent_extensions"))
}

fn object_is_frozen_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("object_is_frozen(obj)")?;
    Ok(Value::Bool(match v {
        Value::Object(map) => meta_truthy(map, "__kab_frozen"),
        _ => false,
    }))
}

fn object_is_sealed_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("object_is_sealed(obj)")?;
    Ok(Value::Bool(match v {
        Value::Object(map) => meta_truthy(map, "__kab_sealed"),
        _ => false,
    }))
}

fn object_define_property_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let obj = args.first().ok_or("Object.defineProperty(obj, key, desc)")?;
    let key_v = args.get(1).ok_or("Object.defineProperty(obj, key, desc)")?;
    let key = property_key_from_value(key_v)?;
    let third = args.get(2).ok_or("Object.defineProperty(obj, key, desc)")?;
    let Value::Object(ref mut map_rc) = obj.clone() else {
        return Err("Object.defineProperty() expects object".into());
    };
    let map = Value::object_make_mut(map_rc);
    let mut desc = if is_descriptor_object(third) {
        parse_descriptor_input(third)?
    } else {
        default_descriptor_for_value(third.clone())
    };
    if desc.get.is_none()
        && desc.set.is_none()
        && desc.value.is_none()
        && has_own_property_key(map, &key)
    {
        if let PropertyKey::String(s) = &key {
            desc.value = map.get(s).cloned();
        } else if let PropertyKey::Symbol(id) = &key {
            if let Ok(Some(v)) = crate::runtime::stdlib::descriptor::get_own_symbol(
                map,
                *id,
                obj,
                env,
            ) {
                desc.value = Some(v);
            }
        }
    }
    define_property_key(map, key, desc, obj, env)?;
    Ok(Value::Object(map_rc.clone()))
}

fn object_get_own_property_descriptor_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let obj = args.first().ok_or("Object.getOwnPropertyDescriptor(obj, key)")?;
    let key = property_key_from_value(args.get(1).ok_or("Object.getOwnPropertyDescriptor(obj, key)")?)?;
    let Value::Object(map) = obj else {
        return Err("Object.getOwnPropertyDescriptor() expects object".into());
    };
    Ok(get_own_property_descriptor_key(map.as_ref(), &key))
}

fn object_get_own_property_symbols_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let obj = args.first().ok_or("Object.getOwnPropertySymbols(obj)")?;
    let Value::Object(map) = obj else {
        return Err("Object.getOwnPropertySymbols() expects object".into());
    };
    Ok(Value::from_array(get_own_property_symbols(map.as_ref())))
}

fn object_create_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = args.first().ok_or("Object.create(parent)")?;
    if !parent_is_object_or_null(parent) {
        return Err("Object.create() parent must be object or null".into());
    }
    let mut map = HashMap::new();
    object_oid(&mut map);
    if !matches!(parent, Value::Null) {
        set_object_parent_map(&mut map, Some(parent.clone()));
    }
    Ok(Value::from_object(map))
}

fn object_get_parent_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Object.getParent(obj)")?;
    let Value::Object(map) = v else {
        return Err("Object.getParent() expects object".into());
    };
    Ok(get_object_parent(map.as_ref()))
}

fn object_set_parent_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let obj = args.first().ok_or("Object.setParent(obj, parent)")?;
    let parent = args.get(1).ok_or("Object.setParent(obj, parent)")?;
    if !parent_is_object_or_null(parent) {
        return Err("Object.setParent() parent must be object or null".into());
    }
    let Value::Object(ref mut map_rc) = obj.clone() else {
        return Err("Object.setParent() expects object".into());
    };
    let map = Value::object_make_mut(map_rc);
    let oid = object_oid(map);
    if !matches!(parent, Value::Null) && would_parent_cycle(oid, parent) {
        return Err("Object.setParent() would introduce parent cycle".into());
    }
    if matches!(parent, Value::Null) {
        set_object_parent_map(map, None);
    } else {
        set_object_parent_map(map, Some(parent.clone()));
    }
    Ok(Value::Object(map_rc.clone()))
}

fn object_define_properties_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let obj = args
        .first()
        .ok_or("Object.defineProperties(obj, props)")?
        .clone();
    let props = args.get(1).ok_or("Object.defineProperties(obj, props)")?;
    let Value::Object(prop_map) = props else {
        return Err("Object.defineProperties() props must be object".into());
    };
    let mut current = obj;
    for (key, desc) in prop_map.iter() {
        if key.starts_with("__kab_") {
            continue;
        }
        current = object_define_property_native(
            &[current, Value::String(key.clone()), desc.clone()],
            env,
        )?;
    }
    Ok(current)
}

fn object_get_own_property_descriptors_native(
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    let obj = args
        .first()
        .ok_or("Object.getOwnPropertyDescriptors(obj)")?;
    let names = object_get_own_property_names_native(&[obj.clone()], env)?;
    let Value::Array(name_vals) = names else {
        return Ok(Value::from_object(HashMap::new()));
    };
    let mut out = HashMap::new();
    for nv in name_vals.iter() {
        let desc = object_get_own_property_descriptor_native(&[obj.clone(), nv.clone()], env)?;
        if let Value::String(k) = nv {
            if !matches!(desc, Value::Undefined) {
                out.insert(k.clone(), desc);
            }
        }
    }
    Ok(Value::from_object(out))
}

fn object_is_extensible_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Object.isExtensible(obj)")?;
    Ok(Value::Bool(match v {
        Value::Object(map) => is_extensible(map.as_ref()),
        _ => false,
    }))
}

fn object_get_own_property_names_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let v = args.first().ok_or("Object.getOwnPropertyNames(obj)")?;
    match v {
        Value::Object(map) => Ok(Value::from_array(
            own_property_names(map)
                .into_iter()
                .map(Value::String)
                .collect(),
        )),
        Value::Array(items) => Ok(Value::from_array(
            (0..items.len())
                .map(|i| Value::String(i.to_string()))
                .collect(),
        )),
        _ => Err("Object.getOwnPropertyNames() expects object or array".into()),
    }
}

fn array_arg(v: &Value) -> Result<&[Value], String> {
    match v {
        Value::Array(items) => Ok(items.as_ref()),
        _ => Err("expected array".into()),
    }
}

fn assign_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("assign(target, ...sources)")?;
    let mut out = match target {
        Value::Object(map) => map.as_ref().clone(),
        _ => return Err("assign() target must be an object".into()),
    };
    for src in args.iter().skip(1) {
        let Value::Object(map) = src else {
            continue;
        };
        for key in enumerable_own_keys(map.as_ref()) {
            if let Some(val) =
                crate::runtime::stdlib::descriptor::get_own_property(map.as_ref(), &key, src, env)?
            {
                crate::runtime::stdlib::descriptor::set_own_property(
                    &mut out,
                    &key,
                    val,
                    target,
                    env,
                )?;
            }
        }
        for sym_id in enumerable_own_symbol_ids(map.as_ref()) {
            let key = PropertyKey::Symbol(sym_id);
            if let Some(val) = get_own_property_key(map.as_ref(), &key, src, env)? {
                set_own_property_key(&mut out, &key, val, target, env)?;
            }
        }
    }
    Ok(Value::from_object(out))
}

fn has_key_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let obj = args.first().ok_or("has_key(obj, key)")?;
    let key_v = args.get(1).ok_or("has_key(obj, key)")?;
    let found = match obj {
        Value::Object(map) => match property_key_from_value(key_v) {
            Ok(k) => has_own_property_key(map, &k),
            Err(_) => false,
        }, Value::Array(items) => match key_v {
            Value::String(k) => k.parse::<usize>().is_ok_and(|i| i < items.len()),
            Value::Number(n) => (*n as usize) < items.len(),
            _ => false,
        },
        Value::String(s) => matches!(key_v, Value::String(k) if s.contains(k)),
        Value::ClassInstance(inst) => match key_v {
            Value::String(k) => inst
                .try_borrow()
                .map(|i| i.fields.contains_key(k))
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    };
    Ok(Value::Bool(found))
}

fn delete_prop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let obj = args.first().ok_or("delete_prop(obj, key)")?;
    let key = property_key_from_value(args.get(1).ok_or("delete_prop(obj, key)")?)?;
    let Value::Object(map) = obj else {
        return Err("delete_prop() expects object".into());
    };
    let mut out = map.as_ref().clone();
    let _ = crate::runtime::stdlib::descriptor::delete_own_property_key(&mut out, &key)?;
    Ok(Value::from_object(out))
}

fn clone_shallow_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("clone_shallow(v)")?;
    Ok(match v {
        Value::Array(items) => Value::Array(items.clone()), Value::Object(map) => Value::Object(map.clone()),
        other => other.clone(),
    })
}

fn object_values_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    crate::runtime::stdlib::values_native(args, env)
}

fn object_entries_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    crate::runtime::stdlib::entries_native(args, env)
}

fn object_from_entries_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pairs = array_arg(args.first().ok_or("object_from_entries(pairs)")?)?;
    let mut map = std::collections::HashMap::new();
    for pair in pairs {
        let Value::Array(entry) = pair else {
            return Err("object_from_entries() expects [[key, value], ...]".into());
        };
        if entry.len() < 2 {
            return Err("object_from_entries() entry needs key and value".into());
        }
        let key = match &entry[0] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => return Err(format!("object_from_entries() key must be string, got {:?}", other)),
        };
        map.insert(key, entry[1].clone());
    }
    Ok(Value::from_object(map))
}

fn structured_clone_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("structured_clone(v)")?;
    Ok(clone_structured(v))
}

fn clone_structured(v: &Value) -> Value {
    match v {
        Value::Array(items) => Value::from_array(items.iter().map(clone_structured).collect()),
        Value::Object(map) => Value::from_object(
            map.iter()
                .filter(|(k, _)| !k.starts_with("__kab_"))
                .map(|(k, v)| (k.clone(), clone_structured(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => {
            if x.is_nan() && y.is_nan() {
                true
            } else {
                x.to_bits() == y.to_bits()
            }
        }
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Number(x), Value::Float(y)) | (Value::Float(y), Value::Number(x)) => {
            (*x as f64).to_bits() == y.to_bits()
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Undefined, Value::Undefined) => true,
        (Value::Array(ax), Value::Array(bx)) => {
            ax.len() == bx.len() && ax.iter().zip(bx.iter()).all(|(x, y)| same_value(x, y))
        }
        (Value::Object(ax), Value::Object(bx)) => {
            let a_keys: Vec<_> = ax
                .keys()
                .filter(|k| !k.starts_with("__kab_"))
                .collect();
            let b_keys: Vec<_> = bx
                .keys()
                .filter(|k| !k.starts_with("__kab_"))
                .collect();
            if a_keys.len() != b_keys.len() {
                return false;
            }
            a_keys.iter().all(|k| {
                bx.get(*k)
                    .zip(ax.get(*k))
                    .is_some_and(|(bv, av)| same_value(av, bv))
            })
        }
        _ => false,
    }
}

fn object_is_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("object_is(a, b)")?;
    let b = args.get(1).ok_or("object_is(a, b)")?;
    Ok(Value::Bool(same_value(a, b)))
}

fn object_group_by_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("object_group_by(items, fn)")?)?;
    let func = args.get(1).ok_or("object_group_by(items, fn)")?;
    let mut out: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    for (i, item) in items.iter().enumerate() {
        let key_v = crate::bytecode::call_value(
            func.clone(),
            vec![item.clone(), Value::Number(i as i64)],
            &[],
            &[],
            &[],
            &[],
            env,
        )?;
        let key = match &key_v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => crate::value::format_value(other),
        };
        match out.get_mut(&key) {
            Some(Value::Array(bucket)) => Rc::make_mut(bucket).push(item.clone()),
            _ => {
                out.insert(key, Value::from_array(vec![item.clone()]));
            }
        }
    }
    Ok(Value::from_object(out))
}

fn object_keys_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("Object.keys(obj)")?;
    match v {
        Value::Object(map) => Ok(Value::from_array(
            enumerable_own_keys(map)
                .into_iter()
                .map(Value::String)
                .collect(),
        )), Value::Array(items) => Ok(Value::from_array(
            (0..items.len())
                .map(|i| Value::String(i.to_string()))
                .collect(),
        )),
        _ => Err("Object.keys() expects an object or array".into()),
    }
}

fn insert_native(
    map: &mut HashMap<String, Value>,
    js_name: &str,
    func: fn(&[Value], &mut Environment) -> Result<Value, String>,
) {
    map.insert(js_name.into(), Value::NativeFunction(func));
}

fn object_array_push_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut obj = args
        .first()
        .cloned()
        .ok_or("object_array_push(obj, key, item)")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("object_array_push key must be a string".into()),
    };
    let item = args
        .get(2)
        .cloned()
        .ok_or("object_array_push(obj, key, item)")?;
    let Value::Object(ref mut map) = obj else {
        return Err("object_array_push requires an object".into());
    };
    let map = Value::object_make_mut(map);
    match map.get_mut(&key) {
        Some(Value::Array(items)) => {
            Value::reject_direct_container_cycle(&Value::Array(Rc::clone(items)), &item)?;
            Value::array_make_mut(items).push(item);
            Ok(Value::Number(items.len() as i64))
        }
        Some(_) => Err("object_array_push field is not an array".into()),
        None => Err(format!("object_array_push missing field `{key}`")),
    }
}

fn object_array_pop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut obj = args
        .first()
        .cloned()
        .ok_or("object_array_pop(obj, key)")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("object_array_pop key must be a string".into()),
    };
    let Value::Object(ref mut map) = obj else {
        return Err("object_array_pop requires an object".into());
    };
    let map = Value::object_make_mut(map);
    match map.get_mut(&key) {
        Some(Value::Array(items)) => {
            let vec = Value::array_make_mut(items);
            Ok(vec.pop().unwrap_or(Value::Null))
        }
        Some(_) => Err("object_array_pop field is not an array".into()),
        None => Err(format!("object_array_pop missing field `{key}`")),
    }
}

fn object_array_clear_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut obj = args
        .first()
        .cloned()
        .ok_or("object_array_clear(obj, key)")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("object_array_clear key must be a string".into()),
    };
    let Value::Object(ref mut map) = obj else {
        return Err("object_array_clear requires an object".into());
    };
    let map = Value::object_make_mut(map);
    match map.get_mut(&key) {
        Some(Value::Array(items)) => {
            Value::array_make_mut(items).clear();
            Ok(Value::Number(0))
        }
        Some(_) => Err("object_array_clear field is not an array".into()),
        None => Err(format!("object_array_clear missing field `{key}`")),
    }
}

fn object_array_truncate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut obj = args
        .first()
        .cloned()
        .ok_or("object_array_truncate(obj, key, len)")?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("object_array_truncate key must be a string".into()),
    };
    let new_len = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("object_array_truncate len must be a non-negative number".into()),
    };
    let Value::Object(ref mut map) = obj else {
        return Err("object_array_truncate requires an object".into());
    };
    let map = Value::object_make_mut(map);
    match map.get_mut(&key) {
        Some(Value::Array(items)) => {
            let vec = Value::array_make_mut(items);
            if new_len > vec.len() {
                return Err("object_array_truncate beyond length".into());
            }
            vec.truncate(new_len);
            Ok(Value::Number(vec.len() as i64))
        }
        Some(_) => Err("object_array_truncate field is not an array".into()),
        None => Err(format!("object_array_truncate missing field `{key}`")),
    }
}

fn object_get_prototype_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    Err(
        "Object.getPrototypeOf is not supported; use Object.getParent (Kabootar Parent model)"
            .into(),
    )
}

fn object_set_prototype_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    Err(
        "Object.setPrototypeOf is not supported; use Object.setParent (Kabootar Parent model)"
            .into(),
    )
}

fn build_object_namespace() -> Value {
    let mut m = HashMap::new();
    insert_native(&mut m, "assign", assign_native);
    insert_native(&mut m, "create", object_create_native);
    insert_native(&mut m, "defineProperty", object_define_property_native);
    insert_native(&mut m, "deleteProperty", delete_prop_native);
    insert_native(&mut m, "entries", object_entries_native);
    insert_native(&mut m, "freeze", object_freeze_native);
    insert_native(&mut m, "fromEntries", object_from_entries_native);
    insert_native(&mut m, "getOwnPropertyDescriptor", object_get_own_property_descriptor_native);
    insert_native(&mut m, "getOwnPropertySymbols", object_get_own_property_symbols_native);
    insert_native(&mut m, "getOwnPropertyNames", object_get_own_property_names_native);
    insert_native(&mut m, "getParent", object_get_parent_native);
    insert_native(&mut m, "setParent", object_set_parent_native);
    insert_native(&mut m, "getPrototypeOf", object_get_prototype_of_native);
    insert_native(&mut m, "setPrototypeOf", object_set_prototype_of_native);
    insert_native(&mut m, "defineProperties", object_define_properties_native);
    insert_native(
        &mut m,
        "getOwnPropertyDescriptors",
        object_get_own_property_descriptors_native,
    );
    insert_native(&mut m, "groupBy", object_group_by_native);
    insert_native(&mut m, "hasOwn", has_key_native);
    insert_native(&mut m, "is", object_is_native);
    insert_native(&mut m, "isExtensible", object_is_extensible_native);
    insert_native(&mut m, "isFrozen", object_is_frozen_native);
    insert_native(&mut m, "isSealed", object_is_sealed_native);
    insert_native(&mut m, "keys", object_keys_native);
    insert_native(&mut m, "preventExtensions", object_prevent_extensions_native);
    insert_native(&mut m, "seal", object_seal_native);
    insert_native(&mut m, "values", object_values_native);
    Value::from_object(m)
}

pub fn register_object(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("assign", assign_native),
        ("has_key", has_key_native),
        ("delete_prop", delete_prop_native),
        ("clone_shallow", clone_shallow_native),
        ("object_values", object_values_native),
        ("object_entries", object_entries_native),
        ("object_assign", assign_native),
        ("object_has_own", has_key_native),
        ("object_has", has_key_native),
        ("object_has_key", has_key_native),
        ("object_delete", delete_prop_native),
        ("object_delete_prop", delete_prop_native),
        ("object_from_entries", object_from_entries_native),
        ("object_is", object_is_native),
        ("object_keys", object_keys_native),
        ("object_clone_shallow", clone_shallow_native),
        ("object_freeze", object_freeze_native),
        ("object_seal", object_seal_native),
        ("object_prevent_extensions", object_prevent_extensions_native),
        ("object_is_frozen", object_is_frozen_native),
        ("object_is_sealed", object_is_sealed_native),
        ("object_is_extensible", object_is_extensible_native),
        ("object_define_property", object_define_property_native),
        ("object_get_own_property_descriptor", object_get_own_property_descriptor_native),
        ("object_get_own_property_symbols", object_get_own_property_symbols_native),
        ("object_create", object_create_native),
        ("object_get_parent", object_get_parent_native),
        ("object_set_parent", object_set_parent_native),
        ("object_get_own_property_names", object_get_own_property_names_native),
        ("object_group_by", object_group_by_native),
        ("group_by", object_group_by_native),
        ("structured_clone", structured_clone_native),
        ("object_array_push", object_array_push_native),
        ("object_array_pop", object_array_pop_native),
        ("object_array_clear", object_array_clear_native),
        ("object_array_truncate", object_array_truncate_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
    env.set("Object".to_string(), build_object_namespace());
}
