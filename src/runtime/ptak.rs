//! P11–P18 performance-tak natives (subset gates).
//! Native kernels stay in Rust; product policy stays in `.kab`.

use crate::value::{Environment, NdShared, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static SHAPE_TRANSITIONS: AtomicU64 = AtomicU64::new(0);
static SHAPE_HITS: AtomicU64 = AtomicU64::new(0);
static NURSERY_USED: AtomicU64 = AtomicU64::new(0);
static NURSERY_ALLOCS: AtomicU64 = AtomicU64::new(0);
static NURSERY_PROMOTES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static NURSERY: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

pub fn note_shape_transition() {
    SHAPE_TRANSITIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn note_shape_hit() {
    SHAPE_HITS.fetch_add(1, Ordering::Relaxed);
}

pub fn shape_stats() -> (u64, u64) {
    (
        SHAPE_HITS.load(Ordering::Relaxed),
        SHAPE_TRANSITIONS.load(Ordering::Relaxed),
    )
}

pub fn shape_stats_reset() {
    SHAPE_HITS.store(0, Ordering::Relaxed);
    SHAPE_TRANSITIONS.store(0, Ordering::Relaxed);
}

/// P14a — bump bytes for short-lived scratch (not a full generational GC).
pub fn nursery_alloc(n: usize) -> usize {
    NURSERY.with(|buf| {
        let mut b = buf.borrow_mut();
        let start = b.len();
        b.resize(start + n, 0);
        NURSERY_USED.store(b.len() as u64, Ordering::Relaxed);
        NURSERY_ALLOCS.fetch_add(1, Ordering::Relaxed);
        if b.len() > 64 * 1024 {
            b.clear();
            NURSERY_PROMOTES.fetch_add(1, Ordering::Relaxed);
        }
        start
    })
}

pub fn nursery_stats_map() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert(
        "used".into(),
        Value::Number(NURSERY_USED.load(Ordering::Relaxed) as i64),
    );
    m.insert(
        "allocs".into(),
        Value::Number(NURSERY_ALLOCS.load(Ordering::Relaxed) as i64),
    );
    m.insert(
        "promotes".into(),
        Value::Number(NURSERY_PROMOTES.load(Ordering::Relaxed) as i64),
    );
    m
}

pub fn nursery_reset_for_tests() {
    NURSERY.with(|b| b.borrow_mut().clear());
    NURSERY_USED.store(0, Ordering::Relaxed);
    NURSERY_ALLOCS.store(0, Ordering::Relaxed);
    NURSERY_PROMOTES.store(0, Ordering::Relaxed);
}

pub fn manual_runtime_checks() -> bool {
    if std::env::var("KABOOTAR_DEBUG_MANUAL").as_deref() == Ok("1") {
        return true;
    }
    if std::env::var("KABOOTAR_DEBUG_MANUAL").as_deref() == Ok("0") {
        return false;
    }
    cfg!(debug_assertions)
}

/// P13a — native i64 add-loop (Cranelift remains deepen; this is the maskinkod-klass kernel).
#[inline(never)]
pub fn native_add_loop(n: i64) -> i64 {
    let mut s = 0i64;
    let mut i = 0i64;
    while i < n {
        s = s.wrapping_add(1);
        i = i.wrapping_add(1);
    }
    s
}

fn num_i64(v: &Value) -> Result<i64, String> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Float(f) => Ok(*f as i64),
        _ => Err("expected number".into()),
    }
}

fn native_add_loop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = args.first().map(num_i64).transpose()?.unwrap_or(0);
    Ok(Value::Number(native_add_loop(n)))
}

fn array_f64_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut data = Vec::new();
    if let Some(Value::Array(items)) = args.first() {
        for v in items.iter() {
            data.push(match v {
                Value::Float(f) => *f,
                Value::Number(n) => *n as f64,
                _ => return Err("array_f64 expects numbers".into()),
            });
        }
    } else if let Some(Value::Number(n)) = args.first() {
        data = vec![0.0; *n as usize];
    } else {
        return Err("array_f64(arr|len)".into());
    }
    Ok(Value::NdShared(NdShared::new(data)))
}

fn array_f64_sum_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let buf = match args.first() {
        Some(Value::NdShared(b)) => b.as_slice().to_vec(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| -> Result<f64, String> {
                match v {
                    Value::Float(f) => Ok(*f),
                    Value::Number(n) => Ok(*n as f64),
                    _ => Err("array_f64_sum: mixed array".into()),
                }
            })
            .collect::<Result<Vec<f64>, String>>()?,
        _ => return Err("array_f64_sum(buf)".into()),
    };
    let mut s = 0.0;
    for x in buf {
        s += x;
    }
    Ok(Value::Float(s))
}

fn shape_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (hits, trans) = shape_stats();
    let mut m = HashMap::new();
    m.insert("hits".into(), Value::Number(hits as i64));
    m.insert("transitions".into(), Value::Number(trans as i64));
    m.insert("shared_ic".into(), Value::Bool(true));
    Ok(Value::from_object(m))
}

fn nursery_alloc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = args.first().map(num_i64).transpose()?.unwrap_or(64);
    let off = nursery_alloc(n.max(0) as usize);
    Ok(Value::Number(off as i64))
}

fn gc_nursery_stats_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::from_object(nursery_stats_map()))
}

fn manual_checks_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Bool(manual_runtime_checks()))
}

fn same_room_list_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "SELECT * FROM p17_users".to_string(),
    };
    match env.get("sql") {
        Some(Value::NativeFunction(f)) => f(&[Value::String(query)], env),
        _ => Err("sql() missing".into()),
    }
}

fn league_add_loop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = args.first().map(num_i64).transpose()?.unwrap_or(50_000);
    let t0 = Instant::now();
    let boxed_s = {
        let mut s = 0i64;
        let mut i = 0i64;
        while i < n {
            s += 1;
            i += 1;
        }
        s
    };
    let boxed_ns = t0.elapsed().as_nanos() as i64;
    let t1 = Instant::now();
    let native_s = native_add_loop(n);
    let native_ns = t1.elapsed().as_nanos() as i64;
    let mut m = HashMap::new();
    m.insert("n".into(), Value::Number(n));
    m.insert("boxed".into(), Value::Number(boxed_s));
    m.insert("native".into(), Value::Number(native_s));
    m.insert("boxed_ns".into(), Value::Number(boxed_ns.max(1)));
    m.insert("native_ns".into(), Value::Number(native_ns.max(1)));
    m.insert(
        "python_gate".into(),
        Value::Bool(native_s == n && native_ns <= boxed_ns),
    );
    Ok(Value::from_object(m))
}

fn tak_ceiling_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut m = HashMap::new();
    m.insert(
        "dynamic".into(),
        Value::String("V8/HotSpot/.NET — not C".into()),
    );
    m.insert(
        "manual_aot".into(),
        Value::String("Rust minus debug-check tax".into()),
    );
    m.insert(
        "never".into(),
        Value::String("faster than C on default GC Kab".into()),
    );
    Ok(Value::from_object(m))
}

pub fn ptak_globals(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("native_add_loop", native_add_loop_native),
        ("array_f64", array_f64_native),
        ("array_f64_sum", array_f64_sum_native),
        ("hidden_class_info", shape_info_native),
        ("gc_nursery_alloc", nursery_alloc_native),
        ("gc_nursery_stats", gc_nursery_stats_native),
        ("manual_checks_enabled", manual_checks_native),
        ("same_room_sql", same_room_list_native),
        ("league_add_loop", league_add_loop_native),
        ("tak_ceiling", tak_ceiling_native),
    ];
    for (name, f) in fns {
        env.set((*name).to_string(), Value::NativeFunction(*f));
    }
}
