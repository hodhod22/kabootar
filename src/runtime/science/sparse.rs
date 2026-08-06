//! Sparse matrices — CSR/COO + SpMV (SC1j).

use super::helpers::{int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet};

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

/// sparse_from_csc(data, indices, indptr, nrows, ncols) — CSC: indices=row, indptr=ncols+1.
fn sparse_from_csc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "sparse_from_csc")?;
    let indices = match args.get(1) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_csc: indices".into()),
    };
    let indptr = match args.get(2) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| num(x).map(|n| n as i64))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_from_csc: indptr".into()),
    };
    let nrows = num_at(args, 3, "sparse_from_csc")? as usize;
    let ncols = num_at(args, 4, "sparse_from_csc")? as usize;
    if data.len() != indices.len() || indptr.len() != ncols + 1 {
        return Err("sparse_from_csc: shape mismatch".into());
    }
    Ok(sparse_out("csc", nrows, ncols, &data, &indices, &indptr))
}

/// sparse_to_csc(coo|csr) -> CSC.
fn sparse_to_csc(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_to_csc")?.clone();
    let (fmt, rows, cols, data, indices, aux) = parse_sparse(&a)?;
    let entries: Vec<(i64, i64, f64)> = match fmt.as_str() {
        "coo" => aux
            .iter()
            .zip(indices.iter())
            .zip(data.iter())
            .map(|((r, c), v)| (*r, *c, *v))
            .collect(),
        "csr" => {
            let mut e = Vec::new();
            for r in 0..rows {
                let start = aux[r] as usize;
                let end = aux[r + 1] as usize;
                for k in start..end {
                    e.push((r as i64, indices[k], data[k]));
                }
            }
            e
        }
        "csc" => return Ok(a),
        _ => {
            // try via COO path after CSR convert
            let csr = if fmt == "coo" {
                sparse_to_csr(&[a], env)?
            } else {
                return Err("sparse_to_csc: unsupported format".into());
            };
            return sparse_to_csc(&[csr], env);
        }
    };
    let mut entries = entries;
    entries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let mut csc_data = Vec::new();
    let mut csc_indices = Vec::new();
    let mut indptr = vec![0i64];
    for col in 0..cols {
        for (r, c, v) in &entries {
            if *c as usize == col {
                csc_data.push(*v);
                csc_indices.push(*r);
            }
        }
        indptr.push(csc_data.len() as i64);
    }
    Ok(sparse_out(
        "csc",
        rows,
        cols,
        &csc_data,
        &csc_indices,
        &indptr,
    ))
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
        "csc" => {
            let indptr = aux;
            for j in 0..cols {
                let start = indptr[j] as usize;
                let end = indptr[j + 1] as usize;
                let xj = x[j];
                for k in start..end {
                    let i = indices[k] as usize;
                    y[i] += data[k] * xj;
                }
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

/// sparse_gather_rows(A, rowIndices) — select rows (CSR/COO in → CSR out).
fn sparse_gather_rows(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_gather_rows(A, rowIndices)")?;
    let row_ix = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_gather_rows: rowIndices array".into()),
    };
    let csr = if parse_sparse(a)?.0 == "csr" {
        a.clone()
    } else {
        sparse_to_csr(&[a.clone()], env)?
    };
    let (_, nrows, ncols, data, indices, indptr) = parse_sparse(&csr)?;
    for &r in &row_ix {
        if r >= nrows {
            return Err("sparse_gather_rows: row OOB".into());
        }
    }
    let mut out_data = Vec::new();
    let mut out_indices = Vec::new();
    let mut out_indptr = vec![0i64];
    for &r in &row_ix {
        let start = indptr[r] as usize;
        let end = indptr[r + 1] as usize;
        for k in start..end {
            out_data.push(data[k]);
            out_indices.push(indices[k]);
        }
        out_indptr.push(out_data.len() as i64);
    }
    Ok(sparse_out(
        "csr",
        row_ix.len(),
        ncols,
        &out_data,
        &out_indices,
        &out_indptr,
    ))
}

/// sparse_compress_rows(A, mask) — keep rows where mask[i] != 0.
fn sparse_compress_rows(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_compress_rows(A, mask)")?;
    let mask = vector_at(args, 1, "sparse_compress_rows")?;
    let (_, nrows, _, _, _, _) = parse_sparse(a)?;
    if mask.len() != nrows {
        return Err("sparse_compress_rows: mask length".into());
    }
    let ix_arr = Value::Array(
        mask.iter()
            .enumerate()
            .filter(|(_, m)| **m != 0.0)
            .map(|(i, _)| int_out(i as i64))
            .collect(),
    );
    sparse_gather_rows(&[a.clone(), ix_arr], env)
}

/// sparse_gather_cols(A, colIndices) — select columns (CSR/COO/CSC in → CSR out).
fn sparse_gather_cols(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_gather_cols(A, colIndices)")?;
    let col_ix = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("sparse_gather_cols: colIndices array".into()),
    };
    // Fast path: CSC column slices.
    if parse_sparse(a)?.0 == "csc" {
        let (_, nrows, ncols, data, indices, indptr) = parse_sparse(a)?;
        for &c in &col_ix {
            if c >= ncols {
                return Err("sparse_gather_cols: col OOB".into());
            }
        }
        let mut out_data = Vec::new();
        let mut out_indices = Vec::new();
        let mut out_indptr = vec![0i64];
        // Build CSR by scanning selected columns into row buckets.
        let mut row_buckets: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nrows];
        for (new_c, &old_c) in col_ix.iter().enumerate() {
            let start = indptr[old_c] as usize;
            let end = indptr[old_c + 1] as usize;
            for k in start..end {
                let r = indices[k] as usize;
                row_buckets[r].push((new_c, data[k]));
            }
        }
        for r in 0..nrows {
            for (c, v) in &row_buckets[r] {
                out_data.push(*v);
                out_indices.push(*c as i64);
            }
            out_indptr.push(out_data.len() as i64);
        }
        return Ok(sparse_out(
            "csr",
            nrows,
            col_ix.len(),
            &out_data,
            &out_indices,
            &out_indptr,
        ));
    }
    let csr = if parse_sparse(a)?.0 == "csr" {
        a.clone()
    } else {
        sparse_to_csr(&[a.clone()], env)?
    };
    let (_, nrows, ncols, data, indices, indptr) = parse_sparse(&csr)?;
    for &c in &col_ix {
        if c >= ncols {
            return Err("sparse_gather_cols: col OOB".into());
        }
    }
    // old col -> new col (first occurrence wins for duplicates)
    let mut remap = vec![None; ncols];
    for (new_c, &old_c) in col_ix.iter().enumerate() {
        if remap[old_c].is_none() {
            remap[old_c] = Some(new_c);
        }
    }
    let mut out_data = Vec::new();
    let mut out_indices = Vec::new();
    let mut out_indptr = vec![0i64];
    for r in 0..nrows {
        let start = indptr[r] as usize;
        let end = indptr[r + 1] as usize;
        for k in start..end {
            let old_c = indices[k] as usize;
            if let Some(new_c) = remap.get(old_c).and_then(|x| *x) {
                out_data.push(data[k]);
                out_indices.push(new_c as i64);
            }
        }
        out_indptr.push(out_data.len() as i64);
    }
    Ok(sparse_out(
        "csr",
        nrows,
        col_ix.len(),
        &out_data,
        &out_indices,
        &out_indptr,
    ))
}

/// sparse_compress_cols(A, mask) — keep columns where mask[i] != 0.
fn sparse_compress_cols(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_compress_cols(A, mask)")?;
    let mask = vector_at(args, 1, "sparse_compress_cols")?;
    let (_, _, ncols, _, _, _) = parse_sparse(a)?;
    if mask.len() != ncols {
        return Err("sparse_compress_cols: mask length".into());
    }
    let ix_arr = Value::Array(
        mask.iter()
            .enumerate()
            .filter(|(_, m)| **m != 0.0)
            .map(|(i, _)| int_out(i as i64))
            .collect(),
    );
    sparse_gather_cols(&[a.clone(), ix_arr], env)
}

/// sparse_slice(A, rowStart, rowStop, colStart, colStop) — half-open rectangular view → CSR.
fn sparse_slice(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_slice(A, r0, r1, c0, c1)")?;
    let r0 = num_at(args, 1, "sparse_slice")? as usize;
    let r1 = num_at(args, 2, "sparse_slice")? as usize;
    let c0 = num_at(args, 3, "sparse_slice")? as usize;
    let c1 = num_at(args, 4, "sparse_slice")? as usize;
    if r1 < r0 || c1 < c0 {
        return Err("sparse_slice: stop < start".into());
    }
    let csr = if parse_sparse(a)?.0 == "csr" {
        a.clone()
    } else {
        sparse_to_csr(&[a.clone()], env)?
    };
    let (_, nrows, ncols, data, indices, indptr) = parse_sparse(&csr)?;
    if r1 > nrows || c1 > ncols {
        return Err("sparse_slice: OOB".into());
    }
    let mut out_data = Vec::new();
    let mut out_indices = Vec::new();
    let mut out_indptr = vec![0i64];
    for r in r0..r1 {
        let start = indptr[r] as usize;
        let end = indptr[r + 1] as usize;
        for k in start..end {
            let c = indices[k] as usize;
            if c >= c0 && c < c1 {
                out_data.push(data[k]);
                out_indices.push((c - c0) as i64);
            }
        }
        out_indptr.push(out_data.len() as i64);
    }
    Ok(sparse_out(
        "csr",
        r1 - r0,
        c1 - c0,
        &out_data,
        &out_indices,
        &out_indptr,
    ))
}

/// sparse_from_dense_mask(denseRows, mask) — COO of kept entries (fancy sparse view).
/// denseRows: array of row arrays; mask: same nrows, truthy keeps row.
fn sparse_from_dense_mask(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let rows = match args.first() {
        Some(Value::Array(r)) => r,
        _ => return Err("sparse_from_dense_mask(rows, mask)".into()),
    };
    let mask = vector_at(args, 1, "sparse_from_dense_mask")?;
    if mask.len() != rows.len() {
        return Err("sparse_from_dense_mask: mask length".into());
    }
    let ncols = match rows.first() {
        Some(Value::Array(c)) => c.len(),
        _ => 0,
    };
    let mut rr = Vec::new();
    let mut cc = Vec::new();
    let mut dd = Vec::new();
    let mut out_r = 0i64;
    for (i, row) in rows.iter().enumerate() {
        if mask[i] == 0.0 {
            continue;
        }
        let Value::Array(cells) = row else {
            return Err("sparse_from_dense_mask: jagged".into());
        };
        for (j, cell) in cells.iter().enumerate() {
            let v = num(cell)?;
            if v != 0.0 {
                rr.push(out_r);
                cc.push(j as i64);
                dd.push(v);
            }
        }
        out_r += 1;
    }
    Ok(sparse_out(
        "coo",
        out_r as usize,
        ncols,
        &dd,
        &cc,
        &rr,
    ))
}

fn csr_to_dense(nrows: usize, ncols: usize, data: &[f64], indices: &[i64], indptr: &[i64]) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0; ncols]; nrows];
    for i in 0..nrows {
        let start = indptr[i] as usize;
        let end = indptr[i + 1] as usize;
        for k in start..end {
            a[i][indices[k] as usize] = data[k];
        }
    }
    a
}

fn dense_to_csr_pattern(a: &[Vec<f64>], pattern: &[Vec<bool>]) -> Value {
    let nrows = a.len();
    let ncols = if nrows == 0 { 0 } else { a[0].len() };
    let mut data = Vec::new();
    let mut indices = Vec::new();
    let mut indptr = vec![0i64];
    for i in 0..nrows {
        for j in 0..ncols {
            if pattern[i][j] {
                data.push(a[i][j]);
                indices.push(j as i64);
            }
        }
        indptr.push(data.len() as i64);
    }
    sparse_out("csr", nrows, ncols, &data, &indices, &indptr)
}

/// sparse_ilu0(A) — ILU(0) incomplete LU. Returns {l, u} as CSR (L unit diagonal).
fn sparse_ilu0(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("sparse_ilu0(A)")?;
    let csr = match parse_sparse(a)?.0.as_str() {
        "csr" => a.clone(),
        "csc" => {
            // via COO: expand then CSR
            let (_, rows, cols, data, indices, indptr) = parse_sparse(a)?;
            let mut rr = Vec::new();
            let mut cc = Vec::new();
            let mut dd = Vec::new();
            for j in 0..cols {
                let start = indptr[j] as usize;
                let end = indptr[j + 1] as usize;
                for k in start..end {
                    rr.push(indices[k]);
                    cc.push(j as i64);
                    dd.push(data[k]);
                }
            }
            let coo = sparse_out("coo", rows, cols, &dd, &cc, &rr);
            sparse_to_csr(&[coo], env)?
        }
        _ => sparse_to_csr(&[a.clone()], env)?,
    };
    let (_, n, n2, data, indices, indptr) = parse_sparse(&csr)?;
    if n != n2 {
        return Err("sparse_ilu0: square required".into());
    }
    let mut pattern = vec![vec![false; n]; n];
    for i in 0..n {
        pattern[i][i] = true;
        let start = indptr[i] as usize;
        let end = indptr[i + 1] as usize;
        for k in start..end {
            pattern[i][indices[k] as usize] = true;
        }
    }
    let mut a = csr_to_dense(n, n, &data, &indices, &indptr);
    // IKJ ILU(0)
    for i in 1..n {
        for k in 0..i {
            if !pattern[i][k] {
                continue;
            }
            if a[k][k].abs() < 1e-15 {
                return Err("sparse_ilu0: zero pivot".into());
            }
            a[i][k] /= a[k][k];
            for j in k + 1..n {
                if pattern[i][j] {
                    a[i][j] -= a[i][k] * a[k][j];
                }
            }
        }
    }
    let mut l = vec![vec![0.0; n]; n];
    let mut u = vec![vec![0.0; n]; n];
    let mut lp = vec![vec![false; n]; n];
    let mut up = vec![vec![false; n]; n];
    for i in 0..n {
        l[i][i] = 1.0;
        lp[i][i] = true;
        for j in 0..n {
            if i > j && pattern[i][j] {
                l[i][j] = a[i][j];
                lp[i][j] = true;
            }
            if i <= j && pattern[i][j] {
                u[i][j] = a[i][j];
                up[i][j] = true;
            }
        }
    }
    let mut out = HashMap::new();
    out.insert("l".into(), dense_to_csr_pattern(&l, &lp));
    out.insert("u".into(), dense_to_csr_pattern(&u, &up));
    out.insert("kind".into(), Value::String("ilu0".into()));
    Ok(Value::Object(out))
}

/// sparse_icc0(A) — incomplete Cholesky (no-fill) for SPD. Returns L as CSR.
fn sparse_icc0(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a_in = args.first().ok_or("sparse_icc0(A)")?;
    let csr = if parse_sparse(a_in)?.0 == "csr" {
        a_in.clone()
    } else {
        sparse_to_csr(&[a_in.clone()], env)?
    };
    let (_, n, n2, data, indices, indptr) = parse_sparse(&csr)?;
    if n != n2 {
        return Err("sparse_icc0: square required".into());
    }
    let mut pattern = vec![vec![false; n]; n];
    for i in 0..n {
        pattern[i][i] = true;
        let start = indptr[i] as usize;
        let end = indptr[i + 1] as usize;
        for k in start..end {
            let j = indices[k] as usize;
            pattern[i][j] = true;
            pattern[j][i] = true;
        }
    }
    let a = csr_to_dense(n, n, &data, &indices, &indptr);
    let mut l = vec![vec![0.0; n]; n];
    let mut lp = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..=i {
            if !pattern[i][j] {
                continue;
            }
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            if i == j {
                if s <= 0.0 {
                    return Err("sparse_icc0: not SPD / zero pivot".into());
                }
                l[i][j] = s.sqrt();
            } else {
                if l[j][j].abs() < 1e-15 {
                    return Err("sparse_icc0: zero pivot".into());
                }
                l[i][j] = s / l[j][j];
            }
            lp[i][j] = true;
        }
    }
    let mut out = HashMap::new();
    out.insert("l".into(), dense_to_csr_pattern(&l, &lp));
    out.insert("kind".into(), Value::String("icc0".into()));
    Ok(Value::Object(out))
}

fn ensure_csr(a: &Value, env: &mut Environment) -> Result<Value, String> {
    match parse_sparse(a)?.0.as_str() {
        "csr" => Ok(a.clone()),
        _ => sparse_to_csr(&[a.clone()], env),
    }
}

/// sparse_ilut(A, droptol, fillFactor?) — ILU with threshold dropping.
fn sparse_ilut(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a_in = args.first().ok_or("sparse_ilut(A, droptol, fill?)")?;
    let droptol = num_at(args, 1, "sparse_ilut")?.abs();
    let fill = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(2.0)
        .max(1.0) as usize;
    let csr = ensure_csr(a_in, env)?;
    let (_, n, n2, data, indices, indptr) = parse_sparse(&csr)?;
    if n != n2 {
        return Err("sparse_ilut: square required".into());
    }
    let mut a = csr_to_dense(n, n, &data, &indices, &indptr);
    let max_keep = ((fill as f64) * (data.len() as f64 / n.max(1) as f64)).ceil() as usize + 1;
    for i in 1..n {
        for k in 0..i {
            if a[i][k].abs() < droptol {
                a[i][k] = 0.0;
                continue;
            }
            if a[k][k].abs() < 1e-15 {
                return Err("sparse_ilut: zero pivot".into());
            }
            a[i][k] /= a[k][k];
            for j in k + 1..n {
                a[i][j] -= a[i][k] * a[k][j];
            }
        }
        // Drop small entries and keep largest off-diagonals.
        let mut offs: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i && a[i][j].abs() >= droptol)
            .map(|j| (j, a[i][j].abs()))
            .collect();
        offs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let keep: HashSet<usize> = offs.into_iter().take(max_keep).map(|(j, _)| j).collect();
        for j in 0..n {
            if j != i && !keep.contains(&j) {
                a[i][j] = 0.0;
            }
        }
    }
    let mut l = vec![vec![0.0; n]; n];
    let mut u = vec![vec![0.0; n]; n];
    let mut lp = vec![vec![false; n]; n];
    let mut up = vec![vec![false; n]; n];
    for i in 0..n {
        l[i][i] = 1.0;
        lp[i][i] = true;
        for j in 0..n {
            if i > j && a[i][j].abs() > 0.0 {
                l[i][j] = a[i][j];
                lp[i][j] = true;
            }
            if i <= j && a[i][j].abs() > 0.0 {
                u[i][j] = a[i][j];
                up[i][j] = true;
            }
        }
        if !up[i][i] {
            u[i][i] = a[i][i];
            up[i][i] = true;
        }
    }
    let mut out = HashMap::new();
    out.insert("l".into(), dense_to_csr_pattern(&l, &lp));
    out.insert("u".into(), dense_to_csr_pattern(&u, &up));
    out.insert("kind".into(), Value::String("ilut".into()));
    Ok(Value::Object(out))
}

/// sparse_ic_k(A, level) — incomplete Cholesky with level-of-fill k (0 = icc0).
fn sparse_ic_k(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a_in = args.first().ok_or("sparse_ic_k(A, level)")?;
    let level = num_at(args, 1, "sparse_ic_k")?.max(0.0) as i32;
    let csr = ensure_csr(a_in, env)?;
    let (_, n, n2, data, indices, indptr) = parse_sparse(&csr)?;
    if n != n2 {
        return Err("sparse_ic_k: square required".into());
    }
    // Level-of-fill pattern: start from A, allow fill when level(i,j) <= k.
    let mut lev = vec![vec![i32::MAX; n]; n];
    for i in 0..n {
        lev[i][i] = 0;
        let start = indptr[i] as usize;
        let end = indptr[i + 1] as usize;
        for k in start..end {
            let j = indices[k] as usize;
            lev[i][j] = 0;
            lev[j][i] = 0;
        }
    }
    if level > 0 {
        for _ in 0..level {
            let mut nxt = lev.clone();
            for i in 0..n {
                for k in 0..i {
                    if lev[i][k] > level {
                        continue;
                    }
                    for j in 0..=i {
                        if lev[k][j] > level {
                            continue;
                        }
                        let cand = lev[i][k] + lev[k][j] + 1;
                        if cand <= level && cand < nxt[i][j] {
                            nxt[i][j] = cand;
                            nxt[j][i] = cand;
                        }
                    }
                }
            }
            lev = nxt;
        }
    }
    let a = csr_to_dense(n, n, &data, &indices, &indptr);
    let mut l = vec![vec![0.0; n]; n];
    let mut lp = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..=i {
            if lev[i][j] > level {
                continue;
            }
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            if i == j {
                if s <= 0.0 {
                    return Err("sparse_ic_k: not SPD / zero pivot".into());
                }
                l[i][j] = s.sqrt();
            } else {
                if l[j][j].abs() < 1e-15 {
                    return Err("sparse_ic_k: zero pivot".into());
                }
                l[i][j] = s / l[j][j];
            }
            lp[i][j] = true;
        }
    }
    let mut out = HashMap::new();
    out.insert("l".into(), dense_to_csr_pattern(&l, &lp));
    out.insert("kind".into(), Value::String("ic_k".into()));
    out.insert("level".into(), Value::Number(level as i64));
    Ok(Value::Object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_sparse_from_coo", "sparse_from_coo"], sparse_from_coo);
    bind(&["science_sparse_from_csr", "sparse_from_csr"], sparse_from_csr);
    bind(&["science_sparse_to_csr", "sparse_to_csr"], sparse_to_csr);
    bind(&["science_sparse_from_csc", "sparse_from_csc"], sparse_from_csc);
    bind(&["science_sparse_to_csc", "sparse_to_csc"], sparse_to_csc);
    bind(&["science_sparse_ilu0", "sparse_ilu0"], sparse_ilu0);
    bind(&["science_sparse_icc0", "sparse_icc0"], sparse_icc0);
    bind(&["science_sparse_ilut", "sparse_ilut"], sparse_ilut);
    bind(&["science_sparse_ic_k", "sparse_ic_k"], sparse_ic_k);
    bind(&["science_sparse_spmv", "sparse_spmv"], sparse_spmv);
    bind(&["science_sparse_lstsq", "sparse_lstsq"], sparse_lstsq);
    bind(
        &["science_sparse_gather_rows", "sparse_gather_rows"],
        sparse_gather_rows,
    );
    bind(
        &["science_sparse_compress_rows", "sparse_compress_rows"],
        sparse_compress_rows,
    );
    bind(
        &["science_sparse_from_dense_mask", "sparse_from_dense_mask"],
        sparse_from_dense_mask,
    );
    bind(
        &["science_sparse_gather_cols", "sparse_gather_cols"],
        sparse_gather_cols,
    );
    bind(
        &["science_sparse_compress_cols", "sparse_compress_cols"],
        sparse_compress_cols,
    );
    bind(&["science_sparse_slice", "sparse_slice"], sparse_slice);
}
