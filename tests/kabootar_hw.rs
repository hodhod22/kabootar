//! Native hardware (cpal + serialport + hidapi + nusb) integration tests

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn os_host_info_reports_hw_backend() {
    let info = eval("os_host_info()");
    assert!(matches!(info, Value::Object(o) if
        o.get("hw_feature").is_some() &&
        o.get("audio_backend").is_some() &&
        o.get("usb_backend").is_some() &&
        o.get("net_backend").is_some()
    ));
}

#[cfg(feature = "hw")]
#[test]
fn os_host_info_reports_full_usb_backend() {
    let info = eval("os_host_info()");
    assert!(matches!(info, Value::Object(o) if
        o.get("usb_backend").and_then(|v| match v {
            Value::String(s) => Some(s == "serialport+hidapi+nusb"),
            _ => None,
        }).unwrap_or(false)
    ));
}

#[cfg(feature = "hw")]
#[test]
fn os_hw_refresh_lists_host_devices_when_present() {
    let n = eval("os_hw_refresh()");
    assert!(matches!(n, Value::Number(_)));
    let audio = eval("os_audio_devices()");
    assert!(matches!(audio, Value::Array(_)));
    let usb = eval("os_usb_devices()");
    assert!(matches!(usb, Value::Array(_)));
}

#[cfg(feature = "hw")]
#[test]
fn virtual_audio_still_works_with_hw_feature() {
    let n = eval(
        r#"
        let h = os_dev_open("audio-out-0");
        os_dev_ioctl(h, "write", [1000, -1000, 500, -500]);
        "#,
    );
    assert!(matches!(n, Value::Number(c) if c == 4));
}

#[cfg(feature = "hw")]
#[test]
fn os_hw_refresh_does_not_panic() {
    let _ = eval("os_hw_refresh()");
    let info = eval("os_host_info()");
    assert!(matches!(info, Value::Object(o) if
        o.get("audio_backend").and_then(|v| match v {
            Value::String(s) => Some(s == "cpal"),
            _ => None,
        }).unwrap_or(false)
    ));
}
