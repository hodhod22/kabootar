//! Write-ahead log for incremental Kabootar database persistence.

use crate::sql::{load_engine, save_engine, SqlEngine};
use crate::value::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub fn wal_path(base: &str) -> String {
    format!("{base}.wal")
}

pub fn wal_v2_path(base: &str) -> String {
    format!("{base}.wal2")
}

const WAL2_MAGIC: &[u8; 4] = b"WAL2";

pub fn load_with_wal(base_path: &str) -> Result<SqlEngine, String> {
    let mut engine = if Path::new(base_path).exists() {
        if crate::sql::is_binary_kdb(base_path) {
            crate::sql::load_engine_v2(base_path)?
        } else {
            load_engine(base_path)?
        }
    } else {
        SqlEngine::new()
    };
    let wal2 = wal_v2_path(base_path);
    if Path::new(&wal2).exists() {
        replay_wal_v2(&mut engine, &wal2)?;
    } else {
        let wal = wal_path(base_path);
        if Path::new(&wal).exists() {
            replay_wal(&mut engine, &wal)?;
        }
    }
    Ok(engine)
}

pub fn append_wal(base_path: &str, sql: &str, params: &[Value]) -> Result<(), String> {
    if crate::sql::is_binary_kdb(base_path) || base_path.ends_with(".kdb2") {
        return append_wal_v2(base_path, sql, params);
    }
    let wal = wal_path(base_path);
    if let Some(parent) = Path::new(&wal).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("WAL mkdir failed: {e}"))?;
        }
    }
    let line = encode_entry(sql, params)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal)
        .map_err(|e| format!("WAL open failed: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("WAL write failed: {e}"))
}

pub fn checkpoint(engine: &SqlEngine, base_path: &str) -> Result<(), String> {
    if engine.uses_binary_storage() || crate::sql::is_binary_kdb(base_path) {
        crate::sql::save_engine_v2(engine, base_path)?;
    } else {
        save_engine(engine, base_path)?;
    }
    let wal = wal_path(base_path);
    if Path::new(&wal).exists() {
        fs::remove_file(&wal).map_err(|e| format!("WAL truncate failed: {e}"))?;
    }
    let wal2 = wal_v2_path(base_path);
    if Path::new(&wal2).exists() {
        fs::remove_file(&wal2).map_err(|e| format!("WAL2 truncate failed: {e}"))?;
    }
    Ok(())
}

pub fn append_wal_v2(base_path: &str, sql: &str, params: &[Value]) -> Result<(), String> {
    let wal = wal_v2_path(base_path);
    if let Some(parent) = Path::new(&wal).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("WAL mkdir failed: {e}"))?;
        }
    }
    let params_json = encode_entry(sql, params)?;
    let params_start = params_json.find("\"params\":[")
        .ok_or("encode failed")?
        + "\"params\":[".len();
    let params_end = params_json.rfind(']').ok_or("encode failed")?;
    let params_blob = &params_json[params_start..params_end];
    let sql_bytes = sql.as_bytes();
    let params_bytes = params_blob.as_bytes();
    let mut record = Vec::new();
    record.extend_from_slice(WAL2_MAGIC);
    record.extend_from_slice(&(sql_bytes.len() as u32).to_le_bytes());
    record.extend_from_slice(sql_bytes);
    record.extend_from_slice(&(params_bytes.len() as u32).to_le_bytes());
    record.extend_from_slice(params_bytes);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal)
        .map_err(|e| format!("WAL2 open failed: {e}"))?;
    file.write_all(&record)
        .map_err(|e| format!("WAL2 write failed: {e}"))
}

fn replay_wal_v2(engine: &mut SqlEngine, wal_path: &str) -> Result<(), String> {
    let data = fs::read(wal_path).map_err(|e| format!("WAL2 read failed: {e}"))?;
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        if &data[pos..pos + 4] != WAL2_MAGIC {
            break;
        }
        pos += 4;
        let sql_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if pos + sql_len + 4 > data.len() {
            break;
        }
        let sql = std::str::from_utf8(&data[pos..pos + sql_len])
            .map_err(|e| format!("WAL2 utf8: {e}"))?
            .to_string();
        pos += sql_len;
        let params_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if pos + params_len > data.len() {
            break;
        }
        let params_blob = std::str::from_utf8(&data[pos..pos + params_len])
            .map_err(|e| format!("WAL2 params utf8: {e}"))?;
        pos += params_len;
        let params = if params_blob.trim().is_empty() {
            Vec::new()
        } else {
            decode_params(params_blob)?
        };
        engine.execute(&sql, &params)?;
    }
    Ok(())
}

fn replay_wal(engine: &mut SqlEngine, wal_path: &str) -> Result<(), String> {
    let file = fs::File::open(wal_path).map_err(|e| format!("WAL read failed: {e}"))?;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("WAL line read failed: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let (sql, params) = decode_entry(&line)?;
        engine.execute(&sql, &params)?;
    }
    Ok(())
}

fn encode_entry(sql: &str, params: &[Value]) -> Result<String, String> {
    let mut out = String::from("{\"sql\":\"");
    escape(sql, &mut out);
    out.push_str("\",\"params\":[");
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        encode_value(p, &mut out)?;
    }
    out.push_str("]}");
    Ok(out)
}

fn decode_entry(line: &str) -> Result<(String, Vec<Value>), String> {
    let sql_key = "\"sql\":\"";
    let sql_start = line
        .find(sql_key)
        .ok_or("Invalid WAL entry")?
        + sql_key.len();
    let sql_end = line[sql_start..]
        .find("\",\"params\"")
        .ok_or("Invalid WAL entry")?
        + sql_start;
    let sql = unescape(&line[sql_start..sql_end]);
    let params_start = line.find("\"params\":[")
        .ok_or("Invalid WAL entry")?
        + "\"params\":[".len();
    let params_end = line.rfind(']').ok_or("Invalid WAL entry")?;
    let params_json = &line[params_start..params_end];
    let params = if params_json.trim().is_empty() {
        Vec::new()
    } else {
        decode_params(params_json)?
    };
    Ok((sql, params))
}

fn decode_params(s: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        while i < s.len() && (s.as_bytes()[i] as char).is_whitespace() || s.as_bytes()[i] == b',' {
            i += 1;
        }
        if i >= s.len() {
            break;
        }
        let (val, consumed) = decode_value_at(&s[i..])?;
        out.push(val);
        i += consumed;
    }
    Ok(out)
}

fn decode_value_at(s: &str) -> Result<(Value, usize), String> {
    let t = s.trim_start();
    let offset = s.len() - t.len();
    if t.starts_with("null") {
        return Ok((Value::Null, offset + 4));
    }
    if t.starts_with("true") {
        return Ok((Value::Bool(true), offset + 4));
    }
    if t.starts_with("false") {
        return Ok((Value::Bool(false), offset + 5));
    }
    if let Some('"') = t.chars().next() {
        let mut end = 1usize;
        let chars: Vec<char> = t.chars().collect();
        while end < chars.len() {
            if chars[end] == '"' && chars[end - 1] != '\\' {
                break;
            }
            end += 1;
        }
        let inner: String = chars[1..end].iter().collect();
        return Ok((Value::String(unescape(&inner)), offset + end + 1));
    }
    let num_end = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.')
        .count();
    let num_str: String = t.chars().take(num_end).collect();
    if num_str.contains('.') {
        let f: f64 = num_str.parse().map_err(|_| "Invalid WAL float")?;
        Ok((Value::Float(f), offset + num_end))
    } else {
        let n: i64 = num_str.parse().map_err(|_| "Invalid WAL integer")?;
        Ok((Value::Number(n), offset + num_end))
    }
}

fn encode_value(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::Float(f) => out.push_str(&f.to_string()),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::String(s) => {
            out.push('"');
            escape(s, out);
            out.push('"');
        }
        Value::Object(map) => {
            out.push('{');
            let mut first = true;
            for (k, val) in map.iter() {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                escape(k, out);
                out.push_str("\":");
                encode_value(val, out)?;
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_value(item, out)?;
            }
            out.push(']');
        }
        _ => out.push_str("null"),
    }
    Ok(())
}

fn escape(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some(c) => out.push(c),
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}
