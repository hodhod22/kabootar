//! Registry natives — `registry_publish`, `registry_install`, `registry_list` (v2.17).

use crate::project::root::project_root;
use crate::registry::{install_package, list_registry, publish_file};
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::path::PathBuf;

fn expect_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects a string argument")),
    }
}

fn registry_publish_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "registry_publish()")?;
    let base = project_root()?;
    let info = publish_file(PathBuf::from(&path).as_path(), &base)?;
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::String(info.name));
    map.insert("version".to_string(), Value::String(info.version));
    Ok(Value::from_object(map))
}

fn registry_install_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let name = expect_string(args, 0, "registry_install()")?;
    let constraint = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        None => "0".to_string(),
        _ => return Err("registry_install(name, version) expects a string or number".into()),
    };
    let base = project_root()?;
    let info = install_package(&name, &constraint, &base)?;
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::String(info.name));
    map.insert("version".to_string(), Value::String(info.version));
    Ok(Value::from_object(map))
}

fn registry_list_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let base = project_root()?;
    let packages = list_registry(&base)?;
    let items: Vec<Value> = packages
        .into_iter()
        .map(|p| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::String(p.name));
            map.insert("version".to_string(), Value::String(p.version));
            Value::from_object(map)
        })
        .collect();
    Ok(Value::from_array(items))
}

pub fn registry_globals(env: &mut Environment) {
    env.set(
        "registry_publish".to_string(),
        Value::NativeFunction(registry_publish_native),
    );
    env.set(
        "registry_install".to_string(),
        Value::NativeFunction(registry_install_native),
    );
    env.set(
        "registry_list".to_string(),
        Value::NativeFunction(registry_list_native),
    );
}
