//! Kabootar SQL v2 — in-process database with JSON, UPSERT, RETURNING, indexes,
//! transactions, WAL persistence, and db_open().

use crate::sql::{append_wal, is_binary_kdb, load_engine_v2, load_with_wal, SqlEngine};
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

thread_local! {
    static DB_POOL: RefCell<HashMap<String, Arc<Mutex<SqlEngine>>>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
pub struct DbConnection {
    pub name: String,
    engine: Arc<Mutex<SqlEngine>>,
    persist_path: Option<String>,
}

impl std::fmt::Debug for DbConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnection")
            .field("name", &self.name)
            .field("persist_path", &self.persist_path)
            .finish_non_exhaustive()
    }
}

fn load_engine_for_path(path: &str) -> SqlEngine {
    let mut engine = if Path::new(path).exists() && is_binary_kdb(path) {
        load_engine_v2(path).unwrap_or_default()
    } else {
        load_with_wal(path).unwrap_or_default()
    };
    engine.persist_path = Some(path.to_string());
    engine
}

impl DbConnection {
    pub fn new() -> Self {
        Self {
            name: "kabootar-db".into(),
            engine: Arc::new(Mutex::new(SqlEngine::new())),
            persist_path: None,
        }
    }

    /// Open or reuse a shared engine for `path` (used by `db_open` and `open_kv`).
    pub fn open(path: &str) -> Self {
        DB_POOL.with(|pool| {
            let engine = pool
                .borrow_mut()
                .entry(path.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(load_engine_for_path(path))))
                .clone();
            Self {
                name: format!("kabootar-db:{path}"),
                engine,
                persist_path: Some(path.to_string()),
            }
        })
    }

    pub fn persist_path(&self) -> Option<&str> {
        self.persist_path.as_deref()
    }

    pub fn execute_sql(&self, sql: &str, params: &[Value]) -> Result<Value, String> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| "Database lock poisoned".to_string())?;
        let result = engine.execute(sql, params)?;
        if let Some(path) = &self.persist_path {
            if should_wal(sql) {
                append_wal(path, sql, params)?;
            }
            if should_checkpoint(sql) {
                let _ = engine.persist_checkpoint();
            }
        }
        Ok(result)
    }
}

fn should_wal(sql: &str) -> bool {
    let upper = sql.trim().to_uppercase();
    upper.starts_with("CREATE ")
        || upper.starts_with("ALTER ")
        || upper.starts_with("DROP ")
        || upper.starts_with("INSERT ")
        || upper.starts_with("UPDATE ")
        || upper.starts_with("DELETE ")
        || upper.starts_with("LOAD DATABASE ")
}

fn should_checkpoint(sql: &str) -> bool {
    let upper = sql.trim().to_uppercase();
    upper == "CHECKPOINT" || upper == "COMMIT" || upper == "COMMIT TRANSACTION"
}

fn sql_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("sql() expects a string query as the first argument".into()),
    };
    let params: Vec<Value> = args.iter().skip(1).cloned().collect();
    let conn = env
        .get("db")
        .ok_or("Database connection not available")?;
    let Value::DbConnection(db) = conn else {
        return Err("Database connection not available".into());
    };
    db.execute_sql(&query, &params)
}

fn db_open_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("db_open() expects a file path string".into()),
    };
    let db = DbConnection::open(&path);
    crate::runtime::open_kv::ensure_kv_schema(&db)?;
    env.set("db".to_string(), Value::DbConnection(db));
    Ok(Value::String(format!("opened {path}")))
}

pub fn db_globals(env: &mut Environment) {
    env.set("db".to_string(), Value::DbConnection(DbConnection::new()));
    env.set("sql".to_string(), Value::NativeFunction(sql_native));
    env.set("db_open".to_string(), Value::NativeFunction(db_open_native));
}
