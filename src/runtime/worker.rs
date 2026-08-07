//! Deno Worker parity — isolated OS-thread workers with JSON message channels.

use crate::evaluator::{create_global_env, eval_source};
use crate::runtime::stdlib::json;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static NEXT_WORKER: AtomicU64 = AtomicU64::new(1);

pub fn worker_object(id: u64) -> Value {
    let mut m = HashMap::new();
    m.insert("__kab_worker".into(), Value::Bool(true));
    m.insert("__kab_id".into(), Value::Number(id as i64));
    Value::from_object(m)
}

pub fn worker_id(v: &Value) -> Result<u64, String> {
    let Value::Object(o) = v else {
        return Err("expected worker".into());
    };
    if !matches!(o.get("__kab_worker"), Some(Value::Bool(true))) {
        return Err("expected worker".into());
    }
    match o.get("__kab_id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid worker handle".into()),
    }
}

fn invoke_callback(env: &mut Environment, func: &Value, args: Vec<Value>) -> Result<Value, String> {
    crate::bytecode::call_value(func.clone(), args, &[], &[], &[], &[], env)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn dispatch_main_onmessage(
    env: &mut Environment,
    id: u64,
    msg: &Value,
) -> Result<(), String> {
    imp::dispatch_main_onmessage(env, id, msg)
}

#[cfg(target_arch = "wasm32")]
pub fn dispatch_main_onmessage(
    env: &mut Environment,
    id: u64,
    msg: &Value,
) -> Result<(), String> {
    imp::dispatch_main_onmessage(env, id, msg)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn dispatch_worker_onmessage(env: &mut Environment, msg: &Value) -> Result<(), String> {
    let msg = crate::runtime::web_streams::adopt_transfers_in_message(msg)?;
    imp::dispatch_worker_onmessage(env, &msg)
}

#[cfg(target_arch = "wasm32")]
pub fn dispatch_worker_onmessage(env: &mut Environment, msg: &Value) -> Result<(), String> {
    let msg = crate::runtime::web_streams::adopt_transfers_in_message(msg)?;
    imp::dispatch_worker_onmessage(env, &msg)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn worker_poll_ipc(timeout_ms: u64) -> Result<Value, String> {
    imp::worker_poll_ipc(timeout_ms)
}

#[cfg(target_arch = "wasm32")]
pub fn worker_poll_ipc(timeout_ms: u64) -> Result<Value, String> {
    imp::worker_poll_ipc(timeout_ms)
}

pub fn worker_set_onmessage(id: u64, handler: Value) -> Result<(), String> {
    imp::worker_set_onmessage(id, handler)
}

pub fn worker_run_message_loop(env: &mut Environment) -> Result<(), String> {
    imp::worker_run_message_loop(env)
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};

    thread_local! {
        static WORKERS: RefCell<HashMap<u64, WorkerState>> = RefCell::new(HashMap::new());
    }

    struct WorkerState {
        inbox: VecDeque<Value>,
        outbox: VecDeque<Value>,
        env: Option<Environment>,
        started: bool,
        onmessage: Option<Value>,
    }

    thread_local! {
        static WORKER_ONMESSAGE: RefCell<Option<Value>> = const { RefCell::new(None) };
    }

    pub fn worker_set_onmessage(id: u64, handler: Value) -> Result<(), String> {
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            w.onmessage = Some(handler);
            Ok(())
        })
    }

    pub fn dispatch_main_onmessage(
        env: &mut Environment,
        id: u64,
        msg: &Value,
    ) -> Result<(), String> {
        let handler = WORKERS.with(|m| {
            m.borrow()
                .get(&id)
                .and_then(|w| w.onmessage.clone())
        });
        if let Some(handler) = handler {
            let _ = super::invoke_callback(env, &handler, vec![msg.clone()])?;
        }
        Ok(())
    }

    pub fn dispatch_worker_onmessage(env: &mut Environment, msg: &Value) -> Result<(), String> {
        let handler = WORKER_ONMESSAGE.with(|h| h.borrow().clone());
        if let Some(handler) = handler {
            let _ = super::invoke_callback(env, &handler, vec![msg.clone()])?;
        }
        Ok(())
    }

    fn onmessage_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
        let handler = args.first().cloned().ok_or("onmessage(handler)")?;
        WORKER_ONMESSAGE.with(|h| *h.borrow_mut() = Some(handler));
        Ok(Value::Undefined)
    }

    fn post_message_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        worker_reply_native(args, env)
    }

    pub fn worker_poll_ipc(_timeout_ms: u64) -> Result<Value, String> {
        Err("worker_poll_ipc is not available on wasm32".into())
    }

    fn worker_poll_wait_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        let msg = worker_poll_native(args, env)?;
        dispatch_worker_onmessage(env, &msg)?;
        Ok(msg)
    }

    fn worker_run_message_loop_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
        worker_run_message_loop(env)?;
        Ok(Value::Undefined)
    }

    pub fn worker_run_message_loop(env: &mut Environment) -> Result<(), String> {
        let worker_id = worker_id_from_env(env)?;
        loop {
            let running = WORKERS.with(|m| m.borrow().contains_key(&worker_id));
            if !running {
                break;
            }
            let msg = worker_poll_native(&[], env)?;
            if !matches!(msg, Value::Null) {
                dispatch_worker_onmessage(env, &msg)?;
            }
        }
        Ok(())
    }

    fn worker_id_from_env(env: &Environment) -> Result<u64, String> {
        env.get("__kab_worker_id")
            .and_then(|v| match v {
                Value::Number(n) if *n > 0 => Some(*n as u64),
                _ => None,
            })
            .ok_or("worker API outside worker context".into())
    }

    fn worker_reply_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        let msg = args.first().cloned().unwrap_or(Value::Null);
        let worker_id = worker_id_from_env(env)?;
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&worker_id)
                .ok_or_else(|| format!("invalid worker id {worker_id}"))?;
            w.outbox.push_back(msg);
            Ok(Value::Undefined)
        })
    }

    fn worker_poll_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
        let worker_id = worker_id_from_env(env)?;
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&worker_id)
                .ok_or_else(|| format!("invalid worker id {worker_id}"))?;
            Ok(w.inbox.pop_front().unwrap_or(Value::Null))
        })
    }

    fn build_worker_env(id: u64) -> Environment {
        let mut env = create_global_env();
        env.set(
            "worker_reply".to_string(),
            Value::NativeFunction(worker_reply_native),
        );
        env.set(
            "worker_poll".to_string(),
            Value::NativeFunction(worker_poll_native),
        );
        env.set(
            "worker_poll_wait".to_string(),
            Value::NativeFunction(worker_poll_wait_native),
        );
        env.set("onmessage".to_string(), Value::NativeFunction(onmessage_native));
        env.set(
            "postMessage".to_string(),
            Value::NativeFunction(post_message_native),
        );
        env.set(
            "worker_run_message_loop".to_string(),
            Value::NativeFunction(worker_run_message_loop_native),
        );
        env.set("__kab_worker_id".to_string(), Value::Number(id as i64));
        env
    }

    fn ensure_worker_env(id: u64) -> Result<(), String> {
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            if w.env.is_none() {
                w.env = Some(build_worker_env(id));
            }
            Ok(())
        })
    }

    pub fn worker_new() -> u64 {
        let id = NEXT_WORKER.fetch_add(1, Ordering::Relaxed);
        WORKERS.with(|m| {
            m.borrow_mut().insert(
                id,
                WorkerState {
                    inbox: VecDeque::new(),
                    outbox: VecDeque::new(),
                    env: None,
                    started: false,
                    onmessage: None,
                },
            );
        });
        id
    }

    pub fn worker_start(id: u64, code: &str) -> Result<(), String> {
        ensure_worker_env(id)?;
        let already = WORKERS.with(|m| {
            m.borrow()
                .get(&id)
                .map(|w| w.started)
                .unwrap_or(false)
        });
        if already {
            return Err("worker already started".into());
        }
        let mut env = WORKERS.with(|m| -> Result<Environment, String> {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            w.env
                .take()
                .ok_or_else(|| "worker env missing".to_string())
        })?;
        eval_source(code, &mut env)?;
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            w.env = Some(env);
            w.started = true;
            Ok(())
        })
    }

    pub fn worker_start_file(_id: u64, _path: &str) -> Result<(), String> {
        Err("worker_start_file is not available on wasm32".into())
    }

    pub fn worker_post_message(id: u64, msg: Value) -> Result<(), String> {
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            w.inbox.push_back(msg);
            Ok(())
        })
    }

    pub fn worker_recv(id: u64) -> Result<Value, String> {
        WORKERS.with(|m| {
            let mut map = m.borrow_mut();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            Ok(w.outbox.pop_front().unwrap_or(Value::Null))
        })
    }

    pub fn worker_join(_id: u64) -> Result<(), String> {
        Ok(())
    }

    pub fn worker_terminate(id: u64) -> Result<(), String> {
        WORKERS.with(|m| {
            if m.borrow_mut().remove(&id).is_some() {
                Ok(())
            } else {
                Err(format!("invalid worker id {id}"))
            }
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
    use std::sync::{Mutex, OnceLock};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    fn workers_map() -> &'static Mutex<HashMap<u64, WorkerState>> {
        static MAP: OnceLock<Mutex<HashMap<u64, WorkerState>>> = OnceLock::new();
        MAP.get_or_init(|| Mutex::new(HashMap::new()))
    }

    struct WorkerState {
        inbox_tx: Sender<String>,
        outbox_rx: Receiver<String>,
        inbox_rx: Mutex<Option<Receiver<String>>>,
        outbox_tx: Mutex<Option<Sender<String>>>,
        handle: Mutex<Option<JoinHandle<()>>>,
        started: AtomicBool,
        terminated: AtomicBool,
    }

    thread_local! {
        static WORKER_IPC: std::cell::RefCell<Option<WorkerIpc>> = const { std::cell::RefCell::new(None) };
        static WORKER_ONMESSAGE: std::cell::RefCell<Option<Value>> = const { std::cell::RefCell::new(None) };
        static MAIN_ONMESSAGE: std::cell::RefCell<HashMap<u64, Value>> = std::cell::RefCell::new(HashMap::new());
    }

    struct WorkerIpc {
        inbox_rx: Receiver<String>,
        outbox_tx: Sender<String>,
    }

    fn encode_worker_msg(msg: &Value) -> String {
        json::stringify(msg)
    }

    fn decode_worker_msg(payload: &str) -> Result<Value, String> {
        json::parse(payload)
    }

    fn worker_reply_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
        let msg = args.first().cloned().unwrap_or(Value::Null);
        let payload = encode_worker_msg(&msg);
        WORKER_IPC.with(|ipc| {
            let guard = ipc.borrow();
            let Some(ipc) = guard.as_ref() else {
                return Err("worker_reply outside worker thread".into());
            };
            ipc.outbox_tx
                .send(payload)
                .map_err(|_| "worker outbox closed".to_string())?;
            Ok(Value::Undefined)
        })
    }

    fn worker_poll_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
        worker_poll_ipc(0)
    }

    pub fn worker_poll_ipc(timeout_ms: u64) -> Result<Value, String> {
        use std::sync::mpsc::RecvTimeoutError;
        WORKER_IPC.with(|ipc| {
            let guard = ipc.borrow();
            let Some(ipc) = guard.as_ref() else {
                return Err("worker_poll outside worker thread".into());
            };
            if timeout_ms == 0 {
                return match ipc.inbox_rx.try_recv() {
                    Ok(payload) => {
                        let msg = decode_worker_msg(&payload)?;
                        crate::runtime::web_streams::adopt_transfers_in_message(&msg)
                    }
                    Err(TryRecvError::Disconnected) => Ok(Value::Null),
                    Err(TryRecvError::Empty) => Ok(Value::Null),
                };
            }
            match ipc.inbox_rx.recv_timeout(Duration::from_millis(timeout_ms)) {
                Ok(payload) => {
                    let msg = decode_worker_msg(&payload)?;
                    crate::runtime::web_streams::adopt_transfers_in_message(&msg)
                }
                Err(RecvTimeoutError::Timeout) => Ok(Value::Null),
                Err(RecvTimeoutError::Disconnected) => Ok(Value::Null),
            }
        })
    }

    pub fn worker_set_onmessage(id: u64, handler: Value) -> Result<(), String> {
        MAIN_ONMESSAGE.with(|m| {
            m.borrow_mut().insert(id, handler);
            Ok(())
        })
    }

    pub fn dispatch_main_onmessage(
        env: &mut Environment,
        id: u64,
        msg: &Value,
    ) -> Result<(), String> {
        let handler = MAIN_ONMESSAGE.with(|m| m.borrow().get(&id).cloned());
        if let Some(handler) = handler {
            let _ = super::invoke_callback(env, &handler, vec![msg.clone()])?;
        }
        Ok(())
    }

    pub fn dispatch_worker_onmessage(env: &mut Environment, msg: &Value) -> Result<(), String> {
        let handler = WORKER_ONMESSAGE.with(|h| h.borrow().clone());
        if let Some(handler) = handler {
            let _ = super::invoke_callback(env, &handler, vec![msg.clone()])?;
        }
        Ok(())
    }

    fn onmessage_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
        let handler = args.first().cloned().ok_or("onmessage(handler)")?;
        WORKER_ONMESSAGE.with(|h| *h.borrow_mut() = Some(handler));
        Ok(Value::Undefined)
    }

    fn post_message_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        worker_reply_native(args, env)
    }

    fn worker_poll_wait_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        let timeout_ms = match args.first() {
            Some(Value::Number(n)) if *n >= 0 => *n as u64,
            _ => 5000,
        };
        let msg = worker_poll_ipc(timeout_ms)?;
        dispatch_worker_onmessage(env, &msg)?;
        Ok(msg)
    }

    fn worker_run_message_loop_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
        worker_run_message_loop(env)?;
        Ok(Value::Undefined)
    }

    pub fn worker_run_message_loop(env: &mut Environment) -> Result<(), String> {
        let worker_id = env
            .get("__kab_worker_id")
            .and_then(|v| match v {
                Value::Number(n) if n > 0 => Some(n as u64),
                _ => None,
            })
            .ok_or("worker_run_message_loop outside worker context")?;
        loop {
            let terminated = workers_map()
                .lock()
                .unwrap()
                .get(&worker_id)
                .map(|w| w.terminated.load(Ordering::Acquire))
                .unwrap_or(true);
            if terminated {
                break;
            }
            let msg = worker_poll_ipc(100)?;
            if !matches!(msg, Value::Null) {
                dispatch_worker_onmessage(env, &msg)?;
            }
        }
        Ok(())
    }

    fn import_scripts_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
        for arg in args.iter() {
            let Value::String(path) = arg else {
                return Err("importScripts(path, ...) expects string paths".into());
            };
            let code = std::fs::read_to_string(path)
                .map_err(|e| format!("importScripts({path}): {e}"))?;
            eval_source(&code, env)?;
        }
        Ok(Value::Undefined)
    }

    fn build_worker_env(id: u64, inbox_rx: Receiver<String>, outbox_tx: Sender<String>) -> Environment {
        WORKER_IPC.with(|ipc| {
            *ipc.borrow_mut() = Some(WorkerIpc { inbox_rx, outbox_tx });
        });
        let mut env = create_global_env();
        env.set(
            "worker_reply".to_string(),
            Value::NativeFunction(worker_reply_native),
        );
        env.set(
            "worker_poll".to_string(),
            Value::NativeFunction(worker_poll_native),
        );
        env.set(
            "worker_poll_wait".to_string(),
            Value::NativeFunction(worker_poll_wait_native),
        );
        env.set(
            "importScripts".to_string(),
            Value::NativeFunction(import_scripts_native),
        );
        env.set("onmessage".to_string(), Value::NativeFunction(onmessage_native));
        env.set(
            "postMessage".to_string(),
            Value::NativeFunction(post_message_native),
        );
        env.set(
            "worker_run_message_loop".to_string(),
            Value::NativeFunction(worker_run_message_loop_native),
        );
        env.set("__kab_worker_id".to_string(), Value::Number(id as i64));
        env
    }

    pub fn worker_new() -> u64 {
        let id = NEXT_WORKER.fetch_add(1, Ordering::Relaxed);
        let (inbox_tx, inbox_rx) = mpsc::channel();
        let (outbox_tx, outbox_rx) = mpsc::channel();
        workers_map().lock().unwrap().insert(
            id,
            WorkerState {
                inbox_tx,
                outbox_rx,
                inbox_rx: Mutex::new(Some(inbox_rx)),
                outbox_tx: Mutex::new(Some(outbox_tx)),
                handle: Mutex::new(None),
                started: AtomicBool::new(false),
                terminated: AtomicBool::new(false),
            },
        );
        id
    }

    pub fn worker_start(id: u64, code: &str) -> Result<(), String> {
        let (inbox_rx, outbox_tx) = {
            let map = workers_map().lock().unwrap();
            let w = map
                .get(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            if w.started.load(Ordering::Acquire) {
                return Err("worker already started".into());
            }
            if w.terminated.load(Ordering::Acquire) {
                return Err("worker terminated".into());
            }
            let inbox_rx = w
                .inbox_rx
                .lock()
                .unwrap()
                .take()
                .ok_or("worker inbox already taken")?;
            let outbox_tx = w
                .outbox_tx
                .lock()
                .unwrap()
                .take()
                .ok_or("worker outbox already taken")?;
            (inbox_rx, outbox_tx)
        };

        let code = code.to_string();
        let handle = thread::Builder::new()
            .name(format!("kab-worker-{id}"))
            .spawn(move || {
                let mut env = build_worker_env(id, inbox_rx, outbox_tx);
                let _ = eval_source(&code, &mut env);
                WORKER_IPC.with(|ipc| *ipc.borrow_mut() = None);
                WORKER_ONMESSAGE.with(|h| *h.borrow_mut() = None);
            })
            .map_err(|e| format!("worker thread spawn failed: {e}"))?;

        let mut map = workers_map().lock().unwrap();
        let w = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid worker id {id}"))?;
        *w.handle.lock().unwrap() = Some(handle);
        w.started.store(true, Ordering::Release);
        Ok(())
    }

    pub fn worker_start_file(id: u64, path: &str) -> Result<(), String> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("worker_start_file({path}): {e}"))?;
        worker_start(id, &code)
    }

    pub fn worker_post_message(id: u64, msg: Value) -> Result<(), String> {
        let map = workers_map().lock().unwrap();
        let w = map
            .get(&id)
            .ok_or_else(|| format!("invalid worker id {id}"))?;
        if w.terminated.load(Ordering::Acquire) {
            return Err("worker terminated".into());
        }
        w.inbox_tx
            .send(encode_worker_msg(&msg))
            .map_err(|_| "worker inbox closed".to_string())
    }

    pub fn worker_recv(id: u64) -> Result<Value, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (payload, finished) = {
                let map = workers_map().lock().unwrap();
                let w = map
                    .get(&id)
                    .ok_or_else(|| format!("invalid worker id {id}"))?;
                let payload = match w.outbox_rx.try_recv() {
                    Ok(p) => Some(p),
                    Err(TryRecvError::Disconnected) => return Ok(Value::Null),
                    Err(TryRecvError::Empty) => None,
                };
                let finished = w
                    .handle
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|h| h.is_finished());
                (payload, finished)
            };
            if let Some(payload) = payload {
                return decode_worker_msg(&payload);
            }
            if finished || Instant::now() >= deadline {
                return Ok(Value::Null);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn worker_join(id: u64) -> Result<(), String> {
        let handle = {
            let mut map = workers_map().lock().unwrap();
            let w = map
                .get_mut(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            let handle = w.handle.lock().unwrap().take();
            handle
        };
        if let Some(handle) = handle {
            handle
                .join()
                .map_err(|_| "worker thread panicked".to_string())?;
        }
        Ok(())
    }

    pub fn worker_terminate(id: u64) -> Result<(), String> {
        MAIN_ONMESSAGE.with(|m| {
            m.borrow_mut().remove(&id);
        });
        let handle = {
            let mut map = workers_map().lock().unwrap();
            let w = map
                .remove(&id)
                .ok_or_else(|| format!("invalid worker id {id}"))?;
            w.terminated.store(true, Ordering::Release);
            drop(w.inbox_tx);
            let handle = w.handle.lock().unwrap().take();
            handle
        };
        if let Some(handle) = handle {
            let _ = handle.join();
        }
        Ok(())
    }
}

pub use imp::{
    worker_join, worker_new, worker_post_message, worker_recv, worker_start, worker_start_file,
    worker_terminate,
};
