//! Opt-in `@manual` ownership — MemBox over `os_mem_*`.

use crate::lang_preprocess::MemoryMode;
use crate::value::{Environment, OwnedBuf, Value};

const MODE_KEY: &str = "__memory_mode";

pub fn set_memory_mode(env: &mut Environment, mode: MemoryMode) {
    env.set(
        MODE_KEY.to_string(),
        Value::String(mode.as_str().to_string()),
    );
}

pub fn is_manual(env: &Environment) -> bool {
    matches!(
        env.get(MODE_KEY),
        Some(Value::String(s)) if s == "manual"
    )
}

fn require_manual(env: &Environment) -> Result<(), String> {
    if is_manual(env) {
        Ok(())
    } else {
        Err(
            "owned_* requires @manual module (systems memory); default is GC".into(),
        )
    }
}

fn expect_owned<'a>(args: &'a [Value], idx: usize, what: &str) -> Result<&'a OwnedBuf, String> {
    match args.get(idx) {
        Some(Value::Owned(buf)) => Ok(buf),
        _ => Err(format!("{what} expects Owned buffer")),
    }
}

fn owned_alloc_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    require_manual(env)?;
    let size = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("owned_alloc(size, label?) expects size".into()),
    };
    let label = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("owned");
    let id = crate::runtime::os::os_handle(env)?.mem_alloc(size, label)?;
    Ok(Value::Owned(OwnedBuf::new(id)))
}

fn owned_read_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    require_manual(env)?;
    let buf = expect_owned(args, 0, "owned_read")?;
    let id = buf.peek_id()?;
    let offset = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(0);
    let len = args
        .get(2)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(64);
    let bytes = crate::runtime::os::os_handle(env)?.mem_read(id, offset, len)?;
    Ok(Value::Array(
        bytes.into_iter().map(|b| Value::Number(b as i64)).collect(),
    ))
}

fn owned_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    require_manual(env)?;
    let buf = expect_owned(args, 0, "owned_write")?;
    let id = buf.peek_id()?;
    let offset = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(0);
    let data: Vec<u8> = match args.get(2) {
        Some(Value::Array(vals)) => vals
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n as u8),
                _ => Err("owned_write expects byte array".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => Vec::new(),
    };
    let n = crate::runtime::os::os_handle(env)?.mem_write(id, offset, &data)?;
    Ok(Value::Number(n as i64))
}

fn drop_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    require_manual(env)?;
    let buf = expect_owned(args, 0, "drop")?;
    let id = buf.take_drop()?;
    let _ = crate::runtime::os::os_handle(env)?.mem_free(id)?;
    Ok(Value::Null)
}

/// Explicit move: invalidate `buf` and return a fresh live handle (same region).
fn owned_move_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    require_manual(env)?;
    let buf = expect_owned(args, 0, "owned_move")?;
    Ok(Value::Owned(buf.take_move()?))
}

/// Free an Owned value if still alive and this is the unique handle (scope / overwrite).
pub fn drop_owned_value(v: &Value, env: &mut Environment) -> Result<(), String> {
    if let Value::Owned(buf) = v {
        // Shared clones (e.g. call args) must not free the caller's buffer.
        if std::rc::Rc::strong_count(&buf.slot) > 1 {
            return Ok(());
        }
        if let Ok(id) = buf.take_drop() {
            let _ = crate::runtime::os::os_handle(env)?.mem_free(id)?;
        }
    }
    Ok(())
}

pub fn ownership_globals(env: &mut Environment) {
    env.set(
        "owned_alloc".to_string(),
        Value::NativeFunction(owned_alloc_native),
    );
    env.set(
        "owned_read".to_string(),
        Value::NativeFunction(owned_read_native),
    );
    env.set(
        "owned_write".to_string(),
        Value::NativeFunction(owned_write_native),
    );
    env.set(
        "owned_move".to_string(),
        Value::NativeFunction(owned_move_native),
    );
    env.set("drop".to_string(), Value::NativeFunction(drop_native));
}
