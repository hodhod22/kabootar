//! P11a — typed frames (i64 / f64 / bool / struct) for numeric bytecode functions.
//!
//! Tight loops stay in dense slots instead of the boxed `Value` enum.
//! Cranelift stays i64-only; float/bool/struct kernels use this interpreter.

use super::types::{BytecodeFnDef, Constant, Opcode};
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static TYPED_I64_HITS: AtomicU64 = AtomicU64::new(0);
static TYPED_I64_FALLBACKS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum T {
    I(i64),
    F(f64),
    B(bool),
    /// Index into the run's struct table (`ClassInstance` with `is_struct`).
    Obj(u16),
    N,
}

pub fn typed_i64_stats() -> (u64, u64) {
    (
        TYPED_I64_HITS.load(Ordering::Relaxed),
        TYPED_I64_FALLBACKS.load(Ordering::Relaxed),
    )
}

pub fn typed_i64_reset_for_tests() {
    TYPED_I64_HITS.store(0, Ordering::Relaxed);
    TYPED_I64_FALLBACKS.store(0, Ordering::Relaxed);
}

pub fn fn_is_typed_i64(func: &BytecodeFnDef) -> bool {
    if func.async_fn || func.generator_fn {
        return false;
    }
    if !func.try_regions.is_empty() || !func.arrow_functions.is_empty() {
        return false;
    }
    if func.local_captures.iter().any(|c| *c) {
        return false;
    }
    if func.immutable_locals.iter().any(|c| *c) {
        return false;
    }
    // Module functions share the module const pool (strings for other helpers).
    // Only constants *referenced* by this body must be typed i64.
    for op in &func.code {
        if let Opcode::Const(i) = op {
            match func.constants.get(*i as usize) {
                Some(Constant::Number(_) | Constant::Bool(_) | Constant::Null | Constant::Float(_)) => {}
                _ => return false,
            }
        }
    }
    func.code.iter().all(opcode_is_typed_i64)
}

fn opcode_is_typed_i64(op: &Opcode) -> bool {
    matches!(
        op,
        Opcode::Const(_)
            | Opcode::LoadLocal(_)
            | Opcode::StoreLocal(_)
            | Opcode::AccAddLocal(_)
            | Opcode::Pop
            | Opcode::Dup
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::And
            | Opcode::Or
            | Opcode::Not
            | Opcode::Neg
            | Opcode::GetMember(_)
            | Opcode::MemberSet(_)
            | Opcode::Swap
            | Opcode::Jump(_)
            | Opcode::JumpIfFalse(_)
            | Opcode::Return
            | Opcode::Halt
    )
}

fn is_struct_value(v: &Value) -> bool {
    match v {
        Value::ClassInstance(inst) => inst
            .try_borrow()
            .map(|g| g.is_struct)
            .unwrap_or(false),
        _ => false,
    }
}

fn args_are_typed(args: &[Value]) -> bool {
    args.iter().all(|a| {
        matches!(
            a,
            Value::Number(_)
                | Value::Float(_)
                | Value::Bool(_)
                | Value::Undefined
                | Value::Null
        ) || is_struct_value(a)
    })
}

fn refs_float_const(func: &BytecodeFnDef) -> bool {
    func.code.iter().any(|op| {
        matches!(
            op,
            Opcode::Const(i)
                if matches!(func.constants.get(*i as usize), Some(Constant::Float(_)))
        )
    })
}

fn is_bool_kernel(func: &BytecodeFnDef) -> bool {
    let has_cmp = func.code.iter().any(|op| {
        matches!(
            op,
            Opcode::Eq
                | Opcode::Ne
                | Opcode::Lt
                | Opcode::Le
                | Opcode::Gt
                | Opcode::Ge
                | Opcode::And
                | Opcode::Or
                | Opcode::Not
        )
    });
    let has_arith = func.code.iter().any(|op| {
        matches!(
            op,
            Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Mod
                | Opcode::AccAddLocal(_)
                | Opcode::Neg
        )
    });
    has_cmp && !has_arith
}

fn uses_struct_ops(func: &BytecodeFnDef) -> bool {
    func.code
        .iter()
        .any(|op| matches!(op, Opcode::GetMember(_) | Opcode::MemberSet(_)))
}

fn args_block_i64_jit(args: &[Value]) -> bool {
    args.iter().any(|a| {
        matches!(a, Value::Float(_) | Value::Bool(_)) || is_struct_value(a)
    })
}

/// Run `func` on a dense typed frame when eligible. `None` means use the boxed VM.
pub fn try_run_typed_i64(
    func: &BytecodeFnDef,
    args: &[Value],
) -> Option<Result<(Value, Vec<Value>), String>> {
    if super::jit::fn_is_index_i64_loop(func) {
        if let Some(jit) = super::jit::try_run_jit(func, args) {
            TYPED_I64_HITS.fetch_add(1, Ordering::Relaxed);
            return Some(jit);
        }
        TYPED_I64_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    if !fn_is_typed_i64(func) {
        return None;
    }
    if !args_are_typed(args) {
        TYPED_I64_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    let try_jit = !refs_float_const(func)
        && !args_block_i64_jit(args)
        && !is_bool_kernel(func)
        && !uses_struct_ops(func);
    if try_jit {
        if let Some(jit) = super::jit::try_run_jit(func, args) {
            TYPED_I64_HITS.fetch_add(1, Ordering::Relaxed);
            return Some(jit);
        }
    }
    TYPED_I64_HITS.fetch_add(1, Ordering::Relaxed);
    Some(run_typed_i64(func, args))
}

fn const_t(c: &Constant) -> T {
    match c {
        Constant::Number(n) => T::I(*n),
        Constant::Float(f) => T::F(*f),
        Constant::Bool(b) => T::B(*b),
        _ => T::N,
    }
}

fn truthy(v: T) -> bool {
    match v {
        T::I(n) => n != 0,
        T::F(f) => f != 0.0 && !f.is_nan(),
        T::B(b) => b,
        T::Obj(_) => true,
        T::N => false,
    }
}

fn as_i(v: T) -> Result<i64, String> {
    match v {
        T::I(n) => Ok(n),
        T::F(f) => Ok(f as i64),
        T::B(b) => Ok(if b { 1 } else { 0 }),
        T::Obj(_) => Err("typed: struct used as integer".into()),
        T::N => Ok(0),
    }
}

fn as_f(v: T) -> f64 {
    match v {
        T::I(n) => n as f64,
        T::F(f) => f,
        T::B(b) => {
            if b {
                1.0
            } else {
                0.0
            }
        }
        T::Obj(_) | T::N => 0.0,
    }
}

fn is_floatish(v: T) -> bool {
    matches!(v, T::F(_))
}

fn intern_obj(objs: &mut Vec<Value>, v: Value) -> T {
    let i = objs.len() as u16;
    objs.push(v);
    T::Obj(i)
}

fn value_to_t(objs: &mut Vec<Value>, v: Value) -> Result<T, String> {
    match v {
        Value::Number(n) => Ok(T::I(n)),
        Value::Float(f) => Ok(T::F(f)),
        Value::Bool(b) => Ok(T::B(b)),
        Value::Null | Value::Undefined => Ok(T::N),
        Value::ClassInstance(_) if is_struct_value(&v) => Ok(intern_obj(objs, v)),
        other => Err(format!("typed struct: unsupported field {other:?}")),
    }
}

fn t_to_value(objs: &[Value], v: T) -> Value {
    match v {
        T::I(n) => Value::Number(n),
        T::F(f) => Value::Float(f),
        T::B(b) => Value::Bool(b),
        T::Obj(i) => objs.get(i as usize).cloned().unwrap_or(Value::Null),
        T::N => Value::Null,
    }
}

fn member_key(func: &BytecodeFnDef, idx: u16) -> Result<&str, String> {
    match func.constants.get(idx as usize) {
        Some(Constant::String(s)) => Ok(s.as_str()),
        _ => Err(format!("typed GetMember: bad key {idx}")),
    }
}

fn struct_get(objs: &mut Vec<Value>, oi: u16, key: &str) -> Result<T, String> {
    let v = objs.get(oi as usize).ok_or("typed struct: bad obj")?;
    let Value::ClassInstance(inst) = v else {
        return Err("typed GetMember: not a struct".into());
    };
    let inst_ref = inst
        .try_borrow()
        .map_err(|e| format!("class instance borrow: {e}"))?;
    if !inst_ref.is_struct {
        return Err("typed GetMember: class instance is not a struct".into());
    }
    let field = inst_ref
        .fields
        .get(key)
        .cloned()
        .ok_or_else(|| format!("struct has no member {key}"))?;
    drop(inst_ref);
    value_to_t(objs, field)
}

fn struct_set(objs: &mut [Value], oi: u16, key: &str, val: Value) -> Result<(), String> {
    let v = objs.get_mut(oi as usize).ok_or("typed struct: bad obj")?;
    let Value::ClassInstance(inst) = v else {
        return Err("typed MemberSet: not a struct".into());
    };
    let mut inst_ref = inst
        .try_borrow_mut()
        .map_err(|e| format!("class instance borrow: {e}"))?;
    if !inst_ref.is_struct {
        return Err("typed MemberSet: class instance is not a struct".into());
    }
    inst_ref.fields.insert(key.to_string(), val);
    Ok(())
}

fn bin_arith(op: Opcode, left: T, right: T) -> Result<T, String> {
    if matches!(left, T::Obj(_)) || matches!(right, T::Obj(_)) {
        return Err("typed: cannot arith a struct".into());
    }
    if is_floatish(left) || is_floatish(right) {
        let a = as_f(left);
        let b = as_f(right);
        let r = match op {
            Opcode::Add => a + b,
            Opcode::Sub => a - b,
            Opcode::Mul => a * b,
            Opcode::Div => a / b,
            Opcode::Mod => a % b,
            _ => return Err("typed: invalid arith".into()),
        };
        return Ok(T::F(r));
    }
    let a = as_i(left)?;
    let b = as_i(right)?;
    let r = match op {
        Opcode::Add => a.wrapping_add(b),
        Opcode::Sub => a.wrapping_sub(b),
        Opcode::Mul => a.wrapping_mul(b),
        Opcode::Div => {
            if b == 0 {
                return Err("Integer division by zero".into());
            }
            a / b
        }
        Opcode::Mod => {
            if b == 0 {
                return Err("Modulo by zero".into());
            }
            a % b
        }
        _ => return Err("typed i64: invalid arith".into()),
    };
    Ok(T::I(r))
}

fn bin_cmp(op: Opcode, left: T, right: T) -> bool {
    if is_floatish(left) || is_floatish(right) {
        let a = as_f(left);
        let b = as_f(right);
        return match op {
            Opcode::Eq => a == b,
            Opcode::Ne => a != b,
            Opcode::Lt => a < b,
            Opcode::Le => a <= b,
            Opcode::Gt => a > b,
            Opcode::Ge => a >= b,
            _ => false,
        };
    }
    if matches!(left, T::B(_)) && matches!(right, T::B(_)) {
        let a = truthy(left);
        let b = truthy(right);
        return match op {
            Opcode::Eq => a == b,
            Opcode::Ne => a != b,
            _ => bin_cmp(op, T::I(as_i(left).unwrap_or(0)), T::I(as_i(right).unwrap_or(0))),
        };
    }
    let a = as_i(left).unwrap_or(0);
    let b = as_i(right).unwrap_or(0);
    match op {
        Opcode::Eq => a == b,
        Opcode::Ne => a != b,
        Opcode::Lt => a < b,
        Opcode::Le => a <= b,
        Opcode::Gt => a > b,
        Opcode::Ge => a >= b,
        _ => false,
    }
}

fn run_typed_i64(func: &BytecodeFnDef, args: &[Value]) -> Result<(Value, Vec<Value>), String> {
    let nloc = func.locals.len().max(1);
    let mut locals = vec![T::N; nloc];
    let mut objs: Vec<Value> = Vec::new();
    for (i, param) in func.params.iter().enumerate() {
        if let Some(idx) = func.locals.iter().position(|l| l == param) {
            locals[idx] = match args.get(i) {
                Some(Value::Number(n)) => T::I(*n),
                Some(Value::Float(f)) => T::F(*f),
                Some(Value::Bool(b)) => T::B(*b),
                Some(v) if is_struct_value(v) => intern_obj(&mut objs, v.clone()),
                _ => T::N,
            };
        }
    }
    let consts: Vec<T> = func.constants.iter().map(const_t).collect();
    let mut stack: Vec<T> = Vec::with_capacity(16);
    let mut ip = 0usize;
    let code = &func.code;
    let result = loop {
        if ip >= code.len() {
            break stack.pop().unwrap_or(T::N);
        }
        match code[ip] {
            Opcode::Const(idx) => {
                stack.push(*consts.get(idx as usize).unwrap_or(&T::N));
            }
            Opcode::LoadLocal(idx) => {
                let i = idx as usize;
                stack.push(*locals.get(i).unwrap_or(&T::N));
            }
            Opcode::StoreLocal(idx) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = idx as usize;
                if i >= locals.len() {
                    locals.resize(i + 1, T::N);
                }
                locals[i] = v;
            }
            Opcode::AccAddLocal(idx) => {
                let rhs = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = idx as usize;
                if i >= locals.len() {
                    locals.resize(i + 1, T::N);
                }
                locals[i] = bin_arith(Opcode::Add, locals[i], rhs)?;
            }
            Opcode::Pop => {
                let _ = stack.pop().ok_or("Bytecode stack underflow")?;
            }
            Opcode::Dup => {
                let v = *stack.last().ok_or("Bytecode stack underflow")?;
                stack.push(v);
            }
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(bin_arith(code[ip], left, right)?);
            }
            Opcode::Eq | Opcode::Ne | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(T::B(bin_cmp(code[ip], left, right)));
            }
            Opcode::And => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(T::B(truthy(left) && truthy(right)));
            }
            Opcode::Or => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(T::B(truthy(left) || truthy(right)));
            }
            Opcode::Not => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(T::B(!truthy(v)));
            }
            Opcode::Neg => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(if is_floatish(v) {
                    T::F(-as_f(v))
                } else {
                    T::I(as_i(v)?.wrapping_neg())
                });
            }
            Opcode::Jump(off) => {
                ip = ((ip as i32 + 1) + off) as usize;
                continue;
            }
            Opcode::JumpIfFalse(off) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                if !truthy(v) {
                    ip = ((ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::Return => {
                break stack.pop().unwrap_or(T::N);
            }
            Opcode::Halt => {
                break stack.pop().unwrap_or(T::N);
            }
            Opcode::GetMember(key_idx) => {
                let key = member_key(func, key_idx)?;
                let c = stack.pop().ok_or("Bytecode stack underflow")?;
                let T::Obj(oi) = c else {
                    return Err("typed GetMember: expected struct".into());
                };
                stack.push(struct_get(&mut objs, oi, key)?);
            }
            Opcode::MemberSet(key_idx) => {
                let key = member_key(func, key_idx)?;
                let val = stack.pop().ok_or("Bytecode stack underflow")?;
                let c = stack.pop().ok_or("Bytecode stack underflow")?;
                let T::Obj(oi) = c else {
                    return Err("typed MemberSet: expected struct".into());
                };
                let written = t_to_value(&objs, val);
                struct_set(&mut objs, oi, key, written)?;
                stack.push(c);
                stack.push(val);
            }
            Opcode::Swap => {
                let top = stack.pop().ok_or("Bytecode stack underflow")?;
                let second = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(top);
                stack.push(second);
            }
            _ => return Err("typed i64: unexpected opcode".into()),
        }
        ip += 1;
    };
    let local_vals: Vec<Value> = locals
        .iter()
        .map(|v| match v {
            T::N => Value::Undefined,
            other => t_to_value(&objs, *other),
        })
        .collect();
    Ok((t_to_value(&objs, result), local_vals))
}
