//! DocAI natives — `import "docai"`.

use crate::docai::{ask, search, topics};
use crate::value::{Environment, Value};

fn doc_ask_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("doc_ask(query) expects a string".into()),
    };
    Ok(Value::String(ask(query).text))
}

fn doc_search_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("doc_search(query) expects a string".into()),
    };
    let limit = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        Some(Value::Float(f)) if *f > 0.0 => *f as usize,
        None => 5,
        _ => return Err("doc_search limit must be a positive number".into()),
    };
    let hits = search(query, limit);
    let items: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            Value::String(format!(
                "[{}] {} — {} | {}",
                h.score,
                h.path,
                h.heading,
                h.excerpt.lines().next().unwrap_or("").trim()
            ))
        })
        .collect();
    Ok(Value::from_array(items))
}

fn doc_sources_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("doc_sources(query) expects a string".into()),
    };
    let answer = ask(query);
    let items: Vec<Value> = answer
        .sources
        .into_iter()
        .map(|h| Value::String(format!("{} — {}", h.path, h.heading)))
        .collect();
    Ok(Value::from_array(items))
}

fn doc_topics_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items: Vec<Value> = topics()
        .into_iter()
        .map(|t| Value::String(t.to_string()))
        .collect();
    Ok(Value::from_array(items))
}

pub fn register(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("doc_ask", doc_ask_native),
        ("doc_search", doc_search_native),
        ("doc_sources", doc_sources_native),
        ("doc_topics", doc_topics_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
