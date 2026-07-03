//! Ecosystem — registry search, catalog, seed, uninstall.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::registry::{install_package, list_registry, publish_file, seed_lib_to_registry};
use kabootar_lib::value::Value;
use std::fs;
use std::process;
use std::sync::Mutex;

static PROJECT_ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_project() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kabootar_eco_{}_{}",
        process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("lib")).unwrap();
    dir
}

struct ProjectGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev_root: Option<String>,
    prev_cwd: Option<std::path::PathBuf>,
}

impl ProjectGuard {
    fn chdir(dir: &std::path::Path) -> Self {
        let lock = PROJECT_ENV_LOCK.lock().unwrap();
        let prev_root = std::env::var("KABOOTAR_PROJECT_ROOT").ok();
        let prev_cwd = std::env::current_dir().ok();
        std::env::set_var(
            "KABOOTAR_PROJECT_ROOT",
            dir.to_string_lossy().to_string(),
        );
        let _ = std::env::set_current_dir(dir);
        Self {
            _lock: lock,
            prev_root,
            prev_cwd,
        }
    }
}

impl Drop for ProjectGuard {
    fn drop(&mut self) {
        if let Some(root) = &self.prev_root {
            std::env::set_var("KABOOTAR_PROJECT_ROOT", root);
        } else {
            std::env::remove_var("KABOOTAR_PROJECT_ROOT");
        }
        if let Some(cwd) = &self.prev_cwd {
            let _ = std::env::set_current_dir(cwd);
        }
    }
}

fn eval_in(dir: &std::path::Path, code: &str) -> Value {
    let _guard = ProjectGuard::chdir(dir);
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn ecosystem_info_counts_modules() {
    let dir = temp_project();
    fs::write(
        dir.join("lib/demo.kab"),
        r#"@version "1.0.0"
pub fn demo() { return 1 }
"#,
    )
    .unwrap();

    let out = eval_in(&dir, "ecosystem_info()");
    let Value::Object(info) = out else {
        panic!("expected object");
    };
    assert!(matches!(
        info.get("lib_modules"),
        Some(Value::Number(n)) if *n >= 1
    ));
    assert!(matches!(
        info.get("builtin_modules"),
        Some(Value::Number(n)) if *n >= 10
    ));
}

#[test]
fn modules_catalog_includes_builtin_and_lib() {
    let dir = temp_project();
    fs::write(
        dir.join("lib/demo.kab"),
        r#"@version "1.0.0"
pub fn demo() { return 1 }
"#,
    )
    .unwrap();

    let out = eval_in(&dir, "modules_catalog()");
    let Value::Array(items) = out else {
        panic!("expected array");
    };
    let has_std = items.iter().any(|v| {
        matches!(
            v,
            Value::Object(m)
                if matches!(m.get("name"), Some(Value::String(s)) if s == "std")
                    && matches!(m.get("source"), Some(Value::String(s)) if s == "builtin")
        )
    });
    let has_demo = items.iter().any(|v| {
        matches!(
            v,
            Value::Object(m)
                if matches!(m.get("name"), Some(Value::String(s)) if s == "demo")
                    && matches!(m.get("source"), Some(Value::String(s)) if s == "lib")
        )
    });
    assert!(has_std && has_demo);
}

#[test]
fn registry_seed_publishes_lib_packages() {
    let dir = temp_project();
    fs::write(
        dir.join("lib/greet.kab"),
        r#"@version "1.0.0"
pub fn hi() { return "hi" }
"#,
    )
    .unwrap();

    let published = seed_lib_to_registry(&dir).unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].name, "greet");
    let list = list_registry(&dir).unwrap();
    assert_eq!(list.len(), 1);
}

#[test]
fn registry_search_finds_greet() {
    let dir = temp_project();
    let src = dir.join("lib/greet.kab");
    fs::write(
        &src,
        r#"@version "1.0.0"
pub fn hi() { return "hi" }
"#,
    )
    .unwrap();
    publish_file(&src, &dir).unwrap();

    let out = eval_in(&dir, r#"registry_search("greet")"#);
    let Value::Array(hits) = out else {
        panic!("expected array");
    };
    assert!(!hits.is_empty());
}

#[test]
fn import_json_builtin_module() {
    let mut env = create_global_env();
    import_module("json", &mut env).unwrap();
    let out = eval_source(r#"parse(`{"ok":true}`)"#, &mut env).unwrap();
    let Value::Object(o) = out else {
        panic!("expected object");
    };
    assert!(o.contains_key("ok"));
}

#[test]
fn install_and_uninstall_roundtrip() {
    let dir = temp_project();
    let src = dir.join("pkg.kab");
    fs::write(
        &src,
        r#"@version "1.0.0"
pub fn v() { return 1 }
"#,
    )
    .unwrap();
    publish_file(&src, &dir).unwrap();
    install_package("pkg", "1.0", &dir).unwrap();

    let out = eval_in(&dir, r#"registry_uninstall("pkg", "1.0")"#);
    let Value::Object(m) = out else {
        panic!("expected object");
    };
    assert!(matches!(m.get("name"), Some(Value::String(s)) if s == "pkg"));
}

#[test]
fn pagination_lib_module_loads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let _guard = ProjectGuard::chdir(root);
    let mut env = create_global_env();
    import_module("pagination", &mut env).unwrap();
    let out = eval_source("page([1, 2, 3, 4], 1, 2)", &mut env).unwrap();
    let Value::Object(o) = out else {
        panic!("expected object");
    };
    let Value::Array(xs) = o.get("items").expect("items") else {
        panic!("expected array");
    };
    assert_eq!(xs.len(), 2);
}
