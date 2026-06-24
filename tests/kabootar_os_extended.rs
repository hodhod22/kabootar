//! OS extended features — VFS, TCP/UDP, memory safety, bytecode optimize

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::Value;
use std::path::PathBuf;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

fn host_mount_dir() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target-local");
    p.push("kabootar-host-mount");
    std::fs::create_dir_all(&p).ok();
    p.to_string_lossy().into_owned()
}

#[test]
fn vfs_rename_copy_and_extended_stat() {
    let out = eval(
        r#"
        os_write("/a.txt", "alpha");
        os_copy("/a.txt", "/b.txt");
        os_rename("/b.txt", "/c.txt");
        os_read("/c.txt")
        "#,
    );
    assert!(matches!(out, Value::String(s) if s == "alpha"));
    let stat = eval(
        r#"
        os_write("/z.txt", "z");
        os_stat("/z.txt")
        "#,
    );
    assert!(matches!(stat, Value::Array(a) if a.len() >= 4));
}

#[test]
fn vfs_host_mount_roundtrip() {
    let host = host_mount_dir();
    let _ = eval(&format!(
        r#"
        os_perm_grant(os_subject(), "perm:admin");
        os_mount("/host", "{host}");
        os_write("/host/mount-test.txt", "from-kabootar");
        os_read("/host/mount-test.txt");
        "#
    ));
    let path = PathBuf::from(&host).join("mount-test.txt");
    let data = std::fs::read_to_string(path).unwrap();
    assert_eq!(data, "from-kabootar");
}

#[test]
fn memory_guarded_read_write() {
    let bytes = eval(
        r#"
        let id = os_mem_alloc(8, "test");
        os_mem_write(id, 0, [72, 105]);
        os_mem_read(id, 0, 2);
        "#,
    );
    assert!(matches!(bytes, Value::Array(a) if
        a.len() == 2 &&
        matches!(a[0], Value::Number(72)) &&
        matches!(a[1], Value::Number(105))
    ));
    let freed = eval(
        r#"
        let id = os_mem_alloc(4, "tmp");
        os_mem_free(id);
        "#,
    );
    assert!(matches!(freed, Value::Bool(true)));
}

#[test]
fn net_listen_poll_loopback_ioctl() {
    let sock = eval(
        r#"
        let h = os_dev_open("net-eth0");
        os_dev_ioctl(h, "listen", "loopback", 0);
        "#,
    );
    assert!(matches!(sock, Value::Number(n) if n >= 1));
    let poll = eval(
        r#"
        let h = os_dev_open("net-eth0");
        let l = os_dev_ioctl(h, "listen", "loopback", 0);
        os_dev_ioctl(h, "poll", [l]);
        "#,
    );
    assert!(matches!(poll, Value::Array(_)));
}

#[test]
fn net_udp_bind_ioctl() {
    let sock = eval(
        r#"
        let h = os_dev_open("net-eth0");
        os_dev_ioctl(h, "udp_bind", "0.0.0.0", 19000);
        "#,
    );
    assert!(matches!(sock, Value::Number(n) if n >= 1));
}

#[test]
fn bytecode_opt_info_after_fold() {
    let info = eval("bytecode_opt_info(\"let x = 1 + 2\nx\")");
    assert!(matches!(info, Value::Object(o) if
        o.get("optimized").and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        }).unwrap_or(false)
    ));
    let result = eval("let x = 1 + 2\nx");
    assert!(matches!(result, Value::Number(3)));
}

#[test]
fn os_caps_include_extended_features() {
    let caps = eval("os_caps()");
    assert!(matches!(caps, Value::Array(list) if
        list.iter().any(|v| matches!(v, Value::String(s) if s == "vfs-extended")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "net-tcp-full")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "memory-safe")) &&
        list.iter().any(|v| matches!(v, Value::String(s) if s == "bytecode-optimize"))
    ));
}
