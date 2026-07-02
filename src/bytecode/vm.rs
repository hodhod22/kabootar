//! Stack bytecode VM (v2.18).

use super::classes::{
    instantiate_class, register_module_classes, register_module_enums, register_module_interfaces,
};
use super::types::{BytecodeClassDef, BytecodeFnDef, BytecodeModule, Constant, GeneratorTryRegion, Opcode};
use crate::ops::{eval_binary_op, get_length, read_index, read_member, write_index, write_member};
use crate::value::{AsyncBody, BytecodeFunction, Environment, Microtask, PromiseValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

const MAX_BYTECODE_STACK: usize = 8192;

pub struct ChunkCursor {
    pub ip: usize,
    pub stack: Vec<Value>,
    pub delegate: Option<Value>,
    pub generator_async: bool,
}

enum ChunkExit {
    Done(Value),
    Yield(Value),
}

/// Resume kind for a suspended generator (`.next(value)` / `.throw(reason)` / `.return(value)`).
#[derive(Debug, Clone)]
pub enum GeneratorResume {
    Next(Value),
    Throw(Value),
    Return(Value),
}

pub fn find_try_region_for_ip(func: &BytecodeFnDef, ip: usize) -> Option<&GeneratorTryRegion> {
    func.try_regions
        .iter()
        .filter(|r| ip >= r.body_start && ip <= r.body_end)
        .max_by_key(|r| r.body_start)
        .or_else(|| {
            if ip > 0 {
                func.try_regions
                    .iter()
                    .filter(|r| {
                        let site = ip - 1;
                        site >= r.body_start && site <= r.body_end
                    })
                    .max_by_key(|r| r.body_start)
            } else {
                None
            }
        })
}

#[inline]
fn push_stack(stack: &mut Vec<Value>, v: Value) -> Result<(), String> {
    if stack.len() >= MAX_BYTECODE_STACK {
        return Err("Bytecode stack overflow (memory safety limit)".into());
    }
    stack.push(v);
    Ok(())
}

fn const_to_value(c: &Constant) -> Value {
    match c {
        Constant::Number(n) => Value::Number(*n),
        Constant::BigInt(s) => Value::BigInt(
            crate::runtime::stdlib::bigint::parse_decimal(s)
                .unwrap_or_else(|_| num_bigint::BigInt::from(0)),
        ),
        Constant::Float(f) => Value::Float(*f),
        Constant::String(s) => Value::String(s.clone()),
        Constant::Bool(b) => Value::Bool(*b),
        Constant::Null => Value::Null,
        Constant::Undefined => Value::Undefined,
        Constant::Nan => Value::Float(f64::NAN),
    }
}

pub fn run_module(module: &BytecodeModule, env: &mut Environment) -> Result<Value, String> {
    for name in &module.imports {
        let imported = crate::modules::import_module_exported(name, env)?;
        if module.pub_imports.iter().any(|m| m == name) {
            for export_name in imported {
                env.mark_exported(export_name);
            }
        }
    }
    register_module_interfaces(module, env);
    register_module_enums(module, env);
    register_module_classes(module, env)?;
    register_functions(module, env)?;
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, local_vals) = run_chunk(
        &module.main_code,
        &module.constants,
        &module.globals,
        &module.main_locals,
        &module.main_immutable_locals,
        &module.arrow_functions,
        &module.classes,
        None,
        Some(module),
        None,
        &mut cursor,
        false,
        env,
    )?;
    let result = match exit {
        ChunkExit::Done(v) => v,
        ChunkExit::Yield(_) => return Err("yield outside generator".into()),
    };
    sync_main_locals(module, &local_vals, env)?;
    refresh_function_closures(module, env);
    for name in &module.exports {
        env.mark_exported(name);
    }
    Ok(result)
}

fn sync_main_locals(
    module: &BytecodeModule,
    local_vals: &[Value],
    env: &mut Environment,
) -> Result<(), String> {
    for (i, name) in module.main_locals.iter().enumerate() {
        let v = env
            .get(name)
            .or_else(|| local_vals.get(i).cloned())
            .unwrap_or(Value::Undefined);
        if matches!(v, Value::Undefined) {
            continue;
        }
        if module.main_immutable_locals.get(i) == Some(&true) {
            env.set_const(name.clone(), v.clone());
        } else {
            env.set(name.clone(), v.clone());
        }
    }
    Ok(())
}

fn register_functions(module: &BytecodeModule, env: &mut Environment) -> Result<(), String> {
    for func in &module.functions {
        let mut f = func.clone();
        if f.globals.is_empty() {
            f.globals = module.globals.clone();
        }
        if f.constants.is_empty() {
            f.constants = module.constants.clone();
        }
        env.set(
            f.name.clone(),
            Value::BytecodeFn(BytecodeFunction {
                def: Rc::new(f),
                closure: env.clone(),
            }),
        );
    }
    Ok(())
}

/// Copy live local slots into `env` so nested `MakeArrowFn` closures capture them.
fn sync_locals_into_env(locals: &[String], local_vals: &[Value], env: &mut Environment) {
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        let Some(v) = local_vals.get(i) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if env.get(name).is_some() {
            let _ = env.assign(name, v.clone());
        } else {
            env.set(name.clone(), v.clone());
        }
    }
}

/// After nested calls mutate captured bindings via `env`, refresh `local_vals`.
fn pull_env_into_local_vals(locals: &[String], local_vals: &mut Vec<Value>, env: &Environment) {
    for (i, name) in locals.iter().enumerate() {
        if name.starts_with("__kab_") {
            continue;
        }
        let Some(v) = env.get(name) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        if i >= local_vals.len() {
            local_vals.resize(i + 1, Value::Undefined);
        }
        local_vals[i] = v.clone();
    }
}

fn store_local_to_env(
    locals: &[String],
    immutable_locals: &[bool],
    i: usize,
    v: &Value,
    env: &mut Environment,
) -> Result<(), String> {
    let Some(name) = locals.get(i) else {
        return Ok(());
    };
    if name.starts_with("__kab_") {
        return Ok(());
    }
    if matches!(v, Value::Undefined) {
        return Ok(());
    }
    if immutable_locals.get(i) == Some(&true) {
        if env.get(name).is_none() {
            env.set_const(name.clone(), v.clone());
        }
    } else if env.get(name).is_some() {
        env.assign(name, v.clone())?;
    } else {
        env.set(name.clone(), v.clone());
    }
    Ok(())
}

fn refresh_function_closures(module: &BytecodeModule, env: &mut Environment) {
    let fn_names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    let data_only = env.clone_excluding(&fn_names);
    let shared_data = data_only.share_bindings();
    let mut fn_table = Environment::child_from(&shared_data);
    let fn_table_handle = fn_table.share_bindings();
    for func in &module.functions {
        if let Some(Value::BytecodeFn(existing)) = env.get(&func.name) {
            fn_table.set(
                func.name.clone(),
                Value::BytecodeFn(BytecodeFunction {
                    def: existing.def.clone(),
                    closure: shared_data.share_bindings(),
                }),
            );
        }
    }
    for func in &module.functions {
        if let Some(Value::BytecodeFn(existing)) = env.get(&func.name) {
            env.set(
                func.name.clone(),
                Value::BytecodeFn(BytecodeFunction {
                    def: existing.def.clone(),
                    closure: fn_table_handle.share_bindings(),
                }),
            );
        }
    }
}

pub fn prepare_exported_bytecode_fn(
    name: &str,
    func: BytecodeFunction,
    module_env: &Environment,
) -> BytecodeFunction {
    let fn_names: Vec<String> = module_env
        .all_binding_names()
        .into_iter()
        .filter(|n| matches!(module_env.get(n), Some(Value::BytecodeFn(_))))
        .collect();
    let exclude: Vec<&str> = fn_names.iter().map(String::as_str).collect();
    let data_only = module_env.clone_excluding(&exclude);
    let shared_data = data_only.share_bindings();
    let mut fn_table = Environment::child_from(&shared_data);
    for sib in &fn_names {
        if sib == name {
            continue;
        }
        let Some(Value::BytecodeFn(sib_fn)) = module_env.get(sib) else {
            continue;
        };
        fn_table.set(
            sib.clone(),
            Value::BytecodeFn(BytecodeFunction {
                def: sib_fn.def.clone(),
                closure: shared_data.share_bindings(),
            }),
        );
    }
    let fn_table_handle = fn_table.share_bindings();
    BytecodeFunction {
        def: func.def,
        closure: fn_table_handle,
    }
}

fn run_chunk(
    code: &[Opcode],
    constants: &[Constant],
    globals: &[String],
    locals: &[String],
    immutable_locals: &[bool],
    arrow_functions: &[BytecodeFnDef],
    classes: &[BytecodeClassDef],
    args: Option<(&BytecodeFnDef, Vec<Value>)>,
    module: Option<&BytecodeModule>,
    resume_local_vals: Option<Vec<Value>>,
    cursor: &mut ChunkCursor,
    generator_mode: bool,
    env: &mut Environment,
) -> Result<(ChunkExit, Vec<Value>), String> {
    let is_fresh = resume_local_vals.is_none();
    let mut local_vals =
        resume_local_vals.unwrap_or_else(|| vec![Value::Undefined; locals.len().max(1)]);
    if is_fresh {
        if let Some((func, arg_vals)) = &args {
            for (i, param) in func.params.iter().enumerate() {
                if let Some(idx) = locals.iter().position(|l| l == param) {
                    local_vals[idx] = arg_vals.get(i).cloned().unwrap_or(Value::Undefined);
                }
            }
        }
    }
    while cursor.ip < code.len() {
        let ip = &mut cursor.ip;
        let stack = &mut cursor.stack;
        match &code[*ip] {
            Opcode::Const(idx) => {
                let c = constants
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid const index {idx}"))?;
                push_stack(stack, const_to_value(c))?;
            }
            Opcode::LoadLocal(idx) => {
                let i = *idx as usize;
                let is_param = match &args {
                    Some((func, _)) => locals
                        .get(i)
                        .is_some_and(|name| func.params.iter().any(|p| p == name)),
                    None => false,
                };
                let v = if args.is_none() || !is_param {
                    if let Some(name) = locals.get(i) {
                        if name.starts_with("__kab_") {
                            local_vals
                                .get(i)
                                .cloned()
                                .unwrap_or(Value::Undefined)
                        } else {
                            env.get(name).unwrap_or_else(|| {
                                local_vals
                                    .get(i)
                                    .cloned()
                                    .unwrap_or(Value::Undefined)
                            })
                        }
                    } else {
                        local_vals
                            .get(i)
                            .cloned()
                            .unwrap_or(Value::Undefined)
                    }
                } else {
                    local_vals
                        .get(i)
                        .cloned()
                        .unwrap_or(Value::Undefined)
                };
                push_stack(stack, v)?;
            }
            Opcode::StoreLocal(idx) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let i = *idx as usize;
                if i >= local_vals.len() {
                    local_vals.resize(i + 1, Value::Undefined);
                }
                if immutable_locals.get(i) == Some(&true)
                    && !matches!(local_vals.get(i), Some(Value::Undefined))
                {
                    let name = locals
                        .get(i)
                        .map(String::as_str)
                        .unwrap_or("binding");
                    return Err(format!("Cannot assign to const `{name}`"));
                }
                local_vals[i] = v.clone();
                store_local_to_env(locals, immutable_locals, i, &v, env)?;
                // Closure refresh runs once at end of `run_module`; refreshing on every
                // module-level StoreLocal clones the full environment and breaks lazy
                // iterators whose state lives only in `__kab_*` local slots.
            }
            Opcode::LoadGlobal(idx) => {
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?;
                let v = env
                    .get(name)
                    .ok_or_else(|| crate::evaluator::undefined_var_message(name, env))?;
                push_stack(stack, v)?;
            }
            Opcode::StoreGlobal(idx) => {
                let name = globals
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid global index {idx}"))?
                    .clone();
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                if env.get(&name).is_some() {
                    env.assign(&name, v)?;
                } else {
                    env.set(name, v);
                }
            }
            Opcode::Pop => {
                let _ = stack.pop().ok_or("Bytecode stack underflow")?;
            }
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::And
            | Opcode::Or
            | Opcode::In
            | Opcode::BitAnd
            | Opcode::BitOr
            | Opcode::BitXor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Ushr => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                let op = code[*ip];
                push_stack(stack, eval_binary_op(&left, &opcode_to_binop(op), &right, env)?)?;
            }
            Opcode::JumpIfNotNullish(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !v.is_null() && !v.is_undefined() {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::Not => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Bool(!v.is_truthy()))?;
            }
            Opcode::Neg => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let negated = match v {
                    Value::Number(n) => Value::Number(-n),
                    Value::Float(f) => Value::Float(-f),
                    other => {
                        if let Some(r) = crate::runtime::stdlib::bigint::try_neg(&other) {
                            r?
                        } else {
                            return Err(format!("Cannot negate {other:?}"));
                        }
                    }
                };
                push_stack(stack, negated)?;
            }
            Opcode::BitNot => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, crate::ops::eval_unary_bitnot(&v)?)?;
            }
            Opcode::Jump(off) => {
                *ip = ((*ip as i32 + 1) + off) as usize;
                continue;
            }
            Opcode::JumpIfFalse(off) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                if !v.is_truthy() {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::Call(argc) => {
                let n = *argc as usize;
                if stack.len() < n + 1 {
                    return Err("Bytecode stack underflow on call".into());
                }
                let callee = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut call_args = Vec::with_capacity(n);
                for _ in 0..n {
                    call_args.push(stack.pop().ok_or("Bytecode stack underflow")?);
                }
                call_args.reverse();
                let result = call_value(callee, call_args, constants, globals, arrow_functions, classes, env)?;
                if args.is_some() {
                    pull_env_into_local_vals(locals, &mut local_vals, env);
                }
                push_stack(stack, result)?;
            }
            Opcode::Dup => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow on dup")?
                    .clone();
                push_stack(stack, v)?;
            }
            Opcode::MakeArray(n) => {
                let count = *n as usize;
                if stack.len() < count {
                    return Err("Bytecode stack underflow on make_array".into());
                }
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.insert(0, stack.pop().ok_or("Bytecode stack underflow")?);
                }
                push_stack(stack, Value::Array(items))?;
            }
            Opcode::MakeObject(n) => {
                let count = *n as usize;
                if stack.len() < count * 2 {
                    return Err("Bytecode stack underflow on make_object".into());
                }
                let mut map = std::collections::HashMap::new();
                for _ in 0..count {
                    let key_val = stack.pop().ok_or("Bytecode stack underflow")?;
                    let Value::String(key) = key_val else {
                        return Err("Object key must be a string".into());
                    };
                    let val = stack.pop().ok_or("Bytecode stack underflow")?;
                    map.insert(key, val);
                }
                crate::runtime::stdlib::object::object_oid(&mut map);
                push_stack(stack, Value::Object(map))?;
            }
            Opcode::IndexGet => {
                let idx = stack.pop().ok_or("Bytecode stack underflow")?;
                let container = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, read_index(&container, &idx, env)?)?;
            }
            Opcode::IndexSet => {
                let val = stack.pop().ok_or("Bytecode stack underflow")?;
                let idx = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut container = stack.pop().ok_or("Bytecode stack underflow")?;
                write_index(&mut container, &idx, val.clone(), env)?;
                push_stack(stack, container)?;
                push_stack(stack, val)?;
            }
            Opcode::GetLength => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, get_length(&v)?)?;
            }
            Opcode::GetMember(key_idx) => {
                let key = member_name(constants, *key_idx)?;
                let container = stack.pop().ok_or("Bytecode stack underflow")?;
                let val = if let Value::EnumNamespace(type_name) = &container {
                    crate::class::resolve_enum_member(type_name, &key, env)?
                } else {
                    read_member(&container, &key, env)?
                };
                push_stack(
                    stack,
                    crate::runtime::stdlib::object::bind_object_method(container, val),
                )?;
            }
            Opcode::MemberSet(key_idx) => {
                let key = member_name(constants, *key_idx)?;
                let val = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut container = stack.pop().ok_or("Bytecode stack underflow")?;
                if let Value::ClassInstance(inst) = &container {
                    let guard = inst
                        .try_borrow()
                        .map_err(|e| format!("class instance borrow: {e}"))?;
                    crate::class::type_check::validate_class_field_write(
                        &guard,
                        &key,
                        &val,
                        env.classes(),
                    )?;
                }
                write_member(&mut container, &key, val.clone(), env)?;
                push_stack(stack, container)?;
                push_stack(stack, val)?;
            }
            Opcode::Swap => {
                let top = stack.pop().ok_or("Bytecode stack underflow")?;
                let second = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, top)?;
                push_stack(stack, second)?;
            }
            Opcode::ConcatArray => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                let (Value::Array(mut a), Value::Array(b)) = (left, right) else {
                    return Err("ConcatArray requires two arrays".into());
                };
                a.extend(b);
                push_stack(stack, Value::Array(a))?;
            }
            Opcode::MergeObject => {
                let right = stack.pop().ok_or("Bytecode stack underflow")?;
                let left = stack.pop().ok_or("Bytecode stack underflow")?;
                let (Value::Object(mut a), Value::Object(b)) = (left, right) else {
                    return Err("MergeObject requires two objects".into());
                };
                for (k, v) in b {
                    a.insert(k, v);
                }
                push_stack(stack, Value::Object(a))?;
            }
            Opcode::CallFromArray => {
                let args_arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let callee = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = args_arr else {
                    return Err("CallFromArray requires an array of arguments".into());
                };
                let result = call_value(callee, items, constants, globals, arrow_functions, classes, env)?;
                push_stack(stack, result)?;
            }
            Opcode::MakeOk => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Result(Ok(Box::new(v))))?;
            }
            Opcode::MakeErr => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Result(Err(Box::new(v))))?;
            }
            Opcode::MakeSome => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                push_stack(stack, Value::Option(Some(Box::new(v))))?;
            }
            Opcode::MakeNone => {
                push_stack(stack, Value::Option(None))?;
            }
            Opcode::JumpIfResultErr(off) => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                match v {
                    Value::Result(Err(e)) => {
                        push_stack(stack, *e)?;
                        *ip = ((*ip as i32 + 1) + off) as usize;
                        continue;
                    }
                    Value::Result(Ok(o)) => push_stack(stack, *o)?,
                    other => push_stack(stack, other)?,
                }
            }
            Opcode::ArraySliceFrom(start) => {
                let arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = arr else {
                    return Err("ArraySliceFrom requires an array".into());
                };
                let start = *start as usize;
                push_stack(stack, Value::Array(items.get(start..).unwrap_or(&[]).to_vec()))?;
            }
            Opcode::MakeArrowFn(idx) => {
                sync_locals_into_env(locals, &local_vals, env);
                let arrow = arrow_functions
                    .get(*idx as usize)
                    .ok_or_else(|| format!("Invalid arrow function index {idx}"))?
                    .clone();
                let mut f = arrow;
                if f.globals.is_empty() {
                    f.globals = globals.to_vec();
                }
                if f.constants.is_empty() {
                    f.constants = constants.to_vec();
                }
                push_stack(stack, Value::BytecodeFn(BytecodeFunction {
                    def: Rc::new(f),
                    closure: env.clone(),
                }))?;
            }
            Opcode::JumpUnlessResultOk(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Result(Ok(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapResultOk => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Result(Ok(inner)) = v else {
                    return Err("UnwrapResultOk requires Result Ok".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessResultErr(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Result(Err(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapResultErr => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Result(Err(inner)) = v else {
                    return Err("UnwrapResultErr requires Result Err".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessOptionSome(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Option(Some(_))) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::UnwrapOptionSome => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Option(Some(inner)) = v else {
                    return Err("UnwrapOptionSome requires Option Some".into());
                };
                push_stack(stack, *inner)?;
            }
            Opcode::JumpUnlessOptionNone(off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Option(None)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessConstEq(const_idx, off) => {
                let v = stack
                    .last()
                    .ok_or("Bytecode stack underflow")?;
                let c = constants
                    .get(*const_idx as usize)
                    .ok_or_else(|| format!("Invalid const index {const_idx}"))?;
                let matches = if matches!(c, Constant::Nan) {
                    Value::Bool(v.is_nan())
                } else {
                    let expected = const_to_value(c);
                    eval_binary_op(v, &crate::ast::BinaryOp::Eq, &expected, env)?
                };
                if let Value::Bool(false) = matches {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessArray(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Array(_)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessObject(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                if !matches!(v, Value::Object(_)) {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessObjectEmpty(off) => {
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                let empty = matches!(v, Value::Object(map) if crate::runtime::stdlib::object::object_is_pattern_empty(map));
                if !empty {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::JumpUnlessHasMember(key_idx, off) => {
                let key = member_name(constants, *key_idx)?;
                let v = stack.last().ok_or("Bytecode stack underflow")?;
                let has = matches!(v, Value::Object(map) if map.contains_key(&key));
                if !has {
                    *ip = ((*ip as i32 + 1) + off) as usize;
                    continue;
                }
            }
            Opcode::IndexPeekFromEnd(from_end) => {
                let from_end = *from_end as usize;
                let peek = {
                    let arr = stack
                        .last()
                        .ok_or("Bytecode stack underflow")?;
                    let Value::Array(items) = arr else {
                        return Err("IndexPeekFromEnd requires an array".into());
                    };
                    if from_end == 0 || from_end > items.len() {
                        return Err("IndexPeekFromEnd index out of range".into());
                    }
                    items[items.len() - from_end].clone()
                };
                push_stack(stack, peek)?;
            }
            Opcode::ArraySliceRest(start_trim, end_trim) => {
                let arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(items) = arr else {
                    return Err("ArraySliceRest requires an array".into());
                };
                let start = *start_trim as usize;
                let end_trim = *end_trim as usize;
                if start + end_trim > items.len() {
                    push_stack(stack, Value::Array(Vec::new()))?;
                } else {
                    push_stack(stack, Value::Array(items[start..items.len() - end_trim].to_vec()))?;
                }
            }
            Opcode::ObjectRest(key_count) => {
                let count = *key_count as usize;
                if stack.len() < count + 1 {
                    return Err("Bytecode stack underflow on object_rest".into());
                }
                let mut exclude = std::collections::HashSet::new();
                for _ in 0..count {
                    let key_val = stack.pop().ok_or("Bytecode stack underflow")?;
                    let Value::String(key) = key_val else {
                        return Err("ObjectRest keys must be strings".into());
                    };
                    exclude.insert(key);
                }
                let obj = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Object(map) = obj else {
                    return Err("ObjectRest requires an object".into());
                };
                let rest: std::collections::HashMap<String, Value> = map
                    .into_iter()
                    .filter(|(k, _)| !exclude.contains(k))
                    .collect();
                push_stack(stack, Value::Object(rest))?;
            }
            Opcode::MatchFail => return Err("No matching pattern".into()),
            Opcode::Await => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let resolved = crate::evaluator::resolve_await_value(v, env)?;
                if args.is_some() {
                    pull_env_into_local_vals(locals, &mut local_vals, env);
                }
                push_stack(stack, resolved)?;
            }
            Opcode::Yield => {
                if !generator_mode {
                    return Err("yield outside generator function".into());
                }
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                *ip += 1;
                return Ok((ChunkExit::Yield(v), local_vals));
            }
            Opcode::YieldStar => {
                if !generator_mode {
                    return Err("yield* outside generator function".into());
                }
                let iterable = stack.pop().ok_or("Bytecode stack underflow")?;
                let mut it = if cursor.generator_async {
                    crate::runtime::stdlib::async_iterator::get_async_iterator(&iterable, env)?
                } else {
                    crate::runtime::stdlib::iterator::get_sync_iterator(&iterable, env)?
                };
                let (value, done) =
                    delegate_next_step(&mut it, None, cursor.generator_async, env)?;
                if done {
                    push_stack(stack, value)?;
                } else {
                    cursor.delegate = Some(it);
                    return Ok((ChunkExit::Yield(value), local_vals));
                }
            }
            Opcode::IteratorStepInPlace => {
                let mut it = stack.pop().ok_or("Bytecode stack underflow")?;
                let (value, done) =
                    crate::runtime::stdlib::iterator::iterator_next(&mut it, env)?;
                push_stack(stack, it)?;
                push_stack(
                    stack,
                    crate::runtime::stdlib::iterator::iterator_result(value, done),
                )?;
            }
            Opcode::AsyncIteratorStepInPlace => {
                let mut it = stack.pop().ok_or("Bytecode stack underflow")?;
                let next_p =
                    crate::runtime::stdlib::async_iterator::async_iterator_next(&mut it, env)?;
                push_stack(stack, it)?;
                push_stack(stack, next_p)?;
            }
            Opcode::NewInstance(class_idx, argc) => {
                let n = *argc as usize;
                if stack.len() < n {
                    return Err("Bytecode stack underflow on new_instance".into());
                }
                let mut call_args = Vec::with_capacity(n);
                for _ in 0..n {
                    call_args.push(stack.pop().ok_or("Bytecode stack underflow")?);
                }
                call_args.reverse();
                let class = classes
                    .get(*class_idx as usize)
                    .ok_or_else(|| format!("Invalid class index {class_idx}"))?;
                let instance = instantiate_class(class, classes, call_args, env)?;
                push_stack(stack, instance)?;
            }
            Opcode::NewInstanceFromArray(class_idx) => {
                let arr = stack.pop().ok_or("Bytecode stack underflow")?;
                let Value::Array(call_args) = arr else {
                    return Err("Spread constructor requires an array of arguments".into());
                };
                let class = classes
                    .get(*class_idx as usize)
                    .ok_or_else(|| format!("Invalid class index {class_idx}"))?;
                let instance = instantiate_class(class, classes, call_args, env)?;
                push_stack(stack, instance)?;
            }
            Opcode::GetSuperMethod(key_idx) => {
                let member_name = member_name(constants, *key_idx)?;
                let this_val = env
                    .get("self")
                    .ok_or_else(|| "`super` used outside of method".to_string())?;
                let Value::ClassInstance(inst) = this_val else {
                    return Err("`super` requires class instance `self`".into());
                };
                let v = crate::evaluator::resolve_super_member(&inst, &member_name, env)?;
                push_stack(stack, v)?;
            }
            Opcode::ResultQuestion => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                match v {
                    Value::Result(Ok(inner)) => push_stack(stack, *inner)?,
                    Value::Result(Err(e)) => push_stack(stack, Value::Result(Err(e)))?,
                    other => {
                        return Err(format!(
                            "? operator requires Result (Ok/Err), got {}",
                            crate::value::format_value(&other)
                        ))?;
                    }
                }
            }
            Opcode::Throw => {
                let v = stack.pop().ok_or("Bytecode stack underflow")?;
                let ip_now = *ip;
                let region = args
                    .as_ref()
                    .and_then(|(func, _)| find_try_region_for_ip(func, ip_now))
                    .or_else(|| {
                        module.and_then(|m| {
                            m.main_try_regions
                                .iter()
                                .filter(|r| ip_now >= r.body_start && ip_now <= r.body_end)
                                .max_by_key(|r| r.body_start)
                        })
                    });
                if let Some(region) = region {
                    let li = region.err_local as usize;
                    if li >= local_vals.len() {
                        local_vals.resize(li + 1, Value::Undefined);
                    }
                    let caught = crate::runtime::stdlib::error::enrich_error_value_for_catch(v.clone());
                    local_vals[li] = caught.clone();
                    store_local_to_env(locals, immutable_locals, li, &caught, env)?;
                    push_stack(stack, caught)?;
                    *ip = region.catch_start;
                    continue;
                }
                return Err(crate::runtime::stdlib::error::throw_value(v));
            }
            Opcode::Return => {
                let v = stack.pop().unwrap_or(Value::Undefined);
                return Ok((ChunkExit::Done(v), local_vals));
            }
            Opcode::Halt => {
                return Ok((
                    ChunkExit::Done(stack.pop().unwrap_or(Value::Null)),
                    local_vals,
                ));
            }
        }
        *ip += 1;
    }
    Ok((
        ChunkExit::Done(cursor.stack.pop().unwrap_or(Value::Null)),
        local_vals,
    ))
}

/// After a bytecode call, copy mutated locals (e.g. object params) back into `env` bindings.
fn sync_fn_locals_to_env(func: &BytecodeFnDef, local_vals: &[Value], env: &mut Environment) {
    for (i, name) in func.locals.iter().enumerate() {
        let Some(v) = local_vals.get(i) else {
            continue;
        };
        if matches!(v, Value::Undefined) {
            continue;
        }
        env.set(name.clone(), v.clone());
    }
}

pub fn run_bytecode_fn(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    run_bytecode_fn_with_locals(func, args, env).map(|(v, _)| v)
}

pub fn run_bytecode_fn_with_locals(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    env: &mut Environment,
) -> Result<(Value, Vec<Value>), String> {
    bind_bytecode_params(func, &args, env);
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, local_vals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        &[],
        Some((func, args)),
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    sync_fn_locals_to_env(func, &local_vals, env);
    let v = match exit {
        ChunkExit::Done(v) => v,
        ChunkExit::Yield(_) => return Err("yield outside generator".into()),
    };
    Ok((v, local_vals))
}

pub fn run_expr_snippet(
    code: &[Opcode],
    constants: &[Constant],
    globals: &[String],
    env: &mut Environment,
) -> Result<Value, String> {
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, _) = run_chunk(
        code,
        constants,
        globals,
        &[],
        &[],
        &[],
        &[],
        None,
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    match exit {
        ChunkExit::Done(v) => Ok(v),
        ChunkExit::Yield(_) => Err("yield outside generator".into()),
    }
}

pub fn run_generator_step(
    func: &BytecodeFnDef,
    arg_vals: &[Value],
    local_vals: &mut Vec<Value>,
    cursor: &mut ChunkCursor,
    env: &mut Environment,
    resume: Option<GeneratorResume>,
) -> Result<(Value, bool), String> {
    if let Some(GeneratorResume::Return(value)) = resume {
        if cursor.delegate.is_some() {
            let mut del = cursor
                .delegate
                .take()
                .ok_or("internal generator delegate missing")?;
            let (completion, done) =
                delegate_return_step(&mut del, value.clone(), cursor.generator_async, env)?;
            if !done {
                cursor.delegate = Some(del);
                return Ok((completion, false));
            }
            return Ok((value, true));
        }
        return Ok((value, true));
    }
    let mut resume = resume;
    if cursor.delegate.is_some() {
        if let Some(yielded) = continue_generator_delegate(cursor, resume.take(), env)? {
            return Ok((yielded, false));
        }
    } else if let Some(res) = resume.take() {
        match res {
            GeneratorResume::Next(v) => {
                cursor.stack.push(v);
            }
            GeneratorResume::Throw(reason) => {
                let ip = cursor.ip;
                let Some(region) = find_try_region_for_ip(func, ip) else {
                    return Err(format!(
                        "uncaught generator throw: {}",
                        crate::value::format_value(&reason)
                    ));
                };
                let li = region.err_local as usize;
                if li >= local_vals.len() {
                    local_vals.resize(li + 1, Value::Undefined);
                }
                local_vals[li] = reason.clone();
                cursor.stack.push(reason);
                cursor.ip = region.catch_start;
            }
            GeneratorResume::Return(_) => unreachable!("return handled above"),
        }
    }
    let (exit, updated_locals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        &[],
        Some((func, arg_vals.to_vec())),
        None,
        Some(std::mem::take(local_vals)),
        cursor,
        true,
        env,
    )?;
    *local_vals = updated_locals;
    match exit {
        ChunkExit::Yield(v) => Ok((v, false)),
        ChunkExit::Done(v) => Ok((v, true)),
    }
}

/// Advance an active `yield*` delegate; returns `Some(value)` when another value should be yielded.
fn continue_generator_delegate(
    cursor: &mut ChunkCursor,
    resume: Option<GeneratorResume>,
    env: &mut Environment,
) -> Result<Option<Value>, String> {
    let mut del = cursor
        .delegate
        .take()
        .ok_or("internal generator delegate missing")?;
    let (value, done) = match resume {
        Some(GeneratorResume::Throw(reason)) => {
            let result = delegate_throw_step(&mut del, reason, cursor.generator_async, env)?;
            crate::runtime::stdlib::iterator::parse_iterator_result(&result)?
        }
        Some(GeneratorResume::Return(_)) => {
            return Err("internal: return resume in delegate continue".into());
        }
        other => delegate_next_step(&mut del, other, cursor.generator_async, env)?,
    };
    if !done {
        cursor.delegate = Some(del);
        return Ok(Some(value));
    }
    cursor.stack.push(value);
    cursor.delegate = None;
    cursor.ip += 1;
    Ok(None)
}

fn delegate_throw_step(
    del: &mut Value,
    reason: Value,
    host_async: bool,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        return crate::runtime::stdlib::generator::throw_generator(del, reason, env);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        if let Value::Object(map) = del {
            if let Some(mut sync) = map.get(crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE).cloned() {
                let result =
                    crate::runtime::stdlib::iterator::iterator_throw(&mut sync, reason, env)?;
                map.insert(
                    crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE.into(),
                    sync,
                );
                return Ok(result);
            }
        }
    }
    crate::runtime::stdlib::iterator::iterator_throw(del, reason, env)
}

fn delegate_return_step(
    del: &mut Value,
    value: Value,
    host_async: bool,
    env: &mut Environment,
) -> Result<(Value, bool), String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        let result = crate::runtime::stdlib::generator::return_generator(del, value, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        if let Value::Object(map) = del {
            if let Some(mut sync) = map.get(crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE).cloned() {
                let result =
                    crate::runtime::stdlib::iterator::iterator_return(&mut sync, value, env)?;
                map.insert(
                    crate::runtime::stdlib::async_iterator::ASYNC_SYNC_DELEGATE.into(),
                    sync,
                );
                return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
            }
        }
    }
    let result = crate::runtime::stdlib::iterator::iterator_return(del, value, env)?;
    crate::runtime::stdlib::iterator::parse_iterator_result(&result)
}

fn delegate_next_step(
    del: &mut Value,
    resume: Option<GeneratorResume>,
    host_async: bool,
    env: &mut Environment,
) -> Result<(Value, bool), String> {
    if crate::runtime::stdlib::generator::is_generator_object(del) {
        let gen_resume = match resume {
            Some(GeneratorResume::Next(v)) => Some(GeneratorResume::Next(v)),
            Some(GeneratorResume::Throw(reason)) => Some(GeneratorResume::Throw(reason)),
            Some(GeneratorResume::Return(value)) => {
                return delegate_return_step(del, value, host_async, env);
            }
            None => None,
        };
        let result =
            crate::runtime::stdlib::generator::advance_generator(del, gen_resume, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    if host_async || crate::runtime::stdlib::async_iterator::is_async_iterator_value(del) {
        let _ = resume;
        let next_p = crate::runtime::stdlib::async_iterator::async_iterator_next(del, env)?;
        let result = crate::evaluator::resolve_await_value(next_p, env)?;
        return crate::runtime::stdlib::iterator::parse_iterator_result(&result);
    }
    let _ = resume;
    crate::runtime::stdlib::iterator::iterator_next(del, env)
}

pub fn run_bytecode_fn_with_classes(
    func: &BytecodeFnDef,
    args: Vec<Value>,
    classes: &[BytecodeClassDef],
    env: &mut Environment,
) -> Result<Value, String> {
    bind_bytecode_params(func, &args, env);
    let mut cursor = ChunkCursor {
        ip: 0,
        stack: Vec::new(),
        delegate: None,
        generator_async: false,
    };
    let (exit, local_vals) = run_chunk(
        &func.code,
        &func.constants,
        &func.globals,
        &func.locals,
        &func.immutable_locals,
        &func.arrow_functions,
        classes,
        Some((func, args)),
        None,
        None,
        &mut cursor,
        false,
        env,
    )?;
    sync_fn_locals_to_env(func, &local_vals, env);
    match exit {
        ChunkExit::Done(v) => Ok(v),
        ChunkExit::Yield(_) => Err("yield outside generator".into()),
    }
}

/// Bind bytecode parameter names in `env` so `LoadGlobal` can resolve them from natives.
pub fn bind_bytecode_params(func: &BytecodeFnDef, args: &[Value], env: &mut Environment) {
    for (p, a) in func.params.iter().zip(args) {
        env.set(p.clone(), a.clone());
    }
}

pub fn schedule_bytecode_async(
    func: Rc<BytecodeFnDef>,
    args: Vec<Value>,
    closure: Environment,
    env: &Environment,
) -> Result<Value, String> {
    if func.params.len() != args.len() {
        return Err(format!(
            "Argument count mismatch: expected {}, got {}",
            func.params.len(),
            args.len()
        ));
    }
    let promise: crate::value::SharedPromise =
        Rc::new(RefCell::new(PromiseValue::Pending));
    env.schedule_microtask(Microtask {
        promise: promise.clone(),
        params: func.params.clone(),
        body: AsyncBody::Bytecode(func),
        env: closure,
        args,
        bindings: Vec::new(),
    });
    Ok(Value::Promise(promise))
}

pub fn call_value(
    callee: Value,
    args: Vec<Value>,
    _constants: &[Constant],
    _globals: &[String],
    _arrow_functions: &[BytecodeFnDef],
    classes: &[BytecodeClassDef],
    env: &mut Environment,
) -> Result<Value, String> {
    match callee {
        Value::BytecodeFn(mut func) => {
            if func.def.generator_fn {
                return crate::runtime::stdlib::generator::create_generator(func, args, env);
            }
            crate::runtime::closure_sync::pull_bytecode_globals(&mut func, env);
            crate::runtime::closure_sync::pull_root_into_closure(&mut func.closure, env);
            if func.def.async_fn {
                return schedule_bytecode_async(func.def.clone(), args, func.closure.clone(), env);
            }
            let mut call_env = Environment::child_from(&func.closure);
            let orig_args = args.clone();
            let (result, local_vals) = run_bytecode_fn_with_locals(
                func.def.as_ref(),
                args,
                &mut call_env,
            )?;
            crate::runtime::closure_sync::sync_closure_writes(&func.closure, &call_env, env);
            crate::runtime::closure_sync::sync_bytecode_globals_to_root(&func, &call_env, env);
            crate::runtime::closure_sync::writeback_object_args(
                func.def.as_ref(),
                &orig_args,
                &local_vals,
                env,
            );
            Ok(result)
        }
        Value::NativeFunction(f) => f(&args, env),
        callee if crate::runtime::stdlib::symbol::is_symbol_ctor_object(&callee) => {
            crate::runtime::stdlib::symbol::try_symbol_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::proxy::is_proxy_ctor_object(&callee) => {
            crate::runtime::stdlib::proxy::try_proxy_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::weak::is_weakref_ctor_object(&callee) => {
            crate::runtime::stdlib::weak::try_weakref_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::weak::is_finreg_ctor_object(&callee) => {
            crate::runtime::stdlib::weak::try_finreg_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::intl::is_number_format_ctor(&callee) => {
            crate::runtime::stdlib::intl::try_number_format_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::intl::is_date_time_format_ctor(&callee) => {
            crate::runtime::stdlib::intl::try_date_time_format_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::temporal::is_plain_date_ctor(&callee) => {
            crate::runtime::stdlib::temporal::try_plain_date_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::temporal::is_instant_ctor(&callee) => {
            crate::runtime::stdlib::temporal::try_instant_ctor_call(&callee, &args, env)
                .unwrap_or(Ok(Value::Undefined))
        }
        callee if crate::runtime::stdlib::proxy::is_proxy(&callee) => {
            crate::runtime::stdlib::proxy::trap_apply(
                &callee,
                Value::Undefined,
                args,
                env,
            )
        }
        Value::BoundNative(receiver, f) => {
            let mut recv = (*receiver).clone();
            crate::runtime::stdlib::object::refresh_value_from_env_by_oid(&mut recv, env);
            let mut call_args = vec![recv];
            call_args.extend(args);
            f(&call_args, env)
        }
        Value::PromiseSettler { ctrl_id, reject } => {
            crate::runtime::stdlib::promise::call_settler(ctrl_id, reject, &args, env)
        }
        Value::Function {
            params,
            body,
            env: closure_env,
            async_fn: false,
            ..
        } => {
            if params.len() != args.len() {
                return Err(format!(
                    "Argument count mismatch: expected {}, got {}",
                    params.len(),
                    args.len()
                ));
            }
            let mut call_env = Environment::child(closure_env.clone());
            for (p, a) in params.iter().zip(args) {
                call_env.set(p.clone(), a);
            }
            let result = crate::evaluator::eval_expr(&body, &mut call_env)?;
            crate::runtime::closure_sync::sync_closure_writes(&closure_env, &call_env, env);
            Ok(result)
        }
        Value::BoundMethod(instance, method) => {
            if method.params.len() != args.len() {
                return Err(format!(
                    "Argument count mismatch: expected {}, got {}",
                    method.params.len(),
                    args.len()
                ));
            }
            let owner = {
                let inst_ref = instance
                    .try_borrow()
                    .map_err(|e| format!("class instance borrow: {e}"))?;
                crate::class::method_owner_class(&method, &inst_ref)
            };
            if let Some(bc) = &method.bytecode {
                let mut call_env = crate::evaluator::create_global_env();
                *call_env.classes_mut() = env.classes().clone();
                call_env.set_private_scope(Some(&owner));
                call_env.set(
                    "self".to_string(),
                    Value::ClassInstance(instance.clone()),
                );
                let result = run_bytecode_fn_with_classes(bc, args, classes, &mut call_env)?;
                if env.get("self").is_some() {
                    if let Some(Value::ClassInstance(updated)) = call_env.get("self") {
                        env.assign("self", Value::ClassInstance(updated.clone()))?;
                    }
                }
                return Ok(result);
            }
            let mut call_env = crate::evaluator::create_global_env();
            *call_env.classes_mut() = env.classes().clone();
            call_env.set_private_scope(Some(&owner));
            call_env.set(
                "self".to_string(),
                Value::ClassInstance(instance.clone()),
            );
            for (p, a) in method.params.iter().zip(args) {
                call_env.set(p.clone(), a);
            }
            let result = crate::evaluator::eval_expr(&method.body, &mut call_env)?;
            if env.get("self").is_some() {
                if let Some(Value::ClassInstance(updated)) = call_env.get("self") {
                    env.assign("self", Value::ClassInstance(updated.clone()))?;
                }
            }
            Ok(result)
        }
        Value::EnumCtor {
            type_name,
            variant,
            arity,
        } => crate::class::invoke_enum_ctor(&type_name, &variant, arity, args),
        other => Err(format!("Not a function: {other:?}")),
    }
}

fn member_name(constants: &[Constant], idx: u16) -> Result<String, String> {
    match constants.get(idx as usize) {
        Some(Constant::String(s)) => Ok(s.clone()),
        _ => Err(format!("Invalid member name const index {idx}")),
    }
}

fn opcode_to_binop(op: Opcode) -> crate::ast::BinaryOp {
    use crate::ast::BinaryOp;
    match op {
        Opcode::Add => BinaryOp::Add,
        Opcode::Sub => BinaryOp::Sub,
        Opcode::Mul => BinaryOp::Mul,
        Opcode::Div => BinaryOp::Div,
        Opcode::Mod => BinaryOp::Mod,
        Opcode::Pow => BinaryOp::Pow,
        Opcode::Eq => BinaryOp::Eq,
        Opcode::Ne => BinaryOp::Ne,
        Opcode::Lt => BinaryOp::Lt,
        Opcode::Le => BinaryOp::Le,
        Opcode::Gt => BinaryOp::Gt,
        Opcode::Ge => BinaryOp::Ge,
        Opcode::And => BinaryOp::And,
        Opcode::Or => BinaryOp::Or,
        Opcode::In => BinaryOp::In,
        Opcode::BitAnd => BinaryOp::BitAnd,
        Opcode::BitOr => BinaryOp::BitOr,
        Opcode::BitXor => BinaryOp::BitXor,
        Opcode::Shl => BinaryOp::Shl,
        Opcode::Shr => BinaryOp::Shr,
        Opcode::Ushr => BinaryOp::Ushr,
        _ => BinaryOp::Add,
    }
}

#[cfg(test)]
mod tests {
    use super::run_module;
    use crate::bytecode::compiler::try_compile;
    use crate::bytecode::compile_source;
    use crate::evaluator::create_global_env;
    use crate::Value;

    #[test]
    fn vm_if_return_done_branch() {
        let p = compile_source(
            r#"
            fn stepNext(it) {
              if (it.n < 3) {
                it.n = it.n + 1
                return { value: it.n, done: false }
              }
              return { value: null, done: true }
            }
            stepNext({ n: 3 })
            "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).unwrap();
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).unwrap();
        assert!(matches!(v, Value::Object(_)), "got {v:?}");
    }

    #[test]
    fn vm_return_undefined_object() {
        let p = compile_source(
            r#"
            fn f() {
              return { value: null, done: true }
            }
            f()
            "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).unwrap();
        let mut env = create_global_env();
        let v = run_module(&bc, &mut env).unwrap();
        assert!(matches!(v, Value::Object(_)), "got {v:?}");
    }
}
