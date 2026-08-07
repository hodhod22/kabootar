//! Optimization: minimize / least_squares / root (SC1f).

use super::helpers::{float_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

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

fn call_vec(f: &Value, x: &[f64], env: &mut Environment) -> Result<f64, String> {
    let v = crate::bytecode::call_value(
        f.clone(),
        vec![vector_out(x)],
        &[],
        &[],
        &[],
        &[],
        env,
    )?;
    num(&v)
}

fn call_residuals(f: &Value, x: &[f64], env: &mut Environment) -> Result<Vec<f64>, String> {
    let v = crate::bytecode::call_value(
        f.clone(),
        vec![vector_out(x)],
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

/// num_root(f, a, b, tol?, max_iter?) — bisection. f(x) scalar.
fn num_root(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_root(f, a, b)")?;
    let mut a = num_at(args, 1, "num_root")?;
    let mut b = num_at(args, 2, "num_root")?;
    let tol = args.get(3).and_then(|v| num(v).ok()).unwrap_or(1e-8);
    let max_iter = args
        .get(4)
        .and_then(|v| num(v).ok())
        .unwrap_or(64.0) as usize;
    let mut fa = call_scalar(&f, a, env)?;
    let fb = call_scalar(&f, b, env)?;
    if fa * fb > 0.0 {
        return Err("num_root: f(a) and f(b) must have opposite signs".into());
    }
    let mut mid = a;
    for _ in 0..max_iter {
        mid = 0.5 * (a + b);
        if (b - a).abs() < tol {
            break;
        }
        let fm = call_scalar(&f, mid, env)?;
        if fm.abs() < tol {
            break;
        }
        if fa * fm <= 0.0 {
            b = mid;
        } else {
            a = mid;
            fa = fm;
        }
    }
    Ok(float_out(mid))
}

/// num_minimize(f, x0, max_iter?, step?) — coordinate + Nelder-Mead lite for vectors.
fn num_minimize(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_minimize(f, x0)")?;
    let x = vector_at(args, 1, "num_minimize")?;
    if x.is_empty() {
        return Err("num_minimize: empty x0".into());
    }
    let max_iter = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(200.0) as usize;
    let step = args.get(3).and_then(|v| num(v).ok()).unwrap_or(0.1);
    let n = x.len();
    // Nelder–Mead simplex
    let mut simplex: Vec<(Vec<f64>, f64)> = Vec::with_capacity(n + 1);
    let f0 = call_vec(&f, &x, env)?;
    simplex.push((x.clone(), f0));
    for i in 0..n {
        let mut xi = x.clone();
        xi[i] += if xi[i].abs() > 1e-6 {
            0.05 * xi[i].abs()
        } else {
            step
        };
        let fi = call_vec(&f, &xi, env)?;
        simplex.push((xi, fi));
    }
    for _ in 0..max_iter {
        simplex.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let best = simplex[0].1;
        let worst = simplex[n].1;
        if (worst - best).abs() < 1e-10 {
            break;
        }
        let mut centroid = vec![0.0; n];
        for item in simplex.iter().take(n) {
            for j in 0..n {
                centroid[j] += item.0[j];
            }
        }
        for c in &mut centroid {
            *c /= n as f64;
        }
        // reflect
        let mut xr = vec![0.0; n];
        for j in 0..n {
            xr[j] = centroid[j] + 1.0 * (centroid[j] - simplex[n].0[j]);
        }
        let fr = call_vec(&f, &xr, env)?;
        if fr < simplex[0].1 {
            // expand
            let mut xe = vec![0.0; n];
            for j in 0..n {
                xe[j] = centroid[j] + 2.0 * (xr[j] - centroid[j]);
            }
            let fe = call_vec(&f, &xe, env)?;
            simplex[n] = if fe < fr { (xe, fe) } else { (xr, fr) };
        } else if fr < simplex[n - 1].1 {
            simplex[n] = (xr, fr);
        } else {
            // contract
            let mut xc = vec![0.0; n];
            for j in 0..n {
                xc[j] = centroid[j] + 0.5 * (simplex[n].0[j] - centroid[j]);
            }
            let fc = call_vec(&f, &xc, env)?;
            if fc < simplex[n].1 {
                simplex[n] = (xc, fc);
            } else {
                // shrink
                let best_x = simplex[0].0.clone();
                for i in 1..=n {
                    for j in 0..n {
                        simplex[i].0[j] = best_x[j] + 0.5 * (simplex[i].0[j] - best_x[j]);
                    }
                    simplex[i].1 = call_vec(&f, &simplex[i].0, env)?;
                }
            }
        }
    }
    simplex.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = HashMap::new();
    out.insert("x".into(), vector_out(&simplex[0].0));
    out.insert("fun".into(), float_out(simplex[0].1));
    Ok(Value::from_object(out))
}

/// num_least_squares(residuals_fn, x0, max_iter?, eps?) — Gauss–Newton with FD Jacobian.
fn num_least_squares(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let f = args.first().cloned().ok_or("num_least_squares(f, x0)")?;
    let mut x = vector_at(args, 1, "num_least_squares")?;
    if x.is_empty() {
        return Err("num_least_squares: empty x0".into());
    }
    let max_iter = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(40.0) as usize;
    let eps = args.get(3).and_then(|v| num(v).ok()).unwrap_or(1e-6);
    let n = x.len();
    let mut cost = 0.0;
    for _ in 0..max_iter {
        let r = call_residuals(&f, &x, env)?;
        let m = r.len();
        cost = r.iter().map(|v| v * v).sum::<f64>() / 2.0;
        // Jacobian via forward differences
        let mut j = vec![vec![0.0; n]; m];
        for col in 0..n {
            let mut xp = x.clone();
            xp[col] += eps;
            let rp = call_residuals(&f, &xp, env)?;
            if rp.len() != m {
                return Err("num_least_squares: residual length changed".into());
            }
            for row in 0..m {
                j[row][col] = (rp[row] - r[row]) / eps;
            }
        }
        // Normal eqs: (JᵀJ) δ = Jᵀ r  (solve for -δ update)
        let mut jtj = vec![vec![0.0; n]; n];
        let mut jtr = vec![0.0; n];
        for i in 0..n {
            for k in 0..n {
                let mut s = 0.0;
                for row in 0..m {
                    s += j[row][i] * j[row][k];
                }
                jtj[i][k] = s;
            }
            let mut s = 0.0;
            for row in 0..m {
                s += j[row][i] * r[row];
            }
            jtr[i] = s;
        }
        // Gauss eliminate
        let mut aug = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            for k in 0..n {
                aug[i][k] = jtj[i][k];
            }
            aug[i][n] = jtr[i];
            aug[i][i] += 1e-8; // damp
        }
        for col in 0..n {
            let mut pivot = col;
            for rrow in col + 1..n {
                if aug[rrow][col].abs() > aug[pivot][col].abs() {
                    pivot = rrow;
                }
            }
            if aug[pivot][col].abs() < 1e-14 {
                continue;
            }
            aug.swap(col, pivot);
            let div = aug[col][col];
            for jcol in col..=n {
                aug[col][jcol] /= div;
            }
            for rrow in 0..n {
                if rrow == col {
                    continue;
                }
                let fac = aug[rrow][col];
                for jcol in col..=n {
                    aug[rrow][jcol] -= fac * aug[col][jcol];
                }
            }
        }
        let mut delta_norm = 0.0;
        for i in 0..n {
            let d = aug[i][n];
            x[i] -= d;
            delta_norm += d * d;
        }
        if delta_norm.sqrt() < 1e-10 {
            break;
        }
    }
    let mut out = HashMap::new();
    out.insert("x".into(), vector_out(&x));
    out.insert("cost".into(), float_out(cost));
    Ok(Value::from_object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_root", "num_root"], num_root);
    bind(&["science_num_minimize", "num_minimize"], num_minimize);
    bind(
        &["science_num_least_squares", "num_least_squares"],
        num_least_squares,
    );
}
