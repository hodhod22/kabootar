//! Deno `openKv` parity — backed by Kabootar SQL (`db_open` / KDB + WAL).

use crate::runtime::db::DbConnection;
use crate::sql::is_binary_kdb;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_KV: AtomicU64 = AtomicU64::new(1);
static NEXT_WATCH: AtomicU64 = AtomicU64::new(1);
static NEXT_LISTEN: AtomicU64 = AtomicU64::new(1);

const KV_TABLE: &str = "_kab_kv";
const KV_KEY_COL: &str = "kv_key";
const KV_VALUE_COL: &str = "kv_value";
const KV_VERSION_COL: &str = "kv_version";
const WATCH_MAX_BUFFER: usize = 64;

thread_local! {
    static KV_DBS: RefCell<HashMap<u64, KvDatabase>> = RefCell::new(HashMap::new());
    static KV_WATCHES: RefCell<HashMap<u64, KvWatch>> = RefCell::new(HashMap::new());
    static KV_LISTENS: RefCell<HashMap<u64, KvListen>> = RefCell::new(HashMap::new());
}

struct KvDatabase {
    db: DbConnection,
}

struct KvWatch {
    kv_id: u64,
    prefix: String,
    stream_id: u64,
}

struct KvListen {
    stream_id: u64,
}

#[derive(Debug, Clone)]
struct KvEntry {
    value: Value,
    version: i64,
}

#[derive(Debug, Clone)]
enum KvAtomicOp {
    Set { key: String, value: Value },
    Delete { key: String },
    Get { key: String },
    Check { key: String, value: Value },
    CheckVersion { key: String, version: i64 },
    Sum { key: String, value: i64 },
    Max { key: String, value: i64 },
    Min { key: String, value: i64 },
    Enqueue { key: String, value: Value },
}

#[derive(Debug, Clone)]
struct WatchEvent {
    key: String,
    kind: String,
    value: Value,
    version: i64,
}

pub fn ensure_kv_schema(db: &DbConnection) -> Result<(), String> {
    db.execute_sql(
        &format!(
            "CREATE TABLE IF NOT EXISTS {KV_TABLE} \
             ({KV_KEY_COL} TEXT PRIMARY KEY, {KV_VALUE_COL} TEXT NOT NULL, \
              {KV_VERSION_COL} INTEGER NOT NULL DEFAULT 0)"
        ),
        &[],
    )?;
    let probe = db.execute_sql(
        &format!("SELECT {KV_VERSION_COL} FROM {KV_TABLE} LIMIT 0"),
        &[],
    );
    if probe.is_err() {
        let _ = db.execute_sql(
            &format!(
                "ALTER TABLE {KV_TABLE} ADD COLUMN {KV_VERSION_COL} INTEGER NOT NULL DEFAULT 0"
            ),
            &[],
        );
    }
    Ok(())
}

fn key_string(parts: &[Value]) -> Result<String, String> {
    if parts.is_empty() {
        return Err("kv key must be a non-empty array".into());
    }
    let mut out = Vec::new();
    for p in parts {
        out.push(match p {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => crate::value::format_value(other),
        });
    }
    Ok(out.join("\x1f"))
}

fn key_to_array(key: &str) -> Value {
    Value::Array(
        key.split('\x1f')
            .map(|s| Value::String(s.to_string()))
            .collect(),
    )
}

fn like_prefix(prefix: &str) -> String {
    let mut out = String::new();
    for ch in prefix.chars() {
        match ch {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out.push('%');
    out
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Null, _) | (_, Value::Null) => false,
        _ => format!("{a:?}") == format!("{b:?}"),
    }
}

fn value_as_i64(v: &Value) -> Result<i64, String> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Null => Ok(0),
        _ => Err(format!("expected number, got {:?}", v)),
    }
}

fn queue_from_value(value: &Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::Null => Ok(Vec::new()),
        _ => Err("kv queue key must hold an array".into()),
    }
}

fn entry_object(key: &str, value: Value, version: i64) -> Value {
    let mut m = HashMap::new();
    m.insert("key".into(), key_to_array(key));
    m.insert("value".into(), value);
    m.insert("version".into(), Value::Number(version));
    Value::Object(m)
}

fn watch_event(kind: &str, key: &str, value: Value, version: i64) -> Value {
    let mut m = HashMap::new();
    m.insert("kind".into(), Value::String(kind.to_string()));
    m.insert("key".into(), key_to_array(key));
    m.insert("value".into(), value);
    m.insert("version".into(), Value::Number(version));
    Value::Object(m)
}

fn key_matches_prefix(key: &str, prefix: &str) -> bool {
    prefix.is_empty() || key.starts_with(prefix)
}

fn emit_watchers(kv_id: u64, events: &[WatchEvent]) {
    if events.is_empty() {
        return;
    }
    KV_WATCHES.with(|m| {
        let watches = m.borrow();
        for event in events {
            for watch in watches.values() {
                if watch.kv_id == kv_id && key_matches_prefix(&event.key, &watch.prefix) {
                    crate::runtime::stdlib::deno::stream_push_capped(
                        watch.stream_id,
                        watch_event(
                            &event.kind,
                            &event.key,
                            event.value.clone(),
                            event.version,
                        ),
                        WATCH_MAX_BUFFER,
                    );
                }
            }
        }
    });
}

fn remove_watches_for_kv(kv_id: u64) {
    KV_WATCHES.with(|m| {
        let mut map = m.borrow_mut();
        let ids: Vec<u64> = map
            .iter()
            .filter(|(_, w)| w.kv_id == kv_id)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            if let Some(watch) = map.remove(&id) {
                crate::runtime::stdlib::deno::stream_remove(watch.stream_id);
            }
        }
    });
}

fn read_legacy_kv_json(path: &str) -> Result<Option<HashMap<String, Value>>, String> {
    if !Path::new(path).exists() || is_binary_kdb(path) {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("open_kv read: {e}"))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    match crate::runtime::stdlib::json::parse(&text) {
        Ok(Value::Object(map))
            if map.contains_key("version") && map.contains_key("tables") =>
        {
            Ok(None)
        }
        Ok(Value::Object(map)) => Ok(Some(map)),
        _ => Ok(None),
    }
}

fn import_legacy(db: &DbConnection, data: HashMap<String, Value>) -> Result<(), String> {
    for (key, value) in data {
        kv_set_db(db, &key, value)?;
    }
    db.execute_sql("CHECKPOINT", &[])?;
    Ok(())
}

fn value_to_text(value: &Value) -> String {
    crate::runtime::stdlib::json::stringify(value)
}

fn text_to_value(text: &str) -> Result<Value, String> {
    crate::runtime::stdlib::json::parse(text)
}

fn kv_version_db(db: &DbConnection, key: &str) -> Result<i64, String> {
    match db.execute_sql(
        &format!("SELECT {KV_VERSION_COL} FROM {KV_TABLE} WHERE {KV_KEY_COL} = $1"),
        &[Value::String(key.to_string())],
    )? {
        Value::Array(rows) if rows.is_empty() => Ok(0),
        Value::Number(n) => Ok(n),
        _ => Ok(0),
    }
}

fn kv_set_db(db: &DbConnection, key: &str, value: Value) -> Result<i64, String> {
    let next_version = kv_version_db(db, key)? + 1;
    let encoded = value_to_text(&value);
    db.execute_sql(
        &format!(
            "INSERT INTO {KV_TABLE} ({KV_KEY_COL}, {KV_VALUE_COL}, {KV_VERSION_COL}) \
             VALUES ($1, $2, $3) ON CONFLICT ({KV_KEY_COL}) DO UPDATE SET \
             {KV_VALUE_COL} = $2, {KV_VERSION_COL} = $3"
        ),
        &[
            Value::String(key.to_string()),
            Value::String(encoded),
            Value::Number(next_version),
        ],
    )?;
    Ok(next_version)
}

fn parse_entry_row(result: Value) -> Result<Option<(Value, i64)>, String> {
    match result {
        Value::Array(rows) if rows.is_empty() => Ok(None),
        Value::Array(rows) if rows.len() == 1 => match &rows[0] {
            Value::Array(cols) if cols.len() >= 2 => {
                let value = match &cols[0] {
                    Value::String(text) => text_to_value(text)?,
                    other => other.clone(),
                };
                let version = match cols[1] {
                    Value::Number(n) => n,
                    _ => 0,
                };
                Ok(Some((value, version)))
            }
            _ => Err("unexpected kv entry row".into()),
        },
        Value::String(text) => Ok(Some((text_to_value(&text)?, 0))),
        _ => Ok(None),
    }
}

fn kv_get_entry_db(db: &DbConnection, key: &str) -> Result<KvEntry, String> {
    let result = db.execute_sql(
        &format!(
            "SELECT {KV_VALUE_COL}, {KV_VERSION_COL} FROM {KV_TABLE} WHERE {KV_KEY_COL} = $1"
        ),
        &[Value::String(key.to_string())],
    )?;
    match parse_entry_row(result)? {
        Some((value, version)) => Ok(KvEntry { value, version }),
        None => Ok(KvEntry {
            value: Value::Null,
            version: 0,
        }),
    }
}

fn kv_get_db(db: &DbConnection, key: &str) -> Result<Value, String> {
    Ok(kv_get_entry_db(db, key)?.value)
}

fn kv_delete_db(db: &DbConnection, key: &str) -> Result<i64, String> {
    let prev = kv_version_db(db, key)?;
    if prev == 0 {
        db.execute_sql(
            &format!("DELETE FROM {KV_TABLE} WHERE {KV_KEY_COL} = $1"),
            &[Value::String(key.to_string())],
        )?;
        return Ok(0);
    }
    let next_version = prev + 1;
    db.execute_sql(
        &format!("DELETE FROM {KV_TABLE} WHERE {KV_KEY_COL} = $1"),
        &[Value::String(key.to_string())],
    )?;
    Ok(next_version)
}

fn kv_list_db(db: &DbConnection, prefix: &str) -> Result<Vec<(Value, Value)>, String> {
    let pattern = if prefix.is_empty() {
        "%".to_string()
    } else {
        like_prefix(prefix)
    };
    let result = db.execute_sql(
        &format!(
            "SELECT {KV_KEY_COL}, {KV_VALUE_COL} FROM {KV_TABLE} \
             WHERE {KV_KEY_COL} LIKE $1 ORDER BY {KV_KEY_COL}"
        ),
        &[Value::String(pattern)],
    )?;
    parse_list_rows(result)
}

fn kv_list_entries_db(db: &DbConnection, prefix: &str) -> Result<Vec<Value>, String> {
    let pattern = if prefix.is_empty() {
        "%".to_string()
    } else {
        like_prefix(prefix)
    };
    let result = db.execute_sql(
        &format!(
            "SELECT {KV_KEY_COL}, {KV_VALUE_COL}, {KV_VERSION_COL} FROM {KV_TABLE} \
             WHERE {KV_KEY_COL} LIKE $1 ORDER BY {KV_KEY_COL}"
        ),
        &[Value::String(pattern)],
    )?;
    parse_list_entry_rows(result)
}

fn parse_list_entry_rows(result: Value) -> Result<Vec<Value>, String> {
    let Value::Array(rows) = result else {
        return Err("unexpected kv_list_entries result".into());
    };
    let mut out = Vec::new();
    for row in rows {
        match row {
            Value::Array(cols) if cols.len() >= 3 => {
                let Value::String(key) = &cols[0] else {
                    return Err("kv_list_entries key must be string".into());
                };
                let value = match &cols[1] {
                    Value::String(text) => text_to_value(text)?,
                    other => other.clone(),
                };
                let version = match cols[2] {
                    Value::Number(n) => n,
                    _ => 0,
                };
                out.push(entry_object(key, value, version));
            }
            _ => return Err("unexpected kv_list_entries row".into()),
        }
    }
    Ok(out)
}

fn kv_enqueue_db(db: &DbConnection, key: &str, value: Value) -> Result<(Value, i64), String> {
    let entry = kv_get_entry_db(db, key)?;
    let mut queue = queue_from_value(&entry.value)?;
    queue.push(value.clone());
    let version = kv_set_db(db, key, Value::Array(queue))?;
    Ok((value, version))
}

fn kv_dequeue_db(db: &DbConnection, key: &str) -> Result<Option<(Value, i64)>, String> {
    let entry = kv_get_entry_db(db, key)?;
    let mut queue = queue_from_value(&entry.value)?;
    if queue.is_empty() {
        return Ok(None);
    }
    let item = queue.remove(0);
    let version = kv_set_db(db, key, Value::Array(queue))?;
    Ok(Some((item, version)))
}

fn parse_list_rows(result: Value) -> Result<Vec<(Value, Value)>, String> {
    let Value::Array(rows) = result else {
        return Err("unexpected kv_list result".into());
    };
    let mut out = Vec::new();
    for row in rows {
        match row {
            Value::Array(cols) if cols.len() >= 2 => {
                let Value::String(key) = &cols[0] else {
                    return Err("kv_list key must be string".into());
                };
                let value = match &cols[1] {
                    Value::String(text) => text_to_value(text)?,
                    other => other.clone(),
                };
                out.push((key_to_array(key), value));
            }
            _ => return Err("unexpected kv_list row".into()),
        }
    }
    Ok(out)
}

fn parse_atomic_op(value: &Value) -> Result<KvAtomicOp, String> {
    let Value::Object(map) = value else {
        return Err("kv_atomic op must be an object".into());
    };
    let op = match map.get("op") {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kv_atomic op requires string field 'op'".into()),
    };
    let key = match map.get("key") {
        Some(Value::Array(parts)) => key_string(parts)?,
        _ => return Err("kv_atomic op requires array field 'key'".into()),
    };
    match op {
        "set" => Ok(KvAtomicOp::Set {
            key,
            value: map.get("value").cloned().unwrap_or(Value::Null),
        }),
        "delete" => Ok(KvAtomicOp::Delete { key }),
        "get" => Ok(KvAtomicOp::Get { key }),
        "check" => {
            if let Some(Value::Number(v)) = map.get("version") {
                return Ok(KvAtomicOp::CheckVersion { key, version: *v });
            }
            Ok(KvAtomicOp::Check {
                key,
                value: map.get("value").cloned().unwrap_or(Value::Null),
            })
        }
        "sum" => Ok(KvAtomicOp::Sum {
            key,
            value: value_as_i64(map.get("value").unwrap_or(&Value::Number(0)))?,
        }),
        "max" => Ok(KvAtomicOp::Max {
            key,
            value: value_as_i64(map.get("value").unwrap_or(&Value::Number(0)))?,
        }),
        "min" => Ok(KvAtomicOp::Min {
            key,
            value: value_as_i64(map.get("value").unwrap_or(&Value::Number(0)))?,
        }),
        "enqueue" => Ok(KvAtomicOp::Enqueue {
            key,
            value: map.get("value").cloned().unwrap_or(Value::Null),
        }),
        other => Err(format!("unknown kv_atomic op: {other}")),
    }
}

fn run_atomic_op(
    db: &DbConnection,
    op: &KvAtomicOp,
    pending: &mut Vec<WatchEvent>,
    results: &mut Vec<Value>,
) -> Result<(), String> {
    match op {
        KvAtomicOp::Set { key, value } => {
            let version = kv_set_db(db, key, value.clone())?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "set".into(),
                value: value.clone(),
                version,
            });
            Ok(())
        }
        KvAtomicOp::Delete { key } => {
            let version = kv_delete_db(db, key)?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "delete".into(),
                value: Value::Null,
                version,
            });
            Ok(())
        }
        KvAtomicOp::Get { key } => {
            let entry = kv_get_entry_db(db, key)?;
            results.push(entry_object(key, entry.value, entry.version));
            Ok(())
        }
        KvAtomicOp::Check { key, value } => {
            let entry = kv_get_entry_db(db, key)?;
            if !values_equal(&entry.value, value) {
                return Err("kv_atomic check failed".into());
            }
            results.push(Value::Bool(true));
            Ok(())
        }
        KvAtomicOp::CheckVersion { key, version } => {
            let entry = kv_get_entry_db(db, key)?;
            if entry.version != *version {
                return Err("kv_atomic version check failed".into());
            }
            results.push(Value::Bool(true));
            Ok(())
        }
        KvAtomicOp::Sum { key, value } => {
            let entry = kv_get_entry_db(db, key)?;
            let current = value_as_i64(&entry.value)?;
            let next = current + value;
            let version = kv_set_db(db, key, Value::Number(next))?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "set".into(),
                value: Value::Number(next),
                version,
            });
            results.push(Value::Number(next));
            Ok(())
        }
        KvAtomicOp::Max { key, value } => {
            let entry = kv_get_entry_db(db, key)?;
            let current = value_as_i64(&entry.value)?;
            let next = if matches!(entry.value, Value::Null) {
                *value
            } else {
                current.max(*value)
            };
            let version = kv_set_db(db, key, Value::Number(next))?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "set".into(),
                value: Value::Number(next),
                version,
            });
            results.push(Value::Number(next));
            Ok(())
        }
        KvAtomicOp::Min { key, value } => {
            let entry = kv_get_entry_db(db, key)?;
            let current = value_as_i64(&entry.value)?;
            let next = if matches!(entry.value, Value::Null) {
                *value
            } else {
                current.min(*value)
            };
            let version = kv_set_db(db, key, Value::Number(next))?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "set".into(),
                value: Value::Number(next),
                version,
            });
            results.push(Value::Number(next));
            Ok(())
        }
        KvAtomicOp::Enqueue { key, value } => {
            let (item, version) = kv_enqueue_db(db, key, value.clone())?;
            let entry = kv_get_entry_db(db, key)?;
            pending.push(WatchEvent {
                key: key.clone(),
                kind: "set".into(),
                value: entry.value,
                version,
            });
            results.push(item);
            Ok(())
        }
    }
}

fn register_kv_db(db: DbConnection) -> Result<u64, String> {
    ensure_kv_schema(&db)?;
    let id = NEXT_KV.fetch_add(1, Ordering::Relaxed);
    KV_DBS.with(|m| {
        m.borrow_mut().insert(id, KvDatabase { db });
    });
    Ok(id)
}

fn register_watch(id: u64, prefix: String) -> u64 {
    let stream_id = crate::runtime::stdlib::deno::stream_allocate();
    let watch_id = NEXT_WATCH.fetch_add(1, Ordering::Relaxed);
    KV_WATCHES.with(|m| {
        m.borrow_mut().insert(
            watch_id,
            KvWatch {
                kv_id: id,
                prefix,
                stream_id,
            },
        );
    });
    stream_id
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_kv(path: &str) -> Result<u64, String> {
    let legacy = read_legacy_kv_json(path)?;
    let db = DbConnection::open(path);
    ensure_kv_schema(&db)?;
    if let Some(data) = legacy {
        import_legacy(&db, data)?;
    }
    register_kv_db(db)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_kv_db(db: DbConnection) -> Result<u64, String> {
    register_kv_db(db)
}

#[cfg(target_arch = "wasm32")]
pub fn open_kv(_path: &str) -> Result<u64, String> {
    Err("open_kv() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn open_kv_db(_db: DbConnection) -> Result<u64, String> {
    Err("open_kv_db() is not available on wasm32".into())
}

fn with_db<T>(id: u64, f: impl FnOnce(&DbConnection) -> Result<T, String>) -> Result<T, String> {
    KV_DBS.with(|m| {
        let map = m.borrow();
        let db = map
            .get(&id)
            .ok_or_else(|| format!("invalid kv id {id}"))?;
        f(&db.db)
    })
}

pub fn kv_get(id: u64, key_parts: &[Value]) -> Result<Value, String> {
    let key = key_string(key_parts)?;
    with_db(id, |db| kv_get_db(db, &key))
}

pub fn kv_get_entry(id: u64, key_parts: &[Value]) -> Result<Value, String> {
    let key = key_string(key_parts)?;
    with_db(id, |db| {
        let entry = kv_get_entry_db(db, &key)?;
        Ok(entry_object(&key, entry.value, entry.version))
    })
}

pub fn kv_get_version(id: u64, key_parts: &[Value]) -> Result<Value, String> {
    let key = key_string(key_parts)?;
    with_db(id, |db| Ok(Value::Number(kv_version_db(db, &key)?)))
}

pub fn kv_set(id: u64, key_parts: &[Value], value: Value) -> Result<(), String> {
    let key = key_string(key_parts)?;
    let version = with_db(id, |db| kv_set_db(db, &key, value.clone()))?;
    emit_watchers(
        id,
        &[WatchEvent {
            key,
            kind: "set".into(),
            value,
            version,
        }],
    );
    Ok(())
}

pub fn kv_delete(id: u64, key_parts: &[Value]) -> Result<(), String> {
    let key = key_string(key_parts)?;
    let version = with_db(id, |db| kv_delete_db(db, &key))?;
    emit_watchers(
        id,
        &[WatchEvent {
            key,
            kind: "delete".into(),
            value: Value::Null,
            version,
        }],
    );
    Ok(())
}

pub fn kv_list(id: u64, prefix_parts: &[Value]) -> Result<Vec<(Value, Value)>, String> {
    let prefix = if prefix_parts.is_empty() {
        String::new()
    } else {
        key_string(prefix_parts)?
    };
    with_db(id, |db| kv_list_db(db, &prefix))
}

pub fn kv_list_entries(id: u64, prefix_parts: &[Value]) -> Result<Vec<Value>, String> {
    let prefix = if prefix_parts.is_empty() {
        String::new()
    } else {
        key_string(prefix_parts)?
    };
    with_db(id, |db| kv_list_entries_db(db, &prefix))
}

pub fn kv_enqueue(id: u64, key_parts: &[Value], value: Value) -> Result<Value, String> {
    let key = key_string(key_parts)?;
    let (item, version) = with_db(id, |db| kv_enqueue_db(db, &key, value))?;
    let entry = with_db(id, |db| kv_get_entry_db(db, &key))?;
    emit_watchers(
        id,
        &[WatchEvent {
            key,
            kind: "set".into(),
            value: entry.value,
            version,
        }],
    );
    Ok(item)
}

pub fn kv_dequeue(id: u64, key_parts: &[Value]) -> Result<Value, String> {
    let key = key_string(key_parts)?;
    let outcome = with_db(id, |db| kv_dequeue_db(db, &key))?;
    let Some((item, version)) = outcome else {
        return Ok(Value::Null);
    };
    let entry = with_db(id, |db| kv_get_entry_db(db, &key))?;
    emit_watchers(
        id,
        &[WatchEvent {
            key,
            kind: "set".into(),
            value: entry.value,
            version,
        }],
    );
    Ok(item)
}

pub fn kv_listen_stream_id(listen: &Value) -> Result<u64, String> {
    let listen_id = kv_listen_id(listen)?;
    KV_LISTENS.with(|m| {
        m.borrow()
            .get(&listen_id)
            .map(|l| l.stream_id)
            .ok_or_else(|| format!("invalid kv listen id {listen_id}"))
    })
}

pub fn kv_stream_read_event(stream_id: u64) -> Result<Value, String> {
    match crate::runtime::stdlib::deno::stream_read_impl(stream_id)? {
        Value::Object(frame) => {
            if matches!(frame.get("done"), Some(Value::Bool(true))) {
                Ok(Value::Null)
            } else {
                Ok(frame.get("value").cloned().unwrap_or(Value::Null))
            }
        }
        other => Ok(other),
    }
}

pub fn kv_watch(id: u64, prefix_parts: &[Value]) -> Result<Value, String> {
    let prefix = if prefix_parts.is_empty() {
        String::new()
    } else {
        key_string(prefix_parts)?
    };
    let stream_id = register_watch(id, prefix);
    Ok(crate::runtime::stdlib::deno::stream_object_pub(stream_id))
}

pub fn kv_listen(id: u64, prefix_parts: &[Value]) -> Result<Value, String> {
    let stream_id = if prefix_parts.is_empty() {
        register_watch(id, String::new())
    } else {
        register_watch(id, key_string(prefix_parts)?)
    };
    let listen_id = NEXT_LISTEN.fetch_add(1, Ordering::Relaxed);
    KV_LISTENS.with(|m| {
        m.borrow_mut().insert(listen_id, KvListen { stream_id });
    });
    let mut obj = HashMap::new();
    obj.insert("__kab_kv_listen".into(), Value::Bool(true));
    obj.insert("__kab_id".into(), Value::Number(listen_id as i64));
    Ok(Value::Object(obj))
}

pub fn kv_listen_recv(listen: &Value) -> Result<Value, String> {
    let listen_id = kv_listen_id(listen)?;
    let stream_id = KV_LISTENS.with(|m| {
        m.borrow()
            .get(&listen_id)
            .map(|l| l.stream_id)
            .ok_or_else(|| format!("invalid kv listen id {listen_id}"))
    })?;
    match crate::runtime::stdlib::deno::stream_read_impl(stream_id)? {
        Value::Object(frame) => {
            if matches!(frame.get("done"), Some(Value::Bool(true))) {
                Ok(Value::Null)
            } else {
                Ok(frame.get("value").cloned().unwrap_or(Value::Null))
            }
        }
        other => Ok(other),
    }
}

pub fn kv_listen_close(listen: &Value) -> Result<(), String> {
    let listen_id = kv_listen_id(listen)?;
    KV_LISTENS.with(|m| {
        m.borrow_mut().remove(&listen_id);
    });
    Ok(())
}

fn kv_listen_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected kv listen handle".into());
    };
    if !matches!(o.get("__kab_kv_listen"), Some(Value::Bool(true))) {
        return Err("expected kv listen handle".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid kv listen handle".into()),
    }
}

pub fn kv_atomic(id: u64, ops: &[Value]) -> Result<Value, String> {
    if ops.is_empty() {
        return Err("kv_atomic(kv, ops) requires a non-empty ops array".into());
    }
    let mut pending = Vec::new();
    let mut results = Vec::new();
    with_db(id, |db| {
        db.execute_sql("BEGIN TRANSACTION", &[])?;
        let run = (|| {
            for op in ops {
                run_atomic_op(db, &parse_atomic_op(op)?, &mut pending, &mut results)?;
            }
            Ok::<(), String>(())
        })();
        if let Err(e) = run {
            let _ = db.execute_sql("ROLLBACK", &[]);
            return Err(e);
        }
        db.execute_sql("COMMIT TRANSACTION", &[])?;
        Ok(())
    })?;
    emit_watchers(id, &pending);
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("results".into(), Value::Array(results));
    Ok(Value::Object(out))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn kv_close(id: u64) -> Result<(), String> {
    remove_watches_for_kv(id);
    KV_DBS.with(|m| {
        let mut map = m.borrow_mut();
        let db = map
            .remove(&id)
            .ok_or_else(|| format!("invalid kv id {id}"))?;
        db.db.execute_sql("CHECKPOINT", &[])?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
pub fn kv_close(_id: u64) -> Result<(), String> {
    Err("kv_close() is not available on wasm32".into())
}

pub fn kv_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_kv".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::Object(m)
}

pub fn kv_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected kv database".into());
    };
    if !matches!(o.get("__kab_kv"), Some(Value::Bool(true))) {
        return Err("expected kv database".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid kv handle".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> String {
        format!("{name}_{}.json", std::process::id())
    }

    fn cleanup(path: &str) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{path}.wal"));
        let _ = std::fs::remove_file(format!("{path}.wal2"));
    }

    #[test]
    fn open_kv_persists_via_sql_engine() {
        let path = temp_path("open_kv_sql_test");
        cleanup(&path);

        let id = open_kv(&path).expect("open");
        kv_set(
            id,
            &[Value::String("app".into()), Value::String("v".into())],
            Value::String("2".into()),
        )
        .expect("set");
        kv_close(id).expect("close");

        let id2 = open_kv(&path).expect("reopen");
        let v = kv_get(id2, &[Value::String("app".into()), Value::String("v".into())]).expect("get");
        assert!(matches!(v, Value::String(s) if s == "2"));
        kv_close(id2).expect("close2");
        cleanup(&path);
    }

    #[test]
    fn kv_version_increments_on_set() {
        let path = temp_path("open_kv_version");
        cleanup(&path);
        let id = open_kv(&path).expect("open");
        kv_set(id, &[Value::String("k".into())], Value::Number(1)).unwrap();
        let v1 = kv_get_version(id, &[Value::String("k".into())]).unwrap();
        kv_set(id, &[Value::String("k".into())], Value::Number(2)).unwrap();
        let v2 = kv_get_version(id, &[Value::String("k".into())]).unwrap();
        assert!(matches!(v1, Value::Number(1)));
        assert!(matches!(v2, Value::Number(2)));
        kv_close(id).unwrap();
        cleanup(&path);
    }

    #[test]
    fn kv_atomic_version_check() {
        let path = temp_path("open_kv_ver_check");
        cleanup(&path);
        let id = open_kv(&path).expect("open");
        kv_set(id, &[Value::String("x".into())], Value::Number(1)).unwrap();
        let ver = kv_get_version(id, &[Value::String("x".into())]).unwrap();
        let Value::Number(ver_n) = ver else {
            panic!("version");
        };
        assert!(kv_atomic(id, &[atomic_check_version("x", ver_n)]).is_ok());
        assert!(kv_atomic(id, &[atomic_check_version("x", ver_n - 1)]).is_err());
        kv_close(id).unwrap();
        cleanup(&path);
    }

    #[test]
    fn kv_listen_recv_event() {
        let path = temp_path("open_kv_listen");
        cleanup(&path);
        let id = open_kv(&path).expect("open");
        let listen = kv_listen(id, &[Value::String("evt".into())]).expect("listen");
        kv_set(
            id,
            &[Value::String("evt".into()), Value::String("1".into())],
            Value::String("hi".into()),
        )
        .unwrap();
        let ev = kv_listen_recv(&listen).unwrap();
        let Value::Object(obj) = ev else {
            panic!("event");
        };
        assert!(matches!(obj.get("kind"), Some(Value::String(s)) if s == "set"));
        assert!(matches!(obj.get("version"), Some(Value::Number(n)) if *n > 0));
        kv_close(id).unwrap();
        cleanup(&path);
    }

    #[test]
    fn open_kv_shares_db_with_sql() {
        let path = temp_path("open_kv_shared");
        cleanup(&path);
        let sql_db = DbConnection::open(&path);
        ensure_kv_schema(&sql_db).unwrap();
        let kv_id = open_kv(&path).expect("open kv");
        kv_set(kv_id, &[Value::String("shared".into())], Value::String("yes".into())).unwrap();
        let row = sql_db
            .execute_sql(
                &format!("SELECT {KV_VALUE_COL} FROM {KV_TABLE} WHERE {KV_KEY_COL} = $1"),
                &[Value::String("shared".to_string())],
            )
            .unwrap();
        assert!(matches!(row, Value::String(s) if s.contains("yes")));
        kv_close(kv_id).unwrap();
        cleanup(&path);
    }

    fn atomic_check_version(key: &str, version: i64) -> Value {
        let mut m = HashMap::new();
        m.insert("op".into(), Value::String("check".into()));
        m.insert(
            "key".into(),
            Value::Array(vec![Value::String(key.into())]),
        );
        m.insert("version".into(), Value::Number(version));
        Value::Object(m)
    }

    #[test]
    fn kv_atomic_sum() {
        let path = temp_path("open_kv_sum");
        cleanup(&path);
        let id = open_kv(&path).expect("open");
        let mut m = HashMap::new();
        m.insert("op".into(), Value::String("sum".into()));
        m.insert(
            "key".into(),
            Value::Array(vec![Value::String("n".into())]),
        );
        m.insert("value".into(), Value::Number(5));
        let out = kv_atomic(id, &[Value::Object(m.clone())]).expect("sum1");
        let Value::Object(r) = out else {
            panic!("result");
        };
        let Value::Array(results) = r.get("results").cloned().unwrap() else {
            panic!("results");
        };
        assert!(matches!(results.first(), Some(Value::Number(5))));
        m.insert("value".into(), Value::Number(3));
        kv_atomic(id, &[Value::Object(m)]).expect("sum2");
        assert!(matches!(
            kv_get(id, &[Value::String("n".into())]),
            Ok(Value::Number(8))
        ));
        kv_close(id).unwrap();
        cleanup(&path);
    }

    #[test]
    fn kv_enqueue_dequeue_roundtrip() {
        let path = temp_path("open_kv_queue");
        cleanup(&path);
        let id = open_kv(&path).expect("open");
        assert!(matches!(
            kv_enqueue(id, &[Value::String("q".into())], Value::String("a".into())),
            Ok(Value::String(s)) if s == "a"
        ));
        assert!(matches!(
            kv_dequeue(id, &[Value::String("q".into())]),
            Ok(Value::String(s)) if s == "a"
        ));
        assert!(matches!(
            kv_dequeue(id, &[Value::String("q".into())]),
            Ok(Value::Null)
        ));
        kv_close(id).unwrap();
        cleanup(&path);
    }
}
