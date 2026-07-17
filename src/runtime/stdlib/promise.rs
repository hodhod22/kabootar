//! Promise helpers — JS `Promise` parity (microtask drain, `Result::Err` = rejection).

use crate::evaluator::{drain_scheduler_step, resolve_await_value};
use crate::value::{Environment, PromiseValue, SharedPromise, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CTRL_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static PROMISE_CTRL: RefCell<HashMap<u64, SharedPromise>> = RefCell::new(HashMap::new());
}

fn call_fn(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    match func {
        Value::BytecodeFn(f) => {
            if f.def.async_fn {
                return crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env);
            }
            let mut call_env = Environment::child(f.closure.clone());
            crate::bytecode::run_bytecode_fn(f.def.as_ref(), args, &mut call_env)
        }
        Value::Function {
            params,
            defaults,
            rest,
            body,
            env: closure_env,
            async_fn: false,
            ..
        } => {
            let mut call_env = Environment::child(closure_env.clone());
            crate::evaluator::bind_call_params(params, defaults, rest, &args, &mut call_env)?;
            crate::evaluator::eval_expr(body, &mut call_env)
        }
        other => crate::bytecode::call_value(other.clone(), args, &[], &[], &[], &[], env),
    }
}

pub fn promise_rejection_reason(v: &Value) -> Option<Value> {
    match v {
        Value::Result(Err(e)) => Some((**e).clone()),
        _ => None,
    }
}

pub fn is_promise_rejection(v: &Value) -> bool {
    promise_rejection_reason(v).is_some()
}

pub fn unwrap_fulfilled(v: Value) -> Value {
    match v {
        Value::Result(Ok(inner)) => *inner,
        other => other,
    }
}

fn make_rejected(reason: Value) -> Value {
    Value::Result(Err(Box::new(reason)))
}

fn alloc_ctrl(promise: SharedPromise) -> u64 {
    let id = NEXT_CTRL_ID.fetch_add(1, Ordering::Relaxed);
    PROMISE_CTRL.with(|m| m.borrow_mut().insert(id, promise));
    id
}

fn ctrl_promise(ctrl_id: u64) -> Result<SharedPromise, String> {
    PROMISE_CTRL
        .with(|m| m.borrow().get(&ctrl_id).cloned())
        .ok_or_else(|| format!("invalid promise control id {ctrl_id}"))
}

pub fn settle_promise(
    promise: &SharedPromise,
    value: Value,
    reject: bool,
    env: &mut Environment,
) -> Result<(), String> {
    if !matches!(*promise.borrow(), PromiseValue::Pending) {
        return Ok(());
    }
    if reject {
        *promise.borrow_mut() = PromiseValue::Resolved(make_rejected(value));
        return Ok(());
    }
    match value {
        Value::Promise(other) => {
            drain_until_resolved(&other, env)?;
            *promise.borrow_mut() = other.borrow().clone();
        }
        other => *promise.borrow_mut() = PromiseValue::Resolved(other),
    }
    Ok(())
}

pub fn call_settler(
    ctrl_id: u64,
    reject: bool,
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    let promise = ctrl_promise(ctrl_id)?;
    let val = args.first().cloned().unwrap_or(Value::Undefined);
    settle_promise(&promise, val, reject, env)?;
    Ok(Value::Undefined)
}

fn normalize_promise(v: Value) -> SharedPromise {
    match v {
        Value::Promise(p) => p,
        other => Rc::new(RefCell::new(PromiseValue::Resolved(other))),
    }
}

fn array_arg(v: &Value) -> Result<&Vec<Value>, String> {
    match v {
        Value::Array(items) => Ok(items),
        _ => Err("expected array".into()),
    }
}

pub fn drain_until_resolved(promise: &SharedPromise, env: &mut Environment) -> Result<(), String> {
    while matches!(*promise.borrow(), PromiseValue::Pending) {
        if !drain_scheduler_step(env)? {
            return Err("promise never resolved".into());
        }
    }
    Ok(())
}

fn settled_entry(status: &str, key: &str, val: Value) -> Value {
    let mut map = HashMap::new();
    map.insert("status".into(), Value::String(status.into()));
    map.insert(key.into(), val);
    Value::Object(map)
}

fn wrap_promise(value: Value) -> Value {
    Value::Promise(Rc::new(RefCell::new(PromiseValue::Resolved(value))))
}

fn as_promise_value(value: Value) -> Value {
    match value {
        Value::Promise(p) => Value::Promise(p),
        other => wrap_promise(other),
    }
}

fn is_skipped_handler(v: &Value) -> bool {
    matches!(v, Value::Null | Value::Undefined)
}

fn apply_handler(
    func: &Value,
    arg: Value,
    env: &mut Environment,
) -> Result<Value, String> {
    let next = call_fn(func, vec![arg], env)?;
    resolve_await_value(next, env)
}

fn chain_promise(value: Value, env: &mut Environment) -> Result<Value, String> {
    Ok(as_promise_value(resolve_await_value(value, env)?))
}

fn promise_resolve_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let v = args.first().cloned().unwrap_or(Value::Null);
    match v {
        Value::Promise(p) => Ok(Value::Promise(p)),
        other => {
            let promise = Rc::new(RefCell::new(PromiseValue::Pending));
            settle_promise(&promise, other, false, env)?;
            Ok(Value::Promise(promise))
        }
    }
}

fn promise_reject_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let reason = args.first().cloned().unwrap_or(Value::Null);
    Ok(wrap_promise(make_rejected(reason)))
}

fn promise_new_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let executor = args.first().ok_or("promise_new(executor)")?;
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    let ctrl_id = alloc_ctrl(promise.clone());
    let resolve = Value::PromiseSettler {
        ctrl_id,
        reject: false,
    };
    let reject_fn = Value::PromiseSettler {
        ctrl_id,
        reject: true,
    };
    call_fn(executor, vec![resolve, reject_fn], env)?;
    Ok(Value::Promise(promise))
}

fn is_promise_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_promise(v)")?;
    Ok(Value::Bool(matches!(v, Value::Promise(_))))
}

fn collect_all_resolved(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("promise_all(values)")?)?;
    let promises: Vec<SharedPromise> = items.iter().cloned().map(normalize_promise).collect();
    for p in &promises {
        drain_until_resolved(p, env)?;
    }
    let mut out = Vec::with_capacity(promises.len());
    for p in promises {
        let v = match p.borrow().clone() {
            PromiseValue::Resolved(v) => v,
            PromiseValue::Pending => return Err("promise_all() promise remained pending".into()),
        };
        if let Some(reason) = promise_rejection_reason(&v) {
            return Err(format!(
                "promise rejected: {}",
                crate::value::format_value(&reason)
            ));
        }
        out.push(unwrap_fulfilled(v));
    }
    Ok(Value::Array(out))
}

fn await_all_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    collect_all_resolved(args, env)
}

fn promise_all_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    chain_promise(collect_all_resolved(args, env)?, env)
}

fn promise_race_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("promise_race(values)")?)?;
    if items.is_empty() {
        return Err("promise_race() expects a non-empty array".into());
    }
    let promises: Vec<SharedPromise> = items.iter().cloned().map(normalize_promise).collect();
    loop {
        for p in &promises {
            if let PromiseValue::Resolved(v) = p.borrow().clone() {
                if let Some(reason) = promise_rejection_reason(&v) {
                    return chain_promise(make_rejected(reason), env);
                }
                return chain_promise(unwrap_fulfilled(v), env);
            }
        }
        if promises
            .iter()
            .all(|p| !matches!(*p.borrow(), PromiseValue::Pending))
        {
            return Err("promise_race() no promise resolved".into());
        }
        if !drain_scheduler_step(env)? {
            return Err("promise_race() deadlock — promises never resolved".into());
        }
    }
}

fn promise_all_settled_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("promise_all_settled(values)")?)?;
    let promises: Vec<SharedPromise> = items.iter().cloned().map(normalize_promise).collect();
    for p in &promises {
        drain_until_resolved(p, env)?;
    }
    let mut out = Vec::with_capacity(promises.len());
    for p in promises {
        let v = match p.borrow().clone() {
            PromiseValue::Resolved(v) => v,
            PromiseValue::Pending => {
                return Err("promise_all_settled() promise remained pending".into())
            }
        };
        if let Some(reason) = promise_rejection_reason(&v) {
            out.push(settled_entry("rejected", "reason", reason));
        } else {
            out.push(settled_entry(
                "fulfilled",
                "value",
                unwrap_fulfilled(v),
            ));
        }
    }
    chain_promise(Value::Array(out), env)
}

fn promise_any_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let items = array_arg(args.first().ok_or("promise_any(values)")?)?;
    if items.is_empty() {
        return Err("promise_any() expects a non-empty array".into());
    }
    let promises: Vec<SharedPromise> = items.iter().cloned().map(normalize_promise).collect();
    loop {
        let mut all_done = true;
        let mut reasons = Vec::new();
        for p in &promises {
            match p.borrow().clone() {
                PromiseValue::Pending => all_done = false,
                PromiseValue::Resolved(v) => {
                    if let Some(reason) = promise_rejection_reason(&v) {
                        reasons.push(reason);
                    } else {
                        return chain_promise(unwrap_fulfilled(v), env);
                    }
                }
            }
        }
        if all_done {
            return chain_promise(
                make_rejected(Value::Array(reasons)),
                env,
            );
        }
        if !drain_scheduler_step(env)? {
            return Err("promise_any() deadlock — promises never resolved".into());
        }
    }
}

pub(crate) fn promise_then_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let p = args.first().ok_or("promise_then(promise, onFulfilled?, onRejected?)")?;
    let on_fulfilled = args.get(1);
    let on_rejected = args.get(2);
    let Value::Promise(promise) = p else {
        return Err("promise_then() expects a promise".into());
    };
    drain_until_resolved(promise, env)?;
    let state = match promise.borrow().clone() {
        PromiseValue::Resolved(v) => v,
        PromiseValue::Pending => return Err("promise_then() promise remained pending".into()),
    };
    if let Some(reason) = promise_rejection_reason(&state) {
        if let Some(handler) = on_rejected.filter(|h| !is_skipped_handler(h)) {
            return chain_promise(apply_handler(handler, reason, env)?, env);
        }
        return chain_promise(state, env);
    }
    let fulfilled = unwrap_fulfilled(state);
    if let Some(handler) = on_fulfilled.filter(|h| !is_skipped_handler(h)) {
        return chain_promise(apply_handler(handler, fulfilled, env)?, env);
    }
    chain_promise(fulfilled, env)
}

fn promise_catch_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let p = args.first().ok_or("promise_catch(promise, fn)")?;
    let handler = args.get(1).ok_or("promise_catch(promise, fn)")?;
    promise_then_native(&[p.clone(), Value::Null, handler.clone()], env)
}

fn promise_finally_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let p = args.first().ok_or("promise_finally(promise, fn)")?;
    let func = args.get(1).ok_or("promise_finally(promise, fn)")?;
    let Value::Promise(promise) = p else {
        return Err("promise_finally() expects a promise".into());
    };
    drain_until_resolved(promise, env)?;
    let state = promise.borrow().clone();
    call_fn(func, vec![], env)?;
    match state {
        PromiseValue::Resolved(v) => chain_promise(v, env),
        PromiseValue::Pending => Err("promise_finally() promise remained pending".into()),
    }
}

fn promise_with_resolvers_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    let ctrl_id = alloc_ctrl(promise.clone());
    let mut obj = HashMap::new();
    obj.insert("promise".into(), Value::Promise(promise));
    obj.insert(
        "resolve".into(),
        Value::PromiseSettler {
            ctrl_id,
            reject: false,
        },
    );
    obj.insert(
        "reject".into(),
        Value::PromiseSettler {
            ctrl_id,
            reject: true,
        },
    );
    Ok(Value::Object(obj))
}

fn promise_try_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let func = args.first().ok_or("promise_try(fn)")?;
    match call_fn(func, vec![], env) {
        Ok(v) => chain_promise(v, env),
        Err(e) => {
            let reason = crate::runtime::stdlib::error::take_throw_value(&e)
                .unwrap_or_else(|| Value::String(e));
            chain_promise(make_rejected(reason), env)
        }
    }
}

pub fn register_promise(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("promise_new", promise_new_native),
        ("promise", promise_new_native),
        ("promise_resolve", promise_resolve_native),
        ("promise_reject", promise_reject_native),
        ("is_promise", is_promise_native),
        ("await_all", await_all_native),
        ("promise_all", promise_all_native),
        ("promise_race", promise_race_native),
        ("promise_all_settled", promise_all_settled_native),
        ("promise_any", promise_any_native),
        ("promise_then", promise_then_native),
        ("promise_catch", promise_catch_native),
        ("promise_finally", promise_finally_native),
        ("promise_with_resolvers", promise_with_resolvers_native),
        ("promise_try", promise_try_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
    env.set("Promise".to_string(), build_promise_namespace());
}

fn build_promise_namespace() -> Value {
    use std::collections::HashMap;
    let mut m = HashMap::new();
    let insert = |map: &mut HashMap<String, Value>,
                  name: &str,
                  f: fn(&[Value], &mut Environment) -> Result<Value, String>| {
        map.insert(name.into(), Value::NativeFunction(f));
    };
    insert(&mut m, "try", promise_try_native);
    insert(&mut m, "withResolvers", promise_with_resolvers_native);
    insert(&mut m, "resolve", promise_resolve_native);
    insert(&mut m, "reject", promise_reject_native);
    insert(&mut m, "all", promise_all_native);
    insert(&mut m, "race", promise_race_native);
    insert(&mut m, "allSettled", promise_all_settled_native);
    insert(&mut m, "any", promise_any_native);
    insert(&mut m, "then", promise_then_native);
    insert(&mut m, "catch", promise_catch_native);
    insert(&mut m, "finally", promise_finally_native);
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{create_global_env, eval_source};
    use crate::value::PromiseValue;

    #[test]
    fn async_iterator_collect_array_native() {
        use crate::runtime::stdlib::async_iterator::for_await_of_items_with_env;
        let mut env = create_global_env();
        let arr = eval_source("[1, 2, 3]", &mut env).unwrap();
        let items = for_await_of_items_with_env(&arr, &mut env).unwrap();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], Value::Number(1)));
        assert!(matches!(items[1], Value::Number(2)));
        assert!(matches!(items[2], Value::Number(3)));
    }

    #[test]
    fn async_for_await_array() {
        use crate::evaluator::drain_all_microtasks;
        let mut env = create_global_env();
        let v = eval_source(
            r#"
            async fn main() {
              let sum = 0
              for await x of [1, 2, 3] { sum = sum + x }
              return sum
            }
            main()
            "#,
            &mut env,
        )
        .unwrap();
        drain_all_microtasks(&mut env).unwrap();
        let Value::Promise(p) = v else {
            panic!("expected promise, got {:?}", v);
        };
        assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));
    }

    #[test]
    fn sync_for_of_mk_closure_iterator() {
        let mut env = create_global_env();
        let out = eval_source(
            r#"
            fn mk() {
              let i = 0
              return {
                next() {
                  if (i < 3) {
                    i = i + 1
                    return { value: i, done: false }
                  }
                  return { value: null, done: true }
                }
              }
            }
            let sum = 0
            for x of mk() { sum = sum + x }
            sum
            "#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(out, Value::Number(6)));
    }

    #[test]
    fn async_for_await_custom_async_iterator() {
        use crate::evaluator::drain_all_microtasks;
        let mut env = create_global_env();
        let v = eval_source(
            r#"
            async fn main() {
              fn mkAsyncIter() {
                let i = 0
                return {
                  next() {
                    if (i < 3) {
                      i = i + 1
                      return promise_resolve({ value: i, done: false })
                    }
                    return promise_resolve({ value: null, done: true })
                  }
                }
              }
              let o = {}
              o[Symbol.asyncIterator] = mkAsyncIter
              let sum = 0
              for await x of o { sum = sum + x }
              return sum
            }
            main()
            "#,
            &mut env,
        )
        .unwrap();
        drain_all_microtasks(&mut env).unwrap();
        let Value::Promise(p) = v else {
            panic!("expected promise, got {:?}", v);
        };
        assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(6))));
    }

    #[test]
    fn sync_for_of_sync_generator() {
        let mut env = create_global_env();
        let out = eval_source(
            r#"
            fn* gen() {
              yield 1
              yield 2
              yield 3
            }
            let sum = 0
            for x of gen() { sum = sum + x }
            sum
            "#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(out, Value::Number(6)));
    }

    #[test]
    fn iterator_for_each_updates_module_local() {
        let mut env = create_global_env();
        let out = eval_source(
            r#"
            let sum = 0
            iterator_for_each(range(1, 4), (n) => { sum = sum + n })
            sum
            "#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(out, Value::Number(6)), "got {:?}", out);
    }

    #[test]
    fn async_for_await_sync_generator() {
        use crate::evaluator::drain_all_microtasks;
        let mut env = create_global_env();
        let v = eval_source(
            r#"
            async fn main() {
              fn* gen() {
                yield 1
                yield 2
              }
              let sum = 0
              for await x of gen() { sum = sum + x }
              return sum
            }
            main()
            "#,
            &mut env,
        )
        .unwrap();
        drain_all_microtasks(&mut env).unwrap();
        let Value::Promise(p) = v else {
            panic!("expected promise");
        };
        assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(3))));
    }

    #[test]
    fn async_for_await_async_generator_native() {
        use crate::runtime::stdlib::async_iterator::for_await_of_items_with_env;
        let mut env = create_global_env();
        let gen = eval_source(
            r#"
            async fn* gen() {
              yield 10
              yield 20
            }
            gen()
            "#,
            &mut env,
        )
        .unwrap();
        let items = for_await_of_items_with_env(&gen, &mut env).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], Value::Number(10)));
        assert!(matches!(items[1], Value::Number(20)));
    }

    #[test]
    fn async_for_await_async_generator() {
        use crate::evaluator::drain_all_microtasks;
        let mut env = create_global_env();
        let v = eval_source(
            r#"
            async fn main() {
              async fn* gen() {
                yield 10
                yield 20
              }
              let sum = 0
              for await x of gen() { sum = sum + x }
              return sum
            }
            main()
            "#,
            &mut env,
        )
        .unwrap();
        drain_all_microtasks(&mut env).unwrap();
        let Value::Promise(p) = v else {
            panic!("expected promise, got {:?}", v);
        };
        assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(30))));
    }

    #[test]
    fn promise_new_binds_executor_params() {
        let mut env = create_global_env();
        let v = eval_source(
            r#"
            fn go(res, rej) { res(21) }
            promise_new(go)
            "#,
            &mut env,
        )
        .unwrap();
        let Value::Promise(p) = v else {
            panic!("expected promise");
        };
        assert!(matches!(*p.borrow(), PromiseValue::Resolved(Value::Number(21))));
    }
}
