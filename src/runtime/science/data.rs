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

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_csv_parse", "csv_parse"], csv_parse);
    bind(&["science_csv_load", "csv_load"], csv_load);
    bind(&["science_table_describe", "table_describe"], table_describe);
    bind(&["science_format_table", "format_table"], format_table);
    bind(&["science_ascii_plot", "ascii_plot"], ascii_plot);
    bind(&["science_plot_line", "plot_line"], plot_line);
    bind(&["science_plot_scatter", "plot_scatter"], plot_scatter);
    bind(&["science_plot_hist", "plot_hist"], plot_hist);
    bind(&["science_pretty", "pretty"], pretty);
}
