//! `Deno.test` / `Deno.bench` — in-process test and benchmark runners.

use crate::bytecode::call_value;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
struct TestRecord {
    name: String,
    passed: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct BenchRecord {
    name: String,
    duration_ms: f64,
}

thread_local! {
    static TESTS: RefCell<Vec<TestRecord>> = RefCell::new(Vec::new());
    static BENCHES: RefCell<Vec<BenchRecord>> = RefCell::new(Vec::new());
}

fn run_callable(func: &Value, env: &mut Environment) -> Result<Value, String> {
    call_value(func.clone(), vec![], &[], &[], &[], &[], env)
}

fn deno_test_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("deno_test(name, fn)".into()),
    };
    let func = args.get(1).ok_or("deno_test(name, fn)")?;
    let result = run_callable(func, env);
    let record = match result {
        Ok(_) => TestRecord {
            name,
            passed: true,
            error: None,
        },
        Err(e) => TestRecord {
            name,
            passed: false,
            error: Some(e),
        },
    };
    TESTS.with(|t| t.borrow_mut().push(record));
    Ok(Value::Undefined)
}

fn deno_bench_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("deno_bench(name, fn)".into()),
    };
    let func = args.get(1).ok_or("deno_bench(name, fn)")?;
    let start = Instant::now();
    run_callable(func, env)?;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    BENCHES.with(|b| {
        b.borrow_mut().push(BenchRecord {
            name,
            duration_ms,
        })
    });
    Ok(Value::Undefined)
}

fn deno_test_report_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    TESTS.with(|t| {
        let records = t.borrow();
        let passed = records.iter().filter(|r| r.passed).count() as i64;
        let failed = records.len() as i64 - passed;
        let mut failures = Vec::new();
        for r in records.iter().filter(|r| !r.passed) {
            let mut item = HashMap::new();
            item.insert("name".into(), Value::String(r.name.clone()));
            item.insert(
                "error".into(),
                Value::String(r.error.clone().unwrap_or_default()),
            );
            failures.push(Value::Object(item));
        }
        let mut out = HashMap::new();
        out.insert("passed".into(), Value::Number(passed));
        out.insert("failed".into(), Value::Number(failed));
        out.insert("total".into(), Value::Number(records.len() as i64));
        out.insert("failures".into(), Value::Array(failures));
        Ok(Value::Object(out))
    })
}

fn deno_bench_report_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    BENCHES.with(|b| {
        let records = b.borrow();
        let items: Vec<Value> = records
            .iter()
            .map(|r| {
                let mut m = HashMap::new();
                m.insert("name".into(), Value::String(r.name.clone()));
                m.insert("durationMs".into(), Value::Float(r.duration_ms));
                Value::Object(m)
            })
            .collect();
        let mut out = HashMap::new();
        out.insert("benches".into(), Value::Array(items));
        Ok(Value::Object(out))
    })
}

pub fn build_test_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_deno_test".into(), Value::Bool(true));
    m.insert("test".into(), Value::NativeFunction(deno_test_native));
    m.insert(
        "report".into(),
        Value::NativeFunction(deno_test_report_native),
    );
    Value::Object(m)
}

pub fn build_bench_namespace() -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_deno_bench".into(), Value::Bool(true));
    m.insert("bench".into(), Value::NativeFunction(deno_bench_native));
    m.insert(
        "report".into(),
        Value::NativeFunction(deno_bench_report_native),
    );
    Value::Object(m)
}

pub fn register_testing(env: &mut Environment) {
    env.set("Deno_test".to_string(), build_test_namespace());
    env.set("Deno_bench".to_string(), build_bench_namespace());
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("deno_test", deno_test_native),
        ("deno_bench", deno_bench_native),
        ("deno_test_report", deno_test_report_native),
        ("deno_bench_report", deno_bench_report_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
