//! Interpolation — cubic spline1d (SC1h).

use super::helpers::{float_out, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};

/// Natural cubic spline coefficients for segment i: a + b*dx + c*dx^2 + d*dx^3
fn spline_coeffs(xs: &[f64], ys: &[f64]) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>), String> {
    let n = xs.len();
    if n < 2 {
        return Err("spline: need >= 2 points".into());
    }
    let mut h = vec![0.0; n - 1];
    for i in 0..n - 1 {
        h[i] = xs[i + 1] - xs[i];
        if h[i] <= 0.0 {
            return Err("spline: xs must be strictly increasing".into());
        }
    }
    let mut alpha = vec![0.0; n];
    for i in 1..n - 1 {
        alpha[i] =
            3.0 / h[i] * (ys[i + 1] - ys[i]) - 3.0 / h[i - 1] * (ys[i] - ys[i - 1]);
    }
    let mut l = vec![1.0; n];
    let mut mu = vec![0.0; n];
    let mut z = vec![0.0; n];
    for i in 1..n - 1 {
        l[i] = 2.0 * (xs[i + 1] - xs[i - 1]) - h[i - 1] * mu[i - 1];
        mu[i] = h[i] / l[i];
        z[i] = (alpha[i] - h[i - 1] * z[i - 1]) / l[i];
    }
    let mut c = vec![0.0; n];
    let mut b = vec![0.0; n - 1];
    let mut d = vec![0.0; n - 1];
    for j in (0..n - 1).rev() {
        c[j] = z[j] - mu[j] * c[j + 1];
        b[j] = (ys[j + 1] - ys[j]) / h[j] - h[j] * (c[j + 1] + 2.0 * c[j]) / 3.0;
        d[j] = (c[j + 1] - c[j]) / (3.0 * h[j]);
    }
    Ok((ys.to_vec(), b, c, d))
}

/// num_interp_spline(xs, ys, x) — evaluate natural cubic spline at x.
fn num_interp_spline(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let xs = vector_at(args, 0, "num_interp_spline")?;
    let ys = vector_at(args, 1, "num_interp_spline")?;
    let x = num_at(args, 2, "num_interp_spline")?;
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err("num_interp_spline: xs/ys length >= 2".into());
    }
    if x <= xs[0] {
        return Ok(float_out(ys[0]));
    }
    if x >= xs[xs.len() - 1] {
        return Ok(float_out(ys[ys.len() - 1]));
    }
    let (a, b, c, d) = spline_coeffs(&xs, &ys)?;
    let mut seg = 0;
    for i in 0..xs.len() - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            seg = i;
            break;
        }
    }
    let dx = x - xs[seg];
    let y = a[seg] + b[seg] * dx + c[seg] * dx * dx + d[seg] * dx * dx * dx;
    Ok(float_out(y))
}

/// num_interp_spline_vec(xs, ys, xs_new) — vector of interpolated points.
fn num_interp_spline_vec(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let xs = vector_at(args, 0, "num_interp_spline_vec")?;
    let ys = vector_at(args, 1, "num_interp_spline_vec")?;
    let xnew = vector_at(args, 2, "num_interp_spline_vec")?;
    let mut out = Vec::with_capacity(xnew.len());
    for &xv in &xnew {
        let v = num_interp_spline(
            &[vector_out(&xs), vector_out(&ys), float_out(xv)],
            env,
        )?;
        out.push(super::helpers::num(&v)?);
    }
    Ok(vector_out(&out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_interp_spline", "num_interp_spline"], num_interp_spline);
    bind(
        &["science_num_interp_spline_vec", "num_interp_spline_vec"],
        num_interp_spline_vec,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Environment;

    #[test]
    fn spline_mid() {
        let xs = vector_out(&[0.0, 1.0, 2.0]);
        let ys = vector_out(&[0.0, 1.0, 4.0]);
        let y = num_interp_spline(&[xs, ys, float_out(1.5)], &mut Environment::new()).unwrap();
        let v = super::super::helpers::num(&y).unwrap();
        assert!(v > 1.5, "spline={v}");
    }
}
