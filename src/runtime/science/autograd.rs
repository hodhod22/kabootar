//! Autograd-lite tape (SC2c) — dense/relu/mse subset.

use super::helpers::{vector_at, vector_out};
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
    Mse { id: u64, pred: u64, target: Vec<f64>, value: f64 },
}

#[derive(Default)]
struct Tape {
    next_id: u64,
    nodes: HashMap<u64, Node>,
    grads: HashMap<u64, Vec<f64>>,
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

fn ag_tensor(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "ag_tensor")?;
    with_tape(|t| {
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Leaf {
                id,
                value: data.clone(),
            },
        );
        Ok(tensor_out(id, &data))
    })
}

fn ag_relu(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_relu(t)")?)?;
    with_tape(|t| {
        let parent_val = match t.nodes.get(&parent) {
            Some(Node::Leaf { value, .. })
            | Some(Node::Relu { value, .. })
            | Some(Node::Dense { value, .. }) => value.clone(),
            _ => return Err("ag_relu: bad parent".into()),
        };
        let out: Vec<f64> = parent_val.iter().map(|x| x.max(0.0)).collect();
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Relu {
                id,
                parent,
                value: out.clone(),
            },
        );
        Ok(tensor_out(id, &out))
    })
}

fn ag_dense(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w_id = tensor_id(args.first().ok_or("ag_dense(w,x,b)")?)?;
    let x_id = tensor_id(args.get(1).ok_or("ag_dense(w,x,b)")?)?;
    let b_id = tensor_id(args.get(2).ok_or("ag_dense(w,x,b)")?)?;
    with_tape(|t| {
        let (w, x, b) = {
            let wv = node_value(t, w_id)?;
            let xv = node_value(t, x_id)?;
            let bv = node_value(t, b_id)?;
            (wv, xv, bv)
        };
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
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Dense {
                id,
                w: w_id,
                x: x_id,
                b: b_id,
                value: y.clone(),
                in_dim,
                out_dim,
            },
        );
        Ok(tensor_out(id, &y))
    })
}

fn node_value(t: &Tape, id: u64) -> Result<Vec<f64>, String> {
    match t.nodes.get(&id) {
        Some(Node::Leaf { value, .. })
        | Some(Node::Relu { value, .. })
        | Some(Node::Dense { value, .. }) => Ok(value.clone()),
        Some(Node::Mse { value, .. }) => Ok(vec![*value]),
        None => Err("unknown autograd id".into()),
    }
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

fn ag_backward(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let loss_id = tensor_id(args.first().ok_or("ag_backward(loss)")?)?;
    with_tape(|t| {
        t.grads.clear();
        let Node::Mse {
            pred,
            target,
            value: _,
            ..
        } = t.nodes.get(&loss_id).cloned().ok_or("ag_backward: need mse node")?
        else {
            return Err("ag_backward: root must be mse".into());
        };
        let pred_v = node_value(t, pred)?;
        let n = pred_v.len() as f64;
        let g_pred: Vec<f64> = pred_v
            .iter()
            .zip(target.iter())
            .map(|(p, y)| 2.0 * (p - y) / n)
            .collect();
        t.grads.insert(pred, g_pred.clone());

        // Walk reverse in id order (sufficient for this subset DAG).
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
                Node::Mse { .. } if id == loss_id => {}
                _ => {}
            }
        }
        let _ = g_pred;
        Ok(Value::Bool(true))
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
    bind(&["science_ag_mse", "ag_mse"], ag_mse);
    bind(&["science_ag_backward", "ag_backward"], ag_backward);
    bind(&["science_ag_grad", "ag_grad"], ag_grad);
    bind(&["science_ag_value", "ag_value"], ag_value);
    bind(&["science_ag_clear", "ag_clear"], ag_clear);
}
