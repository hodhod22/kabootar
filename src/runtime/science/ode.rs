//! ODE integrators (SC1g) — RK4 + odeint trajectory.

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

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_rk4", "num_rk4"], num_rk4);
    bind(&["science_num_odeint", "num_odeint"], num_odeint);
}
