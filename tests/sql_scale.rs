//! Kabootar SQL scale engine — RowStore, KDB2, ANALYZE, MVCC, planner

use kabootar::sql::{is_binary_kdb, SqlEngine};
use kabootar::value::Value;
use std::fs;

fn temp_path(name: &str) -> String {
    let dir = std::env::temp_dir();
    dir.join(format!("kabootar_scale_{name}_{}.kdb2", std::process::id()))
        .to_string_lossy()
        .into_owned()
}

#[test]
fn sql_scale_kdb2_roundtrip() {
    let path = temp_path("kdb2");
    let _ = fs::remove_file(&path);

    let mut e = SqlEngine::new();
    e.execute(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)",
        &[],
    )
    .unwrap();
    e.execute("INSERT INTO items (id, name) VALUES (1, 'alpha')", &[])
        .unwrap();
    e.execute("INSERT INTO items (id, name) VALUES (2, 'beta')", &[])
        .unwrap();
    e.execute(&format!("SAVE DATABASE '{path}'"), &[]).unwrap();

    assert!(is_binary_kdb(&path));

    let mut e2 = SqlEngine::new();
    e2.execute(&format!("LOAD DATABASE '{path}'"), &[]).unwrap();
    let v = e2
        .execute("SELECT name FROM items WHERE id = 2", &[])
        .unwrap();
    assert!(matches!(v, Value::String(s) if s == "beta"));

    let _ = fs::remove_file(&path);
}

#[test]
fn sql_scale_analyze_and_explain() {
    let mut e = SqlEngine::new();
    e.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT)",
        &[],
    )
    .unwrap();
    for i in 1..=20 {
        e.execute(
            &format!("INSERT INTO users (id, email) VALUES ({i}, 'u{i}@x.c')"),
            &[],
        )
        .unwrap();
    }
    e.execute("CREATE INDEX idx_email ON users (email)", &[])
        .unwrap();
    e.execute("ANALYZE users", &[]).unwrap();

    let plan = e
        .execute("EXPLAIN SELECT email FROM users WHERE id = 5", &[])
        .unwrap();
    assert!(matches!(
        plan,
        Value::Object(obj) if obj.get("plan").and_then(|v| match v {
            Value::String(s) => Some(s.contains("Index Scan")),
            _ => None
        }).unwrap_or(false)
    ));
}

#[test]
fn sql_scale_mvcc_rollback_hides_inserts() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, n INTEGER)", &[])
        .unwrap();
    e.execute("BEGIN", &[]).unwrap();
    e.execute("INSERT INTO t (id, n) VALUES (1, 10)", &[]).unwrap();
    e.execute("ROLLBACK", &[]).unwrap();
    let c = e.execute("SELECT COUNT(*) FROM t", &[]).unwrap();
    assert!(matches!(c, Value::Number(0)));
}

#[test]
fn sql_scale_batch_insert_and_compact() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE logs (id INTEGER PRIMARY KEY, msg TEXT)", &[])
        .unwrap();
    e.execute(
        "INSERT INTO logs (id, msg) VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        &[],
    )
    .unwrap();
    e.execute("DELETE FROM logs WHERE id = 2", &[]).unwrap();
    let c = e.execute("SELECT COUNT(*) FROM logs", &[]).unwrap();
    assert!(matches!(c, Value::Number(2)));
}

#[test]
fn sql_scale_checkpoint_binary() {
    let path = temp_path("chk");
    let _ = fs::remove_file(&path);

    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[]).unwrap();
    e.execute("INSERT INTO t (id) VALUES (1)", &[]).unwrap();
    e.execute(&format!("SAVE DATABASE '{path}'"), &[]).unwrap();
    e.execute("CHECKPOINT", &[]).unwrap();

    let mut e2 = SqlEngine::new();
    e2.execute(&format!("LOAD DATABASE '{path}'"), &[]).unwrap();
    let v = e2.execute("SELECT id FROM t WHERE id = 1", &[]).unwrap();
    assert!(matches!(v, Value::Number(1)));

    let _ = fs::remove_file(&path);
}
