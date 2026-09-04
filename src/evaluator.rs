use crate::ast::*;
use crate::class::{ClassDef, ClassInstance, FieldDef, InterfaceDef, MethodDef, MethodSignature, SharedClassInstance};
use crate::modules;
use crate::runtime::{
    browser_globals, browser_platform_globals, db_globals, http_globals, io_async_globals, kabootar_browser_globals,
    kabootar_dom_globals, kstyle_lang_globals, kv8_globals,     lang_features_globals, os_globals, platform_globals, ecosystem_globals, reality_globals, security_globals, stdlib_globals,
    tls_trust_globals,
};
use crate::value::{format_value, unix_ms_now, AsyncBody, Environment, Microtask, PromiseValue, SharedPromise, SleepWaiter, Value, WakeAt};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn bind_call_params(
    params: &[String],
    defaults: &[Option<crate::ast::Expr>],
    rest: &Option<String>,
    args: &[Value],
    env: &mut Environment,
) -> Result<(), String> {
    if defaults.len() != params.len() {
        return Err("internal: default arity mismatch".into());
    }
    let min_args = defaults
        .iter()
        .position(|d| d.is_some())
        .unwrap_or(params.len());
    if args.len() < min_args {
        return Err(format!(
            "Too few arguments: expected at least {}, got {}",
            min_args,
            args.len()
        ));
    }
    if rest.is_none() && args.len() > params.len() {
        return Err(format!(
            "Too many arguments: expected at most {}, got {}",
            params.len(),
            args.len()
        ));
    }
    if rest.is_some() && args.len() < params.len() {
        return Err(format!(
            "Too few arguments: expected at least {}, got {}",
            params.len(),
            args.len()
        ));
    }
    for (i, name) in params.iter().enumerate() {
        let val = if i < args.len() {
            args[i].clone()
        } else if let Some(def) = &defaults[i] {
            eval_expr(def, env)?
        } else {
            return Err(format!("Missing argument for `{}`", name));
        };
        env.set(name.clone(), val);
    }
    if let Some(rest_name) = rest {
        let tail: Vec<Value> = if args.len() > params.len() {
            args[params.len()..].to_vec()
        } else {
            Vec::new()
        };
        env.set(rest_name.clone(), Value::from_array(tail));
    }
    Ok(())
}

fn make_function_value(
    name: String,
    params: Vec<crate::ast::FnParam>,
    rest: Option<String>,
    body: crate::ast::Expr,
    env: Environment,
    public: bool,
    async_fn: bool,
) -> Value {
    Value::Function {
        name,
        params: crate::ast::fn_param_names(&params),
        defaults: crate::ast::fn_param_defaults(&params),
        rest,
        body,
        env,
        public,
        async_fn,
    }
}

pub fn create_global_env() -> Environment {
    let mut env = Environment::new();
    env.set("println".to_string(), Value::NativeFunction(println_native));
    env.set("log".to_string(), Value::NativeFunction(println_native));
    env.set("console_log".to_string(), Value::NativeFunction(println_native));
    env.set("console_warn".to_string(), Value::NativeFunction(console_warn_native));
    env.set("console_error".to_string(), Value::NativeFunction(console_error_native));
    env.set("is_null".to_string(), Value::NativeFunction(is_null_native));
    env.set("is_undefined".to_string(), Value::NativeFunction(is_undefined_native));
    env.set("is_nan".to_string(), Value::NativeFunction(is_nan_native));
    env.set("len".to_string(), Value::NativeFunction(len_native));
    env.set("push".to_string(), Value::NativeFunction(push_native));
    // Alias used by Kab VM ArrayPushLocal/Global handlers (same semantics as push).
    env.set(
        "bytecode_array_push".to_string(),
        Value::NativeFunction(push_native),
    );
    env.set("pop".to_string(), Value::NativeFunction(pop_native));
    env.set("map".to_string(), Value::NativeFunction(map_native));
    env.set("array_map".to_string(), Value::NativeFunction(map_native));
    env.set("filter".to_string(), Value::NativeFunction(filter_native));
    env.set("array_filter".to_string(), Value::NativeFunction(filter_native));
    env.set("typeof".to_string(), Value::NativeFunction(typeof_native));
    env.set("keys".to_string(), Value::NativeFunction(keys_native));
    env.set("object_keys".to_string(), Value::NativeFunction(keys_native));
    env.set("sleep_ticks".to_string(), Value::NativeFunction(sleep_ticks_native));
    env.set("sleep_ms".to_string(), Value::NativeFunction(sleep_ms_native));
    env.set("set_timeout".to_string(), Value::NativeFunction(set_timeout_native));
    env.set("clear_timeout".to_string(), Value::NativeFunction(clear_timeout_native));
    env.set("set_interval".to_string(), Value::NativeFunction(set_interval_native));
    env.set(
        "set_interval_ticks".to_string(),
        Value::NativeFunction(set_interval_ticks_native),
    );
    env.set("clear_interval".to_string(), Value::NativeFunction(clear_interval_native));
    env.set(
        "queue_microtask".to_string(),
        Value::NativeFunction(queue_microtask_native),
    );
    env.set("is_impl".to_string(), Value::NativeFunction(is_impl_native));
    env.set("instanceof".to_string(), Value::NativeFunction(is_native));
    env.set(
        "bytecode_can_compile".to_string(),
        Value::NativeFunction(bytecode_can_compile_native),
    );
    env.set(
        "bytecode_opt_info".to_string(),
        Value::NativeFunction(bytecode_opt_info_native),
    );
    env.set(
        "bytecode_run_kbc".to_string(),
        Value::NativeFunction(bytecode_run_kbc_native),
    );
    env.set(
        "bytecode_host_get".to_string(),
        Value::NativeFunction(bytecode_host_get_native),
    );
    env.set(
        "bytecode_host_map_get".to_string(),
        Value::NativeFunction(bytecode_host_map_get_native),
    );
    env.set(
        "bytecode_host_call".to_string(),
        Value::NativeFunction(bytecode_host_call_native),
    );
    env.set(
        "bytecode_host_import".to_string(),
        Value::NativeFunction(bytecode_host_import_native),
    );
    env.set(
        "bytecode_iterator_step_in_place".to_string(),
        Value::NativeFunction(bytecode_iterator_step_in_place_native),
    );
    browser_globals(&mut env);
    browser_platform_globals(&mut env);
    kabootar_dom_globals(&mut env);
    kstyle_lang_globals(&mut env);
    kv8_globals(&mut env);
    kabootar_browser_globals(&mut env);
    platform_globals(&mut env);
    os_globals(&mut env);
    db_globals(&mut env);
    http_globals(&mut env);
    io_async_globals(&mut env);
    tls_trust_globals(&mut env);
    crate::runtime::registry::registry_globals(&mut env);
    security_globals(&mut env);
    lang_features_globals(&mut env);
    crate::runtime::ptak::ptak_globals(&mut env);
    stdlib_globals(&mut env);
    reality_globals(&mut env);
    ecosystem_globals(&mut env);
    crate::runtime::game::game_globals(&mut env);
    crate::runtime::ownership::ownership_globals(&mut env);
    modules::register_import_builtins(&mut env);
    env
}

static STDLIB_BUILDS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static STDLIB_PROTOTYPE: RefCell<Option<Environment>> = const { RefCell::new(None) };
}

/// Fresh module frame with natives via parent (P10: one stdlib build per thread).
pub fn create_module_env() -> Environment {
    STDLIB_PROTOTYPE.with(|slot| {
        if slot.borrow().is_none() {
            STDLIB_BUILDS.fetch_add(1, Ordering::Relaxed);
            *slot.borrow_mut() = Some(create_global_env());
        }
        Environment::child_from(slot.borrow().as_ref().expect("stdlib prototype"))
    })
}

pub fn stdlib_prototype_builds() -> u64 {
    STDLIB_BUILDS.load(Ordering::Relaxed)
}

/// Evaluate Kabootar source into an existing environment.
pub fn eval_source(code: &str, env: &mut Environment) -> Result<Value, String> {
    let program = crate::bytecode::compile_source(code)?;
    crate::runtime::ownership::set_memory_mode(env, program.memory_mode);
    if let Some(bc) = &program.bytecode {
        if bc.uses_bytecode() {
            let result = crate::bytecode::run_module(bc, env)?;
            drain_all_microtasks(env)?;
            return Ok(result);
        }
    }
    let mut last = Value::Null;
    let disposable_depth = crate::runtime::stdlib::disposable::disposable_depth();
    for stmt in &program.stmts {
        last = eval_stmt(stmt, env)?;
    }
    crate::runtime::stdlib::disposable::dispose_since(disposable_depth, env);
    drain_all_microtasks(env)?;
    Ok(last)
}

/// Invoke a Kabootar function value with `env` as the call environment.
pub fn call_function_value(func: &Value, env: &mut Environment) -> Result<Value, String> {
    match func {
        Value::Function { body, env: closure, .. } => {
            let mut call_env = Environment::child(closure.clone());
            inject_request_context(&mut call_env, env);
            eval_expr(body, &mut call_env)
        }
        Value::BytecodeFn(f) => {
            let mut call_env = Environment::child(f.closure.clone());
            inject_request_context(&mut call_env, env);
            crate::bytecode::run_bytecode_fn(f.def.as_ref(), vec![], &mut call_env)
        }
        _ => Err("Expected function value".into()),
    }
}

fn inject_request_context(call_env: &mut Environment, env: &Environment) {
    for key in ["req_method", "req_path", "req_body"] {
        if let Some(v) = env.get(key) {
            call_env.set(key.to_string(), v);
        }
    }
}

fn println_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    for arg in args.iter() {
        print!("{} ", format_value(arg));
    }
    println!();
    Ok(Value::Null)
}

fn console_warn_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    print!("[warn] ");
    println_native(args, env)
}

fn console_error_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    eprint!("[error] ");
    for arg in args.iter() {
        eprint!("{} ", format_value(arg));
    }
    eprintln!();
    Ok(Value::Null)
}

fn is_null_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_null() expects 1 argument")?;
    Ok(Value::Bool(v.is_null()))
}

fn is_undefined_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_undefined() expects 1 argument")?;
    Ok(Value::Bool(v.is_undefined()))
}

fn is_nan_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_nan() expects 1 argument")?;
    Ok(Value::Bool(v.is_nan()))
}

fn len_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("len() expects 1 argument")?;
    if let Some(n) = crate::runtime::stdlib::iterator::iterator_len(v) {
        return Ok(Value::Number(n as i64));
    }
    let n = match v {
        Value::Array(items) => items.len(),
        // ASCII (Kab sources) is O(1); non-ASCII still counts Unicode scalars.
        Value::String(s) => {
            if s.is_ascii() {
                s.len()
            } else {
                s.chars().count()
            }
        }
        Value::Object(map) if crate::runtime::stdlib::iterator::is_iterator_value(v) => {
            return Err("len() on this iterator requires consuming iteration (no known length)".into());
        }
        Value::Object(map) => map.len(),
        _ => return Err("len() expects array, string, or object".into()),
    };
    Ok(Value::Number(n as i64))
}

fn push_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arr = args.first().ok_or("push() expects 2 arguments")?;
    let item = args.get(1).ok_or("push() expects 2 arguments")?;
    let Value::Array(items) = arr else {
        return Err("push() first argument must be an array".into());
    };
    // One alloc + extend (avoid Array clone + separate push realloc dance).
    let mut out = Vec::with_capacity(items.len() + 1);
    out.extend_from_slice(items);
    out.push(item.clone());
    Ok(Value::from_array(out))
}

fn pop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let arr = args.first().ok_or("pop() expects 1 argument")?;
    let Value::Array(items) = arr else {
        return Err("pop() expects an array".into());
    };
    if items.is_empty() {
        return Ok(Value::from_array(Vec::new()));
    }
    Ok(Value::from_array(items[..items.len() - 1].to_vec()))
}

fn call_with_args(func: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env)
}

fn map_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let arr = args.first().ok_or("map() expects 2 arguments")?;
    let func = args.get(1).ok_or("map() expects 2 arguments")?;
    let Value::Array(items) = arr else {
        return Err("map() first argument must be an array".into());
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items.iter() {
        out.push(call_with_args(func, vec![item.clone()], env)?);
    }
    Ok(Value::from_array(out))
}

fn filter_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let arr = args.first().ok_or("filter() expects 2 arguments")?;
    let func = args.get(1).ok_or("filter() expects 2 arguments")?;
    let Value::Array(items) = arr else {
        return Err("filter() first argument must be an array".into());
    };
    let mut out = Vec::new();
    for item in items.iter() {
        if call_with_args(func, vec![item.clone()], env)?.is_truthy() {
            out.push(item.clone());
        }
    }
    Ok(Value::from_array(out))
}

fn typeof_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("typeof() expects 1 argument")?;
    Ok(Value::String(
        crate::runtime::stdlib::typeof_name(v).to_string(),
    ))
}

fn keys_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("keys() expects 1 argument")?;
    match v {
        Value::Object(map) => {
            let mut names: Vec<_> = map
                .keys()
                .filter(|k| !k.starts_with("__kab_"))
                .cloned()
                .collect();
            names.sort();
            Ok(Value::from_array(
                names.into_iter().map(Value::String).collect(),
            ))
        }
        Value::Array(items) => Ok(Value::from_array(
            (0..items.len())
                .map(|i| Value::String(i.to_string()))
                .collect(),
        )),
        _ => Err("keys() expects an object or array".into()),
    }
}

fn schedule_sleep_ticks(args: &[Value], env: &mut Environment, callback: Option<(Value, Vec<Value>)>) -> Result<Value, String> {
    let ticks = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => return Err("sleep expects a non-negative integer delay".into()),
    };
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    let id = env.alloc_timer_id();
    let wake = WakeAt::Tick(env.current_tick() + ticks);
    let timer_id = callback.is_some();
    env.schedule_sleep(SleepWaiter {
        id,
        promise: promise.clone(),
        wake,
        callback,
        repeat_interval: None,
        repeat_wall_ms: false,
    });
    Ok(if timer_id {
        Value::Number(id as i64)
    } else {
        Value::Promise(promise)
    })
}

fn schedule_sleep_ms(args: &[Value], env: &mut Environment, callback: Option<(Value, Vec<Value>)>) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => return Err("sleep_ms expects a non-negative integer delay".into()),
    };
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    let id = env.alloc_timer_id();
    let wake = WakeAt::WallMs(unix_ms_now().saturating_add(ms));
    let timer_id = callback.is_some();
    env.schedule_sleep(SleepWaiter {
        id,
        promise: promise.clone(),
        wake,
        callback,
        repeat_interval: None,
        repeat_wall_ms: true,
    });
    Ok(if timer_id {
        Value::Number(id as i64)
    } else {
        Value::Promise(promise)
    })
}

fn sleep_ticks_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    schedule_sleep_ticks(args, env, None)
}

fn sleep_ms_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    schedule_sleep_ms(args, env, None)
}

fn set_timeout_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let func = args.first().ok_or("set_timeout(fn, delay_ms)")?.clone();
    let delay = args.get(1).ok_or("set_timeout(fn, delay_ms)")?;
    schedule_sleep_ms(std::slice::from_ref(delay), env, Some((func, vec![])))
}

fn clear_timeout_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as u64,
        _ => return Err("clear_timeout(id) expects a non-negative timer id".into()),
    };
    env.cancel_timer(id);
    Ok(Value::Null)
}

fn set_interval_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let func = args.first().ok_or("set_interval(fn, delay_ms)")?.clone();
    let delay = args.get(1).ok_or("set_interval(fn, delay_ms)")?;
    let ms = match delay {
        Value::Number(n) if *n >= 0 => *n as u64,
        _ => return Err("set_interval(fn, delay_ms) expects non-negative delay".into()),
    };
    schedule_repeating_timer(func, ms, true, env)
}

fn set_interval_ticks_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let func = args.first().ok_or("set_interval_ticks(fn, delay_ticks)")?.clone();
    let delay = args.get(1).ok_or("set_interval_ticks(fn, delay_ticks)")?;
    let ticks = match delay {
        Value::Number(n) if *n >= 0 => *n as u64,
        _ => return Err("set_interval_ticks(fn, delay_ticks) expects non-negative delay".into()),
    };
    schedule_repeating_timer(func, ticks, false, env)
}

fn schedule_repeating_timer(
    func: Value,
    delay: u64,
    wall_ms: bool,
    env: &mut Environment,
) -> Result<Value, String> {
    let id = env.alloc_timer_id();
    let wake = if wall_ms {
        WakeAt::WallMs(unix_ms_now().saturating_add(delay))
    } else {
        WakeAt::Tick(env.current_tick().saturating_add(delay))
    };
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    env.schedule_sleep(SleepWaiter {
        id,
        promise,
        wake,
        callback: Some((func, vec![])),
        repeat_interval: Some(delay),
        repeat_wall_ms: wall_ms,
    });
    Ok(Value::Number(id as i64))
}

fn clear_interval_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    clear_timeout_native(args, env)
}

fn queue_microtask_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let func = args.first().ok_or("queue_microtask(fn, ...args)")?.clone();
    let call_args: Vec<Value> = args.iter().skip(1).cloned().collect();
    env.schedule_microtask_callback(func, call_args);
    Ok(Value::Null)
}

fn is_impl_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let value = args.first().ok_or("is_impl() expects 2 arguments")?;
    let iface_name = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("is_impl() second argument must be a string".into()),
    };
    let Value::ClassInstance(inst) = value else {
        return Ok(Value::Bool(false));
    };
    let inst_ref = inst
        .try_borrow()
        .map_err(|e| format!("class instance borrow: {e}"))?;
    if !inst_ref.interfaces.contains(&iface_name.to_string()) {
        return Ok(Value::Bool(false));
    }
    let Some(iface) = env.get_interface(iface_name) else {
        return Ok(Value::Bool(false));
    };
    for required in &iface.methods {
        let Some(method) = inst_ref.methods.get(&required.name) else {
            return Ok(Value::Bool(false));
        };
        if method.params.len() != required.params.len() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn is_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let value = args.first().ok_or("is() expects 2 arguments")?;
    let class_name = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("is() second argument must be a class name string".into()),
    };
    let Value::ClassInstance(inst) = value else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(instance_of_class(inst, class_name, env)))
}

fn instance_of_class(inst: &SharedClassInstance, class_name: &str, env: &Environment) -> bool {
    let Ok(inst_ref) = inst.try_borrow() else {
        return false;
    };
    let mut current = inst_ref.class_name.clone();
    loop {
        if current == class_name {
            return true;
        }
        let Some(def) = env.get_class(&current) else {
            break;
        };
        let Some(parent) = def.extends.clone() else {
            break;
        };
        current = parent;
    }
    false
}

fn bytecode_can_compile_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("bytecode_can_compile(source) expects a string".into()),
    };
    Ok(Value::Bool(crate::bytecode::can_compile(code)))
}

fn bytecode_opt_info_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("bytecode_opt_info(source) expects a string".into()),
    };
    let program = crate::bytecode::compile_source(code)?;
    let module = program
        .bytecode
        .as_ref()
        .ok_or("source does not compile to bytecode")?;
    let mut m = std::collections::HashMap::new();
    m.insert("optimized".into(), Value::Bool(true));
    m.insert(
        "main_ops".into(),
        Value::Number(module.main_code.len() as i64),
    );
    m.insert(
        "constants".into(),
        Value::Number(module.constants.len() as i64),
    );
    Ok(Value::from_object(m))
}

/// H6e hard path: thin VM syscall — deserialize `.kbc` text and run (policy in `kab/vm`).
fn bytecode_run_kbc_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let kbc = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("bytecode_run_kbc(kbc) expects a string".into()),
    };
    let module = crate::bytecode::deserialize(kbc)?;
    let result = crate::bytecode::run_module(&module, env)?;
    drain_all_microtasks(env)?;
    Ok(result)
}

/// H6e Kab VM: resolve a host binding by name (natives / imports in current env).
fn bytecode_host_get_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("bytecode_host_get(name) expects a string".into()),
    };
    Ok(env.get(name).unwrap_or(Value::Undefined))
}

/// Plain-object field get; returns undefined for non-maps (BytecodeFn, natives, …).
fn bytecode_host_map_get_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Ok(Value::Undefined),
    };
    match args.first() {
        Some(Value::Object(map)) => Ok(map.get(key).cloned().unwrap_or(Value::Undefined)),
        _ => Ok(Value::Undefined),
    }
}

/// H6e Kab VM: call a host callable (native / bytecode fn) with an args array.
fn bytecode_host_call_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let callee = args
        .first()
        .cloned()
        .ok_or("bytecode_host_call(callee, args)")?;
    let call_args = match args.get(1) {
        Some(Value::Array(items)) => items.as_ref().clone(),
        Some(_) => return Err("bytecode_host_call args must be an array".into()),
        None => Vec::new(),
    };
    crate::bytecode::call_value(callee, call_args, &[], &[], &[], &[], env)
}

/// H6e Kab VM: import a module into the current env; return `[[name, value], ...]`.
fn bytecode_host_import_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("bytecode_host_import(name) expects a string".into()),
    };
    let exported = modules::import_module_exported(name, env)?;
    let mut pairs = Vec::new();
    for en in exported {
        if let Some(v) = env.get(&en) {
            pairs.push(Value::from_array(vec![Value::String(en), v]));
        }
    }
    Ok(Value::from_array(pairs))
}

/// H6e Kab VM: `iterator_step_in_place` — returns `[it, { value, done }]`.
fn bytecode_iterator_step_in_place_native(
    args: &[Value],
    env: &mut Environment,
) -> Result<Value, String> {
    let mut it = args
        .first()
        .cloned()
        .ok_or("bytecode_iterator_step_in_place(iterator)")?;
    let (value, done) = crate::runtime::stdlib::iterator::iterator_next(&mut it, env)?;
    let result = crate::runtime::stdlib::iterator::iterator_result(value, done);
    Ok(Value::from_array(vec![it, result]))
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

fn write_index(container: &mut Value, idx: &Value, val: Value, env: &mut Environment) -> Result<(), String> {
    crate::ops::write_index(container, idx, val, env)
}

pub fn eval_stmt(stmt: &Stmt, env: &mut Environment) -> Result<Value, String> {
    match stmt {
        Stmt::Let {
            pattern,
            init,
            public,
            immutable,
        } => {
            let val = if let Some(expr) = init {
                eval_expr(expr, env)?
            } else {
                Value::Undefined
            };
            bind_binding_pattern(pattern, &val, env, *immutable)?;
            if *public {
                for name in crate::ast::exported_binding_names(pattern) {
                    env.mark_exported(name);
                }
            }
            Ok(Value::Null)
        }
        Stmt::Interface {
            name,
            type_params,
            associated_types,
            methods,
        } => {
            let def = InterfaceDef {
                name: name.clone(),
                type_params: type_params.clone(),
                associated_types: associated_types.clone(),
                methods: methods
                    .iter()
                    .map(|m| MethodSignature {
                        name: m.name.clone(),
                        params: m.params.clone(),
                        default_body: m.body.clone(),
                        default_bytecode: None,
                    })
                    .collect(),
            };
            env.classes_mut().register_interface(def);
            Ok(Value::Null)
        }
        Stmt::Class {
            name,
            type_params: _,
            extends,
            extends_type_args: _,
            implements,
            where_clause: _,
            associated_types,
            fields,
            methods,
            is_struct,
        } => {
            let mut def = ClassDef {
                name: name.clone(),
                extends: extends.clone(),
                implements: implements.clone(),
                associated_types: associated_types.clone(),
                fields: fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        type_name: if f.type_name.is_empty() {
                            None
                        } else {
                            Some(f.type_name.clone())
                        },
                        default: f.default.clone(),
                        private: f.private,
                    })
                    .collect(),
                methods: methods
                    .iter()
                    .map(|m| MethodDef {
                        name: m.name.clone(),
                        params: m.params.clone(),
                        body: m.body.clone(),
                        bytecode: None,
                        private: m.private,
                        owner_class: Some(name.clone()),
                    })
                    .collect(),
                is_struct: *is_struct,
            };
            inject_interface_default_methods(&mut def, env)?;
            validate_class_interfaces(&def, env)?;
            env.classes_mut().register(def);
            Ok(Value::Null)
        }
        Stmt::Enum {
            name,
            type_params,
            variants,
        } => {
            if !type_params.is_empty() {
                return Ok(Value::Null);
            }
            let def = crate::class::EnumDef {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|v| crate::class::EnumVariantDef {
                        name: v.name.clone(),
                        fields: v.fields.clone(),
                    })
                    .collect(),
            };
            crate::class::register_enum_in_env(&def, env);
            Ok(Value::Null)
        }
        Stmt::Import { module, public } => {
            let imported = modules::import_module_exported(module, env)?;
            if *public {
                for name in imported {
                    env.mark_exported(name);
                }
            }
            Ok(Value::Null)
        }
        Stmt::Using { name, init } => {
            let resource = eval_expr(init, env)?;
            env.set_const(name.clone(), resource.clone());
            crate::runtime::stdlib::disposable::push_disposable(resource);
            Ok(Value::Null)
        }
        Stmt::Expr(expr) => eval_expr(expr, env),
        Stmt::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
                eval_expr(expr, env)
            } else {
                Ok(Value::Null)
            }
        }
    }
}

fn eval_delete(inner: &Expr, env: &mut Environment) -> Result<Value, String> {
    match inner {
        Expr::Member(obj_expr, field, _) => {
            let mut obj = eval_expr(obj_expr, env)?;
            match obj {
                Value::Object(ref mut map_rc) => {
                    let map = Value::object_make_mut(map_rc);
                    let removed = crate::runtime::stdlib::descriptor::delete_own_property(
                        map,
                        field,
                    )
                    .unwrap_or(false);
                    store_lvalue(obj_expr, Value::Object(map_rc.clone()), env)?;
                    Ok(Value::Bool(removed))
                }
                _ => Ok(Value::Bool(false)),
            }
        }
        _ => Err("delete expects object property access".into()),
    }
}

pub fn eval_expr(expr: &Expr, env: &mut Environment) -> Result<Value, String> {
    match expr {
        Expr::Literal(lit) => eval_literal(lit, env),
        Expr::Variable(name) => env
            .get(name)
            .ok_or_else(|| undefined_var_message(name, env)),
        Expr::Unary(op, inner) => {
            if matches!(op, UnaryOp::Delete) {
                return eval_delete(inner, env);
            }
            if matches!(op, UnaryOp::Throw | UnaryOp::Raise) {
                let v = eval_expr(inner, env)?;
                return Err(crate::runtime::stdlib::error::throw_value(v));
            }
            let v = eval_expr(inner, env)?;
            match op {
                UnaryOp::Ref | UnaryOp::RefMut => Ok(v),
                UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                UnaryOp::Neg => {
                    if let Some(r) = crate::runtime::stdlib::bigint::try_neg(&v) {
                        return r;
                    }
                    match v {
                    Value::Number(n) => Ok(Value::Number(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Err(format!("Cannot negate {:?}", v)),
                    }
                }
                UnaryOp::BitNot => crate::ops::eval_unary_bitnot(&v),
                UnaryOp::Delete | UnaryOp::Throw | UnaryOp::Raise => unreachable!(),
            }
        }
        Expr::Ternary(cond, then_branch, else_branch) => {
            let c = eval_expr(cond, env)?;
            if c.is_truthy() {
                eval_expr(then_branch, env)
            } else {
                eval_expr(else_branch, env)
            }
        }
        Expr::Binary(left, op, right) => match op {
            BinaryOp::And => {
                let l = eval_expr(left, env)?;
                if l.is_truthy() {
                    eval_expr(right, env)
                } else {
                    Ok(l)
                }
            }
            BinaryOp::Or => {
                let l = eval_expr(left, env)?;
                if l.is_truthy() {
                    Ok(l)
                } else {
                    eval_expr(right, env)
                }
            }
            BinaryOp::NullishCoalesce => {
                let l = eval_expr(left, env)?;
                if l.is_null() || l.is_undefined() {
                    eval_expr(right, env)
                } else {
                    Ok(l)
                }
            }
            _ => {
                let l = eval_expr(left, env)?;
                let r = eval_expr(right, env)?;
                crate::ops::eval_binary_op(&l, op, &r, env)
            }
        },
        Expr::Function {
            name,
            type_params: _,
            params,
            rest,
            return_type: _,
            where_clause: _,
            body,
            public,
            async_fn,
            generator_fn,
        } => {
            if *generator_fn {
                return Err("generator functions require bytecode compilation".into());
            }
            let func = make_function_value(
                name.clone(),
                params.clone(),
                rest.clone(),
                *body.clone(),
                env.share_bindings(),
                *public,
                *async_fn,
            );
            env.set(name.clone(), func.clone());
            if *public {
                env.mark_exported(name);
            }
            Ok(func)
        }
        Expr::Arrow {
            params,
            rest,
            body,
            async_fn,
            generator_fn: _,
        } => {
            Ok(make_function_value(
                "<arrow>".to_string(),
                params.clone(),
                rest.clone(),
                *body.clone(),
                env.share_bindings(),
                false,
                *async_fn,
            ))
        }
        Expr::Await(inner) => {
            let val = eval_expr(inner, env)?;
            resolve_await(val, env)
        }
        Expr::Yield(_) => Err("yield requires bytecode compilation".into()),
        Expr::YieldStar(_) => Err("yield* requires bytecode compilation".into()),
        Expr::Call {
            func: func_expr,
            type_args: _,
            args,
        } => {
            if let Expr::Member(obj_expr, method, _) = func_expr.as_ref() {
                if method == "push" && args.len() == 1 {
                    let pushed = match &args[0] {
                        CallArg::Expr(e) => eval_expr(e, env)?,
                        CallArg::Spread(_) => {
                            return Err("push() does not accept spread".into());
                        }
                    };
                    if let Expr::Variable(var_name) = obj_expr.as_ref() {
                        let mut arr_val = env
                            .get(var_name)
                            .ok_or_else(|| format!("Undefined variable: {}", var_name))?;
                        let Value::Array(ref mut items) = arr_val else {
                            return Err("push() requires an array".into());
                        };
                        Rc::make_mut(items).push(pushed);
                        let len = items.len() as i64;
                        env.assign(var_name, arr_val)?;
                        return Ok(Value::Number(len));
                    }
                    let mut arr_val = eval_expr(obj_expr, env)?;
                    let Value::Array(ref mut items) = arr_val else {
                        return Err("push() requires an array".into());
                    };
                    Rc::make_mut(items).push(pushed);
                    return Ok(Value::Number(items.len() as i64));
                }
            }
            if let Expr::Variable(class_name) = func_expr.as_ref() {
                if let Some(class_def) = env.get_class(class_name) {
                    return instantiate_class(&class_def, args, env);
                }
            }
            let func_val = eval_expr(func_expr, env)?;
            let arg_vals = eval_call_args(args, env)?;
            if let Value::EnumCtor {
                type_name,
                variant,
                arity,
            } = &func_val
            {
                return crate::class::invoke_enum_ctor(type_name, variant, *arity, arg_vals);
            }
            match func_val {
                Value::Function {
                    params,
                    defaults,
                    rest,
                    body,
                    env: closure_env,
                    async_fn,
                    ..
                } => {
                    if async_fn {
                        return schedule_async(
                            params.clone(),
                            defaults.clone(),
                            rest.clone(),
                            body.clone(),
                            closure_env.clone(),
                            arg_vals,
                            env,
                        );
                    }
                    let mut new_env = Environment::child(closure_env);
                    bind_call_params(&params, &defaults, &rest, &arg_vals, &mut new_env)?;
                    eval_expr(&body, &mut new_env)
                }
                Value::BytecodeFn(func) => {
                    crate::bytecode::run_bytecode_fn(func.def.as_ref(), arg_vals, env)
                }
                Value::BoundMethod(instance, method) => {
                    if method.params.len() != arg_vals.len() {
                        return Err(format!(
                            "Argument count mismatch: expected {}, got {}",
                            method.params.len(),
                            arg_vals.len()
                        ));
                    }
                    let (owner, recv) = {
                        let inst_ref = instance
                            .try_borrow()
                            .map_err(|e| format!("class instance borrow: {e}"))?;
                        let owner = crate::class::method_owner_class(&method, &inst_ref);
                        let recv = crate::class::receiver_binding(inst_ref.is_struct);
                        (owner, recv)
                    };
                    if let Some(bc) = &method.bytecode {
                        let mut call_env = create_global_env();
                        *call_env.classes_mut() = env.classes().clone();
                        call_env.set_private_scope(Some(&owner));
                        call_env.set(
                            recv.to_string(),
                            Value::ClassInstance(instance.clone()),
                        );
                        let result =
                            crate::bytecode::run_bytecode_fn(bc, arg_vals, &mut call_env)?;
                        writeback_receiver(env, &call_env, recv)?;
                        return Ok(result);
                    }
                    let mut new_env = create_global_env();
                    *new_env.classes_mut() = env.classes().clone();
                    new_env.set_private_scope(Some(&owner));
                    new_env.set(
                        recv.to_string(),
                        Value::ClassInstance(instance.clone()),
                    );
                    for (p, a) in method.params.iter().zip(arg_vals) {
                        new_env.set(p.clone(), a);
                    }
                    let result = eval_expr(&method.body, &mut new_env)?;
                    writeback_receiver(env, &new_env, recv)?;
                    Ok(result)
                }
                Value::NativeFunction(f) => {
                    let result = f(&arg_vals, env)?;
                    crate::runtime::stdlib::object::try_mutator_writeback(
                        func_expr, args, &result, env,
                    )?;
                    Ok(result)
                }
                func_val if crate::runtime::stdlib::symbol::is_symbol_ctor_object(&func_val) => {
                    crate::runtime::stdlib::symbol::try_symbol_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Symbol constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::proxy::is_proxy_ctor_object(&func_val) => {
                    crate::runtime::stdlib::proxy::try_proxy_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Proxy constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::weak::is_weakref_ctor_object(&func_val) => {
                    crate::runtime::stdlib::weak::try_weakref_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("WeakRef constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::weak::is_finreg_ctor_object(&func_val) => {
                    crate::runtime::stdlib::weak::try_finreg_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("FinalizationRegistry constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::intl::is_number_format_ctor(&func_val) => {
                    crate::runtime::stdlib::intl::try_number_format_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Intl.NumberFormat constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::intl::is_date_time_format_ctor(&func_val) => {
                    crate::runtime::stdlib::intl::try_date_time_format_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Intl.DateTimeFormat constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::temporal::is_plain_date_ctor(&func_val) => {
                    crate::runtime::stdlib::temporal::try_plain_date_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Temporal.PlainDate constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::temporal::is_instant_ctor(&func_val) => {
                    crate::runtime::stdlib::temporal::try_instant_ctor_call(&func_val, &arg_vals, env)
                        .unwrap_or(Err("Temporal.Instant constructor failed".into()))
                }
                func_val if crate::runtime::stdlib::proxy::is_proxy(&func_val) => {
                    crate::runtime::stdlib::proxy::trap_apply(
                        &func_val,
                        Value::Undefined,
                        arg_vals,
                        env,
                    )
                }
                Value::BoundNative(receiver, f) => {
                    let mut call_args = vec![(*receiver).clone()];
                    call_args.extend(arg_vals);
                    let result = f(&call_args, env)?;
                    crate::runtime::stdlib::object::try_mutator_writeback(
                        func_expr, args, &result, env,
                    )?;
                    Ok(result)
                }
                Value::PromiseSettler { ctrl_id, reject } => {
                    crate::runtime::stdlib::promise::call_settler(ctrl_id, reject, &arg_vals, env)
                }
                _ => Err(format!("Not a function: {:?}", func_val)),
            }
        }
        Expr::Block(stmts) => {
            let depth = crate::runtime::stdlib::disposable::disposable_depth();
            let mut result = Ok(Value::Null);
            for stmt in stmts {
                result = eval_stmt(stmt, env);
                if result.is_err() {
                    crate::runtime::stdlib::disposable::dispose_since(depth, env);
                    return result;
                }
                if let Ok(Value::Break) = result {
                    break;
                }
                if let Ok(Value::Continue) = result {
                    continue;
                }
            }
            crate::runtime::stdlib::disposable::dispose_since(depth, env);
            result
        }
        Expr::Match(value, arms) => {
            let val = eval_expr(value, env)?;
            for arm in arms {
                if !pattern_matches(&arm.pattern, &val) {
                    continue;
                }
                let mut arm_env = Environment::child(env.clone());
                bind_pattern(&arm.pattern, &val, &mut arm_env);
                if let Some(guard) = &arm.guard {
                    if !eval_expr(guard, &mut arm_env)?.is_truthy() {
                        continue;
                    }
                }
                return eval_expr(&arm.body, &mut arm_env);
            }
            Err("No matching pattern".into())
        }
        Expr::IfLet {
            pattern,
            scrutinee,
            body,
            else_branch,
        } => {
            let val = eval_expr(scrutinee, env)?;
            if pattern_matches(pattern, &val) {
                let mut arm_env = Environment::child(env.clone());
                bind_pattern(pattern, &val, &mut arm_env);
                eval_expr(body, &mut arm_env)
            } else if let Some(else_body) = else_branch {
                eval_expr(else_body, env)
            } else {
                Ok(Value::Null)
            }
        }
        Expr::WhileLet {
            pattern,
            scrutinee,
            body,
        } => {
            loop {
                let val = eval_expr(scrutinee, env)?;
                if !pattern_matches(pattern, &val) {
                    break;
                }
                let mut arm_env = Environment::child(env.clone());
                bind_pattern(pattern, &val, &mut arm_env);
                match eval_expr(body, &mut arm_env) {
                    Ok(Value::Break) => break,
                    Ok(Value::Continue) => continue,
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            Ok(Value::Null)
        }
        Expr::ResultQuestion(inner) => {
            let v = eval_expr(inner, env)?;
            match v {
                Value::Result(Ok(b)) => Ok(*b),
                Value::Result(Err(e)) => Ok(Value::Result(Err(e))),
                other => Err(format!(
                    "? operator requires Result (Ok/Err), got {}",
                    format_value(&other)
                )),
            }
        }
        Expr::While(cond, body) => {
            loop {
                let cond_val = eval_expr(cond, env)?;
                if !cond_val.is_truthy() {
                    break;
                }
                match eval_expr(body, env) {
                    Ok(Value::Break) => break,
                    Ok(Value::Continue) => continue,
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            Ok(Value::Null)
        }
        Expr::DoWhile(body, cond) => {
            loop {
                match eval_expr(body, env) {
                    Ok(Value::Break) => break,
                    Ok(Value::Continue) => (),
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
                if !eval_expr(cond, env)?.is_truthy() {
                    break;
                }
            }
            Ok(Value::Null)
        }
        Expr::ForClassic {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(init_stmt) = init {
                eval_stmt(init_stmt, env)?;
            }
            loop {
                if let Some(c) = cond {
                    if !eval_expr(c, env)?.is_truthy() {
                        break;
                    }
                }
                let should_continue = match eval_expr(body, env) {
                    Ok(Value::Break) => break,
                    Ok(Value::Continue) => true,
                    Ok(_) => false,
                    Err(e) => return Err(e),
                };
                if let Some(s) = step {
                    eval_expr(s, env)?;
                }
                if should_continue {
                    continue;
                }
            }
            Ok(Value::Null)
        }
        Expr::TryCatch {
            body,
            err_name,
            handler,
            finally,
        } => {
            let outcome = match eval_expr(body, env) {
                Ok(Value::Result(Err(e))) => {
                    let mut catch_env = Environment::child(env.clone());
                    catch_env.set(err_name.clone(), *e);
                    eval_expr(handler, &mut catch_env)
                }
                Ok(Value::Result(Ok(v))) => Ok(*v),
                Ok(other) => Ok(other),
                Err(e) => {
                    if let Some(thrown) = crate::runtime::stdlib::error::take_throw_value(&e) {
                        let mut catch_env = Environment::child(env.clone());
                        catch_env.set(err_name.clone(), thrown);
                        eval_expr(handler, &mut catch_env)
                    } else {
                        Err(e)
                    }
                }
            };
            if let Some(fin) = finally {
                eval_expr(fin, env)?;
            }
            outcome
        }
        Expr::ForEach(loop_) => {
            let iter_val = eval_expr(&loop_.iterable, env)?;
            let close_sync = |iter: &mut Value, env: &mut Environment| {
                let _ = crate::runtime::stdlib::iterator::iterator_return(
                    iter,
                    Value::Null,
                    env,
                );
            };
            let close_async = |iter: &mut Value, env: &mut Environment| {
                let _ = crate::runtime::stdlib::async_iterator::async_iterator_close(
                    iter,
                    Value::Null,
                    env,
                );
            };
            let run_body = |env: &mut Environment, item: Value| -> Result<bool, String> {
                if loop_.immutable {
                    env.set_const(loop_.var.clone(), item);
                } else {
                    env.set(loop_.var.clone(), item);
                }
                match eval_expr(&loop_.body, env) {
                    Ok(Value::Break) => Ok(true),
                    Ok(Value::Continue) => Ok(false),
                    Ok(_) => Ok(false),
                    Err(e) => Err(e),
                }
            };
            if loop_.by_value {
                if loop_.async_for {
                    let mut iter =
                        crate::runtime::stdlib::async_iterator::get_async_iterator(&iter_val, env)?;
                    let outcome = (|| -> Result<(), String> {
                        loop {
                            let next_p = crate::runtime::stdlib::async_iterator::async_iterator_step(
                                &mut iter, env,
                            )?;
                            let result = resolve_await_value(next_p, env)?;
                            let (value, done) =
                                crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
                            if done {
                                break;
                            }
                            if run_body(env, value)? {
                                close_async(&mut iter, env);
                                break;
                            }
                        }
                        Ok(())
                    })();
                    if outcome.is_err() {
                        close_async(&mut iter, env);
                    }
                    outcome?;
                } else {
                    let mut iter =
                        crate::runtime::stdlib::iterator::get_sync_iterator(&iter_val, env)?;
                    let outcome = (|| -> Result<(), String> {
                        loop {
                            let result =
                                crate::runtime::stdlib::iterator::iterator_step(&mut iter, env)?;
                            let (value, done) =
                                crate::runtime::stdlib::iterator::parse_iterator_result(&result)?;
                            if done {
                                break;
                            }
                            if run_body(env, value)? {
                                close_sync(&mut iter, env);
                                break;
                            }
                        }
                        Ok(())
                    })();
                    if outcome.is_err() {
                        close_sync(&mut iter, env);
                    }
                    outcome?;
                }
            } else {
                match iter_val {
                    Value::Array(items) => {
                        for i in 0..items.len() {
                            if run_body(env, Value::Number(i as i64))? {
                                break;
                            }
                        }
                    }
                    Value::String(s) => {
                        let len = s.chars().count();
                        for i in 0..len {
                            if run_body(env, Value::Number(i as i64))? {
                                break;
                            }
                        }
                    }
                    Value::Object(map) => {
                        let mut keys: Vec<_> = map
                            .keys()
                            .filter(|k| !k.starts_with("__kab_"))
                            .cloned()
                            .collect();
                        keys.sort();
                        for key in keys {
                            if run_body(env, Value::String(key))? {
                                break;
                            }
                        }
                    }
                    _ => return Err("for-in requires array, string, or object".into()),
                }
            }
            Ok(Value::Null)
        }
        Expr::Switch {
            scrutinee,
            cases,
            default_body,
        } => {
            let val = eval_expr(scrutinee, env)?;
            let mut run_next = false;
            for case in cases {
                let matched = run_next
                    || {
                        let case_val = eval_expr(&case.value, env)?;
                        crate::ops::values_equal(&val, &case_val)
                    };
                if matched {
                    match eval_switch_case_body(&case.body, env)? {
                        SwitchCaseFlow::Value(v) => return Ok(v),
                        SwitchCaseFlow::Fallthrough => run_next = true,
                    }
                }
            }
            if run_next {
                if let Some(def) = default_body {
                    return eval_expr(def, env);
                }
            } else if default_body.is_none() {
                return Ok(Value::Null);
            }
            if let Some(def) = default_body {
                eval_expr(def, env)
            } else {
                Ok(Value::Null)
            }
        }
        Expr::If(cond, then_branch, else_branch) => {
            let cond_val = eval_expr(cond, env)?;
            if cond_val.is_truthy() {
                eval_expr(then_branch, env)
            } else if let Some(else_expr) = else_branch {
                eval_expr(else_expr, env)
            } else {
                Ok(Value::Null)
            }
        }
        Expr::Assign(target, value_expr) => {
            let val = eval_expr(value_expr, env)?;
            match target {
                AssignTarget::Name(name) => {
                    crate::runtime::ownership::store_binding(env, name, val.clone())?;
                    Ok(val)
                }
                AssignTarget::Pattern(pat) => {
                    bind_binding_pattern(pat, &val, env, false)?;
                    Ok(val)
                }
                AssignTarget::Member(obj_expr, field) => {
                    if matches!(obj_expr.as_ref(), Expr::Super) {
                        let mut this_val = env
                            .get("this")
                            .ok_or_else(|| "`super` used outside of method".to_string())?;
                        assign_member_value(&mut this_val, field, val.clone(), env)?;
                        env.assign("this", this_val)?;
                        return Ok(val);
                    }
                    let mut container = eval_expr(obj_expr, env)?;
                    assign_member_value(&mut container, field, val.clone(), env)?;
                    store_lvalue(obj_expr, container, env)?;
                    Ok(val)
                }
                AssignTarget::Index(obj_expr, idx_expr) => {
                    let idx = eval_expr(idx_expr, env)?;
                    let mut container = eval_expr(obj_expr, env)?;
                    write_index(&mut container, &idx, val.clone(), env)?;
                    store_lvalue(obj_expr, container, env)?;
                    Ok(val)
                }
            }
        }
        Expr::Index(obj_expr, idx_expr) => {
            let obj = eval_expr(obj_expr, env)?;
            let idx = eval_expr(idx_expr, env)?;
            crate::ops::read_index(&obj, &idx, env)
        }
        Expr::Slice { start, stop, step } => {
            let mut m = std::collections::HashMap::new();
            m.insert("__kab_slice".into(), Value::Bool(true));
            m.insert(
                "start".into(),
                match start {
                    Some(e) => eval_expr(e, env)?,
                    None => Value::Null,
                },
            );
            m.insert(
                "stop".into(),
                match stop {
                    Some(e) => eval_expr(e, env)?,
                    None => Value::Null,
                },
            );
            m.insert(
                "step".into(),
                match step {
                    Some(e) => eval_expr(e, env)?,
                    None => Value::Number(1),
                },
            );
            Ok(Value::from_object(m))
        }
        Expr::Member(obj_expr, field, _) => {
            if matches!(obj_expr.as_ref(), Expr::Super) {
                let this_val = env
                    .get("this")
                    .ok_or_else(|| "`super` used outside of method".to_string())?;
                let Value::ClassInstance(inst) = this_val else {
                    return Err("`super` requires class instance `this`".into());
                };
                return resolve_super_member(&inst, field, env);
            }
            let obj = eval_expr(obj_expr, env)?;
            if let Value::EnumNamespace(type_name) = &obj {
                return crate::class::resolve_enum_member(type_name, field, env);
            }
            crate::runtime::stdlib::opt::get_member_value(&obj, field, env)
        }
        Expr::OptionalMember(obj_expr, field) => {
            let obj = eval_expr(obj_expr, env)?;
            if crate::runtime::stdlib::opt::is_nullish(&obj) {
                return Ok(Value::Undefined);
            }
            crate::runtime::stdlib::opt::get_member_value(&obj, field, env)
        }
        Expr::OptionalIndex(obj_expr, idx_expr) => {
            let obj = eval_expr(obj_expr, env)?;
            if crate::runtime::stdlib::opt::is_nullish(&obj) {
                return Ok(Value::Undefined);
            }
            let idx = eval_expr(idx_expr, env)?;
            crate::ops::read_index(&obj, &idx, env)
        }
        Expr::OptionalCall(func_expr, args) => {
            let base = eval_expr(func_expr, env)?;
            if crate::runtime::stdlib::opt::is_nullish(&base) {
                return Ok(Value::Undefined);
            }
            let arg_vals = eval_call_args(args, env)?;
            crate::bytecode::call_value(base, arg_vals, &[], &[], &[], &[], env)
        }
        Expr::This => env
            .get("this")
            .ok_or_else(|| "`this` used outside of method".into()),
        Expr::Self_ => env
            .get("self")
            .ok_or_else(|| "`self` used outside of struct method".into()),
        Expr::Super => Err("`super` must be used as super.method(...)".into()),
        Expr::Break => Ok(Value::Break),
        Expr::Continue => Ok(Value::Continue),
        Expr::Fallthrough => Ok(Value::Fallthrough),
        Expr::Pass => Ok(Value::Null),
        Expr::Assert { condition, message } => {
            let c = eval_expr(condition, env)?;
            if c.is_truthy() {
                return Ok(Value::Null);
            }
            let msg = if let Some(m) = message {
                eval_expr(m, env)?
            } else {
                Value::String("Assertion failed".into())
            };
            Err(format!("AssertionError: {}", crate::value::format_value(&msg)))
        }
        Expr::With { name, value, body } => {
            let resource = eval_expr(value, env)?;
            env.set(name.clone(), resource.clone());
            let result = eval_expr(body, env);
            crate::runtime::stdlib::disposable::dispose_resource(&resource, env);
            result
        }
        Expr::ImportMeta => Ok(modules::import_meta_object()),
        Expr::DynamicImport(spec) => {
            let spec_val = eval_expr(spec, env)?;
            modules::dynamic_import(&spec_val, env)
        }
    }
}

enum SwitchCaseFlow {
    Value(Value),
    Fallthrough,
}

fn eval_switch_case_body(body: &Expr, env: &mut Environment) -> Result<SwitchCaseFlow, String> {
    match body {
        Expr::Block(stmts) => {
            let mut last = Value::Null;
            for stmt in stmts {
                match stmt {
                    Stmt::Expr(Expr::Fallthrough) => return Ok(SwitchCaseFlow::Fallthrough),
                    Stmt::Expr(Expr::Break) => return Ok(SwitchCaseFlow::Value(Value::Break)),
                    _ => last = eval_stmt(stmt, env)?,
                }
            }
            Ok(SwitchCaseFlow::Value(last))
        }
        Expr::Fallthrough => Ok(SwitchCaseFlow::Fallthrough),
        other => Ok(SwitchCaseFlow::Value(eval_expr(other, env)?)),
    }
}

fn eval_literal(lit: &Literal, env: &mut Environment) -> Result<Value, String> {
    match lit {
        Literal::Number(n) => Ok(Value::Number(*n)),
        Literal::BigInt(digits) => Ok(crate::runtime::stdlib::bigint::bigint_value(
            crate::runtime::stdlib::bigint::parse_decimal(digits)?,
        )),
        Literal::Float(f) => Ok(Value::Float(*f)),
        Literal::String(s) => Ok(Value::String(s.clone())),
        Literal::Bool(b) => Ok(Value::Bool(*b)),
        Literal::Null => Ok(Value::Null),
        Literal::Undefined => Ok(Value::Undefined),
        Literal::Nan => Ok(Value::Float(f64::NAN)),
        Literal::Some(inner) => {
            let v = eval_expr(inner, env)?;
            Ok(Value::Option(Some(Box::new(v))))
        }
        Literal::None => Ok(Value::Option(None)),
        Literal::Ok(inner) => {
            let v = eval_expr(inner, env)?;
            Ok(Value::Result(Ok(Box::new(v))))
        }
        Literal::Err(inner) => {
            let v = eval_expr(inner, env)?;
            Ok(Value::Result(Err(Box::new(v))))
        }
        Literal::Array(items) => Ok(Value::from_array(expand_array_pieces(items, env)?)),
        Literal::Object(fields) => Ok(Value::from_object(expand_object_pieces(fields, env)?)),
    }
}

fn eval_call_args(args: &[CallArg], env: &mut Environment) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for arg in args.iter() {
        match arg {
            CallArg::Expr(e) => out.push(eval_expr(e, env)?),
            CallArg::Spread(e) => {
                let v = eval_expr(e, env)?;
                match v {
                    Value::Array(items) => out.extend(items.iter().cloned()),
                    _ => return Err("Spread in call requires an array".into()),
                }
            }
        }
    }
    Ok(out)
}

fn expand_array_pieces(pieces: &[ArrayPiece], env: &mut Environment) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for piece in pieces {
        match piece {
            ArrayPiece::Item(e) => out.push(eval_expr(e, env)?),
            ArrayPiece::Spread(e) => {
                let v = eval_expr(e, env)?;
                match v {
                    Value::Array(items) => out.extend(items.iter().cloned()),
                    _ => return Err("Spread requires an array".into()),
                }
            }
        }
    }
    Ok(out)
}

fn expand_object_pieces(
    pieces: &[ObjectPiece],
    env: &mut Environment,
) -> Result<HashMap<String, Value>, String> {
    let mut map = HashMap::new();
    for piece in pieces {
        match piece {
            ObjectPiece::Field { key, value } => {
                map.insert(key.clone(), eval_expr(value, env)?);
            }
            ObjectPiece::Method {
                key,
                params,
                rest,
                body,
                async_fn,
            } => {
                if crate::ast::fn_has_defaults_or_rest(params, rest) {
                    return Err("Object method defaults/rest not supported".into());
                }
                let func = make_function_value(
                    key.clone(),
                    params.clone(),
                    rest.clone(),
                    *body.clone(),
                    env.share_bindings(),
                    false,
                    *async_fn,
                );
                map.insert(key.clone(), func);
            }
            ObjectPiece::Spread(e) => {
                let v = eval_expr(e, env)?;
                match v {
                    Value::Object(obj) => {
                        for (k, v) in obj.iter() {
                            map.insert(k.clone(), v.clone());
                        }
                    }
                    _ => return Err("Spread in object requires an object".into()),
                }
            }
        }
    }
    crate::runtime::stdlib::object::object_oid(&mut map);
    Ok(map)
}

fn bind_binding_pattern(
    pattern: &BindingPattern,
    value: &Value,
    env: &mut Environment,
    immutable: bool,
) -> Result<(), String> {
    match pattern {
        BindingPattern::Name(name) => {
            if immutable {
                env.set_const(name.clone(), value.clone());
            } else {
                env.set(name.clone(), value.clone());
            }
            Ok(())
        }
        BindingPattern::Wildcard => Ok(()),
        BindingPattern::Rest(_name) => {
            Err("Rest pattern must appear inside an array pattern".into())
        }
        BindingPattern::Array(items) => {
            let arr = match value {
                Value::Array(a) => a,
                _ => return Err("Array destructuring requires an array".into()),
            };
            let mut idx = 0usize;
            for item in items.iter() {
                match item {
                    BindingPattern::Rest(name) => {
                        if !name.is_empty() {
                            let rest: Vec<Value> = arr[idx..].to_vec();
                            if immutable {
                                env.set_const(name.clone(), Value::from_array(rest));
                            } else {
                                env.set(name.clone(), Value::from_array(rest));
                            }
                        }
                        return Ok(());
                    }
                    other => {
                        let elem = arr
                            .get(idx)
                            .cloned()
                            .unwrap_or(Value::Undefined);
                        bind_binding_pattern(other, &elem, env, immutable)?;
                        idx += 1;
                    }
                }
            }
            Ok(())
        }
        BindingPattern::Object(fields) => {
            let map = match value {
                Value::Object(m) => m,
                _ => return Err("Object destructuring requires an object".into()),
            };
            let mut bound_keys = HashSet::new();
            for field in fields.iter() {
                match field {
                    ObjectBind::Shorthand(key) => {
                        let v = map.get(key).cloned().unwrap_or(Value::Undefined);
                        if immutable {
                            env.set_const(key.clone(), v);
                        } else {
                            env.set(key.clone(), v);
                        }
                        bound_keys.insert(key.clone());
                    }
                    ObjectBind::Field { key, pattern } => {
                        let v = map.get(key).cloned().unwrap_or(Value::Undefined);
                        bind_binding_pattern(pattern, &v, env, immutable)?;
                        bound_keys.insert(key.clone());
                    }
                    ObjectBind::Rest(name) => {
                        if name.is_empty() {
                            continue;
                        }
                        let mut rest = HashMap::new();
                        for (k, v) in map.iter() {
                            if !bound_keys.contains(k) {
                                rest.insert(k.clone(), v.clone());
                            }
                        }
                        if immutable {
                            env.set_const(name.clone(), Value::from_object(rest));
                        } else {
                            env.set(name.clone(), Value::from_object(rest));
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    for i in 1..=m {
        let mut cur = vec![i];
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur.push(
                (cur[j - 1] + 1)
                    .min(prev[j] + 1)
                    .min(prev[j - 1] + cost),
            );
        }
        prev = cur;
    }
    prev[n]
}

fn suggest_name(name: &str, candidates: &[String]) -> Option<String> {
    let lower = name.to_lowercase();
    let mut best: Option<(String, usize)> = None;
    for c in candidates {
        let cl = c.to_lowercase();
        let score = if cl == lower {
            100
        } else if cl.starts_with(&lower) || lower.starts_with(&cl) {
            50
        } else if cl.contains(&lower) || lower.contains(&cl) {
            20
        } else {
            let dist = levenshtein(&lower, &cl);
            let max_len = lower.len().max(cl.len());
            if dist <= 2 && dist < max_len {
                10 - dist
            } else {
                0
            }
        };
        if score > 0 && best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((c.clone(), score));
        }
    }
    best.map(|(c, _)| c)
}

pub(crate) fn undefined_var_message(name: &str, env: &Environment) -> String {
    let candidates = env.all_binding_names();
    if let Some(hint) = suggest_name(name, &candidates) {
        format!("Undefined variable: {} (did you mean `{}`?)", name, hint)
    } else {
        format!("Undefined variable: {}", name)
    }
}

pub fn resolve_super_method(
    inst: &SharedClassInstance,
    method_name: &str,
    env: &Environment,
) -> Result<MethodDef, String> {
    let inst_ref = inst
        .try_borrow()
        .map_err(|e| format!("class instance borrow: {e}"))?;
    let parent_name = inst_ref
        .super_class
        .as_ref()
        .ok_or_else(|| format!("Class {} has no superclass", inst_ref.class_name))?;
    let parent_def = env
        .get_class(parent_name)
        .ok_or_else(|| format!("Unknown base class: {}", parent_name))?;
    parent_def
        .methods
        .iter()
        .find(|m| m.name == method_name)
        .cloned()
        .ok_or_else(|| format!("Superclass {} has no method {}", parent_name, method_name))
}

pub fn resolve_super_member(
    inst: &SharedClassInstance,
    member: &str,
    env: &Environment,
) -> Result<Value, String> {
    if let Ok(method) = resolve_super_method(inst, member, env) {
        return Ok(Value::BoundMethod(inst.clone(), method));
    }
    let inst_ref = inst
        .try_borrow()
        .map_err(|e| format!("class instance borrow: {e}"))?;
    inst_ref
        .fields
        .get(member)
        .cloned()
        .ok_or_else(|| format!("Super has no member {}", member))
}

fn store_lvalue(expr: &Expr, value: Value, env: &mut Environment) -> Result<(), String> {
    match expr {
        Expr::Variable(name) => env.assign(name, value),
        Expr::This => env.assign("this", value),
        Expr::Self_ => env.assign("self", value),
        Expr::Super => env.assign("this", value),
        Expr::Member(inner, field, _) => {
            if matches!(inner.as_ref(), Expr::Super) {
                let mut this_val = env
                    .get("this")
                    .ok_or_else(|| "`super` used outside of method".to_string())?;
                assign_member_value(&mut this_val, field, value, env)?;
                return env.assign("this", this_val);
            }
            let mut parent = eval_expr(inner, env)?;
            assign_member_value(&mut parent, field, value, env)?;
            store_lvalue(inner, parent, env)
        }
        Expr::Index(container, idx) => {
            let idx_val = eval_expr(idx, env)?;
            let mut parent = eval_expr(container, env)?;
            write_index(&mut parent, &idx_val, value, env)?;
            store_lvalue(container, parent, env)
        }
        _ => Err("Invalid assignment target".into()),
    }
}

fn writeback_receiver(
    env: &mut Environment,
    call_env: &Environment,
    recv: &str,
) -> Result<(), String> {
    if env.get(recv).is_some() {
        if let Some(Value::ClassInstance(updated)) = call_env.get(recv) {
            env.assign(recv, Value::ClassInstance(updated.clone()))?;
        }
    }
    // Nested class method may have outer `this` while struct uses `self`.
    if recv != "this" && env.get("this").is_some() {
        if let Some(Value::ClassInstance(updated)) = call_env.get(recv) {
            let _ = env.assign("this", Value::ClassInstance(updated.clone()));
        }
    }
    if recv != "self" && env.get("self").is_some() {
        if let Some(Value::ClassInstance(updated)) = call_env.get(recv) {
            let _ = env.assign("self", Value::ClassInstance(updated.clone()));
        }
    }
    Ok(())
}

fn assign_member_value(
    obj: &mut Value,
    field: &str,
    val: Value,
    env: &mut Environment,
) -> Result<(), String> {
    let receiver = obj.clone();
    match obj {
        Value::Object(ref mut map_rc) => {
            let map = Value::object_make_mut(map_rc);
            if crate::runtime::browser_platform::canvas_props::try_write_property(
                map,
                field,
                &val,
            )? {
                return Ok(());
            }
            crate::runtime::stdlib::descriptor::set_own_property(
                map,
                field,
                val,
                &receiver,
                env,
            )?;
            Ok(())
        }
        Value::ClassInstance(_inst) => {
            crate::class::with_class_instance_mut(obj, |inst| {
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
                    if !inst.private_fields.contains_key(field) {
                        return Err(format!(
                            "Class {} has no private field {}",
                            inst.class_name, field
                        ));
                    }
                    crate::class::type_check::validate_class_field_write(
                        inst,
                        field,
                        &val,
                        env.classes(),
                    )?;
                    inst.private_fields.insert(field.to_string(), val);
                    return Ok(());
                }
                if !inst.fields.contains_key(field) {
                    return Err(format!(
                        "Class {} has no field {}",
                        inst.class_name, field
                    ));
                }
                crate::class::type_check::validate_class_field_write(
                    inst,
                    field,
                    &val,
                    env.classes(),
                )?;
                inst.fields.insert(field.to_string(), val);
                Ok(())
            })?
        }
        _ => Err("Member assignment requires object or class instance".into()),
    }
}

fn instantiate_class(
    class_def: &ClassDef,
    args: &[CallArg],
    env: &mut Environment,
) -> Result<Value, String> {
    let arg_vals = eval_call_args(args, env)?;
    let instance = materialize_class(class_def, env)?;

    if let Some(init) = instance
        .try_borrow()
        .ok()
        .and_then(|i| i.methods.get("init").cloned())
    {
        if init.params.len() != arg_vals.len() {
            return Err(format!(
                "Class {} init expects {} arguments, got {}",
                class_def.name,
                init.params.len(),
                arg_vals.len()
            ));
        }
        let mut init_env = create_global_env();
        *init_env.classes_mut() = env.classes().clone();
        let owner = {
            let inst_ref = instance
                .try_borrow()
                .map_err(|e| format!("class instance borrow: {e}"))?;
            crate::class::method_owner_class(&init, &inst_ref)
        };
        init_env.set_private_scope(Some(&owner));
        let recv = crate::class::receiver_binding(class_def.is_struct);
        init_env.set(
            recv.to_string(),
            Value::ClassInstance(instance.clone()),
        );
        for (p, a) in init.params.iter().zip(arg_vals.iter()) {
            init_env.set(p.clone(), a.clone());
        }
        eval_expr(&init.body, &mut init_env)?;
    } else if !arg_vals.is_empty() {
        return Err(format!(
            "Class {} does not accept constructor arguments (define fn init)",
            class_def.name
        ));
    }

    Ok(Value::ClassInstance(instance))
}

fn materialize_class(
    class_def: &ClassDef,
    env: &mut Environment,
) -> Result<SharedClassInstance, String> {
    let inst = if let Some(parent_name) = &class_def.extends {
        let parent_def = env
            .get_class(parent_name)
            .ok_or_else(|| format!("Unknown base class: {}", parent_name))?;
        materialize_class(&parent_def, env)?
    } else {
        Rc::new(RefCell::new(ClassInstance {
            class_name: class_def.name.clone(),
            super_class: None,
            interfaces: Vec::new(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            private_fields: HashMap::new(),
            private_methods: HashMap::new(),
            is_struct: class_def.is_struct,
        }))
    };

    {
        let mut instance = inst
            .try_borrow_mut()
            .map_err(|e| format!("class instance borrow_mut: {e}"))?;
        for field in &class_def.fields {
            let val = if let Some(default_expr) = &field.default {
                eval_expr(default_expr, env)?
            } else {
                instance
                    .fields
                    .get(&field.name)
                    .or_else(|| instance.private_fields.get(&field.name))
                    .cloned()
                    .unwrap_or(Value::Undefined)
            };
            if field.private {
                instance.private_fields.insert(field.name.clone(), val);
            } else {
                instance.fields.insert(field.name.clone(), val);
            }
        }

        for field in &class_def.fields {
            if let Some(type_name) = &field.type_name {
                if type_name.is_empty() {
                    continue;
                }
                let v = if field.private {
                    instance.private_fields.get(&field.name)
                } else {
                    instance.fields.get(&field.name)
                };
                if let Some(v) = v {
                    crate::class::type_check::check_field_type(type_name, v, env.classes())?;
                }
            }
        }

        for method in &class_def.methods {
            let method_def = MethodDef {
                name: method.name.clone(),
                params: method.params.clone(),
                body: method.body.clone(),
                bytecode: method.bytecode.clone(),
                owner_class: Some(class_def.name.clone()),
                private: method.private,
            };
            if method.private {
                instance
                    .private_methods
                    .insert(method.name.clone(), method_def);
            } else {
                instance.methods.insert(method.name.clone(), method_def);
            }
        }
        instance.class_name = class_def.name.clone();
        instance.super_class = class_def.extends.clone();
        instance.interfaces = class_def.implements.clone();
        instance.is_struct = class_def.is_struct;
    }

    Ok(inst)
}

fn collect_class_method_arities(
    class_def: &ClassDef,
    env: &Environment,
) -> Result<HashMap<String, usize>, String> {
    let mut methods = HashMap::new();
    if let Some(parent_name) = &class_def.extends {
        let parent_def = env
            .get_class(parent_name)
            .ok_or_else(|| format!("Unknown base class: {}", parent_name))?;
        methods.extend(collect_class_method_arities(&parent_def, env)?);
    }
    for method in &class_def.methods {
        methods.insert(method.name.clone(), method.params.len());
    }
    Ok(methods)
}

pub fn inject_interface_default_methods(
    class_def: &mut ClassDef,
    env: &Environment,
) -> Result<(), String> {
    if class_def.implements.is_empty() {
        return Ok(());
    }
    let existing: HashSet<String> = class_def.methods.iter().map(|m| m.name.clone()).collect();
    let mut to_inject = Vec::new();
    for iface_name in &class_def.implements {
        let base = iface_name.split('$').next().unwrap_or(iface_name);
        let iface = env
            .get_interface(iface_name)
            .or_else(|| env.get_interface(base))
            .ok_or_else(|| format!("Unknown interface: {}", iface_name))?;
        for required in &iface.methods {
            if existing.contains(&required.name) {
                continue;
            }
            if required.default_body.is_none() && required.default_bytecode.is_none() {
                continue;
            }
            to_inject.push(MethodDef {
                name: required.name.clone(),
                params: required.params.clone(),
                body: required
                    .default_body
                    .clone()
                    .unwrap_or(crate::ast::Expr::Literal(crate::ast::Literal::Null)),
                bytecode: required.default_bytecode.clone(),
                private: false,
                owner_class: Some(class_def.name.clone()),
            });
        }
    }
    class_def.methods.extend(to_inject);
    Ok(())
}

pub fn validate_class_interfaces(class_def: &ClassDef, env: &Environment) -> Result<(), String> {
    if class_def.implements.is_empty() {
        return Ok(());
    }
    let methods = collect_class_method_arities(class_def, env)?;
    for iface_name in &class_def.implements {
        let base = iface_name.split('$').next().unwrap_or(iface_name);
        let iface = env
            .get_interface(iface_name)
            .or_else(|| env.get_interface(base))
            .ok_or_else(|| format!("Unknown interface: {}", iface_name))?;
        for assoc in &iface.associated_types {
            if !class_def
                .associated_types
                .iter()
                .any(|(n, _)| n == assoc)
            {
                return Err(format!(
                    "Class {} does not declare associated type {}.{}",
                    class_def.name, iface_name, assoc
                ));
            }
        }
        for required in &iface.methods {
            let Some(arity) = methods.get(&required.name) else {
                return Err(format!(
                    "Class {} does not implement {}.{}",
                    class_def.name, iface_name, required.name
                ));
            };
            if *arity != required.params.len() {
                return Err(format!(
                    "Class {} method {} arity mismatch for interface {}",
                    class_def.name, required.name, iface_name
                ));
            }
        }
    }
    Ok(())
}

fn schedule_async(
    params: Vec<String>,
    defaults: Vec<Option<crate::ast::Expr>>,
    rest: Option<String>,
    body: crate::ast::Expr,
    closure_env: Environment,
    args: Vec<Value>,
    env: &Environment,
) -> Result<Value, String> {
    let mut bind_env = Environment::child(closure_env.clone());
    bind_call_params(&params, &defaults, &rest, &args, &mut bind_env)?;
    let mut bindings = Vec::new();
    for name in &params {
        bindings.push((name.clone(), bind_env.get(name).unwrap()));
    }
    if let Some(rest_name) = &rest {
        bindings.push((rest_name.clone(), bind_env.get(rest_name).unwrap()));
    }
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    env.schedule_microtask(Microtask {
        promise: promise.clone(),
        params: Vec::new(),
        body: AsyncBody::Ast(body),
        env: closure_env,
        args: Vec::new(),
        bindings,
        writeback: Some(env.share_bindings()),
    });
    Ok(Value::Promise(promise))
}

fn run_microtask(task: Microtask) -> Result<(), String> {
    // Keep a shared handle to the scheduled closure so capture/module mutations
    // can be written back (L4).
    let mut closure_root = task.env.share_bindings();
    // Async imported bytecode retains its module closure, but its native calls
    // must use the scheduler that queued this task for the invoking program.
    let scheduler_owner = task.writeback.as_ref().unwrap_or(&closure_root);
    let mut call_env = Environment::child_from_with_scheduler(&closure_root, scheduler_owner);
    let result = match &task.body {
        AsyncBody::Ast(body) => {
            if !task.bindings.is_empty() {
                for (p, a) in &task.bindings {
                    call_env.set(p.clone(), a.clone());
                }
            } else {
                for (p, a) in task.params.iter().zip(&task.args) {
                    call_env.set(p.clone(), a.clone());
                }
            }
            eval_expr(body, &mut call_env)?
        }
        AsyncBody::Bytecode(func) => {
            crate::bytecode::run_bytecode_fn(func, task.args, &mut call_env)?
        }
    };
    let closure_view = closure_root.share_bindings();
    // Globals/module lets only — do not sync arbitrary fn-locals onto a parent frame
    // (that breaks recursive async with same local names).
    if let AsyncBody::Bytecode(func) = &task.body {
        let view = crate::value::BytecodeFunction {
            def: func.clone(),
            closure: closure_root.share_bindings(),
        };
        crate::runtime::closure_sync::sync_bytecode_globals_to_root(
            &view,
            &call_env,
            &mut closure_root,
        );
        if let Some(mut writeback) = task.writeback {
            let wb_view = writeback.share_bindings();
            crate::runtime::closure_sync::sync_bytecode_globals_to_root(
                &view,
                &call_env,
                &mut writeback,
            );
            // Also copy globals that landed on the closure root into the call-site frame
            // when closure and writeback are distinct (module let dual local/global).
            for name in &func.globals {
                if let Some(v) = closure_root.get(name).or_else(|| call_env.get(name)) {
                    if matches!(v, Value::Undefined) {
                        continue;
                    }
                    if writeback.get(name).is_some() {
                        let _ = writeback.assign(name, v);
                    } else if wb_view.get(name).is_some() {
                        let _ = writeback.assign(name, v);
                    } else {
                        writeback.set(name.clone(), v);
                    }
                }
            }
        }
    } else {
        crate::runtime::closure_sync::sync_closure_writes(&closure_view, &call_env, &mut closure_root);
        if let Some(mut writeback) = task.writeback {
            let wb_view = writeback.share_bindings();
            crate::runtime::closure_sync::sync_closure_writes(&wb_view, &call_env, &mut writeback);
        }
    }
    let resolved = resolve_await_value(result, &mut call_env)?;
    *task.promise.borrow_mut() = PromiseValue::Resolved(resolved);
    Ok(())
}

pub(crate) fn drain_scheduler_step(env: &mut Environment) -> Result<bool, String> {
    if drain_one_microtask(env)? {
        return Ok(true);
    }
    if drain_one_microtask_callback(env)? {
        return Ok(true);
    }
    if drain_one_timer_callback(env)? {
        return Ok(true);
    }
    if crate::runtime::io_async::drain_one_ready_io(env)? {
        return Ok(true);
    }
    if env.has_pending_sleeps() || env.has_pending_io() {
        if env.has_pending_wall_sleeps() {
            if let Some(ms) = env.ms_until_wall_wake() {
                if ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
            }
        }
        let tick_before = env.current_tick();
        let woke = env.wake_ready_sleeps();
        if crate::runtime::io_async::drain_one_ready_io(env)? {
            return Ok(true);
        }
        if woke {
            if drain_one_timer_callback(env)? {
                return Ok(true);
            }
            return Ok(true);
        }
        let _ = tick_before;
    }
    Ok(false)
}

fn drain_one_timer_callback(env: &mut Environment) -> Result<bool, String> {
    let Some(cb) = env.pop_timer_callback() else {
        return Ok(false);
    };
    crate::bytecode::call_value(cb.func, cb.args, &[], &[], &[], &[], env)?;
    Ok(true)
}

fn drain_one_microtask_callback(env: &mut Environment) -> Result<bool, String> {
    let cb = env.pop_microtask_callback();
    let Some(cb) = cb else {
        return Ok(false);
    };
    crate::bytecode::call_value(cb.func, cb.args, &[], &[], &[], &[], env)?;
    Ok(true)
}

fn drain_one_microtask(env: &Environment) -> Result<bool, String> {
    let task = env.pop_microtask();
    match task {
        None => Ok(false),
        Some(t) => {
            run_microtask(t)?;
            Ok(true)
        }
    }
}

fn drain_until_resolved(promise: &SharedPromise, env: &mut Environment) -> Result<(), String> {
    const MAX_STEPS: usize = 1_000_000;
    let mut steps = 0usize;
    while matches!(*promise.borrow(), PromiseValue::Pending) {
        steps += 1;
        if steps > MAX_STEPS {
            return Err("await drain exceeded maximum steps".into());
        }
        if !drain_scheduler_step(env)? {
            return Err("Awaited promise was never scheduled".into());
        }
    }
    Ok(())
}

pub fn drain_all_microtasks(env: &mut Environment) -> Result<(), String> {
    const MAX_ROUNDS: usize = 1024;
    const MAX_STEPS_PER_ROUND: usize = 1_000_000;
    for _ in 0..MAX_ROUNDS {
        let mut progressed = false;
        let mut steps = 0usize;
        while drain_scheduler_step(env)? {
            progressed = true;
            steps += 1;
            if steps > MAX_STEPS_PER_ROUND {
                return Err("microtask drain exceeded maximum steps".into());
            }
        }
        crate::runtime::stdlib::weak::run_gc_sweep(env)?;
        while drain_scheduler_step(env)? {
            progressed = true;
            steps += 1;
            if steps > MAX_STEPS_PER_ROUND {
                return Err("microtask drain exceeded maximum steps".into());
            }
        }
        if !progressed {
            return Ok(());
        }
    }
    Err("microtask drain exceeded maximum rounds".into())
}

pub fn resolve_await_value(val: Value, env: &mut Environment) -> Result<Value, String> {
    resolve_await(val, env)
}

fn resolve_await(val: Value, env: &mut Environment) -> Result<Value, String> {
    match val {
        Value::Promise(p) => {
            let state = p.borrow().clone();
            match state {
                PromiseValue::Resolved(v) => {
                    if let Some(reason) = crate::runtime::stdlib::promise::promise_rejection_reason(&v) {
                        return Err(format!(
                            "promise rejected: {}",
                            format_value(&reason)
                        ));
                    }
                    resolve_await(
                        crate::runtime::stdlib::promise::unwrap_fulfilled(v),
                        env,
                    )
                }
                PromiseValue::Pending => {
                    drain_until_resolved(&p, env)?;
                    let state = p.borrow().clone();
                    match state {
                        PromiseValue::Resolved(v) => {
                            if let Some(reason) =
                                crate::runtime::stdlib::promise::promise_rejection_reason(&v)
                            {
                                return Err(format!(
                                    "promise rejected: {}",
                                    format_value(&reason)
                                ));
                            }
                            resolve_await(crate::runtime::stdlib::promise::unwrap_fulfilled(v), env)
                        }
                        PromiseValue::Pending => Err("Promise remained pending after drain".into()),
                    }
                }
            }
        }
        other => Ok(other),
    }
}

fn pattern_matches(pattern: &Pattern, value: &Value) -> bool {
    match (pattern, value) {
        (Pattern::Wildcard, _) => true,
        (Pattern::Number(n), Value::Number(v)) => n == v,
        (Pattern::Float(f), v) => match v {
            Value::Float(vf) => vf.to_bits() == f.to_bits(),
            Value::Number(n) => (*n as f64).to_bits() == f.to_bits(),
            _ => false,
        },
        (Pattern::String(s), Value::String(v)) => s == v,
        (Pattern::Bool(b), Value::Bool(v)) => b == v,
        (Pattern::Null, Value::Null) => true,
        (Pattern::Undefined, Value::Undefined) => true,
        (Pattern::Nan, v) => v.is_nan(),
        (Pattern::Variable(_), _) => true,
        (Pattern::Some(inner), Value::Option(Some(v))) => pattern_matches(inner, v),
        (Pattern::None, Value::Option(None)) => true,
        (Pattern::Ok(inner), Value::Result(Ok(v))) => pattern_matches(inner, v),
        (Pattern::Err(inner), Value::Result(Err(v))) => pattern_matches(inner, v),
        (
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
            },
            Value::EnumValue {
                type_name,
                variant: vname,
                fields: payload,
            },
        ) => {
            variant == vname
                && (enum_name == type_name || type_name.starts_with(&format!("{enum_name}$")))
                && fields.len() == payload.len()
                && fields
                    .iter()
                    .zip(payload.iter())
                    .all(|(p, v)| pattern_matches(p, v))
        }
        (Pattern::Array(pieces), Value::Array(vals)) => array_pattern_matches(pieces, vals),
        (Pattern::Object(fields), Value::Object(map)) => object_pattern_matches(fields, map),
        _ => false,
    }
}

fn array_pattern_matches(pieces: &[PatternPiece], vals: &[Value]) -> bool {
    let rest_at = pieces.iter().position(|p| matches!(p, PatternPiece::Rest(_)));
    match rest_at {
        Some(idx) => {
            let fixed_before = &pieces[..idx];
            let fixed_after = &pieces[idx + 1..];
            if vals.len() < fixed_before.len() + fixed_after.len() {
                return false;
            }
            for (piece, val) in fixed_before.iter().zip(vals.iter()) {
                if !array_piece_matches(piece, val) {
                    return false;
                }
            }
            let after_start = vals.len() - fixed_after.len();
            for (piece, val) in fixed_after.iter().zip(&vals[after_start..]) {
                if !array_piece_matches(piece, val) {
                    return false;
                }
            }
            true
        }
        None => {
            if pieces.len() != vals.len() {
                return false;
            }
            pieces
                .iter()
                .zip(vals.iter())
                .all(|(piece, val)| array_piece_matches(piece, val))
        }
    }
}

fn array_piece_matches(piece: &PatternPiece, val: &Value) -> bool {
    match piece {
        PatternPiece::Item(pat) => pattern_matches(pat, val),
        PatternPiece::Wildcard => true,
        PatternPiece::Rest(_) => false,
    }
}

fn object_pattern_matches(fields: &[PatternField], map: &HashMap<String, Value>) -> bool {
    if fields.is_empty() {
        return crate::runtime::stdlib::object::object_is_pattern_empty(map);
    }
    for field in fields.iter() {
        match field {
            PatternField::Shorthand(key) => {
                if !map.contains_key(key) {
                    return false;
                }
            }
            PatternField::Field { key, pattern } => {
                let Some(val) = map.get(key) else {
                    return false;
                };
                if !pattern_matches(pattern, val) {
                    return false;
                }
            }
            PatternField::Rest(_) => {}
        }
    }
    true
}

fn bind_pattern(pattern: &Pattern, value: &Value, env: &mut Environment) {
    match pattern {
        Pattern::Variable(name) => {
            env.set(name.clone(), value.clone());
        }
        Pattern::Array(pieces) => {
            if let Value::Array(vals) = value {
                bind_array_pattern(pieces, vals, env);
            }
        }
        Pattern::Object(fields) => {
            if let Value::Object(map) = value {
                bind_object_pattern(fields, map, env);
            }
        }
        Pattern::Some(inner) => {
            if let Value::Option(Some(v)) = value {
                bind_pattern(inner, v, env);
            }
        }
        Pattern::Ok(inner) => {
            if let Value::Result(Ok(v)) = value {
                bind_pattern(inner, v, env);
            }
        }
        Pattern::Err(inner) => {
            if let Value::Result(Err(v)) = value {
                bind_pattern(inner, v, env);
            }
        }
        Pattern::EnumVariant { fields, .. } => {
            if let Value::EnumValue { fields: payload, .. } = value {
                for (pat, val) in fields.iter().zip(payload.iter()) {
                    bind_pattern(pat, val, env);
                }
            }
        }
        _ => {}
    }
}

fn bind_array_pattern(pieces: &[PatternPiece], vals: &[Value], env: &mut Environment) {
    let rest_at = pieces.iter().position(|p| matches!(p, PatternPiece::Rest(_)));
    match rest_at {
        Some(idx) => {
            let fixed_before = &pieces[..idx];
            let fixed_after = &pieces[idx + 1..];
            for (piece, val) in fixed_before.iter().zip(vals.iter()) {
                bind_array_piece(piece, val, env);
            }
            if let PatternPiece::Rest(name) = &pieces[idx] {
                if !name.is_empty() {
                    let after_start = vals.len().saturating_sub(fixed_after.len());
                    let rest_vals = vals[idx..after_start].to_vec();
                    env.set(name.clone(), Value::from_array(rest_vals));
                }
            }
            let after_start = vals.len() - fixed_after.len();
            for (piece, val) in fixed_after.iter().zip(&vals[after_start..]) {
                bind_array_piece(piece, val, env);
            }
        }
        None => {
            for (piece, val) in pieces.iter().zip(vals.iter()) {
                bind_array_piece(piece, val, env);
            }
        }
    }
}

fn bind_array_piece(piece: &PatternPiece, val: &Value, env: &mut Environment) {
    match piece {
        PatternPiece::Item(pat) => bind_pattern(pat, val, env),
        PatternPiece::Wildcard => {}
        PatternPiece::Rest(name) => {
            if !name.is_empty() {
                if let Value::Array(items) = val {
                    env.set(name.clone(), Value::from_array(items.as_ref().clone()));
                }
            }
        }
    }
}

fn bind_object_pattern(fields: &[PatternField], map: &HashMap<String, Value>, env: &mut Environment) {
    let mut bound_keys = HashSet::new();
    for field in fields.iter() {
        match field {
            PatternField::Shorthand(key) => {
                let v = map.get(key).cloned().unwrap_or(Value::Undefined);
                env.set(key.clone(), v);
                bound_keys.insert(key.clone());
            }
            PatternField::Field { key, pattern } => {
                let v = map.get(key).cloned().unwrap_or(Value::Undefined);
                bind_pattern(pattern, &v, env);
                bound_keys.insert(key.clone());
            }
            PatternField::Rest(name) => {
                if name.is_empty() {
                    continue;
                }
                let mut rest = HashMap::new();
                for (k, v) in map.iter() {
                    if !bound_keys.contains(k) {
                        rest.insert(k.clone(), v.clone());
                    }
                }
                env.set(name.clone(), Value::from_object(rest));
            }
        }
    }
}
