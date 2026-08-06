//! Autograd tape (SC2c/SC2f/SC2g/SC2h) — first-order + create_graph higher-order for sum/exp/mul/add.

use super::helpers::{num, num_at, vector_at, vector_out};
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone)]
enum Node {
    Leaf { id: u64, value: Vec<f64> },
    Relu { id: u64, parent: u64, value: Vec<f64> },
    Sigmoid { id: u64, parent: u64, value: Vec<f64> },
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
    /// Conv2d: input [C,H,W], weight [O,C,Kh,Kw], bias [O]; stride=1, pad=0.
    Conv2d {
        id: u64,
        x: u64,
        w: u64,
        b: u64,
        value: Vec<f64>,
        cin: usize,
        hin: usize,
        win: usize,
        cout: usize,
        kh: usize,
        kw: usize,
        hout: usize,
        wout: usize,
    },
    /// create_graph: dL/dx as a tape node (parents gy, w).
    Conv2dGradX {
        id: u64,
        gy: u64,
        w: u64,
        value: Vec<f64>,
        cin: usize,
        hin: usize,
        win: usize,
        cout: usize,
        kh: usize,
        kw: usize,
        hout: usize,
        wout: usize,
    },
    /// create_graph: dL/dw as a tape node (parents gy, x).
    Conv2dGradW {
        id: u64,
        gy: u64,
        x: u64,
        value: Vec<f64>,
        cin: usize,
        hin: usize,
        win: usize,
        cout: usize,
        kh: usize,
        kw: usize,
        hout: usize,
        wout: usize,
    },
    Softmax { id: u64, parent: u64, value: Vec<f64> },
    Add { id: u64, left: u64, right: u64, value: Vec<f64> },
    Sub { id: u64, left: u64, right: u64, value: Vec<f64> },
    Mul { id: u64, left: u64, right: u64, value: Vec<f64> },
    Div { id: u64, left: u64, right: u64, value: Vec<f64> },
    Sum { id: u64, parent: u64, value: f64 },
    Exp { id: u64, parent: u64, value: Vec<f64> },
    Mse { id: u64, pred: u64, target: Vec<f64>, value: f64 },
    Ce { id: u64, pred: u64, target: Vec<f64>, value: f64 },
}

struct Tape {
    next_id: u64,
    nodes: HashMap<u64, Node>,
    grads: HashMap<u64, Vec<f64>>,
    /// When create_graph: tape node id holding gradient of key.
    grad_nodes: HashMap<u64, u64>,
    grad_enabled: bool,
}

impl Default for Tape {
    fn default() -> Self {
        Self {
            next_id: 0,
            nodes: HashMap::new(),
            grads: HashMap::new(),
            grad_nodes: HashMap::new(),
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
        | Some(Node::Sigmoid { value, .. })
        | Some(Node::Dense { value, .. })
        | Some(Node::Matmul { value, .. })
        | Some(Node::Conv2d { value, .. })
        | Some(Node::Conv2dGradX { value, .. })
        | Some(Node::Conv2dGradW { value, .. })
        | Some(Node::Softmax { value, .. })
        | Some(Node::Add { value, .. })
        | Some(Node::Sub { value, .. })
        | Some(Node::Mul { value, .. })
        | Some(Node::Div { value, .. })
        | Some(Node::Exp { value, .. }) => Ok(value.clone()),
        Some(Node::Sum { value, .. })
        | Some(Node::Mse { value, .. })
        | Some(Node::Ce { value, .. }) => Ok(vec![*value]),
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

fn ag_sigmoid(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_sigmoid(t)")?)?;
    with_tape(|t| {
        let parent_val = node_value(t, parent)?;
        let out: Vec<f64> = parent_val
            .iter()
            .map(|x| 1.0 / (1.0 + (-x).exp()))
            .collect();
        Ok(detach_or_track(t, parent, out.clone(), |id| Node::Sigmoid {
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

fn idx3(c: usize, h: usize, w: usize, ci: usize, hi: usize, wi: usize) -> usize {
    let _ = c;
    ci * h * w + hi * w + wi
}

/// ag_conv2d(x, w, b, cin, hin, win, cout, kh, kw) — flat tensors; stride=1 pad=0.
fn ag_conv2d(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x_id = tensor_id(args.first().ok_or("ag_conv2d")?)?;
    let w_id = tensor_id(args.get(1).ok_or("ag_conv2d")?)?;
    let b_id = tensor_id(args.get(2).ok_or("ag_conv2d")?)?;
    let cin = num_at(args, 3, "ag_conv2d")? as usize;
    let hin = num_at(args, 4, "ag_conv2d")? as usize;
    let win = num_at(args, 5, "ag_conv2d")? as usize;
    let cout = num_at(args, 6, "ag_conv2d")? as usize;
    let kh = num_at(args, 7, "ag_conv2d")? as usize;
    let kw = num_at(args, 8, "ag_conv2d")? as usize;
    if cin == 0 || hin == 0 || win == 0 || cout == 0 || kh == 0 || kw == 0 {
        return Err("ag_conv2d: dims > 0".into());
    }
    let hout = hin + 1 - kh;
    let wout = win + 1 - kw;
    if hout == 0 || wout == 0 {
        return Err("ag_conv2d: output spatial 0".into());
    }
    with_tape(|t| {
        let x = node_value(t, x_id)?;
        let w = node_value(t, w_id)?;
        let b = node_value(t, b_id)?;
        if x.len() != cin * hin * win {
            return Err("ag_conv2d: x size".into());
        }
        if w.len() != cout * cin * kh * kw {
            return Err("ag_conv2d: w size".into());
        }
        if b.len() != cout {
            return Err("ag_conv2d: bias size".into());
        }
        let mut out = vec![0.0; cout * hout * wout];
        for oc in 0..cout {
            for oh in 0..hout {
                for ow in 0..wout {
                    let mut s = b[oc];
                    for ic in 0..cin {
                        for kh_i in 0..kh {
                            for kw_i in 0..kw {
                                let xv = x[idx3(cin, hin, win, ic, oh + kh_i, ow + kw_i)];
                                let wv = w[oc * (cin * kh * kw) + ic * (kh * kw) + kh_i * kw + kw_i];
                                s += xv * wv;
                            }
                        }
                    }
                    out[idx3(cout, hout, wout, oc, oh, ow)] = s;
                }
            }
        }
        Ok(detach_or_track(t, x_id, out.clone(), |id| Node::Conv2d {
            id,
            x: x_id,
            w: w_id,
            b: b_id,
            value: out,
            cin,
            hin,
            win,
            cout,
            kh,
            kw,
            hout,
            wout,
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

fn ag_sub(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let left = tensor_id(args.first().ok_or("ag_sub(a,b)")?)?;
    let right = tensor_id(args.get(1).ok_or("ag_sub(a,b)")?)?;
    with_tape(|t| {
        let a = node_value(t, left)?;
        let b = node_value(t, right)?;
        if a.len() != b.len() {
            return Err("ag_sub: length mismatch".into());
        }
        let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
        Ok(detach_or_track(t, left, out.clone(), |id| Node::Sub {
            id,
            left,
            right,
            value: out,
        }))
    })
}

fn ag_div(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let left = tensor_id(args.first().ok_or("ag_div(a,b)")?)?;
    let right = tensor_id(args.get(1).ok_or("ag_div(a,b)")?)?;
    with_tape(|t| {
        let a = node_value(t, left)?;
        let b = node_value(t, right)?;
        if a.len() != b.len() {
            return Err("ag_div: length mismatch".into());
        }
        let out: Vec<f64> = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| {
                if y.abs() < 1e-15 {
                    x / 1e-15
                } else {
                    x / y
                }
            })
            .collect();
        Ok(detach_or_track(t, left, out.clone(), |id| Node::Div {
            id,
            left,
            right,
            value: out,
        }))
    })
}

fn ag_sum(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_sum(t)")?)?;
    with_tape(|t| {
        let v = node_value(t, parent)?;
        let s: f64 = v.iter().sum();
        if !t.grad_enabled {
            let id = push_leaf(t, vec![s]);
            return Ok(tensor_out(id, &[s]));
        }
        let id = t.next_id;
        t.next_id += 1;
        t.nodes.insert(
            id,
            Node::Sum {
                id,
                parent,
                value: s,
            },
        );
        Ok(tensor_out(id, &[s]))
    })
}

fn ag_exp(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = tensor_id(args.first().ok_or("ag_exp(t)")?)?;
    with_tape(|t| {
        let parent_val = node_value(t, parent)?;
        let out: Vec<f64> = parent_val.iter().map(|x| x.exp()).collect();
        Ok(detach_or_track(t, parent, out.clone(), |id| Node::Exp {
            id,
            parent,
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

fn push_node(t: &mut Tape, node: Node) -> u64 {
    let id = match &node {
        Node::Leaf { id, .. }
        | Node::Relu { id, .. }
        | Node::Sigmoid { id, .. }
        | Node::Dense { id, .. }
        | Node::Matmul { id, .. }
        | Node::Conv2d { id, .. }
        | Node::Conv2dGradX { id, .. }
        | Node::Conv2dGradW { id, .. }
        | Node::Softmax { id, .. }
        | Node::Add { id, .. }
        | Node::Sub { id, .. }
        | Node::Mul { id, .. }
        | Node::Div { id, .. }
        | Node::Sum { id, .. }
        | Node::Exp { id, .. }
        | Node::Mse { id, .. }
        | Node::Ce { id, .. } => *id,
    };
    t.nodes.insert(id, node);
    id
}

fn alloc_id(t: &mut Tape) -> u64 {
    let id = t.next_id;
    t.next_id += 1;
    id
}

fn accumulate_graph(t: &mut Tape, id: u64, g_vals: &[f64], g_node: u64, create_graph: bool) {
    accumulate(t, id, g_vals);
    if !create_graph {
        return;
    }
    if let Some(&prev) = t.grad_nodes.get(&id) {
        let pv = node_value(t, prev).unwrap_or_else(|_| vec![0.0; g_vals.len()]);
        let out: Vec<f64> = pv
            .iter()
            .zip(g_vals.iter())
            .map(|(a, b)| a + b)
            .collect();
        let nid = alloc_id(t);
        push_node(
            t,
            Node::Add {
                id: nid,
                left: prev,
                right: g_node,
                value: out,
            },
        );
        t.grad_nodes.insert(id, nid);
    } else {
        t.grad_nodes.insert(id, g_node);
    }
}

fn ensure_grad_node(t: &mut Tape, id: u64, vals: &[f64], create_graph: bool) -> Option<u64> {
    if !create_graph {
        return None;
    }
    if let Some(&n) = t.grad_nodes.get(&id) {
        return Some(n);
    }
    let nid = alloc_id(t);
    push_node(
        t,
        Node::Leaf {
            id: nid,
            value: vals.to_vec(),
        },
    );
    t.grad_nodes.insert(id, nid);
    Some(nid)
}

fn ag_backward(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let loss_id = tensor_id(args.first().ok_or("ag_backward(loss, createGraph?)")?)?;
    let create_graph = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => *n != 0,
        _ => false,
    };
    with_tape(|t| {
        t.grads.clear();
        t.grad_nodes.clear();
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
                if create_graph {
                    let nid = alloc_id(t);
                    push_node(
                        t,
                        Node::Leaf {
                            id: nid,
                            value: g_pred.clone(),
                        },
                    );
                    t.grad_nodes.insert(pred, nid);
                }
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
                if create_graph {
                    let nid = alloc_id(t);
                    push_node(
                        t,
                        Node::Leaf {
                            id: nid,
                            value: g_pred.clone(),
                        },
                    );
                    t.grad_nodes.insert(pred, nid);
                }
                t.grads.insert(pred, g_pred);
            }
            Node::Sum { parent, .. } => {
                let pv = node_value(t, parent)?;
                let ones = vec![1.0; pv.len()];
                if create_graph {
                    let nid = alloc_id(t);
                    push_node(
                        t,
                        Node::Leaf {
                            id: nid,
                            value: ones.clone(),
                        },
                    );
                    t.grad_nodes.insert(parent, nid);
                }
                t.grads.insert(parent, ones);
            }
            _ => {
                // Scalar or vector root: seed ones matching root value length.
                let rv = node_value(t, loss_id)?;
                let ones = vec![1.0; rv.len()];
                if create_graph {
                    let nid = alloc_id(t);
                    push_node(
                        t,
                        Node::Leaf {
                            id: nid,
                            value: ones.clone(),
                        },
                    );
                    t.grad_nodes.insert(loss_id, nid);
                }
                t.grads.insert(loss_id, ones);
            }
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
                Node::Sigmoid { parent, value, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let gin: Vec<f64> = gout
                            .iter()
                            .zip(value.iter())
                            .map(|(g, s)| g * s * (1.0 - s))
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
                        if create_graph {
                            let gw_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gw_id,
                                    value: gw.clone(),
                                },
                            );
                            let gx_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gx_id,
                                    value: gx.clone(),
                                },
                            );
                            let gb_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gb_id,
                                    value: gb.clone(),
                                },
                            );
                            let _ = ensure_grad_node(t, id, &gy, true);
                            accumulate_graph(t, w, &gw, gw_id, true);
                            accumulate_graph(t, x, &gx, gx_id, true);
                            accumulate_graph(t, b, &gb, gb_id, true);
                        } else {
                            accumulate(t, w, &gw);
                            accumulate(t, x, &gx);
                            accumulate(t, b, &gb);
                        }
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
                        if create_graph {
                            // ga = gy @ B^T, gb = A^T @ gy — keep as Matmul nodes on tape.
                            let gyn = ensure_grad_node(t, id, &gy, true)
                                .ok_or("matmul create_graph: gy")?;
                            // B^T as Leaf
                            let mut bt = vec![0.0; n * k];
                            for r in 0..k {
                                for c in 0..n {
                                    bt[c * k + r] = bv[r * n + c];
                                }
                            }
                            let bt_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: bt_id,
                                    value: bt,
                                },
                            );
                            let ga_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Matmul {
                                    id: ga_id,
                                    a: gyn,
                                    b: bt_id,
                                    value: ga.clone(),
                                    m,
                                    k: n,
                                    n: k,
                                },
                            );
                            // A^T as Leaf
                            let mut at = vec![0.0; k * m];
                            for r in 0..m {
                                for c in 0..k {
                                    at[c * m + r] = av[r * k + c];
                                }
                            }
                            let at_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: at_id,
                                    value: at,
                                },
                            );
                            let gb_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Matmul {
                                    id: gb_id,
                                    a: at_id,
                                    b: gyn,
                                    value: gb.clone(),
                                    m: k,
                                    k: m,
                                    n,
                                },
                            );
                            accumulate_graph(t, a, &ga, ga_id, true);
                            accumulate_graph(t, b, &gb, gb_id, true);
                        } else {
                            accumulate(t, a, &ga);
                            accumulate(t, b, &gb);
                        }
                    }
                }
                Node::Conv2d {
                    x,
                    w,
                    b,
                    cin,
                    hin,
                    win,
                    cout,
                    kh,
                    kw,
                    hout,
                    wout,
                    ..
                } => {
                    if let Some(gy) = t.grads.get(&id).cloned() {
                        let xv = node_value(t, x)?;
                        let wv = node_value(t, w)?;
                        let mut gx = vec![0.0; cin * hin * win];
                        let mut gw = vec![0.0; cout * cin * kh * kw];
                        let mut gb = vec![0.0; cout];
                        for oc in 0..cout {
                            for oh in 0..hout {
                                for ow in 0..wout {
                                    let g = gy[idx3(cout, hout, wout, oc, oh, ow)];
                                    gb[oc] += g;
                                    for ic in 0..cin {
                                        for kh_i in 0..kh {
                                            for kw_i in 0..kw {
                                                let xi = idx3(cin, hin, win, ic, oh + kh_i, ow + kw_i);
                                                let wi = oc * (cin * kh * kw)
                                                    + ic * (kh * kw)
                                                    + kh_i * kw
                                                    + kw_i;
                                                gw[wi] += g * xv[xi];
                                                gx[xi] += g * wv[wi];
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if create_graph {
                            let gyn = ensure_grad_node(t, id, &gy, true)
                                .ok_or("conv create_graph: gy")?;
                            let gx_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Conv2dGradX {
                                    id: gx_id,
                                    gy: gyn,
                                    w,
                                    value: gx.clone(),
                                    cin,
                                    hin,
                                    win,
                                    cout,
                                    kh,
                                    kw,
                                    hout,
                                    wout,
                                },
                            );
                            let gw_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Conv2dGradW {
                                    id: gw_id,
                                    gy: gyn,
                                    x,
                                    value: gw.clone(),
                                    cin,
                                    hin,
                                    win,
                                    cout,
                                    kh,
                                    kw,
                                    hout,
                                    wout,
                                },
                            );
                            let gb_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gb_id,
                                    value: gb.clone(),
                                },
                            );
                            accumulate_graph(t, x, &gx, gx_id, true);
                            accumulate_graph(t, w, &gw, gw_id, true);
                            accumulate_graph(t, b, &gb, gb_id, true);
                        } else {
                            accumulate(t, x, &gx);
                            accumulate(t, w, &gw);
                            accumulate(t, b, &gb);
                        }
                    }
                }
                Node::Conv2dGradX {
                    gy,
                    w,
                    cin,
                    hin,
                    win,
                    cout,
                    kh,
                    kw,
                    hout,
                    wout,
                    ..
                } => {
                    // value = dL/dx; incoming gout = d²L path through gx.
                    if let Some(g2) = t.grads.get(&id).cloned() {
                        let wv = node_value(t, w)?;
                        let mut dgy = vec![0.0; cout * hout * wout];
                        let mut dw = vec![0.0; cout * cin * kh * kw];
                        for oc in 0..cout {
                            for oh in 0..hout {
                                for ow in 0..wout {
                                    for ic in 0..cin {
                                        for kh_i in 0..kh {
                                            for kw_i in 0..kw {
                                                let xi = idx3(cin, hin, win, ic, oh + kh_i, ow + kw_i);
                                                let wi = oc * (cin * kh * kw)
                                                    + ic * (kh * kw)
                                                    + kh_i * kw
                                                    + kw_i;
                                                let g = g2[xi];
                                                dgy[idx3(cout, hout, wout, oc, oh, ow)] += g * wv[wi];
                                                // Need gy for dw: d(gx)/dw * g2
                                                // gx[xi] includes gy[oc,oh,ow]*w[wi] so dw += g2[xi]*gy[...]
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        let gyv = node_value(t, gy)?;
                        for oc in 0..cout {
                            for oh in 0..hout {
                                for ow in 0..wout {
                                    let gyy = gyv[idx3(cout, hout, wout, oc, oh, ow)];
                                    for ic in 0..cin {
                                        for kh_i in 0..kh {
                                            for kw_i in 0..kw {
                                                let xi = idx3(cin, hin, win, ic, oh + kh_i, ow + kw_i);
                                                let wi = oc * (cin * kh * kw)
                                                    + ic * (kh * kw)
                                                    + kh_i * kw
                                                    + kw_i;
                                                dw[wi] += g2[xi] * gyy;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if create_graph {
                            let dgy_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: dgy_id,
                                    value: dgy.clone(),
                                },
                            );
                            let dw_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: dw_id,
                                    value: dw.clone(),
                                },
                            );
                            accumulate_graph(t, gy, &dgy, dgy_id, true);
                            accumulate_graph(t, w, &dw, dw_id, true);
                        } else {
                            accumulate(t, gy, &dgy);
                            accumulate(t, w, &dw);
                        }
                    }
                }
                Node::Conv2dGradW {
                    gy,
                    x,
                    cin,
                    hin,
                    win,
                    cout,
                    kh,
                    kw,
                    hout,
                    wout,
                    ..
                } => {
                    if let Some(g2) = t.grads.get(&id).cloned() {
                        let xv = node_value(t, x)?;
                        let gyv = node_value(t, gy)?;
                        let mut dgy = vec![0.0; cout * hout * wout];
                        let mut dx = vec![0.0; cin * hin * win];
                        for oc in 0..cout {
                            for oh in 0..hout {
                                for ow in 0..wout {
                                    for ic in 0..cin {
                                        for kh_i in 0..kh {
                                            for kw_i in 0..kw {
                                                let xi = idx3(cin, hin, win, ic, oh + kh_i, ow + kw_i);
                                                let wi = oc * (cin * kh * kw)
                                                    + ic * (kh * kw)
                                                    + kh_i * kw
                                                    + kw_i;
                                                let g = g2[wi];
                                                dgy[idx3(cout, hout, wout, oc, oh, ow)] += g * xv[xi];
                                                dx[xi] += g * gyv[idx3(cout, hout, wout, oc, oh, ow)];
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if create_graph {
                            let dgy_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: dgy_id,
                                    value: dgy.clone(),
                                },
                            );
                            let dx_id = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: dx_id,
                                    value: dx.clone(),
                                },
                            );
                            accumulate_graph(t, gy, &dgy, dgy_id, true);
                            accumulate_graph(t, x, &dx, dx_id, true);
                        } else {
                            accumulate(t, gy, &dgy);
                            accumulate(t, x, &dx);
                        }
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
                        if create_graph {
                            if let Some(gn) = ensure_grad_node(t, id, &gout, true) {
                                accumulate_graph(t, left, &gout, gn, true);
                                accumulate_graph(t, right, &gout, gn, true);
                            } else {
                                accumulate(t, left, &gout);
                                accumulate(t, right, &gout);
                            }
                        } else {
                            accumulate(t, left, &gout);
                            accumulate(t, right, &gout);
                        }
                    }
                }
                Node::Sub { left, right, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let gr: Vec<f64> = gout.iter().map(|g| -g).collect();
                        if create_graph {
                            if let Some(gn) = ensure_grad_node(t, id, &gout, true) {
                                let nid = alloc_id(t);
                                let neg: Vec<f64> = gr.clone();
                                // scale by -1 via mul with leaf -1s
                                let ones_neg = alloc_id(t);
                                push_node(
                                    t,
                                    Node::Leaf {
                                        id: ones_neg,
                                        value: vec![-1.0; gout.len()],
                                    },
                                );
                                push_node(
                                    t,
                                    Node::Mul {
                                        id: nid,
                                        left: gn,
                                        right: ones_neg,
                                        value: neg.clone(),
                                    },
                                );
                                accumulate_graph(t, left, &gout, gn, true);
                                accumulate_graph(t, right, &gr, nid, true);
                            } else {
                                accumulate(t, left, &gout);
                                accumulate(t, right, &gr);
                            }
                        } else {
                            accumulate(t, left, &gout);
                            accumulate(t, right, &gr);
                        }
                    }
                }
                Node::Mul { left, right, value: _, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let lv = node_value(t, left)?;
                        let rv = node_value(t, right)?;
                        let gl: Vec<f64> = gout.iter().zip(rv.iter()).map(|(g, r)| g * r).collect();
                        let gr: Vec<f64> = gout.iter().zip(lv.iter()).map(|(g, l)| g * l).collect();
                        if create_graph {
                            if let Some(gn) = ensure_grad_node(t, id, &gout, true) {
                                let gl_id = alloc_id(t);
                                push_node(
                                    t,
                                    Node::Mul {
                                        id: gl_id,
                                        left: gn,
                                        right,
                                        value: gl.clone(),
                                    },
                                );
                                let gr_id = alloc_id(t);
                                push_node(
                                    t,
                                    Node::Mul {
                                        id: gr_id,
                                        left: gn,
                                        right: left,
                                        value: gr.clone(),
                                    },
                                );
                                accumulate_graph(t, left, &gl, gl_id, true);
                                accumulate_graph(t, right, &gr, gr_id, true);
                            } else {
                                accumulate(t, left, &gl);
                                accumulate(t, right, &gr);
                            }
                        } else {
                            accumulate(t, left, &gl);
                            accumulate(t, right, &gr);
                        }
                    }
                }
                Node::Div { left, right, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let lv = node_value(t, left)?;
                        let rv = node_value(t, right)?;
                        let gl: Vec<f64> = gout
                            .iter()
                            .zip(rv.iter())
                            .map(|(g, r)| {
                                let d = if r.abs() < 1e-15 { 1e-15 } else { *r };
                                g / d
                            })
                            .collect();
                        let gr: Vec<f64> = gout
                            .iter()
                            .zip(lv.iter())
                            .zip(rv.iter())
                            .map(|((g, l), r)| {
                                let d = if r.abs() < 1e-15 { 1e-15 } else { *r };
                                -g * l / (d * d)
                            })
                            .collect();
                        // Numeric path only for Div higher-order (leaf detach of gin).
                        if create_graph {
                            let gl_leaf = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gl_leaf,
                                    value: gl.clone(),
                                },
                            );
                            let gr_leaf = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: gr_leaf,
                                    value: gr.clone(),
                                },
                            );
                            accumulate_graph(t, left, &gl, gl_leaf, true);
                            accumulate_graph(t, right, &gr, gr_leaf, true);
                        } else {
                            accumulate(t, left, &gl);
                            accumulate(t, right, &gr);
                        }
                    }
                }
                Node::Sum { parent, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let g0 = gout.first().copied().unwrap_or(1.0);
                        let pv = node_value(t, parent)?;
                        let gin = vec![g0; pv.len()];
                        if create_graph {
                            let nid = alloc_id(t);
                            push_node(
                                t,
                                Node::Leaf {
                                    id: nid,
                                    value: gin.clone(),
                                },
                            );
                            accumulate_graph(t, parent, &gin, nid, true);
                        } else {
                            accumulate(t, parent, &gin);
                        }
                    }
                }
                Node::Exp { parent, value, .. } => {
                    if let Some(gout) = t.grads.get(&id).cloned() {
                        let gin: Vec<f64> = gout
                            .iter()
                            .zip(value.iter())
                            .map(|(g, e)| g * e)
                            .collect();
                        if create_graph {
                            if let Some(gn) = ensure_grad_node(t, id, &gout, true) {
                                let nid = alloc_id(t);
                                push_node(
                                    t,
                                    Node::Mul {
                                        id: nid,
                                        left: gn,
                                        right: id,
                                        value: gin.clone(),
                                    },
                                );
                                accumulate_graph(t, parent, &gin, nid, true);
                            } else {
                                accumulate(t, parent, &gin);
                            }
                        } else {
                            accumulate(t, parent, &gin);
                        }
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

/// ag_grad_tensor(t) — gradient as an autograd tensor (requires backward(..., true)).
fn ag_grad_tensor(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = tensor_id(args.first().ok_or("ag_grad_tensor(t)")?)?;
    with_tape(|t| {
        let gid = t
            .grad_nodes
            .get(&id)
            .copied()
            .ok_or_else(|| "ag_grad_tensor: missing (run ag_backward(loss, true))".to_string())?;
        let g = t
            .grads
            .get(&id)
            .cloned()
            .ok_or_else(|| "ag_grad_tensor: no numeric grad".to_string())?;
        Ok(tensor_out(gid, &g))
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
    bind(&["science_ag_sigmoid", "ag_sigmoid"], ag_sigmoid);
    bind(&["science_ag_dense", "ag_dense"], ag_dense);
    bind(&["science_ag_matmul", "ag_matmul"], ag_matmul);
    bind(&["science_ag_conv2d", "ag_conv2d"], ag_conv2d);
    bind(&["science_ag_softmax", "ag_softmax"], ag_softmax);
    bind(&["science_ag_add", "ag_add"], ag_add);
    bind(&["science_ag_sub", "ag_sub"], ag_sub);
    bind(&["science_ag_mul", "ag_mul"], ag_mul);
    bind(&["science_ag_div", "ag_div"], ag_div);
    bind(&["science_ag_sum", "ag_sum"], ag_sum);
    bind(&["science_ag_exp", "ag_exp"], ag_exp);
    bind(&["science_ag_mse", "ag_mse"], ag_mse);
    bind(&["science_ag_ce", "ag_ce"], ag_ce);
    bind(&["science_ag_no_grad", "ag_no_grad"], ag_no_grad);
    bind(&["science_ag_enable_grad", "ag_enable_grad"], ag_enable_grad);
    bind(&["science_ag_backward", "ag_backward"], ag_backward);
    bind(&["science_ag_grad", "ag_grad"], ag_grad);
    bind(&["science_ag_grad_tensor", "ag_grad_tensor"], ag_grad_tensor);
    bind(&["science_ag_value", "ag_value"], ag_value);
    bind(&["science_ag_clear", "ag_clear"], ag_clear);
}
