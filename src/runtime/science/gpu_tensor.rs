//! GPU tensor staging + device sync / train-infer path (SC4b/SC4e).

use super::helpers::{num, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

const MARK: &str = "__kab_gpu_tensor";

fn tensor_out(shape: &[usize], data: &[f64], backend: &str, device: &str) -> Value {
    tensor_out_kernel(shape, data, backend, device, None)
}

fn tensor_out_kernel(
    shape: &[usize],
    data: &[f64],
    backend: &str,
    device: &str,
    kernel: Option<&str>,
) -> Value {
    let mut m = HashMap::new();
    m.insert(MARK.into(), Value::Bool(true));
    m.insert(
        "shape".into(),
        Value::Array(shape.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    m.insert("data".into(), vector_out(data));
    m.insert("backend".into(), Value::String(backend.into()));
    m.insert("device".into(), Value::String(device.into()));
    m.insert(
        "gpu".into(),
        Value::Bool(crate::runtime::render::gpu3d::gpu3d_available()),
    );
    if let Some(k) = kernel {
        m.insert("kernel".into(), Value::String(k.into()));
    }
    Value::Object(m)
}

fn default_backend() -> &'static str {
    if crate::runtime::render::gpu3d::gpu3d_available() {
        "wgpu-ready"
    } else {
        "cpu"
    }
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

fn tensor_parts(v: &Value) -> Result<(Vec<usize>, Vec<f64>, String, String), String> {
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
    let backend = match m.get("backend") {
        Some(Value::String(s)) => s.clone(),
        _ => default_backend().into(),
    };
    let device = match m.get("device") {
        Some(Value::String(s)) => s.clone(),
        _ => "host".into(),
    };
    Ok((shape, data, backend, device))
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
    Ok(tensor_out(&shape, &data, default_backend(), "host"))
}

fn gpu_to_device(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data, _, _) = tensor_parts(args.first().ok_or("gpu_to_device(t)")?)?;
    let backend = if crate::runtime::render::gpu3d::gpu3d_available() {
        "device-staging"
    } else {
        "cpu-emulated-device"
    };
    Ok(tensor_out(&shape, &data, backend, "gpu"))
}

fn gpu_to_host(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data, backend, _) = tensor_parts(args.first().ok_or("gpu_to_host(t)")?)?;
    Ok(tensor_out(&shape, &data, &backend, "host"))
}

fn gpu_tensor_to_nd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data, _, _) = tensor_parts(args.first().ok_or("gpu_tensor_to_nd(t)")?)?;
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

fn matmul_cpu(sa: &[usize], a: &[f64], sb: &[usize], b: &[f64]) -> Result<(usize, usize, Vec<f64>), String> {
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
    Ok((m, n, out))
}

fn gpu_matmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a, _, da) = tensor_parts(args.first().ok_or("gpu_matmul(a,b)")?)?;
    let (sb, b, _, db) = tensor_parts(args.get(1).ok_or("gpu_matmul(a,b)")?)?;
    if sa.len() != 2 || sb.len() != 2 || sa[1] != sb[0] {
        return Err("gpu_matmul: expect 2D compatible shapes".into());
    }
    let (m, k, n) = (sa[0], sa[1], sb[1]);
    let on_device = da == "gpu" || db == "gpu";

    if on_device {
        if let Some((out, kid)) = super::gpu_compute::try_matmul_compute(m, k, n, &a, &b) {
            return Ok(tensor_out_kernel(
                &[m, n],
                &out,
                "wgpu-compute",
                "gpu",
                Some(kid),
            ));
        }
    }

    let (m, n, out) = matmul_cpu(&sa, &a, &sb, &b)?;
    let gpu = crate::runtime::render::gpu3d::gpu3d_available();
    let (backend, kernel) = if on_device && gpu {
        ("device-kernel-cpu-exec", Some("matmul_f64_v1"))
    } else if on_device {
        ("cpu-emulated-device", Some("matmul_f64_v1_cpu"))
    } else {
        ("cpu", None)
    };
    let device = if on_device { "gpu" } else { "host" };
    Ok(tensor_out_kernel(&[m, n], &out, backend, device, kernel))
}

/// Explicit kernel entry for train/infer path (SC4b subset).
fn gpu_matmul_kernel(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut out = gpu_matmul(args, env)?;
    if let Value::Object(ref mut m) = out {
        if !m.contains_key("kernel") {
            m.insert("kernel".into(), Value::String("matmul_f64_v1_cpu".into()));
        }
        m.insert(
            "kernel_dispatch".into(),
            Value::String(if crate::runtime::render::gpu3d::gpu3d_available() {
                "wgpu-ready".into()
            } else {
                "cpu-fallback".into()
            }),
        );
    }
    Ok(out)
}

fn gpu_available_kernels(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    Ok(Value::Array(vec![
        Value::String("matmul_f64_v1".into()),
        Value::String("linear_f64_v1".into()),
        Value::String("conv2d_f64_v1".into()),
    ]))
}

/// Infer-side conv on gpu tensors: input [C,H,W], weight [O,C,Kh,Kw] flat shapes.
fn gpu_conv2d(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (sa, a, _, da) = tensor_parts(args.first().ok_or("gpu_conv2d")?)?;
    let (sw, w, _, _) = tensor_parts(args.get(1).ok_or("gpu_conv2d")?)?;
    if sa.len() != 3 || sw.len() != 4 {
        return Err("gpu_conv2d: input [C,H,W], weight [O,C,Kh,Kw]".into());
    }
    // Reuse ml_conv2d via nd objects
    let mut xin = HashMap::new();
    xin.insert("__kab_nd".into(), Value::Bool(true));
    xin.insert(
        "shape".into(),
        Value::Array(sa.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    xin.insert("data".into(), vector_out(&a));
    let mut win = HashMap::new();
    win.insert("__kab_nd".into(), Value::Bool(true));
    win.insert(
        "shape".into(),
        Value::Array(sw.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    win.insert("data".into(), vector_out(&w));
    let bias = args.get(2).cloned().unwrap_or(Value::Undefined);
    let stride = args.get(3).cloned().unwrap_or(Value::Number(1));
    let pad = args.get(4).cloned().unwrap_or(Value::Number(0));
    let out = super::nn_layers::ml_conv2d(
        &[
            Value::Object(xin),
            Value::Object(win),
            bias,
            stride,
            pad,
        ],
        env,
    )?;
    let Value::Object(om) = out else {
        return Err("gpu_conv2d: internal".into());
    };
    let shape = parse_shape(om.get("shape").ok_or("shape")?)?;
    let data = match om.get("data") {
        Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
        _ => return Err("gpu_conv2d: data".into()),
    };
    let device = if da == "gpu" { "gpu" } else { "host" };
    let backend = if device == "gpu" {
        "device-conv-cpu-fallback"
    } else {
        "cpu"
    };
    Ok(tensor_out(&shape, &data, backend, device))
}

/// Explicit train/infer matmul step on device tensors: y = W @ x (+ optional bias).
fn gpu_linear(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sw, w, _, dw) = tensor_parts(args.first().ok_or("gpu_linear(W,x,b?)")?)?;
    let (sx, x, _, dx) = tensor_parts(args.get(1).ok_or("gpu_linear")?)?;
    // W [out,in], x [in] or [in,1]
    let (out_dim, in_dim) = if sw.len() == 2 {
        (sw[0], sw[1])
    } else {
        return Err("gpu_linear: W must be 2D".into());
    };
    let xflat = if sx == [in_dim] || sx == [in_dim, 1] {
        x
    } else {
        return Err("gpu_linear: x shape".into());
    };
    let mut y = vec![0.0; out_dim];
    for o in 0..out_dim {
        let mut s = 0.0;
        for i in 0..in_dim {
            s += w[o * in_dim + i] * xflat[i];
        }
        y[o] = s;
    }
    if let Some(barg) = args.get(2).filter(|v| !matches!(v, Value::Undefined | Value::Null)) {
        let (_, b, _, _) = tensor_parts(barg)?;
        if b.len() != out_dim {
            return Err("gpu_linear: bias length".into());
        }
        for i in 0..out_dim {
            y[i] += b[i];
        }
    }
    let on_device = dw == "gpu" || dx == "gpu";
    Ok(tensor_out(
        &[out_dim],
        &y,
        if on_device {
            "device-linear-cpu-fallback"
        } else {
            "cpu"
        },
        if on_device { "gpu" } else { "host" },
    ))
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
    m.insert(
        "train_infer".into(),
        Value::String("gpu_to_device + gpu_matmul_kernel/gpu_linear/gpu_conv2d + gpu_to_host".into()),
    );
    m.insert(
        "kernels".into(),
        Value::Array(vec![
            Value::String("matmul_f64_v1".into()),
            Value::String("linear_f64_v1".into()),
            Value::String("conv2d_f64_v1".into()),
        ]),
    );
    Ok(Value::Object(m))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_gpu_tensor_from", "gpu_tensor_from"], gpu_tensor_from);
    bind(&["science_gpu_tensor_to_nd", "gpu_tensor_to_nd"], gpu_tensor_to_nd);
    bind(&["science_gpu_to_device", "gpu_to_device"], gpu_to_device);
    bind(&["science_gpu_to_host", "gpu_to_host"], gpu_to_host);
    bind(&["science_gpu_matmul", "gpu_matmul"], gpu_matmul);
    bind(&["science_gpu_matmul_kernel", "gpu_matmul_kernel"], gpu_matmul_kernel);
    bind(
        &["science_gpu_available_kernels", "gpu_available_kernels"],
        gpu_available_kernels,
    );
    bind(&["science_gpu_linear", "gpu_linear"], gpu_linear);
    bind(&["science_gpu_conv2d", "gpu_conv2d"], gpu_conv2d);
    bind(&["science_gpu_tensor_info", "gpu_tensor_info"], gpu_tensor_info);
}
