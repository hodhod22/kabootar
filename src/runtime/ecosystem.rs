//! Ecosystem discovery — builtins, lib/, registry, installed packages.

use crate::modules::list_builtins;
use crate::registry::{
    list_installed, list_lib_modules, list_registry, search_packages, seed_lib_to_registry,
    uninstall_package,
};
use crate::project::root::project_root;
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn pkg_object(name: &str, version: &str, source: &str) -> Value {
    let mut m = HashMap::new();
    m.insert("name".into(), Value::String(name.into()));
    m.insert("version".into(), Value::String(version.into()));
    m.insert("source".into(), Value::String(source.into()));
    Value::Object(m)
}

fn ecosystem_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let base = project_root()?;
    let builtins = list_builtins().len();
    let lib = list_lib_modules(&base)?.len();
    let registry = list_registry(&base)?.len();
    let installed = list_installed(&base)?.len();
    let manifest_deps = crate::project::manifest::load_manifest_cwd()
        .map(|m| m.dependencies.len())
        .unwrap_or(0);

    let mut info = HashMap::new();
    info.insert("stage".into(), Value::String("early".into()));
    info.insert("builtin_modules".into(), Value::Number(builtins as i64));
    info.insert("lib_modules".into(), Value::Number(lib as i64));
    info.insert("registry_packages".into(), Value::Number(registry as i64));
    info.insert("installed_packages".into(), Value::Number(installed as i64));
    info.insert(
        "manifest_dependencies".into(),
        Value::Number(manifest_deps as i64),
    );
    info.insert(
        "summary".into(),
        Value::String(
            "Use modules_catalog(), registry_search(q), and registry_seed() to grow the local ecosystem.".into(),
        ),
    );
    Ok(Value::Object(info))
}

fn modules_catalog_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let base = project_root()?;
    let mut items = Vec::new();

    for name in list_builtins() {
        items.push(pkg_object(name, "", "builtin"));
    }

    for p in list_lib_modules(&base)? {
        items.push(pkg_object(&p.name, &p.version, "lib"));
    }

    for p in list_registry(&base)? {
        items.push(pkg_object(&p.name, &p.version, "registry"));
    }

    for p in list_installed(&base)? {
        items.push(pkg_object(&p.name, &p.version, "installed"));
    }

    Ok(Value::Array(items))
}

fn registry_search_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        None => "",
        _ => return Err("registry_search(query)".into()),
    };
    let base = project_root()?;
    let hits: Vec<Value> = search_packages(query, &base)?
        .into_iter()
        .map(|(name, source, version)| pkg_object(&name, &version, &source))
        .collect();
    Ok(Value::Array(hits))
}

fn registry_seed_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let base = project_root()?;
    let published: Vec<Value> = seed_lib_to_registry(&base)?
        .into_iter()
        .map(|p| pkg_object(&p.name, &p.version, "registry"))
        .collect();
    Ok(Value::Array(published))
}

fn registry_uninstall_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("registry_uninstall(name, version?)".into()),
    };
    let constraint = match args.get(1) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        None => None,
        _ => return Err("registry_uninstall(name, version?)".into()),
    };
    let base = project_root()?;
    let info = uninstall_package(&name, constraint.as_deref(), &base)?;
    Ok(pkg_object(&info.name, &info.version, "uninstalled"))
}

pub fn ecosystem_globals(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("ecosystem_info", ecosystem_info_native),
        ("modules_catalog", modules_catalog_native),
        ("registry_search", registry_search_native),
        ("registry_seed", registry_seed_native),
        ("registry_uninstall", registry_uninstall_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
