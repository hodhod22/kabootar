//! Contiguous ndarray for `import "science"` (SC0 — NumPy-class core).
//!
//! Zero-copy views share an `NdShared` (`Rc<Vec<f64>>`) buffer — cloning a view
//! increments the Rc, so buffers cannot dangle while any view/owner lives.
//! Mutating ops copy-on-write when `strong_count > 1`.
//!
//! Buffer ownership: `nd_take` moves a unique buffer (rc ≤ 2 during the call =
//! caller binding + arg clone) and marks the source `__moved` via OID writeback.
//! Computation graphs / models / metadata stay GC objects (`science/lazy`,
//! `science/tensor` meta).

use super::helpers::{float_out, int_out, num, num_at, vector_at, vector_out};
use crate::runtime::stdlib::object::{object_oid, object_oid_of, writeback_object_by_oid};
use crate::value::{Environment, NdShared, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const ND_MARK: &str = "__kab_nd";
const ND_BUF: &str = "__buf";
const ND_VIEW: &str = "view";
const ND_OFFSET: &str = "offset";
const ND_STRIDES: &str = "strides";
const ND_MOVED: &str = "__moved";
const SLICE_MARK: &str = "__kab_slice";

thread_local! {
    /// Outstanding zero-copy views per buffer pointer (incremented on view create).
    /// Not decremented on drop — after views, `ensureOwned` / new data gets a new ptr.
    static ND_VIEW_COUNTS: RefCell<HashMap<*const Vec<f64>, usize>> = RefCell::new(HashMap::new());
}

fn note_view_buf(buf: &NdShared) {
    let ptr = Rc::as_ptr(&buf.data);
    ND_VIEW_COUNTS.with(|m| {
        *m.borrow_mut().entry(ptr).or_insert(0) += 1;
    });
}

fn buffer_has_recorded_view(buf: &NdShared) -> bool {
    let ptr = Rc::as_ptr(&buf.data);
    ND_VIEW_COUNTS.with(|m| m.borrow().get(&ptr).copied().unwrap_or(0) > 0)
}

fn shape_val(shape: &[usize]) -> Value {
    Value::Array(
        shape
            .iter()
            .map(|d| Value::Number(*d as i64))
            .collect(),
    )
}

fn strides_val(strides: &[usize]) -> Value {
    Value::Array(
        strides
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

fn parse_strides(v: &Value) -> Result<Vec<usize>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|it| {
                let n = num(it)?;
                if n < 0.0 {
                    return Err("nd strides must be non-negative".into());
                }
                Ok(n as usize)
            })
            .collect(),
        _ => Err("nd strides must be array".into()),
    }
}

fn shape_product(shape: &[usize]) -> usize {
    shape.iter().product::<usize>().max(if shape.is_empty() { 1 } else { 0 })
}

fn dtype_width(dtype: &str) -> usize {
    match dtype {
        "complex64" | "c64" => 2,
        _ => 1,
    }
}

/// Strides in f64 buffer units (complex64 uses width 2 per logical element).
fn strides_of_elems(shape: &[usize], width: usize) -> Vec<usize> {
    let mut stride = width.max(1);
    let mut strides = vec![0; shape.len()];
    for i in (0..shape.len()).rev() {
        strides[i] = stride;
        stride = stride.saturating_mul(shape[i]);
    }
    strides
}

fn is_complex_dtype(dtype: &str) -> bool {
    matches!(dtype, "complex64" | "c64")
}

fn dtype_of_value(v: &Value) -> String {
    match v {
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            match m.get("dtype") {
                Some(Value::String(s)) => s.clone(),
                _ => "f64".into(),
            }
        }
        _ => "f64".into(),
    }
}

/// View metadata + shared buffer (zero-copy).
struct NdView {
    buf: NdShared,
    offset: usize,
    shape: Vec<usize>,
    strides: Vec<usize>,
    dtype: String,
}

impl NdView {
    fn owned(shape: Vec<usize>, data: Vec<f64>, dtype: &str) -> Self {
        let w = dtype_width(dtype);
        let strides = strides_of_elems(&shape, w);
        Self {
            buf: NdShared::new(data),
            offset: 0,
            shape,
            strides,
            dtype: dtype.into(),
        }
    }

    fn width(&self) -> usize {
        dtype_width(&self.dtype)
    }

    fn is_c_contiguous(&self) -> bool {
        self.offset == 0 && self.strides == strides_of_elems(&self.shape, self.width())
    }

    fn numel(&self) -> usize {
        shape_product(&self.shape)
    }

    fn to_vec(&self) -> Vec<f64> {
        let n = self.numel();
        let w = self.width();
        let need = n.saturating_mul(w);
        if self.is_c_contiguous() && self.buf.len() >= self.offset + need {
            return self.buf.as_slice()[self.offset..self.offset + need].to_vec();
        }
        let data = self.buf.as_slice();
        let elem_strides = strides_of(&self.shape);
        let mut out = vec![0.0; need];
        for flat in 0..n {
            let idx = unravel(flat, &self.shape, &elem_strides);
            let mut off = self.offset;
            for d in 0..self.shape.len() {
                off += idx[d] * self.strides[d];
            }
            for t in 0..w {
                out[flat * w + t] = data.get(off + t).copied().unwrap_or(0.0);
            }
        }
        out
    }

    fn into_value(self) -> Value {
        self.into_value_flag(None)
    }

    fn into_value_as_view(self) -> Value {
        self.into_value_flag(Some(true))
    }

    fn into_value_flag(self, force_view: Option<bool>) -> Value {
        let is_view = force_view.unwrap_or_else(|| !self.is_c_contiguous());
        if is_view {
            note_view_buf(&self.buf);
        }
        let mut m = HashMap::new();
        m.insert(ND_MARK.into(), Value::Bool(true));
        m.insert(ND_MOVED.into(), Value::Bool(false));
        m.insert(ND_BUF.into(), Value::NdShared(self.buf.clone()));
        m.insert("shape".into(), shape_val(&self.shape));
        m.insert(ND_STRIDES.into(), strides_val(&self.strides));
        m.insert(ND_OFFSET.into(), Value::Number(self.offset as i64));
        m.insert("size".into(), Value::Number(self.numel() as i64));
        m.insert("dtype".into(), Value::String(self.dtype));
        m.insert(ND_VIEW.into(), Value::Bool(is_view));
        m.insert("rc".into(), Value::Number(self.buf.strong_count() as i64));
        if !is_view {
            m.insert("data".into(), vector_out(self.buf.as_slice()));
        }
        object_oid(&mut m);
        Value::Object(m)
    }
}

fn is_moved_map(m: &HashMap<String, Value>) -> bool {
    matches!(m.get(ND_MOVED), Some(Value::Bool(true)))
}

fn nd_from_object(m: &HashMap<String, Value>) -> Result<NdView, String> {
    if is_moved_map(m) {
        return Err("nd: use after move (buffer taken)".into());
    }
    let shape = parse_shape(m.get("shape").ok_or("nd missing shape")?)?;
    let dtype = match m.get("dtype") {
        Some(Value::String(s)) => s.clone(),
        _ => "f64".into(),
    };
    if let Some(Value::NdShared(buf)) = m.get(ND_BUF) {
        let offset = m
            .get(ND_OFFSET)
            .and_then(|v| num(v).ok())
            .unwrap_or(0.0) as usize;
        let strides = if let Some(s) = m.get(ND_STRIDES) {
            parse_strides(s)?
        } else {
            strides_of(&shape)
        };
        return Ok(NdView {
            buf: buf.clone(),
            offset,
            shape,
            strides,
            dtype,
        });
    }
    // Legacy object: owned Array data
    let data = flat_from_value(m.get("data").ok_or("nd missing data")?)?;
    let n = shape_product(&shape).saturating_mul(dtype_width(&dtype));
    if data.len() != n {
        return Err(format!("nd buffer length {n} != data length {}", data.len()));
    }
    Ok(NdView::owned(shape, data, &dtype))
}

fn flat_from_value(v: &Value) -> Result<Vec<f64>, String> {
    if crate::runtime::shared_memory::is_float64_array(v) {
        return crate::runtime::shared_memory::float64_array_to_f64_vec(v);
    }
    match v {
        Value::Array(items) => {
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
            Ok(nd_from_object(m)?.to_vec())
        }
        Value::NdShared(buf) => Ok(buf.as_slice().to_vec()),
        _ => Err("expected array, Float64Array, or ndarray".into()),
    }
}

/// SC0c: wrap a Float64Array as ndarray without copying element storage into Kab Array.
fn nd_from_f64(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let view = args.first().ok_or("nd_from_f64(float64Array, shape?)")?;
    if !crate::runtime::shared_memory::is_float64_array(view) {
        return Err("nd_from_f64: expected Float64Array".into());
    }
    let data = crate::runtime::shared_memory::float64_array_to_f64_vec(view)?;
    let shape = if let Some(s) = args.get(1).filter(|s| !matches!(s, Value::Undefined | Value::Null))
    {
        let shape = parse_shape(s)?;
        if shape_product(&shape) != data.len() {
            return Err("nd_from_f64: shape product must match view length".into());
        }
        shape
    } else {
        vec![data.len()]
    };
    let mut m = HashMap::new();
    m.insert(ND_MARK.into(), Value::Bool(true));
    m.insert("shape".into(), shape_val(&shape));
    m.insert("size".into(), Value::Number(data.len() as i64));
    m.insert("f64".into(), view.clone());
    // Keep a sync snapshot for ops that don't touch the view; prefer f64 view when present.
    m.insert("data".into(), vector_out(&data));
    m.insert("zero_copy".into(), Value::Bool(true));
    Ok(Value::Object(m))
}

/// Materialize / sync ndarray into a Float64Array (in-place write when view supplied).
fn nd_to_f64(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_to_f64")?;
    if let Some(view) = args.get(1) {
        crate::runtime::shared_memory::float64_array_write_slice(view, &data)?;
        let mut m = HashMap::new();
        m.insert(ND_MARK.into(), Value::Bool(true));
        m.insert("shape".into(), shape_val(&shape));
        m.insert("size".into(), Value::Number(data.len() as i64));
        m.insert("f64".into(), view.clone());
        m.insert("data".into(), vector_out(&data));
        m.insert("zero_copy".into(), Value::Bool(true));
        return Ok(Value::Object(m));
    }
    let bytes = data.len().saturating_mul(8).max(8);
    let buf = crate::runtime::shared_memory::sab_new(bytes)?;
    let sab_id = crate::runtime::shared_memory::sab_id(&buf)?;
    let mut view_map = HashMap::new();
    view_map.insert("__kab_f64".into(), Value::Bool(true));
    view_map.insert("__kab_sab_id".into(), Value::Number(sab_id as i64));
    view_map.insert("byteOffset".into(), Value::Number(0));
    view_map.insert("length".into(), Value::Number(data.len() as i64));
    let view = Value::Object(view_map);
    crate::runtime::shared_memory::float64_array_write_slice(&view, &data)?;
    nd_from_f64(&[view, shape_val(&shape)], _env)
}

fn nd_parts(v: &Value) -> Result<(Vec<usize>, Vec<f64>), String> {
    match v {
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            let view = nd_from_object(m)?;
            Ok((view.shape.clone(), view.to_vec()))
        }
        Value::Array(_) => {
            let data = flat_from_value(v)?;
            Ok((vec![data.len()], data))
        }
        _ => Err("expected ndarray".into()),
    }
}

fn nd_out(shape: &[usize], data: &[f64]) -> Value {
    nd_out_dtype(shape, data, "f64")
}

fn nd_out_dtype(shape: &[usize], data: &[f64], dtype: &str) -> Value {
    NdView::owned(shape.to_vec(), data.to_vec(), dtype).into_value()
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
    let (shape, _) = nd_at(args, 0, "nd_size")?;
    Ok(int_out(shape_product(&shape) as i64))
}

fn nd_reshape(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (old_shape, data) = nd_at(args, 0, "nd_reshape")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let shape = parse_shape(args.get(1).ok_or("nd_reshape(a, shape)")?)?;
    let w = dtype_width(&dtype);
    if shape_product(&shape).saturating_mul(w) != data.len() {
        return Err("nd_reshape: size mismatch".into());
    }
    let _ = old_shape;
    Ok(nd_out_dtype(&shape, &data, &dtype))
}

fn flat_index_from(idx: &Value, shape: &[usize], name: &str) -> Result<usize, String> {
    match idx {
        Value::Number(n) if *n >= 0 => Ok(*n as usize),
        Value::Float(f) if *f >= 0.0 => Ok(*f as usize),
        Value::Array(items) => {
            if items.len() != shape.len() {
                return Err(format!("{name}: index rank must match shape"));
            }
            let strides = strides_of(shape);
            let mut flat = 0usize;
            for (i, it) in items.iter().enumerate() {
                let j = num(it)? as usize;
                if j >= shape[i] {
                    return Err(format!("{name}: index out of bounds"));
                }
                flat += j * strides[i];
            }
            Ok(flat)
        }
        _ => Err(format!("{name}: bad index")),
    }
}

fn nd_get(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_get")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let idx = args.get(1).ok_or("nd_get(a, index|indices)")?;
    let flat = flat_index_from(idx, &shape, "nd_get")?;
    let w = dtype_width(&dtype);
    if is_complex_dtype(&dtype) {
        let base = flat.saturating_mul(w);
        if base + 1 >= data.len() {
            return Err("nd_get: index out of bounds".into());
        }
        return Ok(Value::Array(vec![
            float_out(data[base]),
            float_out(data[base + 1]),
        ]));
    }
    data.get(flat)
        .copied()
        .map(float_out)
        .ok_or_else(|| "nd_get: index out of bounds".into())
}

fn nd_set(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, mut data) = nd_at(args, 0, "nd_set")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let idx = args.get(1).ok_or("nd_set(a, index, value)")?;
    let flat = flat_index_from(idx, &shape, "nd_set")?;
    let w = dtype_width(&dtype);
    if is_complex_dtype(&dtype) {
        let base = flat.saturating_mul(w);
        if base + 1 >= data.len() {
            return Err("nd_set: index out of bounds".into());
        }
        let (re, im) = match args.get(2) {
            Some(Value::Array(pair)) if pair.len() >= 2 => (num(&pair[0])?, num(&pair[1])?),
            Some(v) => (num(v)?, 0.0),
            None => return Err("nd_set: missing value".into()),
        };
        data[base] = re;
        data[base + 1] = im;
        return Ok(nd_out_dtype(&shape, &data, &dtype));
    }
    let value = num_at(args, 2, "nd_set")?;
    if flat >= data.len() {
        return Err("nd_set: index out of bounds".into());
    }
    data[flat] = value;
    Ok(nd_out_dtype(&shape, &data, &dtype))
}

fn strides_of(shape: &[usize]) -> Vec<usize> {
    let mut stride = 1usize;
    let mut strides = vec![0; shape.len()];
    for i in (0..shape.len()).rev() {
        strides[i] = stride;
        stride = stride.saturating_mul(shape[i]);
    }
    strides
}

fn broadcast_shapes(a: &[usize], b: &[usize]) -> Result<Vec<usize>, String> {
    let ndim = a.len().max(b.len());
    let mut out = vec![0usize; ndim];
    for i in 0..ndim {
        let da = if i + a.len() < ndim {
            1
        } else {
            a[i + a.len() - ndim]
        };
        let db = if i + b.len() < ndim {
            1
        } else {
            b[i + b.len() - ndim]
        };
        if da == db || da == 1 || db == 1 {
            out[i] = da.max(db);
        } else {
            return Err(format!(
                "broadcast: incompatible shapes {:?} vs {:?}",
                a, b
            ));
        }
    }
    Ok(out)
}

fn unravel(flat: usize, shape: &[usize], strides: &[usize]) -> Vec<usize> {
    let mut idx = vec![0usize; shape.len()];
    let mut rem = flat;
    for i in 0..shape.len() {
        if strides[i] == 0 {
            idx[i] = 0;
        } else {
            idx[i] = rem / strides[i];
            rem %= strides[i];
        }
    }
    idx
}

fn map_broadcast_index(
    out_idx: &[usize],
    src_shape: &[usize],
    out_ndim: usize,
) -> usize {
    let offset = out_ndim - src_shape.len();
    let src_strides = strides_of(src_shape);
    let mut flat = 0usize;
    for i in 0..src_shape.len() {
        let oi = out_idx[i + offset];
        let si = if src_shape[i] == 1 { 0 } else { oi };
        flat += si * src_strides[i];
    }
    flat
}

fn broadcast_binop(
    sa: &[usize],
    a: &[f64],
    sb: &[usize],
    b: &[f64],
    f: impl Fn(f64, f64) -> f64,
) -> Result<(Vec<usize>, Vec<f64>), String> {
    let so = broadcast_shapes(sa, sb)?;
    let n = shape_product(&so);
    let out_strides = strides_of(&so);
    let mut out = vec![0.0; n];
    for flat in 0..n {
        let idx = unravel(flat, &so, &out_strides);
        let ia = map_broadcast_index(&idx, sa, so.len());
        let ib = map_broadcast_index(&idx, sb, so.len());
        out[flat] = f(a[ia], b[ib]);
    }
    Ok((so, out))
}

fn complex_binop(
    sa: &[usize],
    a: &[f64],
    sb: &[usize],
    b: &[f64],
    op: &str,
) -> Result<(Vec<usize>, Vec<f64>), String> {
    let so = broadcast_shapes(sa, sb)?;
    let n = shape_product(&so);
    let out_strides = strides_of(&so);
    let mut out = vec![0.0; n * 2];
    for flat in 0..n {
        let idx = unravel(flat, &so, &out_strides);
        let ia = map_broadcast_index(&idx, sa, so.len());
        let ib = map_broadcast_index(&idx, sb, so.len());
        let ar = a[ia * 2];
        let ai = a[ia * 2 + 1];
        let br = b[ib * 2];
        let bi = b[ib * 2 + 1];
        let (rr, ri) = match op {
            "add" => (ar + br, ai + bi),
            "sub" => (ar - br, ai - bi),
            "mul" => (ar * br - ai * bi, ar * bi + ai * br),
            "div" => {
                let den = br * br + bi * bi;
                if den == 0.0 {
                    return Err("nd: complex divide by zero".into());
                }
                ((ar * br + ai * bi) / den, (ai * br - ar * bi) / den)
            }
            _ => return Err("nd: bad complex op".into()),
        };
        out[flat * 2] = rr;
        out[flat * 2 + 1] = ri;
    }
    Ok((so, out))
}

fn nd_add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let da = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let db = dtype_of_value(args.get(1).unwrap_or(&Value::Null));
    let (sa, a) = nd_at(args, 0, "nd_add")?;
    let (sb, b) = nd_at(args, 1, "nd_add")?;
    if is_complex_dtype(&da) && is_complex_dtype(&db) {
        let (so, out) = complex_binop(&sa, &a, &sb, &b, "add")?;
        return Ok(nd_out_dtype(&so, &out, "complex64"));
    }
    let (so, out) = broadcast_binop(&sa, &a, &sb, &b, |x, y| x + y)?;
    Ok(nd_out(&so, &out))
}

fn nd_mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let da = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let db = dtype_of_value(args.get(1).unwrap_or(&Value::Null));
    let (sa, a) = nd_at(args, 0, "nd_mul")?;
    let (sb, b) = nd_at(args, 1, "nd_mul")?;
    if is_complex_dtype(&da) && is_complex_dtype(&db) {
        let (so, out) = complex_binop(&sa, &a, &sb, &b, "mul")?;
        return Ok(nd_out_dtype(&so, &out, "complex64"));
    }
    let (so, out) = broadcast_binop(&sa, &a, &sb, &b, |x, y| x * y)?;
    Ok(nd_out(&so, &out))
}

fn nd_sub(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let da = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let db = dtype_of_value(args.get(1).unwrap_or(&Value::Null));
    let (sa, a) = nd_at(args, 0, "nd_sub")?;
    let (sb, b) = nd_at(args, 1, "nd_sub")?;
    if is_complex_dtype(&da) && is_complex_dtype(&db) {
        let (so, out) = complex_binop(&sa, &a, &sb, &b, "sub")?;
        return Ok(nd_out_dtype(&so, &out, "complex64"));
    }
    let (so, out) = broadcast_binop(&sa, &a, &sb, &b, |x, y| x - y)?;
    Ok(nd_out(&so, &out))
}

fn nd_div(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let da = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let db = dtype_of_value(args.get(1).unwrap_or(&Value::Null));
    let (sa, a) = nd_at(args, 0, "nd_div")?;
    let (sb, b) = nd_at(args, 1, "nd_div")?;
    if is_complex_dtype(&da) && is_complex_dtype(&db) {
        let (so, out) = complex_binop(&sa, &a, &sb, &b, "div")?;
        return Ok(nd_out_dtype(&so, &out, "complex64"));
    }
    let (so, out) = broadcast_binop(&sa, &a, &sb, &b, |x, y| x / y)?;
    Ok(nd_out(&so, &out))
}

fn nd_ufunc(args: &[Value], name: &str, f: impl Fn(f64) -> f64) -> Result<Value, String> {
    let (shape, a) = nd_at(args, 0, name)?;
    Ok(nd_out(
        &shape,
        &a.iter().map(|x| f(*x)).collect::<Vec<_>>(),
    ))
}

fn nd_abs(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    if is_complex_dtype(&dtype) {
        let (shape, data) = nd_at(args, 0, "nd_abs")?;
        let n = shape_product(&shape);
        let mut out = vec![0.0; n];
        for i in 0..n {
            let re = data[i * 2];
            let im = data[i * 2 + 1];
            out[i] = (re * re + im * im).sqrt();
        }
        return Ok(nd_out(&shape, &out));
    }
    nd_ufunc(args, "nd_abs", |x| x.abs())
}

fn nd_conj(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let (shape, mut data) = nd_at(args, 0, "nd_conj")?;
    if !is_complex_dtype(&dtype) {
        return Ok(nd_out_dtype(&shape, &data, &dtype));
    }
    let n = shape_product(&shape);
    for i in 0..n {
        data[i * 2 + 1] = -data[i * 2 + 1];
    }
    Ok(nd_out_dtype(&shape, &data, "complex64"))
}

fn nd_exp(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    nd_ufunc(args, "nd_exp", |x| x.exp())
}

fn nd_log(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    nd_ufunc(args, "nd_log", |x| x.ln())
}

fn nd_sqrt(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    nd_ufunc(args, "nd_sqrt", |x| x.sqrt())
}

fn nd_clip(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, a) = nd_at(args, 0, "nd_clip")?;
    let lo = num_at(args, 1, "nd_clip")?;
    let hi = num_at(args, 2, "nd_clip")?;
    Ok(nd_out(
        &shape,
        &a.iter().map(|x| x.clamp(lo, hi)).collect::<Vec<_>>(),
    ))
}

fn nd_where(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sc, c) = nd_at(args, 0, "nd_where")?;
    let (sx, x) = nd_at(args, 1, "nd_where")?;
    let (sy, y) = nd_at(args, 2, "nd_where")?;
    let so = broadcast_shapes(&sc, &broadcast_shapes(&sx, &sy)?)?;
    let n = shape_product(&so);
    let out_strides = strides_of(&so);
    let mut out = vec![0.0; n];
    for flat in 0..n {
        let idx = unravel(flat, &so, &out_strides);
        let ic = map_broadcast_index(&idx, &sc, so.len());
        let ix = map_broadcast_index(&idx, &sx, so.len());
        let iy = map_broadcast_index(&idx, &sy, so.len());
        out[flat] = if c[ic] != 0.0 { x[ix] } else { y[iy] };
    }
    Ok(nd_out(&so, &out))
}

/// Zero-copy view-slice: ranges as [[start, stop], ...] (stop exclusive).
/// Shares parent `NdShared` buffer via Rc — no dangling while any view lives.
fn nd_slice(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let parent = match args.first() {
        Some(Value::Object(m)) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            nd_from_object(m)?
        }
        Some(v) => {
            let (shape, data) = nd_parts(v)?;
            NdView::owned(shape, data, "f64")
        }
        None => return Err("nd_slice(a, ranges)".into()),
    };
    let ranges = match args.get(1) {
        Some(Value::Array(items)) => items,
        _ => return Err("nd_slice(a, [[start,stop], ...])".into()),
    };
    view_from_ranges(&parent, ranges)
}

fn view_from_ranges(parent: &NdView, ranges: &[Value]) -> Result<Value, String> {
    if ranges.len() > parent.shape.len() {
        return Err("nd_slice: too many ranges".into());
    }
    let mut starts = vec![0usize; parent.shape.len()];
    let mut stops = parent.shape.clone();
    for (i, r) in ranges.iter().enumerate() {
        if is_slice_spec(r) {
            let (start, stop, _step) = parse_slice_spec(r, parent.shape[i])?;
            starts[i] = start;
            stops[i] = stop;
            continue;
        }
        let Value::Array(pair) = r else {
            // Single index → size-1 slice
            let idx = num(r)? as usize;
            if idx >= parent.shape[i] {
                return Err("nd_slice: index OOB".into());
            }
            starts[i] = idx;
            stops[i] = idx + 1;
            continue;
        };
        let start = if pair.is_empty() {
            0usize
        } else if matches!(pair[0], Value::Null | Value::Undefined) {
            0usize
        } else {
            num(&pair[0])? as usize
        };
        let stop = if pair.len() < 2 || matches!(pair[1], Value::Null | Value::Undefined) {
            parent.shape[i]
        } else {
            num(&pair[1])? as usize
        };
        if start > stop || stop > parent.shape[i] {
            return Err("nd_slice: bad range".into());
        }
        starts[i] = start;
        stops[i] = stop;
    }
    let out_shape: Vec<usize> = starts
        .iter()
        .zip(stops.iter())
        .map(|(a, b)| b - a)
        .collect();
    let mut offset = parent.offset;
    for d in 0..parent.shape.len() {
        offset += starts[d] * parent.strides[d];
    }
    let out = NdView {
        buf: parent.buf.clone(),
        offset,
        shape: out_shape,
        strides: parent.strides.clone(),
        dtype: parent.dtype.clone(),
    };
    Ok(out.into_value_as_view())
}

fn is_slice_spec(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(m) if matches!(m.get(SLICE_MARK), Some(Value::Bool(true)))
    )
}

fn parse_slice_spec(v: &Value, dim: usize) -> Result<(usize, usize, usize), String> {
    let Value::Object(m) = v else {
        return Err("slice spec".into());
    };
    let start = match m.get("start") {
        Some(Value::Null) | Some(Value::Undefined) | None => 0usize,
        Some(x) => {
            let n = num(x)? as i64;
            if n < 0 {
                (dim as i64 + n).max(0) as usize
            } else {
                n as usize
            }
        }
    };
    let stop = match m.get("stop") {
        Some(Value::Null) | Some(Value::Undefined) | None => dim,
        Some(x) => {
            let n = num(x)? as i64;
            if n < 0 {
                (dim as i64 + n).max(0) as usize
            } else {
                (n as usize).min(dim)
            }
        }
    };
    let step = match m.get("step") {
        Some(Value::Null) | Some(Value::Undefined) | None => 1usize,
        Some(x) => {
            let n = num(x)? as usize;
            if n == 0 {
                return Err("slice step must be != 0".into());
            }
            n
        }
    };
    if start > stop || stop > dim {
        return Err("slice: bad range".into());
    }
    Ok((start, stop, step))
}

/// Public entry for IndexGet on nd + slice/multi-index (used by ops::read_index).
pub fn nd_index_view_public(
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    nd_index_view(args, env)
}

fn nd_index_view(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let a = args.first().ok_or("nd_index_view(a, idx)")?;
    let idx = args.get(1).ok_or("nd_index_view: missing idx")?;
    match idx {
        Value::Object(m) if matches!(m.get(SLICE_MARK), Some(Value::Bool(true))) => {
            nd_slice(&[a.clone(), Value::Array(vec![idx.clone()])], env)
        }
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            let dtype = match m.get("dtype") {
                Some(Value::String(s)) => s.as_str(),
                _ => "f64",
            };
            if dtype == "bool" {
                nd_compress(&[a.clone(), idx.clone()], env)
            } else {
                nd_gather(&[a.clone(), idx.clone()], env)
            }
        }
        Value::Array(items)
            if !items.is_empty()
                && items
                    .iter()
                    .all(|x| matches!(x, Value::Number(_) | Value::Float(_))) =>
        {
            nd_gather(&[a.clone(), idx.clone()], env)
        }
        Value::Array(items) => nd_slice(&[a.clone(), Value::Array(items.clone())], env),
        Value::Number(_) | Value::Float(_) => nd_get(&[a.clone(), idx.clone()], env),
        _ => Err("nd_index_view: bad index".into()),
    }
}

fn parse_index_list(v: &Value, name: &str) -> Result<Vec<usize>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| {
                let n = num(x)?;
                if n < 0.0 {
                    return Err(format!("{name}: negative index"));
                }
                Ok(n as usize)
            })
            .collect(),
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            let view = nd_from_object(m)?;
            let data = view.to_vec();
            data.into_iter()
                .map(|x| {
                    if x < 0.0 {
                        return Err(format!("{name}: negative index"));
                    }
                    Ok(x as usize)
                })
                .collect()
        }
        _ => Err(format!("{name}: expect index array")),
    }
}

/// Integer-array gather along axis (default 0). NumPy-like fancy indexing subset.
fn nd_gather(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_gather")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let w = dtype_width(&dtype);
    let indices = parse_index_list(args.get(1).ok_or("nd_gather(a, indices, axis?)")?, "nd_gather")?;
    let axis = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0)
        .max(0.0) as usize;
    if shape.is_empty() {
        return Err("nd_gather: scalar".into());
    }
    if axis >= shape.len() {
        return Err("nd_gather: axis OOB".into());
    }
    for &i in &indices {
        if i >= shape[axis] {
            return Err("nd_gather: index OOB".into());
        }
    }
    let mut out_shape = shape.clone();
    out_shape[axis] = indices.len();
    let n_out = shape_product(&out_shape);
    let in_strides = strides_of(&shape);
    let out_strides = strides_of(&out_shape);
    let mut out = vec![0.0; n_out.saturating_mul(w)];
    for flat in 0..n_out {
        let mut idx = unravel(flat, &out_shape, &out_strides);
        let gather_pos = idx[axis];
        idx[axis] = indices[gather_pos];
        let mut src = 0usize;
        for d in 0..shape.len() {
            src += idx[d] * in_strides[d];
        }
        for t in 0..w {
            out[flat * w + t] = data[src * w + t];
        }
    }
    Ok(nd_out_dtype(&out_shape, &out, &dtype))
}

/// Boolean mask compress → 1D of true positions (NumPy `a[mask]` subset).
/// Mask length matches logical numel (for complex64: one flag per complex element).
fn nd_compress(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_compress")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let w = dtype_width(&dtype);
    let n = shape_product(&shape);
    let (_mshape, mask) = nd_at(args, 1, "nd_compress")?;
    if mask.len() != n {
        return Err("nd_compress: mask length must match array size".into());
    }
    let mut out = Vec::new();
    for i in 0..n {
        if mask[i] != 0.0 {
            for t in 0..w {
                out.push(data[i * w + t]);
            }
        }
    }
    let n_out = out.len() / w.max(1);
    let _ = shape;
    Ok(nd_out_dtype(&[n_out], &out, &dtype))
}

/// nd_nonzero(a) → integer index array of non-zero (or true) flat positions.
fn nd_nonzero(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_nonzero")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let w = dtype_width(&dtype);
    let n = shape_product(&shape);
    let mut idx = Vec::new();
    for i in 0..n {
        let nonzero = if w == 2 {
            data[i * 2] != 0.0 || data[i * 2 + 1] != 0.0
        } else {
            data[i] != 0.0
        };
        if nonzero {
            idx.push(i as f64);
        }
    }
    let m = idx.len();
    Ok(nd_out_dtype(&[m], &idx, "i64"))
}

/// nd_take_along(a, indices, axis?) — gather where `indices` is 1D along `axis`
/// and other dims are broadcast from `a` (NumPy take_along_axis lite for 1D indices).
fn nd_take_along(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    // For the subset: indices must be a 1D integer list; same as nd_gather.
    nd_gather(args, env)
}

/// nd_fancy_index(a, indexArrays, mode?)
/// mode: "broadcast" (default) — elementwise with length-1 broadcast;
///       "outer" — cartesian product of index arrays → multi-dim result.
fn nd_fancy_index(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_fancy_index")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let w = dtype_width(&dtype);
    let index_lists = match args.get(1) {
        Some(Value::Array(items)) => items,
        _ => return Err("nd_fancy_index(a, [ix0, ix1, ...], mode?)".into()),
    };
    if index_lists.is_empty() {
        return Err("nd_fancy_index: need at least one index array".into());
    }
    if index_lists.len() > shape.len() {
        return Err("nd_fancy_index: too many index arrays".into());
    }
    let mode = match args.get(2) {
        Some(Value::String(s)) => s.as_str(),
        _ => "broadcast",
    };
    let mut parsed: Vec<Vec<usize>> = Vec::with_capacity(index_lists.len());
    for (axis, ix) in index_lists.iter().enumerate() {
        let list = parse_index_list(ix, "nd_fancy_index")?;
        for &i in &list {
            if i >= shape[axis] {
                return Err("nd_fancy_index: index OOB".into());
            }
        }
        parsed.push(list);
    }
    let in_strides = strides_of(&shape);

    if mode == "outer" {
        let mut out_shape: Vec<usize> = parsed.iter().map(|l| l.len()).collect();
        // Keep trailing unindexed dims (full range) as outer product axes.
        for axis in parsed.len()..shape.len() {
            out_shape.push(shape[axis]);
        }
        let n_out = shape_product(&out_shape);
        let out_strides = strides_of(&out_shape);
        let mut out = vec![0.0; n_out.saturating_mul(w)];
        for flat in 0..n_out {
            let oidx = unravel(flat, &out_shape, &out_strides);
            let mut src = 0usize;
            for (axis, list) in parsed.iter().enumerate() {
                src += list[oidx[axis]] * in_strides[axis];
            }
            for axis in parsed.len()..shape.len() {
                let local = oidx[axis];
                src += local * in_strides[axis];
            }
            for t2 in 0..w {
                out[flat * w + t2] = data.get(src * w + t2).copied().unwrap_or(0.0);
            }
        }
        return Ok(nd_out_dtype(&out_shape, &out, &dtype));
    }

    // broadcast (default): elementwise with length-1 broadcast.
    let mut out_len = 1usize;
    for list in &parsed {
        if list.len() == 1 {
            continue;
        }
        if out_len == 1 {
            out_len = list.len();
        } else if list.len() != out_len {
            return Err("nd_fancy_index: index length mismatch (use mode \"outer\")".into());
        }
    }
    let mut out = vec![0.0; out_len.saturating_mul(w)];
    for t in 0..out_len {
        let mut src = 0usize;
        for (axis, list) in parsed.iter().enumerate() {
            let j = if list.len() == 1 { list[0] } else { list[t] };
            src += j * in_strides[axis];
        }
        for t2 in 0..w {
            out[t * w + t2] = data.get(src * w + t2).copied().unwrap_or(0.0);
        }
    }
    Ok(nd_out_dtype(&[out_len], &out, &dtype))
}

/// nd_broadcast_shapes(shapeA, shapeB) → broadcast result shape.
fn nd_broadcast_shapes_api(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sa = parse_shape(args.first().ok_or("nd_broadcast_shapes(a,b)")?)?;
    let sb = parse_shape(args.get(1).ok_or("nd_broadcast_shapes(a,b)")?)?;
    Ok(shape_val(&broadcast_shapes(&sa, &sb)?))
}

/// nd_broadcast_to(a, shape) — expand with NumPy broadcasting rules (copy).
fn nd_broadcast_to(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (sa, data) = nd_at(args, 0, "nd_broadcast_to")?;
    let dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let w = dtype_width(&dtype);
    let so = parse_shape(args.get(1).ok_or("nd_broadcast_to(a, shape)")?)?;
    let bc = broadcast_shapes(&sa, &so)?;
    if bc != so {
        return Err(format!(
            "nd_broadcast_to: {:?} does not broadcast exactly to {:?}",
            sa, so
        ));
    }
    let ndim = so.len();
    let n_out = shape_product(&so);
    let out_strides = strides_of(&so);
    let in_strides = strides_of(&sa);
    let mut out = vec![0.0; n_out.saturating_mul(w)];
    for flat in 0..n_out {
        let idx = unravel(flat, &so, &out_strides);
        let mut src = 0usize;
        let offset = ndim - sa.len();
        for d in 0..sa.len() {
            let out_i = idx[offset + d];
            let in_i = if sa[d] == 1 { 0 } else { out_i };
            src += in_i * in_strides[d];
        }
        for t in 0..w {
            out[flat * w + t] = data.get(src * w + t).copied().unwrap_or(0.0);
        }
    }
    Ok(nd_out_dtype(&so, &out, &dtype))
}

fn nd_is_view(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    match args.first() {
        Some(Value::Object(m)) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            Ok(Value::Bool(matches!(
                m.get(ND_VIEW),
                Some(Value::Bool(true))
            )))
        }
        _ => Ok(Value::Bool(false)),
    }
}

fn nd_buf_rc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    match args.first() {
        Some(Value::Object(m)) => {
            if let Some(Value::NdShared(buf)) = m.get(ND_BUF) {
                return Ok(Value::Number(buf.strong_count() as i64));
            }
            Ok(Value::Number(1))
        }
        _ => Err("nd_buf_rc(a)".into()),
    }
}

fn nd_ensure_owned(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if let Some(Value::Object(m)) = args.first() {
        if is_moved_map(m) {
            return Err("nd_ensure_owned: use after move".into());
        }
    }
    let (shape, data) = nd_at(args, 0, "nd_ensure_owned")?;
    Ok(NdView::owned(shape, data, "f64").into_value())
}

/// Product constructor: unique buffer + GC `meta` in one object (no Kab MemberSet).
fn nd_tensor(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = vector_at(args, 0, "nd_tensor")?;
    let shape = if let Some(s) = args.get(1).filter(|s| !matches!(s, Value::Undefined | Value::Null))
    {
        let shape = parse_shape(s)?;
        if shape_product(&shape) != data.len() {
            return Err("nd_tensor: shape product must match data length".into());
        }
        shape
    } else {
        vec![data.len()]
    };
    let mut out = NdView::owned(shape, data, "f64").into_value();
    if let Value::Object(ref mut m) = out {
        m.insert("kind".into(), Value::String("tensor".into()));
        m.insert("meta".into(), Value::Object(HashMap::new()));
    }
    Ok(out)
}

fn nd_is_moved(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    match args.first() {
        Some(Value::Object(m)) => Ok(Value::Bool(is_moved_map(m))),
        _ => Ok(Value::Bool(false)),
    }
}

fn buf_ptr(buf: &NdShared) -> *const Vec<f64> {
    Rc::as_ptr(&buf.data)
}

/// How many env object bindings currently hold this buffer (ignores stack temps).
fn env_buffer_holders(env: &Environment, ptr: *const Vec<f64>) -> usize {
    let mut n = 0usize;
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(m) = &live else {
            continue;
        };
        if let Some(Value::NdShared(buf)) = m.get(ND_BUF) {
            if buf_ptr(buf) == ptr {
                n += 1;
            }
        }
    }
    n
}

fn buffer_has_env_view(env: &Environment, ptr: *const Vec<f64>) -> bool {
    for name in env.all_binding_names() {
        let Some(live) = env.get(&name) else {
            continue;
        };
        let Value::Object(m) = &live else {
            continue;
        };
        if !matches!(m.get(ND_VIEW), Some(Value::Bool(true))) {
            continue;
        }
        if let Some(Value::NdShared(buf)) = m.get(ND_BUF) {
            if buf_ptr(buf) == ptr {
                return true;
            }
        }
    }
    false
}

/// Count NdShared handles for `ptr` reachable from a value (object fields, arrays).
fn count_buf_in_value(v: &Value, ptr: *const Vec<f64>, depth: usize) -> usize {
    if depth > 6 {
        return 0;
    }
    match v {
        Value::NdShared(buf) if buf_ptr(buf) == ptr => 1,
        Value::Object(m) => {
            let mut n = 0;
            for val in m.values() {
                n += count_buf_in_value(val, ptr, depth + 1);
            }
            n
        }
        Value::Array(items) => items
            .iter()
            .map(|x| count_buf_in_value(x, ptr, depth + 1))
            .sum(),
        _ => 0,
    }
}

fn env_buffer_mentions(env: &Environment, ptr: *const Vec<f64>) -> usize {
    let mut n = 0usize;
    for name in env.all_binding_names() {
        if let Some(live) = env.get(&name) {
            n += count_buf_in_value(&live, ptr, 0);
        }
    }
    n
}

/// Unique buffer owner relative to env bindings (stack clones ignored).
fn nd_is_owner(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    match args.first() {
        Some(Value::Object(m))
            if matches!(m.get(ND_MARK), Some(Value::Bool(true))) && !is_moved_map(m) =>
        {
            let is_view = matches!(m.get(ND_VIEW), Some(Value::Bool(true)));
            let Some(Value::NdShared(buf)) = m.get(ND_BUF) else {
                return Ok(Value::Bool(false));
            };
            let holders = env_buffer_holders(env, buf_ptr(buf));
            // Call arg may or may not already be an env binding; unique ⇒ ≤1 env holder.
            Ok(Value::Bool(!is_view && holders <= 1))
        }
        _ => Ok(Value::Bool(false)),
    }
}

/// Move unique buffer ownership to a new ndarray; mark source `__moved` (OID writeback).
///
/// Uniqueness is based on env bindings that hold the buffer (not raw Rc — bytecode
/// MemberSet/stack temps inflate Rc without true aliases).
fn nd_take(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let arg0 = args.first().ok_or("nd_take(a)")?;
    let Value::Object(m) = arg0 else {
        return Err("nd_take: expected ndarray".into());
    };
    if !matches!(m.get(ND_MARK), Some(Value::Bool(true))) {
        return Err("nd_take: expected ndarray".into());
    }
    if is_moved_map(m) {
        return Err("nd_take: buffer already moved".into());
    }
    if matches!(m.get(ND_VIEW), Some(Value::Bool(true))) {
        return Err("nd_take: cannot take a view (ensureOwned first)".into());
    }
    let shape = parse_shape(m.get("shape").ok_or("nd_take: missing shape")?)?;
    let offset = m
        .get(ND_OFFSET)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0) as usize;
    let strides = if let Some(s) = m.get(ND_STRIDES) {
        parse_strides(s)?
    } else {
        strides_of(&shape)
    };
    let dtype = match m.get("dtype") {
        Some(Value::String(s)) => s.clone(),
        _ => "f64".into(),
    };
    if offset != 0 || strides != strides_of(&shape) {
        return Err("nd_take: non-contiguous buffer (ensureOwned first)".into());
    }
    let Some(Value::NdShared(buf_ref)) = m.get(ND_BUF) else {
        return Err("nd_take: missing buffer".into());
    };
    let ptr = buf_ptr(buf_ref);
    if buffer_has_recorded_view(buf_ref) {
        return Err(
            "nd_take: buffer has a live view; ensureOwned or drop views first".into(),
        );
    }
    let holders = env_buffer_holders(env, ptr);
    let mentions = env_buffer_mentions(env, ptr);
    let has_view = buffer_has_env_view(env, ptr);
    // Unique: one logical owner. Shared: a live view or multiple env holders.
    // (Raw Rc is inflated by bytecode stack temps — do not use it here.)
    if has_view || holders > 1 || mentions > 1 {
        return Err(format!(
            "nd_take: buffer is shared (holders={holders}, mentions={mentions}, view={has_view}); ensureOwned or drop views first"
        ));
    }

    let oid = match object_oid_of(m) {
        Some(id) => id,
        None => {
            let mut stamped = m.clone();
            let id = object_oid(&mut stamped);
            writeback_object_by_oid(&Value::Object(stamped), env);
            id
        }
    };

    let buf = match m.get(ND_BUF) {
        Some(Value::NdShared(b)) => b.clone(),
        _ => return Err("nd_take: missing buffer".into()),
    };

    let mut emptied = m.clone();
    emptied.insert("__kab_oid".into(), Value::Number(oid as i64));
    emptied.insert(ND_MOVED.into(), Value::Bool(true));
    emptied.insert(ND_BUF.into(), Value::Null);
    emptied.insert("rc".into(), Value::Number(0));
    emptied.insert("data".into(), Value::Array(vec![]));
    writeback_object_by_oid(&Value::Object(emptied), env);

    let mut out = NdView {
        buf,
        offset: 0,
        shape,
        strides,
        dtype,
    }
    .into_value();
    if let Value::Object(ref mut om) = out {
        if matches!(m.get("kind"), Some(Value::String(k)) if k == "tensor") {
            om.insert("kind".into(), Value::String("tensor".into()));
        }
        if let Some(meta) = m.get("meta") {
            om.insert("meta".into(), meta.clone());
        }
    }
    Ok(out)
}

fn nd_concat(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arrays = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("nd_concat([a,b,...], axis?)".into()),
    };
    if arrays.is_empty() {
        return Err("nd_concat: empty".into());
    }
    let axis = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0) as usize;
    let mut parts: Vec<(Vec<usize>, Vec<f64>)> = Vec::new();
    for a in arrays {
        parts.push(nd_parts(a)?);
    }
    let rank = parts[0].0.len();
    if axis >= rank {
        return Err("nd_concat: axis out of range".into());
    }
    for (s, _) in &parts {
        if s.len() != rank {
            return Err("nd_concat: rank mismatch".into());
        }
        for d in 0..rank {
            if d != axis && s[d] != parts[0].0[d] {
                return Err("nd_concat: shape mismatch on non-concat axis".into());
            }
        }
    }
    let mut out_shape = parts[0].0.clone();
    out_shape[axis] = parts.iter().map(|(s, _)| s[axis]).sum();
    let n = shape_product(&out_shape);
    let mut out = vec![0.0; n];
    let out_strides = strides_of(&out_shape);
    let mut axis_off = 0usize;
    for (shape, data) in &parts {
        let in_strides = strides_of(shape);
        let pn = shape_product(shape);
        for flat in 0..pn {
            let idx = unravel(flat, shape, &in_strides);
            let mut oidx = idx.clone();
            oidx[axis] += axis_off;
            let mut oflat = 0usize;
            for d in 0..rank {
                oflat += oidx[d] * out_strides[d];
            }
            out[oflat] = data[flat];
        }
        axis_off += shape[axis];
    }
    Ok(nd_out(&out_shape, &out))
}

fn nd_stack(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arrays = match args.first() {
        Some(Value::Array(items)) => items,
        _ => return Err("nd_stack([a,b,...], axis?)".into()),
    };
    if arrays.is_empty() {
        return Err("nd_stack: empty".into());
    }
    let axis = args
        .get(1)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0) as usize;
    let mut parts: Vec<(Vec<usize>, Vec<f64>)> = Vec::new();
    for a in arrays {
        parts.push(nd_parts(a)?);
    }
    let base = parts[0].0.clone();
    for (s, _) in &parts {
        if *s != base {
            return Err("nd_stack: all arrays must share shape".into());
        }
    }
    if axis > base.len() {
        return Err("nd_stack: axis out of range".into());
    }
    let mut out_shape = base.clone();
    out_shape.insert(axis, parts.len());
    // Expand each to have size-1 on new axis, then concat.
    let mut expanded = Vec::new();
    for (s, d) in &parts {
        let mut es = s.clone();
        es.insert(axis, 1);
        expanded.push(nd_out(&es, d));
    }
    nd_concat(&[Value::Array(expanded), Value::Number(axis as i64)], _env)
}

fn nd_split(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_split")?;
    let sections = num_at(args, 1, "nd_split")? as usize;
    let axis = args
        .get(2)
        .and_then(|v| num(v).ok())
        .unwrap_or(0.0) as usize;
    if sections == 0 || axis >= shape.len() || shape[axis] % sections != 0 {
        return Err("nd_split: axis size must divide sections".into());
    }
    let chunk = shape[axis] / sections;
    let mut out = Vec::with_capacity(sections);
    for i in 0..sections {
        let mut ranges = Vec::new();
        for d in 0..shape.len() {
            if d == axis {
                let start = i * chunk;
                ranges.push(Value::Array(vec![
                    Value::Number(start as i64),
                    Value::Number((start + chunk) as i64),
                ]));
            } else {
                ranges.push(Value::Array(vec![
                    Value::Number(0),
                    Value::Number(shape[d] as i64),
                ]));
            }
        }
        out.push(nd_slice(&[nd_out(&shape, &data), Value::Array(ranges)], _env)?);
    }
    Ok(Value::Array(out))
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

/// BLAS-class DGEMM: prefer system OpenBLAS/MKL via cblas_dgemm FFI; else matrixmultiply.
fn gemm_blocked(m: usize, k: usize, n: usize, a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; m * n];
    if try_system_dgemm(m, n, k, 1.0, a, b, 0.0, &mut out) {
        return out;
    }
    unsafe {
        matrixmultiply::dgemm(
            m,
            k,
            n,
            1.0,
            a.as_ptr(),
            k as isize,
            1,
            b.as_ptr(),
            n as isize,
            1,
            0.0,
            out.as_mut_ptr(),
            n as isize,
            1,
        );
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
mod system_blas {
    use libloading::{Library, Symbol};
    use std::sync::OnceLock;

    type CblasDgemm = unsafe extern "C" fn(
        order: i32,
        trans_a: i32,
        trans_b: i32,
        m: i32,
        n: i32,
        k: i32,
        alpha: f64,
        a: *const f64,
        lda: i32,
        b: *const f64,
        ldb: i32,
        beta: f64,
        c: *mut f64,
        ldc: i32,
    );

    const CBLAS_ROW_MAJOR: i32 = 101;
    const CBLAS_NO_TRANS: i32 = 111;

    type SetThreadsFn = unsafe extern "C" fn(n: i32);
    type GetThreadsFn = unsafe extern "C" fn() -> i32;

    struct BlasLib {
        _lib: Library,
        name: &'static str,
        dgemm: CblasDgemm,
        set_threads: Option<SetThreadsFn>,
        get_threads: Option<GetThreadsFn>,
    }

    unsafe impl Send for BlasLib {}
    unsafe impl Sync for BlasLib {}

    static BLAS: OnceLock<Option<BlasLib>> = OnceLock::new();
    static THREAD_OVERRIDE: std::sync::atomic::AtomicI32 =
        std::sync::atomic::AtomicI32::new(0);

    fn candidate_libs() -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(p) = std::env::var("KABOOTAR_BLAS_LIB") {
            if !p.is_empty() {
                out.push(p);
            }
        }
        #[cfg(target_os = "windows")]
        {
            out.extend(
                [
                    "openblas.dll",
                    "libopenblas.dll",
                    "libopenblas_omp.dll",
                    "mkl_rt.2.dll",
                    "mkl_rt.dll",
                    "libblas.dll",
                ]
                .map(String::from),
            );
        }
        #[cfg(target_os = "linux")]
        {
            out.extend(
                [
                    "libopenblas.so",
                    "libopenblas.so.0",
                    "libmkl_rt.so",
                    "libmkl_rt.so.2",
                    "libblas.so.3",
                    "libblas.so",
                ]
                .map(String::from),
            );
        }
        #[cfg(target_os = "macos")]
        {
            out.extend(
                [
                    "libopenblas.dylib",
                    "libopenblas.0.dylib",
                    "/usr/local/opt/openblas/lib/libopenblas.dylib",
                    "/opt/homebrew/opt/openblas/lib/libopenblas.dylib",
                ]
                .map(String::from),
            );
        }
        out
    }

    fn resolve_thread_fns(
        lib: &Library,
        name: &str,
    ) -> (Option<SetThreadsFn>, Option<GetThreadsFn>) {
        unsafe {
            if name == "mkl" {
                let set = lib
                    .get::<SetThreadsFn>(b"MKL_Set_Num_Threads")
                    .ok()
                    .map(|s| *s)
                    .or_else(|| {
                        lib.get::<SetThreadsFn>(b"mkl_set_num_threads")
                            .ok()
                            .map(|s| *s)
                    });
                let get = lib
                    .get::<GetThreadsFn>(b"MKL_Get_Max_Threads")
                    .ok()
                    .map(|s| *s)
                    .or_else(|| {
                        lib.get::<GetThreadsFn>(b"mkl_get_max_threads")
                            .ok()
                            .map(|s| *s)
                    });
                return (set, get);
            }
            if name == "openblas" {
                let set = lib
                    .get::<SetThreadsFn>(b"openblas_set_num_threads")
                    .ok()
                    .map(|s| *s);
                let get = lib
                    .get::<GetThreadsFn>(b"openblas_get_num_threads")
                    .ok()
                    .map(|s| *s);
                return (set, get);
            }
            // Generic OpenMP fallback used by many BLAS builds.
            let set = lib
                .get::<SetThreadsFn>(b"omp_set_num_threads")
                .ok()
                .map(|s| *s);
            let get = lib
                .get::<GetThreadsFn>(b"omp_get_max_threads")
                .ok()
                .map(|s| *s);
            (set, get)
        }
    }

    fn probe() -> Option<BlasLib> {
        for path in candidate_libs() {
            let Ok(lib) = (unsafe { Library::new(&path) }) else {
                continue;
            };
            let sym: Result<Symbol<CblasDgemm>, _> = unsafe { lib.get(b"cblas_dgemm") };
            if let Ok(sym) = sym {
                let dgemm = *sym;
                let name = if path.to_ascii_lowercase().contains("mkl") {
                    "mkl"
                } else if path.to_ascii_lowercase().contains("openblas") {
                    "openblas"
                } else {
                    "system_blas"
                };
                let (set_threads, get_threads) = resolve_thread_fns(&lib, name);
                return Some(BlasLib {
                    _lib: lib,
                    name,
                    dgemm,
                    set_threads,
                    get_threads,
                });
            }
        }
        None
    }

    pub fn backend_name() -> &'static str {
        match BLAS.get_or_init(probe) {
            Some(b) => b.name,
            None => "matrixmultiply",
        }
    }

    pub fn set_num_threads(n: i32) -> bool {
        let n = n.max(1);
        THREAD_OVERRIDE.store(n, std::sync::atomic::Ordering::Relaxed);
        match BLAS.get_or_init(probe) {
            Some(b) => {
                if let Some(f) = b.set_threads {
                    unsafe { f(n) };
                    true
                } else {
                    // Soft success: remember override even without vendor symbol.
                    false
                }
            }
            None => false,
        }
    }

    pub fn num_threads() -> i32 {
        let override_n = THREAD_OVERRIDE.load(std::sync::atomic::Ordering::Relaxed);
        if override_n > 0 {
            return override_n;
        }
        match BLAS.get_or_init(probe) {
            Some(b) => {
                if let Some(f) = b.get_threads {
                    unsafe { f() }.max(1)
                } else {
                    1
                }
            }
            None => 1,
        }
    }

    pub fn threads_controllable() -> bool {
        match BLAS.get_or_init(probe) {
            Some(b) => b.set_threads.is_some(),
            None => false,
        }
    }

    pub fn dgemm(
        m: usize,
        n: usize,
        k: usize,
        alpha: f64,
        a: &[f64],
        b: &[f64],
        beta: f64,
        c: &mut [f64],
    ) -> bool {
        let Some(blas) = BLAS.get_or_init(probe).as_ref() else {
            return false;
        };
        if a.len() < m * k || b.len() < k * n || c.len() < m * n {
            return false;
        }
        unsafe {
            (blas.dgemm)(
                CBLAS_ROW_MAJOR,
                CBLAS_NO_TRANS,
                CBLAS_NO_TRANS,
                m as i32,
                n as i32,
                k as i32,
                alpha,
                a.as_ptr(),
                k as i32,
                b.as_ptr(),
                n as i32,
                beta,
                c.as_mut_ptr(),
                n as i32,
            );
        }
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn try_system_dgemm(
    m: usize,
    n: usize,
    k: usize,
    alpha: f64,
    a: &[f64],
    b: &[f64],
    beta: f64,
    c: &mut [f64],
) -> bool {
    system_blas::dgemm(m, n, k, alpha, a, b, beta, c)
}

#[cfg(target_arch = "wasm32")]
fn try_system_dgemm(
    _m: usize,
    _n: usize,
    _k: usize,
    _alpha: f64,
    _a: &[f64],
    _b: &[f64],
    _beta: f64,
    _c: &mut [f64],
) -> bool {
    false
}

fn sci_blas_backend(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Value::String(system_blas::backend_name().into()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Value::String("matrixmultiply".into()))
    }
}

/// sci_blas_set_num_threads(n) — OpenBLAS/MKL/OMP thread control when available.
fn sci_blas_set_num_threads(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let n = num_at(args, 0, "sci_blas_set_num_threads")? as i32;
    if n < 1 {
        return Err("sci_blas_set_num_threads: n >= 1".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ok = system_blas::set_num_threads(n);
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(ok));
        out.insert("threads".into(), Value::Number(n as i64));
        out.insert(
            "controllable".into(),
            Value::Bool(system_blas::threads_controllable()),
        );
        out.insert(
            "backend".into(),
            Value::String(system_blas::backend_name().into()),
        );
        Ok(Value::Object(out))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("threads".into(), Value::Number(1));
        out.insert("controllable".into(), Value::Bool(false));
        out.insert("backend".into(), Value::String("matrixmultiply".into()));
        Ok(Value::Object(out))
    }
}

fn sci_blas_num_threads(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Value::Number(system_blas::num_threads() as i64))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Value::Number(1))
    }
}

/// sci_blas_info() → { backend, threads, controllable }
fn sci_blas_info(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut out = HashMap::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        out.insert(
            "backend".into(),
            Value::String(system_blas::backend_name().into()),
        );
        out.insert(
            "threads".into(),
            Value::Number(system_blas::num_threads() as i64),
        );
        out.insert(
            "controllable".into(),
            Value::Bool(system_blas::threads_controllable()),
        );
    }
    #[cfg(target_arch = "wasm32")]
    {
        out.insert("backend".into(), Value::String("matrixmultiply".into()));
        out.insert("threads".into(), Value::Number(1));
        out.insert("controllable".into(), Value::Bool(false));
    }
    Ok(Value::Object(out))
}

/// sci_blas_dgemm(a, m, k, b, n, alpha?, beta?, c?) — BLAS-style DGEMM API (SC4a).
/// Computes alpha*A*B + beta*C (C optional zeros). System BLAS when available.
fn sci_blas_dgemm(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_blas_dgemm")?;
    let m = num_at(args, 1, "sci_blas_dgemm")? as usize;
    let k = num_at(args, 2, "sci_blas_dgemm")? as usize;
    let b = vector_at(args, 3, "sci_blas_dgemm")?;
    let n = num_at(args, 4, "sci_blas_dgemm")? as usize;
    let alpha = args.get(5).and_then(|v| num(v).ok()).unwrap_or(1.0);
    let beta = args.get(6).and_then(|v| num(v).ok()).unwrap_or(0.0);
    if m == 0 || k == 0 || n == 0 || a.len() != m * k || b.len() != k * n {
        return Err("sci_blas_dgemm: size mismatch".into());
    }
    let mut out = if beta.abs() > 1e-15 {
        if let Some(cv) = args.get(7) {
            let c = vector_at(&[cv.clone()], 0, "sci_blas_dgemm C")?;
            if c.len() != m * n {
                return Err("sci_blas_dgemm: C size".into());
            }
            c
        } else {
            vec![0.0; m * n]
        }
    } else {
        vec![0.0; m * n]
    };
    if try_system_dgemm(m, n, k, alpha, &a, &b, beta, &mut out) {
        return Ok(vector_out(&out));
    }
    // Fallback: matrixmultiply then scale/add.
    let mut prod = gemm_matrixmultiply(m, k, n, &a, &b);
    if (alpha - 1.0).abs() > 1e-15 {
        for v in &mut prod {
            *v *= alpha;
        }
    }
    if beta.abs() > 1e-15 {
        for i in 0..prod.len() {
            prod[i] += beta * out[i];
        }
        return Ok(vector_out(&prod));
    }
    Ok(vector_out(&prod))
}

fn gemm_matrixmultiply(m: usize, k: usize, n: usize, a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; m * n];
    unsafe {
        matrixmultiply::dgemm(
            m,
            k,
            n,
            1.0,
            a.as_ptr(),
            k as isize,
            1,
            b.as_ptr(),
            n as isize,
            1,
            0.0,
            out.as_mut_ptr(),
            n as isize,
            1,
        );
    }
    out
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
    let out = gemm_blocked(m, k, n, &a, &b);
    Ok(nd_out(&[m, n], &out))
}

/// sci_gemm(aFlat, m, k, bFlat, n) — flat blocked GEMM hotpath (SC4a).
fn sci_gemm(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_gemm")?;
    let m = num_at(args, 1, "sci_gemm")? as usize;
    let k = num_at(args, 2, "sci_gemm")? as usize;
    let b = vector_at(args, 3, "sci_gemm")?;
    let n = num_at(args, 4, "sci_gemm")? as usize;
    if m == 0 || k == 0 || n == 0 || a.len() != m * k || b.len() != k * n {
        return Err("sci_gemm: size mismatch".into());
    }
    Ok(vector_out(&gemm_blocked(m, k, n, &a, &b)))
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

/// P5/SC4a: SIMD-friendly vector add (chunked; auto-vectorized).
fn sci_vadd(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_vadd")?;
    let b = vector_at(args, 1, "sci_vadd")?;
    if a.len() != b.len() {
        return Err("sci_vadd: length mismatch".into());
    }
    let mut out = vec![0.0; a.len()];
    let n = a.len();
    let mut i = 0;
    while i + 4 <= n {
        out[i] = a[i] + b[i];
        out[i + 1] = a[i + 1] + b[i + 1];
        out[i + 2] = a[i + 2] + b[i + 2];
        out[i + 3] = a[i + 3] + b[i + 3];
        i += 4;
    }
    while i < n {
        out[i] = a[i] + b[i];
        i += 1;
    }
    Ok(vector_out(&out))
}

fn sci_vmul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_vmul")?;
    let b = vector_at(args, 1, "sci_vmul")?;
    if a.len() != b.len() {
        return Err("sci_vmul: length mismatch".into());
    }
    let mut out = vec![0.0; a.len()];
    let n = a.len();
    let mut i = 0;
    while i + 4 <= n {
        out[i] = a[i] * b[i];
        out[i + 1] = a[i + 1] * b[i + 1];
        out[i + 2] = a[i + 2] * b[i + 2];
        out[i + 3] = a[i + 3] * b[i + 3];
        i += 4;
    }
    while i < n {
        out[i] = a[i] * b[i];
        i += 1;
    }
    Ok(vector_out(&out))
}

fn sci_dot(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = vector_at(args, 0, "sci_dot")?;
    let b = vector_at(args, 1, "sci_dot")?;
    if a.len() != b.len() {
        return Err("sci_dot: length mismatch".into());
    }
    Ok(float_out(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()))
}

fn parse_dtype(v: &Value) -> Result<&'static str, String> {
    match v {
        Value::String(s) => match s.as_str() {
            "f64" | "float64" => Ok("f64"),
            "f32" | "float32" => Ok("f32"),
            "i32" | "int32" => Ok("i32"),
            "i64" | "int64" => Ok("i64"),
            "bool" => Ok("bool"),
            "complex64" | "c64" => Ok("complex64"),
            other => Err(format!("nd: unknown dtype {other}")),
        },
        _ => Err("nd: dtype must be string".into()),
    }
}

fn nd_dtype(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("nd_dtype(a)")?;
    match v {
        Value::Object(m) if matches!(m.get(ND_MARK), Some(Value::Bool(true))) => {
            Ok(m
                .get("dtype")
                .cloned()
                .unwrap_or(Value::String("f64".into())))
        }
        _ => Err("nd_dtype: expected ndarray".into()),
    }
}

fn nd_astype(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_astype")?;
    let src_dtype = dtype_of_value(args.first().unwrap_or(&Value::Null));
    let dtype = parse_dtype(args.get(1).ok_or("nd_astype(a, dtype)")?)?;
    let n = shape_product(&shape);
    let casted: Vec<f64> = if is_complex_dtype(dtype) && !is_complex_dtype(&src_dtype) {
        let mut out = vec![0.0; n * 2];
        for i in 0..n {
            out[i * 2] = data.get(i).copied().unwrap_or(0.0);
            out[i * 2 + 1] = 0.0;
        }
        out
    } else if !is_complex_dtype(dtype) && is_complex_dtype(&src_dtype) {
        // complex → real: take real part
        (0..n).map(|i| data[i * 2]).collect()
    } else if is_complex_dtype(dtype) && is_complex_dtype(&src_dtype) {
        data
    } else {
        match dtype {
            "f64" | "f32" => data,
            "i32" => data.iter().map(|x| (*x as i32) as f64).collect(),
            "i64" => data.iter().map(|x| (*x as i64) as f64).collect(),
            "bool" => data
                .iter()
                .map(|x| if *x != 0.0 { 1.0 } else { 0.0 })
                .collect(),
            _ => data,
        }
    };
    Ok(nd_out_dtype(&shape, &casted, dtype))
}

thread_local! {
    static ND_RNG: std::cell::RefCell<u64> = std::cell::RefCell::new(0xC0FFEE_u64);
}

fn rng_next() -> u64 {
    ND_RNG.with(|r| {
        let mut s = r.borrow_mut();
        // xorshift64*
        let mut x = *s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *s = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    })
}

fn rng_f64() -> f64 {
    (rng_next() >> 11) as f64 / ((1u64 << 53) as f64)
}

fn nd_seed(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let seed = num_at(args, 0, "nd_seed")? as u64;
    ND_RNG.with(|r| {
        *r.borrow_mut() = if seed == 0 { 1 } else { seed };
    });
    Ok(Value::Null)
}

fn nd_rand_uniform(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("nd_rand_uniform(shape, low?, high?)")?)?;
    let low = args.get(1).and_then(|v| num(v).ok()).unwrap_or(0.0);
    let high = args.get(2).and_then(|v| num(v).ok()).unwrap_or(1.0);
    let n = shape_product(&shape);
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        data.push(low + (high - low) * rng_f64());
    }
    Ok(nd_out(&shape, &data))
}

fn nd_rand_normal(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let shape = parse_shape(args.first().ok_or("nd_rand_normal(shape, mean?, std?)")?)?;
    let mean = args.get(1).and_then(|v| num(v).ok()).unwrap_or(0.0);
    let std = args.get(2).and_then(|v| num(v).ok()).unwrap_or(1.0);
    let n = shape_product(&shape);
    let mut data = Vec::with_capacity(n);
    // Box–Muller
    let mut i = 0usize;
    while i < n {
        let u1 = rng_f64().max(1e-12);
        let u2 = rng_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let z0 = r * (2.0 * std::f64::consts::PI * u2).cos();
        data.push(mean + std * z0);
        i += 1;
        if i < n {
            let z1 = r * (2.0 * std::f64::consts::PI * u2).sin();
            data.push(mean + std * z1);
            i += 1;
        }
    }
    Ok(nd_out(&shape, &data))
}

/// Binary format: magic KND1 | dtype u8 | ndim u32 LE | dims u64 LE… | f64 LE payload.
fn nd_save(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (shape, data) = nd_at(args, 0, "nd_save")?;
    let path = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("nd_save(a, path)".into()),
    };
    let dtype = match args.first() {
        Some(Value::Object(m)) => match m.get("dtype") {
            Some(Value::String(s)) => s.as_str(),
            _ => "f64",
        },
        _ => "f64",
    };
    let dtype_tag: u8 = match dtype {
        "f32" => 2,
        "i32" => 3,
        "i64" => 4,
        "bool" => 5,
        "complex64" | "c64" => 6,
        _ => 1,
    };
    let mut buf = Vec::new();
    buf.extend_from_slice(b"KND1");
    buf.push(dtype_tag);
    buf.extend_from_slice(&(shape.len() as u32).to_le_bytes());
    for d in &shape {
        buf.extend_from_slice(&(*d as u64).to_le_bytes());
    }
    for x in &data {
        buf.extend_from_slice(&x.to_le_bytes());
    }
    std::fs::write(path, &buf).map_err(|e| format!("nd_save({path}): {e}"))?;
    Ok(int_out(data.len() as i64))
}

fn nd_load(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("nd_load(path)".into()),
    };
    let buf = std::fs::read(path).map_err(|e| format!("nd_load({path}): {e}"))?;
    if buf.len() < 9 || &buf[0..4] != b"KND1" {
        return Err("nd_load: bad magic".into());
    }
    let dtype = match buf[4] {
        2 => "f32",
        3 => "i32",
        4 => "i64",
        5 => "bool",
        6 => "complex64",
        _ => "f64",
    };
    let ndim = u32::from_le_bytes(buf[5..9].try_into().unwrap()) as usize;
    let mut off = 9usize;
    let mut shape = Vec::with_capacity(ndim);
    for _ in 0..ndim {
        if off + 8 > buf.len() {
            return Err("nd_load: truncated shape".into());
        }
        let d = u64::from_le_bytes(buf[off..off + 8].try_into().unwrap()) as usize;
        shape.push(d);
        off += 8;
    }
    let n = shape_product(&shape).saturating_mul(dtype_width(dtype));
    if off + n * 8 > buf.len() {
        return Err("nd_load: truncated data".into());
    }
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        let x = f64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
        data.push(x);
        off += 8;
    }
    Ok(nd_out_dtype(&shape, &data, dtype))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_nd_zeros", "nd_zeros"], nd_zeros);
    bind(&["science_nd_ones", "nd_ones"], nd_ones);
    bind(&["science_nd_full", "nd_full"], nd_full);
    bind(&["science_nd_arange", "nd_arange"], nd_arange);
    bind(&["science_nd_from", "nd_from"], nd_from);
    bind(&["science_nd_from_f64", "nd_from_f64"], nd_from_f64);
    bind(&["science_nd_to_f64", "nd_to_f64"], nd_to_f64);
    bind(&["science_nd_shape", "nd_shape"], nd_shape);
    bind(&["science_nd_size", "nd_size"], nd_size);
    bind(&["science_nd_reshape", "nd_reshape"], nd_reshape);
    bind(&["science_nd_get", "nd_get"], nd_get);
    bind(&["science_nd_set", "nd_set"], nd_set);
    bind(&["science_nd_add", "nd_add"], nd_add);
    bind(&["science_nd_sub", "nd_sub"], nd_sub);
    bind(&["science_nd_mul", "nd_mul"], nd_mul);
    bind(&["science_nd_div", "nd_div"], nd_div);
    bind(&["science_nd_scale", "nd_scale"], nd_scale);
    bind(&["science_nd_abs", "nd_abs"], nd_abs);
    bind(&["science_nd_exp", "nd_exp"], nd_exp);
    bind(&["science_nd_log", "nd_log"], nd_log);
    bind(&["science_nd_sqrt", "nd_sqrt"], nd_sqrt);
    bind(&["science_nd_clip", "nd_clip"], nd_clip);
    bind(&["science_nd_where", "nd_where"], nd_where);
    bind(&["science_nd_gather", "nd_gather"], nd_gather);
    bind(&["science_nd_compress", "nd_compress"], nd_compress);
    bind(&["science_nd_nonzero", "nd_nonzero"], nd_nonzero);
    bind(&["science_nd_take_along", "nd_take_along"], nd_take_along);
    bind(&["science_nd_fancy_index", "nd_fancy_index"], nd_fancy_index);
    bind(&["science_nd_broadcast_to", "nd_broadcast_to"], nd_broadcast_to);
    bind(
        &["science_nd_broadcast_shapes", "nd_broadcast_shapes"],
        nd_broadcast_shapes_api,
    );
    bind(&["science_nd_conj", "nd_conj"], nd_conj);
    bind(&["science_nd_slice", "nd_slice"], nd_slice);
    bind(&["science_nd_index_view", "nd_index_view"], nd_index_view);
    bind(&["science_nd_is_view", "nd_is_view"], nd_is_view);
    bind(&["science_nd_buf_rc", "nd_buf_rc"], nd_buf_rc);
    bind(&["science_nd_ensure_owned", "nd_ensure_owned"], nd_ensure_owned);
    bind(&["science_nd_tensor", "nd_tensor"], nd_tensor);
    bind(&["science_nd_take", "nd_take"], nd_take);
    bind(&["science_nd_is_moved", "nd_is_moved"], nd_is_moved);
    bind(&["science_nd_is_owner", "nd_is_owner"], nd_is_owner);
    bind(&["science_nd_concat", "nd_concat"], nd_concat);
    bind(&["science_nd_stack", "nd_stack"], nd_stack);
    bind(&["science_nd_split", "nd_split"], nd_split);
    bind(&["science_nd_sum", "nd_sum"], nd_sum);
    bind(&["science_nd_mean", "nd_mean"], nd_mean);
    bind(&["science_nd_dot", "nd_dot"], nd_dot);
    bind(&["science_nd_matmul", "nd_matmul"], nd_matmul);
    bind(&["science_nd_solve", "nd_solve"], nd_solve);
    bind(&["science_nd_to_array", "nd_to_array"], nd_to_array);
    bind(&["science_nd_dtype", "nd_dtype"], nd_dtype);
    bind(&["science_nd_astype", "nd_astype"], nd_astype);
    bind(&["science_nd_seed", "nd_seed"], nd_seed);
    bind(&["science_nd_rand_uniform", "nd_rand_uniform"], nd_rand_uniform);
    bind(&["science_nd_rand_normal", "nd_rand_normal"], nd_rand_normal);
    bind(&["science_nd_save", "nd_save"], nd_save);
    bind(&["science_nd_load", "nd_load"], nd_load);
    bind(&["science_sci_vadd", "sci_vadd"], sci_vadd);
    bind(&["science_sci_vmul", "sci_vmul"], sci_vmul);
    bind(&["science_sci_dot", "sci_dot"], sci_dot);
    bind(&["science_sci_gemm", "sci_gemm"], sci_gemm);
    bind(&["science_sci_blas_dgemm", "sci_blas_dgemm"], sci_blas_dgemm);
    bind(&["science_sci_blas_backend", "sci_blas_backend"], sci_blas_backend);
    bind(
        &["science_sci_blas_set_num_threads", "sci_blas_set_num_threads"],
        sci_blas_set_num_threads,
    );
    bind(
        &["science_sci_blas_num_threads", "sci_blas_num_threads"],
        sci_blas_num_threads,
    );
    bind(&["science_sci_blas_info", "sci_blas_info"], sci_blas_info);
}
