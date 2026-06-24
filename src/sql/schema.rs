//! Table schema, constraints, and validation for Kabootar SQL v2.

use crate::sql::storage::btree::BPlusTree;
use crate::sql::storage::partition::PartitionSpec;
use crate::sql::storage::row_store::{RowSlot, RowStore};
use crate::sql::storage::stats::TableStats;
use crate::value::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlType {
    Integer,
    Text,
    Float,
    Bool,
    Json,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub sql_type: SqlType,
    pub not_null: bool,
    pub unique: bool,
    pub serial: bool,
}

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub struct ForeignKeyDef {
    pub column: String,
    pub ref_table: String,
    pub ref_column: String,
}

#[derive(Debug, Clone)]
pub struct CheckDef {
    pub column: String,
    pub op: String,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct TableDef {
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<String>,
    store: RowStore,
    pub indexes: Vec<IndexDef>,
    pub serial_counters: HashMap<String, i64>,
    pub foreign_keys: Vec<ForeignKeyDef>,
    pub checks: Vec<CheckDef>,
    pub stats: TableStats,
    pub partition: Option<PartitionSpec>,
    pub heap_page: Option<u64>,
    /// PK column value → row slot (O(1) lookup).
    pub pk_index: BTreeMap<String, RowSlot>,
    /// B+tree indexes for range-capable lookups.
    pub btree_indexes: HashMap<String, BPlusTree>,
    /// Legacy hash index mirror for compatibility during migration.
    pub index_entries: HashMap<String, BTreeMap<String, Vec<RowSlot>>>,
}

impl TableDef {
    pub fn from_rows(
        columns: Vec<ColumnDef>,
        primary_key: Option<String>,
        rows: Vec<HashMap<String, Value>>,
    ) -> Self {
        let store = RowStore::from_maps(&columns, rows);
        Self {
            columns,
            primary_key,
            store,
            indexes: Vec::new(),
            serial_counters: HashMap::new(),
            foreign_keys: Vec::new(),
            checks: Vec::new(),
            stats: TableStats::default(),
            partition: None,
            heap_page: None,
            pk_index: BTreeMap::new(),
            btree_indexes: HashMap::new(),
            index_entries: HashMap::new(),
        }
    }

    pub fn empty(columns: Vec<ColumnDef>, primary_key: Option<String>) -> Self {
        Self::from_rows(columns, primary_key, Vec::new())
    }

    pub fn live_row_count(&self) -> usize {
        self.store.live_count()
    }

    pub fn slot_count(&self) -> usize {
        self.store.slot_count()
    }

    pub fn live_slots(&self) -> Vec<RowSlot> {
        self.store.live_slots()
    }

    pub fn row_map(&self, slot: RowSlot) -> Option<HashMap<String, Value>> {
        self.store.row_as_map(slot, &self.columns)
    }

    pub fn push_row(&mut self, row: HashMap<String, Value>) -> RowSlot {
        self.store.insert_map(&row, &self.columns)
    }

    pub fn set_row_map(&mut self, slot: RowSlot, row: HashMap<String, Value>) -> Result<(), String> {
        self.store.update_map(slot, &row, &self.columns)
    }

    pub fn remove_slot(&mut self, slot: RowSlot) -> Option<HashMap<String, Value>> {
        let old = self.row_map(slot);
        self.store.delete_slot(slot);
        old
    }

    pub fn iter_live_maps(&self) -> impl Iterator<Item = (RowSlot, HashMap<String, Value>)> + '_ {
        self.store
            .iter_live()
            .filter_map(|s| self.row_map(s).map(|m| (s, m)))
    }

    pub fn rows_for_persist(&self) -> Vec<HashMap<String, Value>> {
        self.store
            .iter_live()
            .filter_map(|s| self.row_map(s))
            .collect()
    }

    pub fn ensure_auto_indexes(&mut self) {
        if let Some(pk) = self.primary_key.clone() {
            let name = "__pk_auto__".to_string();
            if !self.indexes.iter().any(|i| i.columns == [pk.clone()]) {
                self.indexes.push(IndexDef {
                    name: name.clone(),
                    columns: vec![pk],
                    unique: true,
                });
            }
        }
        let unique_cols: Vec<String> = self
            .columns
            .iter()
            .filter(|c| c.unique && Some(&c.name) != self.primary_key.as_ref())
            .map(|c| c.name.clone())
            .collect();
        for col in unique_cols {
            let auto_name = format!("__unique_{col}__");
            if !self.indexes.iter().any(|i| i.columns == [col.clone()]) {
                self.indexes.push(IndexDef {
                    name: auto_name,
                    columns: vec![col],
                    unique: true,
                });
            }
        }
        for fk in &self.foreign_keys {
            if !self.indexes.iter().any(|i| i.columns == [fk.column.clone()]) {
                self.indexes.push(IndexDef {
                    name: format!("__fk_{}__", fk.column),
                    columns: vec![fk.column.clone()],
                    unique: false,
                });
            }
        }
    }

    pub fn analyze_stats(&mut self) {
        let names: Vec<String> = self.columns.iter().map(|c| c.name.clone()).collect();
        let rows: Vec<HashMap<String, Value>> = self.rows_for_persist();
        self.stats = TableStats::analyze(&names, rows.into_iter());
    }

    pub fn index_only_lookup(&self, column: &str, value: &Value) -> Option<Value> {
        if self.primary_key.as_deref() == Some(column) {
            return Some(value.clone());
        }
        let key = value_to_index_key(value)?;
        for idx in &self.indexes {
            if idx.columns == [column] {
                if let Some(tree) = self.btree_indexes.get(&idx.name) {
                    if tree.unique {
                        return tree.lookup_eq(&key).and_then(|_| Some(value.clone()));
                    }
                }
            }
        }
        None
    }

    pub fn store_mut(&mut self) -> &mut RowStore {
        &mut self.store
    }

    pub fn reload_store(&mut self, rows: Vec<HashMap<String, Value>>) {
        self.store = RowStore::from_maps(&self.columns, rows);
    }

    pub fn store(&self) -> &RowStore {
        &self.store
    }
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }

    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn allocate_serial(&mut self, col: &str) -> i64 {
        let next = self.serial_counters.entry(col.to_string()).or_insert(1);
        let v = *next;
        *next += 1;
        v
    }

    pub fn init_serial_counters(&mut self) {
        for col in &self.columns {
            if col.serial {
                let max_id = self
                    .store
                    .iter_live()
                    .filter_map(|s| self.store.get_column(s, &col.name))
                    .filter_map(|v| value_to_i64(&v))
                    .max()
                    .unwrap_or(0);
                self.serial_counters
                    .insert(col.name.clone(), max_id.saturating_add(1));
            }
        }
    }

    pub fn check_not_null(&self, row: &HashMap<String, Value>) -> Result<(), String> {
        for col in &self.columns {
            if !col.not_null && !col.serial {
                continue;
            }
            match row.get(&col.name) {
                None | Some(Value::Null) => {
                    return Err(format!("NOT NULL violation on column {}", col.name));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn check_unique(
        &self,
        row: &HashMap<String, Value>,
        skip_row_index: Option<usize>,
    ) -> Result<(), String> {
        for slot in self.store.iter_live() {
            if skip_row_index == Some(slot) {
                continue;
            }
            let Some(existing) = self.row_map(slot) else {
                continue;
            };
            for col in &self.columns {
                let is_unique = col.unique
                    || self.primary_key.as_ref() == Some(&col.name);
                if !is_unique {
                    continue;
                }
                let new_val = row.get(&col.name);
                let old_val = existing.get(&col.name);
                if let (Some(n), Some(o)) = (new_val, old_val) {
                    if !matches!(n, Value::Null) && sql_values_equal(n, o) {
                        return Err(format!("UNIQUE violation on column {}", col.name));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn find_conflict_row(
        &self,
        conflict_cols: &[String],
        row: &HashMap<String, Value>,
    ) -> Option<usize> {
        if !conflict_cols.is_empty() {
            if conflict_cols.len() == 1 {
                let col = &conflict_cols[0];
                let val = row.get(col)?;
                return self.find_row_index_where_eq(col, val);
            }
            let values: Vec<Value> = conflict_cols
                .iter()
                .map(|c| row.get(c).cloned().unwrap_or(Value::Null))
                .collect();
            return self.find_row_index_where_columns_eq(conflict_cols, &values);
        }
        if let Some(pk) = &self.primary_key {
            if let Some(val) = row.get(pk) {
                if let Some(idx) = self.pk_row_index(val) {
                    return Some(idx);
                }
            }
        }
        for col in &self.columns {
            if col.unique {
                if let Some(val) = row.get(&col.name) {
                    if let Some(idx) = self.find_row_index_where_eq(&col.name, val) {
                        return Some(idx);
                    }
                }
            }
        }
        for idx_def in &self.indexes {
            if idx_def.unique {
                if let Some(key) = row_index_key(row, &idx_def.columns) {
                    if let Some(entries) = self.index_entries.get(&idx_def.name) {
                        if let Some(indices) = entries.get(&key) {
                            if let Some(&hit) = indices.first() {
                                return Some(hit);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn check_checks(&self, row: &HashMap<String, Value>) -> Result<(), String> {
        for chk in &self.checks {
            let Some(val) = row.get(&chk.column) else {
                continue;
            };
            if !check_compare(val, &chk.op, &chk.value) {
                return Err(format!(
                    "CHECK constraint failed on column {}",
                    chk.column
                ));
            }
        }
        Ok(())
    }

    pub fn check_foreign_keys(
        &self,
        row: &HashMap<String, Value>,
        tables: &HashMap<String, TableDef>,
    ) -> Result<(), String> {
        for fk in &self.foreign_keys {
            let Some(val) = row.get(&fk.column) else {
                continue;
            };
            if matches!(val, Value::Null) {
                continue;
            }
            let ref_table = tables
                .get(&fk.ref_table)
                .ok_or_else(|| format!("Unknown referenced table: {}", fk.ref_table))?;
            let found = ref_table.store.iter_live().any(|s| {
                ref_table
                    .store
                    .get_column(s, &fk.ref_column)
                    .map(|rv| sql_values_equal(&rv, val))
                    .unwrap_or(false)
            });
            if !found {
                return Err(format!(
                    "FOREIGN KEY violation: {} -> {}.{}",
                    fk.column, fk.ref_table, fk.ref_column
                ));
            }
        }
        Ok(())
    }

    pub fn validate_row(
        &self,
        row: &HashMap<String, Value>,
        skip_row_index: Option<usize>,
        tables: &HashMap<String, TableDef>,
    ) -> Result<(), String> {
        self.check_not_null(row)?;
        self.check_unique(row, skip_row_index)?;
        self.check_index_unique(row, skip_row_index)?;
        self.check_checks(row)?;
        self.check_foreign_keys(row, tables)?;
        Ok(())
    }

    pub fn check_delete_referenced(
        &self,
        table_name: &str,
        row: &HashMap<String, Value>,
        tables: &HashMap<String, TableDef>,
    ) -> Result<(), String> {
        for (other_name, other) in tables {
            for fk in &other.foreign_keys {
                if fk.ref_table != table_name {
                    continue;
                }
                let Some(ref_val) = row.get(&fk.ref_column) else {
                    continue;
                };
                for slot in other.store.iter_live() {
                    if let Some(v) = other.store.get_column(slot, &fk.column) {
                        if sql_values_equal(&v, ref_val) {
                            return Err(format!(
                                "FOREIGN KEY violation: row referenced by {}.{}",
                                other_name, fk.column
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn find_row_index_where_eq(&self, column: &str, value: &Value) -> Option<usize> {
        self.row_indices_for_eq(column, value)
            .and_then(|v| v.first().copied())
            .or_else(|| {
                for slot in self.store.iter_live() {
                    if let Some(rv) = self.store.get_column(slot, column) {
                        if sql_values_equal(&rv, value) {
                            return Some(slot);
                        }
                    }
                }
                None
            })
    }

    pub fn find_row_index_where_columns_eq(
        &self,
        columns: &[String],
        values: &[Value],
    ) -> Option<usize> {
        self.row_indices_for_columns_eq(columns, values)
            .and_then(|v| v.first().copied())
            .or_else(|| {
                for slot in self.store.iter_live() {
                    if columns.iter().zip(values.iter()).all(|(c, v)| {
                        self.store
                            .get_column(slot, c)
                            .map(|rv| sql_values_equal(&rv, v))
                            .unwrap_or(false)
                    }) {
                        return Some(slot);
                    }
                }
                None
            })
    }

    pub fn row_indices_for_eq(&self, column: &str, value: &Value) -> Option<Vec<usize>> {
        let key = value_to_index_key(value)?;
        if self.primary_key.as_deref() == Some(column) {
            return self.pk_index.get(&key).map(|&i| vec![i]);
        }
        for idx in &self.indexes {
            if idx.columns.len() == 1 && idx.columns[0] == column {
                return self
                    .index_entries
                    .get(&idx.name)
                    .and_then(|entries| entries.get(&key).cloned());
            }
        }
        for idx in &self.indexes {
            if idx.columns.first().map(|s| s.as_str()) == Some(column) && idx.columns.len() > 1 {
                let entries = self.index_entries.get(&idx.name)?;
                let prefix = format!("{key}\x00");
                let mut out = Vec::new();
                for (k, indices) in entries.iter() {
                    if k == &key || k.starts_with(&prefix) {
                        out.extend(indices.iter().copied());
                    }
                }
                if !out.is_empty() {
                    out.sort_unstable();
                    out.dedup();
                    return Some(out);
                }
            }
        }
        None
    }

    pub fn row_indices_for_columns_eq(
        &self,
        columns: &[String],
        values: &[Value],
    ) -> Option<Vec<usize>> {
        if columns.is_empty() || columns.len() != values.len() {
            return None;
        }
        if columns.len() == 1 {
            return self.row_indices_for_eq(&columns[0], &values[0]);
        }
        let key = row_index_key_from_values(values)?;
        for idx in &self.indexes {
            if idx.columns == *columns {
                return self
                    .index_entries
                    .get(&idx.name)
                    .and_then(|entries| entries.get(&key).cloned());
            }
        }
        None
    }

    pub fn row_indices_for_in(&self, column: &str, values: &[Value]) -> Option<Vec<usize>> {
        let mut out = Vec::new();
        for value in values {
            let Some(mut hits) = self.row_indices_for_eq(column, value) else {
                return None;
            };
            out.append(&mut hits);
        }
        out.sort_unstable();
        out.dedup();
        Some(out)
    }

    pub fn pk_row_index(&self, value: &Value) -> Option<usize> {
        let key = value_to_index_key(value)?;
        self.pk_index.get(&key).copied()
    }

    pub fn rebuild_all_indexes(&mut self) {
        self.pk_index.clear();
        self.index_entries.clear();
        self.btree_indexes.clear();
        if self.primary_key.is_some() {
            self.rebuild_pk_index();
        }
        let names: Vec<String> = self.indexes.iter().map(|i| i.name.clone()).collect();
        for name in names {
            self.rebuild_index(&name);
        }
    }

    pub fn rebuild_pk_index(&mut self) {
        self.pk_index.clear();
        let Some(pk) = self.primary_key.clone() else {
            return;
        };
        for slot in self.store.iter_live() {
            if let Some(val) = self.store.get_column(slot, &pk) {
                if let Some(key) = value_to_index_key(&val) {
                    self.pk_index.insert(key, slot);
                }
            }
        }
    }

    pub fn rebuild_index(&mut self, index_name: &str) {
        let Some(idx_def) = self.indexes.iter().find(|i| i.name == index_name).cloned() else {
            return;
        };
        let mut entries = BTreeMap::new();
        let mut tree = BPlusTree::new(idx_def.unique);
        for slot in self.store.iter_live() {
            if let Some(row) = self.row_map(slot) {
                if let Some(key) = row_index_key(&row, &idx_def.columns) {
                    entries.entry(key.clone()).or_insert_with(Vec::new).push(slot);
                    tree.insert(key, slot);
                }
            }
        }
        self.index_entries.insert(index_name.to_string(), entries);
        self.btree_indexes.insert(index_name.to_string(), tree);
    }

    pub fn register_row(&mut self, row_slot: RowSlot, row: &HashMap<String, Value>) {
        if let Some(pk) = self.primary_key.clone() {
            if let Some(val) = row.get(&pk) {
                if let Some(key) = value_to_index_key(val) {
                    self.pk_index.insert(key, row_slot);
                }
            }
        }
        let index_defs: Vec<IndexDef> = self.indexes.clone();
        for idx_def in index_defs {
            if let Some(key) = row_index_key(row, &idx_def.columns) {
                self.index_entries
                    .entry(idx_def.name.clone())
                    .or_default()
                    .entry(key.clone())
                    .or_default()
                    .push(row_slot);
                self.btree_indexes
                    .entry(idx_def.name.clone())
                    .or_insert_with(|| BPlusTree::new(idx_def.unique))
                    .insert(key, row_slot);
            }
        }
    }

    pub fn replace_row(
        &mut self,
        row_slot: RowSlot,
        old_row: &HashMap<String, Value>,
        new_row: &HashMap<String, Value>,
    ) {
        self.unregister_row(row_slot, old_row);
        self.register_row(row_slot, new_row);
    }

    pub fn unregister_row(&mut self, row_slot: RowSlot, row: &HashMap<String, Value>) {
        if let Some(pk) = self.primary_key.clone() {
            if let Some(val) = row.get(&pk) {
                if let Some(key) = value_to_index_key(val) {
                    self.pk_index.remove(&key);
                }
            }
        }
        let index_defs: Vec<IndexDef> = self.indexes.clone();
        for idx_def in index_defs {
            if let Some(key) = row_index_key(row, &idx_def.columns) {
                if let Some(entries) = self.index_entries.get_mut(&idx_def.name) {
                    if let Some(indices) = entries.get_mut(&key) {
                        indices.retain(|&i| i != row_slot);
                        if indices.is_empty() {
                            entries.remove(&key);
                        }
                    }
                }
                if let Some(tree) = self.btree_indexes.get_mut(&idx_def.name) {
                    tree.remove(&key, row_slot);
                }
            }
        }
    }

    pub fn check_index_unique(
        &self,
        row: &HashMap<String, Value>,
        skip_row_index: Option<usize>,
    ) -> Result<(), String> {
        for idx in &self.indexes {
            if !idx.unique {
                continue;
            }
            let Some(key) = row_index_key(row, &idx.columns) else {
                continue;
            };
            let Some(entries) = self.index_entries.get(&idx.name) else {
                continue;
            };
            let Some(indices) = entries.get(&key) else {
                continue;
            };
            for &hit in indices {
                if skip_row_index != Some(hit) {
                    return Err(format!("UNIQUE index violation: {}", idx.name));
                }
            }
        }
        Ok(())
    }

    pub fn coerce_value(val: &Value, sql_type: &SqlType) -> Result<Value, String> {
        match (sql_type, val) {
            (_, Value::Null) => Ok(Value::Null),
            (SqlType::Integer, Value::Number(n)) => Ok(Value::Number(*n)),
            (SqlType::Integer, Value::Float(f)) => Ok(Value::Number(*f as i64)),
            (SqlType::Float, Value::Float(f)) => Ok(Value::Float(*f)),
            (SqlType::Float, Value::Number(n)) => Ok(Value::Float(*n as f64)),
            (SqlType::Text, Value::String(s)) => Ok(Value::String(s.clone())),
            (SqlType::Bool, Value::Bool(b)) => Ok(Value::Bool(*b)),
            (SqlType::Json, Value::Object(_)) => Ok(val.clone()),
            (SqlType::Json, Value::Array(_)) => Ok(val.clone()),
            (SqlType::Text, v) => Ok(Value::String(format!("{:?}", v))),
            (t, v) => Err(format!(
                "Type mismatch: cannot store {:?} in {:?}",
                v, t
            )),
        }
    }
}

pub fn sql_values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Number(a), Value::Float(b)) | (Value::Float(b), Value::Number(a)) => {
            (*a as f64) == *b
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

pub fn value_to_index_key(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::Number(n) => Some(format!("\x01n:{n}")),
        Value::Float(f) => Some(format!("\x01f:{f}")),
        Value::String(s) => Some(format!("\x01s:{s}")),
        Value::Bool(b) => Some(format!("\x01b:{b}")),
        _ => None,
    }
}

pub fn row_index_key(row: &HashMap<String, Value>, columns: &[String]) -> Option<String> {
    let mut parts = Vec::with_capacity(columns.len());
    for col in columns {
        let v = row.get(col)?;
        parts.push(value_to_index_key(v)?);
    }
    Some(parts.join("\x00"))
}

pub fn row_index_key_from_values(values: &[Value]) -> Option<String> {
    let parts: Vec<String> = values.iter().map(value_to_index_key).collect::<Option<_>>()?;
    if parts.len() != values.len() {
        return None;
    }
    Some(parts.join("\x00"))
}

pub fn value_to_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => Some(*n),
        Value::Float(f) => Some(*f as i64),
        _ => None,
    }
}

fn parse_sql_type(token: &str) -> SqlType {
    match token.to_ascii_uppercase().as_str() {
        "INT" | "INTEGER" | "BIGINT" | "SERIAL" => SqlType::Integer,
        "FLOAT" | "REAL" | "DOUBLE" => SqlType::Float,
        "BOOL" | "BOOLEAN" => SqlType::Bool,
        "JSON" | "JSONB" => SqlType::Json,
        _ => SqlType::Text,
    }
}

pub fn parse_column_defs(
    cols_sql: &str,
) -> Result<(Vec<ColumnDef>, Option<String>, Vec<ForeignKeyDef>, Vec<CheckDef>), String> {
    let mut columns = Vec::new();
    let mut primary_key = None;
    let mut foreign_keys = Vec::new();
    let mut checks = Vec::new();
    for part in split_column_defs(cols_sql) {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        let name = tokens
            .first()
            .ok_or_else(|| format!("Invalid column definition: {}", part))?
            .to_string();
        let upper: Vec<String> = tokens.iter().map(|t| t.to_ascii_uppercase()).collect();
        let mut sql_type = SqlType::Text;
        let mut not_null = false;
        let mut unique = false;
        let mut serial = false;
        let mut i = 1usize;
        while i < upper.len() {
            match upper[i].as_str() {
                "INTEGER" | "INT" | "BIGINT" | "TEXT" | "FLOAT" | "REAL" | "DOUBLE"
                | "BOOL" | "BOOLEAN" | "JSON" | "JSONB" => {
                    sql_type = parse_sql_type(&upper[i]);
                    i += 1;
                }
                "SERIAL" => {
                    serial = true;
                    sql_type = SqlType::Integer;
                    not_null = true;
                    i += 1;
                }
                "NOT" if upper.get(i + 1).map(|s| s.as_str()) == Some("NULL") => {
                    not_null = true;
                    i += 2;
                }
                "NULL" => {
                    i += 1;
                }
                "UNIQUE" => {
                    unique = true;
                    i += 1;
                }
                "PRIMARY" if upper.get(i + 1).map(|s| s.as_str()) == Some("KEY") => {
                    primary_key = Some(name.clone());
                    not_null = true;
                    unique = true;
                    i += 2;
                }
                "REFERENCES" => {
                    let ref_part = tokens[i + 1..].join(" ");
                    let (ref_table, ref_col) = parse_references_target(&ref_part)?;
                    foreign_keys.push(ForeignKeyDef {
                        column: name.clone(),
                        ref_table,
                        ref_column: ref_col,
                    });
                    break;
                }
                "CHECK" => {
                    if let Some(chk) = parse_check_from_part(&part) {
                        checks.push(chk);
                    }
                    break;
                }
                _ => i += 1,
            }
        }
        columns.push(ColumnDef {
            name,
            sql_type,
            not_null,
            unique,
            serial,
        });
    }
    Ok((columns, primary_key, foreign_keys, checks))
}

fn split_column_defs(cols_sql: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in cols_sql.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(cols_sql[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(cols_sql[start..].trim().to_string());
    parts.into_iter().filter(|p| !p.is_empty()).collect()
}

fn parse_references_target(s: &str) -> Result<(String, String), String> {
    let s = s.trim();
    let open = s.find('(').ok_or("Expected ( after REFERENCES table")?;
    let table = s[..open].trim().to_string();
    let col = s[open + 1..]
        .trim_end_matches(')')
        .trim()
        .to_string();
    Ok((table, if col.is_empty() { "id".into() } else { col }))
}

fn parse_check_from_part(part: &str) -> Option<CheckDef> {
    let upper = part.to_uppercase();
    let idx = upper.find("CHECK")?;
    let rest = part[idx + 5..].trim();
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    let inner: Vec<&str> = rest[open + 1..close].split_whitespace().collect();
    if inner.len() < 3 {
        return None;
    }
    let val = parse_check_literal(inner[2]).ok()?;
    Some(CheckDef {
        column: inner[0].to_string(),
        op: inner[1].to_string(),
        value: val,
    })
}

fn parse_check_literal(token: &str) -> Result<Value, String> {
    if token.eq_ignore_ascii_case("true") {
        return Ok(Value::Bool(true));
    }
    if token.eq_ignore_ascii_case("false") {
        return Ok(Value::Bool(false));
    }
    if let Ok(n) = token.parse::<i64>() {
        return Ok(Value::Number(n));
    }
    if let Ok(f) = token.parse::<f64>() {
        return Ok(Value::Float(f));
    }
    if (token.starts_with('\'') && token.ends_with('\''))
        || (token.starts_with('"') && token.ends_with('"'))
    {
        return Ok(Value::String(token[1..token.len() - 1].to_string()));
    }
    Err(format!("Invalid CHECK literal: {token}"))
}

fn check_compare(left: &Value, op: &str, right: &Value) -> bool {
    let Some(a) = value_to_f64(left) else {
        return false;
    };
    let Some(b) = value_to_f64(right) else {
        return false;
    };
    match op {
        ">" => a > b,
        ">=" => a >= b,
        "<" => a < b,
        "<=" => a <= b,
        "=" | "==" => (a - b).abs() < f64::EPSILON,
        "!=" | "<>" => (a - b).abs() >= f64::EPSILON,
        _ => false,
    }
}

fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

// legacy wrapper - kept for internal use
pub fn parse_column_defs_simple(
    cols_sql: &str,
) -> Result<(Vec<ColumnDef>, Option<String>), String> {
    let (cols, pk, _, _) = parse_column_defs(cols_sql)?;
    Ok((cols, pk))
}
