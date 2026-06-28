//! Async IO — Promise-baserade wrappers för OS, HTTP och SQL (v2.8).

use crate::runtime::stdlib::abort::{abort_reason, is_aborted, rejected_abort_promise};
use crate::runtime::db::DbConnection;
use crate::runtime::http::HttpRequest;
use crate::runtime::os::OsHandle;
use crate::value::{
    Environment, IoOp, IoTask, PromiseValue, SharedPromise, Value,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const IO_LATENCY_TICKS: u64 = 1;

pub fn schedule_io_promise(op: IoOp, env: &Environment, abort_id: Option<u64>) -> Value {
    if let Some(id) = abort_id {
        if is_aborted(id) {
            return rejected_abort_promise(abort_reason(id));
        }
    }
    let promise: SharedPromise = Rc::new(RefCell::new(PromiseValue::Pending));
    env.schedule_io(IoTask {
        promise: promise.clone(),
        wake_at: env.current_tick() + IO_LATENCY_TICKS,
        op,
        abort_id,
    });
    Value::Promise(promise)
}

pub fn drain_one_ready_io(env: &mut Environment) -> Result<bool, String> {
    let tick = env.current_tick();
    let task = env.pop_ready_io(tick);
    let Some(task) = task else {
        return Ok(false);
    };
    if let Some(id) = task.abort_id {
        if is_aborted(id) {
            *task.promise.borrow_mut() = PromiseValue::Resolved(Value::Result(Err(Box::new(
                abort_reason(id),
            ))));
            return Ok(true);
        }
    }
    let result = execute_io_op(&task.op, env)?;
    *task.promise.borrow_mut() = PromiseValue::Resolved(result);
    Ok(true)
}

fn execute_io_op(op: &IoOp, env: &mut Environment) -> Result<Value, String> {
    match op {
        IoOp::OsRead(path) => {
            let content = get_os(env)?.read(path)?;
            Ok(Value::String(content))
        }
        IoOp::OsWrite { path, content } => {
            get_os(env)?.write(path, content.clone())?;
            Ok(Value::Null)
        }
        IoOp::HttpRequest { method, path, body } => {
            let req = HttpRequest::new(method.clone(), path.clone(), body.clone());
            let res = crate::http_dispatch::dispatch(env, &req)?;
            Ok(Value::HttpResponse(res))
        }
        IoOp::Sql { query, params } => {
            let db = get_db(env)?;
            db.execute_sql(query, params)
        }
        IoOp::HttpFetch {
            method,
            url,
            body,
            headers,
            timeout_ms,
        } => {
            let trust = env.tls_trust();
            let res = crate::runtime::net::http_fetch(
                method,
                url,
                body,
                headers,
                &trust,
                *timeout_ms,
            )?;
            Ok(Value::HttpResponse(res))
        }
        IoOp::StreamRead { stream_id } => {
            crate::runtime::stdlib::deno::stream_read_impl(*stream_id)
        }
        IoOp::StreamReadAll { stream_id } => {
            crate::runtime::stdlib::deno::stream_read_all_impl(*stream_id)
        }
        IoOp::StreamPipeTo { src_id, dest_id } => {
            crate::runtime::stdlib::deno::stream_pipe_to_impl(*src_id, *dest_id)?;
            Ok(Value::Undefined)
        }
        IoOp::ReaderRead { reader_id } => {
            let stream_id = crate::runtime::web_streams::reader_stream_id_pub(*reader_id)?;
            crate::runtime::stdlib::deno::stream_read_impl(stream_id)
        }
        IoOp::KvListenRead { stream_id } => {
            crate::runtime::open_kv::kv_stream_read_event(*stream_id)
        }
        IoOp::WorkerRecv { worker_id } => {
            let msg = crate::runtime::worker::worker_recv(*worker_id)?;
            crate::runtime::worker::dispatch_main_onmessage(env, *worker_id, &msg)?;
            Ok(msg)
        }
        IoOp::WorkerPoll { timeout_ms } => {
            let msg = crate::runtime::worker::worker_poll_ipc(*timeout_ms)?;
            crate::runtime::worker::dispatch_worker_onmessage(env, &msg)?;
            Ok(msg)
        }
    }
}

fn get_os(env: &Environment) -> Result<OsHandle, String> {
    let os = env.get("os").ok_or("OS handle not available")?;
    let Value::OsHandle(handle) = os else {
        return Err("OS handle not available".into());
    };
    Ok(handle)
}

fn get_db(env: &Environment) -> Result<DbConnection, String> {
    let conn = env.get("db").ok_or("Database connection not available")?;
    let Value::DbConnection(db) = conn else {
        return Err("Database connection not available".into());
    };
    Ok(db)
}

fn expect_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{} expects a string argument", name)),
    }
}

fn os_read_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_read_async()")?;
    Ok(schedule_io_promise(IoOp::OsRead(path), env, None))
}

fn os_write_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_write_async()")?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => crate::value::format_value(other),
        None => String::new(),
    };
    Ok(schedule_io_promise(
        IoOp::OsWrite { path, content },
        env,
        None,
    ))
}

fn http_request_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let method = expect_string(args, 0, "http_request_async()")?;
    let path = expect_string(args, 1, "http_request_async()")?;
    let body = args
        .get(2)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => crate::value::format_value(other),
        })
        .unwrap_or_default();
    Ok(schedule_io_promise(
        IoOp::HttpRequest {
            method,
            path,
            body,
        },
        env,
        None,
    ))
}

fn sql_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let query = expect_string(args, 0, "sql_async()")?;
    let params: Vec<Value> = args.iter().skip(1).cloned().collect();
    Ok(schedule_io_promise(IoOp::Sql { query, params }, env, None))
}

fn value_to_headers(value: &Value, name: &str) -> Result<HashMap<String, String>, String> {
    let Value::Object(map) = value else {
        return Err(format!("{name} expects an object with header names as keys"));
    };
    let mut headers = HashMap::new();
    for (key, val) in map {
        let header_value = match val {
            Value::String(s) => s.clone(),
            other => crate::value::format_value(other),
        };
        headers.insert(key.clone(), header_value);
    }
    Ok(headers)
}

fn parse_timeout_ms(value: &Value, name: &str) -> Result<u64, String> {
    match value {
        Value::Number(n) if *n >= 0 => Ok(*n as u64),
        _ => Err(format!(
            "{name} timeout must be a non-negative number (milliseconds)"
        )),
    }
}

fn http_fetch_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let method = expect_string(args, 0, "http_fetch_async()")?;
    let url = expect_string(args, 1, "http_fetch_async()")?;
    let body = args
        .get(2)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => crate::value::format_value(other),
        })
        .unwrap_or_default();
    let headers = match args.get(3) {
        Some(v) => value_to_headers(v, "http_fetch_async()")?,
        None => HashMap::new(),
    };
    let timeout_ms = match args.get(4) {
        Some(v) => parse_timeout_ms(v, "http_fetch_async()")?,
        None => env.http_fetch_timeout_ms(),
    };
    Ok(schedule_io_promise(
        IoOp::HttpFetch {
            method,
            url,
            body,
            headers,
            timeout_ms,
        },
        env,
        None,
    ))
}

fn stream_read_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let stream = args.first().ok_or("stream_read_async(stream)")?;
    let id = crate::runtime::stdlib::deno::stream_id_pub(stream)?;
    Ok(schedule_io_promise(IoOp::StreamRead { stream_id: id }, env, None))
}

fn stream_read_all_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let stream = args.first().ok_or("stream_read_all_async(stream)")?;
    let id = crate::runtime::stdlib::deno::stream_id_pub(stream)?;
    Ok(schedule_io_promise(
        IoOp::StreamReadAll { stream_id: id },
        env,
        None,
    ))
}

fn stream_pipe_to_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let src = args.first().ok_or("stream_pipe_to_async(src, dest)")?;
    let dest = args.get(1).ok_or("stream_pipe_to_async(src, dest)")?;
    let src_id = crate::runtime::stdlib::deno::stream_id_pub(src)?;
    let dest_id = crate::runtime::stdlib::deno::writable_id_pub(dest)?;
    Ok(schedule_io_promise(
        IoOp::StreamPipeTo {
            src_id,
            dest_id,
        },
        env,
        None,
    ))
}

fn reader_read_async_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let reader = args.first().ok_or("reader_read_async(reader)")?;
    let reader_id = crate::runtime::web_streams::reader_id(reader)?;
    Ok(schedule_io_promise(
        IoOp::ReaderRead { reader_id },
        env,
        None,
    ))
}

pub fn io_async_globals(env: &mut Environment) {
    env.set(
        "os_read_async".to_string(),
        Value::NativeFunction(os_read_async_native),
    );
    env.set(
        "os_write_async".to_string(),
        Value::NativeFunction(os_write_async_native),
    );
    env.set(
        "http_request_async".to_string(),
        Value::NativeFunction(http_request_async_native),
    );
    env.set(
        "sql_async".to_string(),
        Value::NativeFunction(sql_async_native),
    );
    env.set(
        "http_fetch_async".to_string(),
        Value::NativeFunction(http_fetch_async_native),
    );
    env.set(
        "stream_read_async".to_string(),
        Value::NativeFunction(stream_read_async_native),
    );
    env.set(
        "stream_read_all_async".to_string(),
        Value::NativeFunction(stream_read_all_async_native),
    );
    env.set(
        "stream_pipe_to_async".to_string(),
        Value::NativeFunction(stream_pipe_to_async_native),
    );
    env.set(
        "reader_read_async".to_string(),
        Value::NativeFunction(reader_read_async_native),
    );
}
