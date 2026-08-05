//! CSV / table / ASCII plot (SC3a/b + DX pretty).

use super::helpers::{float_out, int_out, num, vector_at};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn csv_parse(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("csv_parse(text) expects string".into()),
    };
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<Value> = line
            .split(',')
            .map(|c| {
                let c = c.trim();
                if let Ok(n) = c.parse::<i64>() {
                    Value::Number(n)
                } else if let Ok(f) = c.parse::<f64>() {
                    Value::Float(f)
                } else {
                    Value::String(c.to_string())
                }
            })
            .collect();
        rows.push(Value::Array(cells));
    }
    Ok(Value::Array(rows))
}

fn csv_load(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("csv_load(path) expects string".into()),
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("csv_load({path}): {e}"))?;
    csv_parse(&[Value::String(text)], _env)
}

fn table_describe(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let rows = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("table_describe(rows)".into()),
    };
    if rows.is_empty() {
        return Err("table_describe: empty".into());
    }
    let ncols = match &rows[0] {
        Value::Array(c) => c.len(),
        _ => return Err("table_describe: expected array of rows".into()),
    };
    let mut means = vec![0.0; ncols];
    let mut counts = vec![0usize; ncols];
    for row in rows {
        let Value::Array(cells) = row else {
            continue;
        };
        for (i, c) in cells.iter().enumerate().take(ncols) {
            if let Ok(v) = num(c) {
                means[i] += v;
                counts[i] += 1;
            }
        }
    }
    for i in 0..ncols {
        if counts[i] > 0 {
            means[i] /= counts[i] as f64;
        }
    }
    let mut out = HashMap::new();
    out.insert("rows".into(), int_out(rows.len() as i64));
    out.insert("cols".into(), int_out(ncols as i64));
    out.insert(
        "mean".into(),
        Value::Array(means.into_iter().map(float_out).collect()),
    );
    Ok(Value::Object(out))
}

fn format_table(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let rows = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("format_table(rows)".into()),
    };
    let mut lines = Vec::new();
    for row in rows {
        match row {
            Value::Array(cells) => {
                let parts: Vec<String> = cells
                    .iter()
                    .map(|c| crate::value::format_value(c))
                    .collect();
                lines.push(parts.join("\t"));
            }
            other => lines.push(crate::value::format_value(other)),
        }
    }
    Ok(Value::String(lines.join("\n")))
}

fn ascii_plot(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ys = vector_at(args, 0, "ascii_plot")?;
    if ys.is_empty() {
        return Err("ascii_plot: empty".into());
    }
    let width = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(40.0)
        .clamp(8.0, 120.0) as usize;
    let height = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(10.0)
        .clamp(4.0, 40.0) as usize;
    let min = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).abs().max(1e-12);
    let mut grid = vec![vec![' '; width]; height];
    for (i, y) in ys.iter().enumerate() {
        let x = if ys.len() == 1 {
            0
        } else {
            i * (width - 1) / (ys.len() - 1)
        };
        let row = ((max - y) / span * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        grid[row][x] = '*';
    }
    let mut out = String::new();
    for r in 0..height {
        out.push_str(&grid[r].iter().collect::<String>());
        out.push('\n');
    }
    out.push_str(&format!("min={min:.4} max={max:.4} n={}", ys.len()));
    Ok(Value::String(out))
}

/// Pretty-print nd / table / array for REPL.
fn pretty(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("pretty(value)")?;
    match v {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let data = m.get("data").cloned().unwrap_or(Value::Array(vec![]));
            let shape = m.get("shape").cloned().unwrap_or(Value::Array(vec![]));
            Ok(Value::String(format!(
                "ndarray shape={} data={}",
                crate::value::format_value(&shape),
                crate::value::format_value(&data)
            )))
        }
        Value::Array(items)
            if items
                .first()
                .map(|x| matches!(x, Value::Array(_)))
                .unwrap_or(false) =>
        {
            format_table(args, env)
        }
        other => Ok(Value::String(crate::value::format_value(other))),
    }
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_csv_parse", "csv_parse"], csv_parse);
    bind(&["science_csv_load", "csv_load"], csv_load);
    bind(&["science_table_describe", "table_describe"], table_describe);
    bind(&["science_format_table", "format_table"], format_table);
    bind(&["science_ascii_plot", "ascii_plot"], ascii_plot);
    bind(&["science_pretty", "pretty"], pretty);
}
