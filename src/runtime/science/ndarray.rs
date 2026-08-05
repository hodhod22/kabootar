//! Contiguous ndarray for `import "science"` (SC0 — NumPy-class core).

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

const ND_MARK: &str = "__kab_nd";

fn shape_val(shape: &[usize]) -> Value {
    Value::Array(
        shape
            .iter()
            .map(|d| Value::Number(*d as i64))
            .collect(),
    )
}

fn parse_shape(v: &Value) -> Result<Vec<usize>, String> {
    match v {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                let n = num(it)?;
                if n < 0.0 || n.fract() != 0.0 {
                    return Err("nd shape dims must be non-negative integers".into());
                }
                out.push(n as usize);
            }
            Ok(out)
        }
        Value::Number(n) if *n >= 0 => Ok(vec![*n as usize]),
        Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Ok(vec![*f as usize]),
        _ => Err("nd shape must be number or array of numbers".into()),
    }
}

fn shape_product(shape: &[usize]) -> usize {
    shape.iter().product::<usize>().max(if shape.is_empty() { 1 } else { 0 })
}

fn flat_from_value(v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::Array(items) => {
            // Nested matrix → row-major flat, or flat vector.
            if items
                .first()
                .map(|x| matches!(x, Value::Array(_)))
                .unwrap_or(false)
            {
                let mut flat = Vec::new();
                for row in items {
                    let Value::Array(cells) = row else {
                        return Err("nd: jagged nested array".into());
                    };
                    for c in cells {
                        flat.push(num(c)?);
                    }
                }
                Ok(flat)
            } else {
                items.iter().map(num).collect()
            }
        }
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            let data = m.get("data").ok_or("nd missing data")?;
            flat_from_value(data)
        }
        _ => Err("expected array or ndarray".into()),
    }
}

fn nd_parts(v: &Value) -> Result<(Vec<usize>, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            let shape = parse_shape(m.get("shape").ok_or("nd missing shape")?)?;
            let data = flat_from_value(m.get("data").ok_or("nd missing data")?)?;
            let n = shape_product(&shape);
            if data.len() != n {
                return Err(format!(
                    "nd shape product {n} != data length {}",
                    data.len()
                ));
            }
            Ok((shape, data))
        }
        Value::Array(_) => {
            let data = flat_from_value(v)?;
            Ok((vec![data.len()], data))
        }
        _ => Err("expected ndarray".into()),
    }
}

fn nd_out(shape: &[usize], data: &[f64]) -> Value {
    let mut m = HashMap::new();
    m.insert(ND_MARK.into(), Value::Bool(true));
    m.insert("shape".into(), shape_val(shape));
    m.insert("data".into(), vector_out(data));
    m.insert("size".into(), Value::Number(data.len() as i64));
    Value::Object(m)
}

fn nd_at(args: &[Value], i: usize, name: &str) -> Result<(Vec<usize>, Vec<f64>), String> {
    let v = args
        .get(i)
        .ok_or_else(|| format!("{name}: missing ndarray arg {i}"))?;
    nd_parts(v)
}

fn nd_zeros(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("nd_zeros(shape)")?)?;
    let n = shape_product(&shape);
    Ok(nd_out(&shape, &vec![0.0; n]))
}

fn nd_ones(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("nd_ones(shape)")?)?;
    let n = shape_product(&shape);
    Ok(nd_out(&shape, &vec![1.0; n]))
}

fn nd_full(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("nd_full(shape, value)")?)?;
    let fill = num_at(args, 1, "nd_full")?;
    let n = shape_product(&shape);
    Ok(nd_out(&shape, &vec![fill; n]))
}

fn nd_arange(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let start = if args.len() >= 3 {
        num_at(args, 0, "nd_arange")?
    } else {
        0.0
    };
    let stop = if args.len() >= 3 {
        num_at(args, 1, "nd_arange")?
    } else {
        num_at(args, 0, "nd_arange")?
    };
    let step = if args.len() >= 3 {
        num_at(args, 2, "nd_arange")?
    } else if args.len() == 2 {
        num_at(args, 1, "nd_arange")?
    } else {
        1.0
    };
    if step == 0.0 {
        return Err("nd_arange: step must be non-zero".into());
    }
    let mut data = Vec::new();
    let mut x = start;
    if step > 0.0 {
        while x < stop {
            data.push(x);
            x += step;
        }
    } else {
        while x > stop {
            data.push(x);
            x += step;
        }
    }
    let n = data.len();
    Ok(nd_out(&[n], &data))
}

fn nd_from(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("nd_from(data, shape?)")?;
    let data = flat_from_value(v)?;
    let shape_arg = args.get(1).filter(|s| !matches!(s, Value::Undefined | Value::Null));
    let shape = if let Some(s) = shape_arg {
        let shape = parse_shape(s)?;
        if shape_product(&shape) != data.len() {
            return Err("nd_from: shape product must match data length".into());
        }
        shape
    } else if let Value::Array(rows) = v {
        if rows
            .first()
            .map(|x| matches!(x, Value::Array(_)))
            .unwrap_or(false)
        {
            let r = rows.len();
            let c = match rows.first() {
                Some(Value::Array(cells)) => cells.len(),
                _ => 0,
            };
            for row in rows {
                let Value::Array(cells) = row else {
                    return Err("nd_from: jagged matrix".into());
                };
                if cells.len() != c {
                    return Err("nd_from: jagged matrix".into());
                }
            }
            vec![r, c]
        } else {
            vec![data.len()]
        }
    } else {
        vec![data.len()]
    };
    Ok(nd_out(&shape, &data))
}

fn nd_shape(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, _) = nd_at(args, 0, "nd_shape")?;
    Ok(shape_val(&shape))
}

fn nd_size(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, data) = nd_at(args, 0, "nd_size")?;
    Ok(int_out(data.len() as i64))
}

fn nd_reshape(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, data) = nd_at(args, 0, "nd_reshape")?;
    let shape = parse_shape(args.get(1).ok_or("nd_reshape(a, shape)")?)?;
    if shape_product(&shape) != data.len() {
        return Err("nd_reshape: size mismatch".into());
    }
    Ok(nd_out(&shape, &data))
}

fn nd_get(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_get")?;
    let idx = args.get(1).ok_or("nd_get(a, index|indices)")?;
    let flat = match idx {
        Value::Number(n) if *n >= 0 => *n as usize,
        Value::Float(f) if *f >= 0.0 => *f as usize,
        Value::Array(items) => {
            if items.len() != shape.len() {
                return Err("nd_get: index rank must match shape".into());
            }
            let mut stride = 1usize;
            let mut strides = vec![0; shape.len()];
            for i in (0..shape.len()).rev() {
                strides[i] = stride;
                stride *= shape[i];
            }
            let mut flat = 0usize;
            for (i, it) in items.iter().enumerate() {
                let j = num(it)? as usize;
                if j >= shape[i] {
                    return Err("nd_get: index out of bounds".into());
                }
                flat += j * strides[i];
            }
            flat
        }
        _ => return Err("nd_get: bad index".into()),
    };
    data.get(flat)
        .copied()
        .map(float_out)
        .ok_or_else(|| "nd_get: index out of bounds".into())
}

fn nd_set(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, mut data) = nd_at(args, 0, "nd_set")?;
    let idx = args.get(1).ok_or("nd_set(a, index, value)")?;
    let value = num_at(args, 2, "nd_set")?;
    let flat = match idx {
        Value::Number(n) if *n >= 0 => *n as usize,
        Value::Float(f) if *f >= 0.0 => *f as usize,
        Value::Array(items) => {
            if items.len() != shape.len() {
                return Err("nd_set: index rank must match shape".into());
            }
            let mut stride = 1usize;
            let mut strides = vec![0; shape.len()];
            for i in (0..shape.len()).rev() {
                strides[i] = stride;
                stride *= shape[i];
            }
            let mut flat = 0usize;
            for (i, it) in items.iter().enumerate() {
                let j = num(it)? as usize;
                if j >= shape[i] {
                    return Err("nd_set: index out of bounds".into());
                }
                flat += j * strides[i];
            }
            flat
        }
        _ => return Err("nd_set: bad index".into()),
    };
    if flat >= data.len() {
        return Err("nd_set: index out of bounds".into());
    }
    data[flat] = value;
    Ok(nd_out(&shape, &data))
}

fn zip_binop(
    a: &[f64],
    b: &[f64],
    name: &str,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Vec<f64>, String> {
    if a.len() != b.len() {
        return Err(format!("{name}: size mismatch (broadcast later)"));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| f(*x, *y)).collect())
}

fn nd_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a) = nd_at(args, 0, "nd_add")?;
    let (sb, b) = nd_at(args, 1, "nd_add")?;
    if sa != sb {
        return Err("nd_add: shape mismatch".into());
    }
    Ok(nd_out(&sa, &zip_binop(&a, &b, "nd_add", |x, y| x + y)?))
}

fn nd_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a) = nd_at(args, 0, "nd_mul")?;
    let (sb, b) = nd_at(args, 1, "nd_mul")?;
    if sa != sb {
        return Err("nd_mul: shape mismatch".into());
    }
    Ok(nd_out(&sa, &zip_binop(&a, &b, "nd_mul", |x, y| x * y)?))
}

fn nd_scale(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, a) = nd_at(args, 0, "nd_scale")?;
    let s = num_at(args, 1, "nd_scale")?;
    Ok(nd_out(
        &shape,
        &a.iter().map(|x| x * s).collect::<Vec<_>>(),
    ))
}

fn nd_sum(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, a) = nd_at(args, 0, "nd_sum")?;
    Ok(float_out(a.iter().sum()))
}

fn nd_mean(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, a) = nd_at(args, 0, "nd_mean")?;
    if a.is_empty() {
        return Err("nd_mean: empty".into());
    }
    Ok(float_out(a.iter().sum::<f64>() / a.len() as f64))
}

fn nd_dot(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a) = nd_at(args, 0, "nd_dot")?;
    let (sb, b) = nd_at(args, 1, "nd_dot")?;
    if sa.len() != 1 || sb.len() != 1 || sa[0] != sb[0] {
        return Err("nd_dot: expect equal-length 1D vectors".into());
    }
    Ok(float_out(
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum(),
    ))
}

fn nd_matmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a) = nd_at(args, 0, "nd_matmul")?;
    let (sb, b) = nd_at(args, 1, "nd_matmul")?;
    if sa.len() != 2 || sb.len() != 2 {
        return Err("nd_matmul: expect 2D arrays".into());
    }
    let (m, k) = (sa[0], sa[1]);
    let (k2, n) = (sb[0], sb[1]);
    if k != k2 {
        return Err("nd_matmul: inner dims must match".into());
    }
    let mut out = vec![0.0; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut s = 0.0;
            for t in 0..k {
                s += a[i * k + t] * b[t * n + j];
            }
            out[i * n + j] = s;
        }
    }
    Ok(nd_out(&[m, n], &out))
}

/// Gaussian elimination with partial pivoting for square Ax=b (SC1b).
fn nd_solve(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a_flat) = nd_at(args, 0, "nd_solve")?;
    let (sb, b) = nd_at(args, 1, "nd_solve")?;
    if sa.len() != 2 || sa[0] != sa[1] {
        return Err("nd_solve: A must be square 2D".into());
    }
    let n = sa[0];
    if !(sb == [n] || sb == [n, 1]) {
        return Err("nd_solve: b must be length n".into());
    }
    let mut aug = vec![vec![0.0; n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a_flat[i * n + j];
        }
        aug[i][n] = b[i];
    }
    for col in 0..n {
        let mut pivot = col;
        for r in col + 1..n {
            if aug[r][col].abs() > aug[pivot][col].abs() {
                pivot = r;
            }
        }
        if aug[pivot][col].abs() < 1e-12 {
            return Err("nd_solve: singular matrix".into());
        }
        aug.swap(col, pivot);
        let div = aug[col][col];
        for j in col..=n {
            aug[col][j] /= div;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let f = aug[r][col];
            for j in col..=n {
                aug[r][j] -= f * aug[col][j];
            }
        }
    }
    let x: Vec<f64> = (0..n).map(|i| aug[i][n]).collect();
    Ok(nd_out(&[n], &x))
}

fn nd_to_array(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_, data) = nd_at(args, 0, "nd_to_array")?;
    Ok(vector_out(&data))
}

/// P5: bulk vector add (SIMD-friendly tight loop).
fn sci_vadd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_vadd")?;
    let b = vector_at(args, 1, "sci_vadd")?;
    if a.len() != b.len() {
        return Err("sci_vadd: length mismatch".into());
    }
    Ok(vector_out(
        &a.iter()
            .zip(b.iter())
            .map(|(x, y)| x + y)
            .collect::<Vec<_>>(),
    ))
}

fn sci_vmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_vmul")?;
    let b = vector_at(args, 1, "sci_vmul")?;
    if a.len() != b.len() {
        return Err("sci_vmul: length mismatch".into());
    }
    Ok(vector_out(
        &a.iter()
            .zip(b.iter())
            .map(|(x, y)| x * y)
            .collect::<Vec<_>>(),
    ))
}

fn sci_dot(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_dot")?;
    let b = vector_at(args, 1, "sci_dot")?;
    if a.len() != b.len() {
        return Err("sci_dot: length mismatch".into());
    }
    Ok(float_out(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_nd_zeros", "nd_zeros"], nd_zeros);
    bind(&["science_nd_ones", "nd_ones"], nd_ones);
    bind(&["science_nd_full", "nd_full"], nd_full);
    bind(&["science_nd_arange", "nd_arange"], nd_arange);
    bind(&["science_nd_from", "nd_from"], nd_from);
    bind(&["science_nd_shape", "nd_shape"], nd_shape);
    bind(&["science_nd_size", "nd_size"], nd_size);
    bind(&["science_nd_reshape", "nd_reshape"], nd_reshape);
    bind(&["science_nd_get", "nd_get"], nd_get);
    bind(&["science_nd_set", "nd_set"], nd_set);
    bind(&["science_nd_add", "nd_add"], nd_add);
    bind(&["science_nd_mul", "nd_mul"], nd_mul);
    bind(&["science_nd_scale", "nd_scale"], nd_scale);
    bind(&["science_nd_sum", "nd_sum"], nd_sum);
    bind(&["science_nd_mean", "nd_mean"], nd_mean);
    bind(&["science_nd_dot", "nd_dot"], nd_dot);
    bind(&["science_nd_matmul", "nd_matmul"], nd_matmul);
    bind(&["science_nd_solve", "nd_solve"], nd_solve);
    bind(&["science_nd_to_array", "nd_to_array"], nd_to_array);
    bind(&["science_sci_vadd", "sci_vadd"], sci_vadd);
    bind(&["science_sci_vmul", "sci_vmul"], sci_vmul);
    bind(&["science_sci_dot", "sci_dot"], sci_dot);
}
