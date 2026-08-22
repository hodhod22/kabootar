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
        "shape".into(), Value::from_array(shape.iter().map(|d| Value::Number(*d as i64)).collect()),
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
    Value::from_object(m)
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
        "shape".into(), Value::from_array(shape.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    m.insert("data".into(), vector_out(&data));
    m.insert("size".into(), Value::Number(data.len() as i64));
    Ok(Value::from_object(m))
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
    if let Value::Object(ref mut m_rc) = out {
        let m = Value::object_make_mut(m_rc);
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
    Ok(Value::from_array(vec![
        Value::String("matmul_f64_v1".into()),
        Value::String("linear_f64_v1".into()),
        Value::String("conv2d_f64_v1".into()),
        Value::String("wgpu-compute-matmul_f32_v1".into()),
        Value::String("wgpu-compute-conv2d_f32_v1".into()),
    ]))
}

fn gpu_zeros(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("gpu_zeros(shape)")?)?;
    let n: usize = shape.iter().product();
    Ok(tensor_out(&shape, &vec![0.0; n], default_backend(), "host"))
}

fn gpu_ones(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("gpu_ones(shape)")?)?;
    let n: usize = shape.iter().product();
    Ok(tensor_out(&shape, &vec![1.0; n], default_backend(), "host"))
}

fn gpu_scale(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data, backend, device) = tensor_parts(args.first().ok_or("gpu_scale(t,s)")?)?;
    let s = num(args.get(1).ok_or("gpu_scale: scalar")?)?;
    let out: Vec<f64> = data.iter().map(|x| x * s).collect();
    Ok(tensor_out(&shape, &out, &backend, &device))
}

fn gpu_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, a, ba, da) = tensor_parts(args.first().ok_or("gpu_add(a,b)")?)?;
    let (sb, b, _, db) = tensor_parts(args.get(1).ok_or("gpu_add(a,b)")?)?;
    if sa != sb || a.len() != b.len() {
        return Err("gpu_add: shape mismatch".into());
    }
    let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
    let device = if da == "gpu" || db == "gpu" {
        "gpu"
    } else {
        "host"
    };
    Ok(tensor_out(&sa, &out, &ba, device))
}

fn bias_vec(args: &[Value], cout: usize) -> Result<Vec<f64>, String> {
    match args.get(2) {
        None | Some(Value::Undefined) | Some(Value::Null) => Ok(vec![0.0; cout]),
        Some(v) => {
            if let Ok((_, b, _, _)) = tensor_parts(v) {
                if b.len() != cout {
                    return Err("gpu_conv2d: bias length".into());
                }
                return Ok(b);
            }
            match v {
                Value::Array(items) => {
                    let b: Vec<f64> = items.iter().map(num).collect::<Result<_, _>>()?;
                    if b.len() != cout {
                        return Err("gpu_conv2d: bias length".into());
                    }
                    Ok(b)
                }
                _ => Err("gpu_conv2d: bias".into()),
            }
        }
    }
}

/// Infer-side conv on gpu tensors: input [C,H,W], weight [O,C,Kh,Kw] flat shapes.
fn gpu_conv2d(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (sa, a, _, da) = tensor_parts(args.first().ok_or("gpu_conv2d")?)?;
    let (sw, w, _, dw) = tensor_parts(args.get(1).ok_or("gpu_conv2d")?)?;
    if sa.len() != 3 || sw.len() != 4 {
        return Err("gpu_conv2d: input [C,H,W], weight [O,C,Kh,Kw]".into());
    }
    let (cin, hin, win) = (sa[0], sa[1], sa[2]);
    let (cout, c2, kh, kw) = (sw[0], sw[1], sw[2], sw[3]);
    if c2 != cin {
        return Err("gpu_conv2d: channel mismatch".into());
    }
    let bias = bias_vec(args, cout)?;
    let on_device = da == "gpu" || dw == "gpu";
    let stride = match args.get(3) {
        Some(Value::Number(n)) => *n,
        _ => 1,
    };
    let pad = match args.get(4) {
        Some(Value::Number(n)) => *n,
        _ => 0,
    };

    if on_device && stride == 1 && pad == 0 {
        if let Some((out, shape, kid)) =
            super::gpu_compute::try_conv2d_compute(cin, hin, win, cout, kh, kw, &a, &w, &bias)
        {
            return Ok(tensor_out_kernel(
                &shape,
                &out,
                "wgpu-compute",
                "gpu",
                Some(kid),
            ));
        }
    }

    // Reuse ml_conv2d via nd objects
    let mut xin = HashMap::new();
    xin.insert("__kab_nd".into(), Value::Bool(true));
    xin.insert(
        "shape".into(), Value::from_array(sa.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    xin.insert("data".into(), vector_out(&a));
    let mut win_o = HashMap::new();
    win_o.insert("__kab_nd".into(), Value::Bool(true));
    win_o.insert(
        "shape".into(), Value::from_array(sw.iter().map(|d| Value::Number(*d as i64)).collect()),
    );
    win_o.insert("data".into(), vector_out(&w));
    let bias_v = vector_out(&bias);
    let out = super::nn_layers::ml_conv2d(
        &[
            Value::from_object(xin), Value::from_object(win_o),
            bias_v,
            args.get(3).cloned().unwrap_or(Value::Number(1)),
            args.get(4).cloned().unwrap_or(Value::Number(0)),
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
    let device = if on_device { "gpu" } else { "host" };
    let (backend, kernel) = if on_device && crate::runtime::render::gpu3d::gpu3d_available() {
        ("device-conv-cpu-fallback", Some("conv2d_f64_v1"))
    } else if on_device {
        ("cpu-emulated-device", Some("conv2d_f64_v1_cpu"))
    } else {
        ("cpu", None)
    };
    Ok(tensor_out_kernel(&shape, &data, backend, device, kernel))
}

fn gpu_conv2d_kernel(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mut out = gpu_conv2d(args, env)?;
    if let Value::Object(ref mut m_rc) = out {
        let m = Value::object_make_mut(m_rc);
        if !m.contains_key("kernel") {
            m.insert("kernel".into(), Value::String("conv2d_f64_v1_cpu".into()));
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
        "kernels".into(), Value::from_array(vec![
            Value::String("matmul_f64_v1".into()),
            Value::String("linear_f64_v1".into()),
            Value::String("conv2d_f64_v1".into()),
            Value::String("wgpu-compute-matmul_f32_v1".into()),
            Value::String("wgpu-compute-conv2d_f32_v1".into()),
        ]),
    );
    Ok(Value::from_object(m))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_gpu_tensor_from", "gpu_tensor_from"], gpu_tensor_from);
    bind(&["science_gpu_tensor_to_nd", "gpu_tensor_to_nd"], gpu_tensor_to_nd);
    bind(&["science_gpu_zeros", "gpu_zeros"], gpu_zeros);
    bind(&["science_gpu_ones", "gpu_ones"], gpu_ones);
    bind(&["science_gpu_scale", "gpu_scale"], gpu_scale);
    bind(&["science_gpu_add", "gpu_add"], gpu_add);
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
    bind(&["science_gpu_conv2d_kernel", "gpu_conv2d_kernel"], gpu_conv2d_kernel);
    bind(&["science_gpu_tensor_info", "gpu_tensor_info"], gpu_tensor_info);
}
