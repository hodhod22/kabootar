//! Deno.fs parity — file APIs mapped onto Kabootar OS VFS.

use crate::runtime::os::{os_handle, VfsEntryKind, VfsStat};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn expect_path(args: &[Value], name: &str) -> Result<String, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects a string path")),
    }
}

fn join_vfs_path(dir: &str, name: &str) -> String {
    let dir = if dir.is_empty() { "/" } else { dir };
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

fn stat_to_object(stat: VfsStat) -> Value {
    let mut m = HashMap::new();
    m.insert(
        "isFile".into(),
        Value::Bool(stat.kind == VfsEntryKind::File),
    );
    m.insert(
        "isDirectory".into(),
        Value::Bool(stat.kind == VfsEntryKind::Directory),
    );
    m.insert("isSymlink".into(), Value::Bool(false));
    m.insert("size".into(), Value::Number(stat.size as i64));
    m.insert("mtime".into(), Value::Number(stat.mtime as i64));
    m.insert("readonly".into(), Value::Bool(stat.readonly));
    if let Some(mount) = stat.mount {
        m.insert("mount".into(), Value::String(mount));
    }
    Value::from_object(m)
}

fn bytes_to_array(data: &[u8]) -> Value {
    Value::from_array(
        data.iter()
            .map(|b| Value::Number(*b as i64))
            .collect(),
    )
}

fn value_to_bytes(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()), Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    Value::Number(n) if (0..=255).contains(n) => out.push(*n as u8),
                    _ => return Err("write_file bytes must be numbers 0–255".into()),
                }
            }
            Ok(out)
        }
        _ => Err("write_file expects string or byte array".into()),
    }
}

fn read_file_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "read_file(path)")?;
    let text = os_handle(env)?.read(&path)?;
    Ok(bytes_to_array(text.as_bytes()))
}

fn write_file_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "write_file(path, data)")?;
    let data = args.get(1).ok_or("write_file(path, data)")?;
    let bytes = value_to_bytes(data)?;
    let content = String::from_utf8_lossy(&bytes).into_owned();
    os_handle(env)?.write(&path, content)?;
    Ok(Value::Undefined)
}

fn read_dir_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let dir = args
        .first()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            _ => "/".to_string(),
        })
        .unwrap_or_else(|| "/".to_string());
    let os = os_handle(env)?;
    let names = os.list(&dir)?;
    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        let path = join_vfs_path(&dir, &name);
        let stat = os.stat(&path)?;
        let mut entry = HashMap::new();
        entry.insert("name".into(), Value::String(name));
        entry.insert(
            "isFile".into(),
            Value::Bool(stat.kind == VfsEntryKind::File),
        );
        entry.insert(
            "isDirectory".into(),
            Value::Bool(stat.kind == VfsEntryKind::Directory),
        );
        entries.push(Value::from_object(entry));
    }
    Ok(Value::from_array(entries))
}

fn mkdir_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "mkdir(path)")?;
    os_handle(env)?.mkdir(&path)?;
    Ok(Value::Undefined)
}

fn stat_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "stat(path)")?;
    Ok(stat_to_object(os_handle(env)?.stat(&path)?))
}

fn remove_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "remove(path)")?;
    os_handle(env)?.delete(&path)?;
    Ok(Value::Undefined)
}

fn exists_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_path(args, "exists(path)")?;
    Ok(Value::Bool(os_handle(env)?.exists(&path)?))
}

pub fn register_fs(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("read_file", read_file_native),
        ("Deno_readFile", read_file_native),
        ("write_file", write_file_native),
        ("Deno_writeFile", write_file_native),
        ("read_dir", read_dir_native),
        ("Deno_readDir", read_dir_native),
        ("mkdir", mkdir_native),
        ("Deno_mkdir", mkdir_native),
        ("stat", stat_native),
        ("Deno_stat", stat_native),
        ("remove", remove_native),
        ("Deno_remove", remove_native),
        ("exists", exists_native),
        ("Deno_exists", exists_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
