//! Column statistics for cost-based planning (Phase 3).

use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ColumnStats {
    pub row_count: u64,
    pub null_count: u64,
    pub distinct_estimate: u64,
    pub min_num: Option<f64>,
    pub max_num: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct TableStats {
    pub row_count: u64,
    pub columns: HashMap<String, ColumnStats>,
    pub last_analyzed: u64,
}

impl TableStats {
    pub fn analyze(
        column_names: &[String],
        rows: impl Iterator<Item = HashMap<String, Value>>,
    ) -> Self {
        let mut col_stats: HashMap<String, ColumnStats> = HashMap::new();
        for name in column_names {
            col_stats.insert(name.clone(), ColumnStats::default());
        }
        let mut row_count = 0u64;
        for row in rows {
            row_count += 1;
            for name in column_names {
                let st = col_stats.get_mut(name).unwrap();
                st.row_count += 1;
                match row.get(name) {
                    None | Some(Value::Null) => st.null_count += 1,
                    Some(v) => {
                        if let Some(n) = value_to_f64(v) {
                            st.min_num = Some(st.min_num.map(|m| m.min(n)).unwrap_or(n));
                            st.max_num = Some(st.max_num.map(|m| m.max(n)).unwrap_or(n));
                        }
                    }
                }
            }
        }
        for st in col_stats.values_mut() {
            st.distinct_estimate = st.row_count.saturating_sub(st.null_count).max(1);
        }
        TableStats {
            row_count,
            columns: col_stats,
            last_analyzed: row_count,
        }
    }

    pub fn estimate_selectivity(&self, column: &str, fraction: f64) -> f64 {
        let base = self.row_count.max(1) as f64;
        let col = self.columns.get(column);
        let distinct = col.map(|c| c.distinct_estimate.max(1) as f64).unwrap_or(base);
        (base / distinct * fraction).max(1.0)
    }
}

fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}
