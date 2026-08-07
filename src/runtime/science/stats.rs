//! Descriptive statistics for `import "science"`.

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
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

/// stat_quantile(data, q) — q in [0,1]
fn stat_quantile(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let data = args.first().cloned().ok_or("stat_quantile(data, q)")?;
    let q = num_at(args, 1, "stat_quantile")?;
    if !(0.0..=1.0).contains(&q) {
        return Err("stat_quantile: q must be 0..1".into());
    }
    stat_percentile(&[data, float_out(q * 100.0)], env)
}

fn erf_approx(x: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let y = 1.0
        - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

fn stat_norm_pdf(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = num_at(args, 0, "stat_norm_pdf")?;
    let mean = args.get(1).and_then(|v| num(v).ok()).unwrap_or(0.0);
    let std = args.get(2).and_then(|v| num(v).ok()).unwrap_or(1.0);
    if std <= 0.0 {
        return Err("stat_norm_pdf: std > 0".into());
    }
    let z = (x - mean) / std;
    let dens = (-0.5 * z * z).exp() / (std * (2.0 * std::f64::consts::PI).sqrt());
    Ok(float_out(dens))
}

fn stat_norm_cdf(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = num_at(args, 0, "stat_norm_cdf")?;
    let mean = args.get(1).and_then(|v| num(v).ok()).unwrap_or(0.0);
    let std = args.get(2).and_then(|v| num(v).ok()).unwrap_or(1.0);
    if std <= 0.0 {
        return Err("stat_norm_cdf: std > 0".into());
    }
    let z = (x - mean) / (std * std::f64::consts::SQRT_2);
    Ok(float_out(0.5 * (1.0 + erf_approx(z))))
}

/// Inverse erf approximation (Winitzki).
fn erfinv_approx(x: f64) -> f64 {
    let a = 0.147;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.clamp(-0.999999, 0.999999);
    let ln = (1.0 - x * x).ln();
    let term = 2.0 / (std::f64::consts::PI * a) + 0.5 * ln;
    let inner = term * term - ln / a;
    sign * (inner.sqrt() - term).sqrt()
}

/// stat_norm_ppf(p, mean?, std?)
fn stat_norm_ppf(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = num_at(args, 0, "stat_norm_ppf")?;
    let mean = args.get(1).and_then(|v| num(v).ok()).unwrap_or(0.0);
    let std = args.get(2).and_then(|v| num(v).ok()).unwrap_or(1.0);
    if std <= 0.0 {
        return Err("stat_norm_ppf: std > 0".into());
    }
    if p <= 0.0 || p >= 1.0 {
        return Err("stat_norm_ppf: p in (0,1)".into());
    }
    let z = std::f64::consts::SQRT_2 * erfinv_approx(2.0 * p - 1.0);
    Ok(float_out(mean + std * z))
}

/// Two-sample Welch t-test → {t, df, mean_a, mean_b}
fn stat_ttest(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "stat_ttest")?;
    let b = vector_at(args, 1, "stat_ttest")?;
    if a.len() < 2 || b.len() < 2 {
        return Err("stat_ttest: need >=2 samples each".into());
    }
    let na = a.len() as f64;
    let nb = b.len() as f64;
    let ma = a.iter().sum::<f64>() / na;
    let mb = b.iter().sum::<f64>() / nb;
    let va = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / (na - 1.0);
    let vb = b.iter().map(|x| (x - mb).powi(2)).sum::<f64>() / (nb - 1.0);
    let se = (va / na + vb / nb).sqrt();
    if se < 1e-15 {
        return Err("stat_ttest: zero pooled SE".into());
    }
    let t = (ma - mb) / se;
    let num = (va / na + vb / nb).powi(2);
    let den = (va / na).powi(2) / (na - 1.0) + (vb / nb).powi(2) / (nb - 1.0);
    let df = if den > 0.0 { num / den } else { na + nb - 2.0 };
    let mut out = std::collections::HashMap::new();
    out.insert("t".into(), float_out(t));
    out.insert("df".into(), float_out(df));
    out.insert("mean_a".into(), float_out(ma));
    out.insert("mean_b".into(), float_out(mb));
    Ok(Value::from_object(out))
}

/// Pearson chi-square goodness of fit: observed vs expected counts.
fn stat_chi2(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let obs = vector_at(args, 0, "stat_chi2")?;
    let exp = vector_at(args, 1, "stat_chi2")?;
    if obs.len() != exp.len() || obs.is_empty() {
        return Err("stat_chi2: length mismatch".into());
    }
    let mut chi = 0.0;
    for (o, e) in obs.iter().zip(exp.iter()) {
        if *e <= 0.0 {
            return Err("stat_chi2: expected > 0".into());
        }
        chi += (o - e).powi(2) / e;
    }
    let mut out = std::collections::HashMap::new();
    out.insert("chi2".into(), float_out(chi));
    out.insert("df".into(), float_out((obs.len() as f64 - 1.0).max(0.0)));
    Ok(Value::from_object(out))
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
    bind(&["science_stat_quantile", "stat_quantile"], stat_quantile);
    bind(&["science_stat_covariance", "stat_covariance"], stat_covariance);
    bind(&["science_stat_corr", "stat_corr"], stat_correlation);
    bind(&["science_stat_correlation", "stat_correlation"], stat_correlation);
    bind(&["science_stat_cov", "stat_cov"], stat_covariance);
    bind(&["science_stat_linreg", "stat_linreg"], stat_linreg);
    bind(&["science_stat_norm_pdf", "stat_norm_pdf"], stat_norm_pdf);
    bind(&["science_stat_norm_cdf", "stat_norm_cdf"], stat_norm_cdf);
    bind(&["science_stat_norm_ppf", "stat_norm_ppf"], stat_norm_ppf);
    bind(&["science_stat_ttest", "stat_ttest"], stat_ttest);
    bind(&["science_stat_chi2", "stat_chi2"], stat_chi2);
}
