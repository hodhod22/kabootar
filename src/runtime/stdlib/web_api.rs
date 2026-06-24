//! Web platform globals — `performance.now`, `crypto.getRandomValues`.

use crate::runtime::security::random_bytes;
use crate::runtime::shared_memory;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

static PERF_ORIGIN: OnceLock<Instant> = OnceLock::new();

fn perf_origin() -> Instant {
    *PERF_ORIGIN.get_or_init(Instant::now)
}

fn performance_now_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = perf_origin().elapsed().as_secs_f64() * 1000.0;
    Ok(Value::Float(ms))
}

const MAX_RANDOM_BYTES: usize = 65_536;

fn crypto_get_random_values_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let target = args
        .first()
        .ok_or("crypto.getRandomValues() expects an Array or Uint8Array")?;
    fill_random_target(target)
}

fn fill_random_target(target: &Value) -> Result<Value, String> {
    if shared_memory::is_uint8_array(target) {
        let len = shared_memory::uint8_array_byte_length(target)?;
        if len > MAX_RANDOM_BYTES {
            return Err("crypto.getRandomValues() maximum length is 65536".into());
        }
        if len == 0 {
            return Ok(target.clone());
        }
        let bytes = random_bytes(len)?;
        shared_memory::fill_uint8_array(target, &bytes)?;
        return Ok(target.clone());
    }

    let Value::Array(items) = target else {
        return Err("crypto.getRandomValues() expects an Array or Uint8Array".into());
    };
    let len = items.len();
    if len > MAX_RANDOM_BYTES {
        return Err("crypto.getRandomValues() maximum length is 65536".into());
    }
    if len == 0 {
        return Ok(Value::Array(Vec::new()));
    }
    let bytes = random_bytes(len)?;
    let filled: Vec<Value> = bytes
        .into_iter()
        .map(|b| Value::Number(b as i64))
        .collect();
    Ok(Value::Array(filled))
}

fn build_performance() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_performance".into(), Value::Bool(true));
    m.insert("now".into(), Value::NativeFunction(performance_now_native));
    Value::Object(m)
}

fn build_crypto() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_crypto_web".into(), Value::Bool(true));
    m.insert(
        "getRandomValues".into(),
        Value::NativeFunction(crypto_get_random_values_native),
    );
    Value::Object(m)
}

pub fn register_web_api(env: &mut Environment) {
    env.set("performance".to_string(), build_performance());
    env.set("crypto".to_string(), build_crypto());
}
