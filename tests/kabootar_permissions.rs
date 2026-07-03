//! Kabootar OS permissions, hotplug, and host bridge

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::path::PathBuf;
use std::sync::Mutex;

static HOTPLUG_TEST_LOCK: Mutex<()> = Mutex::new(());

fn eval(code: &str) -> Result<Value, String> {
    let mut env = create_global_env();
    eval_source(code, &mut env).map_err(|e| e.to_string())
}

fn eval_ok(code: &str) -> Value {
    eval(code).unwrap()
}

fn eval_hotplug(code: &str) -> Value {
    let _lock = HOTPLUG_TEST_LOCK.lock().unwrap();
    eval_ok(code)
}

#[test]
fn os_caps_include_permissions_and_hotplug() {
    let caps = eval_ok("os_caps()");
    assert!(matches!(caps, Value::Array(list) if
        list.iter().any(|v| matches!(v, Value::String(s) if s == "permissions")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "hotplug")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "host-bridge"))
    ));
}

#[test]
fn sandboxed_process_denied_without_grant() {
    let err = eval(
        r#"
        let pid = os_spawn("worker");
        os_perm_clear(pid);
        os_set_subject(pid);
        os_dev_open("gpu-0");
        "#,
    );
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("permission denied"));
}

#[test]
fn grant_allows_device_open() {
    let ok = eval(
        r#"
        let pid = os_spawn("app");
        os_perm_clear(pid);
        os_perm_grant(pid, "device:gpu-0");
        os_set_subject(pid);
        os_dev_open("gpu-0");
        "#,
    );
    assert!(matches!(ok, Ok(Value::Number(n)) if n >= 1));
}

#[test]
fn vfs_read_requires_capability() {
    let err = eval(
        r#"
        os_mkdir("/secure");
        os_write("/secure/secret.txt", "x");
        let pid = os_spawn("reader");
        os_perm_clear(pid);
        os_set_subject(pid);
        os_read("/secure/secret.txt");
        "#,
    );
    assert!(err.is_err());
    let ok = eval(
        r#"
        os_mkdir("/secure2");
        os_write("/secure2/secret.txt", "x");
        let pid = os_spawn("reader2");
        os_perm_clear(pid);
        os_perm_grant(pid, "vfs:read:/secure2");
        os_set_subject(pid);
        os_read("/secure2/secret.txt");
        "#,
    );
    assert!(matches!(ok, Ok(Value::String(s)) if s == "x"));
}

#[test]
fn hotplug_register_emits_event() {
    let events = eval_hotplug(
        r#"
        os_hotplug_register("Acme", "Webcam", "hid");
        os_hotplug_poll();
        "#,
    );
    assert!(matches!(events, Value::Array(list) if
        list.iter().any(|e| matches!(e, Value::Object(o) if
            o.get("action").and_then(|v| match v {
                Value::String(s) => Some(s == "add"),
                _ => None,
            }).unwrap_or(false) &&
            o.get("name").and_then(|v| match v {
                Value::String(s) => Some(s == "Webcam"),
                _ => None,
            }).unwrap_or(false)
        ))
    ));
}

#[test]
fn kb_poll_hotplug_matches_os_bus() {
    let events = eval_hotplug(
        r#"
        os_hotplug_register("Kabootar", "Mic", "hid");
        kb_poll_hotplug();
        "#,
    );
    assert!(matches!(events, Value::Array(list) if !list.is_empty()));
}

#[test]
fn host_bridge_info_available() {
    let info = eval_ok("os_host_info()");
    assert!(matches!(info, Value::Object(o) if o.contains_key("enabled")));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn host_audio_pcm_file_when_bridge_enabled() {
    let path = PathBuf::from(std::env::temp_dir()).join("kabootar-perm-test-audio.pcm");
    let _ = std::fs::remove_file(&path);
    std::env::set_var("KABOOTAR_HOST_BRIDGE", "1");
    std::env::set_var("KABOOTAR_HOST_AUDIO", path.to_string_lossy().to_string());

    let _ = eval_ok(
        r#"
        let h = os_dev_open("audio-out-0");
        os_dev_ioctl(h, "write", [100, 200, 300, 400]);
        "#,
    );

    let meta = std::fs::metadata(&path).expect("host pcm file");
    assert!(meta.len() >= 8);

    std::env::remove_var("KABOOTAR_HOST_BRIDGE");
    std::env::remove_var("KABOOTAR_HOST_AUDIO");
    let _ = std::fs::remove_file(&path);
}
