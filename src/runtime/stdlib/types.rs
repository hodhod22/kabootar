//! Runtime type checks — lightweight parity with TypeScript `typeof` guards.

use crate::runtime::stdlib::map::{is_map_value, is_set_value};
use crate::value::{Environment, Value};

fn is_array_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_array(v)")?;
    Ok(Value::Bool(matches!(v, Value::Array(_))))
}

fn is_object_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_object(v)")?;
    Ok(Value::Bool(matches!(v, Value::Object(_)) && !is_map_value(v) && !is_set_value(v)))
}

fn is_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_string(v)")?;
    Ok(Value::Bool(matches!(v, Value::String(_))))
}

fn is_number_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_number(v)")?;
    Ok(Value::Bool(matches!(v, Value::Number(_) | Value::Float(_))))
}

fn is_bool_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_bool(v)")?;
    Ok(Value::Bool(matches!(v, Value::Bool(_))))
}

fn is_function_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_function(v)")?;
    Ok(Value::Bool(matches!(
        v,
        Value::Function { .. } | Value::NativeFunction(_) | Value::BoundMethod(_, _)
            | Value::PromiseSettler { .. }
    )))
}

fn is_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_map(v)")?;
    Ok(Value::Bool(is_map_value(v)))
}

fn is_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_set(v)")?;
    Ok(Value::Bool(is_set_value(v)))
}

fn type_assert_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("type_assert(v, kind)")?;
    let kind = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("type_assert(v, kind) expects string kind".into()),
    };
    let ok = match kind {
        "array" => matches!(v, Value::Array(_)),
        "object" => matches!(v, Value::Object(_)) && !is_map_value(v) && !is_set_value(v),
        "string" => matches!(v, Value::String(_)),
        "number" => matches!(v, Value::Number(_) | Value::Float(_)),
        "boolean" | "bool" => matches!(v, Value::Bool(_)),
        "function" => matches!(
            v,
            Value::Function { .. } | Value::NativeFunction(_) | Value::BoundMethod(_, _)
            | Value::PromiseSettler { .. }
        ),
        "map" => is_map_value(v),
        "set" => is_set_value(v),
        "null" => matches!(v, Value::Null),
        "undefined" => matches!(v, Value::Undefined),
        other => return Err(format!("unknown type kind: {other}")),
    };
    if ok {
        Ok(v.clone())
    } else {
        Err(format!("type_assert failed: expected {kind}"))
    }
}

pub fn register_types(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("is_array", is_array_native),
        ("array_is_array", is_array_native),
        ("is_object", is_object_native),
        ("is_string", is_string_native),
        ("is_number", is_number_native),
        ("is_bool", is_bool_native),
        ("is_function", is_function_native),
        ("is_map", is_map_native),
        ("is_set", is_set_native),
        ("type_assert", type_assert_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

pub fn typeof_name(v: &Value) -> &'static str {
    if is_map_value(v) {
        return "map";
    }
    if is_set_value(v) {
        return "set";
    }
    match v {
        Value::Undefined => "undefined",
        Value::Null => "null",
        Value::Number(_) | Value::Float(_) => "number",
        Value::BigInt(_) => "bigint",
        Value::String(_) => "string",
        Value::Symbol(_) => "symbol",
        Value::Bool(_) => "boolean", Value::Array(_) => "array", Value::Object(_) => "object",
        Value::Function { .. } | Value::NativeFunction(_) | Value::Promise(_) => "function",
        Value::ClassInstance(_) | Value::BoundMethod(_, _) => "object",
        _ => "object",
    }
}
