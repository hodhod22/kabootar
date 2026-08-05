//! GPU tensor staging subset (SC4b) — handles + matmul with GPU path metadata.

use super::helpers::{num, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

const MARK: &str = "__kab_gpu_tensor";

fn tensor_out(shape: &[usize], data: &[f64], backend: &str) -> Value {
    let mut m = HashMap::new();
    m.insert(MARK.into(), Value::Bool(true));
    m.insert(
        "shape".into(),
        Value::Array(shape.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    m.insert("data".into(), vector_out(data));
    m.insert("backend".into(), Value::String(backend.into()));
    m.insert(
        "gpu".into(),
        Value::Bool(crate::runtime::render::gpu3d::gpu3d_available()),
    );
    Value::Object(m)
}

fn parse_shape(v: &Value) -> Result<Vec<usize>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| {
                let n = num(x)?;
                if n < 0.0 || n.fract() != 0.0 {
                    return Err("bad shape".into());
                }
                Ok(n as usize)
            })
            .collect(),
        Value::Number(n) if *n >= 0 => Ok(vec![*n as usize]),
        _ => Err("gpu_tensor shape".into()),
    }
}

fn tensor_parts(v: &Value) -> Result<(Vec<usize>, Vec<f64>), String> {
    let Value::Object(m) = v else {
        return Err("expected gpu tensor".into());
    };
    if !matches!(m.get(MARK), Some(Value::Bool(true))) {
        return Err("expected gpu tensor".into());
    }
    let shape = parse_shape(m.get("shape").ok_or("missing shape")?)?;
    let data = match m.get("data") {
        Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
        _ => return Err("gpu tensor missing data".into()),
    };
    Ok((shape, data))
}

fn gpu_tensor_from(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "gpu_tensor_from")?;
    let shape = if let Some(s) = args.get(1) {
        parse_shape(s)?
    } else {
        vec![data.len()]
    };
    let n: usize = shape.iter().product();
    if n != data.len() {
        return Err("gpu_tensor_from: size mismatch".into());
    }
    let backend = if crate::runtime::render::gpu3d::gpu3d_available() {
        "wgpu-ready"
    } else {
        "cpu"
    };
    Ok(tensor_out(&shape, &data, backend))
}

fn gpu_tensor_to_nd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = tensor_parts(args.first().ok_or("gpu_tensor_to_nd(t)")?)?;
    let mut m = HashMap::new();
    m.insert("__kab_nd".into(), Value::Bool(true));
    m.insert(
        "shape".into(),
        Value::Array(shape.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    m.insert("data".into(), vector_out(&data));
    m.insert("size".into(), Value::Number(data.len() as i64));
    Ok(Value::Object(m))
}

fn gpu_matmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a) = tensor_parts(args.first().ok_or("gpu_matmul(a,b)")?)?;
    let (sb, b) = tensor_parts(args.get(1).ok_or("gpu_matmul(a,b)")?)?;
    if sa.len() != 2 || sb.len() != 2 || sa[1] != sb[0] {
        return Err("gpu_matmul: expect 2D compatible shapes".into());
    }
    let (m, k, n) = (sa[0], sa[1], sb[1]);
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
    // True GPU compute kernels are future work; path reports adapter readiness.
    let backend = if crate::runtime::render::gpu3d::gpu3d_available() {
        "cpu-on-wgpu-host"
    } else {
        "cpu"
    };
    Ok(tensor_out(&[m, n], &out, backend))
}

fn gpu_tensor_info(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let mut m = HashMap::new();
    m.insert(
        "available".into(),
        Value::Bool(crate::runtime::render::gpu3d::gpu3d_available()),
    );
    m.insert(
        "path".into(),
        Value::String(crate::runtime::render::gpu3d::info_line().into()),
    );
    Ok(Value::Object(m))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_gpu_tensor_from", "gpu_tensor_from"], gpu_tensor_from);
    bind(&["science_gpu_tensor_to_nd", "gpu_tensor_to_nd"], gpu_tensor_to_nd);
    bind(&["science_gpu_matmul", "gpu_matmul"], gpu_matmul);
    bind(&["science_gpu_tensor_info", "gpu_tensor_info"], gpu_tensor_info);
}
