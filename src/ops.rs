//! Shared runtime operators for tree-walk and bytecode VMs.

use crate::ast::BinaryOp;
use crate::runtime::stdlib::descriptor::{property_key_from_value, PropertyKey};
use crate::value::{format_value, Environment, Value};

fn to_float(val: &Value) -> Option<f64> {
    match val {
        Value::Number(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    if let Some(eq) = crate::runtime::stdlib::bigint::loose_equal(left, right) {
        return eq;
    }
    match (left, right) {
        (Value::Undefined, Value::Undefined) => true,
        (Value::Null, Value::Null) => true,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::BigInt(a), Value::BigInt(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Number(a), Value::Float(b)) => (*a as f64) == *b,
        (Value::Float(a), Value::Number(b)) => *a == (*b as f64),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        _ => false,
    }
}

/// Python-style `is` — identity for primitives; map/set handles compare by id.
pub(crate) fn values_identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::BigInt(a), Value::BigInt(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        (l, r) if crate::runtime::stdlib::map::is_map_value(l)
            && crate::runtime::stdlib::map::is_map_value(r) =>
        {
            crate::runtime::stdlib::map::map_id(l).ok()
                == crate::runtime::stdlib::map::map_id(r).ok()
        }
        (l, r) if crate::runtime::stdlib::map::is_set_value(l)
            && crate::runtime::stdlib::map::is_set_value(r) =>
        {
            crate::runtime::stdlib::map::set_id(l).ok()
                == crate::runtime::stdlib::map::set_id(r).ok()
        }
        (l, r) if crate::runtime::stdlib::map::is_counter_value(l)
            && crate::runtime::stdlib::map::is_counter_value(r) =>
        {
            crate::runtime::stdlib::map::counter_id(l).ok()
                == crate::runtime::stdlib::map::counter_id(r).ok()
        }
        (l, r) if crate::runtime::stdlib::map::is_defaultdict_value(l)
            && crate::runtime::stdlib::map::is_defaultdict_value(r) =>
        {
            crate::runtime::stdlib::map::defaultdict_id(l).ok()
                == crate::runtime::stdlib::map::defaultdict_id(r).ok()
        }
        (Value::Range { start: a1, end: e1, step: s1 }, Value::Range { start: a2, end: e2, step: s2 }) => {
            a1 == a2 && e1 == e2 && s1 == s2
        }
        _ => false,
    }
}

fn compare_numeric<F>(left: &Value, right: &Value, cmp: F) -> Result<Value, String>
where
    F: Fn(f64, f64) -> bool,
{
    if crate::runtime::stdlib::bigint::is_bigint(left)
        || crate::runtime::stdlib::bigint::is_bigint(right)
    {
        let (a, b) = match (left, right) {
            (Value::BigInt(a), Value::BigInt(b)) => (a, b),
            _ => return Err(format!("Cannot compare {:?} and {:?}", left, right)),
        };
        let ord = a.cmp(b);
        let ok = match ord {
            std::cmp::Ordering::Less => cmp(-1.0, 0.0),
            std::cmp::Ordering::Equal => cmp(0.0, 0.0),
            std::cmp::Ordering::Greater => cmp(1.0, 0.0),
        };
        return Ok(Value::Bool(ok));
    }
    match (to_float(left), to_float(right)) {
        (Some(a), Some(b)) => Ok(Value::Bool(cmp(a, b))),
        _ => Err(format!("Cannot compare {:?} and {:?}", left, right)),
    }
}

fn index_to_usize(idx: &Value, len: usize) -> Result<usize, String> {
    let Value::Number(n) = idx else {
        return Err("Array index must be a number".into());
    };
    if *n < 0 {
        return Err("Array index out of bounds".into());
    }
    let i = *n as usize;
    if i >= len {
        return Err("Array index out of bounds".into());
    }
    Ok(i)
}

pub fn read_index(
    container: &Value,
    idx: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(container) {
        return crate::runtime::stdlib::reflect::reflect_get(container, idx, container, env);
    }
    match (container, idx) {
        (Value::Array(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::async_iterator::SYMBOL_ASYNC_ITERATOR) =>
        {
            crate::runtime::stdlib::async_iterator::symbol_async_iterator_method(container)
                .ok_or_else(|| "value is not async iterable".into())
        }
        (Value::Array(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::iterator::SYMBOL_ITERATOR) =>
        {
            crate::runtime::stdlib::iterator::symbol_iterator_method(container)
                .ok_or_else(|| "value is not iterable".into())
        }
        (Value::String(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::async_iterator::SYMBOL_ASYNC_ITERATOR) =>
        {
            crate::runtime::stdlib::async_iterator::symbol_async_iterator_method(container)
                .ok_or_else(|| "value is not async iterable".into())
        }
        (Value::String(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::iterator::SYMBOL_ITERATOR) =>
        {
            crate::runtime::stdlib::iterator::symbol_iterator_method(container)
                .ok_or_else(|| "value is not iterable".into())
        }
        (v, idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::async_iterator::SYMBOL_ASYNC_ITERATOR)
            && matches!(v, Value::Range { .. }) =>
        {
            crate::runtime::stdlib::async_iterator::symbol_async_iterator_method(container)
                .ok_or_else(|| "value is not async iterable".into())
        }
        (v, idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::iterator::SYMBOL_ITERATOR)
            && matches!(v, Value::Range { .. }) =>
        {
            crate::runtime::stdlib::iterator::symbol_iterator_method(container)
                .ok_or_else(|| "value is not iterable".into())
        }
        (Value::Object(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::async_iterator::SYMBOL_ASYNC_ITERATOR) =>
        {
            if let Some(method) =
                crate::runtime::stdlib::async_iterator::symbol_async_iterator_method(container)
            {
                return Ok(method);
            }
            let Value::Object(map) = container else {
                return Err("value is not async iterable".into());
            };
            return crate::runtime::stdlib::descriptor::get_own_symbol(
                map,
                crate::runtime::stdlib::async_iterator::SYMBOL_ASYNC_ITERATOR,
                container,
                env,
            )
            .map(|v| v.unwrap_or(Value::Undefined));
        }
        (Value::Object(_), idx) if crate::runtime::stdlib::symbol::symbol_id(idx)
            == Some(crate::runtime::stdlib::iterator::SYMBOL_ITERATOR) =>
        {
            if let Some(method) = crate::runtime::stdlib::iterator::symbol_iterator_method(container)
            {
                return Ok(method);
            }
            let Value::Object(map) = container else {
                return Err("value is not iterable".into());
            };
            return crate::runtime::stdlib::descriptor::get_own_symbol(
                map,
                crate::runtime::stdlib::iterator::SYMBOL_ITERATOR,
                container,
                env,
            )
            .map(|v| v.unwrap_or(Value::Undefined));
        }
        (Value::Array(items), idx) => {
            let i = index_to_usize(idx, items.len())?;
            Ok(items[i].clone())
        }
        (Value::String(s), idx) => {
            let i = index_to_usize(idx, s.chars().count())?;
            let ch = s.chars().nth(i).unwrap();
            Ok(Value::String(ch.to_string()))
        }
        (Value::Object(map), idx) => {
            if let Some(sym_id) = crate::runtime::stdlib::symbol::symbol_id(idx) {
                return crate::runtime::stdlib::descriptor::get_own_symbol(
                    map,
                    sym_id,
                    container,
                    env,
                )
                .map(|v| v.unwrap_or(Value::Undefined));
            }
            let Value::String(key) = idx else {
                return Err("Object index requires string or symbol".into());
            };
            crate::runtime::stdlib::descriptor::get_own_property(map, key, container, env)
                .map(|v| v.unwrap_or(Value::Undefined))
        }
        _ => Err("Invalid index access".into()),
    }
}

pub fn get_length(value: &Value) -> Result<Value, String> {
    match value {
        Value::Array(items) => Ok(Value::Number(items.len() as i64)),
        Value::String(s) => Ok(Value::Number(s.chars().count() as i64)),
        _ => Err("Member access requires object, array, string, or class instance".into()),
    }
}

pub fn read_member(
    container: &Value,
    field: &str,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(container) {
        return crate::runtime::stdlib::reflect::reflect_get(
            container,
            &Value::String(field.to_string()),
            container,
            env,
        );
    }
    if let Some(id) = crate::runtime::stdlib::abort::signal_id(container) {
        return Ok(match field {
            "aborted" => Value::Bool(crate::runtime::stdlib::abort::is_aborted(id)),
            "reason" => crate::runtime::stdlib::abort::abort_reason(id),
            _ => match container {
                Value::Object(map) => map.get(field).cloned().unwrap_or(Value::Undefined),
                _ => Value::Undefined,
            },
        });
    }
    match container {
        Value::Object(_) => crate::runtime::stdlib::opt::get_member_value(container, field, env),
        Value::ClassInstance(inst) => {
            let inst_ref = inst
                .try_borrow()
                .map_err(|e| format!("class instance borrow: {e}"))?;
            if crate::class::is_private_name(field) {
                let scope = env.private_access_class().ok_or_else(|| {
                    format!("Cannot read private member {field} outside class method")
                })?;
                if !crate::class::can_access_private_member(field, &scope, env.classes()) {
                    return Err(format!(
                        "Class {} cannot access private member {field}",
                        scope
                    ));
                }
                if let Some(v) = inst_ref.private_fields.get(field) {
                    return Ok(v.clone());
                }
                if let Some(method) = inst_ref.private_methods.get(field) {
                    return Ok(Value::BoundMethod(inst.clone(), method.clone()));
                }
                return Err(format!(
                    "Class {} has no private member {}",
                    inst_ref.class_name, field
                ));
            }
            if let Some(v) = inst_ref.fields.get(field) {
                Ok(v.clone())
            } else if let Some(method) = inst_ref.methods.get(field) {
                Ok(Value::BoundMethod(inst.clone(), method.clone()))
            } else {
                Err(format!(
                    "Class {} has no member {}",
                    inst_ref.class_name, field
                ))
            }
        }
        Value::Array(items) if field == "length" => Ok(Value::Number(items.len() as i64)),
        Value::Array(items) if field == "toLocaleString" => Ok(Value::BoundNative(
            Box::new(Value::Array(items.clone())),
            crate::runtime::stdlib::array_to_locale_string_method,
        )),
        Value::String(s) if field == "length" => Ok(Value::Number(s.chars().count() as i64)),
        Value::String(s) if field == "matchAll" => Ok(Value::BoundNative(
            Box::new(Value::String(s.clone())),
            crate::runtime::stdlib::str_match_all_method,
        )),
        Value::String(s) if field == "toLocaleString" => Ok(Value::BoundNative(
            Box::new(Value::String(s.clone())),
            crate::runtime::stdlib::str_to_locale_string_method,
        )),
        Value::String(s) if field == "localeCompare" => Ok(Value::BoundNative(
            Box::new(Value::String(s.clone())),
            crate::runtime::stdlib::str_locale_compare_method,
        )),
        Value::KabootarDom(node) => match field {
            "tag" => Ok(Value::String(node.tag.clone())),
            "id" => Ok(Value::Number(node.id as i64)),
            "childCount" => Ok(Value::Number(node.children.len() as i64)),
            "layer" => Ok(Value::String("kabootar".into())),
            _ => Err(format!("Kabootar DOM node has no member {}", field)),
        },
        Value::KabootarBrowser(browser) => match field {
            "location" => Ok(Value::String(browser.location().unwrap_or_default())),
            "userAgent" => Ok(Value::String(browser.user_agent().unwrap_or_default())),
            "layer" => Ok(Value::String("kabootar".into())),
            _ => Err(format!("Kabootar browser has no member {}", field)),
        },
        _ => Err("Member access requires object, array, string, or class instance".into()),
    }
}

pub fn write_index(
    container: &mut Value,
    idx: &Value,
    val: Value,
    env: &mut Environment,
) -> Result<(), String> {
    if crate::runtime::stdlib::proxy::is_proxy(container) {
        let receiver = container.clone();
        let ok = crate::runtime::stdlib::reflect::reflect_set(
            container,
            idx,
            val,
            &receiver,
            env,
        )?;
        return if ok {
            Ok(())
        } else {
            Err("Proxy set trap returned false".into())
        };
    }
    let receiver = container.clone();
    match (container, idx) {
        (Value::Array(items), idx) => {
            let i = index_to_usize(idx, items.len())?;
            items[i] = val;
            Ok(())
        }
        (Value::Object(map), idx) => {
            if let Some(sym_id) = crate::runtime::stdlib::symbol::symbol_id(idx) {
                return crate::runtime::stdlib::descriptor::set_own_symbol(
                    map,
                    sym_id,
                    val,
                    &receiver,
                    env,
                );
            }
            let Value::String(key) = idx else {
                return Err("Object index requires string or symbol".into());
            };
            crate::runtime::stdlib::descriptor::set_own_property(map, key, val, &receiver, env)
        }
        _ => Err("Invalid index assignment".into()),
    }
}

pub fn write_member(
    container: &mut Value,
    field: &str,
    val: Value,
    env: &mut Environment,
) -> Result<(), String> {
    if crate::runtime::stdlib::proxy::is_proxy(container) {
        let receiver = container.clone();
        let key = Value::String(field.to_string());
        let ok = crate::runtime::stdlib::reflect::reflect_set(
            container,
            &key,
            val,
            &receiver,
            env,
        )?;
        return if ok {
            Ok(())
        } else {
            Err(format!("Proxy set trap returned false for \"{field}\""))
        };
    }
    let receiver = container.clone();
    match container {
        Value::Object(map) => {
            if crate::runtime::browser_platform::canvas_props::try_write_property(map, field, &val)?
            {
                return Ok(());
            }
            crate::runtime::stdlib::descriptor::set_own_property(map, field, val, &receiver, env)
        }
        Value::ClassInstance(inst) => {
            let mut inst_ref = inst
                .try_borrow_mut()
                .map_err(|e| format!("class instance borrow_mut: {e}"))?;
            if crate::class::is_private_name(field) {
                let scope = env.private_access_class().ok_or_else(|| {
                    format!("Cannot write private member {field} outside class method")
                })?;
                if !crate::class::can_access_private_member(field, &scope, env.classes()) {
                    return Err(format!(
                        "Class {} cannot access private member {field}",
                        scope
                    ));
                }
                if !inst_ref.private_fields.contains_key(field) {
                    return Err(format!(
                        "Class {} has no private field {}",
                        inst_ref.class_name, field
                    ));
                }
                inst_ref.private_fields.insert(field.to_string(), val);
                return Ok(());
            }
            if !inst_ref.fields.contains_key(field) {
                return Err(format!(
                    "Class {} has no field {}",
                    inst_ref.class_name, field
                ));
            }
            inst_ref.fields.insert(field.to_string(), val);
            Ok(())
        }
        _ => Err("Member assignment requires object or class instance".into()),
    }
}

pub fn eval_binary_op(
    left: &Value,
    op: &BinaryOp,
    right: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    match op {
        BinaryOp::Add => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_add(left, right) {
                return r;
            }
            match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, format_value(b)))),
            (a, Value::String(b)) => Ok(Value::String(format!("{}{}", format_value(a), b))),
            _ => Err(format!("Cannot add {:?} and {:?}", left, right)),
            }
        }
        BinaryOp::Sub => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_sub(left, right) {
                return r;
            }
            match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(format!("Cannot subtract {:?} and {:?}", left, right)),
            }
        }
        BinaryOp::Mul => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_mul(left, right) {
                return r;
            }
            match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(format!("Cannot multiply {:?} and {:?}", left, right)),
            }
        }
        BinaryOp::Div => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_div(left, right) {
                return r;
            }
            match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if *b == 0 {
                    Err("Integer division by zero".into())
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a / *b as f64)),
            _ => Err(format!("Cannot divide {:?} and {:?}", left, right)),
            }
        }
        BinaryOp::Mod => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_mod(left, right) {
                return r;
            }
            match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if *b == 0 {
                    Err("Modulo by zero".into())
                } else {
                    Ok(Value::Number(a % b))
                }
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
            (Value::Number(a), Value::Float(b)) => Ok(Value::Float(*a as f64 % b)),
            (Value::Float(a), Value::Number(b)) => Ok(Value::Float(a % (*b as f64))),
            _ => Err(format!("Cannot modulo {:?} and {:?}", left, right)),
            }
        }
        BinaryOp::Pow => {
            if let Some(r) = crate::runtime::stdlib::bigint::try_pow(left, right) {
                return r;
            }
            eval_pow(left, right)
        }
        BinaryOp::Eq => Ok(Value::Bool(values_equal(left, right))),
        BinaryOp::Ne => Ok(Value::Bool(!values_equal(left, right))),
        BinaryOp::Lt => compare_numeric(left, right, |a, b| a < b),
        BinaryOp::Le => compare_numeric(left, right, |a, b| a <= b),
        BinaryOp::Gt => compare_numeric(left, right, |a, b| a > b),
        BinaryOp::Ge => compare_numeric(left, right, |a, b| a >= b),
        BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
        BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
        BinaryOp::NullishCoalesce => {
            if left.is_null() || left.is_undefined() {
                Ok(right.clone())
            } else {
                Ok(left.clone())
            }
        }
        BinaryOp::In => eval_value_in(left, right, env),
        BinaryOp::Is => Ok(Value::Bool(values_identical(left, right))),
        BinaryOp::IsNot => Ok(Value::Bool(!values_identical(left, right))),
        BinaryOp::BitAnd => Ok(from_int32(to_int32(left)? & to_int32(right)?)),
        BinaryOp::BitOr => Ok(from_int32(to_int32(left)? | to_int32(right)?)),
        BinaryOp::BitXor => Ok(from_int32(to_int32(left)? ^ to_int32(right)?)),
        BinaryOp::Shl => {
            let count = to_uint32(right)? & 0x1f;
            Ok(from_int32(to_int32(left)? << count))
        }
        BinaryOp::Shr => {
            let count = to_uint32(right)? & 0x1f;
            Ok(from_int32(to_int32(left)? >> count))
        }
        BinaryOp::Ushr => {
            let count = to_uint32(right)? & 0x1f;
            Ok(from_int32((to_uint32(left)? >> count) as i32))
        }
    }
}

fn to_int32(v: &Value) -> Result<i32, String> {
    let n = match v {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        other => return Err(format!("Bitwise operands must be numbers, got {:?}", other)),
    };
    if !n.is_finite() {
        return Ok(0);
    }
    let truncated = n.trunc();
    let wrapped = truncated % 2f64.powi(32);
    Ok(wrapped as u32 as i32)
}

fn to_uint32(v: &Value) -> Result<u32, String> {
    Ok(to_int32(v)? as u32)
}

fn from_int32(i: i32) -> Value {
    Value::Number(i as i64)
}

pub fn eval_unary_bitnot(v: &Value) -> Result<Value, String> {
    Ok(from_int32(!to_int32(v)?))
}

fn eval_pow(left: &Value, right: &Value) -> Result<Value, String> {
    let base = match left {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        other => return Err(format!("Cannot exponentiate {:?}", other)),
    };
    let exp = match right {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        other => return Err(format!("Exponent must be a number, got {:?}", other)),
    };
    let out = base.powf(exp);
    if out.fract() == 0.0 && out >= i64::MIN as f64 && out <= i64::MAX as f64 {
        Ok(Value::Number(out as i64))
    } else {
        Ok(Value::Float(out))
    }
}

pub fn eval_value_in(
    needle: &Value,
    haystack: &Value,
    env: &mut Environment,
) -> Result<Value, String> {
    if crate::runtime::stdlib::proxy::is_proxy(haystack) {
        return Ok(Value::Bool(
            crate::runtime::stdlib::reflect::reflect_has(haystack, needle, env)?,
        ));
    }
    let found = match haystack {
        Value::Array(items) => items.iter().any(|v| values_equal(needle, v)),
        Value::String(hay) => match needle {
            Value::String(needle_s) => hay.contains(needle_s.as_str()),
            _ => {
                return Err(format!(
                    "String membership requires string needle, got {:?}",
                    needle
                ));
            }
        },
        Value::Object(_) => {
            crate::runtime::stdlib::reflect::reflect_has(haystack, needle, env)?
        }
        Value::ClassInstance(inst) => match property_key_from_value(needle) {
            Ok(PropertyKey::String(key)) => inst
                .try_borrow()
                .map(|i| i.fields.contains_key(&key))
                .unwrap_or(false),
            Ok(PropertyKey::Symbol(_)) => false,
            Err(e) => return Err(e),
        },
        _ => return Err(format!("Cannot use `in` with {:?}", haystack)),
    };
    Ok(Value::Bool(found))
}
