//! NN layers: Conv2d, MaxPool, Embedding, MultiheadAttention-lite (SC2i).

use super::helpers::{int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn shape3(v: &Value, name: &str) -> Result<(usize, usize, usize, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 3 => {
                    (
                        num(&s[0])? as usize,
                        num(&s[1])? as usize,
                        num(&s[2])? as usize,
                    )
                }
                _ => return Err(format!("{name}: expect shape [C,H,W]")),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err(format!("{name}: missing data")),
            };
            Ok((shape.0, shape.1, shape.2, data))
        }
        Value::Array(items) => {
            // Nested [C][H][W]
            let c = items.len();
            if c == 0 {
                return Err(format!("{name}: empty"));
            }
            let Value::Array(plane0) = &items[0] else {
                return Err(format!("{name}: expect [C][H][W]"));
            };
            let h = plane0.len();
            let Value::Array(row0) = &plane0[0] else {
                return Err(format!("{name}: expect [C][H][W]"));
            };
            let w = row0.len();
            let mut data = Vec::with_capacity(c * h * w);
            for plane in items.iter() {
                let Value::Array(rows) = plane else {
                    return Err(format!("{name}: jagged"));
                };
                if rows.len() != h {
                    return Err(format!("{name}: jagged H"));
                }
                for row in rows.iter() {
                    let Value::Array(cells) = row else {
                        return Err(format!("{name}: jagged W"));
                    };
                    if cells.len() != w {
                        return Err(format!("{name}: jagged W"));
                    }
                    for cell in cells.iter() {
                        data.push(num(cell)?);
                    }
                }
            }
            Ok((c, h, w, data))
        }
        _ => Err(format!("{name}: bad input")),
    }
}

fn nd3(c: usize, h: usize, w: usize, data: &[f64]) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_nd".into(), Value::Bool(true));
    m.insert(
        "shape".into(), Value::from_array(vec![
            int_out(c as i64),
            int_out(h as i64),
            int_out(w as i64),
        ]),
    );
    m.insert("data".into(), vector_out(data));
    m.insert("size".into(), int_out(data.len() as i64));
    m.insert("dtype".into(), Value::String("f64".into()));
    Value::from_object(m)
}

fn idx3(_c: usize, h: usize, w: usize, ci: usize, hi: usize, wi: usize) -> usize {
    ci * h * w + hi * w + wi
}

/// ml_conv2d(input[C,H,W], weight[O,C,Kh,Kw] flat or nested, bias?, stride?, pad?)
pub(crate) fn ml_conv2d(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (cin, hin, win, x) = shape3(args.first().ok_or("ml_conv2d")?, "ml_conv2d")?;
    let weight = args.get(1).ok_or("ml_conv2d: weight")?;
    let (cout, cin_w, kh, kw, wdata) = parse_weight4(weight)?;
    if cin_w != cin {
        return Err("ml_conv2d: in_channels mismatch".into());
    }
    let bias = if let Some(b) = args.get(2).filter(|v| !matches!(v, Value::Undefined | Value::Null)) {
        vector_at(&[b.clone()], 0, "bias")?
    } else {
        vec![0.0; cout]
    };
    if bias.len() != cout {
        return Err("ml_conv2d: bias length".into());
    }
    let stride = args
        .get(3)
        .and_then(|v| num(v).ok())
        .unwrap_or(1.0)
        .max(1.0) as usize;
    let pad = args
        .get(4)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0)
        .max(0.0) as usize;
    let hout = (hin + 2 * pad - kh) / stride + 1;
    let wout = (win + 2 * pad - kw) / stride + 1;
    if hout == 0 || wout == 0 {
        return Err("ml_conv2d: output spatial size 0".into());
    }
    let mut out = vec![0.0; cout * hout * wout];
    for oc in 0..cout {
        for oh in 0..hout {
            for ow in 0..wout {
                let mut s = bias[oc];
                for ic in 0..cin {
                    for kh_i in 0..kh {
                        for kw_i in 0..kw {
                            let ih = oh * stride + kh_i;
                            let iw = ow * stride + kw_i;
                            if ih < pad || iw < pad {
                                continue;
                            }
                            let ih2 = ih - pad;
                            let iw2 = iw - pad;
                            if ih2 >= hin || iw2 >= win {
                                continue;
                            }
                            let xv = x[idx3(cin, hin, win, ic, ih2, iw2)];
                            let wv = wdata[oc * (cin * kh * kw) + ic * (kh * kw) + kh_i * kw + kw_i];
                            s += xv * wv;
                        }
                    }
                }
                out[idx3(cout, hout, wout, oc, oh, ow)] = s;
            }
        }
    }
    Ok(nd3(cout, hout, wout, &out))
}

fn parse_weight4(v: &Value) -> Result<(usize, usize, usize, usize, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 4 => (
                    num(&s[0])? as usize,
                    num(&s[1])? as usize,
                    num(&s[2])? as usize,
                    num(&s[3])? as usize,
                ),
                _ => return Err("weight: expect [O,C,Kh,Kw]".into()),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err("weight missing data".into()),
            };
            Ok((shape.0, shape.1, shape.2, shape.3, data))
        }
        Value::Array(items) => {
            // Flat with explicit dims not available — treat as nested 4D
            let o = items.len();
            let Value::Array(c0) = &items[0] else {
                return Err("weight: nested [O][C][Kh][Kw]".into());
            };
            let c = c0.len();
            let Value::Array(kh0) = &c0[0] else {
                return Err("weight: nested".into());
            };
            let kh = kh0.len();
            let Value::Array(kw0) = &kh0[0] else {
                return Err("weight: nested".into());
            };
            let kw = kw0.len();
            let mut data = Vec::new();
            for oc in items.iter() {
                let Value::Array(ics) = oc else {
                    return Err("weight jagged".into());
                };
                for ic in ics.iter() {
                    let Value::Array(rows) = ic else {
                        return Err("weight jagged".into());
                    };
                    for row in rows.iter() {
                        let Value::Array(cells) = row else {
                            return Err("weight jagged".into());
                        };
                        for cell in cells.iter() {
                            data.push(num(cell)?);
                        }
                    }
                }
            }
            Ok((o, c, kh, kw, data))
        }
        _ => Err("weight: bad format".into()),
    }
}

/// ml_maxpool2d(input[C,H,W], ksize, stride?)
fn ml_maxpool2d(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (c, h, w, x) = shape3(args.first().ok_or("ml_maxpool2d")?, "ml_maxpool2d")?;
    let k = num_at(args, 1, "ml_maxpool2d")? as usize;
    if k == 0 {
        return Err("ml_maxpool2d: ksize > 0".into());
    }
    let stride = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(k as f64)
        .max(1.0) as usize;
    let hout = (h - k) / stride + 1;
    let wout = (w - k) / stride + 1;
    let mut out = vec![0.0; c * hout * wout];
    for ci in 0..c {
        for oh in 0..hout {
            for ow in 0..wout {
                let mut m = f64::NEG_INFINITY;
                for kh in 0..k {
                    for kw in 0..k {
                        let ih = oh * stride + kh;
                        let iw = ow * stride + kw;
                        let v = x[idx3(c, h, w, ci, ih, iw)];
                        if v > m {
                            m = v;
                        }
                    }
                }
                out[idx3(c, hout, wout, ci, oh, ow)] = m;
            }
        }
    }
    Ok(nd3(c, hout, wout, &out))
}

/// ml_embedding(table[vocab*dim] or [vocab][dim], indices) → flat [n*dim] as nd [n,dim]
fn ml_embedding(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let table = args.first().ok_or("ml_embedding(table, indices)")?;
    let (vocab, dim, data) = match table {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 2 => {
                    (num(&s[0])? as usize, num(&s[1])? as usize)
                }
                _ => return Err("embedding table: [vocab, dim]".into()),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err("embedding missing data".into()),
            };
            (shape.0, shape.1, data)
        }
        Value::Array(rows) => {
            let vocab = rows.len();
            let dim = match rows.first() {
                Some(Value::Array(c)) => c.len(),
                _ => return Err("embedding: rows of vectors".into()),
            };
            let mut data = Vec::new();
            for row in rows.iter() {
                let Value::Array(cells) = row else {
                    return Err("embedding jagged".into());
                };
                if cells.len() != dim {
                    return Err("embedding jagged".into());
                }
                for c in cells.iter() {
                    data.push(num(c)?);
                }
            }
            (vocab, dim, data)
        }
        _ => return Err("embedding table".into()),
    };
    let indices = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("ml_embedding: indices array".into()),
    };
    let mut out = Vec::with_capacity(indices.len() * dim);
    for &i in &indices {
        if i >= vocab {
            return Err("ml_embedding: index OOB".into());
        }
        out.extend_from_slice(&data[i * dim..(i + 1) * dim]);
    }
    let mut m = HashMap::new();
    m.insert("__kab_nd".into(), Value::Bool(true));
    m.insert(
        "shape".into(), Value::from_array(vec![int_out(indices.len() as i64), int_out(dim as i64)]),
    );
    m.insert("data".into(), vector_out(&out));
    m.insert("size".into(), int_out(out.len() as i64));
    Ok(Value::from_object(m))
}

fn softmax_row(v: &[f64]) -> Vec<f64> {
    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ex: Vec<f64> = v.iter().map(|x| (x - max).exp()).collect();
    let s: f64 = ex.iter().sum();
    ex.into_iter().map(|x| x / s).collect()
}

/// ml_mha(q, k, v, n_heads?) — seq×d flat arrays or nd [seq,d]; returns [seq,d]
fn ml_mha(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (seq_q, d, q) = matrix2(args.first().ok_or("ml_mha(q,k,v)")?, "q")?;
    let (seq_k, d_k, k) = matrix2(args.get(1).ok_or("ml_mha")?, "k")?;
    let (seq_v, d_v, v) = matrix2(args.get(2).ok_or("ml_mha")?, "v")?;
    if d != d_k || d != d_v || seq_k != seq_v {
        return Err("ml_mha: dim/seq mismatch".into());
    }
    let n_heads = args
        .get(3)
        .and_then(|x| num(x).ok())
        .unwrap_or(1.0)
        .max(1.0) as usize;
    if d % n_heads != 0 {
        return Err("ml_mha: d must divide n_heads".into());
    }
    let head_dim = d / n_heads;
    let scale = 1.0 / (head_dim as f64).sqrt();
    let mut out = vec![0.0; seq_q * d];
    for h in 0..n_heads {
        for i in 0..seq_q {
            let mut scores = vec![0.0; seq_k];
            for j in 0..seq_k {
                let mut dot = 0.0;
                for t in 0..head_dim {
                    let qi = q[i * d + h * head_dim + t];
                    let kj = k[j * d + h * head_dim + t];
                    dot += qi * kj;
                }
                scores[j] = dot * scale;
            }
            let attn = softmax_row(&scores);
            for t in 0..head_dim {
                let mut s = 0.0;
                for j in 0..seq_k {
                    s += attn[j] * v[j * d + h * head_dim + t];
                }
                out[i * d + h * head_dim + t] = s;
            }
        }
    }
    let mut m = HashMap::new();
    m.insert("__kab_nd".into(), Value::Bool(true));
    m.insert(
        "shape".into(), Value::from_array(vec![int_out(seq_q as i64), int_out(d as i64)]),
    );
    m.insert("data".into(), vector_out(&out));
    m.insert("size".into(), int_out(out.len() as i64));
    Ok(Value::from_object(m))
}

fn matrix2(v: &Value, name: &str) -> Result<(usize, usize, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 2 => {
                    (num(&s[0])? as usize, num(&s[1])? as usize)
                }
                _ => return Err(format!("{name}: expect [seq,d]")),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err(format!("{name}: missing data")),
            };
            Ok((shape.0, shape.1, data))
        }
        Value::Array(rows) => {
            let seq = rows.len();
            let d = match rows.first() {
                Some(Value::Array(c)) => c.len(),
                Some(_) => {
                    // flat vector treated as 1×d
                    let data = vector_at(&[v.clone()], 0, name)?;
                    return Ok((1, data.len(), data));
                }
                None => return Err(format!("{name}: empty")),
            };
            let mut data = Vec::new();
            for row in rows.iter() {
                let Value::Array(cells) = row else {
                    return Err(format!("{name}: jagged"));
                };
                if cells.len() != d {
                    return Err(format!("{name}: jagged"));
                }
                for c in cells.iter() {
                    data.push(num(c)?);
                }
            }
            Ok((seq, d, data))
        }
        _ => Err(format!("{name}: bad matrix")),
    }
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ml_conv2d", "ml_conv2d"], ml_conv2d);
    bind(&["science_ml_maxpool2d", "ml_maxpool2d"], ml_maxpool2d);
    bind(&["science_ml_embedding", "ml_embedding"], ml_embedding);
    bind(&["science_ml_mha", "ml_mha"], ml_mha);
}
