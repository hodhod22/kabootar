//! Binary .kdb v2 format — page-oriented persistence (Phase 2).

use crate::sql::{SqlEngine, TableDef};
use crate::sql::schema::{ColumnDef, IndexDef};
use crate::sql::storage::buffer::BufferPool;
use crate::sql::storage::pages::page_checksum;
use crate::value::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub const KDB2_MAGIC: &[u8; 4] = b"KDB2";
pub const FORMAT_V2: u32 = 2;

pub fn is_binary_kdb(path: &str) -> bool {
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).is_ok() && magic == *KDB2_MAGIC
}

pub fn save_engine_v2(engine: &SqlEngine, path: &str) -> Result<(), String> {
    let bytes = encode_engine_v2(engine)?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
        }
    }
    let tmp = format!("{path}.tmp");
    fs::write(&tmp, &bytes).map_err(|e| format!("write failed: {e}"))?;
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }
    fs::rename(&tmp, path).map_err(|e| format!("rename failed: {e}"))
}

pub fn load_engine_v2(path: &str) -> Result<SqlEngine, String> {
    let bytes = fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    decode_engine_v2(&bytes)
}

pub fn flush_dirty_pages(pool: &mut BufferPool, path: &str) -> Result<(), String> {
    let dirty: Vec<u64> = pool.dirty_pages();
    if dirty.is_empty() {
        return Ok(());
    }
    let mut append = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{path}.deltas"))
        .map_err(|e| format!("delta open: {e}"))?;
    for page_id in dirty {
        if let Some(data) = pool.page_bytes(page_id) {
            let checksum = page_checksum(&data);
            append
                .write_all(&page_id.to_le_bytes())
                .map_err(|e| format!("delta write: {e}"))?;
            append
                .write_all(&checksum.to_le_bytes())
                .map_err(|e| format!("delta write: {e}"))?;
            append
                .write_all(&(data.len() as u32).to_le_bytes())
                .map_err(|e| format!("delta write: {e}"))?;
            append
                .write_all(&data)
                .map_err(|e| format!("delta write: {e}"))?;
            pool.mark_clean(page_id);
        }
    }
    Ok(())
}

pub fn incremental_checkpoint(
    engine: &SqlEngine,
    pool: &mut BufferPool,
    path: &str,
) -> Result<(), String> {
    save_engine_v2(engine, path)?;
    flush_dirty_pages(pool, path)
}

fn encode_engine_v2(engine: &SqlEngine) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(KDB2_MAGIC);
    out.extend_from_slice(&FORMAT_V2.to_le_bytes());
    let table_names: Vec<&String> = engine.tables.keys().collect();
    out.extend_from_slice(&(table_names.len() as u32).to_le_bytes());
    for name in table_names {
        let table = engine.tables.get(name).unwrap();
        encode_table_v2(name, table, &mut out)?;
    }
    let checksum = page_checksum(&out);
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

fn encode_table_v2(name: &str, table: &TableDef, out: &mut Vec<u8>) -> Result<(), String> {
    write_str(out, name)?;
    write_u32(out, table.columns.len() as u32)?;
    for col in &table.columns {
        write_str(out, &col.name)?;
        write_u8(out, sql_type_tag(&col.sql_type))?;
        write_u8(out, col.not_null as u8)?;
        write_u8(out, col.unique as u8)?;
        write_u8(out, col.serial as u8)?;
    }
    write_opt_str(out, table.primary_key.as_deref())?;
    let slots: Vec<usize> = table.live_slots();
    write_u32(out, slots.len() as u32)?;
    for slot in slots {
        if let Some(row) = table.row_map(slot) {
            write_row(out, &row, &table.columns)?;
        }
    }
    write_u32(out, table.indexes.len() as u32)?;
    for idx in &table.indexes {
        write_str(out, &idx.name)?;
        write_u32(out, idx.columns.len() as u32)?;
        for c in &idx.columns {
            write_str(out, c)?;
        }
        write_u8(out, idx.unique as u8)?;
    }
    Ok(())
}

fn decode_engine_v2(bytes: &[u8]) -> Result<SqlEngine, String> {
    if bytes.len() < 12 {
        return Err("Truncated KDB2".into());
    }
    if bytes[0..4] != *KDB2_MAGIC {
        return Err("Not KDB2".into());
    }
    let _version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let mut pos = 8usize;
    let table_count = read_u32(bytes, &mut pos)? as usize;
    let mut tables = HashMap::new();
    for _ in 0..table_count {
        let (name, table, consumed) = decode_table_v2(&bytes[pos..])?;
        pos += consumed;
        tables.insert(name, table);
    }
    Ok(SqlEngine::from_tables(tables))
}

fn decode_table_v2(bytes: &[u8]) -> Result<(String, TableDef, usize), String> {
    let mut pos = 0usize;
    let name = read_str(bytes, &mut pos)?;
    let col_count = read_u32(bytes, &mut pos)? as usize;
    let mut columns = Vec::new();
    for _ in 0..col_count {
        let cname = read_str(bytes, &mut pos)?;
        let tag = read_u8(bytes, &mut pos)?;
        let not_null = read_u8(bytes, &mut pos)? != 0;
        let unique = read_u8(bytes, &mut pos)? != 0;
        let serial = read_u8(bytes, &mut pos)? != 0;
        columns.push(ColumnDef {
            name: cname,
            sql_type: tag_to_sql_type(tag),
            not_null,
            unique,
            serial,
        });
    }
    let pk = read_opt_str(bytes, &mut pos)?;
    let row_count = read_u32(bytes, &mut pos)? as usize;
    let mut rows = Vec::new();
    for _ in 0..row_count {
        rows.push(read_row(bytes, &mut pos, &columns)?);
    }
    let idx_count = read_u32(bytes, &mut pos)? as usize;
    let mut indexes = Vec::new();
    for _ in 0..idx_count {
        let iname = read_str(bytes, &mut pos)?;
        let ncol = read_u32(bytes, &mut pos)? as usize;
        let mut cols = Vec::new();
        for _ in 0..ncol {
            cols.push(read_str(bytes, &mut pos)?);
        }
        let unique = read_u8(bytes, &mut pos)? != 0;
        indexes.push(IndexDef {
            name: iname,
            columns: cols,
            unique,
        });
    }
    let mut table = TableDef::from_rows(columns, pk, rows);
    table.indexes = indexes;
    table.foreign_keys = Vec::new();
    table.checks = Vec::new();
    table.ensure_auto_indexes();
    table.rebuild_all_indexes();
    Ok((name, table, pos))
}

fn write_row(out: &mut Vec<u8>, row: &HashMap<String, Value>, cols: &[ColumnDef]) -> Result<(), String> {
    for col in cols {
        write_value(out, row.get(&col.name).unwrap_or(&Value::Null))?;
    }
    Ok(())
}

fn read_row(
    bytes: &[u8],
    pos: &mut usize,
    cols: &[ColumnDef],
) -> Result<HashMap<String, Value>, String> {
    let mut row = HashMap::new();
    for col in cols {
        row.insert(col.name.clone(), read_value(bytes, pos)?);
    }
    Ok(row)
}

fn write_value(out: &mut Vec<u8>, v: &Value) -> Result<(), String> {
    match v {
        Value::Null => write_u8(out, 0),
        Value::Number(n) => {
            write_u8(out, 1)?;
            out.extend_from_slice(&n.to_le_bytes());
            Ok(())
        }
        Value::Float(f) => {
            write_u8(out, 2)?;
            out.extend_from_slice(&f.to_le_bytes());
            Ok(())
        }
        Value::Bool(b) => {
            write_u8(out, 3)?;
            write_u8(out, *b as u8)
        }
        Value::String(s) => {
            write_u8(out, 4)?;
            write_str(out, s)
        }
        Value::Object(m) => {
            write_u8(out, 5)?;
            write_u32(out, m.len() as u32)?;
            for (k, val) in m {
                write_str(out, k)?;
                write_value(out, val)?;
            }
            Ok(())
        }
        Value::Array(a) => {
            write_u8(out, 6)?;
            write_u32(out, a.len() as u32)?;
            for item in a {
                write_value(out, item)?;
            }
            Ok(())
        }
        _ => write_u8(out, 0),
    }
}

fn read_value(bytes: &[u8], pos: &mut usize) -> Result<Value, String> {
    let tag = read_u8(bytes, pos)?;
    Ok(match tag {
        0 => Value::Null,
        1 => Value::Number(i64::from_le_bytes(read_bytes::<8>(bytes, pos)?)),
        2 => Value::Float(f64::from_le_bytes(read_bytes::<8>(bytes, pos)?)),
        3 => Value::Bool(read_u8(bytes, pos)? != 0),
        4 => Value::String(read_str(bytes, pos)?),
        5 => {
            let n = read_u32(bytes, pos)? as usize;
            let mut m = HashMap::new();
            for _ in 0..n {
                let k = read_str(bytes, pos)?;
                m.insert(k, read_value(bytes, pos)?);
            }
            Value::Object(m)
        }
        6 => {
            let n = read_u32(bytes, pos)? as usize;
            let mut a = Vec::new();
            for _ in 0..n {
                a.push(read_value(bytes, pos)?);
            }
            Value::Array(a)
        }
        _ => Value::Null,
    })
}

fn sql_type_tag(t: &crate::sql::schema::SqlType) -> u8 {
    use crate::sql::schema::SqlType;
    match t {
        SqlType::Integer => 1,
        SqlType::Text => 2,
        SqlType::Float => 3,
        SqlType::Bool => 4,
        SqlType::Json => 5,
    }
}

fn tag_to_sql_type(tag: u8) -> crate::sql::schema::SqlType {
    use crate::sql::schema::SqlType;
    match tag {
        1 => SqlType::Integer,
        3 => SqlType::Float,
        4 => SqlType::Bool,
        5 => SqlType::Json,
        _ => SqlType::Text,
    }
}

fn write_u8(out: &mut Vec<u8>, v: u8) -> Result<(), String> {
    out.push(v);
    Ok(())
}

fn write_u32(out: &mut Vec<u8>, v: u32) -> Result<(), String> {
    out.extend_from_slice(&v.to_le_bytes());
    Ok(())
}

fn write_str(out: &mut Vec<u8>, s: &str) -> Result<(), String> {
    let b = s.as_bytes();
    write_u32(out, b.len() as u32)?;
    out.extend_from_slice(b);
    Ok(())
}

fn write_opt_str(out: &mut Vec<u8>, s: Option<&str>) -> Result<(), String> {
    match s {
        Some(v) => {
            write_u8(out, 1)?;
            write_str(out, v)
        }
        None => write_u8(out, 0),
    }
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, String> {
    if *pos >= bytes.len() {
        return Err("EOF".into());
    }
    let v = bytes[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let b: [u8; 4] = read_bytes(bytes, pos)?;
    Ok(u32::from_le_bytes(b))
}

fn read_bytes<const N: usize>(bytes: &[u8], pos: &mut usize) -> Result<[u8; N], String> {
    if *pos + N > bytes.len() {
        return Err("EOF".into());
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes[*pos..*pos + N]);
    *pos += N;
    Ok(arr)
}

fn read_str(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let len = read_u32(bytes, pos)? as usize;
    if *pos + len > bytes.len() {
        return Err("EOF".into());
    }
    let s = String::from_utf8(bytes[*pos..*pos + len].to_vec()).map_err(|e| e.to_string())?;
    *pos += len;
    Ok(s)
}

fn read_opt_str(bytes: &[u8], pos: &mut usize) -> Result<Option<String>, String> {
    if read_u8(bytes, pos)? == 0 {
        Ok(None)
    } else {
        Ok(Some(read_str(bytes, pos)?))
    }
}
