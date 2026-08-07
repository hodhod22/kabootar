//! Layer 1 host canvas — real browser `<canvas>` on WASM, native fallback elsewhere.

use crate::runtime::render::canvas2d;
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_HOST_CANVAS: AtomicU64 = AtomicU64::new(1);
static NEXT_HOST_CTX: AtomicU64 = AtomicU64::new(1);
static NEXT_HOST_GL_CTX: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct HostCanvasRecord {
    width: u32,
    height: u32,
    /// Native engine mirror (compositor + non-WASM fallback).
    native_id: u64,
}

struct HostCtxRecord {
    canvas_id: u64,
    native_id: u64,
}

static HOST_CANVASES: OnceLock<Mutex<HashMap<u64, HostCanvasRecord>>> = OnceLock::new();
static HOST_CTXS: OnceLock<Mutex<HashMap<u64, HostCtxRecord>>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
thread_local! {
    static WASM_CANVASES: std::cell::RefCell<HashMap<u64, web_sys::HtmlCanvasElement>> =
        std::cell::RefCell::new(HashMap::new());
    static WASM_CTXS: std::cell::RefCell<HashMap<u64, web_sys::CanvasRenderingContext2d>> =
        std::cell::RefCell::new(HashMap::new());
}

fn host_canvases() -> &'static Mutex<HashMap<u64, HostCanvasRecord>> {
    HOST_CANVASES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn host_ctxs() -> &'static Mutex<HashMap<u64, HostCtxRecord>> {
    HOST_CTXS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn info() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("api".into(), "host-canvas".into());
    #[cfg(target_arch = "wasm32")]
    m.insert("backend".into(), "web_sys".into());
    #[cfg(not(target_arch = "wasm32"))]
    m.insert("backend".into(), "native-fallback".into());
    m.insert(
        "methods".into(),
        "fillRect,strokeRect,translate,scale,rotate,drawImage,measureText,…".into(),
    );
    m
}

pub fn create_element(width: u32, height: u32) -> Result<Value, String> {
    let w = width.clamp(1, 4096);
    let h = height.clamp(1, 4096);
    let id = NEXT_HOST_CANVAS.fetch_add(1, Ordering::Relaxed);
    let native_id = canvas2d::create(w, h)?;

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(canvas) = wasm_create_canvas(w, h) {
            WASM_CANVASES.with(|m| m.borrow_mut().insert(id, canvas));
        }
    }

    host_canvases()
        .lock()
        .map_err(|_| "host canvas lock poisoned".to_string())?
        .insert(
            id,
            HostCanvasRecord {
                width: w,
                height: h,
                native_id,
            },
        );

    Ok(host_canvas_object(id, w, h))
}

fn host_canvas_object(id: u64, width: u32, height: u32) -> Value {
    let mut o = HashMap::new();
    o.insert("__kab_host_canvas".into(), Value::Bool(true));
    o.insert("host_id".into(), Value::Number(id as i64));
    o.insert("tag".into(), Value::String("CANVAS".into()));
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("layer".into(), Value::String("host".into()));
    o.insert(
        "getContext".into(),
        Value::NativeFunction(host_get_context_native),
    );
    Value::from_object(o)
}

#[cfg(target_arch = "wasm32")]
fn wasm_create_canvas(width: u32, height: u32) -> Option<web_sys::HtmlCanvasElement> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let element = document.create_element("canvas").ok()?;
    let canvas: web_sys::HtmlCanvasElement = element.dyn_into().ok()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Some(canvas)
}

pub fn element_get_context(
    element: &Value,
    kind: &str,
) -> Result<Value, String> {
    if super::webgl_register::is_webgl_kind(kind) {
        return element_get_webgl_context(element, kind);
    }
    if kind != "2d" {
        return Err(format!("host canvas: unsupported getContext(\"{kind}\")"));
    }
    let Value::Object(map) = element else {
        return Err("host getContext expects canvas element object".into());
    };
    let host_id = match map.get("host_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid host canvas element".into()),
    };
    let record = host_canvases()
        .lock()
        .map_err(|_| "host canvas lock poisoned".to_string())?
        .get(&host_id)
        .cloned()
        .ok_or("unknown host canvas")?;

    let ctx_id = NEXT_HOST_CTX.fetch_add(1, Ordering::Relaxed);
    host_ctxs()
        .lock()
        .map_err(|_| "host canvas lock poisoned".to_string())?
        .insert(
            ctx_id,
            HostCtxRecord {
                canvas_id: host_id,
                native_id: record.native_id,
            },
        );

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(ctx) = wasm_get_context_2d(host_id) {
            WASM_CTXS.with(|m| m.borrow_mut().insert(ctx_id, ctx));
        }
    }

    Ok(host_ctx_object(ctx_id, record.width, record.height, record.native_id))
}

fn element_get_webgl_context(element: &Value, kind: &str) -> Result<Value, String> {
    let Value::Object(map) = element else {
        return Err("host getContext expects canvas element object".into());
    };
    let host_id = match map.get("host_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid host canvas element".into()),
    };
    let record = host_canvases()
        .lock()
        .map_err(|_| "host canvas lock poisoned".to_string())?
        .get(&host_id)
        .cloned()
        .ok_or("unknown host canvas")?;

    let host_gl_ctx_id = NEXT_HOST_GL_CTX.fetch_add(1, Ordering::Relaxed);
    let _ = host_id;
    super::webgl_register::create_gl_context(
        record.width,
        record.height,
        kind,
        "host",
        Some(host_gl_ctx_id),
    )
}

fn host_get_context_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let element = args
        .first()
        .ok_or("canvas.getContext(kind) expects canvas element")?;
    let kind = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => "2d",
    };
    element_get_context(element, kind)
}

fn host_ctx_object(ctx_id: u64, width: u32, height: u32, native_id: u64) -> Value {
    let mut o = HashMap::new();
    o.insert("__kab_ctx".into(), Value::Bool(true));
    o.insert("__kab_host_ctx".into(), Value::Bool(true));
    o.insert("host_ctx_id".into(), Value::Number(ctx_id as i64));
    o.insert("id".into(), Value::Number(native_id as i64));
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("kind".into(), Value::String("2d".into()));
    o.insert("layer".into(), Value::String("host".into()));
    attach_host_ctx_methods(&mut o);
    Value::from_object(o)
}

fn attach_host_ctx_methods(o: &mut HashMap<String, Value>) {
    o.insert("fillRect".into(), Value::NativeFunction(host_fill_rect_native));
    o.insert("strokeRect".into(), Value::NativeFunction(host_stroke_rect_native));
    o.insert("clearRect".into(), Value::NativeFunction(host_clear_rect_native));
    o.insert("fillText".into(), Value::NativeFunction(host_fill_text_native));
    o.insert("beginPath".into(), Value::NativeFunction(host_begin_path_native));
    o.insert("moveTo".into(), Value::NativeFunction(host_move_to_native));
    o.insert("lineTo".into(), Value::NativeFunction(host_line_to_native));
    o.insert("arc".into(), Value::NativeFunction(host_arc_native));
    o.insert("fill".into(), Value::NativeFunction(host_fill_native));
    o.insert("stroke".into(), Value::NativeFunction(host_stroke_native));
    o.insert("save".into(), Value::NativeFunction(host_save_native));
    o.insert("restore".into(), Value::NativeFunction(host_restore_native));
    o.insert("translate".into(), Value::NativeFunction(host_translate_native));
    o.insert("scale".into(), Value::NativeFunction(host_scale_native));
    o.insert("rotate".into(), Value::NativeFunction(host_rotate_native));
    o.insert("closePath".into(), Value::NativeFunction(host_close_path_native));
    o.insert("drawImage".into(), Value::NativeFunction(host_draw_image_native));
    o.insert("measureText".into(), Value::NativeFunction(host_measure_text_native));
    o.insert("getImageData".into(), Value::NativeFunction(host_get_image_data_native));
    o.insert("putImageData".into(), Value::NativeFunction(host_put_image_data_native));
    o.insert("setTransform".into(), Value::NativeFunction(host_set_transform_native));
    o.insert("rect".into(), Value::NativeFunction(host_rect_native));
    o.insert("toDataURL".into(), Value::NativeFunction(host_to_data_url_native));
}

fn host_ctx_id_from_receiver(args: &[Value]) -> Result<(u64, u64), String> {
    let receiver = args.first().ok_or("missing canvas context receiver")?;
    let Value::Object(map) = receiver else {
        return Err("expected host canvas context object".into());
    };
    let host_ctx_id = match map.get("host_ctx_id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid host context".into()),
    };
    let native_id = match map.get("id") {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("invalid host context native id".into()),
    };
    Ok((host_ctx_id, native_id))
}

fn f64_arg(args: &[Value], i: usize) -> Result<f64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n as f64),
        Some(Value::Float(f)) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

fn str_arg(args: &[Value], i: usize) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err("expected string".into()),
    }
}

fn host_fill_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let x = f64_arg(args, 1)?;
    let y = f64_arg(args, 2)?;
    let w = f64_arg(args, 3)?;
    let h = f64_arg(args, 4)?;
    #[cfg(target_arch = "wasm32")]
    host_wasm_fill_rect(host_ctx_id, x, y, w, h)?;
    canvas2d::fill_rect(native_id, x, y, w, h)?;
    Ok(Value::Null)
}

fn host_stroke_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let x = f64_arg(args, 1)?;
    let y = f64_arg(args, 2)?;
    let w = f64_arg(args, 3)?;
    let h = f64_arg(args, 4)?;
    #[cfg(target_arch = "wasm32")]
    host_wasm_stroke_rect(host_ctx_id, x, y, w, h)?;
    canvas2d::stroke_rect(native_id, x, y, w, h)?;
    Ok(Value::Null)
}

fn host_clear_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::clear_rect(native_id, f64_arg(args, 1)?, f64_arg(args, 2)?, f64_arg(args, 3)?, f64_arg(args, 4)?)?;
    Ok(Value::Null)
}

fn host_fill_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::fill_text(native_id, &str_arg(args, 1)?, f64_arg(args, 2)?, f64_arg(args, 3)?)?;
    Ok(Value::Null)
}

fn host_begin_path_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::begin_path(native_id)?;
    Ok(Value::Null)
}

fn host_move_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::move_to(native_id, f64_arg(args, 1)?, f64_arg(args, 2)?)?;
    Ok(Value::Null)
}

fn host_line_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::line_to(native_id, f64_arg(args, 1)?, f64_arg(args, 2)?)?;
    Ok(Value::Null)
}

fn host_arc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::arc(
        native_id,
        f64_arg(args, 1)?,
        f64_arg(args, 2)?,
        f64_arg(args, 3)?,
        f64_arg(args, 4)?,
        f64_arg(args, 5)?,
        matches!(args.get(6), Some(Value::Bool(true))),
    )?;
    Ok(Value::Null)
}

fn host_fill_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::fill(native_id)?;
    Ok(Value::Null)
}

fn host_stroke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::stroke(native_id)?;
    Ok(Value::Null)
}

fn host_save_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::save(native_id)?;
    Ok(Value::Null)
}

fn host_restore_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::restore(native_id)?;
    Ok(Value::Null)
}

fn host_translate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let x = f64_arg(args, 1)?;
    let y = f64_arg(args, 2)?;
    #[cfg(target_arch = "wasm32")]
    host_wasm_translate(host_ctx_id, x, y)?;
    canvas2d::translate(native_id, x, y)?;
    Ok(Value::Null)
}

fn host_scale_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let sx = f64_arg(args, 1)?;
    let sy = f64_arg(args, 2)?;
    #[cfg(target_arch = "wasm32")]
    host_wasm_scale(host_ctx_id, sx, sy)?;
    canvas2d::scale(native_id, sx, sy)?;
    Ok(Value::Null)
}

fn host_rotate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let angle = f64_arg(args, 1)?;
    #[cfg(target_arch = "wasm32")]
    host_wasm_rotate(host_ctx_id, angle)?;
    canvas2d::rotate(native_id, angle)?;
    Ok(Value::Null)
}

fn host_close_path_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::close_path(native_id)?;
    Ok(Value::Null)
}

fn canvas_native_id_from_value(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64), Value::Object(map) => match map.get("id") {
            Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
            _ => Err("canvas source missing id".into()),
        },
        _ => Err("drawImage source expects canvas context".into()),
    }
}

fn host_draw_image_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, dst_id) = host_ctx_id_from_receiver(args)?;
    let src = args.get(1).ok_or("drawImage(source, x, y, w, h)")?;
    let src_id = canvas_native_id_from_value(src)?;
    canvas2d::draw_image(
        dst_id,
        src_id,
        f64_arg(args, 2)?,
        f64_arg(args, 3)?,
        f64_arg(args, 4)?,
        f64_arg(args, 5)?,
    )?;
    Ok(Value::Null)
}

fn host_measure_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host_ctx_id, native_id) = host_ctx_id_from_receiver(args)?;
    let (w, h) = canvas2d::measure_text_size(native_id, &str_arg(args, 1)?)?;
    let mut o = HashMap::new();
    o.insert("width".into(), Value::Float(w as f64));
    o.insert("height".into(), Value::Float(h as f64));
    Ok(Value::from_object(o))
}

fn host_get_image_data_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host, native_id) = host_ctx_id_from_receiver(args)?;
    let x = f64_arg(args, 1)? as i32;
    let y = f64_arg(args, 2)? as i32;
    let w = f64_arg(args, 3)? as i32;
    let h = f64_arg(args, 4)? as i32;
    let data = canvas2d::get_image_data(native_id, x, y, w, h)?;
    let mut o = HashMap::new();
    o.insert("width".into(), Value::Number(w as i64));
    o.insert("height".into(), Value::Number(h as i64));
    o.insert(
        "data".into(), Value::from_array(data.into_iter().map(|b| Value::Number(b as i64)).collect()),
    );
    Ok(Value::from_object(o))
}

fn host_put_image_data_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host, native_id) = host_ctx_id_from_receiver(args)?;
    let Value::Object(img) = args.get(1).ok_or("putImageData expects ImageData")? else {
        return Err("putImageData expects ImageData object".into());
    };
    let w = match img.get("width") {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("ImageData.width required".into()),
    };
    let h = match img.get("height") {
        Some(Value::Number(n)) => *n as i32,
        _ => return Err("ImageData.height required".into()),
    };
    let data = match img.get("data") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok::<u8, String>(*n as u8),
                _ => Err("data must be numbers".into()),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("ImageData.data required".into()),
    };
    let dx = f64_arg(args, 2)? as i32;
    let dy = f64_arg(args, 3)? as i32;
    canvas2d::put_image_data(native_id, &data, dx, dy, w, h)?;
    Ok(Value::Null)
}

fn host_set_transform_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::set_transform(
        native_id,
        f64_arg(args, 1)?,
        f64_arg(args, 2)?,
        f64_arg(args, 3)?,
        f64_arg(args, 4)?,
        f64_arg(args, 5)?,
        f64_arg(args, 6)?,
    )?;
    Ok(Value::Null)
}

fn host_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host, native_id) = host_ctx_id_from_receiver(args)?;
    canvas2d::rect_path(
        native_id,
        f64_arg(args, 1)?,
        f64_arg(args, 2)?,
        f64_arg(args, 3)?,
        f64_arg(args, 4)?,
    )?;
    Ok(Value::Null)
}

fn host_to_data_url_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (_host, native_id) = host_ctx_id_from_receiver(args)?;
    let mime = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => "image/png",
    };
    Ok(Value::String(canvas2d::to_data_url(native_id, mime)?))
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_translate(host_ctx_id: u64, x: f64, y: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.translate(x, y);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_scale(host_ctx_id: u64, sx: f64, sy: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.scale(sx, sy);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_rotate(host_ctx_id: u64, angle: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.rotate(angle);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn wasm_get_context_2d(host_id: u64) -> Option<web_sys::CanvasRenderingContext2d> {
    WASM_CANVASES.with(|m| {
        m.borrow()
            .get(&host_id)
            .and_then(|c| c.get_context("2d").ok().flatten())
            .and_then(|ctx| ctx.dyn_into().ok())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_fill_rect(host_ctx_id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.fill_rect(x, y, w, h);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_stroke_rect(host_ctx_id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.stroke_rect(x, y, w, h);
        }
        Ok(())
    })
}

pub fn sync_fill_style(host_ctx_id: u64, color: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    host_wasm_set_fill_style(host_ctx_id, color)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (host_ctx_id, color);
    Ok(())
}

pub fn sync_stroke_style(host_ctx_id: u64, color: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    host_wasm_set_stroke_style(host_ctx_id, color)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (host_ctx_id, color);
    Ok(())
}

pub fn sync_global_alpha(host_ctx_id: u64, alpha: f64) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    host_wasm_set_global_alpha(host_ctx_id, alpha)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (host_ctx_id, alpha);
    Ok(())
}

pub fn sync_line_width(host_ctx_id: u64, width: f64) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    host_wasm_set_line_width(host_ctx_id, width)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (host_ctx_id, width);
    Ok(())
}

pub fn sync_font(host_ctx_id: u64, spec: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    host_wasm_set_font(host_ctx_id, spec)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (host_ctx_id, spec);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_set_fill_style(host_ctx_id: u64, color: &str) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.set_fill_style(color);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_set_stroke_style(host_ctx_id: u64, color: &str) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.set_stroke_style(color);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_set_global_alpha(host_ctx_id: u64, alpha: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.set_global_alpha(alpha);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_set_line_width(host_ctx_id: u64, width: f64) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.set_line_width(width);
        }
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn host_wasm_set_font(host_ctx_id: u64, spec: &str) -> Result<(), String> {
    WASM_CTXS.with(|m| {
        if let Some(ctx) = m.borrow().get(&host_ctx_id) {
            ctx.set_font(spec);
        }
        Ok(())
    })
}

pub fn try_create_element(tag: &str, width: u32, height: u32) -> Option<Value> {
    if tag.eq_ignore_ascii_case("canvas") {
        create_element(width, height).ok()
    } else {
        None
    }
}
