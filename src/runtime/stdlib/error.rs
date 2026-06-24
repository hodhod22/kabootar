//! JS-style `Error` objects, `.cause`, `.stack`, and `throw` propagation.

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;

pub const THROW_MARKER: &str = "\x01kab_throw\x01";

thread_local! {
    static PENDING_THROW: RefCell<Option<Value>> = RefCell::new(None);
}

pub fn throw_value(v: Value) -> String {
    let enriched = enrich_error_value(v);
    PENDING_THROW.with(|slot| *slot.borrow_mut() = Some(enriched));
    THROW_MARKER.to_string()
}

pub fn take_throw_value(err: &str) -> Option<Value> {
    if err == THROW_MARKER {
        PENDING_THROW.with(|slot| slot.borrow_mut().take())
    } else {
        None
    }
}

pub fn capture_stack_trace(skip_frames: usize) -> String {
    let mut lines = Vec::new();
    let bt = std::backtrace::Backtrace::capture();
    let text = format!("{bt}");
    for line in text.lines().skip(skip_frames + 1).take(8) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("backtrace") {
                continue;
            }
            lines.push(format!("    at {trimmed}"));
        }
    if lines.is_empty() {
        lines.push("    at <anonymous>".into());
    }
    lines.join("\n")
}

pub fn enrich_error_value_for_catch(v: Value) -> Value {
    enrich_error_value(v)
}

fn enrich_error_value(v: Value) -> Value {
    let Value::Object(mut map) = v else {
        return v;
    };
    if !matches!(map.get("__kab_error"), Some(Value::Bool(true))) {
        return Value::Object(map);
    }
    if !map.contains_key("stack") {
        map.insert(
            "stack".into(),
            Value::String(format!(
                "{}: {}\n{}",
                map.get("name")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("Error"),
                map.get("message")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or(""),
                capture_stack_trace(3)
            )),
        );
    }
    Value::Object(map)
}

pub fn make_error(name: &str, message: &str) -> Value {
    make_error_with_cause(name, message, None)
}

pub fn make_error_with_cause(name: &str, message: &str, cause: Option<Value>) -> Value {
    let mut obj = HashMap::new();
    obj.insert("__kab_error".into(), Value::Bool(true));
    obj.insert("name".into(), Value::String(name.into()));
    obj.insert("message".into(), Value::String(message.into()));
    if let Some(c) = cause.filter(|v| !matches!(v, Value::Undefined | Value::Null)) {
        obj.insert("cause".into(), c);
    }
    obj.insert(
        "stack".into(),
        Value::String(format!("{name}: {message}\n{}", capture_stack_trace(2))),
    );
    Value::Object(obj)
}

pub fn is_error_value(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if matches!(o.get("__kab_error"), Some(Value::Bool(true)))
    )
}

fn parse_cause_arg(arg: &Value) -> Option<Value> {
    match arg {
        Value::Object(map) => map.get("cause").cloned(),
        other => Some(other.clone()),
    }
}

fn error_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let msg = match args.first() {
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    let cause = args.get(1).and_then(parse_cause_arg);
    Ok(make_error_with_cause("Error", &msg, cause))
}

fn type_error_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let msg = match args.first() {
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    let cause = args.get(1).and_then(parse_cause_arg);
    Ok(make_error_with_cause("TypeError", &msg, cause))
}

fn reference_error_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let msg = match args.first() {
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    let cause = args.get(1).and_then(parse_cause_arg);
    Ok(make_error_with_cause("ReferenceError", &msg, cause))
}

fn range_error_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let msg = match args.first() {
        Some(v) => crate::value::format_value(v),
        None => String::new(),
    };
    let cause = args.get(1).and_then(parse_cause_arg);
    Ok(make_error_with_cause("RangeError", &msg, cause))
}

fn is_error_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_error(v)")?;
    Ok(Value::Bool(is_error_value(v)))
}

fn error_message_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("error_message(err)")?;
    let Value::Object(o) = v else {
        return Err("error_message() expects an error object".into());
    };
    match o.get("message") {
        Some(Value::String(s)) => Ok(Value::String(s.clone())),
        _ => Ok(Value::String(String::new())),
    }
}

fn error_cause_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("error_cause(err)")?;
    let Value::Object(o) = v else {
        return Err("error_cause() expects an error object".into());
    };
    Ok(o.get("cause").cloned().unwrap_or(Value::Undefined))
}

fn error_stack_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("error_stack(err)")?;
    let Value::Object(o) = v else {
        return Err("error_stack() expects an error object".into());
    };
    match o.get("stack") {
        Some(Value::String(s)) => Ok(Value::String(s.clone())),
        _ => Ok(Value::Undefined),
    }
}

pub fn register_error(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("error_new", error_new_native),
        ("error", error_new_native),
        ("type_error", type_error_native),
        ("reference_error", reference_error_native),
        ("range_error", range_error_native),
        ("is_error", is_error_native),
        ("error_message", error_message_native),
        ("error_cause", error_cause_native),
        ("error_stack", error_stack_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
