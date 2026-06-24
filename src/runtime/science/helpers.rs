//! Shared helpers for science natives.

use crate::value::Value;

pub fn num(v: &Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

pub fn num_at(args: &[Value], i: usize, name: &str) -> Result<f64, String> {
    args.get(i)
        .ok_or_else(|| format!("{name}: missing argument {i}"))
        .and_then(num)
}

pub fn float_out(x: f64) -> Value {
    Value::Float(x)
}

pub fn int_out(n: i64) -> Value {
    Value::Number(n)
}

pub fn bool_out(b: bool) -> Value {
    Value::Bool(b)
}

pub fn vector_val(v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::Array(items) => items.iter().map(num).collect(),
        _ => Err("expected numeric array".into()),
    }
}

pub fn vector_at(args: &[Value], i: usize, name: &str) -> Result<Vec<f64>, String> {
    args.get(i)
        .ok_or_else(|| format!("{name}: missing argument {i}"))
        .and_then(vector_val)
}

pub fn vector_out(values: &[f64]) -> Value {
    Value::Array(values.iter().map(|&x| float_out(x)).collect())
}

pub fn matrix_val(v: &Value) -> Result<Vec<Vec<f64>>, String> {
    match v {
        Value::Array(rows) => {
            if rows.is_empty() {
                return Ok(Vec::new());
            }
            let mut out = Vec::with_capacity(rows.len());
            let cols = match &rows[0] {
                Value::Array(r0) => r0.len(),
                _ => return Err("expected matrix (array of rows)".into()),
            };
            for row in rows {
                let Value::Array(r) = row else {
                    return Err("expected matrix row as array".into());
                };
                if r.len() != cols {
                    return Err("matrix rows must have equal length".into());
                }
                out.push(r.iter().map(num).collect::<Result<Vec<_>, _>>()?);
            }
            Ok(out)
        }
        _ => Err("expected matrix (array of rows)".into()),
    }
}

pub fn matrix_at(args: &[Value], i: usize, name: &str) -> Result<Vec<Vec<f64>>, String> {
    args.get(i)
        .ok_or_else(|| format!("{name}: missing argument {i}"))
        .and_then(matrix_val)
}

pub fn matrix_out(m: &[Vec<f64>]) -> Value {
    Value::Array(m.iter().map(|row| vector_out(row)).collect())
}

pub fn matrix_rows(m: &[Vec<f64>]) -> usize {
    m.len()
}

pub fn matrix_cols(m: &[Vec<f64>]) -> Result<usize, String> {
    m.first()
        .map(|r| r.len())
        .ok_or_else(|| "empty matrix".into())
}

pub fn require_square(m: &[Vec<f64>], name: &str) -> Result<(), String> {
    let rows = matrix_rows(m);
    let cols = matrix_cols(m)?;
    if rows != cols {
        return Err(format!("{name}: matrix must be square"));
    }
    if rows == 0 {
        return Err(format!("{name}: empty matrix"));
    }
    Ok(())
}

pub fn require_same_shape(a: &[Vec<f64>], b: &[Vec<f64>], name: &str) -> Result<(), String> {
    if matrix_rows(a) != matrix_rows(b) || matrix_cols(a)? != matrix_cols(b)? {
        return Err(format!("{name}: matrices must have the same shape"));
    }
    Ok(())
}

pub fn require_mul_shape(a: &[Vec<f64>], b: &[Vec<f64>], name: &str) -> Result<(), String> {
    if matrix_cols(a)? != matrix_rows(b) {
        return Err(format!(
            "{name}: column count of A must equal row count of B"
        ));
    }
    Ok(())
}
