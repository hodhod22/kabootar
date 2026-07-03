//! Kabootar OS driver stack — GPU, network, USB, audio

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn os_caps_include_drivers() {
    let caps = eval("os_caps()");
    assert!(matches!(caps, Value::Array(list) if
        list.iter().any(|v| matches!(v, Value::String(s) if s == "device-manager")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "gpu-driver")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "net-driver")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "usb-driver")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "audio-driver"))
    ));
}

#[test]
fn os_dev_list_enumerates_driver_devices() {
    let list = eval("os_dev_list()");
    assert!(matches!(list, Value::Array(devs) if
        devs.iter().any(|d| matches!(d, Value::Object(o) if
            o.get("id").and_then(|v| match v {
                Value::String(s) => Some(s == "gpu-0"),
                _ => None,
            }).unwrap_or(false)
        )) &&
        devs.iter().any(|d| matches!(d, Value::Object(o) if
            o.get("kind").and_then(|v| match v {
                Value::String(s) => Some(s == "usb"),
                _ => None,
            }).unwrap_or(false)
        ))
    ));
}

#[test]
fn os_gpu_info_and_present() {
    let info = eval("os_gpu_info()");
    assert!(matches!(info, Value::Object(o) if
        o.get("width").and_then(|v| match v { Value::Number(n) => Some(*n > 0), _ => None }).unwrap_or(false)
    ));
    let tex = eval(
        r#"
        let h = os_dev_open("gpu-0");
        os_dev_ioctl(h, "present", 640);
        "#,
    );
    assert!(matches!(tex, Value::Number(n) if n >= 1));
}

#[test]
fn os_net_interfaces_and_loopback_socket() {
    let ifaces = eval("os_net_interfaces()");
    assert!(matches!(ifaces, Value::Array(list) if !list.is_empty()));

    // WASM uses simulated socket; native may connect to real host — test ioctl path only.
    let sock = eval(
        r#"
        let h = os_dev_open("net-eth0");
        os_dev_ioctl(h, "connect", "loopback", 0);
        "#,
    );
    assert!(matches!(sock, Value::Number(n) if n >= 1));
}

#[test]
fn os_usb_hid_transfer() {
    let keys = eval(
        r#"
        let h = os_dev_open("usb-hid-0");
        os_dev_ioctl(h, "transfer", "in");
        "#,
    );
    assert!(matches!(keys, Value::Array(a) if !a.is_empty()));
}

#[test]
fn os_usb_mass_storage_sector() {
    let sector = eval(
        r#"
        let h = os_dev_open("usb-ms-0");
        os_dev_ioctl(h, "transfer", "out", [1, 0]);
        "#,
    );
    assert!(matches!(sector, Value::Array(a) if a.len() == 512));
}

#[test]
fn os_audio_write_pcm() {
    let n = eval(
        r#"
        let h = os_dev_open("audio-out-0");
        os_dev_ioctl(h, "write", [1000, -1000, 500, -500]);
        "#,
    );
    assert!(matches!(n, Value::Number(c) if c == 4));
}

#[test]
fn os_syscalls_include_driver_ops() {
    let list = eval("os_syscalls()");
    assert!(matches!(list, Value::Array(s) if
        s.iter().any(|v| matches!(v, Value::String(n) if n == "gpu_info")) &&
        s.iter().any(|v| matches!(v, Value::String(n) if n == "usb_list"))
    ));
}

#[test]
fn os_syscall_gpu_info() {
    let info = eval(r#"os_syscall("gpu_info")"#);
    assert!(matches!(info, Value::Object(_)));
}

#[test]
fn os_usb_hotplug_via_native() {
    let list = eval("os_usb_devices()");
    assert!(matches!(list, Value::Array(devs) if devs.len() >= 3));
}
