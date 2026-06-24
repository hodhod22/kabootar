//! Compact column-ordered row heap with tombstones and slot reuse.

use crate::sql::schema::{ColumnDef, SqlType};
use crate::value::Value;
use std::collections::HashMap;

/// Stable row slot — survives deletes until compaction.
pub type RowSlot = usize;

#[derive(Debug, Clone, Default)]
pub struct RowStore {
    data: Vec<Vec<Value>>,
    alive: Vec<bool>,
    free_list: Vec<RowSlot>,
    col_index: HashMap<String, usize>,
}

impl RowStore {
    pub fn with_columns(columns: &[ColumnDef]) -> Self {
        let mut col_index = HashMap::new();
        for (i, c) in columns.iter().enumerate() {
            col_index.insert(c.name.clone(), i);
        }
        Self {
            data: Vec::new(),
            alive: Vec::new(),
            free_list: Vec::new(),
            col_index,
        }
    }

    pub fn from_maps(columns: &[ColumnDef], rows: Vec<HashMap<String, Value>>) -> Self {
        let mut store = Self::with_columns(columns);
        for row in rows {
            store.insert_map(&row, columns);
        }
        store
    }

    pub fn rebuild_col_index(&mut self, columns: &[ColumnDef]) {
        self.col_index.clear();
        for (i, c) in columns.iter().enumerate() {
            self.col_index.insert(c.name.clone(), i);
        }
    }

    pub fn map_to_vec(
        row: &HashMap<String, Value>,
        columns: &[ColumnDef],
    ) -> Result<Vec<Value>, String> {
        let mut out = Vec::with_capacity(columns.len());
        for col in columns {
            let val = row.get(&col.name).unwrap_or(&Value::Null);
            out.push(TableDefCoerce::coerce_value(val, &col.sql_type)?);
        }
        Ok(out)
    }

    pub fn insert_map(&mut self, row: &HashMap<String, Value>, columns: &[ColumnDef]) -> RowSlot {
        let vals = Self::map_to_vec(row, columns).unwrap_or_else(|_| {
            columns
                .iter()
                .map(|c| row.get(&c.name).cloned().unwrap_or(Value::Null))
                .collect()
        });
        self.insert_vec(vals)
    }

    pub fn insert_vec(&mut self, values: Vec<Value>) -> RowSlot {
        if let Some(slot) = self.free_list.pop() {
            self.data[slot] = values;
            self.alive[slot] = true;
            slot
        } else {
            let slot = self.data.len();
            self.data.push(values);
            self.alive.push(true);
            slot
        }
    }

    pub fn update_map(
        &mut self,
        slot: RowSlot,
        row: &HashMap<String, Value>,
        columns: &[ColumnDef],
    ) -> Result<(), String> {
        if slot >= self.data.len() || !self.alive[slot] {
            return Err("Invalid row slot".into());
        }
        self.data[slot] = Self::map_to_vec(row, columns)?;
        Ok(())
    }

    pub fn row_as_map(&self, slot: RowSlot, columns: &[ColumnDef]) -> Option<HashMap<String, Value>> {
        if slot >= self.data.len() || !self.alive[slot] {
            return None;
        }
        let mut out = HashMap::new();
        for (i, col) in columns.iter().enumerate() {
            if let Some(v) = self.data[slot].get(i) {
                out.insert(col.name.clone(), v.clone());
            }
        }
        Some(out)
    }

    pub fn get_column(&self, slot: RowSlot, col: &str) -> Option<Value> {
        if slot >= self.data.len() || !self.alive[slot] {
            return None;
        }
        let idx = *self.col_index.get(col)?;
        self.data[slot].get(idx).cloned()
    }

    pub fn delete_slot(&mut self, slot: RowSlot) -> Option<Vec<Value>> {
        if slot >= self.data.len() || !self.alive[slot] {
            return None;
        }
        self.alive[slot] = false;
        self.free_list.push(slot);
        Some(self.data[slot].clone())
    }

    pub fn is_alive(&self, slot: RowSlot) -> bool {
        slot < self.alive.len() && self.alive[slot]
    }

    pub fn slot_count(&self) -> usize {
        self.data.len()
    }

    pub fn live_count(&self) -> usize {
        self.alive.iter().filter(|a| **a).count()
    }

    pub fn live_slots(&self) -> Vec<RowSlot> {
        self.alive
            .iter()
            .enumerate()
            .filter_map(|(i, a)| if *a { Some(i) } else { None })
            .collect()
    }

    pub fn iter_live(&self) -> impl Iterator<Item = RowSlot> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter_map(|(i, a)| if *a { Some(i) } else { None })
    }

    pub fn compact(&mut self) -> usize {
        let mut new_data = Vec::new();
        let mut remap = HashMap::new();
        for slot in self.iter_live() {
            let new_idx = new_data.len();
            remap.insert(slot, new_idx);
            new_data.push(self.data[slot].clone());
        }
        let removed = self.data.len() - new_data.len();
        self.data = new_data;
        self.alive = vec![true; self.data.len()];
        self.free_list.clear();
        removed
    }

    pub fn data_slice(&self, slot: RowSlot) -> Option<&[Value]> {
        if self.is_alive(slot) {
            Some(&self.data[slot])
        } else {
            None
        }
    }

    pub fn all_data(&self) -> &[Vec<Value>] {
        &self.data
    }

    pub fn all_alive(&self) -> &[bool] {
        &self.alive
    }

    pub fn load_raw(data: Vec<Vec<Value>>, alive: Vec<bool>) -> Self {
        Self {
            data,
            alive,
            free_list: Vec::new(),
            col_index: HashMap::new(),
        }
    }
}

/// Avoid circular import — mirror coerce here for row_store only.
struct TableDefCoerce;
impl TableDefCoerce {
    fn coerce_value(val: &Value, sql_type: &SqlType) -> Result<Value, String> {
        match (sql_type, val) {
            (_, Value::Null) => Ok(Value::Null),
            (SqlType::Integer, Value::Number(n)) => Ok(Value::Number(*n)),
            (SqlType::Integer, Value::Float(f)) => Ok(Value::Number(*f as i64)),
            (SqlType::Float, Value::Float(f)) => Ok(Value::Float(*f)),
            (SqlType::Float, Value::Number(n)) => Ok(Value::Float(*n as f64)),
            (SqlType::Text, Value::String(s)) => Ok(Value::String(s.clone())),
            (SqlType::Bool, Value::Bool(b)) => Ok(Value::Bool(*b)),
            (SqlType::Json, Value::Object(_)) | (SqlType::Json, Value::Array(_)) => Ok(val.clone()),
            (SqlType::Text, v) => Ok(Value::String(format!("{:?}", v))),
            (t, v) => Err(format!("Type mismatch: cannot store {:?} in {:?}", v, t)),
        }
    }
}
