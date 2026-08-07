//! CodAI natives — `import "codai"`.

use crate::codai::{
    all_ids, all_project_ids, categories, complete, compose, explain, format_scaffold_report,
    format_sync_report, help, progress_report, project_plan, project_tree, resolve_base_path,
    scaffold_project, suggest, suggest_projects, sync_project, util,
};
use crate::value::{Environment, Value};

fn code_utils_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items: Vec<Value> = all_ids()
        .into_iter()
        .map(|id| Value::String(id.to_string()))
        .collect();
    Ok(Value::from_array(items))
}

fn code_util_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_util(id) expects a string".into()),
    };
    Ok(Value::String(util(id)?))
}

fn code_suggest_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_suggest(query) expects a string".into()),
    };
    let limit = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        Some(Value::Float(f)) if *f > 0.0 => *f as usize,
        None => 8,
        _ => return Err("code_suggest limit must be a positive number".into()),
    };
    let hits = suggest(query, limit);
    let items: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            Value::String(format!(
                "[{}] {} — {} | {}",
                h.score, h.id, h.title, h.description
            ))
        })
        .collect();
    Ok(Value::from_array(items))
}

fn code_compose_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ids = match args.first() {
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for v in items.iter() {
                match v {
                    Value::String(s) => out.push(s.as_str()),
                    _ => return Err("code_compose expects array of strings".into()),
                }
            }
            out
        }
        Some(Value::String(s)) => vec![s.as_str()],
        None => return Err("code_compose(ids) expects an array or string".into()),
        _ => return Err("code_compose(ids) expects an array or string".into()),
    };
    Ok(Value::String(compose(&ids)?))
}

fn code_complete_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let partial = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_complete(partial) expects a string".into()),
    };
    match complete(partial) {
        Some(code) => Ok(Value::String(code)),
        None => Ok(Value::String(String::new())),
    }
}

fn code_explain_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_explain(code) expects a string".into()),
    };
    Ok(Value::String(explain(code)))
}

fn code_help_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let topic = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        None => "",
        _ => return Err("code_help(topic?) expects a string".into()),
    };
    Ok(Value::String(help(topic)))
}

fn code_categories_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items: Vec<Value> = categories()
        .into_iter()
        .map(|c| Value::String(c.to_string()))
        .collect();
    Ok(Value::from_array(items))
}

fn code_projects_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items: Vec<Value> = all_project_ids()
        .into_iter()
        .map(|id| Value::String(id.to_string()))
        .collect();
    Ok(Value::from_array(items))
}

fn code_project_suggest_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_project_suggest(query) expects a string".into()),
    };
    let limit = match args.get(1) {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        Some(Value::Float(f)) if *f > 0.0 => *f as usize,
        None => 5,
        _ => return Err("code_project_suggest limit must be a positive number".into()),
    };
    let hits = suggest_projects(query, limit);
    let items: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            Value::String(format!(
                "[{}] {} — {} | {}",
                h.score, h.id, h.title, h.description
            ))
        })
        .collect();
    Ok(Value::from_array(items))
}

fn code_project_tree_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_project_tree(id) expects a string".into()),
    };
    Ok(Value::String(project_tree(id)?))
}

fn code_project_plan_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_project_plan(id) expects a string".into()),
    };
    let plan = project_plan(id)?;
    let items: Vec<Value> = plan
        .into_iter()
        .map(|(path, desc)| Value::String(format!("{path} — {desc}")))
        .collect();
    Ok(Value::from_array(items))
}

fn code_project_scaffold_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_project_scaffold(id, path?, force?) expects a string id".into()),
    };
    let path = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        None => ".",
        _ => return Err("code_project_scaffold path must be a string".into()),
    };
    let force = match args.get(2) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => *n != 0,
        None => false,
        _ => return Err("code_project_scaffold force must be boolean".into()),
    };
    let base = resolve_base_path(path);
    let report = scaffold_project(id, &base, force)?;
    Ok(Value::String(format_scaffold_report(id, &report)))
}

fn code_project_progress_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("code_project_progress(id) expects a string".into()),
    };
    Ok(Value::String(progress_report(id)?))
}

fn code_project_sync_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        None => ".",
        _ => return Err("code_project_sync(path?) expects a string".into()),
    };
    let base = resolve_base_path(path);
    let report = sync_project(&base)?;
    Ok(Value::String(format_sync_report(&report)))
}

pub fn register(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("code_utils", code_utils_native),
        ("code_util", code_util_native),
        ("code_suggest", code_suggest_native),
        ("code_compose", code_compose_native),
        ("code_complete", code_complete_native),
        ("code_explain", code_explain_native),
        ("code_help", code_help_native),
        ("code_categories", code_categories_native),
        ("code_projects", code_projects_native),
        ("code_project_suggest", code_project_suggest_native),
        ("code_project_tree", code_project_tree_native),
        ("code_project_plan", code_project_plan_native),
        ("code_project_scaffold", code_project_scaffold_native),
        ("code_project_progress", code_project_progress_native),
        ("code_project_sync", code_project_sync_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
