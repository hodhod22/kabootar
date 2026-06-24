//! Table partitioning (Phase 3).

use crate::value::Value;

#[derive(Debug, Clone)]
pub struct PartitionRange {
    pub name: String,
    pub min: Option<Value>,
    pub max: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct PartitionSpec {
    pub column: String,
    pub ranges: Vec<PartitionRange>,
}

impl PartitionSpec {
    pub fn route(&self, row: &std::collections::HashMap<String, Value>) -> Option<&str> {
        let val = row.get(&self.column)?;
        for range in &self.ranges {
            if partition_contains(val, &range.min, &range.max) {
                return Some(&range.name);
            }
        }
        None
    }
}

fn partition_contains(val: &Value, min: &Option<Value>, max: &Option<Value>) -> bool {
    let Some(n) = value_to_f64(val) else {
        return false;
    };
    if let Some(minv) = min {
        if let Some(m) = value_to_f64(minv) {
            if n < m {
                return false;
            }
        }
    }
    if let Some(maxv) = max {
        if let Some(m) = value_to_f64(maxv) {
            if n >= m {
                return false;
            }
        }
    }
    true
}

fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

pub fn parse_partition_clause(sql: &str) -> Option<PartitionSpec> {
    let upper = sql.to_uppercase();
    if !upper.contains("PARTITION BY RANGE") {
        return None;
    }
    let idx = upper.find("PARTITION BY RANGE")?;
    let rest = &sql[idx + "PARTITION BY RANGE".len()..];
    let open = rest.find('(')?;
    let col = rest[..open].trim().trim_matches('(').to_string();
    let close = rest.rfind(')')?;
    let body = &rest[open + 1..close];
    let mut ranges = Vec::new();
    for part in body.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if let Some(r) = parse_range_part(p) {
            ranges.push(r);
        }
    }
    if ranges.is_empty() {
        return None;
    }
    Some(PartitionSpec { column: col, ranges })
}

fn parse_range_part(s: &str) -> Option<PartitionRange> {
    let upper = s.to_uppercase();
    if !upper.starts_with("PARTITION") {
        return None;
    }
    let name_start = s.find(' ')? + 1;
    let values_idx = upper.find("VALUES LESS THAN")?;
    let name = s[name_start..values_idx].trim().to_string();
    let val_str = s[values_idx + "VALUES LESS THAN".len()..].trim();
    let max = parse_bound(val_str);
    Some(PartitionRange {
        name,
        min: None,
        max,
    })
}

fn parse_bound(s: &str) -> Option<Value> {
    let t = s.trim().trim_matches(|c| c == '(' || c == ')');
    if let Ok(n) = t.parse::<i64>() {
        return Some(Value::Number(n));
    }
    None
}
