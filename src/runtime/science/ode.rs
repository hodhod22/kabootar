//! ODE integrators (SC1g) — RK4 + odeint + adaptive + quad.

use super::helpers::{float_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn call_deriv(f: &Value, t: f64, y: &[f64], env: &mut Environment) -> Result<Vec<f64>, String> {
    let v = crate::bytecode::call_value(
        f.clone(),
        vec![float_out(t), vector_out(y)],
        &[],
        &[],
        &[],
        &[],
        env,
    )?;
    match v {
        Value::Array(items) => items.iter().map(num).collect(),
        other => Ok(vec![num(&other)?]),
    }
}

fn call_scalar(f: &Value, x: f64, env: &mut Environment) -> Result<f64, String> {
    let v = crate::bytecode::call_value(
        f.clone(),
        vec![float_out(x)],
        &[],
        &[],
        &[],
        &[],
        env,
    )?;
    num(&v)
}

fn rk4_step(f: &Value, t: f64, y: &[f64], dt: f64, env: &mut Environment) -> Result<Vec<f64>, String> {
    let n = y.len();
    let k1 = call_deriv(f, t, y, env)?;
    if k1.len() != n {
        return Err("num_rk4: deriv length mismatch".into());
    }
    let y2: Vec<f64> = y
        .iter()
        .zip(k1.iter())
        .map(|(yi, k)| yi + 0.5 * dt * k)
        .collect();
    let k2 = call_deriv(f, t + 0.5 * dt, &y2, env)?;
    let y3: Vec<f64> = y
        .iter()
        .zip(k2.iter())
        .map(|(yi, k)| yi + 0.5 * dt * k)
        .collect();
    let k3 = call_deriv(f, t + 0.5 * dt, &y3, env)?;
    let y4: Vec<f64> = y
        .iter()
        .zip(k3.iter())
        .map(|(yi, k)| yi + dt * k)
        .collect();
    let k4 = call_deriv(f, t + dt, &y4, env)?;
    let mut out = vec![0.0; n];
    for i in 0..n {
        out[i] = y[i] + (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    Ok(out)
}

/// Heun (improved Euler) — order 2; used for adaptive error estimate vs RK4.
fn heun_step(f: &Value, t: f64, y: &[f64], dt: f64, env: &mut Environment) -> Result<Vec<f64>, String> {
    let n = y.len();
    let k1 = call_deriv(f, t, y, env)?;
    if k1.len() != n {
        return Err("ode: deriv length mismatch".into());
    }
    let y_pred: Vec<f64> = y
        .iter()
        .zip(k1.iter())
        .map(|(yi, k)| yi + dt * k)
        .collect();
    let k2 = call_deriv(f, t + dt, &y_pred, env)?;
    let mut out = vec![0.0; n];
    for i in 0..n {
        out[i] = y[i] + 0.5 * dt * (k1[i] + k2[i]);
    }
    Ok(out)
}

/// num_rk4(f, y0, t0, dt) — one RK4 step. f(t, y) → dy/dt
fn num_rk4(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_rk4(f, y0, t0, dt)")?;
    let y0 = vector_at(args, 1, "num_rk4")?;
    let t0 = num_at(args, 2, "num_rk4")?;
    let dt = num_at(args, 3, "num_rk4")?;
    Ok(vector_out(&rk4_step(&f, t0, &y0, dt, env)?))
}

/// num_odeint(f, y0, t0, t1, n_steps?) → {t, y} trajectory (y rows)
fn num_odeint(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_odeint(f, y0, t0, t1, n?)")?;
    let mut y = vector_at(args, 1, "num_odeint")?;
    let t0 = num_at(args, 2, "num_odeint")?;
    let t1 = num_at(args, 3, "num_odeint")?;
    let n_steps = args
        .get(4)
        .and_then(|v| num(v).ok())
        .unwrap_or(50.0)
        .clamp(1.0, 100_000.0) as usize;
    let dt = (t1 - t0) / n_steps as f64;
    let mut ts = Vec::with_capacity(n_steps + 1);
    let mut ys = Vec::with_capacity(n_steps + 1);
    let mut t = t0;
    ts.push(float_out(t));
    ys.push(vector_out(&y));
    for _ in 0..n_steps {
        y = rk4_step(&f, t, &y, dt, env)?;
        t += dt;
        ts.push(float_out(t));
        ys.push(vector_out(&y));
    }
    let mut out = HashMap::new();
    out.insert("t".into(), Value::Array(ts));
    out.insert("y".into(), Value::Array(ys));
    Ok(Value::Object(out))
}

/// num_odeint_adaptive(f, y0, t0, t1, atol?, rtol?, max_steps?) → {t, y, n_steps, n_eval}
fn num_odeint_adaptive(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_odeint_adaptive")?;
    let mut y = vector_at(args, 1, "num_odeint_adaptive")?;
    let t0 = num_at(args, 2, "num_odeint_adaptive")?;
    let t1 = num_at(args, 3, "num_odeint_adaptive")?;
    let atol = args.get(4).and_then(|v| num(v).ok()).unwrap_or(1e-6).max(1e-15);
    let rtol = args.get(5).and_then(|v| num(v).ok()).unwrap_or(1e-6).max(1e-15);
    let max_steps = args
        .get(6)
        .and_then(|v| num(v).ok())
        .unwrap_or(10_000.0)
        .clamp(1.0, 1_000_000.0) as usize;

    let direction = if t1 >= t0 { 1.0 } else { -1.0 };
    let t_end = t1;
    let mut t = t0;
    let mut dt = (t1 - t0).abs().max(1e-12) * 0.01 * direction;
    let mut ts = vec![float_out(t)];
    let mut ys = vec![vector_out(&y)];
    let mut n_steps = 0usize;
    let mut n_eval = 0usize;

    while (t - t_end) * direction < 0.0 && n_steps < max_steps {
        if (t + dt - t_end) * direction > 0.0 {
            dt = t_end - t;
        }
        let y_rk = rk4_step(&f, t, &y, dt, env)?;
        n_eval += 4;
        let y_lo = heun_step(&f, t, &y, dt, env)?;
        n_eval += 2;
        let mut err: f64 = 0.0;
        for i in 0..y.len() {
            let scale = atol + rtol * y_rk[i].abs().max(y[i].abs());
            err = err.max(((y_rk[i] - y_lo[i]) / scale).abs());
        }
        if err <= 1.0 || dt.abs() < 1e-15 {
            t += dt;
            y = y_rk;
            ts.push(float_out(t));
            ys.push(vector_out(&y));
            n_steps += 1;
            let factor = if err < 1e-12 {
                2.0
            } else {
                (0.9 / err.sqrt()).clamp(0.2, 2.0)
            };
            dt *= factor;
        } else {
            let factor = (0.9 / err.sqrt()).clamp(0.1, 0.5);
            dt *= factor;
        }
    }

    let mut out = HashMap::new();
    out.insert("t".into(), Value::Array(ts));
    out.insert("y".into(), Value::Array(ys));
    out.insert("n_steps".into(), Value::Number(n_steps as i64));
    out.insert("n_eval".into(), Value::Number(n_eval as i64));
    Ok(Value::Object(out))
}

fn simpson_rule(fa: f64, fm: f64, fb: f64, h: f64) -> f64 {
    (h / 6.0) * (fa + 4.0 * fm + fb)
}

fn adaptive_simpson(
    f: &Value,
    a: f64,
    b: f64,
    fa: f64,
    fb: f64,
    tol: f64,
    depth: usize,
    env: &mut Environment,
) -> Result<(f64, usize), String> {
    let m = 0.5 * (a + b);
    let fm = call_scalar(f, m, env)?;
    let mut n_eval = 1usize;
    let whole = simpson_rule(fa, fm, fb, b - a);
    if depth == 0 {
        return Ok((whole, n_eval));
    }
    let lm = 0.5 * (a + m);
    let rm = 0.5 * (m + b);
    let flm = call_scalar(f, lm, env)?;
    let frm = call_scalar(f, rm, env)?;
    n_eval += 2;
    let left = simpson_rule(fa, flm, fm, m - a);
    let right = simpson_rule(fm, frm, fb, b - m);
    if (left + right - whole).abs() <= 15.0 * tol {
        return Ok((left + right + (left + right - whole) / 15.0, n_eval));
    }
    let (l, nl) = adaptive_simpson(f, a, m, fa, fm, tol / 2.0, depth - 1, env)?;
    let (r, nr) = adaptive_simpson(f, m, b, fm, fb, tol / 2.0, depth - 1, env)?;
    Ok((l + r, n_eval + nl + nr))
}

/// num_quad(f, a, b, tol?, max_depth?) → {value, n_eval}  (adaptive Simpson)
fn num_quad(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_quad(f, a, b)")?;
    let a = num_at(args, 1, "num_quad")?;
    let b = num_at(args, 2, "num_quad")?;
    let tol = args.get(3).and_then(|v| num(v).ok()).unwrap_or(1e-8).max(1e-15);
    let max_depth = args
        .get(4)
        .and_then(|v| num(v).ok())
        .unwrap_or(20.0)
        .clamp(1.0, 40.0) as usize;
    if (b - a).abs() < 1e-15 {
        let mut out = HashMap::new();
        out.insert("value".into(), float_out(0.0));
        out.insert("n_eval".into(), Value::Number(0));
        return Ok(Value::Object(out));
    }
    let fa = call_scalar(&f, a, env)?;
    let fb = call_scalar(&f, b, env)?;
    let (value, n_mid) = adaptive_simpson(&f, a, b, fa, fb, tol, max_depth, env)?;
    let mut out = HashMap::new();
    out.insert("value".into(), float_out(value));
    out.insert("n_eval".into(), Value::Number((n_mid + 2) as i64));
    Ok(Value::Object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_rk4", "num_rk4"], num_rk4);
    bind(&["science_num_odeint", "num_odeint"], num_odeint);
    bind(
        &["science_num_odeint_adaptive", "num_odeint_adaptive"],
        num_odeint_adaptive,
    );
    bind(&["science_num_quad", "num_quad"], num_quad);
}
