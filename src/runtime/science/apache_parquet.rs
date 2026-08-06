//! Apache Parquet read/write for flat DOUBLE / INT64 / UTF8 columns (DATA/SC).
//! Host only — wasm keeps KPQT1 via the caller.

#![cfg(not(target_arch = "wasm32"))]

use crate::runtime::science::helpers::{float_out, int_out, num};
use crate::value::Value;
use arrow_array::{Array, ArrayRef, BooleanArray, Float64Array, Int32Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

fn prefer_apache(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".parquet") || lower.ends_with(".parq")
}

pub fn path_wants_apache(path: &str) -> bool {
    prefer_apache(path)
}

pub fn is_apache_magic(buf: &[u8]) -> bool {
    buf.len() >= 4 && &buf[0..4] == b"PAR1"
}

/// Infer column types from row objects (same rules as KPQT1).
fn collect_columns(rows: &[Value]) -> Result<(Vec<String>, Vec<u8>, Vec<Vec<Value>>), String> {
    if rows.is_empty() {
        return Err("parquet_save: empty".into());
    }
    let mut col_names: Vec<String> = Vec::new();
    if let Value::Object(first) = &rows[0] {
        for k in first.keys() {
            if k.starts_with("__kab_") {
                continue;
            }
            col_names.push(k.clone());
        }
        col_names.sort();
    } else {
        return Err("parquet_save: expect array of objects".into());
    }
    let ncols = col_names.len();
    let nrows = rows.len();
    let mut dtypes = vec![1u8; ncols];
    let mut cols: Vec<Vec<Value>> = vec![Vec::with_capacity(nrows); ncols];
    for row in rows {
        let Value::Object(m) = row else {
            return Err("parquet_save: jagged rows".into());
        };
        for (ci, name) in col_names.iter().enumerate() {
            let cell = m.get(name).cloned().unwrap_or(Value::Null);
            match &cell {
                Value::String(_) => dtypes[ci] = 3,
                Value::Number(_) | Value::BigInt(_) => {
                    if dtypes[ci] == 1 {
                        dtypes[ci] = 2;
                    }
                }
                Value::Float(_) => {
                    if dtypes[ci] == 2 {
                        dtypes[ci] = 1;
                    }
                }
                Value::Bool(_) | Value::Null => {}
                _ => {
                    if dtypes[ci] != 3 {
                        dtypes[ci] = 3;
                    }
                }
            }
            cols[ci].push(cell);
        }
    }
    for (ci, col) in cols.iter().enumerate() {
        if dtypes[ci] == 2 {
            for c in col {
                if matches!(c, Value::Float(_)) {
                    dtypes[ci] = 1;
                    break;
                }
            }
        }
    }
    Ok((col_names, dtypes, cols))
}

pub fn apache_parquet_save(path: &str, rows: &[Value]) -> Result<i64, String> {
    let (col_names, dtypes, cols) = collect_columns(rows)?;
    let nrows = rows.len();
    let mut fields = Vec::with_capacity(col_names.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(col_names.len());
    for (ci, name) in col_names.iter().enumerate() {
        match dtypes[ci] {
            1 => {
                fields.push(Field::new(name, DataType::Float64, true));
                let data: Vec<Option<f64>> = cols[ci]
                    .iter()
                    .map(|c| match c {
                        Value::Null => None,
                        other => Some(num(other).unwrap_or(0.0)),
                    })
                    .collect();
                arrays.push(Arc::new(Float64Array::from(data)));
            }
            2 => {
                fields.push(Field::new(name, DataType::Int64, true));
                let data: Vec<Option<i64>> = cols[ci]
                    .iter()
                    .map(|c| match c {
                        Value::Null => None,
                        other => Some(num(other).unwrap_or(0.0) as i64),
                    })
                    .collect();
                arrays.push(Arc::new(Int64Array::from(data)));
            }
            _ => {
                fields.push(Field::new(name, DataType::Utf8, true));
                let data: Vec<Option<String>> = cols[ci]
                    .iter()
                    .map(|c| match c {
                        Value::Null => None,
                        Value::String(s) => Some(s.clone()),
                        other => Some(crate::value::format_value(other)),
                    })
                    .collect();
                arrays.push(Arc::new(StringArray::from(data)));
            }
        }
    }
    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| format!("parquet_save arrow batch: {e}"))?;
    let file = File::create(path).map_err(|e| format!("parquet_save({path}): {e}"))?;
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .map_err(|e| format!("parquet_save writer: {e}"))?;
    writer
        .write(&batch)
        .map_err(|e| format!("parquet_save write: {e}"))?;
    writer
        .close()
        .map_err(|e| format!("parquet_save close: {e}"))?;
    Ok(nrows as i64)
}

pub fn apache_parquet_load(path: &str) -> Result<Vec<Value>, String> {
    let file = File::open(path).map_err(|e| format!("parquet_load({path}): {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("parquet_load open: {e}"))?;
    let mut reader = builder
        .build()
        .map_err(|e| format!("parquet_load reader: {e}"))?;
    let mut out_rows: Vec<HashMap<String, Value>> = Vec::new();
    while let Some(batch) = reader.next() {
        let batch = batch.map_err(|e| format!("parquet_load batch: {e}"))?;
        let n = batch.num_rows();
        let start = out_rows.len();
        out_rows.resize_with(start + n, HashMap::new);
        for (ci, field) in batch.schema().fields().iter().enumerate() {
            let name = field.name().clone();
            let col = batch.column(ci);
            for r in 0..n {
                let cell = if col.is_null(r) {
                    Value::Null
                } else if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                    float_out(arr.value(r))
                } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                    int_out(arr.value(r))
                } else if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                    int_out(arr.value(r) as i64)
                } else if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    Value::String(arr.value(r).to_string())
                } else if let Some(arr) = col.as_any().downcast_ref::<BooleanArray>() {
                    Value::Bool(arr.value(r))
                } else {
                    Value::String(format!("{col:?}"))
                };
                out_rows[start + r].insert(name.clone(), cell);
            }
        }
    }
    Ok(out_rows.into_iter().map(Value::Object).collect())
}
