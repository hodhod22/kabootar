//! Classical ML: PCA, k-means, logistic regression (SC2h).

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn matrix_rows(v: &Value) -> Result<Vec<Vec<f64>>, String> {
    match v {
        Value::Array(rows) => {
            let mut out = Vec::new();
            let mut ncols = None;
            for row in rows {
                let r = match row {
                    Value::Array(cells) => cells.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                    _ => vector_at(&[row.clone()], 0, "matrix")?,
                };
                if let Some(n) = ncols {
                    if r.len() != n {
                        return Err("ml: jagged matrix".into());
                    }
                } else {
                    ncols = Some(r.len());
                }
                out.push(r);
            }
            Ok(out)
        }
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 2 => {
                    (num(&s[0])? as usize, num(&s[1])? as usize)
                }
                _ => return Err("ml: expect 2D ndarray".into()),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err("ml: nd missing data".into()),
            };
            let (r, c) = shape;
            let mut out = vec![vec![0.0; c]; r];
            for i in 0..r {
                for j in 0..c {
                    out[i][j] = data[i * c + j];
                }
            }
            Ok(out)
        }
        _ => Err("ml: expected matrix / 2D array".into()),
    }
}

fn jacobi_sym(a_in: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a_in.len();
    let mut a = a_in.to_vec();
    let mut v = vec![vec![0.0; n]; n];
    for i in 0..n {
        v[i][i] = 1.0;
    }
    for _ in 0..(n * n * 40).max(80) {
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
    let mut pairs: Vec<(f64, usize)> = (0..n).map(|i| (a[i][i], i)).collect();
    pairs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut evals = Vec::new();
    let mut evecs = vec![vec![0.0; n]; n];
    for (col, (val, idx)) in pairs.into_iter().enumerate() {
        evals.push(val);
        for row in 0..n {
            evecs[row][col] = v[row][idx];
        }
    }
    (evals, evecs)
}

/// ml_pca(X, n_components?) → {components, explained, mean, transform}
fn ml_pca(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = matrix_rows(args.first().ok_or("ml_pca(X, k?)")?)?;
    if x.is_empty() || x[0].is_empty() {
        return Err("ml_pca: empty".into());
    }
    let n = x.len();
    let d = x[0].len();
    let k = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(d.min(2) as f64)
        .clamp(1.0, d as f64) as usize;
    let mut mean = vec![0.0; d];
    for row in &x {
        for j in 0..d {
            mean[j] += row[j];
        }
    }
    for m in &mut mean {
        *m /= n as f64;
    }
    let mut cov = vec![vec![0.0; d]; d];
    for row in &x {
        for i in 0..d {
            for j in 0..d {
                cov[i][j] += (row[i] - mean[i]) * (row[j] - mean[j]);
            }
        }
    }
    let scale = 1.0 / (n.saturating_sub(1).max(1) as f64);
    for i in 0..d {
        for j in 0..d {
            cov[i][j] *= scale;
        }
    }
    let (evals, evecs) = jacobi_sym(&cov);
    let total: f64 = evals.iter().map(|e| e.max(0.0)).sum::<f64>().max(1e-12);
    let mut components = Vec::new();
    let mut explained = Vec::new();
    for c in 0..k {
        explained.push(evals[c].max(0.0) / total);
        let mut col = Vec::with_capacity(d);
        for r in 0..d {
            col.push(evecs[r][c]);
        }
        components.push(Value::Array(col.into_iter().map(float_out).collect()));
    }
    let mut transformed = Vec::new();
    for row in &x {
        let mut proj = Vec::with_capacity(k);
        for c in 0..k {
            let mut s = 0.0;
            for j in 0..d {
                s += (row[j] - mean[j]) * evecs[j][c];
            }
            proj.push(float_out(s));
        }
        transformed.push(Value::Array(proj));
    }
    let mut out = HashMap::new();
    out.insert("components".into(), Value::Array(components));
    out.insert(
        "explained".into(),
        Value::Array(explained.into_iter().map(float_out).collect()),
    );
    out.insert(
        "mean".into(),
        Value::Array(mean.into_iter().map(float_out).collect()),
    );
    out.insert("transform".into(), Value::Array(transformed));
    Ok(Value::Object(out))
}

fn lcg(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

/// ml_kmeans(X, k, max_iter?, seed?) → {centroids, labels, inertia}
fn ml_kmeans(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = matrix_rows(args.first().ok_or("ml_kmeans(X, k)")?)?;
    let k = num_at(args, 1, "ml_kmeans")? as usize;
    if x.is_empty() || k == 0 || k > x.len() {
        return Err("ml_kmeans: bad k".into());
    }
    let d = x[0].len();
    let max_iter = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(50.0) as usize;
    let mut seed = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(42.0) as u64;
    if seed == 0 {
        seed = 1;
    }
    // Init: pick k random points
    let mut centroids = Vec::with_capacity(k);
    let mut used = std::collections::HashSet::new();
    while centroids.len() < k {
        let i = (lcg(&mut seed) * x.len() as f64) as usize % x.len();
        if used.insert(i) {
            centroids.push(x[i].clone());
        }
    }
    let mut labels = vec![0usize; x.len()];
    for _ in 0..max_iter {
        let mut changed = false;
        for (i, row) in x.iter().enumerate() {
            let mut best = 0usize;
            let mut best_d = f64::INFINITY;
            for (c, cen) in centroids.iter().enumerate() {
                let dist: f64 = row
                    .iter()
                    .zip(cen.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                if dist < best_d {
                    best_d = dist;
                    best = c;
                }
            }
            if labels[i] != best {
                labels[i] = best;
                changed = true;
            }
        }
        let mut sums = vec![vec![0.0; d]; k];
        let mut counts = vec![0usize; k];
        for (i, row) in x.iter().enumerate() {
            let c = labels[i];
            counts[c] += 1;
            for j in 0..d {
                sums[c][j] += row[j];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..d {
                    centroids[c][j] = sums[c][j] / counts[c] as f64;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut inertia = 0.0;
    for (i, row) in x.iter().enumerate() {
        let c = &centroids[labels[i]];
        inertia += row
            .iter()
            .zip(c.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>();
    }
    let mut out = HashMap::new();
    out.insert(
        "centroids".into(),
        Value::Array(
            centroids
                .into_iter()
                .map(|r| Value::Array(r.into_iter().map(float_out).collect()))
                .collect(),
        ),
    );
    out.insert(
        "labels".into(),
        Value::Array(labels.into_iter().map(|l| int_out(l as i64)).collect()),
    );
    out.insert("inertia".into(), float_out(inertia));
    Ok(Value::Object(out))
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// ml_logreg_fit(X, y, lr?, epochs?) → {w, b}
fn ml_logreg_fit(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = matrix_rows(args.first().ok_or("ml_logreg_fit(X, y)")?)?;
    let y = vector_at(args, 1, "ml_logreg_fit")?;
    if x.len() != y.len() || x.is_empty() {
        return Err("ml_logreg_fit: length mismatch".into());
    }
    let d = x[0].len();
    let lr = args.get(2).and_then(|v| num(v).ok()).unwrap_or(0.1);
    let epochs = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(200.0) as usize;
    let mut w = vec![0.0; d];
    let mut b = 0.0;
    let n = x.len() as f64;
    for _ in 0..epochs {
        let mut gw = vec![0.0; d];
        let mut gb = 0.0;
        for i in 0..x.len() {
            let mut z = b;
            for j in 0..d {
                z += w[j] * x[i][j];
            }
            let p = sigmoid(z);
            let err = p - y[i];
            gb += err;
            for j in 0..d {
                gw[j] += err * x[i][j];
            }
        }
        b -= lr * gb / n;
        for j in 0..d {
            w[j] -= lr * gw[j] / n;
        }
    }
    let mut out = HashMap::new();
    out.insert("w".into(), vector_out(&w));
    out.insert("b".into(), float_out(b));
    Ok(Value::Object(out))
}

/// ml_logreg_predict(model, X) → probs or labels if threshold given
fn ml_logreg_predict(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let model = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("ml_logreg_predict(model, X)".into()),
    };
    let w = match model.get("w") {
        Some(v) => vector_at(&[v.clone()], 0, "w")?,
        _ => return Err("model missing w".into()),
    };
    let b = match model.get("b") {
        Some(v) => num(v)?,
        _ => 0.0,
    };
    let x = matrix_rows(args.get(1).ok_or("ml_logreg_predict: X")?)?;
    let threshold = args.get(2).and_then(|v| num(v).ok());
    let mut out = Vec::new();
    for row in x {
        if row.len() != w.len() {
            return Err("ml_logreg_predict: dim mismatch".into());
        }
        let mut z = b;
        for j in 0..w.len() {
            z += w[j] * row[j];
        }
        let p = sigmoid(z);
        if let Some(t) = threshold {
            out.push(if p >= t {
                int_out(1)
            } else {
                int_out(0)
            });
        } else {
            out.push(float_out(p));
        }
    }
    Ok(Value::Array(out))
}

fn class_ids_f(y: &[f64]) -> Vec<i64> {
    y.iter().map(|v| v.round() as i64).collect()
}

fn gini(counts: &HashMap<i64, usize>, n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let mut impurity = 1.0;
    for c in counts.values() {
        let p = *c as f64 / n as f64;
        impurity -= p * p;
    }
    impurity
}

fn count_labels(y: &[i64], idx: &[usize]) -> HashMap<i64, usize> {
    let mut m = HashMap::new();
    for &i in idx {
        *m.entry(y[i]).or_insert(0) += 1;
    }
    m
}

fn majority(counts: &HashMap<i64, usize>) -> i64 {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(k, _)| *k)
        .unwrap_or(0)
}

fn best_split(x: &[Vec<f64>], y: &[i64], idx: &[usize]) -> Option<(usize, f64, f64)> {
    let d = x[0].len();
    let parent = count_labels(y, idx);
    let parent_gini = gini(&parent, idx.len());
    let mut best: Option<(usize, f64, f64)> = None;
    for feat in 0..d {
        let mut vals: Vec<f64> = idx.iter().map(|&i| x[i][feat]).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        for w in vals.windows(2) {
            let thr = 0.5 * (w[0] + w[1]);
            let mut left = Vec::new();
            let mut right = Vec::new();
            for &i in idx {
                if x[i][feat] <= thr {
                    left.push(i);
                } else {
                    right.push(i);
                }
            }
            if left.is_empty() || right.is_empty() {
                continue;
            }
            let lg = gini(&count_labels(y, &left), left.len());
            let rg = gini(&count_labels(y, &right), right.len());
            let n = idx.len() as f64;
            let gain = parent_gini
                - (left.len() as f64 / n) * lg
                - (right.len() as f64 / n) * rg;
            if best.as_ref().map(|b| gain > b.2).unwrap_or(true) {
                best = Some((feat, thr, gain));
            }
        }
    }
    best.filter(|b| b.2 > 1e-12)
}

fn leaf_obj(label: i64) -> Value {
    let mut m = HashMap::new();
    m.insert("leaf".into(), Value::Bool(true));
    m.insert("label".into(), int_out(label));
    Value::Object(m)
}

fn build_tree(x: &[Vec<f64>], y: &[i64], idx: &[usize], depth: usize, max_depth: usize) -> Value {
    let counts = count_labels(y, idx);
    if depth >= max_depth || idx.len() <= 1 || counts.len() <= 1 {
        return leaf_obj(majority(&counts));
    }
    let Some((feat, thr, _)) = best_split(x, y, idx) else {
        return leaf_obj(majority(&counts));
    };
    let mut left_idx = Vec::new();
    let mut right_idx = Vec::new();
    for &i in idx {
        if x[i][feat] <= thr {
            left_idx.push(i);
        } else {
            right_idx.push(i);
        }
    }
    let mut m = HashMap::new();
    m.insert("leaf".into(), Value::Bool(false));
    m.insert("feature".into(), int_out(feat as i64));
    m.insert("threshold".into(), float_out(thr));
    m.insert(
        "left".into(),
        build_tree(x, y, &left_idx, depth + 1, max_depth),
    );
    m.insert(
        "right".into(),
        build_tree(x, y, &right_idx, depth + 1, max_depth),
    );
    Value::Object(m)
}

fn predict_one(node: &Value, row: &[f64]) -> Result<i64, String> {
    let Value::Object(m) = node else {
        return Err("tree node".into());
    };
    if matches!(m.get("leaf"), Some(Value::Bool(true))) {
        return match m.get("label") {
            Some(v) => Ok(num(v)? as i64),
            _ => Err("leaf label".into()),
        };
    }
    let feat = num(m.get("feature").ok_or("feature")?)? as usize;
    let thr = num(m.get("threshold").ok_or("threshold")?)?;
    if feat >= row.len() {
        return Err("feature OOB".into());
    }
    if row[feat] <= thr {
        predict_one(m.get("left").ok_or("left")?, row)
    } else {
        predict_one(m.get("right").ok_or("right")?, row)
    }
}

/// ml_stump_fit(X, y) → {feature, threshold, left, right} (depth-1 tree)
fn ml_stump_fit(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = matrix_rows(args.first().ok_or("ml_stump_fit(X, y)")?)?;
    let yf = vector_at(args, 1, "ml_stump_fit")?;
    if x.len() != yf.len() || x.is_empty() {
        return Err("ml_stump_fit: length mismatch".into());
    }
    let y = class_ids_f(&yf);
    let idx: Vec<usize> = (0..x.len()).collect();
    let tree = build_tree(&x, &y, &idx, 0, 1);
    Ok(tree)
}

/// ml_tree_fit(X, y, maxDepth?) → decision tree object
fn ml_tree_fit(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = matrix_rows(args.first().ok_or("ml_tree_fit(X, y)")?)?;
    let yf = vector_at(args, 1, "ml_tree_fit")?;
    if x.len() != yf.len() || x.is_empty() {
        return Err("ml_tree_fit: length mismatch".into());
    }
    let max_depth = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(3.0)
        .max(1.0) as usize;
    let y = class_ids_f(&yf);
    let idx: Vec<usize> = (0..x.len()).collect();
    Ok(build_tree(&x, &y, &idx, 0, max_depth))
}

/// ml_tree_predict(tree, X) → class ids
fn ml_tree_predict(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tree = args.first().ok_or("ml_tree_predict(tree, X)")?;
    let x = matrix_rows(args.get(1).ok_or("ml_tree_predict: X")?)?;
    let mut out = Vec::with_capacity(x.len());
    for row in &x {
        out.push(int_out(predict_one(tree, row)?));
    }
    Ok(Value::Array(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ml_pca", "ml_pca"], ml_pca);
    bind(&["science_ml_kmeans", "ml_kmeans"], ml_kmeans);
    bind(&["science_ml_logreg_fit", "ml_logreg_fit"], ml_logreg_fit);
    bind(
        &["science_ml_logreg_predict", "ml_logreg_predict"],
        ml_logreg_predict,
    );
    bind(&["science_ml_stump_fit", "ml_stump_fit"], ml_stump_fit);
    bind(&["science_ml_tree_fit", "ml_tree_fit"], ml_tree_fit);
    bind(&["science_ml_tree_predict", "ml_tree_predict"], ml_tree_predict);
}
