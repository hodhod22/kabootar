//! Kabootar SQL v3 — advanced features tests

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::sql::SqlEngine;
use kabootar_lib::value::Value;
use std::collections::HashMap;

#[test]
fn sql_v3_json_path_and_contains() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE docs (id INTEGER PRIMARY KEY, body JSONB)", &[])
        .unwrap();
    let mut body = HashMap::new();
    body.insert("title".into(), Value::String("hi".into()));
    body.insert("plan".into(), Value::String("pro".into()));
    e.execute("INSERT INTO docs (id, body) VALUES (1, $1)", &[Value::Object(body)])
        .unwrap();
    let v = e
        .execute("SELECT body FROM docs WHERE body->>'title' = 'hi'", &[])
        .unwrap();
    assert!(matches!(v, Value::Object(_)));
    let mut probe = HashMap::new();
    probe.insert("plan".into(), Value::String("pro".into()));
    let v2 = e
        .execute("SELECT id FROM docs WHERE body @> $1", &[Value::Object(probe)])
        .unwrap();
    assert!(matches!(v2, Value::Number(1)));
}

#[test]
fn sql_v3_alter_table() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[]).unwrap();
    e.execute("ALTER TABLE t ADD COLUMN name TEXT", &[]).unwrap();
    e.execute("INSERT INTO t (id, name) VALUES (1, 'Ada')", &[])
        .unwrap();
    e.execute("ALTER TABLE t RENAME COLUMN name TO full_name", &[])
        .unwrap();
    let v = e
        .execute("SELECT full_name FROM t WHERE id = 1", &[])
        .unwrap();
    assert!(matches!(v, Value::String(s) if s == "Ada"));
}

#[test]
fn sql_v3_upsert_on_unique_email() {
    let mut e = SqlEngine::new();
    e.execute(
        "CREATE TABLE users (id SERIAL PRIMARY KEY, email TEXT UNIQUE)",
        &[],
    )
    .unwrap();
    e.execute("INSERT INTO users (email) VALUES ('a@b.c')", &[])
        .unwrap();
    e.execute(
        "INSERT INTO users (email) VALUES ('a@b.c') ON CONFLICT (email) DO UPDATE SET email = 'a@b.c'",
        &[],
    )
    .unwrap();
    let c = e.execute("SELECT COUNT(*) FROM users", &[]).unwrap();
    assert!(matches!(c, Value::Number(1)));
}

#[test]
fn sql_v3_update_validates_not_null() {
    let mut e = SqlEngine::new();
    e.execute(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )
    .unwrap();
    e.execute("INSERT INTO t (id, name) VALUES (1, 'Ada')", &[])
        .unwrap();
    let err = e
        .execute("UPDATE t SET name = NULL WHERE id = 1", &[])
        .unwrap_err();
    assert!(err.contains("NOT NULL"));
}

#[test]
fn sql_v3_distinct_and_subquery() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE a (id INTEGER)", &[]).unwrap();
    e.execute("CREATE TABLE b (id INTEGER)", &[]).unwrap();
    e.execute("INSERT INTO a (id) VALUES (1)", &[]).unwrap();
    e.execute("INSERT INTO a (id) VALUES (1)", &[]).unwrap();
    e.execute("INSERT INTO a (id) VALUES (2)", &[]).unwrap();
    e.execute("INSERT INTO b (id) VALUES (1)", &[]).unwrap();
    let v = e.execute("SELECT DISTINCT id FROM a", &[]).unwrap();
    assert!(matches!(v, Value::Array(rows) if rows.len() == 2));
    let v2 = e
        .execute("SELECT id FROM a WHERE id IN (SELECT id FROM b)", &[])
        .unwrap();
    assert!(matches!(
        v2,
        Value::Array(rows) if !rows.is_empty() && rows.iter().all(|v| matches!(v, Value::Number(1)))
    ));
}

#[test]
fn sql_v3_batch_insert() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER, n INTEGER)", &[])
        .unwrap();
    e.execute("INSERT INTO t (id, n) VALUES (1, 10), (2, 20)", &[])
        .unwrap();
    let c = e.execute("SELECT COUNT(*) FROM t", &[]).unwrap();
    assert!(matches!(c, Value::Number(2)));
}

#[test]
fn sql_v3_between_not_ilike() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER, name TEXT)", &[])
        .unwrap();
    e.execute("INSERT INTO t (id, name) VALUES (1, 'Ada')", &[])
        .unwrap();
    e.execute("INSERT INTO t (id, name) VALUES (5, 'Bob')", &[])
        .unwrap();
    let v = e
        .execute("SELECT id FROM t WHERE id BETWEEN 2 AND 10", &[])
        .unwrap();
    assert!(matches!(v, Value::Number(5)));
    let v2 = e
        .execute("SELECT id FROM t WHERE name ILIKE 'ada'", &[])
        .unwrap();
    assert!(matches!(v2, Value::Number(1)));
    let v3 = e
        .execute("SELECT id FROM t WHERE NOT id = 1", &[])
        .unwrap();
    assert!(matches!(v3, Value::Number(5)));
}

#[test]
fn sql_v3_foreign_key_and_check() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)", &[])
        .unwrap();
    e.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), amount INTEGER CHECK (amount > 0))",
        &[],
    )
    .unwrap();
    e.execute("INSERT INTO users (id) VALUES (1)", &[]).unwrap();
    let fk_err = e
        .execute("INSERT INTO orders (id, user_id, amount) VALUES (1, 9, 5)", &[])
        .unwrap_err();
    assert!(fk_err.contains("FOREIGN KEY"));
    let chk_err = e
        .execute("INSERT INTO orders (id, user_id, amount) VALUES (2, 1, 0)", &[])
        .unwrap_err();
    assert!(chk_err.contains("CHECK"));
    e.execute("INSERT INTO orders (id, user_id, amount) VALUES (3, 1, 10)", &[])
        .unwrap();
}

#[test]
fn sql_v3_db_open_native() {
    let path = std::env::temp_dir().join("kabootar_db_open.kdb");
    let path_str = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&path);
    let mut env = create_global_env();
    eval_source(
        &format!(
            r#"
            db_open("{path_str}")
            sql("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            sql("INSERT INTO t (id, v) VALUES (1, 'x')")
            sql("CHECKPOINT")
        "#
        ),
        &mut env,
    )
    .unwrap();
    let mut env2 = create_global_env();
    eval_source(
        &format!(
            r#"
            db_open("{path_str}")
            sql("SELECT v FROM t WHERE id = 1")
        "#
        ),
        &mut env2,
    )
    .unwrap();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path_str}.wal"));
}

#[test]
fn sql_v3_explain_rich() {
    let mut e = SqlEngine::new();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[]).unwrap();
    e.execute("INSERT INTO t (id) VALUES (1)", &[]).unwrap();
    let plan = e
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1", &[])
        .unwrap();
    assert!(matches!(plan, Value::Object(obj)
        if obj.get("plan").is_some() && obj.get("rows").is_some() && obj.get("cost").is_some()));
}
