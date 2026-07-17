//! Kabootar language feature natives — channels, actors, persist, benchmark, shader.

use crate::value::{format_value, Environment, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

thread_local! {
    static CHANNELS: RefCell<HashMap<u64, ChannelState>> = RefCell::new(HashMap::new());
}

static NEXT_CHANNEL: AtomicU64 = AtomicU64::new(1);
static NEXT_ACTOR: AtomicU64 = AtomicU64::new(100);

struct ChannelState {
    queue: VecDeque<Value>,
    capacity: usize,
}

fn with_channels<F, T>(f: F) -> T
where
    F: FnOnce(&mut HashMap<u64, ChannelState>) -> T,
{
    CHANNELS.with(|c| f(&mut c.borrow_mut()))
}

fn get_os(env: &Environment) -> Option<crate::runtime::os::OsHandle> {
    let os = env.get("os")?;
    let Value::OsHandle(h) = os else {
        return None;
    };
    Some(h)
}

fn channel_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let cap = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(16);
    let id = NEXT_CHANNEL.fetch_add(1, Ordering::Relaxed);
    with_channels(|map| {
        map.insert(
            id,
            ChannelState {
                queue: VecDeque::new(),
                capacity: cap,
            },
        );
    });
    let mut m = HashMap::new();
    m.insert("id".into(), Value::Number(id as i64));
    m.insert("capacity".into(), Value::Number(cap as i64));
    Ok(Value::Object(m))
}

fn channel_send_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        Some(Value::Object(o)) => o
            .get("id")
            .and_then(|v| match v {
                Value::Number(n) => Some(*n as u64),
                _ => None,
            })
            .ok_or("channel_send expects channel id")?,
        _ => return Err("channel_send(channel, value)".into()),
    };
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    with_channels(|g| {
        let ch = g.get_mut(&id).ok_or("unknown channel")?;
        if ch.queue.len() >= ch.capacity {
            return Err("channel full".into());
        }
        ch.queue.push_back(val);
        Ok(Value::Bool(true))
    })
}

fn channel_recv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        Some(Value::Object(o)) => o
            .get("id")
            .and_then(|v| match v {
                Value::Number(n) => Some(*n as u64),
                _ => None,
            })
            .ok_or("channel_recv expects channel id")?,
        _ => return Err("channel_recv(channel)".into()),
    };
    with_channels(|g| {
        let ch = g.get_mut(&id).ok_or("unknown channel")?;
        Ok(ch.queue.pop_front().unwrap_or(Value::Null))
    })
}

fn actor_spawn_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "actor".into(),
    };
    let id = NEXT_ACTOR.fetch_add(1, Ordering::Relaxed);
    let mailbox = channel_new_native(&[Value::Number(32)], env)?;
    if let Some(os) = get_os(env) {
        let _ = os.spawn(&format!("actor-{name}"));
        let _ = os.sched_enqueue(&name);
    }
    let mut m = HashMap::new();
    m.insert("id".into(), Value::Number(id as i64));
    m.insert("name".into(), Value::String(name));
    m.insert("mailbox".into(), mailbox);
    Ok(Value::Object(m))
}

fn persist_save_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("persist_save(path, value)".into()),
    };
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let body = format_value(&val);
    if let Some(os) = get_os(env) {
        os.write(&path, body)?;
        let _ = os.vfs_save(&path);
    }
    Ok(Value::String(path))
}

fn persist_load_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("persist_load(path)".into()),
    };
    if let Some(os) = get_os(env) {
        return Ok(Value::String(os.read(&path)?));
    }
    Ok(Value::Null)
}

fn lang_benchmark_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let label = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "fn".into(),
    };
    let iterations = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u64),
            _ => None,
        })
        .unwrap_or(1000);
    let func = args.get(2).ok_or("lang_benchmark(label, n, fn)")?;
    let start = Instant::now();
    match func {
        Value::BytecodeFn(f) => {
            for _ in 0..iterations {
                let mut call_env = crate::value::Environment::child(f.closure.clone());
                crate::bytecode::run_bytecode_fn(f.def.as_ref(), vec![], &mut call_env)?;
            }
        }
        Value::Function { .. } => {
            for _ in 0..iterations {
                crate::evaluator::call_function_value(func, env)?;
            }
        }
        _ => return Err("lang_benchmark expects Kabootar function".into()),
    }
    let elapsed_ms = start.elapsed().as_millis().max(1) as u64;
    let mut m = HashMap::new();
    m.insert("label".into(), Value::String(label));
    m.insert("iterations".into(), Value::Number(iterations as i64));
    m.insert("elapsed_ms".into(), Value::Number(elapsed_ms as i64));
    m.insert(
        "ns_per_op".into(),
        Value::Number((elapsed_ms as f64 * 1_000_000.0 / iterations as f64) as i64),
    );
    Ok(Value::Object(m))
}

fn comptime_assert_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ok = args.first().map(|v| match v {
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0,
        Value::Null => false,
        _ => true,
    }).unwrap_or(false);
    let msg = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "comptime_assert failed".into());
    if !ok {
        return Err(msg);
    }
    Ok(Value::Bool(true))
}

fn shader_compile_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "shader".into(),
    };
    let source = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let mut m = HashMap::new();
    m.insert("name".into(), Value::String(name));
    m.insert("source".into(), Value::String(source));
    m.insert("backend".into(), Value::String("spirv-stub".into()));
    m.insert("ok".into(), Value::Bool(true));
    Ok(Value::Object(m))
}

fn lang_syscalls_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if env.get("os_syscalls").is_some() {
        if let Some(Value::NativeFunction(f)) = env.get("os_syscalls") {
            return f(&[], env);
        }
    }
    Ok(Value::Array(Vec::new()))
}

fn lang_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let features: Vec<(&str, &str, &str)> = vec![
        ("zero_ffi", "exists", "os_syscall, os_syscalls"),
        ("comptime", "partial", "comptime { } + comptime_assert"),
        ("actors", "partial", "actor Name { } → actor_spawn"),
        ("hot_reload", "partial", "kabootar serve --watch + kbc invalidate"),
        ("auto_simd", "stub", "@simd directive (doc)"),
        ("memory", "ok", "@manual owned_*/os/mem (systems); GC default (web)"),
        ("web_native", "partial", "html! → kv8_run_ui"),
        ("toolchain", "partial", "compile self-host default, registry, fmt, serve"),
        ("static_binary", "partial", "cargo build --release"),
        ("match_guards", "exists", "match x if cond =>"),
        ("effects", "partial", "@pure @io @disk stripped at compile"),
        ("benchmark", "partial", "lang_benchmark + @benchmark"),
        ("doc_examples", "stub", "@example planned"),
        ("channels", "exists", "channel_new/send/recv"),
        ("cache_layout", "stub", "@packed directive"),
        ("post_quantum", "partial", "crypto_kyber_encapsulate"),
        ("persist", "partial", "persist_save/load + @persist"),
        ("gpu_shader", "partial", "shader_compile + webgl_*"),
        ("resumable_errors", "partial", "try/catch returns resume value"),
        ("self_hosting", "partial", "kabootar compile via self_host (Rust fallback)"),
    ];
    let items: Vec<Value> = features
        .into_iter()
        .map(|(k, st, api)| {
            let mut m = HashMap::new();
            m.insert("feature".into(), Value::String(k.into()));
            m.insert("status".into(), Value::String(st.into()));
            m.insert("api".into(), Value::String(api.into()));
            Value::Object(m)
        })
        .collect();
    Ok(Value::Array(items))
}

pub fn lang_features_globals(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("channel_new", channel_new_native),
        ("channel_send", channel_send_native),
        ("channel_recv", channel_recv_native),
        ("actor_spawn", actor_spawn_native),
        ("persist_save", persist_save_native),
        ("persist_load", persist_load_native),
        ("lang_benchmark", lang_benchmark_native),
        ("comptime_assert", comptime_assert_native),
        ("shader_compile", shader_compile_native),
        ("lang_syscalls", lang_syscalls_native),
        ("lang_info", lang_info_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
