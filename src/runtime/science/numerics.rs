//! Numerical analysis for `import "science"`.

use super::helpers::{
    float_out, matrix_at, matrix_rows, num_at, require_square, vector_at, vector_out,
};
use crate::value::{Environment, Value};

fn num_lerp(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x0 = num_at(args, 0, "num_lerp")?;
    let y0 = num_at(args, 1, "num_lerp")?;
    let x1 = num_at(args, 2, "num_lerp")?;
    let y1 = num_at(args, 3, "num_lerp")?;
    let x = num_at(args, 4, "num_lerp")?;
    if (x1 - x0).abs() < 1e-15 {
        return Err("num_lerp: x0 and x1 must differ".into());
    }
    let t = (x - x0) / (x1 - x0);
    Ok(float_out(y0 + t * (y1 - y0)))
}

fn num_trapz(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() == 2 {
        let y = vector_at(args, 0, "num_trapz")?;
        let dx = num_at(args, 1, "num_trapz")?;
        if y.len() < 2 {
            return Err("num_trapz: need at least 2 points".into());
        }
        let sum: f64 = y.windows(2).map(|w| (w[0] + w[1]) / 2.0).sum();
        return Ok(float_out(sum * dx));
    }
    let xs = vector_at(args, 0, "num_trapz")?;
    let ys = vector_at(args, 1, "num_trapz")?;
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err("num_trapz: xs and ys must have equal length >= 2".into());
    }
    let mut area = 0.0;
    for i in 0..xs.len() - 1 {
        let dx = xs[i + 1] - xs[i];
        area += (ys[i] + ys[i + 1]) / 2.0 * dx;
    }
    Ok(float_out(area))
}

fn num_simpson(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let xs = vector_at(args, 0, "num_simpson")?;
    let ys = vector_at(args, 1, "num_simpson")?;
    if xs.len() != ys.len() || xs.len() < 3 {
        return Err("num_simpson: xs and ys must have equal length >= 3".into());
    }
    if xs.len() % 2 == 0 {
        return Err("num_simpson: need odd number of points".into());
    }
    let h = (xs[xs.len() - 1] - xs[0]) / (xs.len() as f64 - 1.0);
    for i in 1..xs.len() {
        if (xs[i] - xs[i - 1] - h).abs() > 1e-9 * h.abs().max(1.0) {
            return Err("num_simpson: xs must be evenly spaced".into());
        }
    }
    let mut sum = ys[0] + ys[ys.len() - 1];
    for i in 1..ys.len() - 1 {
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * ys[i];
    }
    Ok(float_out(sum * h / 3.0))
}

fn num_poly_eval(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let coeffs = vector_at(args, 0, "num_poly_eval")?;
    let x = num_at(args, 1, "num_poly_eval")?;
    let mut result = 0.0;
    let mut power = 1.0;
    for c in coeffs {
        result += c * power;
        power *= x;
    }
    Ok(float_out(result))
}

fn num_newton_step(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = num_at(args, 0, "num_newton_step")?;
    let fx = num_at(args, 1, "num_newton_step")?;
    let dfx = num_at(args, 2, "num_newton_step")?;
    if dfx.abs() < 1e-15 {
        return Err("num_newton_step: derivative near zero".into());
    }
    Ok(float_out(x - fx / dfx))
}

fn num_bisect_mid(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = num_at(args, 0, "num_bisect_mid")?;
    let b = num_at(args, 1, "num_bisect_mid")?;
    Ok(float_out((a + b) / 2.0))
}

fn num_diff_forward(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fx = num_at(args, 0, "num_diff_forward")?;
    let fxh = num_at(args, 1, "num_diff_forward")?;
    let h = num_at(args, 2, "num_diff_forward")?;
    if h.abs() < 1e-15 {
        return Err("num_diff_forward: h too small".into());
    }
    Ok(float_out((fxh - fx) / h))
}

fn num_diff_central(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f_minus = num_at(args, 0, "num_diff_central")?;
    let f_plus = num_at(args, 1, "num_diff_central")?;
    let h = num_at(args, 2, "num_diff_central")?;
    if h.abs() < 1e-15 {
        return Err("num_diff_central: h too small".into());
    }
    Ok(float_out((f_plus - f_minus) / (2.0 * h)))
}

fn gauss_solve(a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>, String> {
    let n = b.len();
    if matrix_rows(a) != n {
        return Err("num_solve: dimension mismatch".into());
    }
    let mut m = a.to_vec();
    let mut rhs = b.to_vec();
    for col in 0..n {
        let mut pivot = col;
        for row in col..n {
            if m[row][col].abs() > m[pivot][col].abs() {
                pivot = row;
            }
        }
        if m[pivot][col].abs() < 1e-12 {
            return Err("num_solve: singular matrix".into());
        }
        if pivot != col {
            m.swap(pivot, col);
            rhs.swap(pivot, col);
        }
        let div = m[col][col];
        for j in col..n {
            m[col][j] /= div;
        }
        rhs[col] /= div;
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = m[row][col];
            for j in col..n {
                m[row][j] -= factor * m[col][j];
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    Ok(rhs)
}

fn num_solve(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = matrix_at(args, 0, "num_solve")?;
    let b = vector_at(args, 1, "num_solve")?;
    require_square(&a, "num_solve")?;
    if matrix_rows(&a) != b.len() {
        return Err("num_solve: b length must match matrix size".into());
    }
    Ok(vector_out(&gauss_solve(&a, &b)?))
}

fn num_interp_linear(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let xs = vector_at(args, 0, "num_interp_linear")?;
    let ys = vector_at(args, 1, "num_interp_linear")?;
    let x = num_at(args, 2, "num_interp_linear")?;
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err("num_interp_linear: xs and ys must have equal length >= 2".into());
    }
    if x <= xs[0] {
        return Ok(float_out(ys[0]));
    }
    if x >= xs[xs.len() - 1] {
        return Ok(float_out(ys[ys.len() - 1]));
    }
    for i in 0..xs.len() - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            return num_lerp(
                &[
                    float_out(xs[i]),
                    float_out(ys[i]),
                    float_out(xs[i + 1]),
                    float_out(ys[i + 1]),
                    float_out(x),
                ],
                _env,
            );
        }
    }
    Err("num_interp_linear: x out of range".into())
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_lerp", "num_lerp"], num_lerp);
    bind(&["science_num_trapz", "num_trapz"], num_trapz);
    bind(&["science_num_simpson", "num_simpson"], num_simpson);
    bind(&["science_num_poly_eval", "num_poly_eval"], num_poly_eval);
    bind(&["science_num_newton_step", "num_newton_step"], num_newton_step);
    bind(&["science_num_bisect_mid", "num_bisect_mid"], num_bisect_mid);
    bind(&["science_num_diff_forward", "num_diff_forward"], num_diff_forward);
    bind(&["science_num_diff_central", "num_diff_central"], num_diff_central);
    bind(&["science_num_solve", "num_solve"], num_solve);
    bind(&["science_num_interp_linear", "num_interp_linear"], num_interp_linear);
}
