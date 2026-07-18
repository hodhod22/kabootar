//! lib/kos — kOS desktop shell (G12.1–G12.5 + boot).

use kabootar_lib::cli;
use kabootar_lib::value::Value;

fn manifest_dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

#[test]
fn kos_g12_shell_smoke() {
    let path = format!("{}/examples/kos_g12_shell_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_g12_shell_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_g12_2_start_smoke() {
    let path = format!("{}/examples/kos_g12_2_start_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_g12_2_start_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_g12_3_explorer_smoke() {
    let path = format!("{}/examples/kos_g12_3_explorer_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_g12_3_explorer_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_g12_4_windows_smoke() {
    let path = format!("{}/examples/kos_g12_4_windows_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_g12_4_windows_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_g12_5_theme_smoke() {
    let path = format!("{}/examples/kos_g12_5_theme_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_g12_5_theme_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_shell_boot_smoke() {
    let path = format!("{}/examples/kos_shell_boot.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_shell_boot.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_shell_mount_smoke() {
    let path = format!("{}/examples/kos_shell_mount_smoke.kab", manifest_dir());
    let result = cli::run_file(&path).expect("examples/kos_shell_mount_smoke.kab should run");
    assert!(matches!(result, Value::Bool(true)), "got {result:?}");
}

#[test]
fn kos_launch_app_smoke() {
    let path = format!("{}/examples/kos_launch_app_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("kos_launch_app_smoke".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = cli::run_file(&path).expect("examples/kos_launch_app_smoke.kab should run");
            matches!(result, Value::Bool(true))
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "launch Start → openWindow smoke failed");
}

#[test]
fn kos_start_click_smoke() {
    let path = format!("{}/examples/kos_start_click_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("kos_start_click_smoke".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = cli::run_file(&path).expect("examples/kos_start_click_smoke.kab should run");
            matches!(result, Value::Bool(true))
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "Start click → launchApp smoke failed");
}

#[test]
fn kos_event_drain_smoke() {
    let path = format!("{}/examples/kos_event_drain_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("kos_event_drain_smoke".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = cli::run_file(&path).expect("examples/kos_event_drain_smoke.kab should run");
            matches!(result, Value::Bool(true))
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "dispatch → drainKosEvents smoke failed");
}

#[test]
fn kos_app_body_smoke() {
    let path = format!("{}/examples/kos_app_body_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("kos_app_body_smoke".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = cli::run_file(&path).expect("examples/kos_app_body_smoke.kab should run");
            matches!(result, Value::Bool(true))
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "launchApp VFS body smoke failed");
}

#[test]
fn kos_host_click_smoke() {
    let path = format!("{}/examples/kos_host_click_smoke.kab", manifest_dir());
    let ok = std::thread::Builder::new()
        .name("kos_host_click_smoke".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let result = cli::run_file(&path).expect("examples/kos_host_click_smoke.kab should run");
            matches!(result, Value::Bool(true))
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(ok, "host click path (dispatch+drain+remount) smoke failed");
}

#[test]
fn kos_shell_build_and_list_apps() {
    let code = r#"
import "kos/shell"
import "kdom/document"
os_mkdir("/apps")
os_write("/apps/a.app", "x")
let desktop = buildShell()
let bar = findTaskbar(desktop)
let apps = listApps()
bar != null && len(apps) >= 1
"#;
    let mut env = kabootar_lib::evaluator::create_global_env();
    let v = kabootar_lib::evaluator::eval_source(code, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
