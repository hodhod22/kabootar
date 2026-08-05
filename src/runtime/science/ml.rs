//! ML / AI subset for `import "science"` (SC2 — activations, dense, SGD).

use super::helpers::{float_out, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};

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

/// y = relu(W @ x + b) if activate, else W @ x + b.
/// W flat row-major [out, in], x [in], b [out].
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

/// w := w - lr * grad  (elementwise).
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

/// One linear regression SGD step on MSE: pred = w·x + b.
/// Returns [w_new..., b_new] given flat params [w..., b], x, y_true, lr.
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

/// P8 subset: map `fn` over array items (sequential now; parallel later).
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

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ml_relu", "ml_relu"], ml_relu);
    bind(&["science_ml_sigmoid", "ml_sigmoid"], ml_sigmoid);
    bind(&["science_ml_softmax", "ml_softmax"], ml_softmax);
    bind(&["science_ml_mse", "ml_mse"], ml_mse);
    bind(&["science_ml_dense", "ml_dense"], ml_dense);
    bind(&["science_ml_sgd_update", "ml_sgd_update"], ml_sgd_update);
    bind(&["science_ml_linreg_step", "ml_linreg_step"], ml_linreg_step);
    bind(&["science_job_map", "job_map"], job_map);
}
