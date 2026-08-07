//! ECMAScript `Proxy` — intercept object operations via handler traps.

use crate::runtime::stdlib::descriptor::is_callable_value;
use crate::bytecode::call_value;
use crate::runtime::stdlib::reflect::{
    apply_internal, construct_internal, define_property_internal, delete_property_internal,
    get_internal, get_own_property_descriptor_internal, get_parent_of_internal,
    has_internal, is_extensible_internal, own_keys_internal, prevent_extensions_internal,
    set_internal, set_parent_of_internal,
};
use crate::value::{Environment, Value};
use std::collections::HashMap;

pub const PROXY_MARKER: &str = "__kab_proxy";
pub const PROXY_TARGET: &str = "__kab_proxy_target";
pub const PROXY_HANDLER: &str = "__kab_proxy_handler";

pub fn is_proxy(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get(PROXY_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

pub fn is_proxy_ctor_object(v: &Value) -> bool {
    match v {
        Value::Object(m) => matches!(m.get("__kab_proxy_ctor"), Some(Value::Bool(true))),
        _ => false,
    }
}

fn proxy_parts(v: &Value) -> Result<(Value, Value), String> {
    let Value::Object(m) = v else {
        return Err("expected proxy".into());
    };
    let target = m
        .get(PROXY_TARGET)
        .cloned()
        .ok_or("invalid proxy: missing target")?;
    let handler = m
        .get(PROXY_HANDLER)
        .cloned()
        .ok_or("invalid proxy: missing handler")?;
    Ok((target, handler))
}

fn handler_trap(handler: &Value, name: &str) -> Option<Value> {
    let Value::Object(m) = handler else {
        return None;
    };
    match m.get(name) {
        Some(trap) if is_callable_value(trap) => Some(trap.clone()),
        _ => None,
    }
}

fn call_trap(
    trap: Value,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    call_value(trap, args, &[], &[], &[], &[], env)
}

fn valid_proxy_target(v: &Value) -> bool {
    matches!(
        v, Value::Object(_)
            | Value::Array(_)
            | Value::Function { .. }
            | Value::BytecodeFn(_)
            | Value::NativeFunction(_)
            | Value::BoundNative(_, _)
            | Value::BoundMethod(_, _)
    )
}

pub fn create_proxy(target: Value, handler: Value) -> Result<Value, String> {
    if !valid_proxy_target(&target) {
        return Err("Proxy target must be an object or function".into());
    }
    if !matches!(handler, Value::Object(_)) {
        return Err("Proxy handler must be an object".into());
    }
    let mut m = HashMap::new();
    m.insert(PROXY_MARKER.into(), Value::Bool(true));
    m.insert(PROXY_TARGET.into(), target);
    m.insert(PROXY_HANDLER.into(), handler);
    Ok(Value::from_object(m))
}

pub fn trap_get(
    proxy: &Value,
    key: &Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "get") {
        return call_trap(
            trap,
            vec![target, key.clone(), receiver.clone()],
            env,
        );
    }
    get_internal(&target, key, receiver, env)
}

pub fn trap_set(
    proxy: &mut Value,
    key: &Value,
    value: Value,
    receiver: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "set") {
        let result = call_trap(
            trap,
            vec![target, key.clone(), value, receiver.clone()],
            env,
        )?;
        return Ok(result.is_truthy());
    }
    let Value::Object(ref mut map_rc) = proxy else {
        return Err("expected proxy".into());
    };
    let map = Value::object_make_mut(map_rc);
    let target = map
        .get_mut(PROXY_TARGET)
        .ok_or("invalid proxy: missing target")?;
    set_internal(target, key, value, receiver, env)
}

pub fn trap_has(proxy: &Value, key: &Value, env: &mut Environment) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "has") {
        let result = call_trap(trap, vec![target, key.clone()], env)?;
        return Ok(result.is_truthy());
    }
    has_internal(&target, key, env)
}

pub fn trap_delete_property(
    proxy: &mut Value,
    key: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "deleteProperty") {
        let result = call_trap(trap, vec![target, key.clone()], env)?;
        return Ok(result.is_truthy());
    }
    let Value::Object(ref mut map_rc) = proxy else {
        return Err("expected proxy".into());
    };
    let map = Value::object_make_mut(map_rc);
    let target = map
        .get_mut(PROXY_TARGET)
        .ok_or("invalid proxy: missing target")?;
    delete_property_internal(target, key)
}

pub fn trap_own_keys(proxy: &Value, env: &mut Environment) -> Result<Vec<Value>, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "ownKeys") {
        let result = call_trap(trap, vec![target], env)?;
        return match result {
            Value::Array(items) => Ok(items.as_ref().clone()),
            _ => Err("ownKeys trap must return an array".into()),
        };
    }
    own_keys_internal(&target)
}

pub fn trap_get_own_property_descriptor(
    proxy: &Value,
    key: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "getOwnPropertyDescriptor") {
        return call_trap(trap, vec![target, key.clone()], env);
    }
    get_own_property_descriptor_internal(&target, key)
}

pub fn trap_define_property(
    proxy: &mut Value,
    key: &Value,
    desc: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let receiver = proxy.clone();
    let handler = match proxy {
        Value::Object(m) => m.get(PROXY_HANDLER).cloned(),
        _ => None,
    };
    if let Some(handler) = handler {
        if let Some(trap) = handler_trap(&handler, "defineProperty") {
            let target = match proxy {
                Value::Object(m) => m.get(PROXY_TARGET).cloned(),
                _ => None,
            }
            .ok_or("invalid proxy: missing target")?;
            let result = call_trap(trap, vec![target, key.clone(), desc.clone()], env)?;
            return Ok(result.is_truthy());
        }
    }
    let Value::Object(ref mut map_rc) = proxy else {
        return Err("expected proxy".into());
    };
    let map = Value::object_make_mut(map_rc);
    let target = map
        .get_mut(PROXY_TARGET)
        .ok_or("invalid proxy: missing target")?;
    define_property_internal(target, key, desc, &receiver, env)
}

pub fn trap_get_parent_of(proxy: &Value, env: &mut Environment) -> Result<Value, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "getParent") {
        return call_trap(trap, vec![target], env);
    }
    get_parent_of_internal(&target)
}

pub fn trap_set_parent_of(
    proxy: &mut Value,
    parent: &Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "setParent") {
        let result = call_trap(trap, vec![target, parent.clone()], env)?;
        return Ok(result.is_truthy());
    }
    let Value::Object(ref mut map_rc) = proxy else {
        return Err("expected proxy".into());
    };
    let map = Value::object_make_mut(map_rc);
    let target = map
        .get_mut(PROXY_TARGET)
        .ok_or("invalid proxy: missing target")?;
    set_parent_of_internal(target, parent)
}

pub fn trap_is_extensible(proxy: &Value, env: &mut Environment) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "isExtensible") {
        let result = call_trap(trap, vec![target], env)?;
        return Ok(result.is_truthy());
    }
    Ok(is_extensible_internal(&target))
}

pub fn trap_prevent_extensions(
    proxy: &mut Value,
    env: &mut Environment,
) -> Result<bool, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "preventExtensions") {
        let result = call_trap(trap, vec![target], env)?;
        return Ok(result.is_truthy());
    }
    let Value::Object(ref mut map_rc) = proxy else {
        return Err("expected proxy".into());
    };
    let map = Value::object_make_mut(map_rc);
    let target = map
        .get_mut(PROXY_TARGET)
        .ok_or("invalid proxy: missing target")?;
    prevent_extensions_internal(target)
}

pub fn trap_apply(
    proxy: &Value,
    this_arg: Value,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "apply") {
        return call_trap(trap, vec![target, this_arg, Value::from_array(args)], env);
    }
    apply_internal(&target, this_arg, args, env)
}

pub fn trap_construct(
    proxy: &Value,
    args: Vec<Value>,
    new_target: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let (target, handler) = proxy_parts(proxy)?;
    if let Some(trap) = handler_trap(&handler, "construct") {
        return call_trap(
            trap,
            vec![target, Value::from_array(args), new_target.clone()],
            env,
        );
    }
    construct_internal(&target, args, new_target, env)
}

fn proxy_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let target = args.first().ok_or("Proxy(target, handler)")?;
    let handler = args.get(1).ok_or("Proxy(target, handler)")?;
    create_proxy(target.clone(), handler.clone())
}

pub fn try_proxy_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_proxy_ctor_object(callee) {
        Some(proxy_ctor_native(args, env))
    } else {
        None
    }
}

pub fn build_proxy_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_proxy_ctor".into(), Value::Bool(true));
    Value::from_object(m)
}

pub fn register_proxy(env: &mut Environment) {
    env.set("Proxy".to_string(), build_proxy_namespace());
    env.set(
        "is_proxy".to_string(),
        Value::NativeFunction(is_proxy_native),
    );
}

fn is_proxy_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_proxy(v)")?;
    Ok(Value::Bool(is_proxy(v)))
}

/// Resolve proxy to its target for iteration (`for...of` on proxy of array).
pub fn proxy_target_for_iteration(v: &Value) -> Option<Value> {
    if is_proxy(v) {
        proxy_parts(v).ok().map(|(t, _)| t)
    } else {
        None
    }
}
