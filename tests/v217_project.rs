//! v2.17 — local package registry (publish, install, import from packages)

use kabootar_lib::cli;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::modules::import_module;
use kabootar_lib::registry::{install_package, list_registry, publish_file, resolve_installed_path};
use kabootar_lib::value::Value;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn temp_project() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kabootar_v217_{}_{}",
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

fn with_dir<F: FnOnce(&PathBuf)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = temp_project();
    let old = std::env::current_dir().ok();
    std::env::set_current_dir(&dir).unwrap();
    f(&dir);
    if let Some(prev) = old {
        let _ = std::env::set_current_dir(prev);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn publish_install_and_import_from_packages() {
    with_dir(|base| {
        let src = base.join("sources").join("hello.kab");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(
            &src,
            r#"@version "1.0.0"

pub fn hello(name) {
    return "pkg:" + name
}
"#,
        )
        .unwrap();

        publish_file(&src, base).unwrap();
        install_package("hello", "1.0", base).unwrap();
        assert!(resolve_installed_path("hello", Some("1.0"), base).is_some());

        let mut env = create_global_env();
        import_module("hello", &mut env).unwrap();
        let v = eval_source(r#"hello("world")"#, &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "pkg:world"));
    });
}

#[test]
fn registry_list_native_lists_published_packages() {
    with_dir(|base| {
        let src = base.join("sources").join("mathlib.kab");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(
            &src,
            r#"@version "2.0.0"
pub fn twice(n) { return n * 2 }
"#,
        )
        .unwrap();
        publish_file(&src, base).unwrap();

        let mut env = create_global_env();
        let v = eval_source("len(registry_list())", &mut env).unwrap();
        assert!(matches!(v, Value::Number(1)));
    });
}

#[test]
fn registry_install_native_installs_package() {
    with_dir(|base| {
        let src = base.join("sources").join("svc.kab");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(
            &src,
            r#"@version "1.2.0"
pub fn ping() { return "pong" }
"#,
        )
        .unwrap();
        publish_file(&src, base).unwrap();

        let mut env = create_global_env();
        let v = eval_source(
            r#"
            let p = registry_install("svc", "1.2")
            p["version"]
        "#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(v, Value::String(s) if s == "1.2.0"));
    });
}

#[test]
fn cli_publish_and_install_commands() {
    with_dir(|base| {
        let src = base.join("lib").join("widget.kab");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(
            &src,
            r#"@version "0.9.0"
pub fn widget() { return "w" }
"#,
        )
        .unwrap();

        assert_eq!(cli::run(&["publish".into(), "widget".into()]), 0);
        let list = list_registry(base).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "widget");

        assert_eq!(cli::run(&["install".into(), "widget@0.9".into()]), 0);
        assert!(resolve_installed_path("widget", Some("0.9"), base).is_some());
    });
}

#[test]
fn install_missing_package_fails() {
    with_dir(|base| {
        let err = install_package("missing", "1.0", base).unwrap_err();
        assert!(err.contains("not found"));
    });
}

#[test]
fn lib_module_takes_priority_over_installed_package() {
    with_dir(|base| {
        fs::create_dir_all(base.join("lib")).unwrap();
        fs::write(
            base.join("lib").join("prio.kab"),
            r#"@version "1.0.0"
pub fn prio() { return "local" }
"#,
        )
        .unwrap();

        let registry_src = base.join("sources").join("prio.kab");
        fs::create_dir_all(registry_src.parent().unwrap()).unwrap();
        fs::write(
            &registry_src,
            r#"@version "1.0.0"
pub fn prio() { return "installed" }
"#,
        )
        .unwrap();
        publish_file(&registry_src, base).unwrap();
        install_package("prio", "1.0", base).unwrap();

        let mut env = create_global_env();
        import_module("prio", &mut env).unwrap();
        let v = eval_source("prio()", &mut env).unwrap();
        assert!(matches!(v, Value::String(s) if s == "local"));
    });
}
