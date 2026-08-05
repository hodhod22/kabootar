//! Transformer inference-lite — forward pass (SC2k).

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn nd2(seq: usize, d: usize, data: &[f64]) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_nd".into(), Value::Bool(true));
    m.insert(
        "shape".into(),
        Value::Array(vec![int_out(seq as i64), int_out(d as i64)]),
    );
    m.insert("data".into(), vector_out(data));
    m.insert("size".into(), int_out(data.len() as i64));
    m.insert("dtype".into(), Value::String("f64".into()));
    Value::Object(m)
}

fn parse_nd2(v: &Value, name: &str) -> Result<(usize, usize, Vec<f64>), String> {
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
                _ => return Err(format!("{name}: jagged")),
            };
            let mut data = Vec::new();
            for row in rows {
                let Value::Array(cells) = row else {
                    return Err(format!("{name}: jagged"));
                };
                for c in cells {
                    data.push(num(c)?);
                }
            }
            Ok((seq, d, data))
        }
        _ => Err(format!("{name}: bad matrix")),
    }
}

fn linear_rows(
    x: &[f64],
    seq: usize,
    in_dim: usize,
    w: &[f64],
    out_dim: usize,
    b: &[f64],
) -> Vec<f64> {
    let mut out = vec![0.0; seq * out_dim];
    for s in 0..seq {
        for o in 0..out_dim {
            let mut sum = b.get(o).copied().unwrap_or(0.0);
            for i in 0..in_dim {
                sum += x[s * in_dim + i] * w[o * in_dim + i];
            }
            out[s * out_dim + o] = sum;
        }
    }
    out
}

fn relu_vec(v: &[f64]) -> Vec<f64> {
    v.iter().map(|x| x.max(0.0)).collect()
}

fn add_mat(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

/// tf_sinusoidal_pos(dModel, maxLen) → [maxLen, dModel]
fn tf_sinusoidal_pos(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let d_model = num_at(args, 0, "tf_sinusoidal_pos")? as usize;
    let max_len = num_at(args, 1, "tf_sinusoidal_pos")? as usize;
    if d_model == 0 || max_len == 0 {
        return Err("tf_sinusoidal_pos: dims > 0".into());
    }
    let mut data = vec![0.0; max_len * d_model];
    for pos in 0..max_len {
        for i in 0..d_model {
            let angle = pos as f64 / 10000_f64.powf(2.0 * (i / 2) as f64 / d_model as f64);
            data[pos * d_model + i] = if i % 2 == 0 {
                angle.sin()
            } else {
                angle.cos()
            };
        }
    }
    Ok(nd2(max_len, d_model, &data))
}

fn mha_forward(
    q: &[f64],
    k: &[f64],
    v: &[f64],
    seq_q: usize,
    seq_k: usize,
    d: usize,
    n_heads: usize,
) -> Vec<f64> {
    let head_dim = d / n_heads;
    let scale = 1.0 / (head_dim as f64).sqrt();
    let mut out = vec![0.0; seq_q * d];
    for h in 0..n_heads {
        for i in 0..seq_q {
            let mut scores = vec![0.0; seq_k];
            for j in 0..seq_k {
                let mut dot = 0.0;
                for t in 0..head_dim {
                    dot += q[i * d + h * head_dim + t] * k[j * d + h * head_dim + t];
                }
                scores[j] = dot * scale;
            }
            let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ex: Vec<f64> = scores.iter().map(|s| (s - max).exp()).collect();
            let sum: f64 = ex.iter().sum();
            let attn: Vec<f64> = ex.iter().map(|e| e / sum).collect();
            for t in 0..head_dim {
                let mut s = 0.0;
                for j in 0..seq_k {
                    s += attn[j] * v[j * d + h * head_dim + t];
                }
                out[i * d + h * head_dim + t] = s;
            }
        }
    }
    out
}

/// tf_transformer_forward(weights, inputIds, nHeads?) → {logits, hidden}
/// weights: {embed, wq, wk, wv, wo, w1, b1, w2, b2, wout, bout?, pos?}
fn tf_transformer_forward(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let weights = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("tf_transformer_forward(weights, ids, nHeads?)".into()),
    };
    let ids = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("tf_transformer_forward: ids array".into()),
    };
    let n_heads = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(1.0)
        .max(1.0) as usize;

    let embed = weights.get("embed").ok_or("weights.embed")?;
    let (vocab, d_model, embed_data) = parse_embed_table(embed)?;

    let mut x = Vec::with_capacity(ids.len() * d_model);
    for &id in &ids {
        if id >= vocab {
            return Err("tf_transformer_forward: id OOB".into());
        }
        x.extend_from_slice(&embed_data[id * d_model..(id + 1) * d_model]);
    }
    let seq = ids.len();

    if let Some(pos) = weights.get("pos") {
        let (pos_len, pos_d, pos_data) = parse_nd2(pos, "pos")?;
        if pos_d != d_model {
            return Err("tf_transformer_forward: pos d_model mismatch".into());
        }
        for s in 0..seq {
            let pi = s.min(pos_len.saturating_sub(1));
            for d in 0..d_model {
                x[s * d_model + d] += pos_data[pi * d_model + d];
            }
        }
    } else {
        let pos = tf_sinusoidal_pos(
            &[float_out(d_model as f64), float_out(seq as f64)],
            _env,
        )?;
        let (_, _, pos_data) = parse_nd2(&pos, "sin_pos")?;
        for i in 0..x.len() {
            x[i] += pos_data[i];
        }
    }

    let w1 = weights.get("w1").ok_or("weights.w1")?;
    let b1 = vector_at(
        &[weights.get("b1").cloned().unwrap_or(Value::Array(vec![]))],
        0,
        "b1",
    )
    .unwrap_or_else(|_| vec![0.0; d_model]);
    let w2 = weights.get("w2").ok_or("weights.w2")?;
    let b2 = vector_at(
        &[weights.get("b2").cloned().unwrap_or(Value::Array(vec![]))],
        0,
        "b2",
    )
    .unwrap_or_else(|_| vec![0.0; d_model]);
    let wout = weights.get("wout").ok_or("weights.wout")?;
    let bout = vector_at(
        &[weights
            .get("bout")
            .cloned()
            .unwrap_or(Value::Array(vec![]))],
        0,
        "bout",
    )
    .unwrap_or_else(|_| vec![0.0; vocab]);

    let wq_v = match weights.get("wq") {
        Some(v) => vector_at(&[v.clone()], 0, "wq")?,
        None => identity_flat(d_model),
    };
    let wk_v = match weights.get("wk") {
        Some(v) => vector_at(&[v.clone()], 0, "wk")?,
        None => identity_flat(d_model),
    };
    let wv_v = match weights.get("wv") {
        Some(v) => vector_at(&[v.clone()], 0, "wv")?,
        None => identity_flat(d_model),
    };
    let wo_v = match weights.get("wo") {
        Some(v) => vector_at(&[v.clone()], 0, "wo")?,
        None => identity_flat(d_model),
    };
    let w1_v = vector_at(&[w1.clone()], 0, "w1")?;
    let w2_v = vector_at(&[w2.clone()], 0, "w2")?;
    let wout_v = vector_at(&[wout.clone()], 0, "wout")?;
    let ff_dim = if w1_v.len() % d_model == 0 {
        w1_v.len() / d_model
    } else {
        d_model
    };

    let q = linear_rows(&x, seq, d_model, &wq_v, d_model, &vec![0.0; d_model]);
    let k = linear_rows(&x, seq, d_model, &wk_v, d_model, &vec![0.0; d_model]);
    let v = linear_rows(&x, seq, d_model, &wv_v, d_model, &vec![0.0; d_model]);
    let attn = mha_forward(&q, &k, &v, seq, seq, d_model, n_heads);
    let proj = linear_rows(&attn, seq, d_model, &wo_v, d_model, &vec![0.0; d_model]);
    x = add_mat(&x, &proj);

    let h1 = linear_rows(&x, seq, d_model, &w1_v, ff_dim, &b1);
    let h1r = relu_vec(&h1);
    let h2 = linear_rows(&h1r, seq, ff_dim, &w2_v, d_model, &b2);
    x = add_mat(&x, &h2);

    let logits = linear_rows(&x, seq, d_model, &wout_v, vocab, &bout);

    let mut out = HashMap::new();
    out.insert("logits".into(), nd2(seq, vocab, &logits));
    out.insert("hidden".into(), nd2(seq, d_model, &x));
    Ok(Value::Object(out))
}

fn softmax_row(logits: &[f64]) -> Vec<f64> {
    let maxv = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut ex: Vec<f64> = logits.iter().map(|x| (x - maxv).exp()).collect();
    let sum: f64 = ex.iter().sum();
    if sum > 0.0 {
        for e in &mut ex {
            *e /= sum;
        }
    }
    ex
}

fn set_weight_vec(weights: &mut HashMap<String, Value>, key: &str, data: &[f64]) {
    weights.insert(key.into(), vector_out(data));
}

fn set_embed_nd(weights: &mut HashMap<String, Value>, vocab: usize, d: usize, data: &[f64]) {
    weights.insert("embed".into(), nd2(vocab, d, data));
}

/// tf_lm_sgd_step(weights, inputIds, targetIds, lr, nHeads?) → {weights, loss}
/// Last-layer CE + SGD on wout/bout (+ light embed nudge). SC2k train subset.
fn tf_lm_sgd_step(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let Value::Object(mut weights) = args.first().cloned().ok_or("tf_lm_sgd_step(weights,...)")?
    else {
        return Err("tf_lm_sgd_step: weights object".into());
    };
    let ids = match args.get(1) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("tf_lm_sgd_step: inputIds".into()),
    };
    let targets = match args.get(2) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| num(x).map(|n| n as usize))
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("tf_lm_sgd_step: targetIds".into()),
    };
    if ids.is_empty() || ids.len() != targets.len() {
        return Err("tf_lm_sgd_step: ids/targets length".into());
    }
    let lr = num_at(args, 3, "tf_lm_sgd_step")?;
    let n_heads = args.get(4).cloned().unwrap_or(Value::Number(1));

    let fwd = tf_transformer_forward(
        &[
            Value::Object(weights.clone()),
            Value::Array(ids.iter().map(|i| Value::Number(*i as i64)).collect()),
            n_heads,
        ],
        env,
    )?;
    let Value::Object(fwd_m) = fwd else {
        return Err("tf_lm_sgd_step: forward".into());
    };
    let (_, vocab, logits) = parse_nd2(fwd_m.get("logits").ok_or("logits")?, "logits")?;
    let (_, d_model, hidden) = parse_nd2(fwd_m.get("hidden").ok_or("hidden")?, "hidden")?;
    let seq = ids.len();
    if logits.len() != seq * vocab || hidden.len() != seq * d_model {
        return Err("tf_lm_sgd_step: shape".into());
    }

    let wout = weights.get("wout").ok_or("wout")?;
    let mut wout_v = vector_at(&[wout.clone()], 0, "wout")?;
    if wout_v.len() != vocab * d_model {
        return Err("tf_lm_sgd_step: wout size".into());
    }
    let mut bout = match weights.get("bout") {
        Some(v) => vector_at(&[v.clone()], 0, "bout").unwrap_or_else(|_| vec![0.0; vocab]),
        None => vec![0.0; vocab],
    };
    if bout.len() != vocab {
        bout = vec![0.0; vocab];
    }

    let mut loss = 0.0;
    let mut gw = vec![0.0; vocab * d_model];
    let mut gb = vec![0.0; vocab];
    let inv = 1.0 / seq as f64;
    for s in 0..seq {
        let row = &logits[s * vocab..(s + 1) * vocab];
        let probs = softmax_row(row);
        let t = targets[s];
        if t >= vocab {
            return Err("tf_lm_sgd_step: target OOB".into());
        }
        loss -= probs[t].max(1e-12).ln() * inv;
        for o in 0..vocab {
            let mut g = probs[o] * inv;
            if o == t {
                g -= inv;
            }
            gb[o] += g;
            for i in 0..d_model {
                gw[o * d_model + i] += g * hidden[s * d_model + i];
            }
        }
    }

    for i in 0..wout_v.len() {
        wout_v[i] -= lr * gw[i];
    }
    for i in 0..bout.len() {
        bout[i] -= lr * gb[i];
    }
    set_weight_vec(&mut weights, "wout", &wout_v);
    set_weight_vec(&mut weights, "bout", &bout);

    // Light embed nudge along last hidden residual direction (subset, not full BP).
    if let Ok((vocab_e, d_e, mut embed)) = parse_embed_table(weights.get("embed").ok_or("embed")?) {
        if vocab_e == vocab && d_e == d_model {
            for s in 0..seq {
                let id = ids[s];
                for i in 0..d_model {
                    embed[id * d_model + i] -= lr * 0.01 * gw[targets[s] * d_model + i];
                }
            }
            set_embed_nd(&mut weights, vocab, d_model, &embed);
        }
    }

    let mut out = HashMap::new();
    out.insert("weights".into(), Value::Object(weights));
    out.insert("loss".into(), float_out(loss));
    Ok(Value::Object(out))
}

fn identity_flat(d: usize) -> Vec<f64> {
    let mut v = vec![0.0; d * d];
    for i in 0..d {
        v[i * d + i] = 1.0;
    }
    v
}

fn parse_embed_table(v: &Value) -> Result<(usize, usize, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_nd"), Some(Value::Bool(true))) => {
            let shape = match m.get("shape") {
                Some(Value::Array(s)) if s.len() == 2 => {
                    (num(&s[0])? as usize, num(&s[1])? as usize)
                }
                _ => return Err("embed: [vocab,d]".into()),
            };
            let data = match m.get("data") {
                Some(Value::Array(items)) => items.iter().map(num).collect::<Result<Vec<_>, _>>()?,
                _ => return Err("embed missing data".into()),
            };
            Ok((shape.0, shape.1, data))
        }
        Value::Array(rows) => {
            let vocab = rows.len();
            let d = match rows.first() {
                Some(Value::Array(c)) => c.len(),
                _ => return Err("embed jagged".into()),
            };
            let mut data = Vec::new();
            for row in rows {
                let Value::Array(cells) = row else {
                    return Err("embed jagged".into());
                };
                for c in cells {
                    data.push(num(c)?);
                }
            }
            Ok((vocab, d, data))
        }
        _ => Err("embed table".into()),
    }
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_tf_sinusoidal_pos", "tf_sinusoidal_pos"], tf_sinusoidal_pos);
    bind(
        &["science_tf_transformer_forward", "tf_transformer_forward"],
        tf_transformer_forward,
    );
    bind(&["science_tf_lm_sgd_step", "tf_lm_sgd_step"], tf_lm_sgd_step);
}
