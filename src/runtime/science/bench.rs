//! Science bench harness (SC4f) — timing Kab natives vs workloads.

use crate::bytecode::call_value;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::time::Instant;

fn int_out(n: i64) -> Value {
    Value::Number(n)
}

/// sci_bench(label, iterations, fn) → {label, iterations, elapsed_ms, ns_per_op}
fn sci_bench(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let label = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "bench".into(),
    };
    let iterations = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        Some(Value::Float(f)) if *f > 0.0 => *f as u64,
        _ => 1000,
    };
    let func = args.get(2).ok_or("sci_bench(label, n, fn)")?;
    let start = Instant::now();
    for _ in 0..iterations {
        call_value(func.clone(), vec![], &[], &[], &[], &[], env)?;
    }
    let elapsed_ms = start.elapsed().as_millis().max(1) as u64;
    let mut m = HashMap::new();
    m.insert("label".into(), Value::String(label));
    m.insert("iterations".into(), int_out(iterations as i64));
    m.insert("elapsed_ms".into(), int_out(elapsed_ms as i64));
    m.insert(
        "ns_per_op".into(),
        int_out((elapsed_ms as f64 * 1_000_000.0 / iterations as f64) as i64),
    );
    Ok(Value::Object(m))
}

/// sci_bench_report(benches[]) — summary object for docs/CI (non-blocking).
fn sci_bench_report(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(a)) => a,
        _ => return Err("sci_bench_report(benches[])".into()),
    };
    let mut total_ms = 0i64;
    let mut lines = Vec::new();
    for b in items {
        if let Value::Object(m) = b {
            let label = match m.get("label") {
                Some(Value::String(s)) => s.clone(),
                _ => "?".into(),
            };
            let ms = match m.get("elapsed_ms") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let ns = match m.get("ns_per_op") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            total_ms += ms;
            lines.push(Value::String(format!("{}: {} ms ({} ns/op)", label, ms, ns)));
        }
    }
    let mut out = HashMap::new();
    out.insert("count".into(), int_out(items.len() as i64));
    out.insert("total_ms".into(), int_out(total_ms));
    out.insert("lines".into(), Value::Array(lines));
    Ok(Value::Object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_sci_bench", "sci_bench"], sci_bench);
    bind(&["science_sci_bench_report", "sci_bench_report"], sci_bench_report);
}
