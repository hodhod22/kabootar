//! Math helpers — JS `Math.*` parity (global functions).

use crate::value::{Environment, Value};

fn to_f64(v: &Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

fn num_out(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Value::Number(f as i64)
    } else {
        Value::Float(f)
    }
}

fn unary_f64(
    args: &[Value],
    name: &str,
    f: fn(f64) -> f64,
) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or_else(|| format!("{name}(n)"))?)?;
    Ok(num_out(f(x)))
}

fn floor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "floor", |x| x.floor())
}

fn ceil_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "ceil", |x| x.ceil())
}

fn round_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "round", |x| x.round())
}

fn trunc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "trunc", |x| x.trunc())
}

fn abs_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "abs", |x| x.abs())
}

fn sign_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "sign", |x| {
        if x == 0.0 {
            0.0
        } else if x > 0.0 {
            1.0
        } else {
            -1.0
        }
    })
}

fn sqrt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("sqrt(n)")?)?;
    if x < 0.0 {
        return Err("sqrt of negative number".into());
    }
    Ok(num_out(x.sqrt()))
}

fn pow_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let base = to_f64(args.first().ok_or("pow(base, exp)")?)?;
    let exp = to_f64(args.get(1).ok_or("pow(base, exp)")?)?;
    Ok(num_out(base.powf(exp)))
}

fn min_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("min() expects at least one number".into());
    }
    let mut best = to_f64(&args[0])?;
    for v in &args[1..] {
        best = best.min(to_f64(v)?);
    }
    Ok(num_out(best))
}

fn max_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("max() expects at least one number".into());
    }
    let mut best = to_f64(&args[0])?;
    for v in &args[1..] {
        best = best.max(to_f64(v)?);
    }
    Ok(num_out(best))
}

fn clamp_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = to_f64(args.first().ok_or("clamp(v, lo, hi)")?)?;
    let lo = to_f64(args.get(1).ok_or("clamp(v, lo, hi)")?)?;
    let hi = to_f64(args.get(2).ok_or("clamp(v, lo, hi)")?)?;
    Ok(num_out(v.clamp(lo.min(hi), lo.max(hi))))
}

fn random_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    use std::cell::Cell;
    thread_local! {
        static SEED: Cell<u64> = Cell::new(0x9E37_79B9_7F4A_7C15);
    }
    let n = SEED.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x as f64) / (u64::MAX as f64)
    });
    Ok(Value::Float(n))
}

fn pi_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Float(std::f64::consts::PI))
}

fn e_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Float(std::f64::consts::E))
}

fn log_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("log(n)")?)?;
    if x <= 0.0 {
        return Err("log() expects positive number".into());
    }
    Ok(num_out(x.ln()))
}

fn log2_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("log2(n)")?)?;
    if x <= 0.0 {
        return Err("log2() expects positive number".into());
    }
    Ok(num_out(x.log2()))
}

fn log10_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("log10(n)")?)?;
    if x <= 0.0 {
        return Err("log10() expects positive number".into());
    }
    Ok(num_out(x.log10()))
}

fn exp_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("exp(n)")?)?;
    Ok(num_out(x.exp()))
}

fn sin_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "sin", |x| x.sin())
}

fn cos_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "cos", |x| x.cos())
}

fn tan_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "tan", |x| x.tan())
}

fn hypot_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Err("hypot() expects at least one number".into());
    }
    let mut sum = 0.0f64;
    for v in args {
        let x = to_f64(v)?;
        sum += x * x;
    }
    Ok(num_out(sum.sqrt()))
}

fn cbrt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "cbrt", |x| x.cbrt())
}

fn asin_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "asin", |x| x.asin())
}

fn acos_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "acos", |x| x.acos())
}

fn atan_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "atan", |x| x.atan())
}

fn atan2_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let y = to_f64(args.first().ok_or("atan2(y, x)")?)?;
    let x = to_f64(args.get(1).ok_or("atan2(y, x)")?)?;
    Ok(num_out(y.atan2(x)))
}

fn fmod_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = to_f64(args.first().ok_or("fmod(a, b)")?)?;
    let b = to_f64(args.get(1).ok_or("fmod(a, b)")?)?;
    Ok(num_out(a % b))
}

fn to_i32(v: &Value) -> Result<i32, String> {
    match v {
        Value::Number(n) => Ok(*n as i32),
        Value::Float(f) => Ok(*f as i32),
        _ => Err("expected number".into()),
    }
}

fn imul_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = to_i32(args.first().ok_or("imul(a, b)")?)?;
    let b = to_i32(args.get(1).ok_or("imul(a, b)")?)?;
    Ok(Value::Number(a.wrapping_mul(b) as i64))
}

fn clz32_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = to_i32(args.first().ok_or("clz32(n)")?)? as u32;
    Ok(Value::Number(n.leading_zeros() as i64))
}

fn fround_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = to_f64(args.first().ok_or("fround(n)")?)?;
    Ok(num_out((x as f32) as f64))
}

fn log1p_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "log1p", |x| x.ln_1p())
}

fn expm1_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "expm1", |x| x.exp_m1())
}

fn sinh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "sinh", |x| x.sinh())
}

fn cosh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "cosh", |x| x.cosh())
}

fn tanh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "tanh", |x| x.tanh())
}

fn asinh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "asinh", |x| x.asinh())
}

fn acosh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "acosh", |x| x.acosh())
}

fn atanh_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    unary_f64(args, "atanh", |x| x.atanh())
}

pub fn register_math(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("floor", floor_native),
        ("ceil", ceil_native),
        ("round", round_native),
        ("trunc", trunc_native),
        ("abs", abs_native),
        ("sign", sign_native),
        ("sqrt", sqrt_native),
        ("pow", pow_native),
        ("min", min_native),
        ("max", max_native),
        ("clamp", clamp_native),
        ("random", random_native),
        ("pi", pi_native),
        ("e", e_native),
        ("log", log_native),
        ("log2", log2_native),
        ("log10", log10_native),
        ("exp", exp_native),
        ("sin", sin_native),
        ("cos", cos_native),
        ("tan", tan_native),
        ("hypot", hypot_native),
        ("cbrt", cbrt_native),
        ("asin", asin_native),
        ("acos", acos_native),
        ("atan", atan_native),
        ("atan2", atan2_native),
        ("fmod", fmod_native),
        ("imul", imul_native),
        ("clz32", clz32_native),
        ("fround", fround_native),
        ("log1p", log1p_native),
        ("expm1", expm1_native),
        ("sinh", sinh_native),
        ("cosh", cosh_native),
        ("tanh", tanh_native),
        ("asinh", asinh_native),
        ("acosh", acosh_native),
        ("atanh", atanh_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
