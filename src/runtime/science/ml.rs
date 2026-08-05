//! ML / AI subset for `import "science"` (SC2 — activations, dense, SGD, Adam, metrics, batch).

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn ml_relu(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "ml_relu")?;
    Ok(vector_out(
        &x.iter().map(|v| v.max(0.0)).collect::<Vec<_>>(),
    ))
}

fn ml_sigmoid(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "ml_sigmoid")?;
    Ok(vector_out(
        &x.iter()
            .map(|v| 1.0 / (1.0 + (-v).exp()))
            .collect::<Vec<_>>(),
    ))
}

fn ml_softmax(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "ml_softmax")?;
    if x.is_empty() {
        return Err("ml_softmax: empty".into());
    }
    let max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ex: Vec<f64> = x.iter().map(|v| (v - max).exp()).collect();
    let sum: f64 = ex.iter().sum();
    if sum == 0.0 {
        return Err("ml_softmax: overflow".into());
    }
    Ok(vector_out(&ex.iter().map(|v| v / sum).collect::<Vec<_>>()))
}

fn ml_mse(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = vector_at(args, 0, "ml_mse")?;
    let pred = vector_at(args, 1, "ml_mse")?;
    if y.len() != pred.len() || y.is_empty() {
        return Err("ml_mse: length mismatch".into());
    }
    let s: f64 = y
        .iter()
        .zip(pred.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    Ok(float_out(s / y.len() as f64))
}

fn ml_dense(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = vector_at(args, 0, "ml_dense")?;
    let x = vector_at(args, 1, "ml_dense")?;
    let b = vector_at(args, 2, "ml_dense")?;
    let out_dim = b.len();
    if out_dim == 0 {
        return Err("ml_dense: empty bias".into());
    }
    if w.len() % out_dim != 0 {
        return Err("ml_dense: W length must be out*in".into());
    }
    let in_dim = w.len() / out_dim;
    if x.len() != in_dim {
        return Err("ml_dense: x length must match W columns".into());
    }
    let activate = match args.get(3) {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(s)) => s == "relu",
        None => false,
        _ => false,
    };
    let mut y = vec![0.0; out_dim];
    for o in 0..out_dim {
        let mut s = b[o];
        for i in 0..in_dim {
            s += w[o * in_dim + i] * x[i];
        }
        y[o] = if activate { s.max(0.0) } else { s };
    }
    Ok(vector_out(&y))
}

fn ml_sgd_update(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = vector_at(args, 0, "ml_sgd_update")?;
    let grad = vector_at(args, 1, "ml_sgd_update")?;
    let lr = num_at(args, 2, "ml_sgd_update")?;
    if w.len() != grad.len() {
        return Err("ml_sgd_update: length mismatch".into());
    }
    Ok(vector_out(
        &w.iter()
            .zip(grad.iter())
            .map(|(wi, gi)| wi - lr * gi)
            .collect::<Vec<_>>(),
    ))
}

fn ml_linreg_step(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let params = vector_at(args, 0, "ml_linreg_step")?;
    let x = vector_at(args, 1, "ml_linreg_step")?;
    let y = num_at(args, 2, "ml_linreg_step")?;
    let lr = num_at(args, 3, "ml_linreg_step")?;
    if params.len() != x.len() + 1 {
        return Err("ml_linreg_step: params = [w..., b]".into());
    }
    let n = x.len();
    let mut pred = params[n];
    for i in 0..n {
        pred += params[i] * x[i];
    }
    let err = pred - y;
    let mut out = params.clone();
    for i in 0..n {
        out[i] -= lr * err * x[i];
    }
    out[n] -= lr * err;
    Ok(vector_out(&out))
}

/// Adam: returns {w, m, v, t}.
fn ml_adam_update(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = vector_at(args, 0, "ml_adam_update")?;
    let grad = vector_at(args, 1, "ml_adam_update")?;
    let mut m = vector_at(args, 2, "ml_adam_update")?;
    let mut v = vector_at(args, 3, "ml_adam_update")?;
    let t = num_at(args, 4, "ml_adam_update")? as i64;
    let lr = args.get(5).and_then(|x| num(x).ok()).unwrap_or(0.001);
    let beta1 = args.get(6).and_then(|x| num(x).ok()).unwrap_or(0.9);
    let beta2 = args.get(7).and_then(|x| num(x).ok()).unwrap_or(0.999);
    let eps = args.get(8).and_then(|x| num(x).ok()).unwrap_or(1e-8);
    if w.len() != grad.len() || w.len() != m.len() || w.len() != v.len() {
        return Err("ml_adam_update: length mismatch".into());
    }
    let t_new = t + 1;
    let mut w_new = w.clone();
    for i in 0..w.len() {
        m[i] = beta1 * m[i] + (1.0 - beta1) * grad[i];
        v[i] = beta2 * v[i] + (1.0 - beta2) * grad[i] * grad[i];
        let mhat = m[i] / (1.0 - beta1.powi(t_new as i32));
        let vhat = v[i] / (1.0 - beta2.powi(t_new as i32));
        w_new[i] -= lr * mhat / (vhat.sqrt() + eps);
    }
    let mut out = HashMap::new();
    out.insert("w".into(), vector_out(&w_new));
    out.insert("m".into(), vector_out(&m));
    out.insert("v".into(), vector_out(&v));
    out.insert("t".into(), int_out(t_new));
    Ok(Value::Object(out))
}

fn class_ids(v: &Value) -> Result<Vec<i64>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n),
                Value::Float(f) => Ok(f.round() as i64),
                _ => Err("expected class id array".into()),
            })
            .collect(),
        _ => Err("expected class id array".into()),
    }
}

fn ml_accuracy(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = class_ids(args.first().ok_or("ml_accuracy(y, pred)")?)?;
    let p = class_ids(args.get(1).ok_or("ml_accuracy(y, pred)")?)?;
    if y.len() != p.len() || y.is_empty() {
        return Err("ml_accuracy: length mismatch".into());
    }
    let ok = y.iter().zip(p.iter()).filter(|(a, b)| a == b).count();
    Ok(float_out(ok as f64 / y.len() as f64))
}

fn ml_f1(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = class_ids(args.first().ok_or("ml_f1(y, pred)")?)?;
    let p = class_ids(args.get(1).ok_or("ml_f1(y, pred)")?)?;
    if y.len() != p.len() || y.is_empty() {
        return Err("ml_f1: length mismatch".into());
    }
    let mut tp = 0.0;
    let mut fp = 0.0;
    let mut fn_ = 0.0;
    for (yt, yp) in y.iter().zip(p.iter()) {
        if *yp == 1 && *yt == 1 {
            tp += 1.0;
        } else if *yp == 1 && *yt == 0 {
            fp += 1.0;
        } else if *yp == 0 && *yt == 1 {
            fn_ += 1.0;
        }
    }
    let prec = if tp + fp == 0.0 { 0.0 } else { tp / (tp + fp) };
    let rec = if tp + fn_ == 0.0 { 0.0 } else { tp / (tp + fn_) };
    let f1 = if prec + rec == 0.0 {
        0.0
    } else {
        2.0 * prec * rec / (prec + rec)
    };
    Ok(float_out(f1))
}

fn ml_confusion(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = class_ids(args.first().ok_or("ml_confusion")?)?;
    let p = class_ids(args.get(1).ok_or("ml_confusion")?)?;
    let n_classes = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or_else(|| {
            y.iter()
                .chain(p.iter())
                .copied()
                .max()
                .unwrap_or(0)
                .max(0) as f64
                + 1.0
        }) as usize;
    if y.len() != p.len() {
        return Err("ml_confusion: length mismatch".into());
    }
    let mut mat = vec![vec![0i64; n_classes]; n_classes];
    for (yt, yp) in y.iter().zip(p.iter()) {
        let r = (*yt).clamp(0, n_classes as i64 - 1) as usize;
        let c = (*yp).clamp(0, n_classes as i64 - 1) as usize;
        mat[r][c] += 1;
    }
    Ok(Value::Array(
        mat.into_iter()
            .map(|row| Value::Array(row.into_iter().map(int_out).collect()))
            .collect(),
    ))
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

fn ml_shuffle(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("ml_shuffle(items, seed?)".into()),
    };
    let mut seed = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(42.0) as u64;
    if seed == 0 {
        seed = 1;
    }
    let mut out = items;
    for i in (1..out.len()).rev() {
        let j = (lcg_next(&mut seed) as usize) % (i + 1);
        out.swap(i, j);
    }
    Ok(Value::Array(out))
}

fn ml_batch_slices(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "ml_batch_slices")? as usize;
    let batch = num_at(args, 1, "ml_batch_slices")? as usize;
    if batch == 0 {
        return Err("ml_batch_slices: batch_size > 0".into());
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < n {
        let end = (start + batch).min(n);
        out.push(Value::Array(vec![
            int_out(start as i64),
            int_out(end as i64),
        ]));
        start = end;
    }
    Ok(Value::Array(out))
}

fn ml_train_test_split(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("ml_train_test_split(x, y, test_ratio?, seed?)".into()),
    };
    let y = match args.get(1) {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("ml_train_test_split: y array".into()),
    };
    if x.len() != y.len() || x.is_empty() {
        return Err("ml_train_test_split: length mismatch".into());
    }
    let ratio = args.get(2).and_then(|v| num(v).ok()).unwrap_or(0.25);
    let seed = args.get(3).and_then(|v| num(v).ok()).unwrap_or(42.0);
    let n = x.len();
    let idx: Vec<Value> = (0..n).map(|i| int_out(i as i64)).collect();
    let shuffled = ml_shuffle(&[Value::Array(idx), float_out(seed)], _env)?;
    let Value::Array(order) = shuffled else {
        return Err("ml_train_test_split: internal".into());
    };
    let n_test = ((n as f64) * ratio).round() as usize;
    let n_test = n_test.clamp(1, n.saturating_sub(1));
    let mut x_train = Vec::new();
    let mut y_train = Vec::new();
    let mut x_test = Vec::new();
    let mut y_test = Vec::new();
    for (k, iv) in order.iter().enumerate() {
        let i = num(iv)? as usize;
        if k < n_test {
            x_test.push(x[i].clone());
            y_test.push(y[i].clone());
        } else {
            x_train.push(x[i].clone());
            y_train.push(y[i].clone());
        }
    }
    let mut out = HashMap::new();
    out.insert("x_train".into(), Value::Array(x_train));
    out.insert("y_train".into(), Value::Array(y_train));
    out.insert("x_test".into(), Value::Array(x_test));
    out.insert("y_test".into(), Value::Array(y_test));
    Ok(Value::Object(out))
}

/// AdamW: returns {w, m, v, t} with decoupled weight decay.
fn ml_adamw_update(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = vector_at(args, 0, "ml_adamw_update")?;
    let grad = vector_at(args, 1, "ml_adamw_update")?;
    let mut m = vector_at(args, 2, "ml_adamw_update")?;
    let mut v = vector_at(args, 3, "ml_adamw_update")?;
    let t = num_at(args, 4, "ml_adamw_update")? as i64;
    let lr = args.get(5).and_then(|x| num(x).ok()).unwrap_or(0.001);
    let beta1 = args.get(6).and_then(|x| num(x).ok()).unwrap_or(0.9);
    let beta2 = args.get(7).and_then(|x| num(x).ok()).unwrap_or(0.999);
    let eps = args.get(8).and_then(|x| num(x).ok()).unwrap_or(1e-8);
    let wd = args.get(9).and_then(|x| num(x).ok()).unwrap_or(0.01);
    if w.len() != grad.len() || w.len() != m.len() || w.len() != v.len() {
        return Err("ml_adamw_update: length mismatch".into());
    }
    let t_new = t + 1;
    let mut w_new = w.clone();
    for i in 0..w.len() {
        m[i] = beta1 * m[i] + (1.0 - beta1) * grad[i];
        v[i] = beta2 * v[i] + (1.0 - beta2) * grad[i] * grad[i];
        let mhat = m[i] / (1.0 - beta1.powi(t_new as i32));
        let vhat = v[i] / (1.0 - beta2.powi(t_new as i32));
        w_new[i] -= lr * mhat / (vhat.sqrt() + eps);
        w_new[i] -= lr * wd * w[i];
    }
    let mut out = HashMap::new();
    out.insert("w".into(), vector_out(&w_new));
    out.insert("m".into(), vector_out(&m));
    out.insert("v".into(), vector_out(&v));
    out.insert("t".into(), int_out(t_new));
    Ok(Value::Object(out))
}

/// Binary ROC-AUC (Mann–Whitney / trapezoid on sorted scores).
fn ml_roc_auc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = class_ids(args.first().ok_or("ml_roc_auc(y, scores)")?)?;
    let scores = vector_at(args, 1, "ml_roc_auc")?;
    if y.len() != scores.len() || y.is_empty() {
        return Err("ml_roc_auc: length mismatch".into());
    }
    let mut pos = 0usize;
    let mut neg = 0usize;
    for &yi in &y {
        if yi == 1 {
            pos += 1;
        } else {
            neg += 1;
        }
    }
    if pos == 0 || neg == 0 {
        return Err("ml_roc_auc: need both classes".into());
    }
    let mut pairs: Vec<(f64, i64)> = scores.iter().copied().zip(y.iter().copied()).collect();
    pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut tp = 0.0;
    let mut fp = 0.0;
    let mut prev_tpr = 0.0;
    let mut prev_fpr = 0.0;
    let mut auc = 0.0;
    let mut i = 0usize;
    while i < pairs.len() {
        let s = pairs[i].0;
        while i < pairs.len() && (pairs[i].0 - s).abs() < 1e-15 {
            if pairs[i].1 == 1 {
                tp += 1.0;
            } else {
                fp += 1.0;
            }
            i += 1;
        }
        let tpr = tp / pos as f64;
        let fpr = fp / neg as f64;
        auc += (fpr - prev_fpr) * (tpr + prev_tpr) / 2.0;
        prev_tpr = tpr;
        prev_fpr = fpr;
    }
    Ok(float_out(auc))
}

fn job_map(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("job_map(items, fn) expects array".into()),
    };
    let f = args
        .get(1)
        .cloned()
        .ok_or_else(|| "job_map(items, fn) expects function".to_string())?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let v = crate::bytecode::call_value(f.clone(), vec![item], &[], &[], &[], &[], env)?;
        out.push(v);
    }
    Ok(Value::Array(out))
}

fn apply_f64_op(x: f64, op: &str) -> Result<f64, String> {
    match op {
        "id" | "identity" => Ok(x),
        "neg" => Ok(-x),
        "abs" => Ok(x.abs()),
        "square" | "sq" => Ok(x * x),
        "double" => Ok(x * 2.0),
        "sqrt" => Ok(x.max(0.0).sqrt()),
        "relu" => Ok(x.max(0.0)),
        _ => Err(format!("job_map_parallel: unknown op '{op}'")),
    }
}

fn parallel_f64_map(xs: &[f64], op: &str, n_workers: usize) -> Result<Vec<f64>, String> {
    let _ = apply_f64_op(0.0, op)?; // validate op early
    let n = xs.len();
    if n == 0 {
        return Ok(vec![]);
    }
    let workers = n_workers.clamp(1, 32).min(n);
    if workers == 1 || n < 64 {
        return xs.iter().map(|x| apply_f64_op(*x, op)).collect();
    }
    let mut out = vec![0.0; n];
    let chunk = (n + workers - 1) / workers;
    let op = op.to_string();
    std::thread::scope(|scope| {
        let mut starts = Vec::new();
        for w in 0..workers {
            let start = w * chunk;
            if start >= n {
                break;
            }
            starts.push(start);
        }
        let mut rest = out.as_mut_slice();
        let mut offset = 0usize;
        for (idx, start) in starts.iter().enumerate() {
            let end = if idx + 1 < starts.len() {
                starts[idx + 1]
            } else {
                n
            };
            let len = end - start;
            let (dst, tail) = rest.split_at_mut(len);
            rest = tail;
            let src = &xs[*start..end];
            let op = op.clone();
            let _ = offset;
            offset = end;
            scope.spawn(move || {
                for (i, x) in src.iter().enumerate() {
                    dst[i] = apply_f64_op(*x, &op).unwrap_or(*x);
                }
            });
        }
    });
    Ok(out)
}

/// job_map_parallel(items, fn|op, nWorkers?)
/// - String op on number arrays → real OS-thread chunk map (SC4c)
/// - Function → sequential Kab fallback (Environment is !Send)
fn job_map_parallel(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err("job_map_parallel(items, fn|op) expects array".into()),
    };
    let n_workers = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get() as f64)
                .unwrap_or(4.0)
                .min(8.0)
        })
        .clamp(1.0, 32.0) as usize;

    match args.get(1) {
        Some(Value::String(op)) => {
            let xs: Vec<f64> = items.iter().map(num).collect::<Result<_, _>>()?;
            let out = parallel_f64_map(&xs, op, n_workers)?;
            Ok(vector_out(&out))
        }
        Some(f) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let v =
                    crate::bytecode::call_value(f.clone(), vec![item], &[], &[], &[], &[], env)?;
                out.push(v);
            }
            Ok(Value::Array(out))
        }
        None => Err("job_map_parallel: missing fn|op".into()),
    }
}

fn value_to_json(v: &Value) -> Result<serde_json::Value, String> {
    match v {
        Value::Null | Value::Undefined => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => Ok(serde_json::json!(*n)),
        Value::Float(f) => Ok(serde_json::json!(*f)),
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Array(items) => {
            let mut arr = Vec::new();
            for it in items {
                arr.push(value_to_json(it)?);
            }
            Ok(serde_json::Value::Array(arr))
        }
        Value::Object(m) => {
            let mut map = serde_json::Map::new();
            for (k, val) in m {
                map.insert(k.clone(), value_to_json(val)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        _ => Err("ml_save_checkpoint: unsupported value type".into()),
    }
}

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Float(0.0)
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => {
            Value::Array(items.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(m) => {
            let mut out = HashMap::new();
            for (k, val) in m {
                out.insert(k.clone(), json_to_value(val));
            }
            Value::Object(out)
        }
    }
}

/// ml_save_checkpoint(path, weightsObj) — JSON checkpoint.
fn ml_save_checkpoint(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ml_save_checkpoint(path, obj)".into()),
    };
    let obj = args.get(1).ok_or("ml_save_checkpoint: missing object")?;
    let mut root = serde_json::Map::new();
    root.insert("format".into(), serde_json::json!("kab-ml-v1"));
    root.insert("weights".into(), value_to_json(obj)?);
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| format!("ml_save_checkpoint: {e}"))?;
    std::fs::write(path, text).map_err(|e| format!("ml_save_checkpoint({path}): {e}"))?;
    Ok(Value::Bool(true))
}

fn ml_load_checkpoint(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("ml_load_checkpoint(path)".into()),
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("ml_load_checkpoint({path}): {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("ml_load_checkpoint: {e}"))?;
    let weights = v
        .get("weights")
        .cloned()
        .unwrap_or(v);
    Ok(json_to_value(&weights))
}

fn ml_cross_entropy(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pred = vector_at(args, 0, "ml_cross_entropy")?;
    let target = vector_at(args, 1, "ml_cross_entropy")?;
    if pred.len() != target.len() || pred.is_empty() {
        return Err("ml_cross_entropy: length mismatch".into());
    }
    let mut loss = 0.0;
    for (p, y) in pred.iter().zip(target.iter()) {
        loss -= y * p.max(1e-12).ln();
    }
    Ok(float_out(loss / pred.len() as f64))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ml_relu", "ml_relu"], ml_relu);
    bind(&["science_ml_sigmoid", "ml_sigmoid"], ml_sigmoid);
    bind(&["science_ml_softmax", "ml_softmax"], ml_softmax);
    bind(&["science_ml_mse", "ml_mse"], ml_mse);
    bind(&["science_ml_cross_entropy", "ml_cross_entropy"], ml_cross_entropy);
    bind(&["science_ml_dense", "ml_dense"], ml_dense);
    bind(&["science_ml_sgd_update", "ml_sgd_update"], ml_sgd_update);
    bind(&["science_ml_linreg_step", "ml_linreg_step"], ml_linreg_step);
    bind(&["science_ml_adam_update", "ml_adam_update"], ml_adam_update);
    bind(&["science_ml_adamw_update", "ml_adamw_update"], ml_adamw_update);
    bind(&["science_ml_accuracy", "ml_accuracy"], ml_accuracy);
    bind(&["science_ml_f1", "ml_f1"], ml_f1);
    bind(&["science_ml_roc_auc", "ml_roc_auc"], ml_roc_auc);
    bind(&["science_ml_confusion", "ml_confusion"], ml_confusion);
    bind(&["science_ml_shuffle", "ml_shuffle"], ml_shuffle);
    bind(&["science_ml_batch_slices", "ml_batch_slices"], ml_batch_slices);
    bind(
        &["science_ml_train_test_split", "ml_train_test_split"],
        ml_train_test_split,
    );
    bind(
        &["science_ml_save_checkpoint", "ml_save_checkpoint"],
        ml_save_checkpoint,
    );
    bind(
        &["science_ml_load_checkpoint", "ml_load_checkpoint"],
        ml_load_checkpoint,
    );
    bind(&["science_job_map", "job_map"], job_map);
    bind(&["science_job_map_parallel", "job_map_parallel"], job_map_parallel);
}
