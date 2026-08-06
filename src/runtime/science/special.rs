//! Special functions — erf, gamma, bessel subset (SC1h).

use super::helpers::{float_out, num_at};
use crate::value::{Environment, Value};

fn erf_impl(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
  let y = 1.0 - (((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t) * (-x * x).exp();
    sign * y
}

/// num_erf(x)
fn num_erf(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(erf_impl(num_at(args, 0, "num_erf")?)))
}

/// num_erfc(x) = 1 - erf(x)
fn num_erfc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(1.0 - erf_impl(num_at(args, 0, "num_erfc")?)))
}

/// Lanczos gamma for positive x; reflection for x <= 0 not supported fully.
fn gamma_impl(x: f64) -> f64 {
    if x < 0.5 {
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma_impl(1.0 - x));
    }
    let x = x - 1.0;
    let p = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343178686095,
        -0.13857109526572012,
        9.984369578019571e-6,
        1.5056327351493116e-7,
    ];
    let t = x + 7.0;
    let mut sum = p[0];
    for i in 1..p.len() {
        sum += p[i] / (x + i as f64);
    }
    let two_pi_sqrt = (2.0 * std::f64::consts::PI).sqrt();
    two_pi_sqrt * t.powf(x + 0.5) * (-t).exp() * sum
}

/// num_gamma(x)
fn num_gamma(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = num_at(args, 0, "num_gamma")?;
    if x <= 0.0 && x.fract() == 0.0 {
        return Err("num_gamma: pole at non-positive integer".into());
    }
    Ok(float_out(gamma_impl(x)))
}

/// Bessel J0(x) — series for small |x|, asymptotic for large.
fn bessel_j0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 8.0 {
        let y = x * x;
        let ans1 = 57568490574.0
            + y * (-13362590354.0 + y * (651619640.7 + y * (-11214424.0 + y * (77392.33017 + y * -184.9052454))));
        let ans2 = 57568490411.0
            + y * (1029532985.0 + y * (9494680.718 + y * (59272.64853 + y * (267.8532712 + y * 1.0))));
        ans1 / ans2
    } else {
        let z = 8.0 / ax;
        let y = z * z;
        let xx = ax - 0.785398164;
        let ans1 = 1.0
            + y * (-0.1098628627e-2 + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6)));
        let ans2 = -0.1562499995e-1
            + y * (0.1430488765e-3 + y * (-0.6911147651e-5 + y * (0.7621095161e-6 - y * 0.934935152e-7)));
        (ans1 * xx.cos() - z * ans2 * xx.sin()) / ax.sqrt()
    }
}

/// num_bessel_j0(x)
fn num_bessel_j0(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(bessel_j0(num_at(args, 0, "num_bessel_j0")?)))
}

/// Bessel J1 via recurrence / series (Abramowitz-style).
fn bessel_j1(x: f64) -> f64 {
    let ax = x.abs();
    let y = if ax < 8.0 {
        let y = x * x;
        let ans1 = x
            * (72362614232.0
                + y * (-7895059235.0
                    + y * (242396853.1 + y * (-2972611.439 + y * (15704.48260 + y * -30.16036606)))));
        let ans2 = 144725228442.0
            + y * (2300535178.0 + y * (18583304.74 + y * (99447.43394 + y * (376.9991397 + y * 1.0))));
        ans1 / ans2
    } else {
        let z = 8.0 / ax;
        let y = z * z;
        let xx = ax - 2.356194491;
        let ans1 = 1.0
            + y * (0.183105e-2 + y * (-0.3516396496e-4 + y * (0.2457520174e-5 + y * -0.240337019e-6)));
        let ans2 = 0.04687499995
            + y * (-0.2002690873e-3 + y * (0.8449199096e-5 + y * (-0.88228987e-6 + y * 0.105787412e-6)));
        let out = (ans1 * xx.cos() - z * ans2 * xx.sin()) / ax.sqrt();
        out
    };
    if x < 0.0 {
        -y
    } else {
        y
    }
}

fn num_bessel_j1(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(bessel_j1(num_at(args, 0, "num_bessel_j1")?)))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_num_erf", "num_erf"], num_erf);
    bind(&["science_num_erfc", "num_erfc"], num_erfc);
    bind(&["science_num_gamma", "num_gamma"], num_gamma);
    bind(&["science_num_bessel_j0", "num_bessel_j0"], num_bessel_j0);
    bind(&["science_num_bessel_j1", "num_bessel_j1"], num_bessel_j1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Environment;

    #[test]
    fn gamma_five_reasonable() {
        let g = num_gamma(&[float_out(5.0)], &mut Environment::new()).unwrap();
        let v = super::super::helpers::num(&g).unwrap();
        assert!(v > 4.0, "gamma(5)={v}");
    }
}
