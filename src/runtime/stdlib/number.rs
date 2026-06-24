//! Number parsing — JS `parseInt` / `parseFloat` parity.

use crate::value::{format_value, Environment, Value};

fn parse_int_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.trim_start(),
        Some(Value::Number(n)) => return Ok(Value::Number(*n)),
        Some(Value::Float(f)) => return Ok(num_from_f64(*f)),
        _ => return Err("parse_int(text, radix?) expects string or number".into()),
    };
    let radix = match args.get(1) {
        Some(Value::Number(n)) if (2..=36).contains(n) => *n as u32,
        None => 10,
        _ => return Err("parse_int radix must be 2..36".into()),
    };
    let prefix_len = if radix == 16 && (s.starts_with("0x") || s.starts_with("0X")) {
        2
    } else if radix == 8 && s.starts_with('0') {
        1
    } else if radix == 2 && (s.starts_with("0b") || s.starts_with("0B")) {
        2
    } else {
        0
    };
    let digits = &s[prefix_len..];
    let mut acc: i64 = 0;
    let mut any = false;
    for ch in digits.chars() {
        let digit = ch.to_digit(radix);
        if let Some(d) = digit {
            any = true;
            acc = acc
                .checked_mul(radix as i64)
                .and_then(|v| v.checked_add(d as i64))
                .ok_or_else(|| "parse_int overflow".to_string())?;
        } else if ch.is_whitespace() {
            if any {
                break;
            }
        } else {
            break;
        }
    }
    if any {
        Ok(Value::Number(acc))
    } else {
        Ok(Value::Float(f64::NAN))
    }
}

fn parse_float_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.trim_start(),
        Some(Value::Number(n)) => return Ok(Value::Number(*n)),
        Some(Value::Float(f)) => return Ok(Value::Float(*f)),
        _ => return Err("parse_float(text) expects string or number".into()),
    };
    let mut end = 0usize;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || ch == '+' || ch == '-' {
            end = i + ch.len_utf8();
        } else if ch.is_whitespace() && end == 0 {
            continue;
        } else {
            break;
        }
    }
    if end == 0 {
        return Ok(Value::Float(f64::NAN));
    }
    match s[..end].parse::<f64>() {
        Ok(f) => Ok(Value::Float(f)),
        Err(_) => Ok(Value::Float(f64::NAN)),
    }
}

fn is_finite_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_finite(n)")?;
    let f = match v {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        _ => return Err("is_finite expects number".into()),
    };
    Ok(Value::Bool(f.is_finite()))
}

fn to_fixed_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f = match args.first() {
        Some(Value::Number(n)) => *n as f64,
        Some(Value::Float(f)) => *f,
        _ => return Err("to_fixed(n, digits?) expects number".into()),
    };
    let digits = match args.get(1) {
        Some(Value::Number(n)) if (0..=100).contains(n) => *n as usize,
        None => 0,
        _ => return Err("to_fixed digits must be 0..100".into()),
    };
    Ok(Value::String(format!("{f:.digits$}")))
}

fn is_integer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_integer(n)")?;
    Ok(Value::Bool(match v {
        Value::Number(_) => true,
        Value::Float(f) => f.is_finite() && f.fract() == 0.0,
        _ => false,
    }))
}

fn to_exponential_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f = match args.first() {
        Some(Value::Number(n)) => *n as f64,
        Some(Value::Float(f)) => *f,
        _ => return Err("to_exponential(n, digits?) expects number".into()),
    };
    let digits = match args.get(1) {
        Some(Value::Number(n)) if (0..=100).contains(n) => *n as usize,
        None => 0,
        _ => return Err("to_exponential digits must be 0..100".into()),
    };
    Ok(Value::String(format!("{f:.digits$e}")))
}

fn to_precision_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let f = match args.first() {
        Some(Value::Number(n)) => *n as f64,
        Some(Value::Float(f)) => *f,
        _ => return Err("to_precision(n, precision?) expects number".into()),
    };
    let prec = match args.get(1) {
        Some(Value::Number(n)) if (1..=100).contains(n) => *n as usize,
        None => 1,
        _ => return Err("to_precision precision must be 1..100".into()),
    };
    Ok(Value::String(format!("{f:.prec$}")))
}

fn is_safe_integer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_safe_integer(n)")?;
    let f = match v {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        _ => return Ok(Value::Bool(false)),
    };
    Ok(Value::Bool(
        f.is_finite()
            && f.fract() == 0.0
            && f >= -(2f64.powi(53) - 1.0)
            && f <= 2f64.powi(53) - 1.0,
    ))
}

fn is_nan_number_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("number_is_nan(n)")?;
    Ok(Value::Bool(v.is_nan()))
}

fn number_to_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("number_to_string(n)")?;
    Ok(Value::String(format_value(v)))
}

fn num_from_f64(f: f64) -> Value {
    if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Value::Number(f as i64)
    } else {
        Value::Float(f)
    }
}

pub fn register_number(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("parse_int", parse_int_native),
        ("parse_float", parse_float_native),
        ("is_finite", is_finite_native),
        ("to_fixed", to_fixed_native),
        ("is_integer", is_integer_native),
        ("number_is_integer", is_integer_native),
        ("to_exponential", to_exponential_native),
        ("to_precision", to_precision_native),
        ("is_safe_integer", is_safe_integer_native),
        ("number_is_finite", is_finite_native),
        ("number_is_nan", is_nan_number_native),
        ("number_to_string", number_to_string_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
