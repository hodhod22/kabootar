//! Local package registry — publish, install, and resolve `.kab` modules (v2.17).

use crate::project::version::strip_version_directive;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

pub fn registry_dir(base: &Path) -> PathBuf {
    base.join(".kabootar").join("registry")
}

pub fn packages_dir(base: &Path) -> PathBuf {
    base.join(".kabootar").join("packages")
}

fn package_file_name(name: &str) -> String {
    format!("{name}.kab")
}

fn read_package_meta(source: &str, fallback_name: &str) -> Result<(String, String), String> {
    let (version, _) = strip_version_directive(source);
    let version = version.ok_or_else(|| {
        format!(
            "Package \"{fallback_name}\" requires @version directive for registry"
        )
    })?;
    Ok((fallback_name.to_string(), version))
}

pub fn publish_file(source_path: &Path, base: &Path) -> Result<PackageInfo, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read {}: {e}", source_path.display()))?;
    let name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid package path: {}", source_path.display()))?;
    let (name, version) = read_package_meta(&source, name)?;
    let dest_dir = registry_dir(base).join(&name).join(&version);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create registry dir {}: {e}", dest_dir.display()))?;
    let dest = dest_dir.join(package_file_name(&name));
    fs::write(&dest, source).map_err(|e| format!("Failed to write {}: {e}", dest.display()))?;
    Ok(PackageInfo { name, version })
}

pub fn list_registry(base: &Path) -> Result<Vec<PackageInfo>, String> {
    let root = registry_dir(base);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for name_entry in fs::read_dir(&root).map_err(|e| format!("Failed to read registry: {e}"))? {
        let name_entry = name_entry.map_err(|e| format!("Failed to read registry entry: {e}"))?;
        if !name_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = name_entry.file_name().to_string_lossy().to_string();
        let version_root = name_entry.path();
        for ver_entry in fs::read_dir(&version_root)
            .map_err(|e| format!("Failed to read versions for {name}: {e}"))?
        {
            let ver_entry = ver_entry.map_err(|e| format!("Failed to read version entry: {e}"))?;
            if !ver_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let version = ver_entry.file_name().to_string_lossy().to_string();
            let kab = ver_entry.path().join(package_file_name(&name));
            if kab.is_file() {
                out.push(PackageInfo { name: name.clone(), version });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(out)
}

fn best_matching_version(versions: &[String], constraint: &str) -> Option<String> {
    let mut matches: Vec<String> = versions
        .iter()
        .filter(|v| crate::project::manifest::version_matches(v, constraint))
        .cloned()
        .collect();
    matches.sort();
    matches.pop()
}

pub fn find_registry_version(
    name: &str,
    constraint: &str,
    base: &Path,
) -> Result<Option<String>, String> {
    let root = registry_dir(base).join(name);
    if !root.is_dir() {
        return Ok(None);
    }
    let mut versions = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| format!("Failed to read registry/{name}: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read registry version: {e}"))?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(best_matching_version(&versions, constraint))
}

pub fn install_package(name: &str, constraint: &str, base: &Path) -> Result<PackageInfo, String> {
    let version = find_registry_version(name, constraint, base)?.ok_or_else(|| {
        format!(
            "Package \"{name}\" version {constraint} not found in registry (run kabootar publish)"
        )
    })?;
    let src = registry_dir(base)
        .join(name)
        .join(&version)
        .join(package_file_name(name));
    if !src.is_file() {
        return Err(format!(
            "Registry package file missing: {}",
            src.display()
        ));
    }
    let dest_dir = packages_dir(base).join(name).join(&version);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create packages dir {}: {e}", dest_dir.display()))?;
    let dest = dest_dir.join(package_file_name(name));
    fs::copy(&src, &dest).map_err(|e| {
        format!(
            "Failed to install {} -> {}: {e}",
            src.display(),
            dest.display()
        )
    })?;
    Ok(PackageInfo {
        name: name.to_string(),
        version,
    })
}

pub fn resolve_installed_path(
    name: &str,
    constraint: Option<&str>,
    base: &Path,
) -> Option<PathBuf> {
    let root = packages_dir(base).join(name);
    if !root.is_dir() {
        return None;
    }
    let mut versions = Vec::new();
    for entry in fs::read_dir(&root).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir() {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    let version = if let Some(c) = constraint {
        best_matching_version(&versions, c)?
    } else {
        let mut sorted = versions;
        sorted.sort();
        sorted.pop()?
    };
    let path = root.join(version).join(package_file_name(name));
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn install_manifest_deps(base: &Path) -> Result<Vec<PackageInfo>, String> {
    let manifest = crate::project::manifest::load_manifest(&base.join("kabootar.toml"))?;
    let mut installed = Vec::new();
    for (name, constraint) in &manifest.dependencies {
        installed.push(install_package(name, constraint, base)?);
    }
    Ok(installed)
}

pub fn list_installed(base: &Path) -> Result<Vec<PackageInfo>, String> {
    let root = packages_dir(base);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for name_entry in fs::read_dir(&root).map_err(|e| format!("Failed to read packages: {e}"))? {
        let name_entry = name_entry.map_err(|e| format!("Failed to read package entry: {e}"))?;
        if !name_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = name_entry.file_name().to_string_lossy().to_string();
        for ver_entry in fs::read_dir(name_entry.path())
            .map_err(|e| format!("Failed to read versions for {name}: {e}"))?
        {
            let ver_entry = ver_entry.map_err(|e| format!("Failed to read version entry: {e}"))?;
            if !ver_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let version = ver_entry.file_name().to_string_lossy().to_string();
            let kab = ver_entry.path().join(package_file_name(&name));
            if kab.is_file() {
                out.push(PackageInfo { name: name.clone(), version });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(out)
}

pub fn list_lib_modules(base: &Path) -> Result<Vec<PackageInfo>, String> {
    let lib = base.join("lib");
    if !lib.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&lib).map_err(|e| format!("Failed to read lib/: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read lib entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "kab" && ext != "kabootar" {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let (version, _) = strip_version_directive(&source);
        out.push(PackageInfo {
            name,
            version: version.unwrap_or_else(|| "0.0.0".into()),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn uninstall_package(name: &str, constraint: Option<&str>, base: &Path) -> Result<PackageInfo, String> {
    let mut installed: Vec<String> = list_installed(base)?
        .into_iter()
        .filter(|p| p.name == name)
        .map(|p| p.version)
        .collect();
    let version = if let Some(c) = constraint {
        best_matching_version(&installed, c)
            .ok_or_else(|| format!("Package \"{name}\" version {c} not installed"))?
    } else {
        installed.sort();
        installed
            .pop()
            .ok_or_else(|| format!("Package \"{name}\" is not installed"))?
    };
    let dir = packages_dir(base).join(name).join(&version);
    if dir.is_dir() {
        fs::remove_dir_all(&dir)
            .map_err(|e| format!("Failed to remove {}: {e}", dir.display()))?;
    }
    Ok(PackageInfo {
        name: name.to_string(),
        version,
    })
}

/// Publish every `lib/*.kab` with `@version` into the local registry.
pub fn seed_lib_to_registry(base: &Path) -> Result<Vec<PackageInfo>, String> {
    let lib = base.join("lib");
    if !lib.is_dir() {
        return Ok(Vec::new());
    }
    let mut published = Vec::new();
    for entry in fs::read_dir(&lib).map_err(|e| format!("Failed to read lib/: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read lib entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("kab") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if strip_version_directive(&source).0.is_none() {
            continue;
        }
        published.push(publish_file(&path, base)?);
    }
    published.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(published)
}

pub fn search_packages(query: &str, base: &Path) -> Result<Vec<(String, String, String)>, String> {
    let q = query.to_ascii_lowercase();
    let mut hits: Vec<(String, String, String)> = Vec::new();

    for name in crate::modules::list_builtins() {
        if q.is_empty() || name.to_ascii_lowercase().contains(&q) {
            hits.push((name.to_string(), "builtin".into(), "".into()));
        }
    }

    for p in list_lib_modules(base)? {
        if q.is_empty() || p.name.to_ascii_lowercase().contains(&q) {
            hits.push((p.name.clone(), "lib".into(), p.version.clone()));
        }
    }

    for p in list_registry(base)? {
        if q.is_empty() || p.name.to_ascii_lowercase().contains(&q) {
            hits.push((p.name.clone(), "registry".into(), p.version.clone()));
        }
    }

    for p in list_installed(base)? {
        if q.is_empty() || p.name.to_ascii_lowercase().contains(&q) {
            hits.push((p.name.clone(), "installed".into(), p.version.clone()));
        }
    }

    hits.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    hits.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kabootar_registry_{}_{}",
            process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn publish_and_install_roundtrip() {
        let base = temp_root();
        let src = base.join("greet.kab");
        fs::write(&src, r#"@version "1.0.0"

pub fn greet(name) {
    return "hi:" + name
}
"#)
        .unwrap();
        let info = publish_file(&src, &base).unwrap();
        assert_eq!(info.name, "greet");
        assert_eq!(info.version, "1.0.0");

        let installed = install_package("greet", "1.0", &base).unwrap();
        assert_eq!(installed.version, "1.0.0");
        let path = resolve_installed_path("greet", Some("1.0"), &base).unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn list_registry_returns_published_packages() {
        let base = temp_root();
        let src = base.join("utils.kab");
        fs::write(&src, r#"@version "2.1.0"
pub fn ok() { return true }
"#)
        .unwrap();
        publish_file(&src, &base).unwrap();
        let list = list_registry(&base).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "utils");
    }
}
