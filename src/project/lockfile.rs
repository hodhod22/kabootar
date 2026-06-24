//! `kabootar.lock` — pinned npm/JSR dependency lockfile.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct LockPackage {
    pub version: String,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lockfile {
    pub version: u32,
    pub packages: HashMap<String, LockPackage>,
}

pub fn lockfile_path(root: &Path) -> PathBuf {
    root.join("kabootar.lock")
}

pub fn read_lockfile(path: &Path) -> Result<Lockfile, String> {
    if !path.is_file() {
        return Ok(Lockfile::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    parse_lockfile(&text)
}

pub fn write_lockfile(path: &Path, lock: &Lockfile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create lockfile dir: {e}"))?;
    }
    fs::write(path, serialize_lockfile(lock))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub fn sync_lockfile_from_manifest() -> Result<Lockfile, String> {
    let root = crate::project::root::project_root()?;
    let manifest = crate::project::manifest::load_manifest_cwd()?;
    let mut lock = read_lockfile(&lockfile_path(&root))?;
    if lock.version == 0 {
        lock.version = 1;
    }
    for (name, constraint) in &manifest.dependencies {
        let resolved = crate::runtime::npm_remote::resolve_lock_package(name, constraint, &root);
        let entry = lock.packages.entry(name.clone()).or_insert(LockPackage {
            version: String::new(),
            integrity: None,
        });
        if entry.version.is_empty() || !crate::project::manifest::version_matches(&resolved.version, &entry.version) {
            entry.version = resolved.version;
        }
        if entry.integrity.is_none() {
            entry.integrity = resolved.integrity;
        }
    }
    write_lockfile(&lockfile_path(&root), &lock)?;
    Ok(lock)
}

fn parse_lockfile(text: &str) -> Result<Lockfile, String> {
    let mut lock = Lockfile::default();
    let mut in_packages = false;
    let mut current_pkg: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[packages]" {
            in_packages = true;
            continue;
        }
        if line.starts_with('[') {
            in_packages = false;
            current_pkg = None;
            continue;
        }
        if !in_packages {
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = parse_value(v.trim());
                if k == "version" {
                    lock.version = v.parse().unwrap_or(1);
                }
            }
            continue;
        }
        if line.starts_with("[[packages]]") || line.starts_with("[packages.") {
            current_pkg = None;
            if let Some(name) = line.strip_prefix("[packages.") {
                let name = name.trim_end_matches(']');
                current_pkg = Some(name.to_string());
                lock.packages
                    .entry(name.to_string())
                    .or_insert_with(|| LockPackage {
                        version: String::new(),
                        integrity: None,
                    });
            }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = parse_value(v.trim());
        if k == "name" {
            current_pkg = Some(v.clone());
            lock.packages
                .entry(v)
                .or_insert_with(|| LockPackage {
                    version: String::new(),
                    integrity: None,
                });
            continue;
        }
        if let Some(ref pkg_name) = current_pkg {
            let entry = lock.packages.entry(pkg_name.clone()).or_insert_with(|| LockPackage {
                version: String::new(),
                integrity: None,
            });
            match k {
                "version" => entry.version = v,
                "integrity" => entry.integrity = Some(v),
                _ => {}
            }
        }
    }
    if lock.version == 0 {
        lock.version = 1;
    }
    Ok(lock)
}

fn parse_value(raw: &str) -> String {
    raw.trim_matches('"').to_string()
}

pub fn serialize_lockfile(lock: &Lockfile) -> String {
    let mut out = format!("version = {}\n\n[packages]\n", lock.version);
    let mut names: Vec<_> = lock.packages.keys().collect();
    names.sort();
    for name in names {
        let pkg = &lock.packages[name];
        out.push_str(&format!("\n[packages.{}]\n", name));
        out.push_str(&format!("name = \"{}\"\n", name));
        out.push_str(&format!("version = \"{}\"\n", pkg.version));
        if let Some(integrity) = &pkg.integrity {
            out.push_str(&format!("integrity = \"{integrity}\"\n"));
        }
    }
    out
}

pub fn lockfile_to_value(lock: &Lockfile) -> crate::value::Value {
    use crate::value::Value;
    let mut packages = HashMap::new();
    for (name, pkg) in &lock.packages {
        let mut m = HashMap::new();
        m.insert("version".into(), Value::String(pkg.version.clone()));
        if let Some(integrity) = &pkg.integrity {
            m.insert("integrity".into(), Value::String(integrity.clone()));
        }
        packages.insert(name.clone(), Value::Object(m));
    }
    let mut root = HashMap::new();
    root.insert("version".into(), Value::Number(lock.version as i64));
    root.insert("packages".into(), Value::Object(packages));
    Value::Object(root)
}
