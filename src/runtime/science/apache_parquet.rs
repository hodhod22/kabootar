//! Apache Parquet read/write for flat and nested columns (DATA/SC).
//! Nested subset: List<Float64|Int64|Utf8> and Struct of flat fields.
//! Host only — wasm keeps KPQT1 via the caller.

#![cfg(not(target_arch = "wasm32"))]

use crate::runtime::science::helpers::{float_out, int_out, num};
use crate::value::Value;
use arrow_array::builder::{
    Float64Builder, Int64Builder, ListBuilder, StringBuilder, StructBuilder,
};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int32Array, Int64Array, ListArray, RecordBatch,
    StringArray, StructArray,
};
use arrow_schema::{DataType, Field, Fields, Schema};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Float,
    Int,
    Utf8,
    ListFloat,
    ListInt,
    ListUtf8,
    Struct,
}

fn promote(kind: ColKind, cell: &Value) -> ColKind {
    match (kind, cell) {
        (_, Value::Null) | (_, Value::Bool(_)) => kind,
        (ColKind::Float | ColKind::Int, Value::Array(_)) => ColKind::ListFloat,
        (ColKind::Utf8, Value::Array(_)) => ColKind::ListUtf8,
        (ColKind::Float | ColKind::Int, Value::Object(_)) => ColKind::Struct,
        (ColKind::Utf8, Value::Object(_)) => ColKind::Struct,
        (ColKind::ListFloat, Value::Array(items)) => {
            if items.iter().any(|v| matches!(v, Value::String(_))) {
                ColKind::ListUtf8
            } else if items
                .iter()
                .any(|v| matches!(v, Value::Number(_) | Value::BigInt(_)))
                && !items.iter().any(|v| matches!(v, Value::Float(_)))
            {
                ColKind::ListInt
            } else {
                ColKind::ListFloat
            }
        }
        (ColKind::Float, Value::String(_)) => ColKind::Utf8,
        (ColKind::Int, Value::String(_)) => ColKind::Utf8,
        (ColKind::Int, Value::Float(_)) => ColKind::Float,
        (ColKind::Float, Value::Number(_) | Value::BigInt(_)) => kind,
        (ColKind::Float, Value::Float(_)) => ColKind::Float,
        (_, Value::String(_)) => ColKind::Utf8,
        (_, Value::Object(_)) => ColKind::Struct,
        (_, Value::Array(_)) => ColKind::ListFloat,
        _ => kind,
    }
}

fn collect_columns(
    rows: &[Value],
) -> Result<(Vec<String>, Vec<ColKind>, Vec<Vec<Value>>), String> {
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
    let mut kinds = vec![ColKind::Float; ncols];
    let mut cols: Vec<Vec<Value>> = vec![Vec::with_capacity(nrows); ncols];
    for row in rows.iter() {
        let Value::Object(m) = row else {
            return Err("parquet_save: jagged rows".into());
        };
        for (ci, name) in col_names.iter().enumerate() {
            let cell = m.get(name).cloned().unwrap_or(Value::Null);
            if !matches!(cell, Value::Null) {
                kinds[ci] = promote(kinds[ci], &cell);
            }
            cols[ci].push(cell);
        }
    }
    Ok((col_names, kinds, cols))
}

fn struct_fields_from_rows(col: &[Value]) -> Result<Fields, String> {
    let mut names: Vec<String> = Vec::new();
    for cell in col {
        if let Value::Object(m) = cell {
            for k in m.keys() {
                if !names.iter().any(|n| n == k) {
                    names.push(k.clone());
                }
            }
        }
    }
    names.sort();
    if names.is_empty() {
        names.push("_empty".into());
    }
    let fields: Vec<Field> = names
        .iter()
        .map(|n| Field::new(n, DataType::Utf8, true))
        .collect();
    Ok(Fields::from(fields))
}

pub fn apache_parquet_save(path: &str, rows: &[Value]) -> Result<i64, String> {
    let (col_names, kinds, cols) = collect_columns(rows)?;
    let nrows = rows.len();
    let mut fields = Vec::with_capacity(col_names.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(col_names.len());
    for (ci, name) in col_names.iter().enumerate() {
        match kinds[ci] {
            ColKind::Float => {
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
            ColKind::Int => {
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
            ColKind::Utf8 => {
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
            ColKind::ListFloat | ColKind::ListInt => {
                let item_ty = if kinds[ci] == ColKind::ListInt {
                    DataType::Int64
                } else {
                    DataType::Float64
                };
                fields.push(Field::new(
                    name,
                    DataType::List(Arc::new(Field::new("item", item_ty.clone(), true))),
                    true,
                ));
                if kinds[ci] == ColKind::ListInt {
                    let mut builder = ListBuilder::new(Int64Builder::new());
                    for cell in &cols[ci] {
                        match cell {
                            Value::Null => builder.append(false), Value::Array(items) => {
                                for it in items.iter() {
                                    match it {
                                        Value::Null => builder.values().append_null(),
                                        other => builder
                                            .values()
                                            .append_value(num(other).unwrap_or(0.0) as i64),
                                    }
                                }
                                builder.append(true);
                            }
                            other => {
                                builder
                                    .values()
                                    .append_value(num(other).unwrap_or(0.0) as i64);
                                builder.append(true);
                            }
                        }
                    }
                    arrays.push(Arc::new(builder.finish()));
                } else {
                    let mut builder = ListBuilder::new(Float64Builder::new());
                    for cell in &cols[ci] {
                        match cell {
                            Value::Null => builder.append(false), Value::Array(items) => {
                                for it in items.iter() {
                                    match it {
                                        Value::Null => builder.values().append_null(),
                                        other => {
                                            builder.values().append_value(num(other).unwrap_or(0.0))
                                        }
                                    }
                                }
                                builder.append(true);
                            }
                            other => {
                                builder.values().append_value(num(other).unwrap_or(0.0));
                                builder.append(true);
                            }
                        }
                    }
                    arrays.push(Arc::new(builder.finish()));
                }
            }
            ColKind::ListUtf8 => {
                fields.push(Field::new(
                    name,
                    DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                    true,
                ));
                let mut builder = ListBuilder::new(StringBuilder::new());
                for cell in &cols[ci] {
                    match cell {
                        Value::Null => builder.append(false), Value::Array(items) => {
                            for it in items.iter() {
                                match it {
                                    Value::Null => builder.values().append_null(),
                                    Value::String(s) => builder.values().append_value(s),
                                    other => builder
                                        .values()
                                        .append_value(crate::value::format_value(other)),
                                }
                            }
                            builder.append(true);
                        }
                        Value::String(s) => {
                            builder.values().append_value(s);
                            builder.append(true);
                        }
                        other => {
                            builder
                                .values()
                                .append_value(crate::value::format_value(other));
                            builder.append(true);
                        }
                    }
                }
                arrays.push(Arc::new(builder.finish()));
            }
            ColKind::Struct => {
                let struct_fields = struct_fields_from_rows(&cols[ci])?;
                fields.push(Field::new(
                    name,
                    DataType::Struct(struct_fields.clone()),
                    true,
                ));
                let mut builder = StructBuilder::from_fields(struct_fields.clone(), nrows);
                for cell in &cols[ci] {
                    match cell {
                        Value::Null => {
                            for fi in 0..builder.num_fields() {
                                builder
                                    .field_builder::<StringBuilder>(fi)
                                    .unwrap()
                                    .append_null();
                            }
                            builder.append(false);
                        }
                        Value::Object(m) => {
                            for (fi, f) in struct_fields.iter().enumerate() {
                                let fb = builder.field_builder::<StringBuilder>(fi).unwrap();
                                match m.get(f.name()) {
                                    None | Some(Value::Null) => fb.append_null(),
                                    Some(Value::String(s)) => fb.append_value(s),
                                    Some(other) => fb.append_value(crate::value::format_value(other)),
                                }
                            }
                            builder.append(true);
                        }
                        other => {
                            // Non-object → stash under first field as string.
                            for fi in 0..builder.num_fields() {
                                let fb = builder.field_builder::<StringBuilder>(fi).unwrap();
                                if fi == 0 {
                                    fb.append_value(crate::value::format_value(other));
                                } else {
                                    fb.append_null();
                                }
                            }
                            builder.append(true);
                        }
                    }
                }
                arrays.push(Arc::new(builder.finish()));
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

fn list_cell(col: &dyn Array, r: usize) -> Value {
    let Some(list) = col.as_any().downcast_ref::<ListArray>() else {
        return Value::String(format!("{col:?}"));
    };
    if list.is_null(r) {
        return Value::Null;
    }
    let values = list.value(r);
    let mut out = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        if values.is_null(i) {
            out.push(Value::Null);
        } else if let Some(arr) = values.as_any().downcast_ref::<Float64Array>() {
            out.push(float_out(arr.value(i)));
        } else if let Some(arr) = values.as_any().downcast_ref::<Int64Array>() {
            out.push(int_out(arr.value(i)));
        } else if let Some(arr) = values.as_any().downcast_ref::<Int32Array>() {
            out.push(int_out(arr.value(i) as i64));
        } else if let Some(arr) = values.as_any().downcast_ref::<StringArray>() {
            out.push(Value::String(arr.value(i).to_string()));
        } else {
            out.push(Value::String(format!("{values:?}")));
        }
    }
    Value::from_array(out)
}

fn struct_cell(col: &dyn Array, r: usize) -> Value {
    let Some(st) = col.as_any().downcast_ref::<StructArray>() else {
        return Value::String(format!("{col:?}"));
    };
    if st.is_null(r) {
        return Value::Null;
    }
    let mut m = HashMap::new();
    for (i, field) in st.fields().iter().enumerate() {
        let child = st.column(i);
        let cell = if child.is_null(r) {
            Value::Null
        } else if let Some(arr) = child.as_any().downcast_ref::<Float64Array>() {
            float_out(arr.value(r))
        } else if let Some(arr) = child.as_any().downcast_ref::<Int64Array>() {
            int_out(arr.value(r))
        } else if let Some(arr) = child.as_any().downcast_ref::<Int32Array>() {
            int_out(arr.value(r) as i64)
        } else if let Some(arr) = child.as_any().downcast_ref::<StringArray>() {
            Value::String(arr.value(r).to_string())
        } else if let Some(arr) = child.as_any().downcast_ref::<BooleanArray>() {
            Value::Bool(arr.value(r))
        } else {
            Value::String(format!("{child:?}"))
        };
        m.insert(field.name().clone(), cell);
    }
    Value::from_object(m)
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
                } else if matches!(field.data_type(), DataType::List(_)) {
                    list_cell(col.as_ref(), r)
                } else if matches!(field.data_type(), DataType::Struct(_)) {
                    struct_cell(col.as_ref(), r)
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
    Ok(out_rows.into_iter().map(Value::from_object).collect())
}
