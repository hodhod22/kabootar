//! Linear algebra: QR, SVD, Cholesky, lstsq, symmetric eig (SC1e).

use super::helpers::{float_out, matrix_at, matrix_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn rsvd_rng_next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = if x == 0 { 1 } else { x };
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

fn rsvd_randn(state: &mut u64) -> f64 {
    let u1 = ((rsvd_rng_next(state) >> 11) as f64 / ((1u64 << 53) as f64)).max(1e-12);
    let u2 = (rsvd_rng_next(state) >> 11) as f64 / ((1u64 << 53) as f64);
    let r = (-2.0 * u1.ln()).sqrt();
    r * (2.0 * std::f64::consts::PI * u2).cos()
}

fn mat_from_flat(m: usize, n: usize, data: &[f64]) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0; n]; m];
    for i in 0..m {
        for j in 0..n {
            a[i][j] = data[i * n + j];
        }
    }
    a
}

fn flat_from_mat(a: &[Vec<f64>]) -> Vec<f64> {
    a.iter().flat_map(|r| r.iter().copied()).collect()
}

fn matrix_dims(args: &[Value], i: usize, name: &str) -> Result<(usize, usize, Vec<f64>), String> {
    // Prefer ndarray {shape,data}; fall back to nested matrix.
    if let Some(Value::Object(m)) = args.get(i) {
        if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 2 => {
                    (num(&s[0])? as usize, num(&s[1])? as usize)
                }
                _ => return Err(format!("{name}: expect 2D ndarray")),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err(format!("{name}: nd missing data")),
            };
            return Ok((shape.0, shape.1, data));
        }
    }
    let mat = matrix_at(args, i, name)?;
    if mat.is_empty() {
        return Err(format!("{name}: empty matrix"));
    }
    let m = mat.len();
    let n = mat[0].len();
    Ok((m, n, flat_from_mat(&mat)))
}

/// Thin QR via modified Gram–Schmidt. Returns {q, r}.
fn qr_mgs(a: &[Vec<f64>]) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>), String> {
    let m = a.len();
    if m == 0 {
        return Err("mat_qr: empty".into());
    }
    let n = a[0].len();
    let mut v = a.to_vec();
    let mut q = vec![vec![0.0; n.min(m)]; m];
    let k = n.min(m);
    let mut r = vec![vec![0.0; n]; k];
    for j in 0..k {
        for i in 0..j {
            let mut dot = 0.0;
            for row in 0..m {
                dot += q[row][i] * v[row][j];
            }
            r[i][j] = dot;
            for row in 0..m {
                v[row][j] -= dot * q[row][i];
            }
        }
        let mut norm = 0.0;
        for row in 0..m {
            norm += v[row][j] * v[row][j];
        }
        norm = norm.sqrt();
        if norm < 1e-15 {
            return Err("mat_qr: linearly dependent columns".into());
        }
        r[j][j] = norm;
        for row in 0..m {
            q[row][j] = v[row][j] / norm;
        }
    }
    // Extra columns of R for n > m (full R width n)
    if n > k {
        // already sized R as k×n with zeros for j>=k unused; fill remaining via Q^T A
        for j in k..n {
            for i in 0..k {
                let mut dot = 0.0;
                for row in 0..m {
                    dot += q[row][i] * a[row][j];
                }
                r[i][j] = dot;
            }
        }
    }
    Ok((q, r))
}

fn mat_qr(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_qr")?;
    let a = mat_from_flat(m, n, &data);
    let mode = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => "thin",
    };
    let (mut q, r) = qr_mgs(&a)?;
    // "full": pad Q to m×m with orthonormal complement (Householder-lite via residual Gram-Schmidt).
    if mode == "full" && m > n.min(m) {
        let k = n.min(m);
        let mut q_full = vec![vec![0.0; m]; m];
        for i in 0..m {
            for j in 0..k {
                q_full[i][j] = q[i][j];
            }
        }
        let mut col = k;
        let mut basis = 0usize;
        while col < m && basis < m * 4 {
            let mut v = vec![0.0; m];
            v[basis % m] = 1.0;
            basis += 1;
            for j in 0..col {
                let mut dot = 0.0;
                for i in 0..m {
                    dot += q_full[i][j] * v[i];
                }
                for i in 0..m {
                    v[i] -= dot * q_full[i][j];
                }
            }
            let mut norm = 0.0;
            for i in 0..m {
                norm += v[i] * v[i];
            }
            norm = norm.sqrt();
            if norm < 1e-10 {
                continue;
            }
            for i in 0..m {
                q_full[i][col] = v[i] / norm;
            }
            col += 1;
        }
        q = q_full;
        let mut r_full = vec![vec![0.0; n]; m];
        for i in 0..r.len() {
            for j in 0..n {
                r_full[i][j] = r[i][j];
            }
        }
        let mut out = HashMap::new();
        out.insert("q".into(), matrix_out(&q));
        out.insert("r".into(), matrix_out(&r_full));
        out.insert("mode".into(), Value::String("full".into()));
        return Ok(Value::from_object(out));
    }
    let mut out = HashMap::new();
    out.insert("q".into(), matrix_out(&q));
    out.insert("r".into(), matrix_out(&r));
    out.insert("mode".into(), Value::String("thin".into()));
    Ok(Value::from_object(out))
}

/// mat_qr_err(a) → max |A − Q R| for thin QR.
fn mat_qr_err(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_qr_err")?;
    let a = mat_from_flat(m, n, &data);
    let qr = mat_qr(&[args[0].clone()], env)?;
    let Value::Object(map) = qr else {
        return Err("mat_qr_err: internal".into());
    };
    let q = match map.get("q") {
        Some(v) => matrix_at(&[v.clone()], 0, "mat_qr_err")?,
        _ => return Err("mat_qr_err: missing q".into()),
    };
    let r = match map.get("r") {
        Some(v) => matrix_at(&[v.clone()], 0, "mat_qr_err")?,
        _ => return Err("mat_qr_err: missing r".into()),
    };
    let qr_mat = matmul_nn(&q, &r)?;
    let mut err = 0.0_f64;
    for i in 0..m {
        for j in 0..n {
            err = err.max((a[i][j] - qr_mat[i][j]).abs());
        }
    }
    Ok(float_out(err))
}

fn mat_cholesky(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (n, n2, data) = matrix_dims(args, 0, "mat_cholesky")?;
    if n != n2 {
        return Err("mat_cholesky: square matrix required".into());
    }
    let a = mat_from_flat(n, n, &data);
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i][k] * l[j][k];
            }
            if i == j {
                let v = a[i][i] - sum;
                if v <= 0.0 {
                    return Err("mat_cholesky: not positive definite".into());
                }
                l[i][j] = v.sqrt();
            } else {
                if l[j][j].abs() < 1e-15 {
                    return Err("mat_cholesky: not positive definite".into());
                }
                l[i][j] = (a[i][j] - sum) / l[j][j];
            }
        }
    }
    Ok(matrix_out(&l))
}

/// Jacobi eigenvalue decomposition for symmetric matrices → {values, vectors}.
fn jacobi_sym(a_in: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<Vec<f64>>), String> {
    let n = a_in.len();
    let mut a = a_in.to_vec();
    let mut v = vec![vec![0.0; n]; n];
    for i in 0..n {
        v[i][i] = 1.0;
    }
    for _ in 0..(n * n * 30).max(50) {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut max = 0.0;
        for i in 0..n {
            for j in i + 1..n {
                let x = a[i][j].abs();
                if x > max {
                    max = x;
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-12 {
            break;
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        let theta = if (app - aqq).abs() < 1e-15 {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * (2.0 * apq).atan2(app - aqq)
        };
        let c = theta.cos();
        let s = theta.sin();
        for i in 0..n {
            let aip = a[i][p];
            let aiq = a[i][q];
            a[i][p] = c * aip + s * aiq;
            a[i][q] = -s * aip + c * aiq;
        }
        for i in 0..n {
            let api = a[p][i];
            let aqi = a[q][i];
            a[p][i] = c * api + s * aqi;
            a[q][i] = -s * api + c * aqi;
        }
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for i in 0..n {
            let vip = v[i][p];
            let viq = v[i][q];
            v[i][p] = c * vip + s * viq;
            v[i][q] = -s * vip + c * viq;
        }
    }
    let mut values: Vec<(f64, usize)> = (0..n).map(|i| (a[i][i], i)).collect();
    values.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut evals = Vec::with_capacity(n);
    let mut evecs = vec![vec![0.0; n]; n];
    for (col, (val, idx)) in values.into_iter().enumerate() {
        evals.push(val);
        for row in 0..n {
            evecs[row][col] = v[row][idx];
        }
    }
    Ok((evals, evecs))
}

fn mat_eig(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (n, n2, data) = matrix_dims(args, 0, "mat_eig")?;
    if n != n2 {
        return Err("mat_eig: square matrix required".into());
    }
    let a = mat_from_flat(n, n, &data);
    // Symmetrize for Jacobi path (document as symmetric eig).
    let mut sym = a.clone();
    for i in 0..n {
        for j in i + 1..n {
            let m = 0.5 * (a[i][j] + a[j][i]);
            sym[i][j] = m;
            sym[j][i] = m;
        }
    }
    let (vals, vecs) = jacobi_sym(&sym)?;
    let mut out = HashMap::new();
    out.insert("values".into(), vector_out(&vals));
    out.insert("vectors".into(), matrix_out(&vecs));
    Ok(Value::from_object(out))
}

fn matmul_nn(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let m = a.len();
    let k = a[0].len();
    let k2 = b.len();
    let n = b[0].len();
    if k != k2 {
        return Err("matmul: inner dims".into());
    }
    let mut out = vec![vec![0.0; n]; m];
    for i in 0..m {
        for j in 0..n {
            let mut s = 0.0;
            for t in 0..k {
                s += a[i][t] * b[t][j];
            }
            out[i][j] = s;
        }
    }
    Ok(out)
}

fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if a.is_empty() {
        return vec![];
    }
    let m = a.len();
    let n = a[0].len();
    let mut t = vec![vec![0.0; m]; n];
    for i in 0..m {
        for j in 0..n {
            t[j][i] = a[i][j];
        }
    }
    t
}

/// SVD via eig of AᵀA or AAᵀ. Returns {u, s, vt, mode}.
/// mode: "thin" (default) | "econ" (economy k=min(m,n)) | "full" (pad U to m×m).
fn mat_svd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_svd")?;
    let mode = match args.get(1) {
        Some(Value::String(s)) => match s.as_str() {
            "full" => "full",
            "econ" | "economy" => "econ",
            _ => "thin",
        },
        _ => "thin",
    };
    let a = mat_from_flat(m, n, &data);
    let k = n.min(m);
    let (mut u, s, vt) = if mode == "econ" && m < n {
        // Economy wide: eig of AAᵀ → U (m×m), S (m), VT (m×n).
        let at = transpose(&a);
        let aat = matmul_nn(&a, &at)?;
        let (evals, u_m) = jacobi_sym(&aat)?;
        let s_vec: Vec<f64> = evals.iter().map(|e| e.max(0.0).sqrt()).collect();
        let mut vt_m = vec![vec![0.0; n]; k];
        for j in 0..k {
            if s_vec[j] < 1e-12 {
                continue;
            }
            for col in 0..n {
                let mut sum = 0.0;
                for row in 0..m {
                    sum += u_m[row][j] * a[row][col];
                }
                vt_m[j][col] = sum / s_vec[j];
            }
        }
        let u_out: Vec<Vec<f64>> = (0..m)
            .map(|i| (0..k).map(|j| u_m[i][j]).collect())
            .collect();
        (u_out, s_vec[..k].to_vec(), vt_m)
    } else {
        // Tall/square (or thin/full): eig of AᵀA.
        let at = transpose(&a);
        let ata = matmul_nn(&at, &a)?;
        let (evals, v) = jacobi_sym(&ata)?;
        let mut s_vec: Vec<f64> = evals.iter().map(|e| e.max(0.0).sqrt()).collect();
        let mut u_m = vec![vec![0.0; k]; m];
        for j in 0..k {
            if s_vec[j] < 1e-12 {
                continue;
            }
            for i in 0..m {
                let mut sum = 0.0;
                for t in 0..n {
                    sum += a[i][t] * v[t][j];
                }
                u_m[i][j] = sum / s_vec[j];
            }
        }
        let mut vt_m = transpose(&v);
        if mode == "econ" {
            s_vec.truncate(k);
            vt_m.truncate(k);
        }
        (u_m, s_vec, vt_m)
    };

    if mode == "full" && m > k {
        let mut u_full = vec![vec![0.0; m]; m];
        for i in 0..m {
            for j in 0..k {
                u_full[i][j] = u[i][j];
            }
        }
        let mut col = k;
        let mut basis = 0usize;
        while col < m && basis < m * 4 {
            let mut vecv = vec![0.0; m];
            vecv[basis % m] = 1.0;
            basis += 1;
            for j in 0..col {
                let mut dot = 0.0;
                for i in 0..m {
                    dot += u_full[i][j] * vecv[i];
                }
                for i in 0..m {
                    vecv[i] -= dot * u_full[i][j];
                }
            }
            let mut norm = 0.0;
            for i in 0..m {
                norm += vecv[i] * vecv[i];
            }
            norm = norm.sqrt();
            if norm < 1e-10 {
                continue;
            }
            for i in 0..m {
                u_full[i][col] = vecv[i] / norm;
            }
            col += 1;
        }
        u = u_full;
    }
    let mut out = HashMap::new();
    out.insert("u".into(), matrix_out(&u));
    out.insert("s".into(), vector_out(&s));
    out.insert("vt".into(), matrix_out(&vt));
    out.insert("mode".into(), Value::String(mode.into()));
    Ok(Value::from_object(out))
}

/// mat_randomized_svd(A, rank, nOver?, seed?) — Halko et al. randomized SVD.
fn mat_randomized_svd(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_randomized_svd")?;
    let rank = num_at(args, 1, "mat_randomized_svd")? as usize;
    let n_over = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(5.0)
        .max(0.0) as usize;
    let seed = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(42.0) as u64;
    if rank == 0 || rank > m.min(n) {
        return Err("mat_randomized_svd: bad rank".into());
    }
    let l = (rank + n_over).min(n).min(m);
    let a = mat_from_flat(m, n, &data);
    let mut state = if seed == 0 { 1 } else { seed };
    // Omega: n x l
    let mut omega = vec![vec![0.0; l]; n];
    for i in 0..n {
        for j in 0..l {
            omega[i][j] = rsvd_randn(&mut state);
        }
    }
    let y = matmul_nn(&a, &omega)?; // m x l
    let y_v = matrix_out(&y);
    let qr = mat_qr(&[y_v], env)?;
    let Value::Object(qrm) = qr else {
        return Err("mat_randomized_svd: qr".into());
    };
    let q = matrix_at(&[qrm.get("q").cloned().ok_or("mat_randomized_svd: q")?], 0, "mat_randomized_svd")?;
    let qt = transpose(&q);
    let b = matmul_nn(&qt, &a)?; // l x n
    let b_v = matrix_out(&b);
    let svd = mat_svd(&[b_v, Value::String("econ".into())], env)?;
    let Value::Object(sm) = svd else {
        return Err("mat_randomized_svd: svd".into());
    };
    let uhat = matrix_at(&[sm.get("u").cloned().ok_or("u")?], 0, "mat_randomized_svd")?;
    let s_full = vector_at(&[sm.get("s").cloned().ok_or("s")?], 0, "mat_randomized_svd")?;
    let vt_full = matrix_at(&[sm.get("vt").cloned().ok_or("vt")?], 0, "mat_randomized_svd")?;
    let u = matmul_nn(&q, &uhat)?;
    let k = rank.min(s_full.len()).min(u[0].len()).min(vt_full.len());
    let u_k: Vec<Vec<f64>> = u
        .iter()
        .map(|row| row.iter().take(k).copied().collect())
        .collect();
    let s_k = s_full[..k].to_vec();
    let vt_k: Vec<Vec<f64>> = vt_full.iter().take(k).cloned().collect();
    let mut out = HashMap::new();
    out.insert("u".into(), matrix_out(&u_k));
    out.insert("s".into(), vector_out(&s_k));
    out.insert("vt".into(), matrix_out(&vt_k));
    out.insert("mode".into(), Value::String("rand".into()));
    out.insert("rank".into(), Value::Number(k as i64));
    Ok(Value::from_object(out))
}

/// mat_streaming_svd(A, rank, blockRows?, nOver?, seed?) — truncated SVD via row-block sketch.
fn mat_streaming_svd(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_streaming_svd")?;
    let rank = num_at(args, 1, "mat_streaming_svd")? as usize;
    let block = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(32.0)
        .max(1.0) as usize;
    let n_over = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(5.0)
        .max(0.0) as usize;
    let seed = args
        .get(4)
        .and_then(|v| num(v).ok())
        .unwrap_or(42.0) as u64;
    if rank == 0 || rank > m.min(n) {
        return Err("mat_streaming_svd: bad rank".into());
    }
    let l = (rank + n_over).min(n).min(m);
    let mut state = if seed == 0 { 1 } else { seed };
    let mut omega = vec![vec![0.0; l]; n];
    for i in 0..n {
        for j in 0..l {
            omega[i][j] = rsvd_randn(&mut state);
        }
    }
    // Stream rows: Y (m x l) = A @ Omega accumulated by blocks.
    let mut y = vec![vec![0.0; l]; m];
    let mut row0 = 0usize;
    while row0 < m {
        let row1 = (row0 + block).min(m);
        for i in row0..row1 {
            for j in 0..l {
                let mut s = 0.0;
                for t in 0..n {
                    s += data[i * n + t] * omega[t][j];
                }
                y[i][j] = s;
            }
        }
        row0 = row1;
    }
    let y_v = matrix_out(&y);
    let qr = mat_qr(&[y_v], env)?;
    let Value::Object(qrm) = qr else {
        return Err("mat_streaming_svd: qr".into());
    };
    let q = matrix_at(
        &[qrm.get("q").cloned().ok_or("mat_streaming_svd: q")?],
        0,
        "mat_streaming_svd",
    )?;
    let qt = transpose(&q);
    let a = mat_from_flat(m, n, &data);
    let b = matmul_nn(&qt, &a)?;
    let svd = mat_svd(&[matrix_out(&b), Value::String("econ".into())], env)?;
    let Value::Object(sm) = svd else {
        return Err("mat_streaming_svd: svd".into());
    };
    let uhat = matrix_at(&[sm.get("u").cloned().ok_or("u")?], 0, "mat_streaming_svd")?;
    let s_full = vector_at(&[sm.get("s").cloned().ok_or("s")?], 0, "mat_streaming_svd")?;
    let vt_full = matrix_at(&[sm.get("vt").cloned().ok_or("vt")?], 0, "mat_streaming_svd")?;
    let u = matmul_nn(&q, &uhat)?;
    let k = rank.min(s_full.len()).min(u[0].len()).min(vt_full.len());
    let u_k: Vec<Vec<f64>> = u
        .iter()
        .map(|row| row.iter().take(k).copied().collect())
        .collect();
    let mut out = HashMap::new();
    out.insert("u".into(), matrix_out(&u_k));
    out.insert("s".into(), vector_out(&s_full[..k]));
    out.insert(
        "vt".into(),
        matrix_out(&vt_full.iter().take(k).cloned().collect::<Vec<_>>()),
    );
    out.insert("mode".into(), Value::String("stream".into()));
    out.insert("rank".into(), Value::Number(k as i64));
    out.insert("blockRows".into(), Value::Number(block as i64));
    Ok(Value::from_object(out))
}

/// Moore–Penrose pseudoinverse via thin SVD.
fn mat_pinv(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (m, n, _) = matrix_dims(args, 0, "mat_pinv")?;
    let svd = mat_svd(args, env)?;
    let Value::Object(map) = svd else {
        return Err("mat_pinv: internal".into());
    };
    let u = match map.get("u") {
        Some(v) => matrix_at(&[v.clone()], 0, "mat_pinv")?,
        _ => return Err("mat_pinv: missing u".into()),
    };
    let s = match map.get("s") {
        Some(v) => vector_at(&[v.clone()], 0, "mat_pinv")?,
        _ => return Err("mat_pinv: missing s".into()),
    };
    let vt = match map.get("vt") {
        Some(v) => matrix_at(&[v.clone()], 0, "mat_pinv")?,
        _ => return Err("mat_pinv: missing vt".into()),
    };
    let k = s.len().min(u.first().map(|r| r.len()).unwrap_or(0)).min(vt.len());
    // A+ = V S+ U^T  (vt is V^T → V = vt^T)
    let mut sp = vec![0.0; k];
    for i in 0..k {
        if s[i] > 1e-12 {
            sp[i] = 1.0 / s[i];
        }
    }
    // tmp = S⁺ Uᵀ → k×m
    let mut tmp = vec![vec![0.0; m]; k];
    for i in 0..k {
        for j in 0..m {
            tmp[i][j] = sp[i] * u[j][i];
        }
    }
    // pinv = V * tmp → n×m
    let v = transpose(&vt);
    let mut pinv = vec![vec![0.0; m]; n];
    for i in 0..n {
        for j in 0..m {
            let mut sum = 0.0;
            for t in 0..k {
                sum += v[i][t] * tmp[t][j];
            }
            pinv[i][j] = sum;
        }
    }
    Ok(matrix_out(&pinv))
}

fn mat_lstsq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (m, n, adata) = matrix_dims(args, 0, "mat_lstsq")?;
    let b = vector_at(args, 1, "mat_lstsq")?;
    if b.len() != m {
        return Err("mat_lstsq: b length must match rows".into());
    }
    let a = mat_from_flat(m, n, &adata);
    let at = transpose(&a);
    let ata = matmul_nn(&at, &a)?;
    let mut atb = vec![0.0; n];
    for i in 0..n {
        for r in 0..m {
            atb[i] += at[i][r] * b[r];
        }
    }
    // Solve ATA x = ATb via Gauss
    let mut aug = vec![vec![0.0; n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = ata[i][j];
        }
        aug[i][n] = atb[i];
    }
    for col in 0..n {
        let mut pivot = col;
        for r in col + 1..n {
            if aug[r][col].abs() > aug[pivot][col].abs() {
                pivot = r;
            }
        }
        if aug[pivot][col].abs() < 1e-12 {
            return Err("mat_lstsq: singular normal matrix".into());
        }
        aug.swap(col, pivot);
        let div = aug[col][col];
        for j in col..=n {
            aug[col][j] /= div;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let f = aug[r][col];
            for j in col..=n {
                aug[r][j] -= f * aug[col][j];
            }
        }
    }
    let x: Vec<f64> = (0..n).map(|i| aug[i][n]).collect();
    Ok(vector_out(&x))
}

/// Doolittle LU with partial pivoting. Returns {l, u, piv, sign}.
fn mat_lu(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (n, n2, data) = matrix_dims(args, 0, "mat_lu")?;
    if n != n2 {
        return Err("mat_lu: square matrix required".into());
    }
    let mut a = mat_from_flat(n, n, &data);
    let mut piv: Vec<i64> = (0..n as i64).collect();
    let mut sign = 1.0_f64;
    for k in 0..n {
        let mut pivot = k;
        for i in k + 1..n {
            if a[i][k].abs() > a[pivot][k].abs() {
                pivot = i;
            }
        }
        if a[pivot][k].abs() < 1e-15 {
            return Err("mat_lu: singular".into());
        }
        if pivot != k {
            a.swap(pivot, k);
            piv.swap(pivot, k);
            sign = -sign;
        }
        for i in k + 1..n {
            a[i][k] /= a[k][k];
            for j in k + 1..n {
                a[i][j] -= a[i][k] * a[k][j];
            }
        }
    }
    let mut l = vec![vec![0.0; n]; n];
    let mut u = vec![vec![0.0; n]; n];
    for i in 0..n {
        l[i][i] = 1.0;
        for j in 0..n {
            if i > j {
                l[i][j] = a[i][j];
            } else {
                u[i][j] = a[i][j];
            }
        }
    }
    let mut out = HashMap::new();
    out.insert("l".into(), matrix_out(&l));
    out.insert("u".into(), matrix_out(&u));
    out.insert(
        "piv".into(), Value::from_array(piv.iter().map(|p| Value::Number(*p)).collect()),
    );
    out.insert("sign".into(), float_out(sign));
    Ok(Value::from_object(out))
}

/// mat_slogdet(a) -> {sign, logabsdet}
fn mat_slogdet(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let lu = mat_lu(args, env)?;
    let Value::Object(m) = lu else {
        return Err("mat_slogdet: internal".into());
    };
    let u = match m.get("u") {
        Some(v) => matrix_at(&[v.clone()], 0, "mat_slogdet")?,
        _ => return Err("mat_slogdet: missing u".into()),
    };
    let sign = match m.get("sign") {
        Some(v) => num(v)?,
        _ => 1.0,
    };
    let mut logabs = 0.0_f64;
    let mut s = sign;
    for i in 0..u.len() {
        let d = u[i][i];
        if d == 0.0 {
            let mut out = HashMap::new();
            out.insert("sign".into(), float_out(0.0));
            out.insert("logabsdet".into(), float_out(f64::NEG_INFINITY));
            return Ok(Value::from_object(out));
        }
        if d < 0.0 {
            s = -s;
        }
        logabs += d.abs().ln();
    }
    let mut out = HashMap::new();
    out.insert("sign".into(), float_out(s));
    out.insert("logabsdet".into(), float_out(logabs));
    Ok(Value::from_object(out))
}

/// mat_norm_ord(a, ord) — matrix norms: "fro"|"1"|"inf" (default fro).
fn mat_norm_ord(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_norm_ord")?;
    let a = mat_from_flat(m, n, &data);
    let ord = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        Some(Value::Null) | Some(Value::Undefined) | None => "fro",
        _ => "fro",
    };
    let v = match ord {
        "1" => (0..n)
            .map(|j| (0..m).map(|i| a[i][j].abs()).sum::<f64>())
            .fold(0.0_f64, f64::max),
        "inf" => a
            .iter()
            .map(|row| row.iter().map(|x| x.abs()).sum::<f64>())
            .fold(0.0_f64, f64::max),
        _ => data.iter().map(|x| x * x).sum::<f64>().sqrt(),
    };
    Ok(float_out(v))
}

fn mat_cond(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let svd = mat_svd(args, _env)?;
    let Value::Object(m) = svd else {
        return Err("mat_cond: internal".into());
    };
    let s = match m.get("s") {
        Some(v) => super::helpers::vector_val(v)?,
        _ => return Err("mat_cond: missing s".into()),
    };
    let smax = s.iter().copied().fold(0.0_f64, f64::max);
    let smin = s
        .iter()
        .copied()
        .filter(|x| *x > 1e-15)
        .fold(f64::INFINITY, f64::min);
    if !smin.is_finite() || smin == 0.0 {
        return Ok(float_out(f64::INFINITY));
    }
    Ok(float_out(smax / smin))
}

fn batch_matrices(args: &[Value], name: &str) -> Result<Vec<Value>, String> {
    match args.first() {
        Some(Value::Array(items)) if !items.is_empty() => Ok(items.as_ref().clone()),
        _ => Err(format!("{name}(batchMatrices, ...)")),
    }
}

/// mat_batch_qr(batch, mode?) -> { q: [...], r: [...], mode, n }
fn mat_batch_qr(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let batch = batch_matrices(args, "mat_batch_qr")?;
    let n_batch = batch.len() as i64;
    let mode = args.get(1).cloned().unwrap_or(Value::Undefined);
    let mut qs = Vec::new();
    let mut rs = Vec::new();
    let mut mode_s = "thin".to_string();
    for m in batch {
        let qr = if matches!(mode, Value::Undefined | Value::Null) {
            mat_qr(&[m], env)?
        } else {
            mat_qr(&[m, mode.clone()], env)?
        };
        let Value::Object(map) = qr else {
            return Err("mat_batch_qr: internal".into());
        };
        qs.push(map.get("q").cloned().ok_or("mat_batch_qr: q")?);
        rs.push(map.get("r").cloned().ok_or("mat_batch_qr: r")?);
        if let Some(Value::String(s)) = map.get("mode") {
            mode_s = s.clone();
        }
    }
    let mut out = HashMap::new();
    out.insert("q".into(), Value::from_array(qs));
    out.insert("r".into(), Value::from_array(rs));
    out.insert("mode".into(), Value::String(mode_s));
    out.insert("n".into(), Value::Number(n_batch));
    Ok(Value::from_object(out))
}

/// mat_batch_svd(batch, mode?) -> { u, s, vt, mode, n }
fn mat_batch_svd(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let batch = batch_matrices(args, "mat_batch_svd")?;
    let mode = args.get(1).cloned().unwrap_or(Value::Undefined);
    let n_batch = batch.len() as i64;
    let mut us = Vec::new();
    let mut ss = Vec::new();
    let mut vts = Vec::new();
    let mut mode_s = "thin".to_string();
    for m in batch {
        let svd = if matches!(mode, Value::Undefined | Value::Null) {
            mat_svd(&[m], env)?
        } else {
            mat_svd(&[m, mode.clone()], env)?
        };
        let Value::Object(map) = svd else {
            return Err("mat_batch_svd: internal".into());
        };
        us.push(map.get("u").cloned().ok_or("mat_batch_svd: u")?);
        ss.push(map.get("s").cloned().ok_or("mat_batch_svd: s")?);
        vts.push(map.get("vt").cloned().ok_or("mat_batch_svd: vt")?);
        if let Some(Value::String(s)) = map.get("mode") {
            mode_s = s.clone();
        }
    }
    let mut out = HashMap::new();
    out.insert("u".into(), Value::from_array(us));
    out.insert("s".into(), Value::from_array(ss));
    out.insert("vt".into(), Value::from_array(vts));
    out.insert("mode".into(), Value::String(mode_s));
    out.insert("n".into(), Value::Number(n_batch));
    Ok(Value::from_object(out))
}

/// mat_batch_eig(batch) -> { values: [...], vectors: [...], n }
fn mat_batch_eig(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let batch = batch_matrices(args, "mat_batch_eig")?;
    let n_batch = batch.len() as i64;
    let mut values = Vec::new();
    let mut vectors = Vec::new();
    for m in batch {
        let eig = mat_eig(&[m], env)?;
        let Value::Object(map) = eig else {
            return Err("mat_batch_eig: internal".into());
        };
        values.push(map.get("values").cloned().ok_or("mat_batch_eig: values")?);
        vectors.push(map.get("vectors").cloned().ok_or("mat_batch_eig: vectors")?);
    }
    let mut out = HashMap::new();
    out.insert("values".into(), Value::from_array(values));
    out.insert("vectors".into(), Value::from_array(vectors));
    out.insert("n".into(), Value::Number(n_batch));
    Ok(Value::from_object(out))
}

/// mat_batch_solve(batchA, batchB) — per-item Ax=b via Gauss.
fn mat_batch_solve(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let batch_a = batch_matrices(args, "mat_batch_solve")?;
    let batch_b = match args.get(1) {
        Some(Value::Array(items)) => items.clone(),
        _ => return Err("mat_batch_solve(batchA, batchB)".into()),
    };
    if batch_a.len() != batch_b.len() {
        return Err("mat_batch_solve: batch length mismatch".into());
    }
    let mut xs = Vec::new();
    for (a_v, b_v) in batch_a.iter().zip(batch_b.iter()) {
        let (n, n2, adata) = matrix_dims(&[a_v.clone()], 0, "mat_batch_solve")?;
        if n != n2 {
            return Err("mat_batch_solve: square A required".into());
        }
        let b = vector_at(std::slice::from_ref(b_v), 0, "mat_batch_solve")?;
        if b.len() != n {
            return Err("mat_batch_solve: b length".into());
        }
        let mut aug = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = adata[i * n + j];
            }
            aug[i][n] = b[i];
        }
        for col in 0..n {
            let mut pivot = col;
            for r in col + 1..n {
                if aug[r][col].abs() > aug[pivot][col].abs() {
                    pivot = r;
                }
            }
            if aug[pivot][col].abs() < 1e-12 {
                return Err("mat_batch_solve: singular".into());
            }
            aug.swap(col, pivot);
            let div = aug[col][col];
            for j in col..=n {
                aug[col][j] /= div;
            }
            for r in 0..n {
                if r == col {
                    continue;
                }
                let f = aug[r][col];
                for j in col..=n {
                    aug[r][j] -= f * aug[col][j];
                }
            }
        }
        let x: Vec<f64> = (0..n).map(|i| aug[i][n]).collect();
        xs.push(vector_out(&x));
    }
    Ok(Value::from_array(xs))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_mat_qr", "mat_qr"], mat_qr);
    bind(&["science_mat_qr_err", "mat_qr_err"], mat_qr_err);
    bind(&["science_mat_svd", "mat_svd"], mat_svd);
    bind(
        &["science_mat_randomized_svd", "mat_randomized_svd"],
        mat_randomized_svd,
    );
    bind(
        &["science_mat_streaming_svd", "mat_streaming_svd"],
        mat_streaming_svd,
    );
    bind(&["science_mat_pinv", "mat_pinv"], mat_pinv);
    bind(&["science_mat_cholesky", "mat_cholesky"], mat_cholesky);
    bind(&["science_mat_eig", "mat_eig"], mat_eig);
    bind(&["science_mat_lstsq", "mat_lstsq"], mat_lstsq);
    bind(&["science_mat_cond", "mat_cond"], mat_cond);
    bind(&["science_mat_lu", "mat_lu"], mat_lu);
    bind(&["science_mat_slogdet", "mat_slogdet"], mat_slogdet);
    bind(&["science_mat_norm_ord", "mat_norm_ord"], mat_norm_ord);
    bind(&["science_mat_batch_qr", "mat_batch_qr"], mat_batch_qr);
    bind(&["science_mat_batch_svd", "mat_batch_svd"], mat_batch_svd);
    bind(&["science_mat_batch_eig", "mat_batch_eig"], mat_batch_eig);
    bind(
        &["science_mat_batch_solve", "mat_batch_solve"],
        mat_batch_solve,
    );
}
