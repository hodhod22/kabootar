//! `.knb` notebook format — exploration cells sharing a Session (Våg DX2).

use crate::session::Session;
use crate::value::{format_value, Value};
use serde_json::{json, Value as JsonValue};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Notebook {
    pub version: u32,
    pub cells: Vec<NotebookCell>,
}

#[derive(Debug, Clone)]
pub struct NotebookCell {
    pub id: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct CellResult {
    pub id: String,
    pub ok: bool,
    pub output: String,
}

pub fn parse_notebook(text: &str) -> Result<Notebook, String> {
    let v: JsonValue =
        serde_json::from_str(text).map_err(|e| format!("invalid .knb JSON: {e}"))?;
    let version = v
        .get("version")
        .and_then(|x| x.as_u64())
        .unwrap_or(1) as u32;
    let cells_json = v
        .get("cells")
        .and_then(|c| c.as_array())
        .ok_or_else(|| ".knb missing cells array".to_string())?;
    let mut cells = Vec::new();
    for (i, c) in cells_json.iter().enumerate() {
        let id = c
            .get("id")
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("cell{i}"));
        let source = match c.get("source") {
            Some(JsonValue::String(s)) => s.clone(),
            Some(JsonValue::Array(parts)) => parts
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        };
        cells.push(NotebookCell { id, source });
    }
    Ok(Notebook { version, cells })
}

pub fn notebook_to_json(nb: &Notebook) -> String {
    let cells: Vec<JsonValue> = nb
        .cells
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "source": c.source,
            })
        })
        .collect();
    serde_json::to_string_pretty(&json!({
        "version": nb.version,
        "cells": cells,
    }))
    .unwrap_or_else(|_| "{}".into())
}

pub fn load_notebook(path: &Path) -> Result<Notebook, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_notebook(&text)
}

/// Run all cells in order on a fresh session (optional science preload).
pub fn run_notebook(nb: &Notebook, preload_science: bool) -> Result<(Session, Vec<CellResult>), String> {
    let mut session = Session::new();
    if preload_science {
        session.import_science()?;
    }
    let mut results = Vec::new();
    for cell in &nb.cells {
        if cell.source.trim().is_empty() {
            continue;
        }
        match session.eval_cell(&cell.source) {
            Ok(v) => results.push(CellResult {
                id: cell.id.clone(),
                ok: true,
                output: format_value(&v),
            }),
            Err(e) => {
                results.push(CellResult {
                    id: cell.id.clone(),
                    ok: false,
                    output: e.clone(),
                });
                return Err(format!("cell {}: {e}", cell.id));
            }
        }
    }
    Ok((session, results))
}

pub fn run_notebook_file(path: &Path, preload_science: bool) -> Result<Value, String> {
    let nb = load_notebook(path)?;
    let (_session, results) = run_notebook(&nb, preload_science)?;
    let last = results
        .last()
        .map(|r| Value::String(r.output.clone()))
        .unwrap_or(Value::Undefined);
    Ok(last)
}
