//! Science & engineering toolbox — loaded via `import "science"`.
//!
//! Complex numbers, math, physics, chemistry, economics, digital/bit ops,
//! ndarray (SC0), ML/autograd (SC2), FFT/SVD, CSV/plot, GPU tensors.

pub mod autograd;
pub mod bench;
pub mod classic_ml;
pub mod data;
pub mod dataframe;
#[cfg(not(target_arch = "wasm32"))]
pub mod apache_parquet;
pub mod gpu_compute;
pub mod gpu_tensor;
pub mod helpers;
pub mod interpolate;
pub mod linalg;
pub mod matrix;
pub mod ml;
pub mod ndarray;
pub mod nn_layers;
pub mod numerics;
pub mod ode;
pub mod optimize;
pub mod signal;
pub mod sparse;
pub mod special;
pub mod stats;
pub mod tokenizer;
pub mod training;
pub mod transformer;

use crate::value::{Environment, Value};

const R_GAS: f64 = 8.314462618; // J/(mol·K)
const C_LIGHT: f64 = 299_792_458.0; // m/s

fn num(v: &Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

fn num_at(args: &[Value], i: usize, name: &str) -> Result<f64, String> {
    args.get(i)
        .ok_or_else(|| format!("{}: missing argument {}", name, i))
        .and_then(num)
}

fn cplx_val(v: &Value) -> Result<(f64, f64), String> {
    match v {
        Value::Array(items) if items.len() == 2 => Ok((num(&items[0])?, num(&items[1])?)),
        _ => Err("expected complex number [re, im] from cplx()".into()),
    }
}

fn cplx_out(re: f64, im: f64) -> Value {
    Value::Array(vec![Value::Float(re), Value::Float(im)])
}

fn float_out(x: f64) -> Value {
    Value::Float(x)
}

fn int_out(n: i64) -> Value {
    Value::Number(n)
}

// --- Complex ---

fn science_cplx(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(cplx_out(num_at(args, 0, "cplx")?, num_at(args, 1, "cplx")?))
}

fn science_c_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (ar, ai) = cplx_val(args.first().ok_or("c_add expects a")?)?;
    let (br, bi) = cplx_val(args.get(1).ok_or("c_add expects b")?)?;
    Ok(cplx_out(ar + br, ai + bi))
}

fn science_c_sub(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (ar, ai) = cplx_val(args.first().ok_or("c_sub expects a")?)?;
    let (br, bi) = cplx_val(args.get(1).ok_or("c_sub expects b")?)?;
    Ok(cplx_out(ar - br, ai - bi))
}

fn science_c_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (ar, ai) = cplx_val(args.first().ok_or("c_mul expects a")?)?;
    let (br, bi) = cplx_val(args.get(1).ok_or("c_mul expects b")?)?;
    Ok(cplx_out(ar * br - ai * bi, ar * bi + ai * br))
}

fn science_c_div(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (ar, ai) = cplx_val(args.first().ok_or("c_div expects a")?)?;
    let (br, bi) = cplx_val(args.get(1).ok_or("c_div expects b")?)?;
    let denom = br * br + bi * bi;
    if denom == 0.0 {
        return Err("complex division by zero".into());
    }
    Ok(cplx_out((ar * br + ai * bi) / denom, (ai * br - ar * bi) / denom))
}

fn science_c_conj(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (r, i) = cplx_val(args.first().ok_or("c_conj expects z")?)?;
    Ok(cplx_out(r, -i))
}

fn science_c_abs(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (r, i) = cplx_val(args.first().ok_or("c_abs expects z")?)?;
    Ok(float_out((r * r + i * i).sqrt()))
}

fn science_c_arg(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (r, i) = cplx_val(args.first().ok_or("c_arg expects z")?)?;
    Ok(float_out(i.atan2(r)))
}

fn science_c_exp(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (r, i) = cplx_val(args.first().ok_or("c_exp expects z")?)?;
    let er = r.exp();
    Ok(cplx_out(er * i.cos(), er * i.sin()))
}

fn science_c_sqrt(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (r, i) = cplx_val(args.first().ok_or("c_sqrt expects z")?)?;
    let mag = (r * r + i * i).sqrt();
    let theta = i.atan2(r) / 2.0;
    Ok(cplx_out(mag * theta.cos(), mag * theta.sin()))
}

fn science_c_polar(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mag = num_at(args, 0, "c_polar")?;
    let theta = num_at(args, 1, "c_polar")?;
    Ok(cplx_out(mag * theta.cos(), mag * theta.sin()))
}

// --- Real math ---

fn science_sqrt(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = num_at(args, 0, "sqrt")?;
    if x < 0.0 {
        return Err("sqrt of negative; use c_sqrt(cplx(x, 0))".into());
    }
    Ok(float_out(x.sqrt()))
}

fn science_pow(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(
        num_at(args, 0, "pow")?.powf(num_at(args, 1, "pow")?),
    ))
}

fn science_fact(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "fact")?;
    if n < 0.0 || n.fract() != 0.0 {
        return Err("factorial requires non-negative integer".into());
    }
    let n = n as u64;
    if n > 20 {
        return Err("factorial limited to n <= 20".into());
    }
    let mut acc: u64 = 1;
    for k in 2..=n {
        acc *= k;
    }
    Ok(int_out(acc as i64))
}

fn science_gcd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut a = num_at(args, 0, "gcd")? as i64;
    let mut b = num_at(args, 1, "gcd")? as i64;
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    Ok(int_out(a))
}

fn science_lcm(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = num_at(args, 0, "lcm")? as i64;
    let b = num_at(args, 1, "lcm")? as i64;
    if a == 0 || b == 0 {
        return Ok(int_out(0));
    }
    let g = {
        let mut x = a.abs();
        let mut y = b.abs();
        while y != 0 {
            let t = y;
            y = x % y;
            x = t;
        }
        x
    };
    Ok(int_out((a.abs() * b.abs()) / g))
}

macro_rules! unary_float {
    ($name:ident, $op:expr) => {
        fn $name(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
            Ok(float_out($op(num_at(args, 0, stringify!($name))?)))
        }
    };
}

unary_float!(science_sin, |x: f64| x.sin());
unary_float!(science_cos, |x: f64| x.cos());
unary_float!(science_tan, |x: f64| x.tan());
unary_float!(science_ln, |x: f64| x.ln());
unary_float!(science_log10, |x: f64| x.log10());
unary_float!(science_deg2rad, |x: f64| x.to_radians());
unary_float!(science_rad2deg, |x: f64| x.to_degrees());

fn science_quadratic(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = num_at(args, 0, "quadratic")?;
    let b = num_at(args, 1, "quadratic")?;
    let c = num_at(args, 2, "quadratic")?;
    if a == 0.0 {
        return Err("quadratic: a must not be zero".into());
    }
    let disc = b * b - 4.0 * a * c;
    if disc >= 0.0 {
        let s = disc.sqrt();
        Ok(Value::Array(vec![
            float_out((-b + s) / (2.0 * a)),
            float_out((-b - s) / (2.0 * a)),
        ]))
    } else {
        let s = (-disc).sqrt() / (2.0 * a);
        let re = -b / (2.0 * a);
        Ok(Value::Array(vec![
            cplx_out(re, s),
            cplx_out(re, -s),
        ]))
    }
}

// --- Physics ---

fn science_kinetic_energy(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = num_at(args, 0, "kinetic_energy")?;
    let v = num_at(args, 1, "kinetic_energy")?;
    Ok(float_out(0.5 * m * v * v))
}

fn science_potential_energy(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = num_at(args, 0, "potential_energy")?;
    let g = num_at(args, 1, "potential_energy")?;
    let h = num_at(args, 2, "potential_energy")?;
    Ok(float_out(m * g * h))
}

fn science_force(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(
        num_at(args, 0, "force")? * num_at(args, 1, "force")?,
    ))
}

fn science_ohms_v(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(num_at(args, 0, "ohms_v")? * num_at(args, 1, "ohms_v")?))
}

fn science_ohms_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(num_at(args, 0, "ohms_p")? * num_at(args, 1, "ohms_p")?))
}

fn science_wavelength(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f = num_at(args, 0, "wavelength")?;
    if f == 0.0 {
        return Err("frequency must not be zero".into());
    }
    Ok(float_out(C_LIGHT / f))
}

fn science_photon_energy(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f = num_at(args, 0, "photon_energy")?;
    Ok(float_out(6.62607015e-34 * f))
}

fn science_relativity_e(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let m = num_at(args, 0, "relativity_e")?;
    Ok(float_out(m * C_LIGHT * C_LIGHT))
}

// --- Chemistry ---

fn science_ph(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let h = num_at(args, 0, "ph")?;
    if h <= 0.0 {
        return Err("H+ concentration must be positive".into());
    }
    Ok(float_out(-h.log10()))
}

fn science_h_plus(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(float_out(10f64.powf(-num_at(args, 0, "h_plus")?)))
}

fn science_molarity(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let moles = num_at(args, 0, "molarity")?;
    let vol = num_at(args, 1, "molarity")?;
    if vol == 0.0 {
        return Err("volume must not be zero".into());
    }
    Ok(float_out(moles / vol))
}

fn science_ideal_gas_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "ideal_gas_p")?;
    let t = num_at(args, 1, "ideal_gas_p")?;
    let v = num_at(args, 2, "ideal_gas_p")?;
    if v == 0.0 {
        return Err("volume must not be zero".into());
    }
    Ok(float_out(n * R_GAS * t / v))
}

fn science_dilution(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    // C1*V1 = C2*V2  => V2 = C1*V1/C2
    let c1 = num_at(args, 0, "dilution")?;
    let v1 = num_at(args, 1, "dilution")?;
    let c2 = num_at(args, 2, "dilution")?;
    if c2 == 0.0 {
        return Err("target concentration must not be zero".into());
    }
    Ok(float_out(c1 * v1 / c2))
}

// --- Economics ---

fn science_compound(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = num_at(args, 0, "compound")?;
    let r = num_at(args, 1, "compound")?;
    let n = num_at(args, 2, "compound")?;
    Ok(float_out(p * (1.0 + r).powf(n)))
}

fn science_present_value(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fv = num_at(args, 0, "present_value")?;
    let r = num_at(args, 1, "present_value")?;
    let n = num_at(args, 2, "present_value")?;
    Ok(float_out(fv / (1.0 + r).powf(n)))
}

fn science_break_even(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fixed = num_at(args, 0, "break_even")?;
    let price = num_at(args, 1, "break_even")?;
    let var = num_at(args, 2, "break_even")?;
    let margin = price - var;
    if margin == 0.0 {
        return Err("price and variable cost must differ".into());
    }
    Ok(float_out(fixed / margin))
}

fn science_roi(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let gain = num_at(args, 0, "roi")?;
    let cost = num_at(args, 1, "roi")?;
    if cost == 0.0 {
        return Err("cost must not be zero".into());
    }
    Ok(float_out((gain - cost) / cost * 100.0))
}

fn science_margin(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let revenue = num_at(args, 0, "margin")?;
    let cost = num_at(args, 1, "margin")?;
    if revenue == 0.0 {
        return Err("revenue must not be zero".into());
    }
    Ok(float_out((revenue - cost) / revenue * 100.0))
}

// --- Digital ---

fn science_bit_and(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(int_out(
        (num_at(args, 0, "bit_and")? as i64) & (num_at(args, 1, "bit_and")? as i64),
    ))
}

fn science_bit_or(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(int_out(
        (num_at(args, 0, "bit_or")? as i64) | (num_at(args, 1, "bit_or")? as i64),
    ))
}

fn science_bit_xor(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(int_out(
        (num_at(args, 0, "bit_xor")? as i64) ^ (num_at(args, 1, "bit_xor")? as i64),
    ))
}

fn science_bit_not(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(int_out(!(num_at(args, 0, "bit_not")? as i64)))
}

fn science_shl(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let bits = (num_at(args, 1, "shl")? as i64).clamp(0, 63) as u32;
    Ok(int_out((num_at(args, 0, "shl")? as i64) << bits))
}

fn science_shr(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let bits = (num_at(args, 1, "shr")? as i64).clamp(0, 63) as u32;
    Ok(int_out((num_at(args, 0, "shr")? as i64) >> bits))
}

fn parse_uint_base(s: &str, base: u32) -> Result<i64, String> {
    if s.is_empty() {
        return Err("empty digit string".into());
    }
    i64::from_str_radix(s, base).map_err(|e| format!("parse error: {}", e))
}

fn science_hex(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(x)) => x.trim(),
        _ => return Err("hex() expects string".into()),
    };
    Ok(int_out(parse_uint_base(s, 16)?))
}

fn science_bin(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(x)) => x.trim(),
        _ => return Err("bin() expects string".into()),
    };
    Ok(int_out(parse_uint_base(s, 2)?))
}

fn science_hamming_weight(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut n = num_at(args, 0, "hamming_weight")? as u64;
    let mut count = 0i64;
    while n > 0 {
        count += (n & 1) as i64;
        n >>= 1;
    }
    Ok(int_out(count))
}

pub fn register(env: &mut Environment) {
    let mut bind = |names: &[&str], func: fn(&[Value], &mut Environment) -> Result<Value, String>| {
        for name in names {
            env.set((*name).to_string(), Value::NativeFunction(func));
        }
    };

    bind(&["science_cplx", "cplx"], science_cplx);
    bind(&["science_c_add", "c_add"], science_c_add);
    bind(&["science_c_sub", "c_sub"], science_c_sub);
    bind(&["science_c_mul", "c_mul"], science_c_mul);
    bind(&["science_c_div", "c_div"], science_c_div);
    bind(&["science_c_conj", "c_conj"], science_c_conj);
    bind(&["science_c_abs", "c_abs"], science_c_abs);
    bind(&["science_c_arg", "c_arg"], science_c_arg);
    bind(&["science_c_exp", "c_exp"], science_c_exp);
    bind(&["science_c_sqrt", "c_sqrt"], science_c_sqrt);
    bind(&["science_c_polar", "c_polar"], science_c_polar);
    bind(&["science_sqrt", "sqrt"], science_sqrt);
    bind(&["science_pow", "pow"], science_pow);
    bind(&["science_fact", "fact"], science_fact);
    bind(&["science_gcd", "gcd"], science_gcd);
    bind(&["science_lcm", "lcm"], science_lcm);
    bind(&["science_sin", "sin"], science_sin);
    bind(&["science_cos", "cos"], science_cos);
    bind(&["science_tan", "tan"], science_tan);
    bind(&["science_ln", "ln"], science_ln);
    bind(&["science_log10", "log10"], science_log10);
    bind(&["science_deg2rad", "deg2rad"], science_deg2rad);
    bind(&["science_rad2deg", "rad2deg"], science_rad2deg);
    bind(&["science_quadratic", "quadratic"], science_quadratic);
    bind(&["science_kinetic_energy", "kinetic_energy"], science_kinetic_energy);
    bind(&["science_potential_energy", "potential_energy"], science_potential_energy);
    bind(&["science_force", "force"], science_force);
    bind(&["science_ohms_v", "ohms_v"], science_ohms_v);
    bind(&["science_ohms_p", "ohms_p"], science_ohms_p);
    bind(&["science_wavelength", "wavelength"], science_wavelength);
    bind(&["science_photon_energy", "photon_energy"], science_photon_energy);
    bind(&["science_relativity_e", "relativity_e"], science_relativity_e);
    bind(&["science_ph", "ph"], science_ph);
    bind(&["science_h_plus", "h_plus"], science_h_plus);
    bind(&["science_molarity", "molarity"], science_molarity);
    bind(&["science_ideal_gas_p", "ideal_gas_p"], science_ideal_gas_p);
    bind(&["science_dilution", "dilution"], science_dilution);
    bind(&["science_compound", "compound"], science_compound);
    bind(&["science_present_value", "present_value"], science_present_value);
    bind(&["science_break_even", "break_even"], science_break_even);
    bind(&["science_roi", "roi"], science_roi);
    bind(&["science_margin", "margin"], science_margin);
    bind(&["science_bit_and", "bit_and"], science_bit_and);
    bind(&["science_bit_or", "bit_or"], science_bit_or);
    bind(&["science_bit_xor", "bit_xor"], science_bit_xor);
    bind(&["science_bit_not", "bit_not"], science_bit_not);
    bind(&["science_shl", "shl"], science_shl);
    bind(&["science_shr", "shr"], science_shr);
    bind(&["science_hex", "hex"], science_hex);
    bind(&["science_bin", "bin"], science_bin);
    bind(&["science_hamming_weight", "hamming_weight"], science_hamming_weight);
    stats::register(&mut bind);
    matrix::register(&mut bind);
    numerics::register(&mut bind);
    ndarray::register(&mut bind);
    ml::register(&mut bind);
    classic_ml::register(&mut bind);
    nn_layers::register(&mut bind);
    autograd::register(&mut bind);
    signal::register(&mut bind);
    linalg::register(&mut bind);
    interpolate::register(&mut bind);
    special::register(&mut bind);
    sparse::register(&mut bind);
    optimize::register(&mut bind);
    ode::register(&mut bind);
    data::register(&mut bind);
    dataframe::register(&mut bind);
    gpu_tensor::register(&mut bind);
    training::register(&mut bind);
    tokenizer::register(&mut bind);
    transformer::register(&mut bind);
    bench::register(&mut bind);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_abs() {
        let z = cplx_out(3.0, 4.0);
        let r = science_c_abs(&[z], &mut Environment::new()).unwrap();
        assert!(matches!(r, Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    #[test]
    fn quadratic_real() {
        let r = science_quadratic(
            &[Value::Number(1), Value::Number(-5), Value::Number(6)],
            &mut Environment::new(),
        )
        .unwrap();
        match r {
            Value::Array(v) => assert_eq!(v.len(), 2),
            _ => panic!("expected two roots"),
        }
    }

}
