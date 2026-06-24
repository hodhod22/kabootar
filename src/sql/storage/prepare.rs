//! Prepared statement cache (Phase 1).

use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PreparedQuery {
    pub sql: String,
    pub kind: QueryKind,
    pub param_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryKind {
    Select,
    Insert,
    Update,
    Delete,
    Other,
}

#[derive(Debug, Default, Clone)]
pub struct PreparedCache {
    entries: HashMap<u64, PreparedQuery>,
    sql_to_id: HashMap<String, u64>,
    next_id: u64,
    hits: u64,
}

impl PreparedCache {
    pub fn prepare(&mut self, sql: &str) -> u64 {
        if let Some(&id) = self.sql_to_id.get(sql) {
            self.hits += 1;
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        let kind = classify_sql(sql);
        let param_count = sql.matches('$').count();
        let entry = PreparedQuery {
            sql: sql.to_string(),
            kind,
            param_count,
        };
        self.entries.insert(id, entry);
        self.sql_to_id.insert(sql.to_string(), id);
        id
    }

    pub fn get(&self, sql: &str) -> Option<&PreparedQuery> {
        self.sql_to_id.get(sql).and_then(|id| self.entries.get(id))
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }
}

fn classify_sql(sql: &str) -> QueryKind {
    let upper = sql.trim().to_uppercase();
    if upper.starts_with("SELECT") {
        QueryKind::Select
    } else if upper.starts_with("INSERT") {
        QueryKind::Insert
    } else if upper.starts_with("UPDATE") {
        QueryKind::Update
    } else if upper.starts_with("DELETE") {
        QueryKind::Delete
    } else {
        QueryKind::Other
    }
}

pub fn validate_params(prepared: &PreparedQuery, params: &[Value]) -> Result<(), String> {
    if params.len() != prepared.param_count {
        return Err(format!(
            "Expected {} parameters, got {}",
            prepared.param_count,
            params.len()
        ));
    }
    Ok(())
}
