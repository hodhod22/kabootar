//! ECMAScript `BigInt` — literals (`123n`), `BigInt()`, and bigint arithmetic.

use crate::value::{Environment, Value};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

pub fn parse_decimal(digits: &str) -> Result<BigInt, String> {
    if digits.is_empty() {
        return Ok(BigInt::zero());
    }
    BigInt::parse_bytes(digits.as_bytes(), 10)
        .ok_or_else(|| format!("invalid BigInt literal: {digits}"))
}

pub fn is_bigint(v: &Value) -> bool {
    matches!(v, Value::BigInt(_))
}

pub fn bigint_value(b: BigInt) -> Value {
    Value::BigInt(b)
}

pub fn format_bigint(b: &BigInt) -> String {
    format!("{b}n")
}

fn mixed_type_err(op: &str) -> String {
    format!("Cannot {op} BigInt and Number")
}

fn pair_bigint<'a>(left: &'a Value, right: &'a Value, op: &str) -> Result<(&'a BigInt, &'a BigInt), String> {
    match (left, right) {
        (Value::BigInt(a), Value::BigInt(b)) => Ok((a, b)),
        _ => Err(mixed_type_err(op)),
    }
}

pub fn try_add(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(pair_bigint(left, right, "add").map(|(a, b)| bigint_value(a + b)))
}

pub fn try_sub(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(pair_bigint(left, right, "subtract").map(|(a, b)| bigint_value(a - b)))
}

pub fn try_mul(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(pair_bigint(left, right, "multiply").map(|(a, b)| bigint_value(a * b)))
}

pub fn try_div(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(
        pair_bigint(left, right, "divide").and_then(|(a, b)| {
            if b.is_zero() {
                Err("BigInt division by zero".into())
            } else {
                Ok(bigint_value(a / b))
            }
        }),
    )
}

pub fn try_mod(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(
        pair_bigint(left, right, "modulo").and_then(|(a, b)| {
            if b.is_zero() {
                Err("BigInt modulo by zero".into())
            } else {
                Ok(bigint_value(a % b))
            }
        }),
    )
}

pub fn try_pow(left: &Value, right: &Value) -> Option<Result<Value, String>> {
    if !is_bigint(left) && !is_bigint(right) {
        return None;
    }
    Some(
        pair_bigint(left, right, "exponentiate").and_then(|(base, exp)| {
            if exp.is_negative() {
                return Err("BigInt negative exponent".into());
            }
            let exp_u32 = exp
                .to_u32()
                .ok_or_else(|| "BigInt exponent too large".to_string())?;
            Ok(bigint_value(base.pow(exp_u32)))
        }),
    )
}

pub fn try_neg(v: &Value) -> Option<Result<Value, String>> {
    let Value::BigInt(n) = v else {
        return None;
    };
    Some(Ok(bigint_value(-n.clone())))
}

pub fn loose_equal(left: &Value, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::BigInt(a), Value::BigInt(b)) => Some(a == b),
        (Value::BigInt(a), Value::Number(n)) => bigint_from_i64(*n).ok().map(|b| a == &b),
        (Value::Number(n), Value::BigInt(b)) => bigint_from_i64(*n).ok().map(|a| &a == b),
        (Value::BigInt(a), Value::Float(f)) => bigint_from_float(*f).ok().map(|b| a == &b),
        (Value::Float(f), Value::BigInt(b)) => bigint_from_float(*f).ok().map(|a| &a == b),
        (Value::BigInt(a), Value::String(s)) => parse_decimal(s).ok().map(|b| a == &b),
        (Value::String(s), Value::BigInt(b)) => parse_decimal(s).ok().map(|a| &a == b),
        _ => None,
    }
}

pub fn bigint_from_i64(n: i64) -> Result<BigInt, String> {
    Ok(BigInt::from(n))
}

fn bigint_from_float(f: f64) -> Result<BigInt, String> {
    if !f.is_finite() || f.fract() != 0.0 {
        return Err("number is not an integer".into());
    }
    if f < i64::MIN as f64 || f > i64::MAX as f64 {
        return Err("number out of integral range".into());
    }
    Ok(BigInt::from(f as i64))
}

pub fn value_to_bigint(v: &Value) -> Result<BigInt, String> {
    match v {
        Value::BigInt(b) => Ok(b.clone()),
        Value::String(s) => parse_decimal(s),
        Value::Number(n) => bigint_from_i64(*n),
        Value::Float(f) => bigint_from_float(*f),
        Value::Bool(true) => Ok(BigInt::from(1)),
        Value::Bool(false) => Ok(BigInt::zero()),
        Value::Null | Value::Undefined => Err("Cannot convert nullish value to BigInt".into()),
        other => Err(format!("Cannot convert {other:?} to BigInt")),
    }
}

fn bigint_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arg = args.first().ok_or("BigInt() expects one argument")?;
    Ok(bigint_value(value_to_bigint(arg)?))
}

pub fn register_bigint(env: &mut Environment) {
    env.set("BigInt".to_string(), Value::NativeFunction(bigint_native));
}
