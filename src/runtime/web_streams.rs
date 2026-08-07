//! WHATWG Web Streams — readable/writable, readers/writers, bytes, transform, transfer.

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_STREAM: AtomicU64 = AtomicU64::new(1);
static NEXT_WRITABLE: AtomicU64 = AtomicU64::new(1);
static NEXT_READER: AtomicU64 = AtomicU64::new(1);
static NEXT_WRITER: AtomicU64 = AtomicU64::new(1);
static NEXT_TRANSFER: AtomicU64 = AtomicU64::new(1);

const WRITABLE_HIGH_WATER: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadableState {
    Readable,
    Locked,
    Closed,
    Errored,
    Cancelled,
}

#[derive(Debug, Clone)]
struct ReadableRecord {
    chunks: Vec<Value>,
    bytes: Vec<u8>,
    byte_mode: bool,
    state: ReadableState,
    error: Option<String>,
    cancel_reason: Option<String>,
    tee_group: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WritableState {
    Writable,
    Locked,
    Closed,
    Errored,
    Aborted,
}

#[derive(Debug, Clone)]
struct WritableRecord {
    chunks: Vec<Value>,
    state: WritableState,
    error: Option<String>,
    abort_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct TransformLink {
    readable_id: u64,
    transform: Value,
}

#[derive(Debug, Clone)]
struct TransferredReadable {
    chunks_json: Vec<String>,
    bytes: Vec<u8>,
    byte_mode: bool,
    state: ReadableState,
    error: Option<String>,
    cancel_reason: Option<String>,
}

thread_local! {
    static READABLES: RefCell<HashMap<u64, ReadableRecord>> = RefCell::new(HashMap::new());
    static WRITABLES: RefCell<HashMap<u64, WritableRecord>> = RefCell::new(HashMap::new());
    static STREAM_LOCKED: RefCell<HashMap<u64, bool>> = RefCell::new(HashMap::new());
    static READER_FOR_STREAM: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
    static WRITER_FOR_WRITABLE: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
    static TRANSFORM_LINKS: RefCell<HashMap<u64, TransformLink>> = RefCell::new(HashMap::new());
    static NEXT_TEE_GROUP: RefCell<u64> = const { RefCell::new(1) };
}

fn transfer_registry() -> &'static Mutex<HashMap<u64, TransferredReadable>> {
    static REG: OnceLock<Mutex<HashMap<u64, TransferredReadable>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

fn invoke_fn(env: &mut Environment, func: &Value, args: Vec<Value>) -> Result<Value, String> {
    crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env)
}

fn read_result(chunk: Value) -> Value {
    let mut out = HashMap::new();
    out.insert("done".into(), Value::Bool(false));
    out.insert("value".into(), chunk);
    Value::from_object(out)
}

fn done_result() -> Value {
    let mut out = HashMap::new();
    out.insert("done".into(), Value::Bool(true));
    out.insert("value".into(), Value::Undefined);
    Value::from_object(out)
}

fn state_name(state: ReadableState) -> &'static str {
    match state {
        ReadableState::Readable => "readable",
        ReadableState::Locked => "locked",
        ReadableState::Closed => "closed",
        ReadableState::Errored => "errored",
        ReadableState::Cancelled => "cancelled",
    }
}

fn writable_state_name(state: WritableState) -> &'static str {
    match state {
        WritableState::Writable => "writable",
        WritableState::Locked => "locked",
        WritableState::Closed => "closed",
        WritableState::Errored => "errored",
        WritableState::Aborted => "aborted",
    }
}

pub fn stream_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_stream".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::from_object(m)
}

pub fn writable_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_writable".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::from_object(m)
}

fn reader_object(id: u64, stream_id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_reader".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    m.insert("__kab_stream_id".into(), Value::Number(stream_id as i64));
    Value::from_object(m)
}

fn writer_object(id: u64, writable_id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_writer".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    m.insert("__kab_writable_id".into(), Value::Number(writable_id as i64));
    Value::from_object(m)
}

pub fn stream_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected stream".into());
    };
    if !matches!(o.get("__kab_stream"), Some(Value::Bool(true))) {
        return Err("expected stream".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid stream handle".into()),
    }
}

pub fn writable_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected writable stream".into());
    };
    if !matches!(o.get("__kab_writable"), Some(Value::Bool(true))) {
        return Err("expected writable stream".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid writable stream handle".into()),
    }
}

pub fn reader_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected stream reader".into());
    };
    if !matches!(o.get("__kab_reader"), Some(Value::Bool(true))) {
        return Err("expected stream reader".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid stream reader".into()),
    }
}

pub fn writer_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected stream writer".into());
    };
    if !matches!(o.get("__kab_writer"), Some(Value::Bool(true))) {
        return Err("expected stream writer".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid stream writer".into()),
    }
}

fn reader_stream_id(reader: u64) -> Result<u64, String> {
    READER_FOR_STREAM.with(|m| {
        m.borrow()
            .get(&reader)
            .copied()
            .ok_or_else(|| format!("invalid reader id {reader}"))
    })
}

pub fn reader_stream_id_pub(reader: u64) -> Result<u64, String> {
    reader_stream_id(reader)
}

fn writer_writable_id(writer: u64) -> Result<u64, String> {
    WRITER_FOR_WRITABLE.with(|m| {
        m.borrow()
            .get(&writer)
            .copied()
            .ok_or_else(|| format!("invalid writer id {writer}"))
    })
}

fn insert_readable(id: u64, record: ReadableRecord) {
    READABLES.with(|m| m.borrow_mut().insert(id, record));
}

fn insert_writable(id: u64, record: WritableRecord) {
    WRITABLES.with(|m| m.borrow_mut().insert(id, record));
}

pub fn stream_allocate() -> u64 {
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        id,
        ReadableRecord {
            chunks: Vec::new(),
            bytes: Vec::new(),
            byte_mode: false,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    id
}

pub fn stream_object_pub(id: u64) -> Value {
    stream_object(id)
}

pub fn stream_push(id: u64, chunk: Value) {
    READABLES.with(|m| {
        if let Some(r) = m.borrow_mut().get_mut(&id) {
            if r.byte_mode {
                if let Value::Number(n) = chunk {
                    if n >= 0 && n <= 255 {
                        r.bytes.push(n as u8);
                    }
                } else if let Value::String(s) = chunk {
                    r.bytes.extend(s.as_bytes());
                }
            } else {
                r.chunks.push(chunk);
            }
        }
    });
}

pub fn stream_push_capped(id: u64, chunk: Value, max: usize) {
    READABLES.with(|m| {
        if let Some(r) = m.borrow_mut().get_mut(&id) {
            if r.chunks.len() >= max {
                r.chunks.remove(0);
            }
            r.chunks.push(chunk);
        }
    });
}

pub fn stream_remove(id: u64) {
    READABLES.with(|m| {
        m.borrow_mut().remove(&id);
    });
    STREAM_LOCKED.with(|m| m.borrow_mut().remove(&id));
}

pub fn stream_len(id: u64) -> usize {
    READABLES.with(|m| {
        m.borrow()
            .get(&id)
            .map(|r| if r.byte_mode { r.bytes.len() } else { r.chunks.len() })
            .unwrap_or(0)
    })
}

pub fn stream_id_pub(v: &Value) -> Result<u64, String> {
    stream_id(v)
}

pub fn writable_id_pub(v: &Value) -> Result<u64, String> {
    writable_id(v)
}

pub fn stream_read_impl(id: u64) -> Result<Value, String> {
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        let Some(r) = map.get_mut(&id) else {
            return Err(format!("invalid stream id {id}"));
        };
        match r.state {
            ReadableState::Errored => {
                return Err(r.error.clone().unwrap_or_else(|| "stream errored".into()));
            }
            ReadableState::Cancelled => {
                return Err(r.cancel_reason.clone().unwrap_or_else(|| "stream cancelled".into()));
            }
            _ => {}
        }
        if r.byte_mode {
            if r.bytes.is_empty() {
                if matches!(r.state, ReadableState::Closed) {
                    return Ok(done_result());
                }
                return Ok(done_result());
            }
            let b = r.bytes.remove(0);
            return Ok(read_result(Value::Number(b as i64)));
        }
        if r.chunks.is_empty() {
            if matches!(r.state, ReadableState::Closed) {
                return Ok(done_result());
            }
            return Ok(done_result());
        }
        Ok(read_result(r.chunks.remove(0)))
    })
}

pub fn stream_read_all_impl(id: u64) -> Result<Value, String> {
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        let r = map.remove(&id).ok_or_else(|| format!("invalid stream id {id}"))?;
        if r.byte_mode {
            Ok(Value::from_array(
                r.bytes.into_iter().map(|b| Value::Number(b as i64)).collect(),
            ))
        } else {
            Ok(Value::from_array(r.chunks))
        }
    })
}

pub fn stream_pipe_to_impl(src_id: u64, dest_id: u64) -> Result<(), String> {
    let chunks = READABLES.with(|m| m.borrow_mut().remove(&src_id).map(|r| r.chunks).unwrap_or_default());
    for chunk in chunks {
        writable_write_impl(dest_id, chunk, None)?;
    }
    writable_close_impl(dest_id)?;
    Ok(())
}

fn writable_write_impl(id: u64, chunk: Value, env: Option<&mut Environment>) -> Result<(), String> {
    let transform = TRANSFORM_LINKS.with(|m| m.borrow().get(&id).cloned());
    if let Some(link) = transform {
        let out = if let Some(env) = env {
            invoke_fn(env, &link.transform, vec![chunk])?
        } else {
            chunk
        };
        if !matches!(out, Value::Null | Value::Undefined) {
            stream_push(link.readable_id, out);
        }
        return Ok(());
    }

    WRITABLES.with(|m| {
        let mut map = m.borrow_mut();
        let w = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid writable id {id}"))?;
        match w.state {
            WritableState::Closed | WritableState::Aborted | WritableState::Errored => {
                return Err(format!("writable stream is {}", writable_state_name(w.state)));
            }
            WritableState::Locked | WritableState::Writable => {}
        }
        if w.chunks.len() >= WRITABLE_HIGH_WATER {
            return Err("writable stream backpressure: buffer full".into());
        }
        w.chunks.push(chunk);
        Ok(())
    })
}

fn writable_close_impl(id: u64) -> Result<(), String> {
    WRITABLES.with(|m| {
        let mut map = m.borrow_mut();
        let w = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid writable id {id}"))?;
        w.state = WritableState::Closed;
        Ok(())
    })
}

pub fn from_array(items: Vec<Value>) -> Value {
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        id,
        ReadableRecord {
            chunks: items,
            bytes: Vec::new(),
            byte_mode: false,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    stream_object(id)
}

pub fn from_string(text: String) -> Value {
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    let chunks = if text.is_empty() {
        Vec::new()
    } else {
        vec![Value::String(text)]
    };
    insert_readable(
        id,
        ReadableRecord {
            chunks,
            bytes: Vec::new(),
            byte_mode: false,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    stream_object(id)
}

pub fn stream_new() -> Value {
    let id = stream_allocate();
    stream_object(id)
}

pub fn stream_cancel(id: u64) -> Result<(), String> {
    let tee_group = READABLES.with(|m| m.borrow().get(&id).and_then(|r| r.tee_group));
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        for (sid, r) in map.iter_mut() {
            if *sid == id || tee_group.is_some_and(|g| r.tee_group == Some(g)) {
                r.state = ReadableState::Cancelled;
                r.chunks.clear();
                r.bytes.clear();
            }
        }
    });
    Ok(())
}

pub fn stream_abort(id: u64, reason: Option<String>) -> Result<(), String> {
    let tee_group = READABLES.with(|m| m.borrow().get(&id).and_then(|r| r.tee_group));
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        for (sid, r) in map.iter_mut() {
            if *sid == id || tee_group.is_some_and(|g| r.tee_group == Some(g)) {
                r.state = ReadableState::Cancelled;
                r.cancel_reason = reason.clone();
                r.chunks.clear();
                r.bytes.clear();
            }
        }
    });
    Ok(())
}

pub fn stream_state(id: u64) -> Result<String, String> {
    READABLES.with(|m| {
        let map = m.borrow();
        let r = map.get(&id).ok_or_else(|| format!("invalid stream id {id}"))?;
        Ok(state_name(r.state).to_string())
    })
}

pub fn stream_enqueue(id: u64, chunk: Value) -> Result<(), String> {
    READABLES.with(|m| {
        let map = m.borrow();
        let r = map.get(&id).ok_or_else(|| format!("invalid stream id {id}"))?;
        if !matches!(r.state, ReadableState::Readable) {
            return Err(format!("stream is {}", state_name(r.state)));
        }
        Ok(())
    })?;
    stream_push(id, chunk);
    Ok(())
}

pub fn stream_close_readable(id: u64) -> Result<(), String> {
    READABLES.with(|m| {
        if let Some(r) = m.borrow_mut().get_mut(&id) {
            r.state = ReadableState::Closed;
        }
        Ok(())
    })
}

pub fn stream_tee(id: u64) -> Result<Value, String> {
    let record = READABLES.with(|m| {
        m.borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("invalid stream id {id}"))
    })?;
    let tee_group = NEXT_TEE_GROUP.with(|g| {
        let mut n = g.borrow_mut();
        let id = *n;
        *n += 1;
        id
    });
    let id_a = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    let id_b = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    let mut record_a = record.clone();
    let mut record_b = record;
    record_a.tee_group = Some(tee_group);
    record_b.tee_group = Some(tee_group);
    insert_readable(id_a, record_a);
    insert_readable(id_b, record_b);
    Ok(Value::from_array(vec![stream_object(id_a), stream_object(id_b)]))
}

pub fn stream_locked(id: u64) -> bool {
    STREAM_LOCKED.with(|m| *m.borrow().get(&id).unwrap_or(&false))
}

pub fn stream_lock(id: u64) -> Result<(), String> {
    let already = STREAM_LOCKED.with(|m| *m.borrow().get(&id).unwrap_or(&false));
    if already {
        return Err("stream is already locked".into());
    }
    STREAM_LOCKED.with(|m| m.borrow_mut().insert(id, true));
    READABLES.with(|m| {
        if let Some(r) = m.borrow_mut().get_mut(&id) {
            r.state = ReadableState::Locked;
        }
    });
    Ok(())
}

pub fn stream_unlock(id: u64) {
    STREAM_LOCKED.with(|m| m.borrow_mut().remove(&id));
    READABLES.with(|m| {
        if let Some(r) = m.borrow_mut().get_mut(&id) {
            if matches!(r.state, ReadableState::Locked) {
                r.state = ReadableState::Readable;
            }
        }
    });
}

pub fn stream_desired_size(id: u64) -> Result<i64, String> {
    let locked = STREAM_LOCKED.with(|m| *m.borrow().get(&id).unwrap_or(&false));
    if locked {
        return Ok(0);
    }
    let pending = stream_len(id) as i64;
    Ok(pending)
}

pub fn get_reader(stream: u64) -> Result<Value, String> {
    stream_lock(stream)?;
    let reader = NEXT_READER.fetch_add(1, Ordering::Relaxed);
    READER_FOR_STREAM.with(|m| m.borrow_mut().insert(reader, stream));
    Ok(reader_object(reader, stream))
}

pub fn reader_read(reader: u64) -> Result<Value, String> {
    let stream = reader_stream_id(reader)?;
    stream_read_impl(stream)
}

pub fn reader_release_lock(reader: u64) -> Result<(), String> {
    let stream = reader_stream_id(reader)?;
    READER_FOR_STREAM.with(|m| m.borrow_mut().remove(&reader));
    stream_unlock(stream);
    Ok(())
}

pub fn reader_cancel(reader: u64, reason: Option<String>) -> Result<(), String> {
    let stream = reader_stream_id(reader)?;
    stream_abort(stream, reason)?;
    READER_FOR_STREAM.with(|m| m.borrow_mut().remove(&reader));
    stream_unlock(stream);
    Ok(())
}

pub fn writable_stream_new() -> Value {
    let id = NEXT_WRITABLE.fetch_add(1, Ordering::Relaxed);
    insert_writable(
        id,
        WritableRecord {
            chunks: Vec::new(),
            state: WritableState::Writable,
            error: None,
            abort_reason: None,
        },
    );
    writable_object(id)
}

pub fn writable_write(id: u64, chunk: Value, env: &mut Environment) -> Result<(), String> {
    writable_write_impl(id, chunk, Some(env))
}

pub fn writable_close(id: u64) -> Result<(), String> {
    writable_close_impl(id)
}

pub fn writable_abort(id: u64, reason: Option<String>) -> Result<(), String> {
    WRITABLES.with(|m| {
        if let Some(w) = m.borrow_mut().get_mut(&id) {
            w.state = WritableState::Aborted;
            w.abort_reason = reason;
            w.chunks.clear();
        }
        Ok(())
    })
}

pub fn writable_read_all(id: u64) -> Result<Value, String> {
    WRITABLES.with(|m| {
        let chunks = m.borrow_mut().remove(&id).map(|w| w.chunks).unwrap_or_default();
        Ok(Value::from_array(chunks))
    })
}

pub fn writable_locked(id: u64) -> bool {
    WRITABLES.with(|m| {
        m.borrow()
            .get(&id)
            .map(|w| matches!(w.state, WritableState::Locked | WritableState::Closed | WritableState::Aborted))
            .unwrap_or(false)
    })
}

pub fn writable_desired_size(id: u64) -> i64 {
    WRITABLES.with(|m| {
        let map = m.borrow();
        let Some(w) = map.get(&id) else {
            return 0;
        };
        if !matches!(w.state, WritableState::Writable | WritableState::Locked) {
            return 0;
        }
        if w.chunks.len() >= WRITABLE_HIGH_WATER {
            0
        } else {
            (WRITABLE_HIGH_WATER - w.chunks.len()) as i64
        }
    })
}

pub fn get_writer(writable: u64) -> Result<Value, String> {
    WRITABLES.with(|m| -> Result<(), String> {
        let mut map = m.borrow_mut();
        let w = map
            .get_mut(&writable)
            .ok_or_else(|| format!("invalid writable id {writable}"))?;
        if matches!(w.state, WritableState::Locked) {
            return Err("writable stream is already locked".into());
        }
        w.state = WritableState::Locked;
        Ok(())
    })?;
    let writer = NEXT_WRITER.fetch_add(1, Ordering::Relaxed);
    WRITER_FOR_WRITABLE.with(|m| m.borrow_mut().insert(writer, writable));
    Ok(writer_object(writer, writable))
}

pub fn writer_write(writer: u64, chunk: Value, env: &mut Environment) -> Result<(), String> {
    let writable = writer_writable_id(writer)?;
    writable_write_impl(writable, chunk, Some(env))
}

pub fn writer_close(writer: u64) -> Result<(), String> {
    let writable = writer_writable_id(writer)?;
    writable_close_impl(writable)
}

pub fn writer_abort(writer: u64, reason: Option<String>) -> Result<(), String> {
    let writable = writer_writable_id(writer)?;
    writable_abort(writable, reason)
}

pub fn writer_release_lock(writer: u64) -> Result<(), String> {
    let writable = writer_writable_id(writer)?;
    WRITER_FOR_WRITABLE.with(|m| m.borrow_mut().remove(&writer));
    WRITABLES.with(|m| {
        if let Some(w) = m.borrow_mut().get_mut(&writable) {
            if matches!(w.state, WritableState::Locked) {
                w.state = WritableState::Writable;
            }
        }
    });
    Ok(())
}

pub fn transform_stream_new(transform: Value, env: &mut Environment) -> Result<Value, String> {
    let _ = env;
    let readable_id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        readable_id,
        ReadableRecord {
            chunks: Vec::new(),
            bytes: Vec::new(),
            byte_mode: false,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    let writable_id = NEXT_WRITABLE.fetch_add(1, Ordering::Relaxed);
    insert_writable(
        writable_id,
        WritableRecord {
            chunks: Vec::new(),
            state: WritableState::Writable,
            error: None,
            abort_reason: None,
        },
    );
    TRANSFORM_LINKS.with(|m| {
        m.borrow_mut().insert(
            writable_id,
            TransformLink {
                readable_id,
                transform,
            },
        );
    });
    let mut pair = HashMap::new();
    pair.insert("readable".into(), stream_object(readable_id));
    pair.insert("writable".into(), writable_object(writable_id));
    Ok(Value::from_object(pair))
}

pub fn byte_stream_new() -> Value {
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        id,
        ReadableRecord {
            chunks: Vec::new(),
            bytes: Vec::new(),
            byte_mode: true,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    stream_object(id)
}

pub fn byte_stream_from_bytes(data: &[u8]) -> Value {
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        id,
        ReadableRecord {
            chunks: Vec::new(),
            bytes: data.to_vec(),
            byte_mode: true,
            state: ReadableState::Readable,
            error: None,
            cancel_reason: None,
            tee_group: None,
        },
    );
    stream_object(id)
}

pub fn byte_stream_read(id: u64, max: usize) -> Result<Value, String> {
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        let r = map.get_mut(&id).ok_or_else(|| format!("invalid stream id {id}"))?;
        if !r.byte_mode {
            return Err("expected byte stream".into());
        }
        let n = max.min(r.bytes.len());
        let slice: Vec<Value> = r.bytes.drain(..n).map(|b| Value::Number(b as i64)).collect();
        Ok(Value::from_array(slice))
    })
}

pub fn byte_stream_byob_read(id: u64, buffer: &mut [i64]) -> Result<usize, String> {
    READABLES.with(|m| {
        let mut map = m.borrow_mut();
        let r = map.get_mut(&id).ok_or_else(|| format!("invalid stream id {id}"))?;
        if !r.byte_mode {
            return Err("expected byte stream".into());
        }
        let mut read = 0usize;
        for slot in buffer.iter_mut() {
            if r.bytes.is_empty() {
                break;
            }
            let b = r.bytes.remove(0);
            *slot = b as i64;
            read += 1;
        }
        Ok(read)
    })
}

pub fn stream_transfer(id: u64) -> Result<Value, String> {
    let record = READABLES.with(|m| {
        m.borrow_mut()
            .remove(&id)
            .ok_or_else(|| format!("invalid stream id {id}"))
    })?;
    STREAM_LOCKED.with(|m| m.borrow_mut().remove(&id));
    let chunks_json: Vec<String> = record
        .chunks
        .iter()
        .map(crate::runtime::stdlib::json::stringify)
        .collect();
    let token = NEXT_TRANSFER.fetch_add(1, Ordering::Relaxed);
    transfer_registry()
        .lock()
        .map_err(|_| "stream transfer registry lock poisoned".to_string())?
        .insert(
            token,
            TransferredReadable {
                chunks_json,
                bytes: record.bytes,
                byte_mode: record.byte_mode,
                state: record.state,
                error: record.error,
                cancel_reason: record.cancel_reason,
            },
        );
    let mut out = HashMap::new();
    out.insert("__kab_stream_transfer".into(), Value::Bool(true));
    out.insert("kabTransfer".into(), Value::String("stream".into()));
    out.insert("token".into(), Value::Number(token as i64));
    Ok(Value::from_object(out))
}

pub fn stream_from_transfer(token: u64) -> Result<Value, String> {
    let transferred = transfer_registry()
        .lock()
        .map_err(|_| "stream transfer registry lock poisoned".to_string())?
        .remove(&token)
        .ok_or_else(|| format!("invalid stream transfer token {token}"))?;
    let chunks: Vec<Value> = transferred
        .chunks_json
        .iter()
        .map(|s| crate::runtime::stdlib::json::parse(s))
        .collect::<Result<_, _>>()?;
    let id = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
    insert_readable(
        id,
        ReadableRecord {
            chunks,
            bytes: transferred.bytes,
            byte_mode: transferred.byte_mode,
            state: transferred.state,
            error: transferred.error,
            cancel_reason: transferred.cancel_reason,
            tee_group: None,
        },
    );
    Ok(stream_object(id))
}

fn transfer_token_id(map: &HashMap<String, Value>) -> Option<u64> {
    match map.get("token") {
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    }
}

fn is_stream_transfer_token(map: &HashMap<String, Value>) -> Option<u64> {
    let token = transfer_token_id(map)?;
    if matches!(map.get("__kab_stream_transfer"), Some(Value::Bool(true))) {
        return Some(token);
    }
    if matches!(map.get("kabTransfer"), Some(Value::String(s)) if s == "stream") {
        return Some(token);
    }
    None
}

fn is_sab_transfer_token(map: &HashMap<String, Value>) -> Option<u64> {
    let token = transfer_token_id(map)?;
    if matches!(map.get("__kab_sab_transfer"), Some(Value::Bool(true))) {
        return Some(token);
    }
    if matches!(map.get("kabTransfer"), Some(Value::String(s)) if s == "sab") {
        return Some(token);
    }
    None
}

pub fn adopt_transfers_in_message(msg: &Value) -> Result<Value, String> {
    match msg {
        Value::Object(map) => {
            if let Some(token) = is_stream_transfer_token(map) {
                return stream_from_transfer(token);
            }
            if let Some(token) = is_sab_transfer_token(map) {
                return crate::runtime::shared_memory::sab_from_transfer(token);
            }
            if let Some(Value::Array(items)) = map.get("transfers") {
                let mut adopted = Vec::new();
                for item in items.iter() {
                    if let Value::Object(t) = item {
                        if let Some(token) = is_stream_transfer_token(t) {
                            adopted.push(stream_from_transfer(token)?);
                            continue;
                        }
                        if let Some(token) = is_sab_transfer_token(t) {
                            adopted.push(
                                crate::runtime::shared_memory::sab_from_transfer(token)?,
                            );
                            continue;
                        }
                    }
                    adopted.push(item.clone());
                }
                let mut out = map.as_ref().clone();
                out.insert("transfers".into(), Value::from_array(adopted));
                return Ok(Value::from_object(out));
            }
            Ok(msg.clone())
        }
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items.iter() {
                out.push(adopt_transfers_in_message(item)?);
            }
            Ok(Value::from_array(out))
        }
        other => Ok(other.clone()),
    }
}

pub fn encode_transfer_list(items: &[Value]) -> Result<Vec<Value>, String> {
    items
        .iter()
        .map(|item| {
            if crate::runtime::shared_memory::sab_id(item).is_ok() {
                crate::runtime::shared_memory::sab_transfer(crate::runtime::shared_memory::sab_id(
                    item,
                )?)
            } else {
                stream_transfer(stream_id(item)?)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_reads_chunks() {
        let s = from_array(vec![Value::Number(1), Value::Number(2)]);
        let id = stream_id(&s).unwrap();
        let reader = get_reader(id).unwrap();
        let rid = reader_id(&reader).unwrap();
        let c1 = reader_read(rid).unwrap();
        let Value::Object(m1) = c1 else { panic!() };
        assert!(matches!(m1.get("value"), Some(Value::Number(1))));
        reader_release_lock(rid).unwrap();
    }

    #[test]
    fn byte_stream_read_drains() {
        let s = byte_stream_from_bytes(&[10, 20, 30]);
        let id = stream_id(&s).unwrap();
        let out = byte_stream_read(id, 2).unwrap();
        let Value::Array(a) = out else {
            panic!("expected array");
        };
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn transfer_roundtrip() {
        let s = from_array(vec![Value::Number(5)]);
        let id = stream_id(&s).unwrap();
        let token = stream_transfer(id).unwrap();
        let Value::Object(t) = token else { panic!() };
        let tok = match t.get("token") {
            Some(Value::Number(n)) => *n as u64,
            _ => panic!(),
        };
        let s2 = stream_from_transfer(tok).unwrap();
        let id2 = stream_id(&s2).unwrap();
        let chunk = stream_read_impl(id2).unwrap();
        let Value::Object(ch) = chunk else { panic!() };
        assert!(matches!(ch.get("value"), Some(Value::Number(5))));
    }
}
