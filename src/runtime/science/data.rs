//! CSV / table / ASCII plot (SC3a/b + DX pretty).

use super::helpers::{float_out, int_out, num, vector_at};
use crate::runtime::render::canvas2d;
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

/// KPQT1 — Kab Parquet-lite columnar (SC7c). Array of row-objects → binary file.
/// Magic KPQT | ver u32 LE=1 | ncols | (name_len, name, dtype u8)* | nrows | column payloads.
/// dtype: 1=f64, 2=i64, 3=utf8.
fn parquet_save(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("parquet_save(path, rows)".into()),
    };
    let rows = match args.get(1) {
        Some(Value::Array(items)) => items,
        _ => return Err("parquet_save: rows array".into()),
    };
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
    let mut dtypes = vec![1u8; ncols]; // default f64; promote to str/i64 as needed
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
    // Mixed int/float column → f64.
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
    let mut buf = Vec::new();
    buf.extend_from_slice(b"KPQT");
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&(ncols as u32).to_le_bytes());
    for (ci, name) in col_names.iter().enumerate() {
        let nb = name.as_bytes();
        buf.extend_from_slice(&(nb.len() as u32).to_le_bytes());
        buf.extend_from_slice(nb);
        buf.push(dtypes[ci]);
    }
    buf.extend_from_slice(&(nrows as u32).to_le_bytes());
    for (ci, col) in cols.iter().enumerate() {
        match dtypes[ci] {
            1 => {
                for c in col {
                    let v = num(c).unwrap_or(0.0);
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
            2 => {
                for c in col {
                    let v = num(c).unwrap_or(0.0) as i64;
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
            _ => {
                for c in col {
                    let s = match c {
                        Value::String(s) => s.clone(),
                        Value::Null => String::new(),
                        other => crate::value::format_value(other),
                    };
                    let sb = s.as_bytes();
                    buf.extend_from_slice(&(sb.len() as u32).to_le_bytes());
                    buf.extend_from_slice(sb);
                }
            }
        }
    }
    std::fs::write(path, &buf).map_err(|e| format!("parquet_save({path}): {e}"))?;
    Ok(int_out(nrows as i64))
}

fn parquet_load(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("parquet_load(path)".into()),
    };
    let buf = std::fs::read(path).map_err(|e| format!("parquet_load({path}): {e}"))?;
    if buf.len() < 12 || &buf[0..4] != b"KPQT" {
        return Err("parquet_load: bad magic (expect KPQT1)".into());
    }
    let ver = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    if ver != 1 {
        return Err(format!("parquet_load: unsupported version {ver}"));
    }
    let mut off = 8usize;
    let take_u32 = |buf: &[u8], off: &mut usize| -> Result<u32, String> {
        if *off + 4 > buf.len() {
            return Err("parquet_load: truncated".into());
        }
        let v = u32::from_le_bytes(buf[*off..*off + 4].try_into().unwrap());
        *off += 4;
        Ok(v)
    };
    let ncols = take_u32(&buf, &mut off)? as usize;
    let mut names = Vec::with_capacity(ncols);
    let mut dtypes = Vec::with_capacity(ncols);
    for _ in 0..ncols {
        let nlen = take_u32(&buf, &mut off)? as usize;
        if off + nlen + 1 > buf.len() {
            return Err("parquet_load: truncated name".into());
        }
        let name = String::from_utf8_lossy(&buf[off..off + nlen]).into_owned();
        off += nlen;
        dtypes.push(buf[off]);
        off += 1;
        names.push(name);
    }
    let nrows = take_u32(&buf, &mut off)? as usize;
    let mut cols: Vec<Vec<Value>> = Vec::with_capacity(ncols);
    for di in 0..ncols {
        let mut col = Vec::with_capacity(nrows);
        match dtypes[di] {
            1 => {
                for _ in 0..nrows {
                    if off + 8 > buf.len() {
                        return Err("parquet_load: truncated f64".into());
                    }
                    let v = f64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
                    off += 8;
                    col.push(float_out(v));
                }
            }
            2 => {
                for _ in 0..nrows {
                    if off + 8 > buf.len() {
                        return Err("parquet_load: truncated i64".into());
                    }
                    let v = i64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
                    off += 8;
                    col.push(int_out(v));
                }
            }
            _ => {
                for _ in 0..nrows {
                    let nlen = take_u32(&buf, &mut off)? as usize;
                    if off + nlen > buf.len() {
                        return Err("parquet_load: truncated str".into());
                    }
                    let s = String::from_utf8_lossy(&buf[off..off + nlen]).into_owned();
                    off += nlen;
                    col.push(Value::String(s));
                }
            }
        }
        cols.push(col);
    }
    let mut out_rows = Vec::with_capacity(nrows);
    for r in 0..nrows {
        let mut m = HashMap::new();
        for c in 0..ncols {
            m.insert(names[c].clone(), cols[c][r].clone());
        }
        out_rows.push(Value::Object(m));
    }
    Ok(Value::Array(out_rows))
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

fn plot_canvas_result(id: u64, w: u32, h: u32) -> Result<Value, String> {
    let mut o = HashMap::new();
    o.insert("id".into(), int_out(id as i64));
    o.insert("width".into(), int_out(w as i64));
    o.insert("height".into(), int_out(h as i64));
    o.insert("kind".into(), Value::String("canvas2d".into()));
    if let Ok(url) = canvas2d::to_data_url(id, "image/png") {
        o.insert("dataUrl".into(), Value::String(url));
    }
    Ok(Value::Object(o))
}

fn map_xy(xs: &[f64], ys: &[f64], w: f64, h: f64, pad: f64) -> Vec<(f64, f64)> {
    let xmin = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let xmax = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ymin = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let ymax = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let xspan = (xmax - xmin).abs().max(1e-12);
    let yspan = (ymax - ymin).abs().max(1e-12);
    xs.iter()
        .zip(ys.iter())
        .map(|(x, y)| {
            let px = pad + (*x - xmin) / xspan * (w - 2.0 * pad);
            let py = h - pad - (*y - ymin) / yspan * (h - 2.0 * pad);
            (px, py)
        })
        .collect()
}

fn plot_line(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ys = vector_at(args, 0, "plot_line")?;
    if ys.is_empty() {
        return Err("plot_line: empty".into());
    }
    let w = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(320.0)
        .clamp(64.0, 2048.0) as u32;
    let h = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(180.0)
        .clamp(48.0, 2048.0) as u32;
    let xs: Vec<f64> = (0..ys.len()).map(|i| i as f64).collect();
    let id = canvas2d::create(w, h)?;
    canvas2d::set_fill_style(id, "#ffffff")?;
    canvas2d::fill_rect(id, 0.0, 0.0, w as f64, h as f64)?;
    canvas2d::set_stroke_style(id, "#3366cc")?;
    canvas2d::set_line_width(id, 2.0)?;
    canvas2d::begin_path(id)?;
    let pts = map_xy(&xs, &ys, w as f64, h as f64, 12.0);
    if let Some((x0, y0)) = pts.first() {
        canvas2d::move_to(id, *x0, *y0)?;
        for (x, y) in pts.iter().skip(1) {
            canvas2d::line_to(id, *x, *y)?;
        }
    }
    canvas2d::stroke(id)?;
    plot_canvas_result(id, w, h)
}

fn plot_scatter(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let xs = vector_at(args, 0, "plot_scatter")?;
    let ys = vector_at(args, 1, "plot_scatter")?;
    if xs.len() != ys.len() || xs.is_empty() {
        return Err("plot_scatter: xs/ys length mismatch".into());
    }
    let w = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(320.0)
        .clamp(64.0, 2048.0) as u32;
    let h = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(180.0)
        .clamp(48.0, 2048.0) as u32;
    let id = canvas2d::create(w, h)?;
    canvas2d::set_fill_style(id, "#ffffff")?;
    canvas2d::fill_rect(id, 0.0, 0.0, w as f64, h as f64)?;
    canvas2d::set_fill_style(id, "#cc4433")?;
    let pts = map_xy(&xs, &ys, w as f64, h as f64, 12.0);
    for (x, y) in pts {
        canvas2d::begin_path(id)?;
        canvas2d::arc(id, x, y, 3.0, 0.0, std::f64::consts::TAU, false)?;
        canvas2d::fill(id)?;
    }
    plot_canvas_result(id, w, h)
}

fn plot_hist(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ys = vector_at(args, 0, "plot_hist")?;
    if ys.is_empty() {
        return Err("plot_hist: empty".into());
    }
    let bins = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(10.0)
        .clamp(2.0, 64.0) as usize;
    let w = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(320.0)
        .clamp(64.0, 2048.0) as u32;
    let h = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(180.0)
        .clamp(48.0, 2048.0) as u32;
    let min = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).abs().max(1e-12);
    let mut counts = vec![0.0; bins];
    for y in &ys {
        let mut b = ((*y - min) / span * bins as f64).floor() as usize;
        if b >= bins {
            b = bins - 1;
        }
        counts[b] += 1.0;
    }
    let cmax = counts.iter().copied().fold(1e-12_f64, f64::max);
    let id = canvas2d::create(w, h)?;
    canvas2d::set_fill_style(id, "#ffffff")?;
    canvas2d::fill_rect(id, 0.0, 0.0, w as f64, h as f64)?;
    canvas2d::set_fill_style(id, "#4488aa")?;
    let pad = 12.0;
    let bw = (w as f64 - 2.0 * pad) / bins as f64;
    for (i, c) in counts.iter().enumerate() {
        let bar_h = (*c / cmax) * (h as f64 - 2.0 * pad);
        let x = pad + i as f64 * bw;
        let y = h as f64 - pad - bar_h;
        canvas2d::fill_rect(id, x, y, bw * 0.9, bar_h)?;
    }
    plot_canvas_result(id, w, h)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn rows_to_html(rows: &[Value]) -> String {
    let mut html = String::from("<table class=\"kab-table\">");
    for (i, row) in rows.iter().enumerate() {
        html.push_str("<tr>");
        match row {
            Value::Array(cells) => {
                for c in cells {
                    let tag = if i == 0 { "th" } else { "td" };
                    html.push_str(&format!(
                        "<{tag}>{}</{tag}>",
                        html_escape(&crate::value::format_value(c))
                    ));
                }
            }
            other => {
                html.push_str(&format!(
                    "<td>{}</td>",
                    html_escape(&crate::value::format_value(other))
                ));
            }
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    html
}

/// rich_display(value) → {mime, text, html?, image?}
fn rich_display(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("rich_display(value)")?;
    let mut out = HashMap::new();
    let text = match pretty(args, env)? {
        Value::String(s) => s,
        other => crate::value::format_value(&other),
    };
    out.insert("text".into(), Value::String(text.clone()));
    out.insert("mime".into(), Value::String("text/plain".into()));

    if let Value::Object(m) = v {
        if matches!(m.get("kind"), Some(Value::String(k)) if k == "canvas2d") {
            if let Some(Value::String(url)) = m.get("dataUrl") {
                out.insert("mime".into(), Value::String("image/png".into()));
                out.insert("image".into(), Value::String(url.clone()));
                out.insert(
                    "html".into(),
                    Value::String(format!(
                        "<img class=\"kab-plot\" alt=\"plot\" src=\"{}\" />",
                        html_escape(url)
                    )),
                );
                return Ok(Value::Object(out));
            }
        }
        if matches!(m.get("kind"), Some(Value::String(k)) if k == "train_progress") {
            let epoch = match m.get("epoch") {
                Some(v) => num(v).unwrap_or(0.0) as i64,
                _ => 0,
            };
            let loss = match m.get("loss") {
                Some(v) => num(v).unwrap_or(0.0),
                _ => 0.0,
            };
            let bar_pct = (100.0 * (1.0 - loss.min(1.0))).clamp(0.0, 100.0);
            let html = format!(
                "<div class=\"kab-train\"><div><b>epoch {}</b> loss={}</div>\
                 <div class=\"kab-train-bar\" style=\"width:{}%;max-width:100%\"></div></div>",
                epoch, loss, bar_pct
            );
            out.insert("mime".into(), Value::String("text/html".into()));
            out.insert("html".into(), Value::String(html));
            out.insert(
                "text".into(),
                Value::String(format!("epoch {} loss={}", epoch, loss)),
            );
            return Ok(Value::Object(out));
        }
        if matches!(m.get("__kab_df"), Some(Value::Bool(true))) {
            if let Some(Value::Object(cols)) = m.get("columns") {
                let names: Vec<String> = match m.get("names") {
                    Some(Value::Array(ns)) => ns
                        .iter()
                        .filter_map(|n| match n {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                    _ => cols.keys().cloned().collect(),
                };
                let nrows = match m.get("nrows") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => 0,
                };
                let mut rows = Vec::new();
                rows.push(Value::Array(
                    names.iter().map(|n| Value::String(n.clone())).collect(),
                ));
                for i in 0..nrows {
                    let mut row = Vec::new();
                    for name in &names {
                        if let Some(Value::Array(series)) = cols.get(name) {
                            row.push(series.get(i).cloned().unwrap_or(Value::Null));
                        } else {
                            row.push(Value::Null);
                        }
                    }
                    rows.push(Value::Array(row));
                }
                let html = rows_to_html(&rows);
                out.insert("mime".into(), Value::String("text/html".into()));
                out.insert("html".into(), Value::String(html));
                return Ok(Value::Object(out));
            }
        }
    }

    if let Value::Array(items) = v {
        if items
            .first()
            .map(|x| matches!(x, Value::Array(_)))
            .unwrap_or(false)
        {
            let html = rows_to_html(items);
            out.insert("mime".into(), Value::String("text/html".into()));
            out.insert("html".into(), Value::String(html));
            return Ok(Value::Object(out));
        }
    }

    if text.contains("min=") && text.contains("max=") {
        out.insert("mime".into(), Value::String("text/html".into()));
        out.insert(
            "html".into(),
            Value::String(format!(
                "<pre class=\"kab-ascii\">{}</pre>",
                html_escape(&text)
            )),
        );
    }

    Ok(Value::Object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_csv_parse", "csv_parse"], csv_parse);
    bind(&["science_csv_load", "csv_load"], csv_load);
    bind(&["science_parquet_save", "parquet_save"], parquet_save);
    bind(&["science_parquet_load", "parquet_load"], parquet_load);
    bind(&["science_table_describe", "table_describe"], table_describe);
    bind(&["science_format_table", "format_table"], format_table);
    bind(&["science_ascii_plot", "ascii_plot"], ascii_plot);
    bind(&["science_plot_line", "plot_line"], plot_line);
    bind(&["science_plot_scatter", "plot_scatter"], plot_scatter);
    bind(&["science_plot_hist", "plot_hist"], plot_hist);
    bind(&["science_pretty", "pretty"], pretty);
    bind(&["science_rich_display", "rich_display"], rich_display);
}
