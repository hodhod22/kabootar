//! `kabootar.toml` manifest parsing.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectManifest {
    pub version: Option<String>,
    pub template: Option<String>,
    pub entry: Option<String>,
    pub port: Option<u16>,
    pub dependencies: HashMap<String, String>,
}

pub fn load_manifest(path: &Path) -> Result<ProjectManifest, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    parse_manifest(&text)
}

pub fn load_manifest_cwd() -> Result<ProjectManifest, String> {
    let root = crate::project::root::project_root()?;
    load_manifest(&root.join("kabootar.toml"))
}

pub fn parse_manifest(text: &str) -> Result<ProjectManifest, String> {
    let mut manifest = ProjectManifest::default();
    let mut in_deps = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if line.starts_with('[') {
            in_deps = false;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = parse_toml_value(value.trim());

        if in_deps {
            manifest.dependencies.insert(key.to_string(), value);
        } else {
            match key {
                "version" => manifest.version = Some(value),
                "template" => manifest.template = Some(value),
                "entry" => manifest.entry = Some(value),
                "port" => {
                    manifest.port = Some(
                        value
                            .parse::<u16>()
                            .map_err(|_| format!("Invalid port: {value}"))?,
                    );
                }
                _ => {}
            }
        }
    }
    Ok(manifest)
}

fn parse_toml_value(raw: &str) -> String {
    raw.trim_matches('"').trim().to_string()
}

/// Compare module version against a dependency constraint (`1.0`, `1.0.0`, `^1.0`).
pub fn version_matches(module_version: &str, constraint: &str) -> bool {
    let module = normalize_version(module_version);
    let constraint = normalize_version(constraint.trim_start_matches('^'));
    if constraint.is_empty() {
        return true;
    }
    if module == constraint {
        return true;
    }
    // Allow prefix match: constraint "1.0" matches "1.0.3"
    module.starts_with(&format!("{constraint}.")) || module == constraint
}

fn normalize_version(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dependencies() {
        let text = r#"
version = "1.0.0"
entry = "main.kab"
port = 3000

[dependencies]
greet = "1.0.0"
utils = "^1.2"
"#;
        let m = parse_manifest(text).unwrap();
        assert_eq!(m.entry.as_deref(), Some("main.kab"));
        assert_eq!(m.port, Some(3000));
        assert_eq!(m.dependencies.get("greet").map(String::as_str), Some("1.0.0"));
    }

    #[test]
    fn version_prefix_match() {
        assert!(version_matches("1.0.3", "1.0"));
        assert!(!version_matches("2.0.0", "1.0"));
    }
}
