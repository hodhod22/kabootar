//! Kabootar SQL v2 — modern database features

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::sql::SqlEngine;
use kabootar_lib::value::Value;

fn eval_sql(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn sql_v2_create_table_if_not_exists() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        )
        .unwrap();
    engine
        .execute("INSERT INTO items (id, name) VALUES (1, 'a')", &[])
        .unwrap();
    let v = engine
        .execute("SELECT name FROM items WHERE id = 1", &[])
        .unwrap();
    assert!(matches!(v, Value::String(s) if s == "a"));
}

#[test]
fn sql_v2_serial_auto_increment() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE logs (id SERIAL PRIMARY KEY, msg TEXT NOT NULL)",
            &[],
        )
        .unwrap();
    engine
        .execute("INSERT INTO logs (msg) VALUES ('a')", &[])
        .unwrap();
    engine
        .execute("INSERT INTO logs (msg) VALUES ('b')", &[])
        .unwrap();
    let v = engine.execute("SELECT id FROM logs ORDER BY id", &[]).unwrap();
    assert!(matches!(v, Value::Array(ids) if ids.len() == 2));
}

#[test]
fn sql_v2_upsert_do_update() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT)",
            &[],
        )
        .unwrap();
    engine
        .execute("INSERT INTO kv (k, v) VALUES ('a', '1')", &[])
        .unwrap();
    engine
        .execute(
            "INSERT INTO kv (k, v) VALUES ('a', '2') ON CONFLICT DO UPDATE SET v = 'updated'",
            &[],
        )
        .unwrap();
    let v = engine.execute("SELECT v FROM kv WHERE k = 'a'", &[]).unwrap();
    assert!(matches!(v, Value::String(s) if s == "updated"));
}

#[test]
fn sql_v2_returning_insert() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE t (id SERIAL PRIMARY KEY, n INTEGER)", &[])
        .unwrap();
    let v = engine
        .execute("INSERT INTO t (n) VALUES (9) RETURNING id, n", &[])
        .unwrap();
    assert!(matches!(v, Value::Array(row) if row.len() == 1 && matches!(&row[0], Value::Array(cols) if cols.len() == 2)));
}

#[test]
fn sql_v2_left_join() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE a (id INTEGER, x TEXT)", &[])
        .unwrap();
    engine
        .execute("CREATE TABLE b (id INTEGER, y TEXT)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO a (id, x) VALUES (1, 'only')", &[])
        .unwrap();
    let v = engine
        .execute(
            "SELECT a.x, b.y FROM a LEFT JOIN b ON a.id = b.id",
            &[],
        )
        .unwrap();
    assert!(matches!(v, Value::Array(rows) if rows.len() == 1 && matches!(&rows[0], Value::Array(cols) if cols.len() == 2)));
}

#[test]
fn sql_v2_group_by_sum() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE sales (dept TEXT, amount INTEGER)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('a', 10)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('a', 15)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('b', 5)", &[])
        .unwrap();
    let v = engine
        .execute(
            "SELECT dept, SUM(amount) FROM sales GROUP BY dept ORDER BY dept",
            &[],
        )
        .unwrap();
    assert!(matches!(v, Value::Array(rows) if !rows.is_empty()));
}

#[test]
fn sql_v2_where_in_and_like() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE u (id INTEGER, name TEXT)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO u (id, name) VALUES (1, 'Ada')", &[])
        .unwrap();
    engine
        .execute("INSERT INTO u (id, name) VALUES (2, 'Bob')", &[])
        .unwrap();
    let v = engine
        .execute("SELECT id FROM u WHERE id IN (1, 3)", &[])
        .unwrap();
    assert!(matches!(v, Value::Number(1)));
    let v2 = engine
        .execute("SELECT id FROM u WHERE name LIKE 'A%'", &[])
        .unwrap();
    assert!(matches!(v2, Value::Number(1)));
}

#[test]
fn sql_v2_json_column_via_eval() {
    let v = eval_sql(
        r#"
        sql("CREATE TABLE docs (id SERIAL PRIMARY KEY, body JSONB)")
        sql("INSERT INTO docs (body) VALUES ($1)", { "title": "hi", "n": 1 })
        sql("SELECT body FROM docs WHERE id = 1")
    "#,
    );
    assert!(matches!(v, Value::Object(_)));
}

#[test]
fn sql_v2_index_scan_via_explain() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT)",
            &[],
        )
        .unwrap();
    for i in 1..=50 {
        engine
            .execute(
                &format!("INSERT INTO users (id, email) VALUES ({}, 'u{}@x.c')", i, i),
                &[],
            )
            .unwrap();
    }
    engine
        .execute("CREATE INDEX idx_users_email ON users (email)", &[])
        .unwrap();
    let plan = engine
        .execute("EXPLAIN SELECT email FROM users WHERE id = 42", &[])
        .unwrap();
    assert!(matches!(plan, Value::Object(obj) if obj.get("plan").and_then(|v| match v { Value::String(s) => Some(s.contains("Index Scan")), _ => None }).unwrap_or(false)));
    let hit = engine
        .execute("SELECT email FROM users WHERE id = 42", &[])
        .unwrap();
    assert!(matches!(hit, Value::String(s) if s == "u42@x.c"));
}

#[test]
fn sql_v2_returning_update_and_delete() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, n INTEGER)",
            &[],
        )
        .unwrap();
    engine
        .execute("INSERT INTO t (id, n) VALUES (1, 10)", &[])
        .unwrap();
    let updated = engine
        .execute(
            "UPDATE t SET n = 99 WHERE id = 1 RETURNING id, n",
            &[],
        )
        .unwrap();
    assert!(
        matches!(updated, Value::Array(rows) if rows.len() == 1 && matches!(&rows[0], Value::Array(cols) if cols.len() == 2))
    );
    let deleted = engine
        .execute("DELETE FROM t WHERE id = 1 RETURNING id", &[])
        .unwrap();
    assert!(matches!(deleted, Value::Number(1)));
}

#[test]
fn sql_v2_having_filters_groups() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE sales (dept TEXT, amount INTEGER)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('a', 10)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('a', 15)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO sales (dept, amount) VALUES ('b', 5)", &[])
        .unwrap();
    let v = engine
        .execute(
            "SELECT dept FROM sales GROUP BY dept HAVING SUM(amount) > 20 ORDER BY dept",
            &[],
        )
        .unwrap();
    assert!(matches!(v, Value::String(s) if s == "a"));
}

#[test]
fn sql_v2_transactions_commit_and_rollback() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[])
        .unwrap();

    engine.execute("BEGIN", &[]).unwrap();
    engine
        .execute("INSERT INTO t (id) VALUES (1)", &[])
        .unwrap();
    engine.execute("ROLLBACK", &[]).unwrap();
    let count = engine
        .execute("SELECT COUNT(*) FROM t", &[])
        .unwrap();
    assert!(matches!(count, Value::Number(0)));

    engine.execute("BEGIN TRANSACTION", &[]).unwrap();
    engine
        .execute("INSERT INTO t (id) VALUES (2)", &[])
        .unwrap();
    engine.execute("COMMIT TRANSACTION", &[]).unwrap();
    let count = engine
        .execute("SELECT COUNT(*) FROM t", &[])
        .unwrap();
    assert!(matches!(count, Value::Number(1)));
}

#[test]
fn sql_v2_savepoint_rollback() {
    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO t (id) VALUES (1)", &[])
        .unwrap();
    engine.execute("BEGIN", &[]).unwrap();
    engine
        .execute("INSERT INTO t (id) VALUES (2)", &[])
        .unwrap();
    engine.execute("SAVEPOINT sp1", &[]).unwrap();
    engine
        .execute("INSERT INTO t (id) VALUES (3)", &[])
        .unwrap();
    engine.execute("ROLLBACK TO SAVEPOINT sp1", &[]).unwrap();
    let count = engine
        .execute("SELECT COUNT(*) FROM t", &[])
        .unwrap();
    assert!(matches!(count, Value::Number(2)));
    engine.execute("COMMIT", &[]).unwrap();
}

#[test]
fn sql_v2_composite_index_scan() {
    let mut engine = SqlEngine::new();
    engine
        .execute(
            "CREATE TABLE loc (dept TEXT, region TEXT, headcount INTEGER)",
            &[],
        )
        .unwrap();
    engine
        .execute(
            "INSERT INTO loc (dept, region, headcount) VALUES ('sales', 'eu', 10)",
            &[],
        )
        .unwrap();
    engine
        .execute(
            "INSERT INTO loc (dept, region, headcount) VALUES ('sales', 'us', 5)",
            &[],
        )
        .unwrap();
    engine
        .execute(
            "CREATE INDEX idx_loc_dept_region ON loc (dept, region)",
            &[],
        )
        .unwrap();
    let plan = engine
        .execute(
            "EXPLAIN SELECT headcount FROM loc WHERE dept = 'sales' AND region = 'eu'",
            &[],
        )
        .unwrap();
    assert!(matches!(plan, Value::Object(obj) if obj.get("plan").and_then(|v| match v { Value::String(s) => Some(s.contains("Index Scan")), _ => None }).unwrap_or(false)));
    let v = engine
        .execute(
            "SELECT headcount FROM loc WHERE dept = 'sales' AND region = 'eu'",
            &[],
        )
        .unwrap();
    assert!(matches!(v, Value::Number(10)));
}

#[test]
fn sql_v2_persistence_save_load() {
    let path = std::env::temp_dir().join("kabootar_sql_v2_persist.kdb");
    let path_str = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&path);

    let mut engine = SqlEngine::new();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[])
        .unwrap();
    engine
        .execute("INSERT INTO users (id, name) VALUES (1, 'Ada')", &[])
        .unwrap();
    engine
        .execute(&format!("SAVE DATABASE '{path_str}'"), &[])
        .unwrap();

    let mut engine2 = SqlEngine::new();
    engine2
        .execute(&format!("LOAD DATABASE '{path_str}'"), &[])
        .unwrap();
    let name = engine2
        .execute("SELECT name FROM users WHERE id = 1", &[])
        .unwrap();
    assert!(matches!(name, Value::String(s) if s == "Ada"));
    let _ = std::fs::remove_file(&path);
}
