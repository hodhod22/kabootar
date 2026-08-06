//! Linear algebra: QR, SVD, Cholesky, lstsq, symmetric eig (SC1e).

use super::helpers::{float_out, matrix_at, matrix_out, num, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

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
        return Ok(Value::Object(out));
    }
    let mut out = HashMap::new();
    out.insert("q".into(), matrix_out(&q));
    out.insert("r".into(), matrix_out(&r));
    out.insert("mode".into(), Value::String("thin".into()));
    Ok(Value::Object(out))
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
    Ok(Value::Object(out))
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

/// SVD via AᵀA eig (compact). Returns {u, s, vt, mode}.
/// mode: "thin" (default) or "full" (pad U to m×m).
fn mat_svd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (m, n, data) = matrix_dims(args, 0, "mat_svd")?;
    let mode = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => "thin",
    };
    let a = mat_from_flat(m, n, &data);
    let at = transpose(&a);
    let ata = matmul_nn(&at, &a)?;
    let (evals, v) = jacobi_sym(&ata)?;
    let mut s = Vec::with_capacity(n);
    for e in &evals {
        s.push(e.max(0.0).sqrt());
    }
    // U = A V S^{+}
    let k = n.min(m);
    let mut u = vec![vec![0.0; k]; m];
    for j in 0..k {
        if s[j] < 1e-12 {
            continue;
        }
        for i in 0..m {
            let mut sum = 0.0;
            for t in 0..n {
                sum += a[i][t] * v[t][j];
            }
            u[i][j] = sum / s[j];
        }
    }
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
    let vt = transpose(&v);
    let mut out = HashMap::new();
    out.insert("u".into(), matrix_out(&u));
    out.insert("s".into(), vector_out(&s));
    out.insert("vt".into(), matrix_out(&vt));
    out.insert(
        "mode".into(),
        Value::String(if mode == "full" {
            "full".into()
        } else {
            "thin".into()
        }),
    );
    Ok(Value::Object(out))
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

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_mat_qr", "mat_qr"], mat_qr);
    bind(&["science_mat_qr_err", "mat_qr_err"], mat_qr_err);
    bind(&["science_mat_svd", "mat_svd"], mat_svd);
    bind(&["science_mat_pinv", "mat_pinv"], mat_pinv);
    bind(&["science_mat_cholesky", "mat_cholesky"], mat_cholesky);
    bind(&["science_mat_eig", "mat_eig"], mat_eig);
    bind(&["science_mat_lstsq", "mat_lstsq"], mat_lstsq);
    bind(&["science_mat_cond", "mat_cond"], mat_cond);
}
