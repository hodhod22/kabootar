//! Remote npm / JSR registry fetch, cache, and resolution (Deno våg 14).

use crate::registry::PackageInfo;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryKind {
    Npm,
    Jsr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub kind: RegistryKind,
    pub name: String,
    pub version: Option<String>,
    pub subpath: Option<String>,
}

pub fn npm_cache_dir(base: &Path) -> PathBuf {
    base.join(".kabootar").join("npm")
}

pub fn jsr_cache_dir(base: &Path) -> PathBuf {
    base.join(".kabootar").join("jsr")
}

fn cache_root(kind: RegistryKind, base: &Path) -> PathBuf {
    match kind {
        RegistryKind::Npm => npm_cache_dir(base),
        RegistryKind::Jsr => jsr_cache_dir(base),
    }
}

/// Parse `npm:lodash@4`, `jsr:@std/fmt@1.0.0`, or bare `lodash@4`.
pub fn parse_package_spec(raw: &str) -> Result<PackageSpec, String> {
    let trimmed = raw.trim();
    let (kind, rest) = if let Some(r) = trimmed.strip_prefix("jsr:") {
        (RegistryKind::Jsr, r)
    } else if let Some(r) = trimmed.strip_prefix("npm:") {
        (RegistryKind::Npm, r)
    } else if trimmed.starts_with('@') {
        (RegistryKind::Jsr, trimmed)
    } else {
        (RegistryKind::Npm, trimmed)
    };

    let (name_part, version) = split_name_and_version(rest);
    let (name, subpath) = split_name_subpath(&name_part);
    if name.is_empty() {
        return Err(format!("Invalid package spec: {raw}"));
    }
    Ok(PackageSpec {
        kind,
        name,
        version,
        subpath,
    })
}

fn split_name_subpath(name: &str) -> (String, Option<String>) {
    if let Some((base, sub)) = name.split_once('/') {
        if base.starts_with('@') {
            if let Some((scope, pkg)) = base.split_once('/') {
                if !scope.is_empty() && !pkg.is_empty() {
                    return (format!("{scope}/{pkg}"), Some(sub.to_string()));
                }
            }
        } else if !base.is_empty() && !sub.is_empty() {
            return (base.to_string(), Some(sub.to_string()));
        }
    }
    (name.to_string(), None)
}

fn split_name_and_version(rest: &str) -> (String, Option<String>) {
    if rest.starts_with('@') {
        for (idx, _) in rest.match_indices('@').skip(1) {
            let ver = &rest[idx + 1..];
            if looks_like_version(ver) {
                return (rest[..idx].to_string(), Some(ver.to_string()));
            }
        }
        return (rest.to_string(), None);
    }
    if let Some(idx) = rest.rfind('@') {
        let ver = &rest[idx + 1..];
        if looks_like_version(ver) {
            return (rest[..idx].to_string(), Some(ver.to_string()));
        }
    }
    (rest.to_string(), None)
}

fn looks_like_version(s: &str) -> bool {
    let s = s
        .trim()
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches('v');
    if s.is_empty() {
        return false;
    }
    if s == "latest" || s == "*" {
        return true;
    }
    s.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
}

fn encode_package_name(name: &str) -> String {
    if name.starts_with('@') {
        let encoded = name.replace('/', "%2F");
        encoded
    } else {
        name.to_string()
    }
}

fn registry_metadata_url(kind: RegistryKind, name: &str) -> String {
    let encoded = encode_package_name(name);
    match kind {
        RegistryKind::Npm => format!("https://registry.npmjs.org/{encoded}"),
        RegistryKind::Jsr => format!("https://npm.jsr.io/{encoded}"),
    }
}

fn fetch_registry_metadata(kind: RegistryKind, name: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (kind, name);
        return Err("npm/jsr fetch requires native runtime (not available on WASM)".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = registry_metadata_url(kind, name);
        let trust = crate::runtime::tls_trust::TlsTrust::default();
        let bytes = crate::runtime::net::http_fetch_bytes(
            "GET",
            &url,
            "",
            &HashMap::new(),
            &trust,
            30_000,
        )?;
        String::from_utf8(bytes).map_err(|e| format!("registry metadata not UTF-8: {e}"))
    }
}

fn fetch_tarball(url: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        return Err("npm/jsr fetch requires native runtime (not available on WASM)".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let trust = crate::runtime::tls_trust::TlsTrust::default();
        crate::runtime::net::http_fetch_bytes(
            "GET",
            url,
            "",
            &HashMap::new(),
            &trust,
            60_000,
        )
    }
}

fn json_string_field(json: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let start = json.find(&needle)? + needle.len();
    let tail = json[start..].trim_start();
    if !tail.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in tail[1..].chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

fn json_object_keys(json: &str, object_field: &str) -> Vec<String> {
    let needle = format!("\"{object_field}\":");
    let Some(start) = json.find(&needle) else {
        return Vec::new();
    };
    let tail = json[start + needle.len()..].trim_start();
    let Some(body) = tail.strip_prefix('{') else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    let mut depth = 1i32;
    let mut i = 0usize;
    let bytes = body.as_bytes();
    while i < bytes.len() && depth > 0 {
        if bytes[i] == b'"' && depth == 1 {
            let mut j = i + 1;
            let mut escaped = false;
            let mut key = String::new();
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if escaped {
                    key.push(ch);
                    escaped = false;
                    j += 1;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    j += 1;
                    continue;
                }
                if ch == '"' {
                    keys.push(key);
                    break;
                }
                key.push(ch);
                j += 1;
            }
            i = j + 1;
            continue;
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    keys
}

fn version_block(json: &str, version: &str) -> Option<String> {
    let needle = format!("\"{version}\":");
    let start = json.find(&needle)? + needle.len();
    let tail = json[start..].trim_start();
    if !tail.starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    let mut end = 0usize;
    for (idx, ch) in tail.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = idx + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    Some(tail[..end].to_string())
}

fn resolve_version_from_metadata(metadata: &str, constraint: &str) -> Result<String, String> {
    let constraint = constraint.trim();
    if constraint.is_empty() || constraint == "latest" || constraint == "*" || constraint == "0" {
        if let Some(block) = version_block(metadata, "dist-tags") {
            if let Some(latest) = json_string_field(&block, "latest") {
                return Ok(latest);
            }
        }
    }
    let versions = json_object_keys(metadata, "versions");
    if versions.is_empty() {
        return Err("registry metadata has no versions".into());
    }
    let mut matches: Vec<String> = versions
        .into_iter()
        .filter(|v| crate::project::manifest::version_matches(v, constraint))
        .collect();
    matches.sort();
    matches
        .pop()
        .ok_or_else(|| format!("No version matches constraint {constraint}"))
}

fn tarball_url_for_version(metadata: &str, version: &str) -> Result<String, String> {
    let block = version_block(metadata, version)
        .ok_or_else(|| format!("Version {version} not found in registry metadata"))?;
    json_string_field(&block, "tarball")
        .ok_or_else(|| format!("No tarball URL for version {version}"))
}

pub fn package_dir(kind: RegistryKind, name: &str, version: &str, base: &Path) -> PathBuf {
    let safe_name = name.replace('/', "__");
    cache_root(kind, base).join(safe_name).join(version)
}

fn extract_tarball(bytes: &[u8], dest: &Path) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (bytes, dest);
        return Err("tarball extract requires native runtime".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use flate2::read::GzDecoder;
        use tar::Archive;

        if dest.exists() {
            fs::remove_dir_all(dest)
                .map_err(|e| format!("Failed to clear cache dir {}: {e}", dest.display()))?;
        }
        fs::create_dir_all(dest)
            .map_err(|e| format!("Failed to create cache dir {}: {e}", dest.display()))?;
        let decoder = GzDecoder::new(bytes);
        let mut archive = Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| format!("Failed to extract npm tarball: {e}"))?;
        Ok(())
    }
}

fn package_root_dir(install_dir: &Path) -> PathBuf {
    let nested = install_dir.join("package");
    if nested.is_dir() {
        nested
    } else {
        install_dir.to_path_buf()
    }
}

fn read_package_main(pkg_root: &Path) -> Result<PathBuf, String> {
    let pkg_json = pkg_root.join("package.json");
    if pkg_json.is_file() {
        let text = fs::read_to_string(&pkg_json)
            .map_err(|e| format!("Failed to read {}: {e}", pkg_json.display()))?;
        let main = json_string_field(&text, "main")
            .or_else(|| json_string_field(&text, "module"))
            .unwrap_or_else(|| "index.js".into());
        return Ok(pkg_root.join(main));
    }
    for candidate in ["mod.ts", "index.ts", "index.js", "index.mjs"] {
        let path = pkg_root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "No entry file found in package {}",
        pkg_root.display()
    ))
}

pub fn resolve_entry_path(
    kind: RegistryKind,
    name: &str,
    version: &str,
    subpath: Option<&str>,
    base: &Path,
) -> Result<PathBuf, String> {
    let install_dir = package_dir(kind, name, version, base);
    if !install_dir.is_dir() {
        return Err(format!(
            "Package {name}@{version} not in {} cache",
            match kind {
                RegistryKind::Npm => "npm",
                RegistryKind::Jsr => "jsr",
            }
        ));
    }
    let pkg_root = package_root_dir(&install_dir);
    if let Some(sub) = subpath {
        let path = pkg_root.join(sub);
        if path.is_file() {
            return Ok(path);
        }
        for ext in ["", ".ts", ".js", ".mjs"] {
            let with_ext = if ext.is_empty() {
                path.clone()
            } else {
                path.with_extension(&ext[1..])
            };
            if with_ext.is_file() {
                return Ok(with_ext);
            }
        }
        return Err(format!("Subpath not found: {sub}"));
    }
    read_package_main(&pkg_root)
}

pub fn fetch_remote_package(
    kind: RegistryKind,
    name: &str,
    constraint: &str,
    base: &Path,
) -> Result<PackageInfo, String> {
    let metadata = fetch_registry_metadata(kind, name)?;
    let version = resolve_version_from_metadata(&metadata, constraint)?;
    let install_dir = package_dir(kind, name, &version, base);
    let pkg_root = package_root_dir(&install_dir);
    if pkg_root.is_dir() && read_package_main(&pkg_root).is_ok() {
        return Ok(PackageInfo {
            name: name.to_string(),
            version,
        });
    }
    let tarball_url = tarball_url_for_version(&metadata, &version)?;
    let bytes = fetch_tarball(&tarball_url)?;
    extract_tarball(&bytes, &install_dir)?;
    Ok(PackageInfo {
        name: name.to_string(),
        version,
    })
}

pub fn fetch_npm_package(name: &str, constraint: &str, base: &Path) -> Result<PackageInfo, String> {
    fetch_remote_package(RegistryKind::Npm, name, constraint, base)
}

pub fn fetch_jsr_package(name: &str, constraint: &str, base: &Path) -> Result<PackageInfo, String> {
    fetch_remote_package(RegistryKind::Jsr, name, constraint, base)
}

pub fn read_cached_source(
    kind: RegistryKind,
    name: &str,
    version: &str,
    subpath: Option<&str>,
    base: &Path,
) -> Result<String, String> {
    let path = resolve_entry_path(kind, name, version, subpath, base)?;
    fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

pub fn list_remote_cache(base: &Path) -> Result<Vec<(RegistryKind, PackageInfo)>, String> {
    let mut out = Vec::new();
    for (kind, root) in [
        (RegistryKind::Npm, npm_cache_dir(base)),
        (RegistryKind::Jsr, jsr_cache_dir(base)),
    ] {
        if !root.is_dir() {
            continue;
        }
        for name_entry in fs::read_dir(&root).map_err(|e| format!("read {}: {e}", root.display()))?
        {
            let name_entry = name_entry.map_err(|e| format!("read cache entry: {e}"))?;
            if !name_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let safe_name = name_entry.file_name().to_string_lossy().to_string();
            let name = safe_name.replace("__", "/");
            for ver_entry in
                fs::read_dir(name_entry.path()).map_err(|e| format!("read versions: {e}"))?
            {
                let ver_entry = ver_entry.map_err(|e| format!("read version entry: {e}"))?;
                if !ver_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let version = ver_entry.file_name().to_string_lossy().to_string();
                let pkg_root = package_root_dir(&ver_entry.path());
                if pkg_root.is_dir() {
                    out.push((
                        kind,
                        PackageInfo {
                            name: name.clone(),
                            version,
                        },
                    ));
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.1.name
            .cmp(&b.1.name)
            .then(a.1.version.cmp(&b.1.version))
            .then((a.0 as u8).cmp(&(b.0 as u8)))
    });
    Ok(out)
}

pub fn resolve_cached_version(
    kind: RegistryKind,
    name: &str,
    constraint: &str,
    base: &Path,
) -> Option<String> {
    let safe_name = name.replace('/', "__");
    let root = cache_root(kind, base).join(safe_name);
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
    let mut matches: Vec<String> = versions
        .into_iter()
        .filter(|v| crate::project::manifest::version_matches(v, constraint))
        .collect();
    matches.sort();
    matches.pop()
}

/// `sha256-<hex>` integrity for a cached package directory (uses `package.json` bytes).
#[cfg(not(target_arch = "wasm32"))]
pub fn package_integrity_sha256(kind: RegistryKind, name: &str, version: &str, base: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};

    let install_dir = package_dir(kind, name, version, base);
    let pkg_root = package_root_dir(&install_dir);
    let pkg_json = pkg_root.join("package.json");
    let bytes = fs::read(pkg_json).ok()?;
    let digest = Sha256::digest(&bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Some(format!("sha256-{hex}"))
}

#[cfg(target_arch = "wasm32")]
pub fn package_integrity_sha256(
    _kind: RegistryKind,
    _name: &str,
    _version: &str,
    _base: &Path,
) -> Option<String> {
    None
}

/// Resolve a manifest dependency against npm/JSR cache when available.
pub fn resolve_lock_package(
    name: &str,
    constraint: &str,
    base: &Path,
) -> crate::project::lockfile::LockPackage {
    use crate::project::lockfile::LockPackage;

    for kind in [RegistryKind::Npm, RegistryKind::Jsr] {
        if let Some(version) = resolve_cached_version(kind, name, constraint, base) {
            let integrity = package_integrity_sha256(kind, name, &version, base);
            return LockPackage {
                version,
                integrity,
            };
        }
    }
    LockPackage {
        version: constraint.to_string(),
        integrity: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_npm_spec_with_version() {
        let spec = parse_package_spec("npm:lodash@4.17.21").unwrap();
        assert_eq!(spec.kind, RegistryKind::Npm);
        assert_eq!(spec.name, "lodash");
        assert_eq!(spec.version.as_deref(), Some("4.17.21"));
    }

    #[test]
    fn parse_jsr_scoped_spec() {
        let spec = parse_package_spec("jsr:@std/fmt@1.0.0").unwrap();
        assert_eq!(spec.kind, RegistryKind::Jsr);
        assert_eq!(spec.name, "@std/fmt");
        assert_eq!(spec.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn parse_scoped_without_version() {
        let spec = parse_package_spec("jsr:@std/fmt").unwrap();
        assert_eq!(spec.name, "@std/fmt");
        assert!(spec.version.is_none());
    }

    #[test]
    fn resolve_version_from_metadata_json() {
        let json = r#"{
            "dist-tags": { "latest": "2.0.0" },
            "versions": {
                "1.0.0": { "dist": { "tarball": "https://example/1.tgz" } },
                "2.0.0": { "dist": { "tarball": "https://example/2.tgz" } }
            }
        }"#;
        assert_eq!(resolve_version_from_metadata(json, "1").unwrap(), "1.0.0");
        assert_eq!(resolve_version_from_metadata(json, "latest").unwrap(), "2.0.0");
        let url = tarball_url_for_version(json, "2.0.0").unwrap();
        assert_eq!(url, "https://example/2.tgz");
    }

    #[test]
    fn seed_cache_read_roundtrip() {
        let base = std::env::temp_dir().join(format!(
            "kabootar_npm_cache_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        let install = package_dir(RegistryKind::Npm, "math-lite", "1.0.0", &base);
        let pkg = install.join("package");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"math-lite","main":"index.js"}"#,
        )
        .unwrap();
        fs::write(
            pkg.join("index.js"),
            "pub fn twice(x) { return x + x }",
        )
        .unwrap();
        let text = read_cached_source(
            RegistryKind::Npm,
            "math-lite",
            "1.0.0",
            None,
            &base,
        )
        .unwrap();
        assert!(text.contains("twice"));
        let _ = fs::remove_dir_all(&base);
    }
}
