//! `Deno.permissions` — capability query, request, revoke (runtime prompts).

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionState {
    Granted,
    Denied,
    Prompt,
}

thread_local! {
    static PERMISSIONS: RefCell<HashMap<String, PermissionState>> = RefCell::new(HashMap::new());
}

fn perm_key(name: &str, path: Option<&str>) -> String {
    match path {
        Some(p) if !p.is_empty() => format!("{name}:{p}"),
        _ => name.to_string(),
    }
}

fn parse_descriptor(v: &Value) -> Result<(&str, Option<String>), String> {
    let Value::Object(map) = v else {
        return Err("permission descriptor expects object".into());
    };
    let name = match map.get("name") {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("permission descriptor requires name".into()),
    };
    let path = map
        .get("path")
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        });
    Ok((name, path))
}

fn state_name(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        PermissionState::Prompt => "prompt",
    }
}

fn default_state(name: &str) -> PermissionState {
    match name {
        "env" | "read" | "write" | "net" | "run" | "ffi" | "import" => PermissionState::Prompt,
        _ => PermissionState::Denied,
    }
}

fn query_state(name: &str, path: Option<&str>) -> PermissionState {
    let key = perm_key(name, path);
    PERMISSIONS.with(|m| {
        m.borrow()
            .get(&key)
            .copied()
            .unwrap_or_else(|| default_state(name))
    })
}

fn set_state(name: &str, path: Option<&str>, state: PermissionState) {
    let key = perm_key(name, path);
    PERMISSIONS.with(|m| {
        m.borrow_mut().insert(key, state);
    });
}

fn permissions_query_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = args.first().ok_or("permissions_query(descriptor)")?;
    let (name, path) = parse_descriptor(desc)?;
    Ok(Value::String(
        state_name(query_state(name, path.as_deref())).into(),
    ))
}

fn permissions_request_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = args.first().ok_or("permissions_request(descriptor)")?;
    let (name, path) = parse_descriptor(desc)?;
    let current = query_state(name, path.as_deref());
    let resolved = match current {
        PermissionState::Prompt => PermissionState::Granted,
        other => other,
    };
    set_state(name, path.as_deref(), resolved);
    Ok(Value::String(state_name(resolved).into()))
}

fn permissions_revoke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = args.first().ok_or("permissions_revoke(descriptor)")?;
    let (name, path) = parse_descriptor(desc)?;
    set_state(name, path.as_deref(), PermissionState::Denied);
    Ok(Value::Undefined)
}

fn permissions_grant_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = args.first().ok_or("permissions_grant(descriptor)")?;
    let (name, path) = parse_descriptor(desc)?;
    set_state(name, path.as_deref(), PermissionState::Granted);
    Ok(Value::Undefined)
}

pub fn build_permissions_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_deno_permissions".into(), Value::Bool(true));
    m.insert(
        "query".into(),
        Value::NativeFunction(permissions_query_native),
    );
    m.insert(
        "request".into(),
        Value::NativeFunction(permissions_request_native),
    );
    m.insert(
        "revoke".into(),
        Value::NativeFunction(permissions_revoke_native),
    );
    m.insert(
        "grant".into(),
        Value::NativeFunction(permissions_grant_native),
    );
    Value::Object(m)
}

pub fn register_permissions(env: &mut Environment) {
    env.set(
        "Deno_permissions".to_string(),
        build_permissions_namespace(),
    );
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("permissions_query", permissions_query_native),
        ("permissions_request", permissions_request_native),
        ("permissions_revoke", permissions_revoke_native),
        ("permissions_grant", permissions_grant_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
