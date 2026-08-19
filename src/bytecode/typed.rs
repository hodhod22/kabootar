//! P11a — typed i64 frames for numeric bytecode functions.
//!
//! Tight add-loops stay in dense `i64` slots instead of the boxed `Value` enum.

use super::types::{BytecodeFnDef, Constant, Opcode};
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static TYPED_I64_HITS: AtomicU64 = AtomicU64::new(0);
static TYPED_I64_FALLBACKS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum T {
    I(i64),
    B(bool),
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
    for c in &func.constants {
        match c {
            Constant::Number(_) | Constant::Bool(_) | Constant::Null | Constant::Float(_) => {}
            _ => return false,
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
            | Opcode::Not
            | Opcode::Neg
            | Opcode::Jump(_)
            | Opcode::JumpIfFalse(_)
            | Opcode::Return
            | Opcode::Halt
    )
}

fn args_are_i64(args: &[Value]) -> bool {
    args.iter().all(|a| {
        matches!(
            a,
            Value::Number(_) | Value::Float(_) | Value::Undefined | Value::Null
        )
    })
}

/// Run `func` on a dense i64 frame when eligible. `None` means use the boxed VM.
pub fn try_run_typed_i64(
    func: &BytecodeFnDef,
    args: &[Value],
) -> Option<Result<(Value, Vec<Value>), String>> {
    if !fn_is_typed_i64(func) {
        return None;
    }
    if !args_are_i64(args) {
        TYPED_I64_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    if let Some(jit) = super::jit::try_run_jit(func, args) {
        TYPED_I64_HITS.fetch_add(1, Ordering::Relaxed);
        return Some(jit);
    }
    TYPED_I64_HITS.fetch_add(1, Ordering::Relaxed);
    Some(run_typed_i64(func, args))
}

fn const_t(c: &Constant) -> T {
    match c {
        Constant::Number(n) => T::I(*n),
        Constant::Float(f) => T::I(*f as i64),
        Constant::Bool(b) => T::B(*b),
        _ => T::N,
    }
}

fn truthy(v: T) -> bool {
    match v {
        T::I(n) => n != 0,
        T::B(b) => b,
        T::N => false,
    }
}

fn as_i(v: T) -> Result<i64, String> {
    match v {
        T::I(n) => Ok(n),
        T::B(b) => Ok(if b { 1 } else { 0 }),
        T::N => Ok(0),
    }
}

fn to_value(v: T) -> Value {
    match v {
        T::I(n) => Value::Number(n),
        T::B(b) => Value::Bool(b),
        T::N => Value::Null,
    }
}

fn bin_arith(op: Opcode, a: i64, b: i64) -> Result<i64, String> {
    match op {
        Opcode::Add => Ok(a.wrapping_add(b)),
        Opcode::Sub => Ok(a.wrapping_sub(b)),
        Opcode::Mul => Ok(a.wrapping_mul(b)),
        Opcode::Div => {
            if b == 0 {
                Err("Integer division by zero".into())
            } else {
                Ok(a / b)
            }
        }
        Opcode::Mod => {
            if b == 0 {
                Err("Modulo by zero".into())
            } else {
                Ok(a % b)
            }
        }
        _ => Err("typed i64: invalid arith".into()),
    }
}

fn bin_cmp(op: Opcode, a: i64, b: i64) -> bool {
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
    for (i, param) in func.params.iter().enumerate() {
        if let Some(idx) = func.locals.iter().position(|l| l == param) {
            locals[idx] = match args.get(i) {
                Some(Value::Number(n)) => T::I(*n),
                Some(Value::Float(f)) => T::I(*f as i64),
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
                let rhs = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                let i = idx as usize;
                if i >= locals.len() {
                    locals.resize(i + 1, T::N);
                }
                let lhs = as_i(locals[i])?;
                locals[i] = T::I(lhs.wrapping_add(rhs));
            }
            Opcode::Pop => {
                let _ = stack.pop().ok_or("Bytecode stack underflow")?;
            }
            Opcode::Dup => {
                let v = *stack.last().ok_or("Bytecode stack underflow")?;
                stack.push(v);
            }
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod => {
                let right = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                let left = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                stack.push(T::I(bin_arith(code[ip], left, right)?));
            }
            Opcode::Eq | Opcode::Ne | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge => {
                let right = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                let left = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                stack.push(T::B(bin_cmp(code[ip], left, right)));
            }
            Opcode::Not => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                stack.push(T::B(!truthy(v)));
            }
            Opcode::Neg => {
                let n = as_i(stack.pop().ok_or("Bytecode stack underflow")?)?;
                stack.push(T::I(n.wrapping_neg()));
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
            _ => return Err("typed i64: unexpected opcode".into()),
        }
        ip += 1;
    };
    let local_vals: Vec<Value> = locals
        .iter()
        .map(|v| match v {
            T::N => Value::Undefined,
            other => to_value(*other),
        })
        .collect();
    Ok((to_value(result), local_vals))
}
