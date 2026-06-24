//! Background HTTP accept loop + main-thread dispatch (`serve_async_ready`).

use crate::http_dispatch;
use crate::runtime::http::{parse_http_request, HttpResponse};
use crate::runtime::stdlib::deno::SERVE_HANDLER_KEY;
use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};

static NEXT_SERVE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct ServeJob {
    raw: String,
    reply: Sender<HttpResponse>,
}

struct ServeRuntime {
    port: u16,
    jobs: Receiver<ServeJob>,
    handle: Option<JoinHandle<()>>,
}

static SERVE_RUNTIMES: OnceLock<Mutex<HashMap<u64, ServeRuntime>>> = OnceLock::new();

fn runtimes() -> &'static Mutex<HashMap<u64, ServeRuntime>> {
    SERVE_RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    static ACTIVE_SERVE_ID: RefCell<Option<u64>> = RefCell::new(None);
}

/// Bind port, spawn accept loop, return serve id. Handler lives in caller `env` (`SERVE_HANDLER_KEY`).
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_serve(port: u16, bind: &str) -> Result<u64, String> {
    if port == 0 {
        return Err("serve_async: invalid port".into());
    }
    let bind = bind.to_string();
    let listener = TcpListener::bind(format!("{bind}:{port}"))
        .map_err(|e| format!("serve_async bind failed: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("serve_async set_nonblocking: {e}"))?;

    let (job_tx, job_rx) = mpsc::channel::<ServeJob>();
    let id = NEXT_SERVE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let handle = thread::Builder::new()
        .name(format!("kabootar-serve-{port}"))
        .spawn(move || {
            use std::time::Duration;
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buffer = [0u8; 8192];
                        let Ok(n) = stream.read(&mut buffer) else {
                            continue;
                        };
                        if n == 0 {
                            continue;
                        }
                        let raw = String::from_utf8_lossy(&buffer[..n]).into_owned();
                        let (reply_tx, reply_rx) = mpsc::channel();
                        if job_tx.send(ServeJob { raw, reply: reply_tx }).is_err() {
                            break;
                        }
                        if let Ok(res) = reply_rx.recv() {
                            let _ = stream.write_all(res.to_http_string().as_bytes());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|e| format!("serve_async spawn failed: {e}"))?;

    runtimes().lock().unwrap().insert(
        id,
        ServeRuntime {
            port,
            jobs: job_rx,
            handle: Some(handle),
        },
    );
    ACTIVE_SERVE_ID.with(|s| *s.borrow_mut() = Some(id));
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_serve(_port: u16, _bind: &str) -> Result<u64, String> {
    Err("serve_async is not available on wasm32".into())
}

pub fn active_serve_id() -> Option<u64> {
    ACTIVE_SERVE_ID.with(|s| *s.borrow())
}

pub fn serve_port(id: u64) -> Option<u16> {
    runtimes().lock().unwrap().get(&id).map(|s| s.port)
}

/// Dispatch pending accepted connections using `env`'s serve handler.
pub fn poll_serve(env: &mut Environment) -> Result<u32, String> {
    let id = active_serve_id().ok_or("serve_async_poll: no active server")?;
    let handler = env
        .get(SERVE_HANDLER_KEY)
        .filter(|h| !matches!(h, Value::Null | Value::Undefined))
        .ok_or("serve_async_poll: no serve handler registered")?;
    let _ = handler;

    let mut runtimes = runtimes().lock().unwrap();
    let runtime = runtimes
        .get_mut(&id)
        .ok_or_else(|| format!("invalid serve id {id}"))?;

    let mut handled = 0u32;
    loop {
        match runtime.jobs.try_recv() {
            Ok(job) => {
                let response = match parse_http_request(&job.raw) {
                    Ok(req) => http_dispatch::dispatch(env, &req)?,
                    Err(e) => HttpResponse::new(400, e),
                };
                let _ = job.reply.send(response);
                handled += 1;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => break,
        }
    }
    Ok(handled)
}

pub fn stop_serve(id: u64) -> Result<(), String> {
    let mut map = runtimes().lock().unwrap();
    let runtime = map
        .remove(&id)
        .ok_or_else(|| format!("invalid serve id {id}"))?;
    drop(runtime.jobs);
    if let Some(handle) = runtime.handle {
        drop(handle);
    }
    ACTIVE_SERVE_ID.with(|s| {
        if *s.borrow() == Some(id) {
            *s.borrow_mut() = None;
        }
    });
    Ok(())
}
