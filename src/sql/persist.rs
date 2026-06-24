//! Kabootar database persistence — JSON snapshot to disk.

use crate::sql::{SqlEngine, TableDef};
use crate::sql::schema::{ColumnDef, IndexDef, SqlType};
use crate::value::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const FORMAT_VERSION: u32 = 1;

pub fn save_engine(engine: &SqlEngine, path: &str) -> Result<(), String> {
    let json = encode_engine(engine)?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
        }
    }
    fs::write(path, json).map_err(|e| format!("Failed to write database file: {e}"))
}

pub fn load_engine(path: &str) -> Result<SqlEngine, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Failed to read database file: {e}"))?;
    decode_engine(&text)
}

fn encode_engine(engine: &SqlEngine) -> Result<String, String> {
    let mut out = String::from("{\"version\":1,\"tables\":{");
    let mut first_table = true;
    for (name, table) in &engine.tables {
        if !first_table {
            out.push(',');
        }
        first_table = false;
        encode_table(name, table, &mut out)?;
    }
    out.push_str("}}");
    Ok(out)
}

fn encode_table(name: &str, table: &TableDef, out: &mut String) -> Result<(), String> {
    out.push('"');
    escape_string(name, out);
    out.push_str("\":{\"columns\":[");
    for (i, col) in table.columns.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        encode_column(col, out)?;
    }
    out.push_str("],\"rows\":[");
    for (i, row) in table.rows_for_persist().iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        encode_row(row, out)?;
    }
    out.push_str("],\"indexes\":[");
    for (i, idx) in table.indexes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        encode_index(idx, out);
    }
    out.push_str("],\"serial_counters\":{");
    let mut first_serial = true;
    for (col, counter) in &table.serial_counters {
        if !first_serial {
            out.push(',');
        }
        first_serial = false;
        out.push('"');
        escape_string(col, out);
        out.push('"');
        out.push(':');
        out.push_str(&counter.to_string());
    }
    out.push_str("},\"primary_key\":");
    match &table.primary_key {
        Some(pk) => {
            out.push('"');
            escape_string(pk, out);
            out.push('"');
        }
        None => out.push_str("null"),
    }
    out.push('}');
    Ok(())
}

fn encode_column(col: &ColumnDef, out: &mut String) -> Result<(), String> {
    out.push_str("{\"name\":\"");
    escape_string(&col.name, out);
    out.push_str("\",\"sql_type\":\"");
    out.push_str(sql_type_name(&col.sql_type));
    out.push_str("\",\"not_null\":");
    out.push_str(if col.not_null { "true" } else { "false" });
    out.push_str(",\"unique\":");
    out.push_str(if col.unique { "true" } else { "false" });
    out.push_str(",\"serial\":");
    out.push_str(if col.serial { "true" } else { "false" });
    out.push('}');
    Ok(())
}

fn encode_index(idx: &IndexDef, out: &mut String) {
    out.push_str("{\"name\":\"");
    escape_string(&idx.name, out);
    out.push_str("\",\"columns\":[");
    for (i, col) in idx.columns.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        escape_string(col, out);
        out.push('"');
    }
    out.push_str("],\"unique\":");
    out.push_str(if idx.unique { "true" } else { "false" });
    out.push('}');
}

fn encode_row(row: &HashMap<String, Value>, out: &mut String) -> Result<(), String> {
    out.push('{');
    let mut first = true;
    for (key, val) in row {
        if !first {
            out.push(',');
        }
        first = false;
        out.push('"');
        escape_string(key, out);
        out.push_str("\":");
        encode_value(val, out)?;
    }
    out.push('}');
    Ok(())
}

fn encode_value(val: &Value, out: &mut String) -> Result<(), String> {
    match val {
        Value::Null => out.push_str("null"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::Float(f) => out.push_str(&f.to_string()),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::String(s) => {
            out.push('"');
            escape_string(s, out);
            out.push('"');
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
        Value::Object(map) => {
            out.push('{');
            let mut first = true;
            for (k, v) in map {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                escape_string(k, out);
                out.push_str("\":");
                encode_value(v, out)?;
            }
            out.push('}');
        }
        _ => out.push_str("null"),
    }
    Ok(())
}

fn escape_string(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
}

fn sql_type_name(t: &SqlType) -> &'static str {
    match t {
        SqlType::Integer => "integer",
        SqlType::Text => "text",
        SqlType::Float => "float",
        SqlType::Bool => "bool",
        SqlType::Json => "json",
    }
}

fn decode_engine(text: &str) -> Result<SqlEngine, String> {
    let mut parser = JsonParser::new(text);
    parser.expect('{')?;
    parser.expect_key("version")?;
    let version = parser.read_number_u32()?;
    if version != FORMAT_VERSION {
        return Err(format!("Unsupported database format version: {version}"));
    }
    parser.expect(',')?;
    parser.expect_key("tables")?;
    parser.expect('{')?;
    let mut tables = HashMap::new();
    if parser.peek() != Some('}') {
        loop {
            let table_name = parser.read_string()?;
            parser.expect(':')?;
            parser.expect('{')?;
            let table = decode_table(&mut parser)?;
            tables.insert(table_name, table);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some('}') => break,
                other => return Err(format!("Expected ',' or '}}' in tables, found {other:?}")),
            }
        }
    }
    parser.expect('}')?;
    parser.expect('}')?;
    let mut engine = SqlEngine::new();
    for (name, mut table) in tables {
        table.rebuild_all_indexes();
        engine.tables.insert(name, table);
    }
    Ok(engine)
}

fn decode_table(parser: &mut JsonParser<'_>) -> Result<TableDef, String> {
    parser.expect_key("columns")?;
    parser.expect('[')?;
    let mut columns = Vec::new();
    if parser.peek() != Some(']') {
        loop {
            columns.push(decode_column(parser)?);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some(']') => break,
                other => return Err(format!("Expected ',' or ']' in columns, found {other:?}")),
            }
        }
    }
    parser.expect(']')?;
    parser.expect(',')?;
    parser.expect_key("rows")?;
    parser.expect('[')?;
    let mut rows = Vec::new();
    if parser.peek() != Some(']') {
        loop {
            rows.push(decode_row(parser)?);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some(']') => break,
                other => return Err(format!("Expected ',' or ']' in rows, found {other:?}")),
            }
        }
    }
    parser.expect(']')?;
    parser.expect(',')?;
    parser.expect_key("indexes")?;
    parser.expect('[')?;
    let mut indexes = Vec::new();
    if parser.peek() != Some(']') {
        loop {
            indexes.push(decode_index(parser)?);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some(']') => break,
                other => return Err(format!("Expected ',' or ']' in indexes, found {other:?}")),
            }
        }
    }
    parser.expect(']')?;
    parser.expect(',')?;
    parser.expect_key("serial_counters")?;
    parser.expect('{')?;
    let mut serial_counters = HashMap::new();
    if parser.peek() != Some('}') {
        loop {
            let col = parser.read_string()?;
            parser.expect(':')?;
            let counter = parser.read_number_i64()?;
            serial_counters.insert(col, counter);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some('}') => break,
                other => {
                    return Err(format!(
                        "Expected ',' or '}}' in serial_counters, found {other:?}"
                    ));
                }
            }
        }
    }
    parser.expect('}')?;
    parser.expect(',')?;
    parser.expect_key("primary_key")?;
    let primary_key = if parser.peek() == Some('n') {
        parser.read_null()?;
        None
    } else {
        Some(parser.read_string()?)
    };
    parser.expect('}')?;
    let mut table = TableDef::from_rows(columns, primary_key, rows);
    table.indexes = indexes;
    table.serial_counters = serial_counters;
    table.ensure_auto_indexes();
    table.rebuild_all_indexes();
    Ok(table)
}

fn decode_column(parser: &mut JsonParser<'_>) -> Result<ColumnDef, String> {
    parser.expect('{')?;
    parser.expect_key("name")?;
    let name = parser.read_string()?;
    parser.expect(',')?;
    parser.expect_key("sql_type")?;
    let sql_type = parse_sql_type_name(&parser.read_string()?);
    parser.expect(',')?;
    parser.expect_key("not_null")?;
    let not_null = parser.read_bool()?;
    parser.expect(',')?;
    parser.expect_key("unique")?;
    let unique = parser.read_bool()?;
    parser.expect(',')?;
    parser.expect_key("serial")?;
    let serial = parser.read_bool()?;
    parser.expect('}')?;
    Ok(ColumnDef {
        name,
        sql_type,
        not_null,
        unique,
        serial,
    })
}

fn decode_index(parser: &mut JsonParser<'_>) -> Result<IndexDef, String> {
    parser.expect('{')?;
    parser.expect_key("name")?;
    let name = parser.read_string()?;
    parser.expect(',')?;
    parser.expect_key("columns")?;
    parser.expect('[')?;
    let mut columns = Vec::new();
    if parser.peek() != Some(']') {
        loop {
            columns.push(parser.read_string()?);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some(']') => break,
                other => return Err(format!("Expected ',' or ']' in index columns, found {other:?}")),
            }
        }
    }
    parser.expect(']')?;
    parser.expect(',')?;
    parser.expect_key("unique")?;
    let unique = parser.read_bool()?;
    parser.expect('}')?;
    Ok(IndexDef {
        name,
        columns,
        unique,
    })
}

fn decode_row(parser: &mut JsonParser<'_>) -> Result<HashMap<String, Value>, String> {
    parser.expect('{')?;
    let mut row = HashMap::new();
    if parser.peek() != Some('}') {
        loop {
            let key = parser.read_string()?;
            parser.expect(':')?;
            row.insert(key, parser.read_value()?);
            match parser.peek() {
                Some(',') => {
                    parser.next()?;
                }
                Some('}') => break,
                other => return Err(format!("Expected ',' or '}}' in row, found {other:?}")),
            }
        }
    }
    parser.expect('}')?;
    Ok(row)
}

fn parse_sql_type_name(name: &str) -> SqlType {
    match name {
        "integer" => SqlType::Integer,
        "float" => SqlType::Float,
        "bool" => SqlType::Bool,
        "json" => SqlType::Json,
        _ => SqlType::Text,
    }
}

struct JsonParser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn next(&mut self) -> Result<char, String> {
        let ch = self.peek().ok_or("Unexpected end of JSON")?;
        self.pos += ch.len_utf8();
        Ok(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), String> {
        self.skip_ws();
        let got = self.next()?;
        if got != ch {
            return Err(format!("Expected '{ch}', found '{got}'"));
        }
        Ok(())
    }

    fn expect_key(&mut self, key: &str) -> Result<(), String> {
        self.skip_ws();
        let read = self.read_string()?;
        if read != key {
            return Err(format!("Expected key '{key}', found '{read}'"));
        }
        self.skip_ws();
        self.expect(':')
    }

    fn read_null(&mut self) -> Result<(), String> {
        self.skip_ws();
        if !self.text[self.pos..].starts_with("null") {
            return Err("Expected null".into());
        }
        self.pos += 4;
        Ok(())
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        self.skip_ws();
        if self.text[self.pos..].starts_with("true") {
            self.pos += 4;
            return Ok(true);
        }
        if self.text[self.pos..].starts_with("false") {
            self.pos += 5;
            return Ok(false);
        }
        Err("Expected boolean".into())
    }

    fn read_number_u32(&mut self) -> Result<u32, String> {
        self.skip_ws();
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        self.text[start..self.pos]
            .parse()
            .map_err(|_| "Invalid number".to_string())
    }

    fn read_number_i64(&mut self) -> Result<i64, String> {
        self.skip_ws();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        self.text[start..self.pos]
            .parse()
            .map_err(|_| "Invalid number".to_string())
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let ch = self.next()?;
            match ch {
                '"' => break,
                '\\' => {
                    let esc = self.next()?;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        Ok(out)
    }

    fn read_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('"') => Ok(Value::String(self.read_string()?)),
            Some('[') => {
                self.next()?;
                let mut items = Vec::new();
                if self.peek() != Some(']') {
                    loop {
                        items.push(self.read_value()?);
                        self.skip_ws();
                        match self.peek() {
                            Some(',') => {
                                self.next();
                            }
                            Some(']') => break,
                            other => return Err(format!("Expected ',' or ']', found {other:?}")),
                        }
                    }
                }
                self.expect(']')?;
                Ok(Value::Array(items))
            }
            Some('{') => {
                self.next()?;
                let mut map = HashMap::new();
                if self.peek() != Some('}') {
                    loop {
                        let key = self.read_string()?;
                        self.expect(':')?;
                        map.insert(key, self.read_value()?);
                        self.skip_ws();
                        match self.peek() {
                            Some(',') => {
                                self.next();
                            }
                            Some('}') => break,
                            other => return Err(format!("Expected ',' or '}}', found {other:?}")),
                        }
                    }
                }
                self.expect('}')?;
                Ok(Value::Object(map))
            }
            Some('n') => {
                self.read_null()?;
                Ok(Value::Null)
            }
            Some('t') | Some('f') => Ok(Value::Bool(self.read_bool()?)),
            Some('-') | Some('0'..='9') => self.read_numeric_value(),
            other => Err(format!("Unexpected JSON value start: {other:?}")),
        }
    }

    fn read_numeric_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') {
            self.pos += 1;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.pos += ch.len_utf8();
                } else {
                    break;
                }
            }
            let n: f64 = self.text[start..self.pos]
                .parse()
                .map_err(|_| "Invalid float".to_string())?;
            return Ok(Value::Float(n));
        }
        let n: i64 = self.text[start..self.pos]
            .parse()
            .map_err(|_| "Invalid integer".to_string())?;
        Ok(Value::Number(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::schema::parse_column_defs;

    #[test]
    fn roundtrip_save_load() {
        let mut engine = SqlEngine::new();
        let (columns, pk, _, _) = parse_column_defs("id SERIAL PRIMARY KEY, name TEXT").unwrap();
        let mut users = TableDef::from_rows(columns, pk, vec![HashMap::from([
            ("id".into(), Value::Number(1)),
            ("name".into(), Value::String("Ada".into())),
        ])]);
        users.serial_counters = HashMap::from([("id".into(), 2)]);
        engine.tables.insert("users".into(), users);
        let path = std::env::temp_dir().join("kabootar_persist_test.kdb");
        let path_str = path.to_string_lossy().to_string();
        let _ = fs::remove_file(&path);
        save_engine(&engine, &path_str).unwrap();
        let loaded = load_engine(&path_str).unwrap();
        let users = loaded.tables.get("users").unwrap();
        assert_eq!(users.live_row_count(), 1);
        assert!(matches!(
            users.row_map(0).and_then(|r| r.get("name").cloned()),
            Some(Value::String(s)) if s == "Ada"
        ));
        let _ = fs::remove_file(&path);
    }
}
