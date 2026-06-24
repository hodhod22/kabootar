//! npm / TypeScript parity helpers for Deno.

use crate::project::root::project_root;
use crate::registry::{install_package, resolve_installed_path};
use crate::runtime::npm_remote::{
    self, fetch_jsr_package, fetch_npm_package, list_remote_cache, parse_package_spec,
    read_cached_source, resolve_cached_version, RegistryKind,
};
use crate::value::Value;
use std::collections::HashMap;

fn package_info_object(name: &str, version: &str, registry: &str) -> Value {
    let mut map = HashMap::new();
    map.insert("name".into(), Value::String(name.into()));
    map.insert("version".into(), Value::String(version.into()));
    map.insert("registry".into(), Value::String(registry.into()));
    Value::Object(map)
}

pub fn npm_parse_spec(raw: &str) -> Result<Value, String> {
    let spec = parse_package_spec(raw)?;
    let mut map = HashMap::new();
    map.insert(
        "kind".into(),
        Value::String(match spec.kind {
            RegistryKind::Npm => "npm".into(),
            RegistryKind::Jsr => "jsr".into(),
        }),
    );
    map.insert("name".into(), Value::String(spec.name));
    if let Some(v) = spec.version {
        map.insert("version".into(), Value::String(v));
    }
    if let Some(s) = spec.subpath {
        map.insert("subpath".into(), Value::String(s));
    }
    Ok(Value::Object(map))
}

pub fn npm_install(name: &str, version: Option<&str>) -> Result<Value, String> {
    let constraint = version.unwrap_or("0");
    let base = project_root()?;

    if let Ok(info) = install_package(name, constraint, &base) {
        return Ok(package_info_object(&info.name, &info.version, "local"));
    }

    let spec = parse_package_spec(name)?;
    let constraint = spec.version.as_deref().unwrap_or(constraint);
    let info = match spec.kind {
        RegistryKind::Npm => fetch_npm_package(&spec.name, constraint, &base)?,
        RegistryKind::Jsr => fetch_jsr_package(&spec.name, constraint, &base)?,
    };
    Ok(package_info_object(
        &info.name,
        &info.version,
        match spec.kind {
            RegistryKind::Npm => "npm",
            RegistryKind::Jsr => "jsr",
        },
    ))
}

pub fn npm_fetch(name: &str, version: Option<&str>) -> Result<Value, String> {
    let base = project_root()?;
    let spec = parse_package_spec(name)?;
    let constraint = version.or(spec.version.as_deref()).unwrap_or("latest");
    let info = match spec.kind {
        RegistryKind::Npm => fetch_npm_package(&spec.name, constraint, &base)?,
        RegistryKind::Jsr => fetch_jsr_package(&spec.name, constraint, &base)?,
    };
    Ok(package_info_object(
        &info.name,
        &info.version,
        match spec.kind {
            RegistryKind::Npm => "npm",
            RegistryKind::Jsr => "jsr",
        },
    ))
}

pub fn jsr_fetch(name: &str, version: Option<&str>) -> Result<Value, String> {
    let base = project_root()?;
    let spec = if name.starts_with("jsr:") || name.starts_with('@') {
        parse_package_spec(name)?
    } else {
        parse_package_spec(&format!("jsr:{name}"))?
    };
    let constraint = version.or(spec.version.as_deref()).unwrap_or("latest");
    let info = fetch_jsr_package(&spec.name, constraint, &base)?;
    Ok(package_info_object(&info.name, &info.version, "jsr"))
}

pub fn npm_resolve(name: &str, version: Option<&str>) -> Result<Value, String> {
    let base = project_root()?;
    let spec = parse_package_spec(name)?;
    let constraint = version.or(spec.version.as_deref()).unwrap_or("latest");
    if let Some(ver) = resolve_cached_version(spec.kind, &spec.name, constraint, &base) {
        return Ok(package_info_object(
            &spec.name,
            &ver,
            match spec.kind {
                RegistryKind::Npm => "npm",
                RegistryKind::Jsr => "jsr",
            },
        ));
    }
    npm_fetch(name, Some(constraint))
}

pub fn npm_list_cache() -> Result<Value, String> {
    let base = project_root()?;
    let items: Vec<Value> = list_remote_cache(&base)?
        .into_iter()
        .map(|(kind, info)| {
            package_info_object(
                &info.name,
                &info.version,
                match kind {
                    RegistryKind::Npm => "npm",
                    RegistryKind::Jsr => "jsr",
                },
            )
        })
        .collect();
    Ok(Value::Array(items))
}

pub fn npm_import_source(name: &str, version: Option<&str>) -> Result<String, String> {
    let constraint = version.unwrap_or("0");
    let base = project_root()?;

    if let Some(path) = resolve_installed_path(name, Some(constraint), &base) {
        return std::fs::read_to_string(path).map_err(|e| format!("npm_import read: {e}"));
    }

    let spec = parse_package_spec(name)?;
    let constraint = version.or(spec.version.as_deref()).unwrap_or(constraint);

    if let Some(ver) = resolve_cached_version(spec.kind, &spec.name, constraint, &base) {
        return read_cached_source(
            spec.kind,
            &spec.name,
            &ver,
            spec.subpath.as_deref(),
            &base,
        );
    }

    let info = match spec.kind {
        RegistryKind::Npm => fetch_npm_package(&spec.name, constraint, &base)?,
        RegistryKind::Jsr => fetch_jsr_package(&spec.name, constraint, &base)?,
    };
    read_cached_source(
        spec.kind,
        &spec.name,
        &info.version,
        spec.subpath.as_deref(),
        &base,
    )
}

pub fn npm_import_source_prepared(name: &str, version: Option<&str>) -> Result<String, String> {
    let source = npm_import_source(name, version)?;
    if source.contains("interface ")
        || source.contains("type ")
        || source.contains(": string")
        || source.contains(": number")
    {
        Ok(ts_strip_types(&source))
    } else {
        Ok(source)
    }
}

pub fn ensure_registry_module(name: &str, version: Option<&str>) -> Result<std::path::PathBuf, String> {
    let base = project_root()?;
    let spec = parse_package_spec(name)?;
    let constraint = version.or(spec.version.as_deref()).unwrap_or("latest");
    let ver = if let Some(v) = resolve_cached_version(spec.kind, &spec.name, constraint, &base) {
        v
    } else {
        let info = match spec.kind {
            RegistryKind::Npm => fetch_npm_package(&spec.name, constraint, &base)?,
            RegistryKind::Jsr => fetch_jsr_package(&spec.name, constraint, &base)?,
        };
        info.version
    };
    npm_remote::resolve_entry_path(
        spec.kind,
        &spec.name,
        &ver,
        spec.subpath.as_deref(),
        &base,
    )
}

/// Strip common TypeScript syntax to Kabootar-compatible source.
pub fn ts_strip_types(source: &str) -> String {
    crate::runtime::ts_compile::compile_to_kabootar(
        source,
        &crate::runtime::ts_compile::TsCompileOptions::default(),
    )
    .0
}

pub fn ts_compile(source: &str) -> Value {
    let (code, diags) = crate::runtime::ts_compile::compile_to_kabootar(
        source,
        &crate::runtime::ts_compile::TsCompileOptions::default(),
    );
    let mut map = HashMap::new();
    map.insert("code".into(), Value::String(code));
    map.insert(
        "diagnostics".into(),
        Value::Array(crate::runtime::ts_compile::diagnostics_to_values(&diags)),
    );
    Value::Object(map)
}

pub fn ts_compile_file(path: &str) -> Result<Value, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("ts_compile_file read {}: {e}", path))?;
    Ok(ts_compile(&source))
}

pub fn ts_transpile(source: &str) -> Value {
    Value::String(ts_strip_types(source))
}
