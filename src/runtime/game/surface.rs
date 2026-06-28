//! Unified game surface — KDOM canvas + compositor present in all environments.

use crate::runtime::browser_platform::canvas_register::native_canvas_context;
use crate::runtime::browser_platform::webgl;
use crate::runtime::browser_platform::webgl_register;
use crate::runtime::frame_buffer;
use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::platform::RuntimeLayer;
use crate::runtime::render::canvas2d;
use crate::value::{Environment, Value};
use std::collections::HashMap;

pub fn active_layer(env: &Environment) -> RuntimeLayer {
    let Some(Value::Object(map)) = env.get("__platform") else {
        return RuntimeLayer::Hybrid;
    };
    map.get("active")
        .and_then(|v| match v {
            Value::String(s) => RuntimeLayer::from_str(s),
            _ => None,
        })
        .unwrap_or(RuntimeLayer::Hybrid)
}

pub fn create_surface(env: &Environment, width: u32, height: u32) -> Result<Value, String> {
    let w = width.clamp(1, 4096);
    let h = height.clamp(1, 4096);
    let _layer = active_layer(env);

    #[cfg(target_arch = "wasm32")]
    if matches!(layer, RuntimeLayer::Host) {
        return create_host_surface(w, h);
    }

    create_kabootar_surface(w, h)
}

pub fn create_gl_surface(env: &Environment, width: u32, height: u32) -> Result<Value, String> {
    let w = width.clamp(1, 4096);
    let h = height.clamp(1, 4096);
    let _layer = active_layer(env);

    #[cfg(target_arch = "wasm32")]
    if matches!(layer, RuntimeLayer::Host) {
        return create_host_gl_surface(w, h);
    }

    create_kabootar_gl_surface(w, h)
}

fn create_kabootar_gl_surface(width: u32, height: u32) -> Result<Value, String> {
    let gl = webgl_register::create_gl_context(width, height, "webgl2", "kabootar", None)?;
    let mut o = HashMap::new();
    o.insert("__kab_game_surface".into(), Value::Bool(true));
    o.insert("gl".into(), gl);
    o.insert("mode".into(), Value::String("3d".into()));
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("layer".into(), Value::String("kabootar".into()));
    o.insert("present".into(), Value::NativeFunction(game_present_native));
    Ok(Value::Object(o))
}

#[cfg(target_arch = "wasm32")]
fn create_host_gl_surface(width: u32, height: u32) -> Result<Value, String> {
    let canvas = canvas_host::create_element(width, height)?;
    let gl = canvas_host::element_get_context(&canvas, "webgl2")?;
    let mut o = HashMap::new();
    o.insert("__kab_game_surface".into(), Value::Bool(true));
    o.insert("gl".into(), gl);
    o.insert("canvas".into(), canvas);
    o.insert("mode".into(), Value::String("3d".into()));
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("layer".into(), Value::String("host".into()));
    o.insert("present".into(), Value::NativeFunction(game_present_native));
    Ok(Value::Object(o))
}

#[cfg(not(target_arch = "wasm32"))]
fn create_host_gl_surface(width: u32, height: u32) -> Result<Value, String> {
    create_kabootar_gl_surface(width, height)
}

fn create_kabootar_surface(width: u32, height: u32) -> Result<Value, String> {
    let mut root = DomNode::element("div");
    root.set_attr("style", "margin:0;padding:0;background:#000000");
    let mut canvas = DomNode::element("canvas");
    canvas.set_attr("width", &width.to_string());
    canvas.set_attr("height", &height.to_string());
    let canvas_id = canvas.id;
    root.append(canvas);

    let ctx_id = canvas2d::bind_dom(canvas_id, width, height)?;
    let ctx = native_canvas_context(ctx_id)?;

    let mut o = HashMap::new();
    o.insert("__kab_game_surface".into(), Value::Bool(true));
    o.insert("ctx".into(), ctx);
    o.insert("root".into(), Value::KabootarDom(root));
    o.insert("canvas_id".into(), Value::Number(canvas_id as i64));
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("layer".into(), Value::String("kabootar".into()));
    o.insert("present".into(), Value::NativeFunction(game_present_native));
    Ok(Value::Object(o))
}

#[cfg(target_arch = "wasm32")]
fn create_host_surface(width: u32, height: u32) -> Result<Value, String> {
    let canvas = canvas_host::create_element(width, height)?;
    let ctx = canvas_host::element_get_context(&canvas, "2d")?;
    let mut o = HashMap::new();
    o.insert("__kab_game_surface".into(), Value::Bool(true));
    o.insert("ctx".into(), ctx);
    o.insert("canvas".into(), canvas);
    o.insert("width".into(), Value::Number(width as i64));
    o.insert("height".into(), Value::Number(height as i64));
    o.insert("layer".into(), Value::String("host".into()));
    o.insert("present".into(), Value::NativeFunction(game_present_native));
    Ok(Value::Object(o))
}

#[cfg(not(target_arch = "wasm32"))]
fn create_host_surface(width: u32, height: u32) -> Result<Value, String> {
    // Native host preference still uses KDOM compositor (visible in shell + kb_paint).
    create_kabootar_surface(width, height)
}

pub fn present_surface(env: &mut Environment, surface: &Value) -> Result<Value, String> {
    let Value::Object(map) = surface else {
        return Err("game_present expects surface object".into());
    };
    if map.get("mode").and_then(|v| match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }) == Some("3d")
        || map.contains_key("gl")
    {
        return present_gl_surface(map);
    }
    let layer = map
        .get("layer")
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("kabootar");

    let w = map
        .get("width")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            _ => None,
        })
        .unwrap_or(800.0);
    let h = map
        .get("height")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n as f64),
            _ => None,
        })
        .unwrap_or(600.0);

    if layer == "host" {
        if let Some(Value::Object(ctx)) = map.get("ctx") {
            if let Some(Value::Number(id)) = ctx.get("id") {
                if let Ok(px) = canvas2d::to_rgba_bytes(*id as u64) {
                    frame_buffer::publish_pixels(w, h, px);
                    return Ok(Value::Bool(true));
                }
            }
        }
        return Err("host game surface present failed".into());
    }

    let Value::KabootarDom(root) = map
        .get("root")
        .ok_or("game surface missing root node")?
    else {
        return Err("game surface root must be KabootarDom".into());
    };

    let browser = env
        .get("kbrowser")
        .ok_or("kbrowser not available")?;
    let Value::KabootarBrowser(browser) = browser else {
        return Err("kbrowser handle expected".into());
    };
    let os = env.get("os").and_then(|v| match v {
        Value::OsHandle(h) => Some(h),
        _ => None,
    });
    browser.set_document(root.clone())?;
    browser.set_viewport(w, h)?;
    let _frame = browser.paint(os.as_ref())?;
    Ok(Value::Bool(true))
}

fn gl_id_from_surface(map: &HashMap<String, Value>) -> Result<u64, String> {
    let Value::Object(gl) = map.get("gl").ok_or("3d surface missing gl")? else {
        return Err("3d surface gl must be object".into());
    };
    match gl.get("id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("3d surface gl missing id".into()),
    }
}

fn present_gl_surface(map: &HashMap<String, Value>) -> Result<Value, String> {
    let gl_id = gl_id_from_surface(map)?;
    if frame_buffer::last_frame_pixels().is_none() {
        let _ = webgl::clear(gl_id, 0, 0, 0, 255);
    }
    if let Some((w, h, px)) = frame_buffer::last_frame_pixels() {
        frame_buffer::publish_pixels(w as f64, h as f64, px);
        return Ok(Value::Bool(true));
    }
    Err("3d game surface present failed".into())
}

fn game_present_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let surface = args
        .first()
        .ok_or("surface.present() expects surface receiver")?;
    present_surface(env, surface)
}
