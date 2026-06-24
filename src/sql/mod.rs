//! Kabootar SQL v2 — in-process database tuned for modern apps.
//!
//! PostgreSQL-compatible where it matters, simpler to run: zero config, native
//! JSON, SERIAL ids, UPSERT, RETURNING, LEFT JOIN, GROUP BY, HAVING, indexes, transactions.

mod json_ops;
mod persist;
mod schema;
pub mod storage;
pub mod wal;

use crate::value::Value;
pub use persist::{load_engine, save_engine};
pub use schema::{CheckDef, ColumnDef, ForeignKeyDef, IndexDef, SqlType, TableDef};
pub use storage::{
    flush_dirty_pages, incremental_checkpoint, is_binary_kdb, load_engine_v2, save_engine_v2,
    AccessMethod, BufferPool, MvccState, PreparedCache, QueryPlanner, TableStats,
};
pub use wal::{append_wal, checkpoint, load_with_wal};
use json_ops::{json_contains, json_get_text};
use persist::{save_engine as save_db_file};
use schema::{parse_column_defs, sql_values_equal};
use storage::partition::parse_partition_clause;
use storage::parallel::parallel_count;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum StorageFormat {
    #[default]
    JsonV1,
    BinaryV2,
}

#[derive(Debug, Clone, Default)]
pub struct SqlEngine {
    pub tables: HashMap<String, TableDef>,
    pub persist_path: Option<String>,
    transaction_snapshot: Option<HashMap<String, TableDef>>,
    savepoints: Vec<(String, HashMap<String, TableDef>)>,
    prepared: PreparedCache,
    mvcc: MvccState,
    buffer_pool: BufferPool,
    storage_format: StorageFormat,
}

impl SqlEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn persist_checkpoint(&self) -> Result<(), String> {
        let Some(path) = &self.persist_path else {
            return Ok(());
        };
        if self.storage_format == StorageFormat::BinaryV2 {
            save_engine_v2(self, path)
        } else {
            checkpoint(self, path)
        }
    }

    pub fn uses_binary_storage(&self) -> bool {
        self.storage_format == StorageFormat::BinaryV2
    }

    pub fn from_tables(tables: HashMap<String, TableDef>) -> Self {
        Self {
            tables,
            ..Self::default()
        }
    }

    pub fn execute(&mut self, sql: &str, params: &[Value]) -> Result<Value, String> {
        let sql = sql.trim().trim_end_matches(';').trim();
        if sql.is_empty() {
            return Err("Empty SQL query".into());
        }

        self.prepared.prepare(sql);

        let upper = sql.to_uppercase();
        if upper.starts_with("EXPLAIN ") {
            return self.explain(&sql[8..].trim());
        }
        if upper == "ANALYZE" {
            return self.analyze_all();
        }
        if upper.starts_with("ANALYZE ") {
            return self.analyze_table(&sql[8..].trim());
        }
        if upper == "BEGIN" || upper == "BEGIN TRANSACTION" {
            return self.begin_transaction();
        }
        if upper == "COMMIT" || upper == "COMMIT TRANSACTION" {
            return self.commit_transaction();
        }
        if upper == "ROLLBACK" || upper == "ROLLBACK TRANSACTION" {
            return self.rollback_transaction();
        }
        if upper.starts_with("SAVEPOINT ") {
            return self.create_savepoint(sql);
        }
        if upper.starts_with("ROLLBACK TO SAVEPOINT ") {
            return self.rollback_to_savepoint(sql);
        }
        if upper.starts_with("RELEASE SAVEPOINT ") {
            return self.release_savepoint(sql);
        }
        if upper.starts_with("SAVE DATABASE ") {
            return self.save_database(sql);
        }
        if upper.starts_with("LOAD DATABASE ") {
            return self.load_database(sql);
        }
        if upper == "CHECKPOINT" {
            return self.checkpoint_database();
        }
        if upper.starts_with("ALTER TABLE") {
            return self.alter_table(sql);
        }
        if upper.starts_with("CREATE TABLE") {
            self.create_table(sql)
        } else if upper.starts_with("CREATE UNIQUE INDEX") || upper.starts_with("CREATE INDEX") {
            self.create_index(sql)
        } else if upper.starts_with("DROP TABLE") {
            self.drop_table(sql)
        } else if upper.starts_with("INSERT INTO") {
            self.insert(sql, params)
        } else if upper.starts_with("UPDATE") {
            self.update(sql, params)
        } else if upper.starts_with("DELETE FROM") {
            self.delete(sql, params)
        } else if upper.starts_with("SELECT") {
            self.select(sql, params)
        } else {
            Err(format!("SQL not supported: {}", sql))
        }
    }

    fn begin_transaction(&mut self) -> Result<Value, String> {
        if self.transaction_snapshot.is_some() {
            return Err("Transaction already active".into());
        }
        self.mvcc.begin();
        self.transaction_snapshot = Some(self.tables.clone());
        self.savepoints.clear();
        Ok(Value::String("BEGIN".into()))
    }

    fn commit_transaction(&mut self) -> Result<Value, String> {
        if self.transaction_snapshot.is_none() {
            return Err("No active transaction".into());
        }
        self.mvcc.commit();
        self.transaction_snapshot = None;
        self.savepoints.clear();
        if let Some(path) = self.persist_path.clone() {
            if self.storage_format == StorageFormat::BinaryV2 {
                let _ = save_engine_v2(self, &path);
            }
        }
        Ok(Value::String("COMMIT".into()))
    }

    fn rollback_transaction(&mut self) -> Result<Value, String> {
        let Some(snapshot) = self.transaction_snapshot.take() else {
            return Err("No active transaction".into());
        };
        self.mvcc.rollback();
        self.tables = snapshot;
        self.savepoints.clear();
        Ok(Value::String("ROLLBACK".into()))
    }

    fn analyze_all(&mut self) -> Result<Value, String> {
        let names: Vec<String> = self.tables.keys().cloned().collect();
        for name in names {
            if let Some(t) = self.tables.get_mut(&name) {
                t.analyze_stats();
            }
        }
        Ok(Value::String("ANALYZE".into()))
    }

    fn analyze_table(&mut self, name: &str) -> Result<Value, String> {
        let table = self
            .tables
            .get_mut(name)
            .ok_or_else(|| format!("Unknown table: {name}"))?;
        table.analyze_stats();
        Ok(Value::String(format!("ANALYZE {name}")))
    }

    fn create_savepoint(&mut self, sql: &str) -> Result<Value, String> {
        if self.transaction_snapshot.is_none() {
            return Err("SAVEPOINT requires an active transaction".into());
        }
        let name = sql["SAVEPOINT ".len()..].trim().to_string();
        if name.is_empty() {
            return Err("Expected savepoint name".into());
        }
        self.savepoints.push((name.clone(), self.tables.clone()));
        Ok(Value::String(format!("SAVEPOINT {name}")))
    }

    fn rollback_to_savepoint(&mut self, sql: &str) -> Result<Value, String> {
        if self.transaction_snapshot.is_none() {
            return Err("ROLLBACK TO SAVEPOINT requires an active transaction".into());
        }
        let name = sql["ROLLBACK TO SAVEPOINT ".len()..].trim().to_string();
        let pos = self
            .savepoints
            .iter()
            .rposition(|(sp, _)| sp == &name)
            .ok_or_else(|| format!("Unknown savepoint: {name}"))?;
        self.tables = self.savepoints[pos].1.clone();
        self.savepoints.truncate(pos + 1);
        Ok(Value::String(format!("ROLLBACK TO SAVEPOINT {name}")))
    }

    fn release_savepoint(&mut self, sql: &str) -> Result<Value, String> {
        if self.transaction_snapshot.is_none() {
            return Err("RELEASE SAVEPOINT requires an active transaction".into());
        }
        let name = sql["RELEASE SAVEPOINT ".len()..].trim().to_string();
        let pos = self
            .savepoints
            .iter()
            .position(|(sp, _)| sp == &name)
            .ok_or_else(|| format!("Unknown savepoint: {name}"))?;
        self.savepoints.remove(pos);
        Ok(Value::String(format!("RELEASE SAVEPOINT {name}")))
    }

    fn save_database(&mut self, sql: &str) -> Result<Value, String> {
        let path = parse_quoted_path(sql, "SAVE DATABASE")?;
        if path.ends_with(".kdb2") {
            self.storage_format = StorageFormat::BinaryV2;
            save_engine_v2(self, &path)?;
        } else if self.storage_format == StorageFormat::BinaryV2 {
            save_engine_v2(self, &path)?;
        } else {
            save_db_file(self, &path)?;
        }
        self.persist_path = Some(path.clone());
        Ok(Value::String(format!("SAVED {path}")))
    }

    fn load_database(&mut self, sql: &str) -> Result<Value, String> {
        let path = parse_quoted_path(sql, "LOAD DATABASE")?;
        let loaded = if is_binary_kdb(&path) {
            self.storage_format = StorageFormat::BinaryV2;
            load_engine_v2(&path)?
        } else {
            self.storage_format = StorageFormat::JsonV1;
            load_with_wal(&path)?
        };
        self.tables = loaded.tables;
        self.persist_path = Some(path.clone());
        self.transaction_snapshot = None;
        self.savepoints.clear();
        Ok(Value::String(format!("LOADED {path}")))
    }

    fn checkpoint_database(&mut self) -> Result<Value, String> {
        let path = self
            .persist_path
            .clone()
            .ok_or("CHECKPOINT requires LOAD DATABASE or SAVE DATABASE first")?;
        if self.storage_format == StorageFormat::BinaryV2 {
            save_engine_v2(self, &path)?;
            flush_dirty_pages(&mut self.buffer_pool, &path)?;
        } else {
            checkpoint(self, &path)?;
        }
        Ok(Value::String(format!("CHECKPOINT {path}")))
    }

    fn alter_table(&mut self, sql: &str) -> Result<Value, String> {
        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "ALTER")?;
        expect_keyword(&tokens, &mut i, "TABLE")?;
        let table = expect_ident(&tokens, &mut i)?;
        let table_def = self
            .tables
            .get_mut(&table)
            .ok_or_else(|| format!("Unknown table: {}", table))?;
        if peek_keyword(&tokens, i) == Some("ADD") {
            i += 1;
            expect_keyword(&tokens, &mut i, "COLUMN")?;
            let col_sql = tokens[i..].join(" ");
            let (mut cols, _, fks, chks) = parse_column_defs(&col_sql)?;
            let col = cols
                .pop()
                .ok_or("ALTER TABLE ADD COLUMN requires a column definition")?;
            if col.serial {
                table_def
                    .serial_counters
                    .insert(col.name.clone(), 1);
            }
            table_def.columns.push(col);
            table_def.foreign_keys.extend(fks);
            table_def.checks.extend(chks);
        } else if peek_keyword(&tokens, i) == Some("DROP") {
            i += 1;
            expect_keyword(&tokens, &mut i, "COLUMN")?;
            let col = expect_ident(&tokens, &mut i)?;
            if table_def.primary_key.as_deref() == Some(&col) {
                return Err("Cannot DROP PRIMARY KEY column".into());
            }
            table_def.columns.retain(|c| c.name != col);
            table_def
                .foreign_keys
                .retain(|fk| fk.column != col);
            table_def.checks.retain(|c| c.column != col);
            let mut rows: Vec<HashMap<String, Value>> = table_def
                .iter_live_maps()
                .map(|(_, mut m)| {
                    m.remove(&col);
                    m
                })
                .collect();
            table_def.reload_store(rows);
            table_def.rebuild_all_indexes();
        } else if peek_keyword(&tokens, i) == Some("RENAME") {
            i += 1;
            expect_keyword(&tokens, &mut i, "COLUMN")?;
            let old = expect_ident(&tokens, &mut i)?;
            expect_keyword(&tokens, &mut i, "TO")?;
            let new_name = expect_ident(&tokens, &mut i)?;
            if table_def.get_column(&new_name).is_some() {
                return Err(format!("Column already exists: {}", new_name));
            }
            if table_def.primary_key.as_deref() == Some(&old) {
                table_def.primary_key = Some(new_name.clone());
            }
            for c in &mut table_def.columns {
                if c.name == old {
                    c.name = new_name.clone();
                }
            }
            let rows: Vec<HashMap<String, Value>> = table_def
                .iter_live_maps()
                .map(|(_, mut m)| {
                    if let Some(v) = m.remove(&old) {
                        m.insert(new_name.clone(), v);
                    }
                    m
                })
                .collect();
            table_def.reload_store(rows);
            table_def.rebuild_all_indexes();
        } else {
            return Err("ALTER TABLE supports ADD/DROP/RENAME COLUMN".into());
        }
        Ok(Value::Null)
    }

    fn create_index(&mut self, sql: &str) -> Result<Value, String> {
        let upper = sql.to_uppercase();
        let unique = upper.starts_with("CREATE UNIQUE INDEX");
        let keyword = if unique {
            "CREATE UNIQUE INDEX"
        } else {
            "CREATE INDEX"
        };
        let mut rest = sql[keyword.len()..].trim();
        let if_not_exists = rest.to_uppercase().starts_with("IF NOT EXISTS");
        if if_not_exists {
            rest = rest[12..].trim();
        }
        let (idx_name, rest2) = rest
            .split_once(' ')
            .ok_or("Expected index name")?;
        let rest2 = rest2.trim();
        if !rest2.to_uppercase().starts_with("ON ") {
            return Err("Expected ON after index name".into());
        }
        let rest2 = rest2[3..].trim();
        let open = rest2
            .find('(')
            .ok_or("Expected '(' after table name in CREATE INDEX")?;
        let table = rest2[..open].trim().to_string();
        let cols_sql = &rest2[open + 1..rest2.rfind(')').ok_or("Expected ')'")?];
        let columns: Vec<String> = cols_sql
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if columns.is_empty() {
            return Err("CREATE INDEX requires at least one column".into());
        }
        let table_def = self
            .tables
            .get_mut(&table)
            .ok_or_else(|| format!("Unknown table: {}", table))?;
        if if_not_exists && table_def.indexes.iter().any(|i| i.name == idx_name) {
            return Ok(Value::Null);
        }
        if table_def.indexes.iter().any(|i| i.name == idx_name) {
            return Err(format!("Index already exists: {}", idx_name));
        }
        table_def.indexes.push(IndexDef {
            name: idx_name.to_string(),
            columns,
            unique,
        });
        let idx_name = idx_name.to_string();
        table_def.rebuild_index(&idx_name);
        if unique {
            if let Some(entries) = table_def.index_entries.get(&idx_name) {
                for indices in entries.values() {
                    if indices.len() > 1 {
                        table_def.indexes.retain(|i| i.name != idx_name);
                        table_def.index_entries.remove(&idx_name);
                        return Err(format!(
                            "Could not create unique index {}: duplicate key values",
                            idx_name
                        ));
                    }
                }
            }
        }
        Ok(Value::Null)
    }

    fn drop_table(&mut self, sql: &str) -> Result<Value, String> {
        let upper = sql.to_uppercase();
        let keyword = "DROP TABLE";
        let start = upper.find(keyword).ok_or("Expected DROP TABLE")? + keyword.len();
        let name = sql[start..]
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .next()
            .ok_or("Expected table name after DROP TABLE")?
            .to_string();
        if self.tables.remove(&name).is_none() {
            return Err(format!("Unknown table: {}", name));
        }
        Ok(Value::Null)
    }

    fn create_table(&mut self, sql: &str) -> Result<Value, String> {
        let upper = sql.to_uppercase();
        let if_not_exists = upper.contains("IF NOT EXISTS");
        let keyword = if if_not_exists {
            "CREATE TABLE IF NOT EXISTS"
        } else {
            "CREATE TABLE"
        };
        let start = upper
            .find(keyword)
            .ok_or("Expected CREATE TABLE")?
            + keyword.len();
        let rest = sql[start..].trim();
        let (name, cols_sql) = split_name_and_parens(rest)?;
        let (columns, primary_key, foreign_keys, checks) = parse_column_defs(cols_sql)?;
        if columns.is_empty() {
            return Err("CREATE TABLE requires at least one column".into());
        }
        if if_not_exists && self.tables.contains_key(&name) {
            return Ok(Value::Null);
        }
        if self.tables.contains_key(&name) {
            return Err(format!("Table already exists: {}", name));
        }
        let mut serial_counters = HashMap::new();
        for col in &columns {
            if col.serial {
                serial_counters.insert(col.name.clone(), 1);
            }
        }
        let partition = parse_partition_clause(cols_sql);
        let mut table = TableDef::empty(columns, primary_key);
        table.serial_counters = serial_counters;
        table.foreign_keys = foreign_keys;
        table.checks = checks;
        table.partition = partition;
        table.ensure_auto_indexes();
        table.rebuild_all_indexes();
        self.tables.insert(name, table);
        Ok(Value::Null)
    }

    fn insert(&mut self, sql: &str, params: &[Value]) -> Result<Value, String> {
        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "INSERT")?;
        expect_keyword(&tokens, &mut i, "INTO")?;
        let table = expect_ident(&tokens, &mut i)?;

        let columns = if peek_token(&tokens, i) == Some("(") {
            i += 1;
            let cols = parse_ident_list(&tokens, &mut i)?;
            expect_token(&tokens, &mut i, ")")?;
            cols
        } else {
            Vec::new()
        };

        expect_keyword(&tokens, &mut i, "VALUES")?;
        let value_rows = parse_insert_value_rows(&tokens, &mut i, params)?;
        let (on_conflict, returning) = parse_insert_tail(&tokens, &mut i, params)?;

        let mut inserted: Vec<HashMap<String, Value>> = Vec::new();
        let mut count = 0i64;
        for values in value_rows {
            let (row, n) = self.insert_one_row(&table, &columns, &values, on_conflict.as_ref())?;
            count += n;
            if n > 0 {
                inserted.push(row);
            }
        }
        Ok(returning_or_count(&inserted, returning, count))
    }

    fn insert_one_row(
        &mut self,
        table: &str,
        columns: &[String],
        values: &[Value],
        on_conflict: Option<&OnConflictClause>,
    ) -> Result<(HashMap<String, Value>, i64), String> {
        let tables_snapshot = self.tables.clone();
        let mark = self.mvcc.in_transaction();
        let table_name = table.to_string();

        let result: Result<(HashMap<String, Value>, i64, Option<usize>), String> = (|| {
            let table_def = self
                .tables
                .get_mut(table)
                .ok_or_else(|| format!("Unknown table: {}", table))?;

            let serial_cols: Vec<String> = table_def
                .columns
                .iter()
                .filter(|c| c.serial)
                .map(|c| c.name.clone())
                .collect();

            let col_names = if columns.is_empty() {
                table_def
                    .columns
                    .iter()
                    .filter(|c| !c.serial)
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
            } else {
                columns.to_vec()
            };

            if col_names.len() != values.len() {
                return Err(format!(
                    "Column count ({}) does not match value count ({})",
                    col_names.len(),
                    values.len()
                ));
            }

            let mut row = HashMap::new();
            for (col_name, val) in col_names.iter().zip(values.iter()) {
                let col_def = table_def
                    .get_column(col_name)
                    .ok_or_else(|| format!("Unknown column: {}", col_name))?
                    .clone();
                row.insert(
                    col_name.clone(),
                    TableDef::coerce_value(val, &col_def.sql_type)?,
                );
            }

            for col in serial_cols {
                if !row.contains_key(&col) {
                    let n = table_def.allocate_serial(&col);
                    row.insert(col, Value::Number(n));
                }
            }

            if let Some(conflict) = on_conflict {
                let conflict_cols = &conflict.conflict_columns;
                if let Some(idx) = table_def.find_conflict_row(conflict_cols, &row) {
                    if conflict.do_nothing {
                        return Ok((HashMap::new(), 0, None));
                    }
                    let mut updated_row = table_def.row_map(idx).unwrap_or_default();
                    let old_row = updated_row.clone();
                    for (col, val) in &conflict.assignments {
                        let col_def = table_def
                            .get_column(col)
                            .ok_or_else(|| format!("Unknown column: {}", col))?
                            .clone();
                        updated_row.insert(
                            col.clone(),
                            TableDef::coerce_value(val, &col_def.sql_type)?,
                        );
                    }
                    table_def.validate_row(&updated_row, Some(idx), &tables_snapshot)?;
                    table_def.set_row_map(idx, updated_row.clone())?;
                    table_def.replace_row(idx, &old_row, &updated_row);
                    return Ok((updated_row, 1, None));
                }
            } else if table_def.find_conflict_row(&[], &row).is_some() {
                let pk = table_def.primary_key.clone().unwrap_or_default();
                return Err(format!("Duplicate key value on {}", pk));
            }

            table_def.validate_row(&row, None, &tables_snapshot)?;
            let row_idx = table_def.push_row(row.clone());
            table_def.register_row(row_idx, &row);
            Ok((row, 1, Some(row_idx)))
        })();

        match result {
            Ok((out_row, n, Some(row_idx))) => {
                if mark {
                    self.mvcc.mark_insert(&table_name, row_idx);
                }
                Ok((out_row, n))
            }
            Ok((out_row, n, None)) => Ok((out_row, n)),
            Err(e) => Err(e),
        }
    }

    fn update(&mut self, sql: &str, params: &[Value]) -> Result<Value, String> {
        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "UPDATE")?;
        let table = expect_ident(&tokens, &mut i)?;
        expect_keyword(&tokens, &mut i, "SET")?;
        let assignments = parse_assignments(&tokens, &mut i, params)?;

        let where_clause = if peek_keyword(&tokens, i) == Some("WHERE") {
            i += 1;
            Some(parse_where_expr(&tokens, &mut i, params)?)
        } else {
            None
        };

        let returning = parse_returning(&tokens, &mut i)?;

        if i < tokens.len() {
            return Err(format!("Unexpected token in UPDATE: {}", tokens[i]));
        }

        let match_indices: Vec<usize> = {
            let snap = self
                .tables
                .get(&table)
                .ok_or_else(|| format!("Unknown table: {}", table))?;
            snap.store()
                .iter_live()
                .filter(|&slot| self.mvcc.is_visible(&table, slot))
                .filter_map(|slot| {
                    let row = snap.row_map(slot)?;
                    let qualified = qualify_row(&row, &table);
                    let matches = where_clause
                        .as_ref()
                        .map(|w| eval_where(w, &qualified, self))
                        .unwrap_or(true);
                    if matches { Some(slot) } else { None }
                })
                .collect()
        };

        let tables_snapshot = self.tables.clone();
        let table_def = self
            .tables
            .get_mut(&table)
            .ok_or_else(|| format!("Unknown table: {}", table))?;

        let col_names: Vec<String> = table_def.column_names();
        let mut affected: Vec<HashMap<String, Value>> = Vec::new();
        let mut replacements: Vec<(usize, HashMap<String, Value>, HashMap<String, Value>)> =
            Vec::new();
        for idx in match_indices {
            let old_row = table_def.row_map(idx).unwrap_or_default();
            let mut new_row = old_row.clone();
            for (col, val) in &assignments {
                if !col_names.iter().any(|c| c == col) {
                    return Err(format!("Unknown column: {}", col));
                }
                let col_def = table_def
                    .get_column(col)
                    .ok_or_else(|| format!("Unknown column: {}", col))?
                    .clone();
                new_row.insert(col.clone(), TableDef::coerce_value(val, &col_def.sql_type)?);
            }
            table_def.validate_row(&new_row, Some(idx), &tables_snapshot)?;
            affected.push(new_row.clone());
            replacements.push((idx, old_row, new_row));
        }
        for (idx, old_row, new_row) in replacements {
            table_def.set_row_map(idx, new_row.clone())?;
            table_def.replace_row(idx, &old_row, &new_row);
        }
        Ok(returning_or_count(
            &affected,
            returning,
            affected.len() as i64,
        ))
    }

    fn delete(&mut self, sql: &str, params: &[Value]) -> Result<Value, String> {
        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "DELETE")?;
        expect_keyword(&tokens, &mut i, "FROM")?;
        let table = expect_ident(&tokens, &mut i)?;

        let where_clause = if peek_keyword(&tokens, i) == Some("WHERE") {
            i += 1;
            Some(parse_where_expr(&tokens, &mut i, params)?)
        } else {
            None
        };

        let returning = parse_returning(&tokens, &mut i)?;

        if i < tokens.len() {
            return Err(format!("Unexpected token in DELETE: {}", tokens[i]));
        }

        let all_tables = self.tables.clone();
        let delete_indices: Vec<usize> = if let Some(ref where_expr) = where_clause {
            let snap = self
                .tables
                .get(&table)
                .ok_or_else(|| format!("Unknown table: {}", table))?;
            snap.store()
                .iter_live()
                .filter(|&slot| self.mvcc.is_visible(&table, slot))
                .filter_map(|slot| {
                    let row = snap.row_map(slot)?;
                    let qualified = qualify_row(&row, &table);
                    if eval_where(where_expr, &qualified, self) {
                        Some(slot)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            let snap = self.tables.get(&table).ok_or_else(|| format!("Unknown table: {}", table))?;
            snap.live_slots()
        };

        for &idx in &delete_indices {
            let row = self.tables.get(&table).unwrap().row_map(idx).unwrap_or_default();
            if let Some(t) = all_tables.get(&table) {
                t.check_delete_referenced(&table, &row, &all_tables)?;
            }
        }

        let mut removed = Vec::new();
        let mark = self.mvcc.in_transaction();
        let table_name = table.clone();
        {
            let table_def = self
                .tables
                .get_mut(&table)
                .ok_or_else(|| format!("Unknown table: {}", table))?;
            for &idx in &delete_indices {
                let old = table_def.row_map(idx).unwrap_or_default();
                table_def.unregister_row(idx, &old);
                table_def.remove_slot(idx);
                removed.push(old);
            }
        }
        if mark {
            for &idx in &delete_indices {
                self.mvcc.mark_delete(&table_name, idx);
            }
        }
        Ok(returning_or_count(
            &removed,
            returning,
            removed.len() as i64,
        ))
    }

    fn select(&mut self, sql: &str, params: &[Value]) -> Result<Value, String> {
        if sql.to_uppercase() == "SELECT 1" {
            return Ok(Value::Number(1));
        }

        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "SELECT")?;
        let distinct = if peek_keyword(&tokens, i) == Some("DISTINCT") {
            i += 1;
            true
        } else {
            false
        };
        let select_items = parse_select_items(&tokens, &mut i)?;
        expect_keyword(&tokens, &mut i, "FROM")?;
        let from = parse_table_ref(&tokens, &mut i)?;

        let mut joins = Vec::new();
        loop {
            let kind = if peek_keyword(&tokens, i) == Some("LEFT") {
                i += 1;
                expect_keyword(&tokens, &mut i, "JOIN")?;
                JoinKind::Left
            } else if peek_keyword(&tokens, i) == Some("JOIN")
                || peek_keyword(&tokens, i) == Some("INNER")
            {
                if peek_keyword(&tokens, i) == Some("INNER") {
                    i += 1;
                }
                expect_keyword(&tokens, &mut i, "JOIN")?;
                JoinKind::Inner
            } else {
                break;
            };
            let join_table = parse_table_ref(&tokens, &mut i)?;
            expect_keyword(&tokens, &mut i, "ON")?;
            let on_left = parse_qualified_ident(&tokens, &mut i)?;
            expect_token(&tokens, &mut i, "=")?;
            let on_right = parse_qualified_ident(&tokens, &mut i)?;
            joins.push(JoinClause {
                kind,
                table: join_table,
                on_left,
                on_right,
            });
        }

        let where_clause = if peek_keyword(&tokens, i) == Some("WHERE") {
            i += 1;
            Some(parse_where_expr(&tokens, &mut i, params)?)
        } else {
            None
        };

        let group_by = parse_group_by(&tokens, &mut i)?;
        let having = parse_having(&tokens, &mut i, params)?;
        let order_by = parse_order_by(&tokens, &mut i)?;
        let limit = parse_limit(&tokens, &mut i)?;
        let offset = parse_offset(&tokens, &mut i)?;

        if i < tokens.len() {
            return Err(format!("Unexpected token in SELECT: {}", tokens[i]));
        }

        let (mut rows, where_applied) = if joins.is_empty() {
            if let Some(where_expr) = &where_clause {
                if let Some(indexed) = self.try_indexed_rows(&from, where_expr) {
                    (indexed, true)
                } else {
                    (self.load_table_rows(&from)?, false)
                }
            } else {
                (self.load_table_rows(&from)?, false)
            }
        } else {
            (self.join_rows(&from, &joins)?, false)
        };
        if let Some(where_expr) = where_clause {
            if !where_applied {
                rows.retain(|row| eval_where(&where_expr, row, self));
            }
        }

        if distinct && group_by.is_none() {
            rows = dedupe_rows(&rows, &select_items);
        }

        let projected: Vec<HashMap<String, Value>> = if let Some(group_cols) = &group_by {
            let agg_items = merge_select_items_for_group(&select_items, having.as_ref());
            let mut grouped = group_and_aggregate(&rows, group_cols, &agg_items)?;
            if let Some(having_expr) = &having {
                grouped.retain(|row| eval_where(having_expr, row, self));
            }
            if let Some(orders) = &order_by {
                grouped.sort_by(|a, b| compare_rows(a, b, orders));
            }
            if let Some(off) = offset {
                let off = off as usize;
                if off < grouped.len() {
                    grouped = grouped.split_off(off);
                } else {
                    grouped.clear();
                }
            }
            if let Some(lim) = limit {
                grouped.truncate(lim as usize);
            }
            grouped
                .iter()
                .map(|row| project_row(row, &select_items))
                .collect::<Result<Vec<_>, _>>()?
        } else if select_items.iter().any(|i| {
            matches!(
                i,
                SelectItem::CountAll
                    | SelectItem::CountColumn(_)
                    | SelectItem::Sum(_)
                    | SelectItem::Avg(_)
                    | SelectItem::Min(_)
                    | SelectItem::Max(_)
            )
        }) {
            vec![aggregate_row(&rows, &select_items)?]
        } else {
            if let Some(orders) = &order_by {
                rows.sort_by(|a, b| compare_rows(a, b, orders));
            }
            if let Some(off) = offset {
                let off = off as usize;
                if off < rows.len() {
                    rows = rows.split_off(off);
                } else {
                    rows.clear();
                }
            }
            if let Some(lim) = limit {
                rows.truncate(lim as usize);
            }
            rows.iter()
                .map(|row| project_row(row, &select_items))
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(format_select_result(&projected, &select_items))
    }

    fn join_rows(
        &self,
        from: &TableRef,
        joins: &[JoinClause],
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let base_rows = self.load_table_rows(from)?;
        let mut combined = base_rows;

        for join in joins {
            let right_rows = self.load_table_rows(&join.table)?;
            let mut next = Vec::new();
            for left in &combined {
                let mut matched = false;
                for right in &right_rows {
                    let merged = {
                        let mut m = left.clone();
                        m.extend(right.clone());
                        m
                    };
                    let left_val = row_get(&merged, &join.on_left);
                    let right_val = row_get(&merged, &join.on_right);
                    if let (Some(l), Some(r)) = (&left_val, &right_val) {
                        if sql_values_equal(l, r) {
                            next.push(merged);
                            matched = true;
                        }
                    }
                }
                if !matched && join.kind == JoinKind::Left {
                    let mut merged = left.clone();
                    let alias = join
                        .table
                        .alias
                        .as_deref()
                        .unwrap_or(&join.table.name);
                    let right_table = self.tables.get(&join.table.name).ok_or_else(|| {
                        format!("Unknown table: {}", join.table.name)
                    })?;
                    for col in &right_table.columns {
                        merged.insert(format!("{}.{}", alias, col.name), Value::Null);
                    }
                    next.push(merged);
                }
            }
            combined = next;
        }
        Ok(combined)
    }

    fn load_table_rows(&self, table_ref: &TableRef) -> Result<Vec<HashMap<String, Value>>, String> {
        let table = self
            .tables
            .get(&table_ref.name)
            .ok_or_else(|| format!("Unknown table: {}", table_ref.name))?;
        let alias = table_ref
            .alias
            .as_deref()
            .unwrap_or(&table_ref.name);
        Ok(table
            .store()
            .iter_live()
            .filter(|&slot| self.mvcc.is_visible(&table_ref.name, slot))
            .filter_map(|slot| table.row_map(slot))
            .map(|row| qualify_row(&row, alias))
            .collect())
    }

    fn try_indexed_rows(
        &self,
        table_ref: &TableRef,
        where_expr: &WhereExpr,
    ) -> Option<Vec<HashMap<String, Value>>> {
        let table = self.tables.get(&table_ref.name)?;
        let alias = table_ref
            .alias
            .as_deref()
            .unwrap_or(&table_ref.name);
        let indices = table_row_indices_for_where(table, &table_ref.name, where_expr)?;
        Some(
            indices
                .iter()
                .filter(|&&slot| self.mvcc.is_visible(&table_ref.name, slot))
                .filter_map(|&slot| table.row_map(slot))
                .map(|row| qualify_row(&row, alias))
                .collect(),
        )
    }

    fn execute_subquery(&self, sub: &SubQuery) -> Result<Vec<Value>, String> {
        let mut rows = self.load_table_rows(&sub.from)?;
        if let Some(where_expr) = &sub.where_clause {
            rows.retain(|row| eval_where(where_expr.as_ref(), row, self));
        }
        Ok(rows
            .iter()
            .filter_map(|row| row_get(row, &sub.column))
            .collect())
    }

    fn explain_plan_details(
        &self,
        from: &TableRef,
        joins: usize,
        where_clause: Option<&WhereExpr>,
        select_columns: &[String],
    ) -> (String, usize, f64, Option<String>) {
        let row_estimate = self
            .tables
            .get(&from.name)
            .map(|t| t.live_row_count())
            .unwrap_or(0);
        if joins > 0 {
            return (
                format!("Nested Loop Join ({} tables)", joins + 1),
                row_estimate,
                (joins as f64 + 1.0) * row_estimate as f64,
                None,
            );
        }
        if let Some(where_expr) = where_clause {
            if let Some(table) = self.tables.get(&from.name) {
                if let WhereExpr::Compare(qn, CompareOp::Eq, _) = where_expr {
                    if qn.json_path.is_none() {
                        let plan = QueryPlanner::plan_point_lookup(
                            table,
                            &from.name,
                            &qn.column,
                            &table.stats,
                            select_columns,
                        );
                        return (
                            QueryPlanner::format_plan(&plan),
                            plan.estimated_rows as usize,
                            plan.estimated_cost,
                            plan.index_name,
                        );
                    }
                }
                if let Some(indices) = table_row_indices_for_where(table, &from.name, where_expr) {
                    let index_name = pick_index_name(table, where_expr);
                    return (
                        format!("Index Scan on {}", from.name),
                        indices.len(),
                        indices.len() as f64 * 0.1,
                        index_name,
                    );
                }
            }
        }
        if let Some(table) = self.tables.get(&from.name) {
            let plan = QueryPlanner::plan_seq_scan(&from.name, &table.stats, table.live_row_count());
            return (
                QueryPlanner::format_plan(&plan),
                plan.estimated_rows as usize,
                plan.estimated_cost,
                None,
            );
        }
        (
            format!("Seq Scan on {}", from.name),
            row_estimate,
            row_estimate as f64,
            None,
        )
    }

    fn explain(&self, sql: &str) -> Result<Value, String> {
        if sql.trim().to_uppercase() == "SELECT 1" {
            return Ok(Value::String("Result".into()));
        }
        let tokens = tokenize_sql(sql)?;
        let mut i = 0;
        expect_keyword(&tokens, &mut i, "SELECT")?;
        let select_items = parse_select_items(&tokens, &mut i)?;
        let select_columns: Vec<String> = select_items
            .iter()
            .filter_map(|item| {
                if let SelectItem::Column(qn) = item {
                    Some(qn.column.clone())
                } else {
                    None
                }
            })
            .collect();
        expect_keyword(&tokens, &mut i, "FROM")?;
        let from = parse_table_ref(&tokens, &mut i)?;
        let mut joins = 0usize;
        while peek_keyword(&tokens, i) == Some("LEFT")
            || peek_keyword(&tokens, i) == Some("JOIN")
            || peek_keyword(&tokens, i) == Some("INNER")
        {
            if peek_keyword(&tokens, i) == Some("LEFT") {
                i += 1;
            } else if peek_keyword(&tokens, i) == Some("INNER") {
                i += 1;
            }
            expect_keyword(&tokens, &mut i, "JOIN")?;
            let _ = parse_table_ref(&tokens, &mut i)?;
            expect_keyword(&tokens, &mut i, "ON")?;
            let _ = parse_qualified_ident(&tokens, &mut i)?;
            expect_token(&tokens, &mut i, "=")?;
            let _ = parse_qualified_ident(&tokens, &mut i)?;
            joins += 1;
        }
        let where_clause = if peek_keyword(&tokens, i) == Some("WHERE") {
            i += 1;
            Some(parse_where_expr(&tokens, &mut i, &[])?)
        } else {
            None
        };
        let plan_info = self.explain_plan_details(
            &from,
            joins,
            where_clause.as_ref(),
            &select_columns,
        );
        let mut out = HashMap::new();
        out.insert("plan".into(), Value::String(plan_info.0));
        out.insert("rows".into(), Value::Number(plan_info.1 as i64));
        out.insert("cost".into(), Value::Float(plan_info.2));
        if let Some(idx) = plan_info.3 {
            out.insert("index".into(), Value::String(idx));
        }
        Ok(Value::Object(out))
    }
}

#[derive(Debug, Clone)]
struct TableRef {
    name: String,
    alias: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinKind {
    Inner,
    Left,
}

#[derive(Debug, Clone)]
struct JoinClause {
    kind: JoinKind,
    table: TableRef,
    on_left: QualifiedName,
    on_right: QualifiedName,
}

#[derive(Debug, Clone)]
struct QualifiedName {
    table: String,
    column: String,
    json_path: Option<String>,
}

#[derive(Debug, Clone)]
enum SelectItem {
    All,
    Column(QualifiedName),
    CountAll,
    CountColumn(QualifiedName),
    Sum(QualifiedName),
    Avg(QualifiedName),
    Min(QualifiedName),
    Max(QualifiedName),
}

#[derive(Debug, Clone)]
struct OrderClause {
    column: QualifiedName,
    ascending: bool,
}

#[derive(Debug, Clone)]
enum WhereExpr {
    Compare(QualifiedName, CompareOp, Value),
    IsNull(QualifiedName, bool),
    In(QualifiedName, Vec<Value>),
    InSubquery(QualifiedName, Box<SubQuery>),
    Like(QualifiedName, String),
    Ilike(QualifiedName, String),
    Between(QualifiedName, Value, Value),
    Not(Box<WhereExpr>),
    JsonContains(QualifiedName, Value),
    And(Box<WhereExpr>, Box<WhereExpr>),
    Or(Box<WhereExpr>, Box<WhereExpr>),
}

#[derive(Debug, Clone, PartialEq)]
enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
struct SubQuery {
    column: QualifiedName,
    from: TableRef,
    where_clause: Option<Box<WhereExpr>>,
}

#[derive(Debug, Clone)]
struct OnConflictClause {
    conflict_columns: Vec<String>,
    do_nothing: bool,
    assignments: Vec<(String, Value)>,
}

fn parse_insert_tail(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<(Option<OnConflictClause>, Option<Vec<String>>), String> {
    let on_conflict = if peek_keyword(tokens, *i) == Some("ON") {
        expect_keyword(tokens, i, "ON")?;
        expect_keyword(tokens, i, "CONFLICT")?;
        let conflict_columns = if peek_token(tokens, *i) == Some("(") {
            *i += 1;
            let cols = parse_ident_list(tokens, i)?;
            expect_token(tokens, i, ")")?;
            cols
        } else {
            Vec::new()
        };
        expect_keyword(tokens, i, "DO")?;
        if peek_keyword(tokens, *i) == Some("NOTHING") {
            *i += 1;
            Some(OnConflictClause {
                conflict_columns,
                do_nothing: true,
                assignments: Vec::new(),
            })
        } else {
            expect_keyword(tokens, i, "UPDATE")?;
            expect_keyword(tokens, i, "SET")?;
            Some(OnConflictClause {
                conflict_columns,
                do_nothing: false,
                assignments: parse_assignments(tokens, i, params)?,
            })
        }
    } else {
        None
    };
    let returning = parse_returning(tokens, i)?;
    if *i < tokens.len() {
        return Err(format!("Unexpected token in INSERT: {}", tokens[*i]));
    }
    Ok((on_conflict, returning))
}

fn parse_returning(tokens: &[String], i: &mut usize) -> Result<Option<Vec<String>>, String> {
    if peek_keyword(tokens, *i) != Some("RETURNING") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "RETURNING")?;
    Ok(Some(parse_ident_list(tokens, i)?))
}

fn returning_or_count(
    rows: &[HashMap<String, Value>],
    returning: Option<Vec<String>>,
    count: i64,
) -> Value {
    if let Some(cols) = returning {
        let projected: Vec<HashMap<String, Value>> = rows
            .iter()
            .map(|row| {
                let mut out = HashMap::new();
                for col in &cols {
                    out.insert(
                        col.clone(),
                        row.get(col).cloned().unwrap_or(Value::Null),
                    );
                }
                out
            })
            .collect();
        let items: Vec<SelectItem> = cols
            .iter()
            .map(|c| SelectItem::Column(QualifiedName {
                table: String::new(),
                column: c.clone(),
                json_path: None,
            }))
            .collect();
        return format_select_result(&projected, &items);
    }
    Value::Number(count)
}

fn split_name_and_parens(input: &str) -> Result<(String, &str), String> {
    let input = input.trim();
    let open = input
        .find('(')
        .ok_or("Expected '(' after table name in CREATE TABLE")?;
    let name = input[..open].trim().to_string();
    let close = input
        .rfind(')')
        .ok_or("Expected ')' in CREATE TABLE")?;
    Ok((name, &input[open + 1..close]))
}

fn parse_assignments(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<Vec<(String, Value)>, String> {
    let mut assignments = Vec::new();
    loop {
        let col = expect_ident(tokens, i)?;
        expect_token(tokens, i, "=")?;
        let val = parse_value_token(
            tokens.get(*i).ok_or("Expected value in SET")?,
            params,
        )?;
        *i += 1;
        assignments.push((col, val));
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(assignments)
}

fn parse_group_by(tokens: &[String], i: &mut usize) -> Result<Option<Vec<QualifiedName>>, String> {
    if peek_keyword(tokens, *i) != Some("GROUP") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "GROUP")?;
    expect_keyword(tokens, i, "BY")?;
    let mut cols = Vec::new();
    loop {
        cols.push(parse_qualified_ident(tokens, i)?);
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(Some(cols))
}

fn parse_having(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<Option<WhereExpr>, String> {
    if peek_keyword(tokens, *i) != Some("HAVING") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "HAVING")?;
    Ok(Some(parse_having_expr(tokens, i, params)?))
}

fn parse_having_expr(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<WhereExpr, String> {
    let mut expr = parse_having_term(tokens, i, params)?;
    while peek_keyword(tokens, *i) == Some("OR") {
        *i += 1;
        let right = parse_having_term(tokens, i, params)?;
        expr = WhereExpr::Or(Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_having_term(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<WhereExpr, String> {
    let mut expr = parse_having_compare(tokens, i, params)?;
    while peek_keyword(tokens, *i) == Some("AND") {
        *i += 1;
        let right = parse_having_compare(tokens, i, params)?;
        expr = WhereExpr::And(Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_having_compare(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<WhereExpr, String> {
    let left = parse_having_lhs(tokens, i)?;
    let op_token = tokens
        .get(*i)
        .ok_or("Expected comparison operator in HAVING")?;
    let op = match op_token.as_str() {
        "=" => CompareOp::Eq,
        "!=" | "<>" => CompareOp::Ne,
        "<" => CompareOp::Lt,
        "<=" => CompareOp::Le,
        ">" => CompareOp::Gt,
        ">=" => CompareOp::Ge,
        _ => return Err(format!("Unsupported operator in HAVING: {}", op_token)),
    };
    *i += 1;
    let right = parse_value_token(
        tokens.get(*i).ok_or("Expected value in HAVING")?,
        params,
    )?;
    *i += 1;
    Ok(WhereExpr::Compare(left, op, right))
}

fn parse_having_lhs(tokens: &[String], i: &mut usize) -> Result<QualifiedName, String> {
    if let Some(kind) = peek_keyword(tokens, *i) {
        if matches!(kind, "COUNT" | "SUM" | "AVG" | "MIN" | "MAX") {
            let kind = tokens[*i].to_ascii_uppercase();
            *i += 1;
            expect_token(tokens, i, "(")?;
            if kind == "COUNT" && peek_token(tokens, *i) == Some("*") {
                *i += 1;
                expect_token(tokens, i, ")")?;
                return Ok(QualifiedName {
                    table: String::new(),
                    column: "count".into(),
                    json_path: None,
                });
            }
            let col = parse_qualified_ident(tokens, i)?;
            expect_token(tokens, i, ")")?;
            let key = match kind.as_str() {
                "COUNT" => format!("count({})", qualified_key(&col)),
                "SUM" => format!("sum({})", qualified_key(&col)),
                "AVG" => format!("avg({})", qualified_key(&col)),
                "MIN" => format!("min({})", qualified_key(&col)),
                "MAX" => format!("max({})", qualified_key(&col)),
                _ => unreachable!(),
            };
            return Ok(QualifiedName {
                table: String::new(),
                column: key,
                json_path: None,
            });
        }
    }
    parse_qualified_ident(tokens, i)
}

fn parse_order_by(tokens: &[String], i: &mut usize) -> Result<Option<Vec<OrderClause>>, String> {
    if peek_keyword(tokens, *i) != Some("ORDER") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "ORDER")?;
    expect_keyword(tokens, i, "BY")?;
    let mut orders = Vec::new();
    loop {
        let column = parse_qualified_ident(tokens, i)?;
        let ascending = match peek_keyword(tokens, *i) {
            Some("ASC") => {
                *i += 1;
                true
            }
            Some("DESC") => {
                *i += 1;
                false
            }
            _ => true,
        };
        orders.push(OrderClause { column, ascending });
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(Some(orders))
}

fn parse_limit(tokens: &[String], i: &mut usize) -> Result<Option<i64>, String> {
    if peek_keyword(tokens, *i) != Some("LIMIT") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "LIMIT")?;
    let n: i64 = tokens
        .get(*i)
        .ok_or("Expected LIMIT value")?
        .parse()
        .map_err(|_| "Invalid LIMIT value")?;
    *i += 1;
    Ok(Some(n))
}

fn parse_offset(tokens: &[String], i: &mut usize) -> Result<Option<i64>, String> {
    if peek_keyword(tokens, *i) != Some("OFFSET") {
        return Ok(None);
    }
    expect_keyword(tokens, i, "OFFSET")?;
    let n: i64 = tokens
        .get(*i)
        .ok_or("Expected OFFSET value")?
        .parse()
        .map_err(|_| "Invalid OFFSET value")?;
    *i += 1;
    Ok(Some(n))
}

fn compare_rows(
    a: &HashMap<String, Value>,
    b: &HashMap<String, Value>,
    orders: &[OrderClause],
) -> std::cmp::Ordering {
    for order in orders {
        let va = row_get(a, &order.column).unwrap_or(Value::Null);
        let vb = row_get(b, &order.column).unwrap_or(Value::Null);
        let ord = compare_sort_values(&va, &vb);
        if ord != std::cmp::Ordering::Equal {
            return if order.ascending { ord } else { ord.reverse() };
        }
    }
    std::cmp::Ordering::Equal
}

fn compare_sort_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn aggregate_row(
    rows: &[HashMap<String, Value>],
    items: &[SelectItem],
) -> Result<HashMap<String, Value>, String> {
    let mut out = HashMap::new();
    for item in items {
        match item {
            SelectItem::CountAll => {
                let count = if storage::parallel::should_parallelize(rows.len()) {
                    parallel_count(rows.len())
                } else {
                    rows.len() as i64
                };
                out.insert("count".to_string(), Value::Number(count));
            }
            SelectItem::CountColumn(name) => {
                let key = qualified_key(name);
                let count = rows
                    .iter()
                    .filter(|row| {
                        row_get(row, name)
                            .map(|v| !matches!(v, Value::Null))
                            .unwrap_or(false)
                    })
                    .count();
                out.insert(format!("count({})", key), Value::Number(count as i64));
            }
            SelectItem::Sum(name) => {
                let key = qualified_key(name);
                let sum: f64 = rows
                    .iter()
                    .filter_map(|row| row_get(row, name))
                    .filter_map(|v| to_f64(&v))
                    .sum();
                out.insert(format!("sum({})", key), Value::Float(sum));
            }
            SelectItem::Avg(name) => {
                let key = qualified_key(name);
                let vals: Vec<f64> = rows
                    .iter()
                    .filter_map(|row| row_get(row, name))
                    .filter_map(|v| to_f64(&v))
                    .collect();
                let avg = if vals.is_empty() {
                    0.0
                } else {
                    vals.iter().sum::<f64>() / vals.len() as f64
                };
                out.insert(format!("avg({})", key), Value::Float(avg));
            }
            SelectItem::Min(name) => {
                let key = qualified_key(name);
                let min = rows
                    .iter()
                    .filter_map(|row| row_get(row, name))
                    .filter_map(|v| to_f64(&v))
                    .fold(f64::INFINITY, f64::min);
                out.insert(
                    format!("min({})", key),
                    Value::Float(if min.is_finite() { min } else { 0.0 }),
                );
            }
            SelectItem::Max(name) => {
                let key = qualified_key(name);
                let max = rows
                    .iter()
                    .filter_map(|row| row_get(row, name))
                    .filter_map(|v| to_f64(&v))
                    .fold(f64::NEG_INFINITY, f64::max);
                out.insert(
                    format!("max({})", key),
                    Value::Float(if max.is_finite() { max } else { 0.0 }),
                );
            }
            _ => {}
        }
    }
    Ok(out)
}

fn merge_select_items_for_group(
    select_items: &[SelectItem],
    having: Option<&WhereExpr>,
) -> Vec<SelectItem> {
    let mut items = select_items.to_vec();
    if let Some(expr) = having {
        for extra in collect_having_aggregates(expr) {
            if !items.iter().any(|i| select_items_same(i, &extra)) {
                items.push(extra);
            }
        }
    }
    items
}

fn collect_having_aggregates(expr: &WhereExpr) -> Vec<SelectItem> {
    match expr {
        WhereExpr::Compare(left, _, _) => aggregate_item_from_having_key(&left.column),
        WhereExpr::And(a, b) | WhereExpr::Or(a, b) => {
            let mut out = collect_having_aggregates(a);
            for extra in collect_having_aggregates(b) {
                if !out.iter().any(|i| select_items_same(i, &extra)) {
                    out.push(extra);
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

fn aggregate_item_from_having_key(column: &str) -> Vec<SelectItem> {
    if column == "count" {
        return vec![SelectItem::CountAll];
    }
    let mk = |col: &str| QualifiedName {
        table: String::new(),
        column: col.to_string(),
        json_path: None,
    };
    if let Some(inner) = column.strip_prefix("sum(").and_then(|s| s.strip_suffix(')')) {
        return vec![SelectItem::Sum(mk(inner))];
    }
    if let Some(inner) = column.strip_prefix("avg(").and_then(|s| s.strip_suffix(')')) {
        return vec![SelectItem::Avg(mk(inner))];
    }
    if let Some(inner) = column.strip_prefix("min(").and_then(|s| s.strip_suffix(')')) {
        return vec![SelectItem::Min(mk(inner))];
    }
    if let Some(inner) = column.strip_prefix("max(").and_then(|s| s.strip_suffix(')')) {
        return vec![SelectItem::Max(mk(inner))];
    }
    if let Some(inner) = column.strip_prefix("count(").and_then(|s| s.strip_suffix(')')) {
        return vec![SelectItem::CountColumn(mk(inner))];
    }
    Vec::new()
}

fn select_items_same(a: &SelectItem, b: &SelectItem) -> bool {
    match (a, b) {
        (SelectItem::All, SelectItem::All) => true,
        (SelectItem::CountAll, SelectItem::CountAll) => true,
        (SelectItem::Column(x), SelectItem::Column(y))
        | (SelectItem::CountColumn(x), SelectItem::CountColumn(y))
        | (SelectItem::Sum(x), SelectItem::Sum(y))
        | (SelectItem::Avg(x), SelectItem::Avg(y))
        | (SelectItem::Min(x), SelectItem::Min(y))
        | (SelectItem::Max(x), SelectItem::Max(y)) => x.table == y.table && x.column == y.column,
        _ => false,
    }
}

fn group_and_aggregate(
    rows: &[HashMap<String, Value>],
    group_cols: &[QualifiedName],
    items: &[SelectItem],
) -> Result<Vec<HashMap<String, Value>>, String> {
    let mut groups: HashMap<String, Vec<&HashMap<String, Value>>> = HashMap::new();
    for row in rows {
        let key = group_cols
            .iter()
            .map(|c| {
                row_get(row, c)
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_else(|| "null".into())
            })
            .collect::<Vec<_>>()
            .join("|");
        groups.entry(key).or_default().push(row);
    }
    let mut out = Vec::new();
    for (_key, group_rows) in groups {
        let owned: Vec<HashMap<String, Value>> =
            group_rows.iter().map(|r| (*r).clone()).collect();
        let mut row = aggregate_row(&owned, items)?;
        for col in group_cols {
            let k = qualified_key(col);
            if let Some(v) = owned.first().and_then(|r| row_get(r, col)) {
                row.insert(k, v);
            }
        }
        for item in items {
            if let SelectItem::Column(name) = item {
                let k = qualified_key(name);
                if let Some(v) = owned.first().and_then(|r| row_get(r, name)) {
                    row.insert(k, v);
                }
            }
        }
        out.push(row);
    }
    Ok(out)
}

fn qualified_key(name: &QualifiedName) -> String {
    let base = if name.table.is_empty() {
        name.column.clone()
    } else {
        format!("{}.{}", name.table, name.column)
    };
    if let Some(path) = &name.json_path {
        format!("{base}->>{path}")
    } else {
        base
    }
}

fn pick_index_name(table: &TableDef, expr: &WhereExpr) -> Option<String> {
    if let WhereExpr::Compare(qn, CompareOp::Eq, _) = expr {
        if table.primary_key.as_deref() == Some(qn.column.as_str()) {
            return Some("PRIMARY".into());
        }
        for idx in &table.indexes {
            if idx.columns == [qn.column.clone()] || idx.columns.first().map(|s| s.as_str()) == Some(qn.column.as_str()) {
                return Some(idx.name.clone());
            }
        }
    }
    None
}

fn dedupe_rows(
    rows: &[HashMap<String, Value>],
    items: &[SelectItem],
) -> Vec<HashMap<String, Value>> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for row in rows {
        let projected = project_row(row, items).ok();
        let key = projected
            .as_ref()
            .map(|p| format!("{:?}", p))
            .unwrap_or_default();
        if !seen.contains(&key) {
            seen.push(key);
            out.push(row.clone());
        }
    }
    out
}

fn tokenize_sql(sql: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut chars = sql.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        match c {
            '(' | ')' | ',' | '*' => {
                tokens.push(c.to_string());
                chars.next();
            }
            '!' if chars.clone().nth(1) == Some('=') => {
                chars.next();
                chars.next();
                tokens.push("!=".into());
            }
            '-' if chars.clone().nth(1) == Some('>') && chars.clone().nth(2) == Some('>') => {
                chars.next();
                chars.next();
                chars.next();
                tokens.push("->>".into());
            }
            '<' if chars.clone().nth(1) == Some('>') && chars.clone().nth(2) == Some('>') => {
                chars.next();
                chars.next();
                chars.next();
                tokens.push("->>".into());
            }
            '@' if chars.clone().nth(1) == Some('>') => {
                chars.next();
                chars.next();
                tokens.push("@>".into());
            }
            '<' if chars.clone().nth(1) == Some('=') => {
                chars.next();
                chars.next();
                tokens.push("<=".into());
            }
            '<' if chars.clone().nth(1) == Some('>') => {
                chars.next();
                chars.next();
                tokens.push("<>".into());
            }
            '<' => {
                tokens.push("<".into());
                chars.next();
            }
            '>' if chars.clone().nth(1) == Some('=') => {
                chars.next();
                chars.next();
                tokens.push(">=".into());
            }
            '>' => {
                tokens.push(">".into());
                chars.next();
            }
            '=' => {
                tokens.push("=".into());
                chars.next();
            }
            '\'' => {
                chars.next();
                let mut s = String::new();
                for ch in chars.by_ref() {
                    if ch == '\'' {
                        break;
                    }
                    s.push(ch);
                }
                tokens.push(format!("'{}'", s));
            }
            _ if c.is_ascii_digit() || c == '-' || c == '.' => {
                let mut s = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' {
                        s.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(s);
            }
            _ if c.is_alphanumeric() || c == '_' || c == '$' => {
                let mut s = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                        s.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(s);
            }
            _ => return Err(format!("Unexpected character in SQL: {}", c)),
        }
    }
    Ok(tokens)
}

fn peek_token<'a>(tokens: &'a [String], i: usize) -> Option<&'a str> {
    tokens.get(i).map(String::as_str)
}

fn peek_keyword(tokens: &[String], i: usize) -> Option<&str> {
    peek_token(tokens, i).filter(|t| t.chars().all(|c| c.is_ascii_alphabetic()))
}

fn expect_token(tokens: &[String], i: &mut usize, expected: &str) -> Result<(), String> {
    let found = tokens
        .get(*i)
        .ok_or_else(|| format!("Expected '{}', found end of query", expected))?;
    if found.eq_ignore_ascii_case(expected) || found == expected {
        *i += 1;
        Ok(())
    } else {
        Err(format!("Expected '{}', found '{}'", expected, found))
    }
}

fn expect_keyword(tokens: &[String], i: &mut usize, expected: &str) -> Result<(), String> {
    expect_token(tokens, i, expected)
}

fn expect_ident(tokens: &[String], i: &mut usize) -> Result<String, String> {
    let found = tokens
        .get(*i)
        .ok_or_else(|| "Expected identifier".to_string())?;
    if found.eq_ignore_ascii_case("SELECT")
        || found.eq_ignore_ascii_case("FROM")
        || found.eq_ignore_ascii_case("WHERE")
        || found.eq_ignore_ascii_case("JOIN")
        || found.eq_ignore_ascii_case("ON")
        || found.eq_ignore_ascii_case("INSERT")
        || found.eq_ignore_ascii_case("INTO")
        || found.eq_ignore_ascii_case("VALUES")
        || found.eq_ignore_ascii_case("UPDATE")
        || found.eq_ignore_ascii_case("DELETE")
        || found.eq_ignore_ascii_case("DROP")
        || found.eq_ignore_ascii_case("SET")
        || found.eq_ignore_ascii_case("ORDER")
        || found.eq_ignore_ascii_case("BY")
        || found.eq_ignore_ascii_case("LIMIT")
        || found.eq_ignore_ascii_case("OFFSET")
        || found.eq_ignore_ascii_case("IS")
        || found.eq_ignore_ascii_case("NOT")
        || found.eq_ignore_ascii_case("ASC")
        || found.eq_ignore_ascii_case("DESC")
        || found.eq_ignore_ascii_case("COUNT")
        || found.eq_ignore_ascii_case("SUM")
        || found.eq_ignore_ascii_case("AVG")
        || found.eq_ignore_ascii_case("MIN")
        || found.eq_ignore_ascii_case("MAX")
        || found.eq_ignore_ascii_case("LEFT")
        || found.eq_ignore_ascii_case("INNER")
        || found.eq_ignore_ascii_case("GROUP")
        || found.eq_ignore_ascii_case("RETURNING")
        || found.eq_ignore_ascii_case("CONFLICT")
        || found.eq_ignore_ascii_case("DO")
        || found.eq_ignore_ascii_case("NOTHING")
        || found.eq_ignore_ascii_case("IN")
        || found.eq_ignore_ascii_case("LIKE")
        || found.eq_ignore_ascii_case("INDEX")
        || found.eq_ignore_ascii_case("UNIQUE")
        || found.eq_ignore_ascii_case("EXISTS")
        || found.eq_ignore_ascii_case("PRIMARY")
        || found.eq_ignore_ascii_case("KEY")
        || found == "("
        || found == ")"
        || found == ","
        || found == "="
        || found == "*"
    {
        return Err(format!("Expected identifier, found '{}'", found));
    }
    *i += 1;
    Ok(found.clone())
}

fn parse_ident_list(tokens: &[String], i: &mut usize) -> Result<Vec<String>, String> {
    let mut cols = Vec::new();
    loop {
        cols.push(expect_ident(tokens, i)?);
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(cols)
}

fn parse_value_list(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    loop {
        values.push(parse_value_token(
            tokens.get(*i).ok_or("Expected value")?,
            params,
        )?);
        *i += 1;
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(values)
}

fn parse_value_token(token: &str, params: &[Value]) -> Result<Value, String> {
    if token.starts_with('\'') && token.ends_with('\'') {
        return Ok(Value::String(token[1..token.len() - 1].to_string()));
    }
    if token.starts_with('$') {
        let index: usize = token[1..]
            .parse()
            .map_err(|_| format!("Invalid parameter: {}", token))?;
        return params
            .get(index - 1)
            .cloned()
            .ok_or_else(|| format!("Missing parameter {}", token));
    }
    if token.eq_ignore_ascii_case("NULL") {
        return Ok(Value::Null);
    }
    if token.eq_ignore_ascii_case("TRUE") {
        return Ok(Value::Bool(true));
    }
    if token.eq_ignore_ascii_case("FALSE") {
        return Ok(Value::Bool(false));
    }
    if token.contains('.') {
        let f: f64 = token
            .parse()
            .map_err(|_| format!("Invalid float literal: {}", token))?;
        return Ok(Value::Float(f));
    }
    let n: i64 = token
        .parse()
        .map_err(|_| format!("Invalid integer literal: {}", token))?;
    Ok(Value::Number(n))
}

fn parse_select_items(tokens: &[String], i: &mut usize) -> Result<Vec<SelectItem>, String> {
    let mut items = Vec::new();
    loop {
        match peek_keyword(tokens, *i) {
            Some("COUNT") | Some("SUM") | Some("AVG") | Some("MIN") | Some("MAX") => {
                let kind = tokens[*i].to_ascii_uppercase();
                *i += 1;
                expect_token(tokens, i, "(")?;
                if kind == "COUNT" && peek_token(tokens, *i) == Some("*") {
                    *i += 1;
                    items.push(SelectItem::CountAll);
                } else {
                    let col = parse_qualified_ident(tokens, i)?;
                    match kind.as_str() {
                        "COUNT" => items.push(SelectItem::CountColumn(col)),
                        "SUM" => items.push(SelectItem::Sum(col)),
                        "AVG" => items.push(SelectItem::Avg(col)),
                        "MIN" => items.push(SelectItem::Min(col)),
                        "MAX" => items.push(SelectItem::Max(col)),
                        _ => unreachable!(),
                    }
                }
                expect_token(tokens, i, ")")?;
            }
            _ if peek_token(tokens, *i) == Some("*") => {
                *i += 1;
                items.push(SelectItem::All);
            }
            _ => items.push(SelectItem::Column(parse_qualified_ident(tokens, i)?)),
        }
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(items)
}

fn parse_table_ref(tokens: &[String], i: &mut usize) -> Result<TableRef, String> {
    let name = expect_ident(tokens, i)?;
    if let Some(next) = peek_keyword(tokens, *i) {
        if !matches!(
            next,
            "JOIN" | "LEFT" | "INNER" | "ON" | "WHERE" | "AND" | "OR" | "ORDER" | "GROUP"
                | "LIMIT" | "OFFSET" | "SET" | "HAVING"
        ) {
            let alias = expect_ident(tokens, i)?;
            return Ok(TableRef {
                name,
                alias: Some(alias),
            });
        }
    }
    Ok(TableRef { name, alias: None })
}

fn parse_qualified_ident(tokens: &[String], i: &mut usize) -> Result<QualifiedName, String> {
    let first = expect_ident(tokens, i)?;
    let (table, column) = if peek_token(tokens, *i) == Some(".") {
        *i += 1;
        (first, expect_ident(tokens, i)?)
    } else {
        (String::new(), first)
    };
    let json_path = if peek_token(tokens, *i) == Some("->>") {
        *i += 1;
        let path = parse_value_token(
            tokens.get(*i).ok_or("Expected JSON path after ->>")?,
            &[],
        )?;
        *i += 1;
        let Value::String(s) = path else {
            return Err("JSON path after ->> must be a string".into());
        };
        Some(s)
    } else {
        None
    };
    Ok(QualifiedName {
        table,
        column,
        json_path,
    })
}

fn parse_insert_value_rows(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<Vec<Vec<Value>>, String> {
    let mut rows = Vec::new();
    loop {
        expect_token(tokens, i, "(")?;
        rows.push(parse_value_list(tokens, i, params)?);
        expect_token(tokens, i, ")")?;
        if peek_token(tokens, *i) == Some(",") {
            *i += 1;
        } else {
            break;
        }
    }
    Ok(rows)
}

fn parse_subquery(tokens: &[String], i: &mut usize, params: &[Value]) -> Result<SubQuery, String> {
    expect_keyword(tokens, i, "SELECT")?;
    if peek_keyword(tokens, *i) == Some("DISTINCT") {
        *i += 1;
    }
    let items = parse_select_items(tokens, i)?;
    let column = match items.first() {
        Some(SelectItem::Column(c)) => c.clone(),
        _ => return Err("Subquery must select a single column".into()),
    };
    expect_keyword(tokens, i, "FROM")?;
    let from = parse_table_ref(tokens, i)?;
    let where_clause = if peek_keyword(tokens, *i) == Some("WHERE") {
        *i += 1;
        Some(Box::new(parse_where_expr(tokens, i, params)?))
    } else {
        None
    };
    Ok(SubQuery {
        column,
        from,
        where_clause,
    })
}

fn parse_where_expr(tokens: &[String], i: &mut usize, params: &[Value]) -> Result<WhereExpr, String> {
    let mut expr = parse_where_term(tokens, i, params)?;
    while peek_keyword(tokens, *i) == Some("OR") {
        *i += 1;
        let right = parse_where_term(tokens, i, params)?;
        expr = WhereExpr::Or(Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_where_term(tokens: &[String], i: &mut usize, params: &[Value]) -> Result<WhereExpr, String> {
    let mut expr = parse_where_compare(tokens, i, params)?;
    while peek_keyword(tokens, *i) == Some("AND") {
        *i += 1;
        let right = parse_where_compare(tokens, i, params)?;
        expr = WhereExpr::And(Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_where_compare(
    tokens: &[String],
    i: &mut usize,
    params: &[Value],
) -> Result<WhereExpr, String> {
    if peek_keyword(tokens, *i) == Some("NOT") {
        *i += 1;
        if peek_keyword(tokens, *i) == Some("BETWEEN") {
            *i += 1;
            let left = parse_qualified_ident(tokens, i)?;
            let lo = parse_value_token(
                tokens.get(*i).ok_or("Expected BETWEEN low value")?,
                params,
            )?;
            *i += 1;
            expect_keyword(tokens, i, "AND")?;
            let hi = parse_value_token(
                tokens.get(*i).ok_or("Expected BETWEEN high value")?,
                params,
            )?;
            *i += 1;
            return Ok(WhereExpr::Not(Box::new(WhereExpr::Between(left, lo, hi))));
        }
        let inner = parse_where_compare(tokens, i, params)?;
        return Ok(WhereExpr::Not(Box::new(inner)));
    }
    let left = parse_qualified_ident(tokens, i)?;
    if peek_keyword(tokens, *i) == Some("IS") {
        *i += 1;
        let not = if peek_keyword(tokens, *i) == Some("NOT") {
            *i += 1;
            true
        } else {
            false
        };
        expect_keyword(tokens, i, "NULL")?;
        return Ok(WhereExpr::IsNull(left, !not));
    }
    if peek_keyword(tokens, *i) == Some("IN") {
        *i += 1;
        expect_token(tokens, i, "(")?;
        if peek_keyword(tokens, *i) == Some("SELECT") {
            let sub = parse_subquery(tokens, i, params)?;
            expect_token(tokens, i, ")")?;
            return Ok(WhereExpr::InSubquery(left, Box::new(sub)));
        }
        let mut vals = Vec::new();
        loop {
            vals.push(parse_value_token(
                tokens.get(*i).ok_or("Expected value in IN list")?,
                params,
            )?);
            *i += 1;
            if peek_token(tokens, *i) == Some(",") {
                *i += 1;
            } else {
                break;
            }
        }
        expect_token(tokens, i, ")")?;
        return Ok(WhereExpr::In(left, vals));
    }
    if peek_keyword(tokens, *i) == Some("BETWEEN") {
        *i += 1;
        let lo = parse_value_token(
            tokens.get(*i).ok_or("Expected BETWEEN low value")?,
            params,
        )?;
        *i += 1;
        expect_keyword(tokens, i, "AND")?;
        let hi = parse_value_token(
            tokens.get(*i).ok_or("Expected BETWEEN high value")?,
            params,
        )?;
        *i += 1;
        return Ok(WhereExpr::Between(left, lo, hi));
    }
    if peek_keyword(tokens, *i) == Some("ILIKE") {
        *i += 1;
        let pat = parse_value_token(
            tokens.get(*i).ok_or("Expected ILIKE pattern")?,
            params,
        )?;
        *i += 1;
        let Value::String(s) = pat else {
            return Err("ILIKE pattern must be a string".into());
        };
        return Ok(WhereExpr::Ilike(left, s));
    }
    if peek_keyword(tokens, *i) == Some("LIKE") {
        *i += 1;
        let pat = parse_value_token(
            tokens.get(*i).ok_or("Expected LIKE pattern")?,
            params,
        )?;
        *i += 1;
        let Value::String(s) = pat else {
            return Err("LIKE pattern must be a string".into());
        };
        return Ok(WhereExpr::Like(left, s));
    }
    if peek_token(tokens, *i) == Some("@>") {
        *i += 1;
        let right = parse_value_token(
            tokens.get(*i).ok_or("Expected JSON value after @>")?,
            params,
        )?;
        *i += 1;
        return Ok(WhereExpr::JsonContains(left, right));
    }
    let op_token = tokens
        .get(*i)
        .ok_or("Expected comparison operator in WHERE")?;
    let op = match op_token.as_str() {
        "=" => CompareOp::Eq,
        "!=" | "<>" => CompareOp::Ne,
        "<" => CompareOp::Lt,
        "<=" => CompareOp::Le,
        ">" => CompareOp::Gt,
        ">=" => CompareOp::Ge,
        _ => return Err(format!("Unsupported operator in WHERE: {}", op_token)),
    };
    *i += 1;
    let right = parse_value_token(
        tokens.get(*i).ok_or("Expected value in WHERE")?,
        params,
    )?;
    *i += 1;
    Ok(WhereExpr::Compare(left, op, right))
}

fn qualify_row(row: &HashMap<String, Value>, alias: &str) -> HashMap<String, Value> {
    row.iter()
        .map(|(k, v)| (format!("{}.{}", alias, k), v.clone()))
        .collect()
}

fn row_get(row: &HashMap<String, Value>, name: &QualifiedName) -> Option<Value> {
    let base = if !name.table.is_empty() {
        let key = format!("{}.{}", name.table, name.column);
        if let Some(v) = row.get(&key) {
            Some(v.clone())
        } else {
            row.get(&name.column).cloned()
        }
    } else if let Some(v) = row.get(&name.column) {
        Some(v.clone())
    } else {
        let suffix = format!(".{}", name.column);
        row.iter()
            .find(|(k, _)| k.ends_with(&suffix))
            .map(|(_, v)| v.clone())
    }?;
    if let Some(path) = &name.json_path {
        return json_get_text(&base, path);
    }
    Some(base)
}

fn eval_where(expr: &WhereExpr, row: &HashMap<String, Value>, engine: &SqlEngine) -> bool {
    match expr {
        WhereExpr::Compare(left, op, right) => {
            let Some(left_val) = row_get(row, left) else {
                return false;
            };
            compare_values(&left_val, op, right)
        }
        WhereExpr::IsNull(col, want_null) => {
            let val = row_get(row, col);
            if *want_null {
                matches!(val, Some(Value::Null) | None)
            } else {
                matches!(val, Some(v) if !matches!(v, Value::Null))
            }
        }
        WhereExpr::In(col, vals) => {
            let Some(left_val) = row_get(row, col) else {
                return false;
            };
            vals.iter().any(|v| sql_values_equal(&left_val, v))
        }
        WhereExpr::InSubquery(col, sub) => {
            let Some(left_val) = row_get(row, col) else {
                return false;
            };
            engine
                .execute_subquery(sub.as_ref())
                .map(|vals| vals.iter().any(|v| sql_values_equal(&left_val, v)))
                .unwrap_or(false)
        }
        WhereExpr::Like(col, pattern) => {
            let Some(Value::String(s)) = row_get(row, col) else {
                return false;
            };
            sql_like_match(&s, pattern)
        }
        WhereExpr::Ilike(col, pattern) => {
            let Some(Value::String(s)) = row_get(row, col) else {
                return false;
            };
            sql_like_match(&s.to_ascii_lowercase(), &pattern.to_ascii_lowercase())
        }
        WhereExpr::Between(col, lo, hi) => {
            let Some(val) = row_get(row, col) else {
                return false;
            };
            numeric_compare(&val, lo, |a, b| a >= b) && numeric_compare(&val, hi, |a, b| a <= b)
        }
        WhereExpr::Not(inner) => !eval_where(inner, row, engine),
        WhereExpr::JsonContains(col, right) => {
            let Some(left_val) = row_get(row, col) else {
                return false;
            };
            json_contains(&left_val, right)
        }
        WhereExpr::And(a, b) => eval_where(a, row, engine) && eval_where(b, row, engine),
        WhereExpr::Or(a, b) => eval_where(a, row, engine) || eval_where(b, row, engine),
    }
}

fn qualified_name_matches_table(qn: &QualifiedName, table_name: &str) -> bool {
    qn.table.is_empty() || qn.table == table_name
}

fn table_row_indices_for_where(
    table: &TableDef,
    table_name: &str,
    expr: &WhereExpr,
) -> Option<Vec<usize>> {
    if let Some((cols, vals)) = extract_and_eq_columns(table_name, expr) {
        if cols.len() > 1 {
            if let Some(indices) = table.row_indices_for_columns_eq(&cols, &vals) {
                return Some(indices);
            }
        }
    }
    match expr {
        WhereExpr::Compare(qn, CompareOp::Eq, val) if qn.json_path.is_none() => {
            if !qualified_name_matches_table(qn, table_name) {
                return None;
            }
            table.row_indices_for_eq(&qn.column, val)
        }
        WhereExpr::In(qn, vals) if qn.json_path.is_none() => {
            if !qualified_name_matches_table(qn, table_name) {
                return None;
            }
            table.row_indices_for_in(&qn.column, vals)
        }
        WhereExpr::And(a, b) => {
            let left = table_row_indices_for_where(table, table_name, a)?;
            let right = table_row_indices_for_where(table, table_name, b)?;
            intersect_indices(&left, &right)
        }
        _ => None,
    }
}

fn extract_and_eq_columns(table_name: &str, expr: &WhereExpr) -> Option<(Vec<String>, Vec<Value>)> {
    match expr {
        WhereExpr::Compare(qn, CompareOp::Eq, val) if qn.json_path.is_none() => {
            if !qualified_name_matches_table(qn, table_name) {
                return None;
            }
            Some((vec![qn.column.clone()], vec![val.clone()]))
        }
        WhereExpr::And(a, b) => {
            let (mut cols_a, mut vals_a) = extract_and_eq_columns(table_name, a)?;
            let (cols_b, vals_b) = extract_and_eq_columns(table_name, b)?;
            cols_a.extend(cols_b);
            vals_a.extend(vals_b);
            Some((cols_a, vals_a))
        }
        _ => None,
    }
}

fn intersect_indices(left: &[usize], right: &[usize]) -> Option<Vec<usize>> {
    let mut a = left.to_vec();
    let mut b = right.to_vec();
    a.sort_unstable();
    b.sort_unstable();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn parse_quoted_path(sql: &str, keyword: &str) -> Result<String, String> {
    let rest = sql[keyword.len()..].trim();
    let path = if let Some(inner) = rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        inner.replace("\\'", "'")
    } else if let Some(inner) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        inner.replace("\\\"", "\"")
    } else {
        rest.split_whitespace()
            .next()
            .ok_or("Expected database file path")?
            .to_string()
    };
    if path.is_empty() {
        return Err("Expected database file path".into());
    }
    Ok(path.to_string())
}

fn sql_like_match(text: &str, pattern: &str) -> bool {
    if !pattern.contains('%') && !pattern.contains('_') {
        return text == pattern;
    }
    let parts: Vec<&str> = pattern.split('%').collect();
    if parts.len() == 1 {
        return text == pattern;
    }
    let mut pos = 0usize;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 {
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if idx == parts.len() - 1 {
            return text[pos..].ends_with(part);
        } else if let Some(found) = text[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

fn compare_values(left: &Value, op: &CompareOp, right: &Value) -> bool {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return false;
    }
    match op {
        CompareOp::Eq => sql_values_equal(left, right),
        CompareOp::Ne => !sql_values_equal(left, right),
        CompareOp::Lt => numeric_compare(left, right, |a, b| a < b),
        CompareOp::Le => numeric_compare(left, right, |a, b| a <= b),
        CompareOp::Gt => numeric_compare(left, right, |a, b| a > b),
        CompareOp::Ge => numeric_compare(left, right, |a, b| a >= b),
    }
}

fn numeric_compare<F>(left: &Value, right: &Value, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let Some(a) = to_f64(left) else {
        return false;
    };
    let Some(b) = to_f64(right) else {
        return false;
    };
    cmp(a, b)
}

fn to_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn project_row(
    row: &HashMap<String, Value>,
    items: &[SelectItem],
) -> Result<HashMap<String, Value>, String> {
    let mut out = HashMap::new();
    for item in items {
        match item {
            SelectItem::All => out.extend(row.clone()),
            SelectItem::Column(name) => {
                let key = if name.table.is_empty() {
                    name.column.clone()
                } else {
                    format!("{}.{}", name.table, name.column)
                };
                let val = row_get(row, name).ok_or_else(|| format!("Unknown column: {}", key))?;
                out.insert(key, val);
            }
            SelectItem::CountAll
            | SelectItem::CountColumn(_)
            | SelectItem::Sum(_)
            | SelectItem::Avg(_)
            | SelectItem::Min(_)
            | SelectItem::Max(_) => {}
        }
    }
    Ok(out)
}

fn select_item_result_key(item: &SelectItem) -> Option<String> {
    match item {
        SelectItem::CountAll => Some("count".to_string()),
        SelectItem::CountColumn(name) => Some(format!("count({})", qualified_key(name))),
        SelectItem::Sum(name) => Some(format!("sum({})", qualified_key(name))),
        SelectItem::Avg(name) => Some(format!("avg({})", qualified_key(name))),
        SelectItem::Min(name) => Some(format!("min({})", qualified_key(name))),
        SelectItem::Max(name) => Some(format!("max({})", qualified_key(name))),
        SelectItem::Column(name) => Some(qualified_key(name)),
        SelectItem::All => None,
    }
}

fn format_select_result(rows: &[HashMap<String, Value>], items: &[SelectItem]) -> Value {
    if rows.is_empty() {
        return Value::Array(Vec::new());
    }

    let column_keys: Vec<String> = if items.iter().any(|item| {
        matches!(
            item,
            SelectItem::CountAll
                | SelectItem::CountColumn(_)
                | SelectItem::Sum(_)
                | SelectItem::Avg(_)
                | SelectItem::Min(_)
                | SelectItem::Max(_)
        )
    }) {
        items.iter().filter_map(select_item_result_key).collect()
    } else if items.iter().all(|item| matches!(item, SelectItem::Column(_))) {
        items
            .iter()
            .map(|item| {
                let SelectItem::Column(name) = item else {
                    unreachable!()
                };
                if name.table.is_empty() {
                    name.column.clone()
                } else {
                    format!("{}.{}", name.table, name.column)
                }
            })
            .collect()
    } else if items.len() == 1 && matches!(items.first(), Some(SelectItem::All)) {
        rows[0].keys().cloned().collect()
    } else {
        rows[0].keys().cloned().collect()
    };

    if rows.len() == 1 && column_keys.len() == 1 {
        let key = &column_keys[0];
        return rows[0].get(key).cloned().unwrap_or(Value::Null);
    }

    if column_keys.len() == 1 {
        let key = &column_keys[0];
        return Value::Array(
            rows.iter()
                .map(|row| row.get(key).cloned().unwrap_or(Value::Null))
                .collect(),
        );
    }

    Value::Array(
        rows.iter()
            .map(|row| {
                Value::Array(
                    column_keys
                        .iter()
                        .map(|key| row.get(key).cloned().unwrap_or(Value::Null))
                        .collect(),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_insert_select_where() {
        let mut engine = SqlEngine::new();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT)", &[])
            .unwrap();
        engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Ada')", &[])
            .unwrap();
        engine
            .execute("INSERT INTO users (id, name) VALUES (2, 'Bob')", &[])
            .unwrap();
        let result = engine
            .execute("SELECT name FROM users WHERE id = $1", &[Value::Number(1)])
            .unwrap();
        assert!(matches!(result, Value::String(s) if s == "Ada"));
    }

    #[test]
    fn join_two_tables() {
        let mut engine = SqlEngine::new();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT)", &[])
            .unwrap();
        engine
            .execute("CREATE TABLE orders (id INTEGER, user_id INTEGER, total INTEGER)", &[])
            .unwrap();
        engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Ada')", &[])
            .unwrap();
        engine
            .execute("INSERT INTO orders (id, user_id, total) VALUES (1, 1, 100)", &[])
            .unwrap();
        let result = engine
            .execute(
                "SELECT users.name, orders.total FROM users JOIN orders ON users.id = orders.user_id",
                &[],
            )
            .unwrap();
        assert!(matches!(
            result,
            Value::Array(rows)
                if rows.len() == 1
                && matches!(
                    &rows[0],
                    Value::Array(cols)
                        if cols.len() == 2
                        && matches!(&cols[0], Value::String(s) if s == "Ada")
                        && matches!(&cols[1], Value::Number(100))
                )
        ));
    }

    #[test]
    fn update_delete_order_count() {
        fn assert_number(val: Value, n: i64) {
            match val {
                Value::Number(v) => assert_eq!(v, n),
                other => panic!("expected Number({}), got {:?}", n, other),
            }
        }

        let mut engine = SqlEngine::new();
        engine
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT, score INTEGER)", &[])
            .unwrap();
        engine
            .execute("INSERT INTO items (id, name, score) VALUES (1, 'Ada', 10)", &[])
            .unwrap();
        engine
            .execute("INSERT INTO items (id, name, score) VALUES (2, 'Bob', 20)", &[])
            .unwrap();
        engine
            .execute("UPDATE items SET score = 99 WHERE id = 1", &[])
            .unwrap();
        let updated = engine
            .execute("SELECT score FROM items WHERE id = 1", &[])
            .unwrap();
        assert_number(updated, 99);
        let ordered = engine
            .execute("SELECT id FROM items ORDER BY score DESC LIMIT 1", &[])
            .unwrap();
        assert_number(ordered, 1);
        let count = engine
            .execute("SELECT COUNT(*) FROM items", &[])
            .unwrap();
        assert_number(count, 2);
        engine
            .execute("DELETE FROM items WHERE id = 2", &[])
            .unwrap();
        let after = engine
            .execute("SELECT COUNT(*) FROM items", &[])
            .unwrap();
        assert_number(after, 1);
    }

    #[test]
    fn primary_key_rejects_duplicate() {
        let mut engine = SqlEngine::new();
        engine
            .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[])
            .unwrap();
        engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Ada')", &[])
            .unwrap();
        let err = engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Bob')", &[])
            .unwrap_err();
        assert!(err.contains("Duplicate key value"));
    }

    #[test]
    fn json_path_and_contains() {
        let mut e = SqlEngine::new();
        e.execute("CREATE TABLE docs (id INTEGER PRIMARY KEY, body JSONB)", &[])
            .unwrap();
        let mut body = HashMap::new();
        body.insert("title".into(), Value::String("hi".into()));
        body.insert("plan".into(), Value::String("pro".into()));
        e.execute("INSERT INTO docs (id, body) VALUES (1, $1)", &[Value::Object(body)])
            .unwrap();
        e.execute("SELECT body FROM docs WHERE body->>'title' = 'hi'", &[])
            .unwrap();
        let mut probe = HashMap::new();
        probe.insert("plan".into(), Value::String("pro".into()));
        let v2 = e
            .execute("SELECT id FROM docs WHERE body @> $1", &[Value::Object(probe)])
            .unwrap();
        assert!(matches!(v2, Value::Number(1)));
    }
}
