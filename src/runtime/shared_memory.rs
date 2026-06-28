//! SharedArrayBuffer + Atomics + transferable shared memory (Deno våg 18).

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};

static NEXT_SAB: AtomicU64 = AtomicU64::new(1);
static NEXT_SAB_TRANSFER: AtomicU64 = AtomicU64::new(1);

struct SharedBlock {
    bytes: Box<[u8]>,
}

thread_local! {
    static LOCAL_SABS: RefCell<HashMap<u64, std::sync::Arc<SharedBlock>>> =
        RefCell::new(HashMap::new());
}

fn sab_transfer_registry() -> &'static Mutex<HashMap<u64, std::sync::Arc<SharedBlock>>> {
    static REG: OnceLock<Mutex<HashMap<u64, std::sync::Arc<SharedBlock>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

fn wait_registry() -> &'static Mutex<HashMap<(u64, usize), (Mutex<()>, Condvar)>> {
    static REG: OnceLock<Mutex<HashMap<(u64, usize), (Mutex<()>, Condvar)>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sab_object(id: u64, byte_length: usize) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_sab".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    m.insert("byteLength".into(), Value::Number(byte_length as i64));
    Value::Object(m)
}

pub fn sab_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected SharedArrayBuffer".into());
    };
    if !matches!(o.get("__kab_sab"), Some(Value::Bool(true))) {
        return Err("expected SharedArrayBuffer".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid SharedArrayBuffer handle".into()),
    }
}

fn block_for_sab(id: u64) -> Result<std::sync::Arc<SharedBlock>, String> {
    LOCAL_SABS.with(|m| {
        m.borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("invalid SharedArrayBuffer id {id}"))
    })
}

fn usize_arg(v: &Value, name: &str) -> Result<usize, String> {
    match v {
        Value::Number(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(format!("{name} expects non-negative number")),
    }
}

fn atomic_i32_at(block: &SharedBlock, byte_offset: usize) -> Result<&AtomicI32, String> {
    if byte_offset % 4 != 0 {
        return Err("Int32Array offset must be 4-byte aligned".into());
    }
    if byte_offset.saturating_add(4) > block.bytes.len() {
        return Err("Int32Array index out of range".into());
    }
    Ok(unsafe { &*(block.bytes.as_ptr().add(byte_offset) as *const AtomicI32) })
}

pub fn is_uint8_array(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if matches!(o.get("__kab_u8"), Some(Value::Bool(true)))
    )
}

pub fn uint8_array_byte_length(v: &Value) -> Result<usize, String> {
    let (_, _, length) = u8_view_parts(v)?;
    Ok(length)
}

pub fn fill_uint8_array(view: &Value, bytes: &[u8]) -> Result<(), String> {
    let (sab_id, offset, length) = u8_view_parts(view)?;
    if bytes.len() != length {
        return Err(format!(
            "fill_uint8_array: expected {} bytes, got {}",
            length,
            bytes.len()
        ));
    }
    let block = block_for_sab(sab_id)?;
    let ptr = block.bytes.as_ptr() as *mut u8;
    // SAFETY: single-byte writes within the view range; Arc shared across threads.
    unsafe {
        for (i, b) in bytes.iter().enumerate() {
            *ptr.add(offset + i) = *b;
        }
    }
    Ok(())
}

fn u8_view_parts(v: &Value) -> Result<(u64, usize, usize), String> {
    let Value::Object(o) = v else {
        return Err("expected Uint8Array".into());
    };
    if !matches!(o.get("__kab_u8"), Some(Value::Bool(true))) {
        return Err("expected Uint8Array".into());
    }
    let sab_id = match o.get("__kab_sab_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid Uint8Array".into()),
    };
    let offset = match o.get("byteOffset") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let length = match o.get("byteLength") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("invalid Uint8Array byteLength".into()),
    };
    Ok((sab_id, offset, length))
}

fn i32_view_parts(v: &Value) -> Result<(u64, usize, usize), String> {
    let Value::Object(o) = v else {
        return Err("expected Int32Array".into());
    };
    if !matches!(o.get("__kab_i32"), Some(Value::Bool(true))) {
        return Err("expected Int32Array".into());
    }
    let sab_id = match o.get("__kab_sab_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid Int32Array".into()),
    };
    let offset = match o.get("byteOffset") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let length = match o.get("length") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("invalid Int32Array length".into()),
    };
    Ok((sab_id, offset, length))
}

fn i32_index_byte_offset(view: &Value, index: usize) -> Result<(u64, usize), String> {
    let (sab_id, byte_offset, length) = i32_view_parts(view)?;
    if index >= length {
        return Err("Int32Array index out of range".into());
    }
    Ok((sab_id, byte_offset + index * 4))
}

pub fn sab_new(byte_length: usize) -> Result<Value, String> {
    if byte_length == 0 {
        return Err("SharedArrayBuffer byteLength must be > 0".into());
    }
    let padded = (byte_length + 3) & !3;
    let bytes = vec![0u8; padded].into_boxed_slice();
    let block = std::sync::Arc::new(SharedBlock { bytes });
    let id = NEXT_SAB.fetch_add(1, Ordering::Relaxed);
    LOCAL_SABS.with(|m| m.borrow_mut().insert(id, block));
    Ok(sab_object(id, padded))
}

pub fn sab_byte_length(sab: &Value) -> Result<usize, String> {
    let id = sab_id(sab)?;
    Ok(block_for_sab(id)?.bytes.len())
}

pub fn sab_transfer(sab_id: u64) -> Result<Value, String> {
    let block = LOCAL_SABS.with(|m| {
        m.borrow_mut()
            .remove(&sab_id)
            .ok_or_else(|| format!("invalid SharedArrayBuffer id {sab_id}"))
    })?;
    let token = NEXT_SAB_TRANSFER.fetch_add(1, Ordering::Relaxed);
    sab_transfer_registry()
        .lock()
        .map_err(|_| "sab transfer registry lock poisoned".to_string())?
        .insert(token, block);
    let mut out = HashMap::new();
    out.insert("__kab_sab_transfer".into(), Value::Bool(true));
    out.insert("kabTransfer".into(), Value::String("sab".into()));
    out.insert("token".into(), Value::Number(token as i64));
    Ok(Value::Object(out))
}

pub fn sab_from_transfer(token: u64) -> Result<Value, String> {
    let block = sab_transfer_registry()
        .lock()
        .map_err(|_| "sab transfer registry lock poisoned".to_string())?
        .remove(&token)
        .ok_or_else(|| format!("invalid SharedArrayBuffer transfer token {token}"))?;
    let id = NEXT_SAB.fetch_add(1, Ordering::Relaxed);
    let byte_length = block.bytes.len();
    LOCAL_SABS.with(|m| m.borrow_mut().insert(id, block));
    Ok(sab_object(id, byte_length))
}


fn sab_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let size = usize_arg(args.first().ok_or("sab_new(byteLength)")?, "sab_new")?;
    sab_new(size)
}

fn sab_byte_length_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sab = args.first().ok_or("sab_byte_length(sab)")?;
    Ok(Value::Number(sab_byte_length(sab)? as i64))
}

fn sab_transfer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = sab_id(args.first().ok_or("sab_transfer(sab)")?)?;
    sab_transfer(id)
}

fn sab_from_transfer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let token = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("sab_from_transfer(token)".into()),
    };
    sab_from_transfer(token)
}

fn sab_is_shared_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Bool(sab_id(args.first().ok_or("sab_is_shared(sab)")?).is_ok()))
}

fn uint8_array_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sab = args.first().ok_or("uint8_array_new(sab, offset?, length?)")?;
    let sab_id = sab_id(sab)?;
    let total = block_for_sab(sab_id)?.bytes.len();
    let offset = match args.get(1) {
        Some(v) => usize_arg(v, "uint8_array_new offset")?,
        None => 0,
    };
    let length = match args.get(2) {
        Some(v) => usize_arg(v, "uint8_array_new length")?,
        None => total.saturating_sub(offset),
    };
    if offset.saturating_add(length) > total {
        return Err("Uint8Array range exceeds SharedArrayBuffer".into());
    }
    let mut m = HashMap::new();
    m.insert("__kab_u8".into(), Value::Bool(true));
    m.insert("__kab_sab_id".into(), Value::Number(sab_id as i64));
    m.insert("byteOffset".into(), Value::Number(offset as i64));
    m.insert("byteLength".into(), Value::Number(length as i64));
    Ok(Value::Object(m))
}

fn uint8_array_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("uint8_array_get(view, index)")?;
    let index = usize_arg(args.get(1).ok_or("uint8_array_get(view, index)")?, "index")?;
    let (sab_id, offset, length) = u8_view_parts(view)?;
    if index >= length {
        return Err("Uint8Array index out of range".into());
    }
    let block = block_for_sab(sab_id)?;
    Ok(Value::Number(block.bytes[offset + index] as i64))
}

fn uint8_array_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("uint8_array_set(view, index, value)")?;
    let index = usize_arg(
        args.get(1).ok_or("uint8_array_set(view, index, value)")?,
        "index",
    )?;
    let value = match args.get(2) {
        Some(Value::Number(n)) if (0..=255).contains(n) => *n as u8,
        _ => return Err("uint8_array_set value must be 0..255".into()),
    };
    let (sab_id, offset, length) = u8_view_parts(view)?;
    if index >= length {
        return Err("Uint8Array index out of range".into());
    }
    let block = block_for_sab(sab_id)?;
    // SAFETY: single-byte write; Arc shared across threads.
    let ptr = block.bytes.as_ptr() as *mut u8;
    unsafe {
        *ptr.add(offset + index) = value;
    }
    Ok(Value::Undefined)
}

fn int32_array_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sab = args.first().ok_or("int32_array_new(sab, offset?, length?)")?;
    let sab_id = sab_id(sab)?;
    let total = block_for_sab(sab_id)?.bytes.len();
    let offset = match args.get(1) {
        Some(v) => usize_arg(v, "int32_array_new offset")?,
        None => 0,
    };
    if offset % 4 != 0 {
        return Err("Int32Array byteOffset must be 4-byte aligned".into());
    }
    let length = match args.get(2) {
        Some(v) => usize_arg(v, "int32_array_new length")?,
        None => (total.saturating_sub(offset)) / 4,
    };
    if offset.saturating_add(length.saturating_mul(4)) > total {
        return Err("Int32Array range exceeds SharedArrayBuffer".into());
    }
    let mut m = HashMap::new();
    m.insert("__kab_i32".into(), Value::Bool(true));
    m.insert("__kab_sab_id".into(), Value::Number(sab_id as i64));
    m.insert("byteOffset".into(), Value::Number(offset as i64));
    m.insert("length".into(), Value::Number(length as i64));
    Ok(Value::Object(m))
}

fn int32_array_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("int32_array_get(view, index)")?;
    let index = usize_arg(args.get(1).ok_or("int32_array_get(view, index)")?, "index")?;
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    Ok(Value::Number(atomic.load(Ordering::SeqCst) as i64))
}

fn int32_array_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("int32_array_set(view, index, value)")?;
    let index = usize_arg(
        args.get(1).ok_or("int32_array_set(view, index, value)")?,
        "index",
    )?;
    let value = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("int32_array_set value must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    atomic.store(value, Ordering::SeqCst);
    Ok(Value::Undefined)
}

fn f64_view_parts(v: &Value) -> Result<(u64, usize, usize), String> {
    let Value::Object(o) = v else {
        return Err("expected Float64Array".into());
    };
    if !matches!(o.get("__kab_f64"), Some(Value::Bool(true))) {
        return Err("expected Float64Array".into());
    }
    let sab_id = match o.get("__kab_sab_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid Float64Array".into()),
    };
    let offset = match o.get("byteOffset") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let length = match o.get("length") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("invalid Float64Array length".into()),
    };
    Ok((sab_id, offset, length))
}

fn f64_index_byte_offset(view: &Value, index: usize) -> Result<(u64, usize), String> {
    let (sab_id, byte_offset, length) = f64_view_parts(view)?;
    if index >= length {
        return Err("Float64Array index out of range".into());
    }
    Ok((sab_id, byte_offset + index * 8))
}

fn read_f64_le(bytes: &[u8], offset: usize) -> Result<f64, String> {
    if offset.saturating_add(8) > bytes.len() {
        return Err("DataView read out of range".into());
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[offset..offset + 8]);
    Ok(f64::from_le_bytes(arr))
}

fn write_f64_le(bytes: &mut [u8], offset: usize, value: f64) -> Result<(), String> {
    if offset.saturating_add(8) > bytes.len() {
        return Err("DataView write out of range".into());
    }
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn array_buffer_new_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    sab_new_native(args, env)
}

fn float64_array_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sab = args.first().ok_or("float64_array_new(buffer, offset?, length?)")?;
    let sab_id = sab_id(sab)?;
    let total = block_for_sab(sab_id)?.bytes.len();
    let offset = match args.get(1) {
        Some(v) => usize_arg(v, "float64_array_new offset")?,
        None => 0,
    };
    if offset % 8 != 0 {
        return Err("Float64Array byteOffset must be 8-byte aligned".into());
    }
    let length = match args.get(2) {
        Some(v) => usize_arg(v, "float64_array_new length")?,
        None => (total.saturating_sub(offset)) / 8,
    };
    if offset.saturating_add(length.saturating_mul(8)) > total {
        return Err("Float64Array range exceeds ArrayBuffer".into());
    }
    let mut m = HashMap::new();
    m.insert("__kab_f64".into(), Value::Bool(true));
    m.insert("__kab_sab_id".into(), Value::Number(sab_id as i64));
    m.insert("byteOffset".into(), Value::Number(offset as i64));
    m.insert("length".into(), Value::Number(length as i64));
    m.insert("BYTES_PER_ELEMENT".into(), Value::Number(8));
    Ok(Value::Object(m))
}

fn float64_array_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("float64_array_get(view, index)")?;
    let index = usize_arg(args.get(1).ok_or("float64_array_get(view, index)")?, "index")?;
    let (sab_id, byte_offset) = f64_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    Ok(Value::Float(read_f64_le(&block.bytes, byte_offset)?))
}

fn float64_array_set_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("float64_array_set(view, index, value)")?;
    let index = usize_arg(
        args.get(1).ok_or("float64_array_set(view, index, value)")?,
        "index",
    )?;
    let value = match args.get(2) {
        Some(Value::Float(f)) => *f,
        Some(Value::Number(n)) => *n as f64,
        _ => return Err("float64_array_set value must be number".into()),
    };
    let (sab_id, byte_offset) = f64_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let ptr = block.bytes.as_ptr() as *mut u8;
    unsafe {
        let slice = std::slice::from_raw_parts_mut(ptr, block.bytes.len());
        write_f64_le(slice, byte_offset, value)?;
    }
    Ok(Value::Undefined)
}

fn data_view_parts(v: &Value) -> Result<(u64, usize, usize), String> {
    let Value::Object(o) = v else {
        return Err("expected DataView".into());
    };
    if !matches!(o.get("__kab_dv"), Some(Value::Bool(true))) {
        return Err("expected DataView".into());
    }
    let sab_id = match o.get("__kab_sab_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid DataView".into()),
    };
    let offset = match o.get("byteOffset") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let length = match o.get("byteLength") {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("invalid DataView byteLength".into()),
    };
    Ok((sab_id, offset, length))
}

fn data_view_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sab = args.first().ok_or("data_view_new(buffer, offset?, byteLength?)")?;
    let sab_id = sab_id(sab)?;
    let total = block_for_sab(sab_id)?.bytes.len();
    let offset = match args.get(1) {
        Some(v) => usize_arg(v, "data_view_new offset")?,
        None => 0,
    };
    let length = match args.get(2) {
        Some(v) => usize_arg(v, "data_view_new byteLength")?,
        None => total.saturating_sub(offset),
    };
    if offset.saturating_add(length) > total {
        return Err("DataView range exceeds ArrayBuffer".into());
    }
    let mut m = HashMap::new();
    m.insert("__kab_dv".into(), Value::Bool(true));
    m.insert("__kab_sab_id".into(), Value::Number(sab_id as i64));
    m.insert("byteOffset".into(), Value::Number(offset as i64));
    m.insert("byteLength".into(), Value::Number(length as i64));
    Ok(Value::Object(m))
}

fn data_view_get_float64_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("data_view_get_float64(view, byteOffset)")?;
    let (sab_id, base, length) = data_view_parts(view)?;
    let rel = usize_arg(
        args.get(1).ok_or("data_view_get_float64(view, byteOffset)")?,
        "byteOffset",
    )?;
    if rel.saturating_add(8) > length {
        return Err("DataView read out of range".into());
    }
    let block = block_for_sab(sab_id)?;
    Ok(Value::Float(read_f64_le(&block.bytes, base + rel)?))
}

fn data_view_set_float64_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("data_view_set_float64(view, byteOffset, value)")?;
    let (sab_id, base, length) = data_view_parts(view)?;
    let rel = usize_arg(
        args.get(1)
            .ok_or("data_view_set_float64(view, byteOffset, value)")?,
        "byteOffset",
    )?;
    let value = match args.get(2) {
        Some(Value::Float(f)) => *f,
        Some(Value::Number(n)) => *n as f64,
        _ => return Err("data_view_set_float64 value must be number".into()),
    };
    if rel.saturating_add(8) > length {
        return Err("DataView write out of range".into());
    }
    let block = block_for_sab(sab_id)?;
    let ptr = block.bytes.as_ptr() as *mut u8;
    unsafe {
        let slice = std::slice::from_raw_parts_mut(ptr, block.bytes.len());
        write_f64_le(slice, base + rel, value)?;
    }
    Ok(Value::Undefined)
}

pub fn is_float64_array(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if matches!(o.get("__kab_f64"), Some(Value::Bool(true)))
    )
}

pub fn is_data_view(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(o) if matches!(o.get("__kab_dv"), Some(Value::Bool(true)))
    )
}

fn atomics_load_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    int32_array_get_native(args, _env)
}

fn atomics_store_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_store(view, index, value)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_store(view, index, value)")?,
        "index",
    )?;
    let value = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_store value must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    atomic.store(value, Ordering::SeqCst);
    Ok(Value::Number(value as i64))
}

fn atomics_add_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_add(view, index, delta)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_add(view, index, delta)")?,
        "index",
    )?;
    let delta = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_add delta must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    Ok(Value::Number(
        atomic.fetch_add(delta, Ordering::SeqCst) as i64,
    ))
}

fn atomics_sub_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_sub(view, index, delta)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_sub(view, index, delta)")?,
        "index",
    )?;
    let delta = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_sub delta must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    Ok(Value::Number(
        atomic.fetch_sub(delta, Ordering::SeqCst) as i64,
    ))
}

fn atomics_bitwise_native(
    args: &[Value],
    op: fn(&AtomicI32, i32, Ordering) -> i32,
    name: &str,
) -> Result<Value, String> {
    let view = args.first().ok_or(name)?;
    let index = usize_arg(args.get(1).ok_or(name)?, "index")?;
    let value = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err(format!("{name} value must be number")),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    Ok(Value::Number(op(atomic, value, Ordering::SeqCst) as i64))
}

fn atomics_and_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    atomics_bitwise_native(args, AtomicI32::fetch_and, "atomics_and(view, index, value)")
}

fn atomics_or_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    atomics_bitwise_native(args, AtomicI32::fetch_or, "atomics_or(view, index, value)")
}

fn atomics_xor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    atomics_bitwise_native(args, AtomicI32::fetch_xor, "atomics_xor(view, index, value)")
}

fn atomics_exchange_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_exchange(view, index, value)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_exchange(view, index, value)")?,
        "index",
    )?;
    let value = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_exchange value must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    Ok(Value::Number(
        atomic.swap(value, Ordering::SeqCst) as i64,
    ))
}

fn atomics_compare_exchange_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_compare_exchange(view, index, expected, replacement)")?;
    let index = usize_arg(
        args.get(1)
            .ok_or("atomics_compare_exchange(view, index, expected, replacement)")?,
        "index",
    )?;
    let expected = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_compare_exchange expected must be number".into()),
    };
    let replacement = match args.get(3) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_compare_exchange replacement must be number".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    let ok = atomic
        .compare_exchange(expected, replacement, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    Ok(Value::Bool(ok))
}

fn atomics_wait_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_wait(view, index, value, timeout_ms?)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_wait(view, index, value, timeout_ms?)")?,
        "index",
    )?;
    let expected = match args.get(2) {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("atomics_wait value must be number".into()),
    };
    let timeout_ms = match args.get(3) {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        None => 0,
        _ => return Err("atomics_wait timeout_ms must be non-negative".into()),
    };
    let (sab_id, byte_offset) = i32_index_byte_offset(view, index)?;
    let block = block_for_sab(sab_id)?;
    let atomic = atomic_i32_at(&block, byte_offset)?;
    if atomic.load(Ordering::SeqCst) != expected {
        return Ok(Value::String("not-equal".into()));
    }
    if timeout_ms == 0 {
        return Ok(Value::String("timed-out".into()));
    }
    let mut reg = wait_registry().lock().map_err(|_| "wait registry lock poisoned")?;
    let (lock, cvar) = reg
        .entry((sab_id, index))
        .or_insert_with(|| (Mutex::new(()), Condvar::new()));
    let guard = lock.lock().map_err(|_| "wait mutex poisoned")?;
    let (guard, timeout) = cvar
        .wait_timeout(guard, std::time::Duration::from_millis(timeout_ms))
        .map_err(|_| "wait mutex poisoned")?;
    drop(guard);
    drop(reg);
    if atomic.load(Ordering::SeqCst) != expected {
        Ok(Value::String("ok".into()))
    } else if timeout.timed_out() {
        Ok(Value::String("timed-out".into()))
    } else {
        Ok(Value::String("ok".into()))
    }
}

fn atomics_notify_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("atomics_notify(view, index, count?)")?;
    let index = usize_arg(
        args.get(1).ok_or("atomics_notify(view, index, count?)")?,
        "index",
    )?;
    let count = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        None => usize::MAX,
        _ => return Err("atomics_notify count must be non-negative".into()),
    };
    let (sab_id, _) = i32_index_byte_offset(view, index)?;
    let reg = wait_registry().lock().map_err(|_| "wait registry lock poisoned")?;
    if let Some((_, cvar)) = reg.get(&(sab_id, index)) {
        let notified = if count == usize::MAX {
            cvar.notify_all();
            1
        } else {
            cvar.notify_all();
            count.min(1)
        };
        Ok(Value::Number(notified as i64))
    } else {
        Ok(Value::Number(0))
    }
}

pub fn register_shared_memory(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("sab_new", sab_new_native),
        ("shared_array_buffer_new", sab_new_native),
        ("sab_byte_length", sab_byte_length_native),
        ("sab_transfer", sab_transfer_native),
        ("sab_from_transfer", sab_from_transfer_native),
        ("sab_is_shared", sab_is_shared_native),
        ("uint8_array_new", uint8_array_new_native),
        ("uint8_array_get", uint8_array_get_native),
        ("uint8_array_set", uint8_array_set_native),
        ("int32_array_new", int32_array_new_native),
        ("int32_array_get", int32_array_get_native),
        ("int32_array_set", int32_array_set_native),
        ("array_buffer_new", array_buffer_new_native),
        ("float64_array_new", float64_array_new_native),
        ("float64_array_get", float64_array_get_native),
        ("float64_array_set", float64_array_set_native),
        ("data_view_new", data_view_new_native),
        ("data_view_get_float64", data_view_get_float64_native),
        ("data_view_set_float64", data_view_set_float64_native),
        ("atomics_load", atomics_load_native),
        ("atomics_store", atomics_store_native),
        ("atomics_add", atomics_add_native),
        ("atomics_sub", atomics_sub_native),
        ("atomics_and", atomics_and_native),
        ("atomics_or", atomics_or_native),
        ("atomics_xor", atomics_xor_native),
        ("atomics_exchange", atomics_exchange_native),
        ("atomics_compare_exchange", atomics_compare_exchange_native),
        ("atomics_wait", atomics_wait_native),
        ("atomics_notify", atomics_notify_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sab_roundtrip_transfer() {
        let sab = sab_new(8).unwrap();
        let id = sab_id(&sab).unwrap();
        let token = sab_transfer(id).unwrap();
        let Value::Object(t) = token else {
            panic!("expected token");
        };
        let tok = match t.get("token") {
            Some(Value::Number(n)) => *n as u64,
            _ => panic!("missing token"),
        };
        let sab2 = sab_from_transfer(tok).unwrap();
        assert_eq!(sab_byte_length(&sab2).unwrap(), 8);
    }

    #[test]
    fn sab_adopts_after_json_worker_wire() {
        let sab = sab_new(4).unwrap();
        let id = sab_id(&sab).unwrap();
        let token = sab_transfer(id).unwrap();
        let json = crate::runtime::stdlib::json::stringify(&token);
        assert!(json.contains("kabTransfer"));
        let parsed = crate::runtime::stdlib::json::parse(&json).unwrap();
        let mut msg = HashMap::new();
        msg.insert("transfers".into(), Value::Array(vec![parsed]));
        let adopted =
            crate::runtime::web_streams::adopt_transfers_in_message(&Value::Object(msg)).unwrap();
        let Value::Object(map) = adopted else {
            panic!("expected object");
        };
        let transfers = map.get("transfers").expect("transfers");
        let Value::Array(items) = transfers else {
            panic!("expected array");
        };
        assert!(sab_id(&items[0]).is_ok());
    }
}
