//! Descriptive statistics for `import "science"`.

use super::helpers::{float_out, int_out, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};

fn sorted_copy(data: &[f64]) -> Vec<f64> {
    let mut v = data.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v
}

fn stat_mean(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_mean")?;
    if data.is_empty() {
        return Err("stat_mean: empty data".into());
    }
    Ok(float_out(data.iter().sum::<f64>() / data.len() as f64))
}

fn stat_sum(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_sum")?;
    Ok(float_out(data.iter().sum()))
}

fn stat_count(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_count")?;
    Ok(int_out(data.len() as i64))
}

fn stat_min(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_min")?;
    data.iter()
        .copied()
        .reduce(f64::min)
        .map(float_out)
        .ok_or_else(|| "stat_min: empty data".into())
}

fn stat_max(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_max")?;
    data.iter()
        .copied()
        .reduce(f64::max)
        .map(float_out)
        .ok_or_else(|| "stat_max: empty data".into())
}

fn stat_median(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_median")?;
    if data.is_empty() {
        return Err("stat_median: empty data".into());
    }
    let s = sorted_copy(&data);
    let n = s.len();
    let med = if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    };
    Ok(float_out(med))
}

fn stat_var(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_var")?;
    if data.len() < 2 {
        return Err("stat_var: need at least 2 values".into());
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let var = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
    Ok(float_out(var))
}

fn stat_std(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = stat_var(args, _env)?;
    match v {
        Value::Float(f) => Ok(float_out(f.sqrt())),
        _ => unreachable!(),
    }
}

fn stat_sample_var(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_sample_var")?;
    if data.len() < 2 {
        return Err("stat_sample_var: need at least 2 values".into());
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let var =
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() as f64 - 1.0);
    Ok(float_out(var))
}

fn stat_sample_std(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = stat_sample_var(args, _env)?;
    match v {
        Value::Float(f) => Ok(float_out(f.sqrt())),
        _ => unreachable!(),
    }
}

fn stat_percentile(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "stat_percentile")?;
    let p = num_at(args, 1, "stat_percentile")?;
    if data.is_empty() {
        return Err("stat_percentile: empty data".into());
    }
    if !(0.0..=100.0).contains(&p) {
        return Err("stat_percentile: p must be 0..100".into());
    }
    let s = sorted_copy(&data);
    let rank = (p / 100.0) * (s.len() as f64 - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    Ok(float_out(s[lo] * (1.0 - frac) + s[hi] * frac))
}

fn stat_covariance(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "stat_covariance")?;
    let y = vector_at(args, 1, "stat_covariance")?;
    if x.len() != y.len() || x.is_empty() {
        return Err("stat_covariance: x and y must have equal non-zero length".into());
    }
    let mx = x.iter().sum::<f64>() / x.len() as f64;
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let cov = x
        .iter()
        .zip(y.iter())
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>()
        / x.len() as f64;
    Ok(float_out(cov))
}

fn stat_correlation(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "stat_correlation")?;
    let y = vector_at(args, 1, "stat_correlation")?;
    if x.len() != y.len() || x.len() < 2 {
        return Err("stat_correlation: x and y must have equal length >= 2".into());
    }
    let mx = x.iter().sum::<f64>() / x.len() as f64;
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for (a, b) in x.iter().zip(y.iter()) {
        let da = a - mx;
        let db = b - my;
        num += da * db;
        dx2 += da * da;
        dy2 += db * db;
    }
    let denom = (dx2 * dy2).sqrt();
    if denom == 0.0 {
        return Err("stat_correlation: zero variance".into());
    }
    Ok(float_out(num / denom))
}

fn stat_linreg(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = vector_at(args, 0, "stat_linreg")?;
    let y = vector_at(args, 1, "stat_linreg")?;
    if x.len() != y.len() || x.len() < 2 {
        return Err("stat_linreg: x and y must have equal length >= 2".into());
    }
    let mx = x.iter().sum::<f64>() / x.len() as f64;
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    for (a, b) in x.iter().zip(y.iter()) {
        let dx = a - mx;
        let dy = b - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx == 0.0 {
        return Err("stat_linreg: x has zero variance".into());
    }
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let r2 = if syy == 0.0 { 1.0 } else { (sxy * sxy) / (sxx * syy) };
    Ok(vector_out(&[slope, intercept, r2]))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_stat_mean", "stat_mean"], stat_mean);
    bind(&["science_stat_sum", "stat_sum"], stat_sum);
    bind(&["science_stat_count", "stat_count"], stat_count);
    bind(&["science_stat_min", "stat_min"], stat_min);
    bind(&["science_stat_max", "stat_max"], stat_max);
    bind(&["science_stat_median", "stat_median"], stat_median);
    bind(&["science_stat_var", "stat_var"], stat_var);
    bind(&["science_stat_std", "stat_std"], stat_std);
    bind(&["science_stat_sample_var", "stat_sample_var"], stat_sample_var);
    bind(&["science_stat_sample_std", "stat_sample_std"], stat_sample_std);
    bind(&["science_stat_percentile", "stat_percentile"], stat_percentile);
    bind(&["science_stat_covariance", "stat_covariance"], stat_covariance);
    bind(&["science_stat_correlation", "stat_correlation"], stat_correlation);
    bind(&["science_stat_linreg", "stat_linreg"], stat_linreg);
}
