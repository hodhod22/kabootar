//! Autograd tape (SC2c/SC2f) — dense/relu/mse + matmul/softmax/CE/add/mul + no_grad.

use super::helpers::{num, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone)]
enum Node {
    Leaf { id: u64, value: Vec<f64> },
    Relu { id: u64, parent: u64, value: Vec<f64> },
    Dense {
        id: u64,
        w: u64,
        x: u64,
        b: u64,
        value: Vec<f64>,
        in_dim: usize,
        out_dim: usize,
    },
    Matmul {
        id: u64,
        a: u64,
        b: u64,
        value: Vec<f64>,
        m: usize,
        k: usize,
        n: usize,
    },
    Softmax { id: u64, parent: u64, value: Vec<f64> },
    Add { id: u64, left: u64, right: u64, value: Vec<f64> },
    Mul { id: u64, left: u64, right: u64, value: Vec<f64> },
    Mse { id: u64, pred: u64, target: Vec<f64>, value: f64 },
    Ce { id: u64, pred: u64, target: Vec<f64>, value: f64 },
}

struct Tape {
    next_id: u64,
    nodes: HashMap<u64, Node>,
    grads: HashMap<u64, Vec<f64>>,
    grad_enabled: bool,
}

impl Default for Tape {
    fn default() -> Self {
        Self {
            next_id: 0,
            nodes: HashMap::new(),
            grads: HashMap::new(),
            grad_enabled: true,
        }
    }
}

thread_local! {
    static TAPE: RefCell<Tape> = RefCell::new(Tape::default());
}

fn with_tape<R>(f: impl FnOnce(&mut Tape) -> R) -> R {
    TAPE.with(|t| f(&mut t.borrow_mut()))
}

fn tensor_out(id: u64, value: &[f64]) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_ag".into(), Value::Bool(true));
    m.insert("id".into(), Value::Number(id as i64));
    m.insert("data".into(), vector_out(value));
    Value::Object(m)
}

fn tensor_id(v: &Value) -> Result<u64, String> {
    match v {
        Value::Object(m) if matches!(m.get("__kab_ag"), Some(Value::Bool(true))) => {
            match m.get("id") {
                Some(Value::Number(n)) if *n >= 0 => Ok(*n as u64),
                _ => Err("autograd tensor missing id".into()),
            }
        }
        _ => Err("expected autograd tensor".into()),
    }
}

fn node_value(t: &Tape, id: u64) -> Result<Vec<f64>, String> {
    match t.nodes.get(&id) {
        Some(Node::Leaf { value, .. })
        | Some(Node::Relu { value, .. })
        | Some(Node::Dense { value, .. })
        | Some(Node::Matmul { value, .. })
        | Some(Node::Softmax { value, .. })
        | Some(Node::Add { value, .. })
        | Some(Node::Mul { value, .. }) => Ok(value.clone()),
        Some(Node::Mse { value, .. }) | Some(Node::Ce { value, .. }) => Ok(vec![*value]),
        None => Err("unknown autograd id".into()),
    }
}

fn push_leaf(t: &mut Tape, data: Vec<f64>) -> u64 {
    let id = t.next_id;
    t.next_id += 1;
    t.nodes.insert(
        id,
        Node::Leaf {
            id,
            value: data,
        },
    );
    id
}

fn ag_tensor(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "ag_tensor")?;
    with_tape(|t| {
        let id = push_leaf(t, data.clone());
        Ok(tensor_out(id, &data))
    })
}

fn ag_no_grad(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let enabled = match args.first() {
        Some(Value::Bool(b)) => !*b,
        Some(Value::Number(n)) => *n == 0,
        None => false,
        _ => false,
    };
    with_tape(|t| {
        t.grad_enabled = enabled;
        Ok(Value::Bool(t.grad_enabled))
    })
}

fn ag_enable_grad(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let on = match args.first() {
        Some(Value::Bool(b)) => *b,
        None => true,
        _ => true,
    };
    with_tape(|t| {
        t.grad_enabled = on;
        Ok(Value::Bool(on))
    })
}

fn detach_or_track(t: &mut Tape, parent: u64, out: Vec<f64>, make: impl FnOnce(u64) -> Node) -> Value {
    if !t.grad_enabled {
        let id = push_leaf(t, out.clone());
        return tensor_out(id, &out);
    }
    let id = t.next_id;
    t.next_id += 1;
    let _ = parent;
    t.nodes.insert(id, make(id));
    tensor_out(id, &out)
}

fn ag_relu(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_relu(t)")?)?;
    with_tape(|t| {
        let parent_val = node_value(t, parent)?;
        let out: Vec<f64> = parent_val.iter().map(|x| x.max(0.0)).collect();
        Ok(detach_or_track(t, parent, out.clone(), |id| Node::Relu {
            id,
            parent,
            value: out,
        }))
    })
}

fn ag_dense(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w_id = tensor_id(args.first().ok_or("ag_dense(w,x,b)")?)?;
    let x_id = tensor_id(args.get(1).ok_or("ag_dense(w,x,b)")?)?;
    let b_id = tensor_id(args.get(2).ok_or("ag_dense(w,x,b)")?)?;
    with_tape(|t| {
        let w = node_value(t, w_id)?;
        let x = node_value(t, x_id)?;
        let b = node_value(t, b_id)?;
        let out_dim = b.len();
        if out_dim == 0 || w.len() % out_dim != 0 {
            return Err("ag_dense: bad shapes".into());
        }
        let in_dim = w.len() / out_dim;
        if x.len() != in_dim {
            return Err("ag_dense: x dim mismatch".into());
        }
        let mut y = vec![0.0; out_dim];
        for o in 0..out_dim {
            let mut s = b[o];
            for i in 0..in_dim {
                s += w[o * in_dim + i] * x[i];
            }
            y[o] = s;
        }
        Ok(detach_or_track(t, w_id, y.clone(), |id| Node::Dense {
            id,
            w: w_id,
            x: x_id,
            b: b_id,
            value: y,
            in_dim,
            out_dim,
        }))
    })
}

fn ag_matmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a_id = tensor_id(args.first().ok_or("ag_matmul(a,b,m,k,n)")?)?;
    let b_id = tensor_id(args.get(1).ok_or("ag_matmul(a,b,m,k,n)")?)?;
    let m = num(args.get(2).ok_or("ag_matmul: m")?)? as usize;
    let k = num(args.get(3).ok_or("ag_matmul: k")?)? as usize;
    let n = num(args.get(4).ok_or("ag_matmul: n")?)? as usize;
    with_tape(|t| {
        let a = node_value(t, a_id)?;
        let b = node_value(t, b_id)?;
        if a.len() != m * k || b.len() != k * n {
            return Err("ag_matmul: size mismatch".into());
        }
        let mut out = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0;
                for tdim in 0..k {
                    s += a[i * k + tdim] * b[tdim * n + j];
                }
                out[i * n + j] = s;
            }
        }
        Ok(detach_or_track(t, a_id, out.clone(), |id| Node::Matmul {
            id,
            a: a_id,
            b: b_id,
            value: out,
            m,
            k,
            n,
        }))
    })
}

fn ag_softmax(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_softmax(t)")?)?;
    with_tape(|t| {
        let x = node_value(t, parent)?;
        if x.is_empty() {
            return Err("ag_softmax: empty".into());
        }
        let max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ex: Vec<f64> = x.iter().map(|v| (v - max).exp()).collect();
        let sum: f64 = ex.iter().sum();
        let out: Vec<f64> = ex.iter().map(|v| v / sum).collect();
        Ok(detach_or_track(t, parent, out.clone(), |id| Node::Softmax {
            id,
            parent,
            value: out,
        }))
    })
}

fn ag_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let left = tensor_id(args.first().ok_or("ag_add(a,b)")?)?;
    let right = tensor_id(args.get(1).ok_or("ag_add(a,b)")?)?;
    with_tape(|t| {
        let a = node_value(t, left)?;
        let b = node_value(t, right)?;
        if a.len() != b.len() {
            return Err("ag_add: length mismatch".into());
        }
        let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        Ok(detach_or_track(t, left, out.clone(), |id| Node::Add {
            id,
            left,
            right,
            value: out,
        }))
    })
}

fn ag_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let left = tensor_id(args.first().ok_or("ag_mul(a,b)")?)?;
    let right = tensor_id(args.get(1).ok_or("ag_mul(a,b)")?)?;
    with_tape(|t| {
        let a = node_value(t, left)?;
        let b = node_value(t, right)?;
        if a.len() != b.len() {
            return Err("ag_mul: length mismatch".into());
        }
        let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
        Ok(detach_or_track(t, left, out.clone(), |id| Node::Mul {
            id,
            left,
            right,
            value: out,
        }))
    })
}

fn ag_mse(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pred_id = tensor_id(args.first().ok_or("ag_mse(pred, target)")?)?;
    let target = vector_at(args, 1, "ag_mse")?;
    with_tape(|t| {
        let pred = node_value(t, pred_id)?;
        if pred.len() != target.len() || pred.is_empty() {
            return Err("ag_mse: length mismatch".into());
        }
        let loss = pred
            .iter()
            .zip(target.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / pred.len() as f64;
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Mse {
                id,
                pred: pred_id,
                target: target.clone(),
                value: loss,
            },
        );
        Ok(tensor_out(id, &[loss]))
    })
}

fn ag_ce(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pred_id = tensor_id(args.first().ok_or("ag_ce(pred, target)")?)?;
    let target = vector_at(args, 1, "ag_ce")?;
    with_tape(|t| {
        let pred = node_value(t, pred_id)?;
        if pred.len() != target.len() || pred.is_empty() {
            return Err("ag_ce: length mismatch".into());
        }
        let mut loss = 0.0;
        for (p, y) in pred.iter().zip(target.iter()) {
            loss -= y * p.max(1e-12).ln();
        }
        loss /= pred.len() as f64;
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Ce {
                id,
                pred: pred_id,
                target: target.clone(),
                value: loss,
            },
        );
        Ok(tensor_out(id, &[loss]))
    })
}

fn accumulate(t: &mut Tape, id: u64, g: &[f64]) {
    let entry = t.grads.entry(id).or_insert_with(|| vec![0.0; g.len()]);
    if entry.len() != g.len() {
        *entry = g.to_vec();
        return;
    }
    for (a, b) in entry.iter_mut().zip(g.iter()) {
        *a += *b;
    }
}

fn ag_backward(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let loss_id = tensor_id(args.first().ok_or("ag_backward(loss)")?)?;
    with_tape(|t| {
        t.grads.clear();
        let root = t.nodes.get(&loss_id).cloned().ok_or("ag_backward: missing root")?;
        match root {
            Node::Mse {
                pred, target, ..
            } => {
                let pred_v = node_value(t, pred)?;
                let n = pred_v.len() as f64;
                let g_pred: Vec<f64> = pred_v
                    .iter()
                    .zip(target.iter())
                    .map(|(p, y)| 2.0 * (p - y) / n)
                    .collect();
                t.grads.insert(pred, g_pred);
            }
            Node::Ce {
                pred, target, ..
            } => {
                let pred_v = node_value(t, pred)?;
                let n = pred_v.len() as f64;
                let g_pred: Vec<f64> = pred_v
                    .iter()
                    .zip(target.iter())
                    .map(|(p, y)| -y / p.max(1e-12) / n)
                    .collect();
                t.grads.insert(pred, g_pred);
            }
            _ => return Err("ag_backward: root must be mse or ce".into()),
        }

        let mut ids: Vec<u64> = t.nodes.keys().copied().collect();
        ids.sort_by(|a, b| b.cmp(a));
        for id in ids {
            let node = t.nodes.get(&id).cloned().ok_or("missing node")?;
            match node {
                Node::Relu { parent, value, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let gin: Vec<f64> = gout
                            .iter()
                            .zip(value.iter())
                            .map(|(g, v)| if *v > 0.0 { *g } else { 0.0 })
                            .collect();
                        accumulate(t, parent, &gin);
                    }
                }
                Node::Dense {
                    w,
                    x,
                    b,
                    in_dim,
                    out_dim,
                    ..
                } => {
                    if let Some(gy) = t.grads.get(&id).cloned() {
                        let xv = node_value(t, x)?;
                        let wv = node_value(t, w)?;
                        let mut gw = vec![0.0; out_dim * in_dim];
                        let mut gx = vec![0.0; in_dim];
                        let mut gb = vec![0.0; out_dim];
                        for o in 0..out_dim {
                            gb[o] += gy[o];
                            for i in 0..in_dim {
                                gw[o * in_dim + i] += gy[o] * xv[i];
                                gx[i] += gy[o] * wv[o * in_dim + i];
                            }
                        }
                        accumulate(t, w, &gw);
                        accumulate(t, x, &gx);
                        accumulate(t, b, &gb);
                    }
                }
                Node::Matmul {
                    a,
                    b,
                    m,
                    k,
                    n,
                    ..
                } => {
                    if let Some(gy) = t.grads.get(&id).cloned() {
                        let av = node_value(t, a)?;
                        let bv = node_value(t, b)?;
                        let mut ga = vec![0.0; m * k];
                        let mut gb = vec![0.0; k * n];
                        for i in 0..m {
                            for j in 0..n {
                                let g = gy[i * n + j];
                                for tdim in 0..k {
                                    ga[i * k + tdim] += g * bv[tdim * n + j];
                                    gb[tdim * n + j] += g * av[i * k + tdim];
                                }
                            }
                        }
                        accumulate(t, a, &ga);
                        accumulate(t, b, &gb);
                    }
                }
                Node::Softmax { parent, value, .. } => {
                    if let Some(gy) = t.grads.get(&id).cloned() {
                        let dot: f64 = gy.iter().zip(value.iter()).map(|(g, s)| g * s).sum();
                        let gin: Vec<f64> = value
                            .iter()
                            .zip(gy.iter())
                            .map(|(s, g)| s * (g - dot))
                            .collect();
                        accumulate(t, parent, &gin);
                    }
                }
                Node::Add { left, right, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        accumulate(t, left, &gout);
                        accumulate(t, right, &gout);
                    }
                }
                Node::Mul { left, right, value: _, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let lv = node_value(t, left)?;
                        let rv = node_value(t, right)?;
                        let gl: Vec<f64> = gout.iter().zip(rv.iter()).map(|(g, r)| g * r).collect();
                        let gr: Vec<f64> = gout.iter().zip(lv.iter()).map(|(g, l)| g * l).collect();
                        accumulate(t, left, &gl);
                        accumulate(t, right, &gr);
                    }
                }
                Node::Mse { .. } | Node::Ce { .. } if id == loss_id => {}
                _ => {}
            }
        }
        Ok(Value::Bool(true))
    })
}

fn ag_grad(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = tensor_id(args.first().ok_or("ag_grad(t)")?)?;
    with_tape(|t| {
        t.grads
            .get(&id)
            .cloned()
            .map(|g| vector_out(&g))
            .ok_or_else(|| "ag_grad: no gradient (run ag_backward)".into())
    })
}

fn ag_value(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = tensor_id(args.first().ok_or("ag_value(t)")?)?;
    with_tape(|t| Ok(vector_out(&node_value(t, id)?)))
}

fn ag_clear(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    with_tape(|t| {
        *t = Tape::default();
        Ok(Value::Null)
    })
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ag_tensor", "ag_tensor"], ag_tensor);
    bind(&["science_ag_relu", "ag_relu"], ag_relu);
    bind(&["science_ag_dense", "ag_dense"], ag_dense);
    bind(&["science_ag_matmul", "ag_matmul"], ag_matmul);
    bind(&["science_ag_softmax", "ag_softmax"], ag_softmax);
    bind(&["science_ag_add", "ag_add"], ag_add);
    bind(&["science_ag_mul", "ag_mul"], ag_mul);
    bind(&["science_ag_mse", "ag_mse"], ag_mse);
    bind(&["science_ag_ce", "ag_ce"], ag_ce);
    bind(&["science_ag_no_grad", "ag_no_grad"], ag_no_grad);
    bind(&["science_ag_enable_grad", "ag_enable_grad"], ag_enable_grad);
    bind(&["science_ag_backward", "ag_backward"], ag_backward);
    bind(&["science_ag_grad", "ag_grad"], ag_grad);
    bind(&["science_ag_value", "ag_value"], ag_value);
    bind(&["science_ag_clear", "ag_clear"], ag_clear);
}
