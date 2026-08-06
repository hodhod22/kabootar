//! DataFrame-lite (SC3f) — columns, select/filter, groupby, join.

use super::helpers::{float_out, int_out, num};
use crate::value::{Environment, Value};
use std::collections::HashMap;

const DF_MARK: &str = "__kab_df";

fn df_out(columns: HashMap<String, Vec<Value>>, nrows: usize) -> Value {
    let mut cols_obj = HashMap::new();
    let mut names = Vec::new();
    for (k, v) in columns {
        names.push(Value::String(k.clone()));
        cols_obj.insert(k, Value::Array(v));
    }
    names.sort_by(|a, b| match (a, b) {
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    let mut m = HashMap::new();
    m.insert(DF_MARK.into(), Value::Bool(true));
    m.insert("columns".into(), Value::Object(cols_obj));
    m.insert("names".into(), Value::Array(names));
    m.insert("nrows".into(), int_out(nrows as i64));
    Value::Object(m)
}

fn df_parts(v: &Value) -> Result<(HashMap<String, Vec<Value>>, usize), String> {
    match v {
        Value::Object(m) if matches!(m.get(DF_MARK), Some(Value::Bool(true))) => {
            let cols = match m.get("columns") {
                Some(Value::Object(c)) => c,
                _ => return Err("df: missing columns".into()),
            };
            let mut out = HashMap::new();
            let mut nrows = 0usize;
            for (k, val) in cols {
                let Value::Array(items) = val else {
                    return Err("df: column must be array".into());
                };
                if out.is_empty() {
                    nrows = items.len();
                } else if items.len() != nrows {
                    return Err("df: column length mismatch".into());
                }
                out.insert(k.clone(), items.clone());
            }
            Ok((out, nrows))
        }
        _ => Err("expected dataframe".into()),
    }
}

/// df_from({col: [...], ...}) or df_from_rows([[...]], ["a","b"])
fn df_from(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if let Some(Value::Object(m)) = args.first() {
        if !matches!(m.get(DF_MARK), Some(Value::Bool(true))) {
            let mut cols = HashMap::new();
            let mut nrows = None;
            for (k, v) in m {
                if k.starts_with("__kab") {
                    continue;
                }
                let Value::Array(items) = v else {
                    continue;
                };
                if let Some(n) = nrows {
                    if items.len() != n {
                        return Err("df_from: length mismatch".into());
                    }
                } else {
                    nrows = Some(items.len());
                }
                cols.insert(k.clone(), items.clone());
            }
            if cols.is_empty() {
                return Err("df_from: columns must be arrays".into());
            }
            return Ok(df_out(cols, nrows.unwrap_or(0)));
        }
    }
    // rows + names
    let rows = match args.first() {
        Some(Value::Array(r)) => r,
        _ => return Err("df_from(colsObj) or df_from(rows, names)".into()),
    };
    let names = match args.get(1) {
        Some(Value::Array(n)) => n
            .iter()
            .map(|x| match x {
                Value::String(s) => Ok::<String, String>(s.clone()),
                _ => Err("df_from: names must be strings".into()),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            if rows.is_empty() {
                return Ok(df_out(HashMap::new(), 0));
            }
            let ncols = match &rows[0] {
                Value::Array(c) => c.len(),
                _ => return Err("df_from: rows must be arrays".into()),
            };
            (0..ncols).map(|i| format!("c{i}")).collect()
        }
    };
    let mut cols: HashMap<String, Vec<Value>> =
        names.iter().map(|n| (n.clone(), Vec::new())).collect();
    for row in rows {
        let Value::Array(cells) = row else {
            return Err("df_from: jagged row".into());
        };
        if cells.len() != names.len() {
            return Err("df_from: row width mismatch".into());
        }
        for (i, name) in names.iter().enumerate() {
            cols.get_mut(name).unwrap().push(cells[i].clone());
        }
    }
    Ok(df_out(cols, rows.len()))
}

fn df_select(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (cols, nrows) = df_parts(args.first().ok_or("df_select(df, names)")?)?;
    let names = match args.get(1) {
        Some(Value::Array(n)) => n,
        Some(Value::String(s)) => {
            let mut out = HashMap::new();
            let col = cols.get(s).ok_or_else(|| format!("df_select: missing {s}"))?;
            out.insert(s.clone(), col.clone());
            return Ok(df_out(out, nrows));
        }
        _ => return Err("df_select(df, [names]|name)".into()),
    };
    let mut out = HashMap::new();
    for n in names {
        let Value::String(s) = n else {
            return Err("df_select: name must be string".into());
        };
        let col = cols.get(s).ok_or_else(|| format!("df_select: missing {s}"))?;
        out.insert(s.clone(), col.clone());
    }
    Ok(df_out(out, nrows))
}

fn cmp_val(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (num(a), num(b)) {
        (Ok(x), Ok(y)) => x.partial_cmp(&y),
        _ => match (a, b) {
            (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
            _ => None,
        },
    }
}

/// df_filter(df, col, op, value) — op: == != > >= < <=
fn df_filter(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (cols, nrows) = df_parts(args.first().ok_or("df_filter")?)?;
    let colname = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("df_filter(df, col, op, value)".into()),
    };
    let op = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("df_filter: op string".into()),
    };
    let rhs = args.get(3).ok_or("df_filter: value")?;
    let series = cols
        .get(colname)
        .ok_or_else(|| format!("df_filter: missing {colname}"))?;
    let mut keep = vec![false; nrows];
    for i in 0..nrows {
        let ok = match op {
            "==" | "eq" => cmp_val(&series[i], rhs) == Some(std::cmp::Ordering::Equal),
            "!=" | "ne" => cmp_val(&series[i], rhs) != Some(std::cmp::Ordering::Equal),
            ">" | "gt" => cmp_val(&series[i], rhs) == Some(std::cmp::Ordering::Greater),
            ">=" | "ge" => matches!(
                cmp_val(&series[i], rhs),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ),
            "<" | "lt" => cmp_val(&series[i], rhs) == Some(std::cmp::Ordering::Less),
            "<=" | "le" => matches!(
                cmp_val(&series[i], rhs),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ),
            _ => return Err("df_filter: unknown op".into()),
        };
        keep[i] = ok;
    }
    let mut out = HashMap::new();
    let mut new_n = 0usize;
    for (name, series) in &cols {
        let filtered: Vec<Value> = series
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, v)| v.clone())
            .collect();
        new_n = filtered.len();
        out.insert(name.clone(), filtered);
    }
    Ok(df_out(out, new_n))
}

/// df_groupby(df, key_col, agg_col, how) — how: mean|sum|count
fn df_groupby(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (cols, nrows) = df_parts(args.first().ok_or("df_groupby")?)?;
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("df_groupby(df, key, agg, how)".into()),
    };
    let agg = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("df_groupby: agg col".into()),
    };
    let how = match args.get(3) {
        Some(Value::String(s)) => s.as_str(),
        _ => "mean",
    };
    let keys = cols.get(key).ok_or_else(|| format!("df_groupby: missing {key}"))?;
    let vals = cols.get(agg).ok_or_else(|| format!("df_groupby: missing {agg}"))?;
    let mut order: Vec<String> = Vec::new();
    let mut sums: HashMap<String, f64> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for i in 0..nrows {
        let k = crate::value::format_value(&keys[i]);
        if !sums.contains_key(&k) {
            order.push(k.clone());
        }
        let v = num(&vals[i]).unwrap_or(0.0);
        *sums.entry(k.clone()).or_insert(0.0) += v;
        *counts.entry(k).or_insert(0) += 1;
    }
    let mut key_col = Vec::new();
    let mut agg_col = Vec::new();
    for k in order {
        key_col.push(Value::String(k.clone()));
        let c = *counts.get(&k).unwrap_or(&0);
        let s = *sums.get(&k).unwrap_or(&0.0);
        let out = match how {
            "sum" => s,
            "count" => c as f64,
            _ => {
                if c == 0 {
                    0.0
                } else {
                    s / c as f64
                }
            }
        };
        agg_col.push(float_out(out));
    }
    let mut out = HashMap::new();
    out.insert(key.into(), key_col);
    out.insert(format!("{agg}_{how}"), agg_col);
    Ok(df_out(out, counts.len()))
}

/// df_join(left, right, on, how?) — how: inner|left|outer (default inner)
fn df_join(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (left, ln) = df_parts(args.first().ok_or("df_join")?)?;
    let (right, rn) = df_parts(args.get(1).ok_or("df_join")?)?;
    let on = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("df_join(left, right, on, how?)".into()),
    };
    let how = match args.get(3) {
        Some(Value::String(s)) => s.as_str(),
        _ => "inner",
    };
    let lk = left.get(on).ok_or_else(|| format!("df_join: left missing {on}"))?;
    let rk = right.get(on).ok_or_else(|| format!("df_join: right missing {on}"))?;
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for i in 0..rn {
        index
            .entry(crate::value::format_value(&rk[i]))
            .or_default()
            .push(i);
    }
    let mut out_cols: HashMap<String, Vec<Value>> = HashMap::new();
    for name in left.keys() {
        out_cols.insert(name.clone(), Vec::new());
    }
    for name in right.keys() {
        if name != on {
            let rname = if left.contains_key(name) {
                format!("{name}_r")
            } else {
                name.clone()
            };
            out_cols.insert(rname, Vec::new());
        }
    }
    let right_only_names: Vec<String> = out_cols
        .keys()
        .filter(|k| !left.contains_key(*k))
        .cloned()
        .collect();
    let mut matched_right: Vec<bool> = vec![false; rn];
    let mut nrows = 0usize;

    let push_left_nulls = |out_cols: &mut HashMap<String, Vec<Value>>, i: usize| {
        for (name, series) in &left {
            out_cols.get_mut(name).unwrap().push(series[i].clone());
        }
        for rname in &right_only_names {
            out_cols.get_mut(rname).unwrap().push(Value::Null);
        }
    };

    for i in 0..ln {
        let key = crate::value::format_value(&lk[i]);
        if let Some(js) = index.get(&key) {
            for &j in js {
                matched_right[j] = true;
                for (name, series) in &left {
                    out_cols.get_mut(name).unwrap().push(series[i].clone());
                }
                for (name, series) in &right {
                    if name == on {
                        continue;
                    }
                    let rname = if left.contains_key(name) {
                        format!("{name}_r")
                    } else {
                        name.clone()
                    };
                    out_cols.get_mut(&rname).unwrap().push(series[j].clone());
                }
                nrows += 1;
            }
        } else if how == "left" || how == "outer" {
            push_left_nulls(&mut out_cols, i);
            nrows += 1;
        }
    }
    if how == "outer" {
        for j in 0..rn {
            if matched_right[j] {
                continue;
            }
            for name in left.keys() {
                if name == on {
                    out_cols
                        .get_mut(name)
                        .unwrap()
                        .push(rk[j].clone());
                } else {
                    out_cols.get_mut(name).unwrap().push(Value::Null);
                }
            }
            for (name, series) in &right {
                if name == on {
                    continue;
                }
                let rname = if left.contains_key(name) {
                    format!("{name}_r")
                } else {
                    name.clone()
                };
                out_cols.get_mut(&rname).unwrap().push(series[j].clone());
            }
            nrows += 1;
        }
    }
    Ok(df_out(out_cols, nrows))
}

fn df_nrows(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, n) = df_parts(args.first().ok_or("df_nrows")?)?;
    Ok(int_out(n as i64))
}

fn df_head(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (cols, nrows) = df_parts(args.first().ok_or("df_head")?)?;
    let n = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(5.0)
        .max(0.0) as usize;
    let take = n.min(nrows);
    let mut out = HashMap::new();
    for (k, v) in cols {
        out.insert(k, v.into_iter().take(take).collect());
    }
    Ok(df_out(out, take))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_df_from", "df_from"], df_from);
    bind(&["science_df_select", "df_select"], df_select);
    bind(&["science_df_filter", "df_filter"], df_filter);
    bind(&["science_df_groupby", "df_groupby"], df_groupby);
    bind(&["science_df_join", "df_join"], df_join);
    bind(&["science_df_nrows", "df_nrows"], df_nrows);
    bind(&["science_df_head", "df_head"], df_head);
}
