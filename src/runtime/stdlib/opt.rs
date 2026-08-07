//! Optional chaining (`?.`) runtime helpers.

use crate::bytecode::call_value;
use crate::runtime::stdlib::abort::{abort_reason, is_aborted, signal_id};
use crate::runtime::stdlib::descriptor::get_own_property;
use crate::runtime::stdlib::object::get_object_parent;
use crate::value::{Environment, Value};

pub fn is_nullish(v: &Value) -> bool {
    v.is_null() || v.is_undefined()
}

fn object_wants_bound_native(map: &std::collections::HashMap<String, Value>) -> bool {
    matches!(map.get("__kab_ctx"), Some(Value::Bool(true)))
        || matches!(map.get("__kab_gl_ctx"), Some(Value::Bool(true)))
        || matches!(map.get("__kab_gl_tex"), Some(Value::Bool(true)))
        || matches!(map.get("__kab_host_canvas"), Some(Value::Bool(true)))
        || matches!(map.get("__kab_game_surface"), Some(Value::Bool(true)))
        || matches!(map.get("__kab_mo"), Some(Value::Bool(true)))
}

pub fn maybe_bind_native_method(receiver: &Value, member: Value) -> Value {
    let Value::Object(map) = receiver else {
        return member;
    };
    if !object_wants_bound_native(map) {
        return member;
    }
    if let Value::NativeFunction(f) = member {
        Value::BoundNative(Box::new(receiver.clone()), f)
    } else {
        member
    }
}

pub fn get_member_value(obj: &Value, field: &str, env: &mut Environment) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(obj) {
        return crate::runtime::stdlib::reflect::reflect_get(
            obj,
            &Value::String(field.to_string()),
            obj,
            env,
        );
    }
    if let Some(id) = signal_id(obj) {
        return Ok(match field {
            "aborted" => Value::Bool(is_aborted(id)),
            "reason" => abort_reason(id),
            _ => Value::Undefined,
        });
    }
    match obj {
        Value::ClassInstance(_) => crate::ops::read_member(obj, field, env), Value::Array(items) if field == "length" => Ok(Value::Number(items.len() as i64)),
        Value::String(s) if field == "length" => {
            let n = if s.is_ascii() {
                s.len()
            } else {
                s.chars().count()
            };
            Ok(Value::Number(n as i64))
        }
        Value::Object(map) => {
            if crate::runtime::stdlib::map::is_map_value(obj) {
                if let Some(native) = crate::runtime::stdlib::map::map_instance_method(field) {
                    return Ok(Value::BoundNative(Box::new(obj.clone()), native));
                }
            }
            if crate::runtime::stdlib::async_iterator::needs_async_instance_methods(obj)
                && crate::runtime::stdlib::async_iterator::is_async_instance_method(field)
            {
                let mut updated = obj.clone();
                crate::runtime::stdlib::async_iterator::ensure_async_iterator_instance_methods(
                    &mut updated,
                );
                crate::runtime::stdlib::iterator::writeback_iterator_by_oid(&updated, env);
                return get_member_value(&updated, field, env);
            }
            if crate::runtime::stdlib::iterator::needs_sync_iterator_instance_methods(obj)
                && crate::runtime::stdlib::iterator::is_sync_instance_method(field)
            {
                let mut updated = obj.clone();
                crate::runtime::stdlib::iterator::ensure_sync_iterator_instance_methods(&mut updated);
                crate::runtime::stdlib::iterator::writeback_iterator_by_oid(&updated, env);
                return get_member_value(&updated, field, env);
            }
            if let Some(v) =
                crate::runtime::browser_platform::canvas_props::try_read_property(map, field)
            {
                return Ok(v);
            }
            if !field.starts_with("__kab_") {
                if let Some(v) = get_own_property(map, field, obj, env)? {
                    return Ok(maybe_bind_native_method(obj, v));
                }
            }
            let parent = get_object_parent(map);
            if !is_nullish(&parent) {
                return get_member_value(&parent, field, env);
            }
            Ok(Value::Undefined)
        }
        _ => Err("Member access requires object, array, string, or class instance".into()),
    }
}

fn opt_member_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let base = args.first().ok_or("opt_member(base, field)")?;
    let field = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("opt_member(base, field) expects string field".into()),
    };
    if is_nullish(base) {
        return Ok(Value::Undefined);
    }
    get_member_value(base, field, env)
}

fn opt_index_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let base = args.first().ok_or("opt_index(base, index)")?;
    let idx = args.get(1).ok_or("opt_index(base, index)")?;
    if is_nullish(base) {
        return Ok(Value::Undefined);
    }
    crate::ops::read_index(base, idx, env)
}

fn opt_call_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let base = args.first().ok_or("opt_call(base, ...args)")?.clone();
    if is_nullish(&base) {
        return Ok(Value::Undefined);
    }
    let call_args: Vec<Value> = args.iter().skip(1).cloned().collect();
    call_value(base, call_args, &[], &[], &[], &[], env)
}

pub fn register_opt(env: &mut Environment) {
    env.set(
        "__opt_member".to_string(),
        Value::NativeFunction(opt_member_native),
    );
    env.set(
        "__opt_index".to_string(),
        Value::NativeFunction(opt_index_native),
    );
    env.set(
        "__opt_call".to_string(),
        Value::NativeFunction(opt_call_native),
    );
}
