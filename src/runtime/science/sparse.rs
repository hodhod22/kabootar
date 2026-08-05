//! Sparse matrices — CSR/COO + SpMV (SC1j).

use super::helpers::{int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn sparse_out(
    fmt: &str,
    rows: usize,
    cols: usize,
    data: &[f64],
    indices: &[i64],
    indptr: &[i64],
) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_sparse".into(), Value::Bool(true));
    m.insert("format".into(), Value::String(fmt.into()));
    m.insert("nrows".into(), int_out(rows as i64));
    m.insert("ncols".into(), int_out(cols as i64));
    m.insert("data".into(), vector_out(data));
    m.insert("indices".into(), Value::Array(indices.iter().map(|n| int_out(*n)).collect()));
    if !indptr.is_empty() {
        m.insert(
            "indptr".into(),
            Value::Array(indptr.iter().map(|n| int_out(*n)).collect()),
        );
    }
    Value::Object(m)
}

fn parse_sparse(v: &Value) -> Result<(String, usize, usize, Vec<f64>, Vec<i64>, Vec<i64>), String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_sparse"), Some(Value::Bool(true))) => {
            let fmt = match m.get("format") {
                Some(Value::String(s)) => s.clone(),
                _ => "csr".into(),
            };
            let rows = num(m.get("nrows").ok_or("sparse: nrows")?)? as usize;
            let cols = num(m.get("ncols").ok_or("sparse: ncols")?)? as usize;
            let data = match m.get("data") {
                Some(Value::Array(a)) => a.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err("sparse: data".into()),
            };
            let indices = match m.get("indices") {
                Some(Value::Array(a)) => a
                    .iter()
                    .map(|x| num(x).map(|n| n as i64))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("sparse: indices".into()),
            };
            let indptr = if let Some(Value::Array(a)) = m.get("indptr") {
                a
                    .iter()
                    .map(|x| num(x).map(|n| n as i64))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };
            Ok((fmt, rows, cols, data, indices, indptr))
        }
        _ => Err("expected sparse matrix object".into()),
    }
}

/// sparse_from_coo(rows[], cols[], data[], nrows, ncols)
fn sparse_from_coo(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let r = match args.first() {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_coo(rows, cols, data, nrows, ncols)".into()),
    };
    let c = match args.get(1) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_coo: cols".into()),
    };
    let d = vector_at(args, 2, "sparse_from_coo")?;
    let nrows = num_at(args, 3, "sparse_from_coo")? as usize;
    let ncols = num_at(args, 4, "sparse_from_coo")? as usize;
    if r.len() != c.len() || r.len() != d.len() {
        return Err("sparse_from_coo: length mismatch".into());
    }
    Ok(sparse_out("coo", nrows, ncols, &d, &c, &r))
}

/// sparse_from_csr(data, indices, indptr, nrows, ncols)
fn sparse_from_csr(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "sparse_from_csr")?;
    let indices = match args.get(1) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_csr: indices".into()),
    };
    let indptr = match args.get(2) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_csr: indptr".into()),
    };
    let nrows = num_at(args, 3, "sparse_from_csr")? as usize;
    let ncols = num_at(args, 4, "sparse_from_csr")? as usize;
    if data.len() != indices.len() || indptr.len() != nrows + 1 {
        return Err("sparse_from_csr: shape mismatch".into());
    }
    Ok(sparse_out("csr", nrows, ncols, &data, &indices, &indptr))
}

/// sparse_to_csr(coo) — sort COO → CSR
fn sparse_to_csr(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (fmt, rows, cols, data, indices, row_idx) = parse_sparse(args.first().ok_or("sparse_to_csr")?)?;
    if fmt != "coo" {
        return Err("sparse_to_csr: expected COO".into());
    }
    let mut entries: Vec<(i64, i64, f64)> = row_idx
        .iter()
        .zip(indices.iter())
        .zip(data.iter())
        .map(|((r, c), v)| (*r, *c, *v))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut csr_data = Vec::new();
    let mut csr_indices = Vec::new();
    let mut indptr = vec![0i64];
    for row in 0..rows {
        for (r, c, v) in &entries {
            if *r as usize == row {
                csr_data.push(*v);
                csr_indices.push(*c);
            }
        }
        indptr.push(csr_data.len() as i64);
    }
    Ok(sparse_out("csr", rows, cols, &csr_data, &csr_indices, &indptr))
}

/// sparse_spmv(A, x) — y = A*x
fn sparse_spmv(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (fmt, rows, cols, data, indices, aux) = parse_sparse(args.first().ok_or("sparse_spmv")?)?;
    let x = vector_at(args, 1, "sparse_spmv")?;
    if x.len() != cols {
        return Err("sparse_spmv: x length mismatch".into());
    }
    let mut y = vec![0.0; rows];
    match fmt.as_str() {
        "csr" => {
            let indptr = aux;
            for i in 0..rows {
                let start = indptr[i] as usize;
                let end = indptr[i + 1] as usize;
                let mut s = 0.0;
                for k in start..end {
                    let j = indices[k] as usize;
                    s += data[k] * x[j];
                }
                y[i] = s;
            }
        }
        "coo" => {
            let row_idx = aux;
            for k in 0..data.len() {
                let r = row_idx[k] as usize;
                let c = indices[k] as usize;
                y[r] += data[k] * x[c];
            }
        }
        _ => return Err("sparse_spmv: unknown format".into()),
    }
    Ok(vector_out(&y))
}

/// sparse_lstsq(A_csr, b, iters?) — CG on normal equations (subset).
fn sparse_lstsq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_lstsq(A, b)")?;
    let b = vector_at(args, 1, "sparse_lstsq")?;
    let iters = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(200.0) as usize;
    let csr = if parse_sparse(a)?.0 == "csr" {
        a.clone()
    } else {
        sparse_to_csr(&[a.clone()], _env)?
    };
    let (_, rows, cols, _, _, _) = parse_sparse(&csr)?;
    if b.len() != rows {
        return Err("sparse_lstsq: b length".into());
    }
    // x = CG(A^T A, A^T b)
    let mut x = vec![0.0; cols];
    let mut atb = vec![0.0; cols];
    for j in 0..cols {
        let mut unit = vec![0.0; cols];
        unit[j] = 1.0;
        let col_a = sparse_spmv(&[csr.clone(), vector_out(&unit)], _env)?;
        let col = vector_at(&[col_a], 0, "col")?;
        let mut s = 0.0;
        for i in 0..rows {
            s += col[i] * b[i];
        }
        atb[j] = s;
    }
    let mut r = atb.clone();
    let mut p = r.clone();
    let mut rsold = r.iter().map(|v| v * v).sum::<f64>();
    for _ in 0..iters {
        let ap = vector_at(&[sparse_spmv(&[csr.clone(), vector_out(&p)], _env)?], 0, "ap")?;
        let mut atap = vec![0.0; cols];
        for j in 0..cols {
            let mut unit = vec![0.0; cols];
            unit[j] = 1.0;
            let col_a = sparse_spmv(&[csr.clone(), vector_out(&unit)], _env)?;
            let col = vector_at(&[col_a], 0, "atap")?;
            let mut s = 0.0;
            for i in 0..rows {
                s += col[i] * ap[i];
            }
            atap[j] = s;
        }
        let alpha = rsold / p.iter().zip(atap.iter()).map(|(pi, ai)| pi * ai).sum::<f64>();
        for i in 0..cols {
            x[i] += alpha * p[i];
            r[i] -= alpha * atap[i];
        }
        let rsnew = r.iter().map(|v| v * v).sum::<f64>();
        if rsnew < 1e-12 {
            break;
        }
        let beta = rsnew / rsold;
        for i in 0..cols {
            p[i] = r[i] + beta * p[i];
        }
        rsold = rsnew;
    }
    Ok(vector_out(&x))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_sparse_from_coo", "sparse_from_coo"], sparse_from_coo);
    bind(&["science_sparse_from_csr", "sparse_from_csr"], sparse_from_csr);
    bind(&["science_sparse_to_csr", "sparse_to_csr"], sparse_to_csr);
    bind(&["science_sparse_spmv", "sparse_spmv"], sparse_spmv);
    bind(&["science_sparse_lstsq", "sparse_lstsq"], sparse_lstsq);
}
