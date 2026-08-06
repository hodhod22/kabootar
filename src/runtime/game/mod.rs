//! Game runtime — frame loop, input, unified surface (v2.59).

mod frame;
pub mod gltf;
mod hot_reload;
pub mod image_png;
mod input;
mod surface;
mod xr_ffi;

use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::sync::Mutex;

pub use frame::{has_pending_frames, tick as tick_frame};
pub use input::{is_down, key_down, key_up, pointer_down, pointer_move, pointer_up};

pub fn info() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("api".into(), "kabootar-game".into());
    m.insert("version".into(), "0.1".into());
    m.insert(
        "features".into(),
        "rAF,input,surface,present,3d,gltf,png,atlas,hot,shader,batch,physics,audio,ecs,debug,assets,nav,net,editor,profiler,host".into(),
    );
    m
}

/// Run one frame: rAF callbacks, optional `present` is caller's responsibility.
pub fn shell_step(env: &mut Environment) -> Result<(), String> {
    if has_pending_frames() {
        tick_frame(env)?;
    }
    Ok(())
}

fn expect_surface(args: &[Value], i: usize) -> Result<&Value, String> {
    args.get(i).ok_or("expected game surface argument".into())
}

fn request_animation_frame_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let cb = args
        .first()
        .ok_or("requestAnimationFrame(callback)")?
        .clone();
    let id = frame::request_frame(cb);
    Ok(Value::Number(id as i64))
}

fn cancel_animation_frame_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as u64,
        _ => return Err("cancelAnimationFrame(id) expects positive id".into()),
    };
    frame::cancel_frame(id);
    Ok(Value::Null)
}

fn game_tick_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    tick_frame(env)
}

fn kb_on_frame_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    request_animation_frame_native(args, env)
}

fn game_run_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let max_frames = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u64),
            _ => None,
        })
        .unwrap_or(u64::MAX);
    let mut ran = 0u64;
    while ran < max_frames && has_pending_frames() {
        tick_frame(env)?;
        ran += 1;
    }
    Ok(Value::Number(ran as i64))
}

fn game_surface_create_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let w = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(800);
    let h = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(600);
    surface::create_surface(env, w, h)
}

fn game_surface_create_3d_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let w = args
        .first()
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(800);
    let h = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u32),
            _ => None,
        })
        .unwrap_or(600);
    surface::create_gl_surface(env, w, h)
}

fn game_present_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let surface = expect_surface(args, 0)?;
    surface::present_surface(env, surface)
}

fn input_key_down_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("input_key_down(key) expects string".into()),
    };
    key_down(key);
    Ok(Value::Null)
}

fn input_key_up_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("input_key_up(key) expects string".into()),
    };
    key_up(key);
    Ok(Value::Null)
}

fn input_pointer_move_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = f64_arg(args, 0)?;
    let y = f64_arg(args, 1)?;
    pointer_move(x, y);
    Ok(Value::Null)
}

fn input_pointer_down_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = f64_arg(args, 0)?;
    let y = f64_arg(args, 1)?;
    pointer_down(x, y);
    Ok(Value::Null)
}

fn input_pointer_up_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let x = f64_arg(args, 0)?;
    let y = f64_arg(args, 1)?;
    pointer_up(x, y);
    Ok(Value::Null)
}

fn input_poll_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(input::poll())
}

fn input_is_down_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("input_is_down(key) expects string".into()),
    };
    Ok(Value::Bool(is_down(key)))
}

fn game_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Object(
        info()
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect(),
    ))
}

fn f64_from(v: &Value, default: f64) -> f64 {
    match v {
        Value::Number(n) => *n as f64,
        Value::Float(f) => *f,
        _ => default,
    }
}

/// GP7b — GPU scene viewport descriptor (+ optional wgpu frame when `gpu` feature).
/// editor_scene_gpu_viewport({width,height,camX,camY,camZ,zoom,gizmos}) → gpu view object
fn editor_scene_gpu_viewport_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("editor_scene_gpu_viewport(descriptor)".into()),
    };
    let width = f64_from(desc.get("width").unwrap_or(&Value::Null), 640.0)
        .clamp(16.0, 4096.0) as u32;
    let height = f64_from(desc.get("height").unwrap_or(&Value::Null), 360.0)
        .clamp(16.0, 4096.0) as u32;
    let cam_x = f64_from(desc.get("camX").unwrap_or(&Value::Null), 0.0) as f32;
    let cam_y = f64_from(desc.get("camY").unwrap_or(&Value::Null), 0.0) as f32;
    let cam_z = f64_from(desc.get("camZ").unwrap_or(&Value::Null), 10.0) as f32;
    let zoom = f64_from(desc.get("zoom").unwrap_or(&Value::Null), 1.0).max(0.05) as f32;
    let gizmos = match desc.get("gizmos") {
        Some(Value::Array(items)) => items.as_slice(),
        _ => &[],
    };

    let view_proj = scene_view_proj(cam_x, cam_y, cam_z, zoom, width, height);
    let mut draws = Vec::new();
    let mut vertices: Vec<f32> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    for (gi, g) in gizmos.iter().enumerate() {
        let Value::Object(gm) = g else {
            continue;
        };
        let x = f64_from(gm.get("x").unwrap_or(&Value::Null), 0.0) as f32;
        let y = f64_from(gm.get("y").unwrap_or(&Value::Null), 0.0) as f32;
        let z = f64_from(gm.get("z").unwrap_or(&Value::Null), 0.0) as f32;
        let selected = matches!(gm.get("selected"), Some(Value::Bool(true)));
        let name = match gm.get("name") {
            Some(Value::String(s)) => s.clone(),
            _ => format!("gizmo_{gi}"),
        };
        let color = if selected {
            [0.2f32, 0.85, 0.35, 1.0]
        } else {
            [0.55, 0.6, 0.75, 1.0]
        };
        let mut draw = HashMap::new();
        draw.insert("name".into(), Value::String(name));
        draw.insert("x".into(), Value::Float(x as f64));
        draw.insert("y".into(), Value::Float(y as f64));
        draw.insert("z".into(), Value::Float(z as f64));
        draw.insert("selected".into(), Value::Bool(selected));
        draw.insert(
            "color".into(),
            Value::Array(color.iter().map(|c| Value::Float(*c as f64)).collect()),
        );
        draws.push(Value::Object(draw));

        // Unit diamond (4 verts) centered at gizmo position — solid pipeline xyz.
        let base = (vertices.len() / 3) as u16;
        let s = 0.35;
        vertices.extend_from_slice(&[x, y + s, z, x - s, y, z, x + s, y, z, x, y - s, z]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    if vertices.is_empty() {
        // Default ground tri so the viewport always has a drawable.
        vertices.extend_from_slice(&[0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0]);
        indices.extend_from_slice(&[0, 1, 2]);
    }

    let available = crate::runtime::render::gpu3d::gpu3d_available();
    let mut rendered = false;
    let mut pixel_count = 0i64;
    if available && !vertices.is_empty() {
        let frame = crate::runtime::render::gpu3d::Gpu3dFrame {
            width,
            height,
            clear_color: [0.12, 0.13, 0.16, 1.0],
            view_proj,
            model: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            draw_color: [0.55, 0.7, 0.95, 1.0],
            uv_transform: [1.0, 1.0, 0.0, 0.0],
            vertices: vertices.clone(),
            component_count: 3,
            vert_count: (vertices.len() / 3) as u32,
            indices: Some(indices.clone()),
            index_offset: 0,
            index_count: indices.len() as u32,
            depth_test: true,
            texture: None,
            instance_count: 1,
        };
        if let Ok(px) = crate::runtime::render::gpu3d::render_frame(&frame) {
            rendered = true;
            pixel_count = px.len() as i64;
        }
    }

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("gpu_viewport".into()));
    out.insert("mode".into(), Value::String("scene".into()));
    out.insert("available".into(), Value::Bool(available));
    out.insert(
        "backend".into(),
        Value::String(if available {
            crate::runtime::render::gpu3d::info_line().into()
        } else {
            "cpu_descriptor".into()
        }),
    );
    out.insert("width".into(), Value::Number(width as i64));
    out.insert("height".into(), Value::Number(height as i64));
    out.insert(
        "viewProj".into(),
        Value::Array(view_proj.iter().map(|f| Value::Float(*f as f64)).collect()),
    );
    out.insert("draws".into(), Value::Array(draws));
    out.insert("vertCount".into(), Value::Number((vertices.len() / 3) as i64));
    out.insert("indexCount".into(), Value::Number(indices.len() as i64));
    out.insert("rendered".into(), Value::Bool(rendered));
    out.insert("pixelBytes".into(), Value::Number(pixel_count));
    Ok(Value::Object(out))
}

fn scene_view_proj(cx: f32, cy: f32, cz: f32, zoom: f32, w: u32, h: u32) -> [f32; 16] {
    let aspect = (w as f32 / h.max(1) as f32).max(0.1);
    let fovy = (0.8 / zoom).clamp(0.2, 2.0);
    let f = 1.0 / (fovy * 0.5).tan();
    let near = 0.1f32;
    let far = 200.0f32;
    let persp = [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (near - far),
        -1.0,
        0.0,
        0.0,
        (2.0 * far * near) / (near - far),
        0.0,
    ];
    // Translate world by -camera (look toward origin along -Z from (cx,cy,cz)).
    let view = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -cx, -cy, -cz, 1.0,
    ];
    mat4_mul(persp, view)
}

fn mat4_mul(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut o = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            o[col * 4 + row] = a[row] * b[col * 4]
                + a[4 + row] * b[col * 4 + 1]
                + a[8 + row] * b[col * 4 + 2]
                + a[12 + row] * b[col * 4 + 3];
        }
    }
    o
}

fn f64_arg(args: &[Value], i: usize) -> Result<f64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n as f64),
        Some(Value::Float(f)) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

/// GP6g — depth shadow map pass via gpu3d (subset).
fn game_gpu_shadow_render_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("game_gpu_shadow_render(descriptor)".into()),
    };
    let map_size = f64_from(desc.get("mapSize").unwrap_or(&Value::Null), 512.0)
        .clamp(64.0, 2048.0) as u32;
    let soft = matches!(desc.get("soft"), Some(Value::Bool(true)));
    let bias = f64_from(desc.get("bias").unwrap_or(&Value::Null), 0.005) as f32;

    let mut vertices: Vec<f32> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    if let Some(Value::Array(pos)) = desc.get("positions") {
        for p in pos {
            if let Value::Object(pm) = p {
                let x = f64_from(pm.get("x").unwrap_or(&Value::Null), 0.0) as f32;
                let y = f64_from(pm.get("y").unwrap_or(&Value::Null), 0.0) as f32;
                let z = f64_from(pm.get("z").unwrap_or(&Value::Null), 0.0) as f32;
                vertices.extend_from_slice(&[x, y, z]);
            }
        }
    }
    if vertices.is_empty() {
        vertices.extend_from_slice(&[0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0]);
    }
    if let Some(Value::Array(idx)) = desc.get("indices") {
        for v in idx {
            if let Value::Number(n) = v {
                if *n >= 0 && *n <= u16::MAX as i64 {
                    indices.push(*n as u16);
                }
            }
        }
    }
    if indices.is_empty() {
        indices.extend_from_slice(&[0, 1, 2]);
    }

    let available = crate::runtime::render::gpu3d::gpu3d_available();
    let view_proj = scene_view_proj(0.0, 5.0, 10.0, 1.0, map_size, map_size);
    let mut rendered = false;
    let mut pixel_bytes = 0i64;
    if available {
        let frame = crate::runtime::render::gpu3d::Gpu3dFrame {
            width: map_size,
            height: map_size,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            view_proj,
            model: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            draw_color: [0.0, 0.0, 0.0, 1.0],
            uv_transform: [1.0, 1.0, 0.0, 0.0],
            vertices: vertices.clone(),
            component_count: 3,
            vert_count: (vertices.len() / 3) as u32,
            indices: Some(indices.clone()),
            index_offset: 0,
            index_count: indices.len() as u32,
            depth_test: true,
            texture: None,
            instance_count: 1,
        };
        if let Ok(px) = crate::runtime::render::gpu3d::render_frame(&frame) {
            rendered = true;
            pixel_bytes = px.len() as i64;
        }
    }

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("shadow-map-gpu".into()));
    out.insert("mapSize".into(), Value::Number(map_size as i64));
    out.insert("bias".into(), Value::Float(bias as f64));
    out.insert("soft".into(), Value::Bool(soft));
    out.insert("available".into(), Value::Bool(available));
    out.insert(
        "backend".into(),
        Value::String(if available {
            crate::runtime::render::gpu3d::info_line().into()
        } else {
            "cpu_descriptor".into()
        }),
    );
    out.insert("rendered".into(), Value::Bool(rendered));
    out.insert("pixelBytes".into(), Value::Number(pixel_bytes));
    out.insert("vertCount".into(), Value::Number((vertices.len() / 3) as i64));
    out.insert("indexCount".into(), Value::Number(indices.len() as i64));
    Ok(Value::Object(out))
}

/// GP6g — sample shadow visibility (0..1) for lit pipeline.
fn game_gpu_shadow_sample_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("game_gpu_shadow_sample(descriptor)".into()),
    };
    let u = f64_from(desc.get("u").unwrap_or(&Value::Null), 0.5).clamp(0.0, 1.0);
    let v = f64_from(desc.get("v").unwrap_or(&Value::Null), 0.5).clamp(0.0, 1.0);
    let py = match desc.get("point") {
        Some(Value::Object(pm)) => f64_from(pm.get("y").unwrap_or(&Value::Null), 0.0),
        _ => 0.0,
    };
    let soft = match desc.get("shadow") {
        Some(Value::Object(sm)) => matches!(sm.get("soft"), Some(Value::Bool(true))),
        _ => false,
    };
    let radius = match desc.get("shadow") {
        Some(Value::Object(sm)) => {
            f64_from(sm.get("radius").unwrap_or(&Value::Null), 1.5) as f32
        }
        _ => 1.5,
    };
    let bias = match desc.get("shadow") {
        Some(Value::Object(sm)) => {
            f64_from(sm.get("bias").unwrap_or(&Value::Null), 0.005) as f32
        }
        _ => 0.005,
    };

    let du = (u - 0.5) as f32;
    let dv = (v - 0.5) as f32;
    let dist = (du * du + dv * dv).sqrt();
    let mut factor = if py > 0.55 - bias as f64 {
        1.0
    } else if dist > 0.35 {
        0.25
    } else {
        1.0
    };
    if soft {
        let t = (dist / (0.35 + radius * 0.08)).clamp(0.0, 1.0);
        factor = factor + (1.0 - factor) * t as f64;
    }

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("shadow-sample".into()));
    out.insert("factor".into(), Value::Float(factor));
    out.insert("u".into(), Value::Float(u));
    out.insert("v".into(), Value::Float(v));
    Ok(Value::Object(out))
}

static NET_HTTP_HUB: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn net_http_hub_reset_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if let Ok(mut q) = NET_HTTP_HUB.lock() {
        q.clear();
    }
    Ok(Value::Null)
}

fn net_http_hub_push_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let body = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => crate::value::format_value(other),
        None => String::new(),
    };
    if !body.is_empty() {
        if let Ok(mut q) = NET_HTTP_HUB.lock() {
            q.push(body);
        }
    }
    Ok(Value::Null)
}

fn net_http_hub_poll_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let max = match args.first() {
        Some(Value::Number(n)) if *n > 0 => *n as usize,
        _ => 1,
    };
    let mut out = Vec::new();
    if let Ok(mut q) = NET_HTTP_HUB.lock() {
        while out.len() < max && !q.is_empty() {
            out.push(Value::String(q.remove(0)));
        }
    }
    Ok(Value::Array(out))
}

fn xr_stub_enabled() -> bool {
    std::env::var("KABOOTAR_XR_STUB")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// OpenXR/WebXR-like runtime swapchain state (host stub until real headset FFI).
struct XrRuntimeState {
    next_id: i64,
    frame_index: i64,
    predicted_display_time_ns: i64,
    waiting: bool,
    acquired: HashMap<i64, i64>, // swapchain_id -> image_index
    swapchains: HashMap<i64, XrSwapchain>,
}

struct XrSwapchain {
    id: i64,
    eye: String,
    width: u32,
    height: u32,
    image_count: i64,
    next_image: i64,
}

static XR_RUNTIME: Mutex<Option<XrRuntimeState>> = Mutex::new(None);

fn xr_runtime_mut() -> Result<std::sync::MutexGuard<'static, Option<XrRuntimeState>>, String> {
    XR_RUNTIME
        .lock()
        .map_err(|_| "xr runtime lock poisoned".into())
}

fn ensure_xr_runtime(guard: &mut Option<XrRuntimeState>) -> &mut XrRuntimeState {
    if guard.is_none() {
        *guard = Some(XrRuntimeState {
            next_id: 1,
            frame_index: 0,
            predicted_display_time_ns: 0,
            waiting: false,
            acquired: HashMap::new(),
            swapchains: HashMap::new(),
        });
    }
    guard.as_mut().unwrap()
}

fn xr_reset_runtime() {
    if let Ok(mut g) = XR_RUNTIME.lock() {
        *g = None;
    }
}

/// GP6n — XR host capability probe (OpenXR/WebXR FFI + stub).
fn xr_host_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let stub = xr_stub_enabled();
    let ffi = xr_ffi::probe();
    let openxr = ffi.openxr_loader || ffi.openxr_runtime || (stub && !cfg!(target_arch = "wasm32"));
    let webxr = ffi.webxr || (stub && cfg!(target_arch = "wasm32"));
    let available = stub || ffi.openxr_loader || ffi.openxr_runtime || ffi.webxr || ffi.bound;
    let backend = if stub {
        "xr-stub"
    } else if !ffi.backend.is_empty() && ffi.backend != "none" {
        ffi.backend.as_str()
    } else if cfg!(target_arch = "wasm32") {
        "webxr-descriptor"
    } else {
        "openxr-descriptor"
    };
    let mut out = HashMap::new();
    out.insert("available".into(), Value::Bool(available));
    out.insert("backend".into(), Value::String(backend.into()));
    out.insert("openxr".into(), Value::Bool(openxr));
    out.insert("webxr".into(), Value::Bool(webxr));
    out.insert("runtime".into(), Value::String("kab-xr-runtime".into()));
    out.insert("ffi".into(), xr_ffi::status_value());
    Ok(Value::Object(out))
}

fn xr_ffi_probe_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = xr_ffi::probe();
    Ok(xr_ffi::status_value())
}

fn xr_bind_headset_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let force_stub = xr_stub_enabled()
        || matches!(args.first(), Some(Value::Bool(true)))
        || matches!(
            args.first().and_then(|v| match v {
                Value::Object(m) => m.get("stub"),
                _ => None,
            }),
            Some(Value::Bool(true))
        );
    match xr_ffi::bind_headset(force_stub) {
        Ok(st) => {
            let mut out = HashMap::new();
            out.insert("ok".into(), Value::Bool(true));
            out.insert("bound".into(), Value::Bool(st.bound));
            out.insert("backend".into(), Value::String(st.backend));
            out.insert("detail".into(), Value::String(st.detail));
            out.insert("openxrLoader".into(), Value::Bool(st.openxr_loader));
            out.insert("openxrRuntime".into(), Value::Bool(st.openxr_runtime));
            out.insert("webxr".into(), Value::Bool(st.webxr));
            out.insert("hmdConnected".into(), Value::Bool(st.hmd_connected));
            out.insert("vendor".into(), Value::String(st.vendor));
            out.insert("formFactor".into(), Value::String(st.form_factor));
            Ok(Value::Object(out))
        }
        Err(e) => {
            let mut out = HashMap::new();
            out.insert("ok".into(), Value::Bool(false));
            out.insert("bound".into(), Value::Bool(false));
            out.insert("error".into(), Value::String(e));
            out.insert("ffi".into(), xr_ffi::status_value());
            Ok(Value::Object(out))
        }
    }
}

fn xr_hmd_driver_present_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let composition = args
        .first()
        .ok_or_else(|| "xr_hmd_driver_present(composition)".to_string())?;
    xr_ffi::present_to_hmd(composition)
}

fn xr_compositor_open_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    xr_ffi::compositor_open()
}

fn xr_compositor_submit_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let composition = args
        .first()
        .ok_or_else(|| "xr_compositor_submit(composition)".to_string())?;
    xr_ffi::compositor_submit(composition)
}

fn xr_compositor_poll_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    xr_ffi::compositor_poll()
}

fn xr_request_session_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mode = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => "immersive-vr",
    };
    xr_ffi::request_session(mode)
}

fn xr_compositor_process_spawn_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    xr_ffi::compositor_process_spawn()
}

fn xr_compositor_process_tick_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    xr_ffi::compositor_process_tick()
}

fn xr_compositor_process_stop_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    xr_ffi::compositor_process_stop()
}

fn xr_compositor_process_status_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    Ok(xr_ffi::compositor_process_status())
}

fn xr_loader_end_frame_status_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    Ok(xr_ffi::loader_end_frame_status())
}

fn xr_raf_bind_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    xr_ffi::raf_bind()
}

fn xr_request_animation_frame_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let cb = args
        .first()
        .ok_or_else(|| "xr_request_animation_frame(callback)".to_string())?
        .clone();
    xr_ffi::request_animation_frame(cb)
}

fn xr_cancel_animation_frame_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => return Err("xr_cancel_animation_frame(id)".into()),
    };
    xr_ffi::cancel_animation_frame(id)
}

fn xr_raf_tick_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    xr_ffi::raf_tick(env)
}

fn xr_raf_status_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(xr_ffi::raf_status())
}

fn xr_create_swapchain_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("xr_create_swapchain(descriptor)".into()),
    };
    let eye = match desc.get("eye") {
        Some(Value::String(s)) => s.clone(),
        _ => "left".into(),
    };
    let width = f64_from(desc.get("width").unwrap_or(&Value::Null), 1280.0)
        .clamp(320.0, 4096.0) as u32;
    let height = f64_from(desc.get("height").unwrap_or(&Value::Null), 720.0)
        .clamp(240.0, 4096.0) as u32;
    let image_count = f64_from(desc.get("imageCount").unwrap_or(&Value::Null), 3.0)
        .clamp(2.0, 4.0) as i64;

    let mut guard = xr_runtime_mut()?;
    let rt = ensure_xr_runtime(&mut guard);
    let id = rt.next_id;
    rt.next_id += 1;
    rt.swapchains.insert(
        id,
        XrSwapchain {
            id,
            eye: eye.clone(),
            width,
            height,
            image_count,
            next_image: 0,
        },
    );

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_swapchain".into()));
    out.insert("id".into(), Value::Number(id));
    out.insert("eye".into(), Value::String(eye));
    out.insert("width".into(), Value::Number(width as i64));
    out.insert("height".into(), Value::Number(height as i64));
    out.insert("imageCount".into(), Value::Number(image_count));
    out.insert("format".into(), Value::String("rgba8".into()));
    Ok(Value::Object(out))
}

fn xr_wait_frame_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut guard = xr_runtime_mut()?;
    let rt = ensure_xr_runtime(&mut guard);
    rt.waiting = true;
    rt.frame_index += 1;
    // 90 Hz predicted display time (OpenXR-style ns).
    rt.predicted_display_time_ns = rt.frame_index * 11_111_111;
    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_frame_state".into()));
    out.insert("shouldRender".into(), Value::Bool(true));
    out.insert("frameIndex".into(), Value::Number(rt.frame_index));
    out.insert(
        "predictedDisplayTime".into(),
        Value::Number(rt.predicted_display_time_ns),
    );
    out.insert("periodNs".into(), Value::Number(11_111_111));
    Ok(Value::Object(out))
}

fn xr_acquire_swapchain_image_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n,
        Some(Value::Object(m)) => match m.get("id") {
            Some(Value::Number(n)) => *n,
            _ => return Err("xr_acquire_swapchain_image(id|swapchain)".into()),
        },
        _ => return Err("xr_acquire_swapchain_image(id|swapchain)".into()),
    };
    let mut guard = xr_runtime_mut()?;
    let rt = ensure_xr_runtime(&mut guard);
    if !rt.waiting {
        return Err("xr_acquire_swapchain_image: call xr_wait_frame first".into());
    }
    let sc = rt
        .swapchains
        .get_mut(&id)
        .ok_or_else(|| format!("xr_acquire_swapchain_image: unknown swapchain {id}"))?;
    let image = sc.next_image % sc.image_count;
    sc.next_image += 1;
    rt.acquired.insert(id, image);
    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_swapchain_image".into()));
    out.insert("swapchainId".into(), Value::Number(id));
    out.insert("imageIndex".into(), Value::Number(image));
    out.insert("eye".into(), Value::String(sc.eye.clone()));
    out.insert("width".into(), Value::Number(sc.width as i64));
    out.insert("height".into(), Value::Number(sc.height as i64));
    Ok(Value::Object(out))
}

fn xr_release_swapchain_image_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n,
        Some(Value::Object(m)) => match m.get("swapchainId").or_else(|| m.get("id")) {
            Some(Value::Number(n)) => *n,
            _ => return Err("xr_release_swapchain_image(id|image)".into()),
        },
        _ => return Err("xr_release_swapchain_image(id|image)".into()),
    };
    let mut guard = xr_runtime_mut()?;
    let rt = ensure_xr_runtime(&mut guard);
    let image = rt
        .acquired
        .remove(&id)
        .ok_or_else(|| format!("xr_release_swapchain_image: swapchain {id} not acquired"))?;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("swapchainId".into(), Value::Number(id));
    out.insert("imageIndex".into(), Value::Number(image));
    Ok(Value::Object(out))
}

fn xr_end_frame_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let layers = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Object(_)) => vec![args[0].clone()],
        None => Vec::new(),
        _ => return Err("xr_end_frame(layers?)".into()),
    };
    let mut guard = xr_runtime_mut()?;
    let rt = ensure_xr_runtime(&mut guard);
    if !rt.waiting {
        return Err("xr_end_frame: call xr_wait_frame first".into());
    }
    if !rt.acquired.is_empty() {
        return Err("xr_end_frame: release all acquired images first".into());
    }
    let frame_index = rt.frame_index;
    let display_time = rt.predicted_display_time_ns;
    rt.waiting = false;
    drop(guard);

    let composed = compose_projection_layers(&layers)?;

    let loader = xr_ffi::loader_end_frame(frame_index, layers.len() as i64, &composed)?;

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_end_frame".into()));
    out.insert("submitted".into(), Value::Bool(true));
    out.insert("frameIndex".into(), Value::Number(frame_index));
    out.insert("displayTime".into(), Value::Number(display_time));
    out.insert("layerCount".into(), Value::Number(layers.len() as i64));
    out.insert("composition".into(), composed);
    out.insert("loaderEndFrame".into(), loader);
    Ok(Value::Object(out))
}

/// OpenXR-style projection layer composition (stereo views → HMD submit descriptor).
fn compose_projection_layers(layers: &[Value]) -> Result<Value, String> {
    let mut views_out = Vec::new();
    let mut layer_kinds = Vec::new();
    let mut total_w = 0u32;
    let mut total_h = 0u32;
    let stub = xr_stub_enabled() || xr_ffi::status().bound;
    let gpu = crate::runtime::render::gpu3d::gpu3d_available();

    for layer in layers {
        let Value::Object(lm) = layer else {
            continue;
        };
        let kind = match lm.get("type").or_else(|| lm.get("kind")) {
            Some(Value::String(s)) => s.clone(),
            _ => "projection".into(),
        };
        layer_kinds.push(Value::String(kind.clone()));

        if kind == "projection" || kind == "COMPOSITION_LAYER_PROJECTION" {
            let views = match lm.get("views") {
                Some(Value::Array(v)) => v.as_slice(),
                _ => &[],
            };
            for view in views {
                let Value::Object(vm) = view else {
                    continue;
                };
                let eye = match vm.get("eye") {
                    Some(Value::String(s)) => s.clone(),
                    _ => "left".into(),
                };
                let width = f64_from(vm.get("width").unwrap_or(&Value::Null), 640.0)
                    .clamp(64.0, 4096.0) as u32;
                let height = f64_from(vm.get("height").unwrap_or(&Value::Null), 720.0)
                    .clamp(64.0, 4096.0) as u32;
                let cx = if eye == "right" { 0.032 } else { -0.032 };
                let (rendered, pixel_bytes) =
                    render_eye_swapchain(&eye, cx as f32, width, height, stub, gpu);
                total_w += width;
                total_h = total_h.max(height);

                let pose = match vm.get("pose") {
                    Some(v) => v.clone(),
                    None => {
                        let mut p = HashMap::new();
                        p.insert("x".into(), Value::Float(cx));
                        p.insert("y".into(), Value::Float(1.6));
                        p.insert("z".into(), Value::Float(0.0));
                        p.insert("qx".into(), Value::Float(0.0));
                        p.insert("qy".into(), Value::Float(0.0));
                        p.insert("qz".into(), Value::Float(0.0));
                        p.insert("qw".into(), Value::Float(1.0));
                        Value::Object(p)
                    }
                };
                let fov = match vm.get("fov") {
                    Some(v) => v.clone(),
                    None => {
                        let mut f = HashMap::new();
                        f.insert("angleLeft".into(), Value::Float(-0.785));
                        f.insert("angleRight".into(), Value::Float(0.785));
                        f.insert("angleUp".into(), Value::Float(0.785));
                        f.insert("angleDown".into(), Value::Float(-0.785));
                        Value::Object(f)
                    }
                };
                let mut vo = HashMap::new();
                vo.insert("eye".into(), Value::String(eye));
                vo.insert("width".into(), Value::Number(width as i64));
                vo.insert("height".into(), Value::Number(height as i64));
                vo.insert("pose".into(), pose);
                vo.insert("fov".into(), fov);
                vo.insert("rendered".into(), Value::Bool(rendered));
                vo.insert("pixelBytes".into(), Value::Number(pixel_bytes));
                if let Some(sub) = vm.get("subImage") {
                    vo.insert("subImage".into(), sub.clone());
                }
                views_out.push(Value::Object(vo));
            }
        } else {
            // Quad / cylinder layers: record only (composition deferred).
            let mut vo = HashMap::new();
            vo.insert("kind".into(), Value::String(kind));
            vo.insert("composed".into(), Value::Bool(false));
            views_out.push(Value::Object(vo));
        }
    }

    // Side-by-side composite size (left|right).
    let sbs_w = if total_w > 0 { total_w } else { 1280 };
    let sbs_h = if total_h > 0 { total_h } else { 720 };

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_hmd_composition".into()));
    out.insert("layerTypes".into(), Value::Array(layer_kinds));
    out.insert("views".into(), Value::Array(views_out.clone()));
    out.insert("viewCount".into(), Value::Number(views_out.len() as i64));
    out.insert("sideBySideWidth".into(), Value::Number(sbs_w as i64));
    out.insert("sideBySideHeight".into(), Value::Number(sbs_h as i64));
    out.insert(
        "submittedToHmd".into(),
        Value::Bool(stub || xr_ffi::status().openxr_runtime || xr_ffi::status().webxr),
    );
    out.insert(
        "backend".into(),
        Value::String(if xr_ffi::status().bound {
            xr_ffi::status().backend
        } else if stub {
            "xr-stub-compose".into()
        } else {
            "descriptor-compose".into()
        }),
    );
    Ok(Value::Object(out))
}

fn xr_compose_hmd_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let layers = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Object(_)) => vec![args[0].clone()],
        _ => return Err("xr_compose_hmd(layers)".into()),
    };
    compose_projection_layers(&layers)
}

fn render_eye_swapchain(
    eye: &str,
    cx: f32,
    width: u32,
    height: u32,
    stub: bool,
    gpu: bool,
) -> (bool, i64) {
    let mut rendered = false;
    let mut pixel_bytes = 0i64;
    if gpu || stub {
        let view_proj = scene_view_proj(cx, 1.6, 10.0, 1.0, width, height);
        let vertices: Vec<f32> = vec![0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0];
        let indices: Vec<u16> = vec![0, 1, 2];
        if gpu {
            let frame = crate::runtime::render::gpu3d::Gpu3dFrame {
                width,
                height,
                clear_color: if eye == "left" {
                    [0.15, 0.18, 0.28, 1.0]
                } else {
                    [0.18, 0.15, 0.28, 1.0]
                },
                view_proj,
                model: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                draw_color: [0.55, 0.7, 0.95, 1.0],
                uv_transform: [1.0, 1.0, 0.0, 0.0],
                vertices,
                component_count: 3,
                vert_count: 3,
                indices: Some(indices),
                index_offset: 0,
                index_count: 3,
                depth_test: true,
                texture: None,
                instance_count: 1,
            };
            if let Ok(px) = crate::runtime::render::gpu3d::render_frame(&frame) {
                rendered = true;
                pixel_bytes = px.len() as i64;
            }
        } else {
            rendered = true;
            pixel_bytes = (width as i64) * (height as i64) * 4;
        }
    }
    (rendered, pixel_bytes)
}

/// GP6n — XR present (stereo swapchain + optional GPU probe).
fn xr_host_present_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let desc = match args.first() {
        Some(Value::Object(m)) => m,
        _ => return Err("xr_host_present(descriptor)".into()),
    };
    let width = f64_from(desc.get("width").unwrap_or(&Value::Null), 1280.0)
        .clamp(320.0, 4096.0) as u32;
    let height = f64_from(desc.get("height").unwrap_or(&Value::Null), 720.0)
        .clamp(240.0, 4096.0) as u32;
    let mode = match desc.get("mode") {
        Some(Value::String(s)) => s.clone(),
        _ => "vr".into(),
    };
    let stub = xr_stub_enabled();
    let gpu = crate::runtime::render::gpu3d::gpu3d_available();
    let eye_offsets: [(&str, f32); 2] = [("left", -0.032), ("right", 0.032)];

    let mut swapchains = Vec::new();
    let mut total_pixels = 0i64;
    for (eye, cx) in eye_offsets {
        let (rendered, pixel_bytes) = render_eye_swapchain(eye, cx, width, height, stub, gpu);
        total_pixels += pixel_bytes;
        let mut chain = HashMap::new();
        chain.insert("eye".into(), Value::String(eye.into()));
        chain.insert("width".into(), Value::Number(width as i64));
        chain.insert("height".into(), Value::Number(height as i64));
        chain.insert("format".into(), Value::String("rgba8".into()));
        chain.insert("rendered".into(), Value::Bool(rendered));
        chain.insert("pixelBytes".into(), Value::Number(pixel_bytes));
        swapchains.push(Value::Object(chain));
    }

    let presented = stub || total_pixels > 0;

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_present_host".into()));
    out.insert("presented".into(), Value::Bool(presented));
    out.insert("mode".into(), Value::String(mode));
    out.insert("width".into(), Value::Number(width as i64));
    out.insert("height".into(), Value::Number(height as i64));
    out.insert("eyeCount".into(), Value::Number(2));
    out.insert("swapchains".into(), Value::Array(swapchains));
    out.insert("pixelBytes".into(), Value::Number(total_pixels));
    out.insert(
        "backend".into(),
        Value::String(if stub {
            "xr-stub-swapchain".into()
        } else if gpu {
            format!("{}-swapchain", crate::runtime::render::gpu3d::info_line())
        } else {
            "descriptor".into()
        }),
    );
    Ok(Value::Object(out))
}

#[cfg(not(target_arch = "wasm32"))]
fn net_http_session_serve_once_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let port = match args.first() {
        Some(Value::Number(n)) if *n > 0 && *n < 65536 => *n as u16,
        _ => return Err("net_http_session_serve_once(port)".into()),
    };

    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("net_http_session_serve_once bind {port}: {e}"))?;
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("net_http_session_serve_once accept: {e}"))?;

    let mut buffer = [0u8; 8192];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("net_http_session_serve_once read: {e}"))?;
    let raw = String::from_utf8_lossy(&buffer[..n]);
    let req = crate::runtime::http::parse_http_request(&raw)?;

    let res = if req.path.contains("/kab/net/push") && req.method.eq_ignore_ascii_case("POST") {
        net_http_hub_push_native(
            &[Value::String(req.body.clone())],
            &mut crate::value::Environment::new(),
        )?;
        crate::runtime::http::HttpResponse::new(200, "ok")
    } else if req.path.contains("/kab/net/pull") && req.method.eq_ignore_ascii_case("GET") {
        let polled = net_http_hub_poll_native(&[Value::Number(1)], &mut crate::value::Environment::new())?;
        if let Value::Array(items) = polled {
            if let Some(Value::String(body)) = items.first() {
                crate::runtime::http::HttpResponse::new(200, body.clone())
            } else {
                crate::runtime::http::HttpResponse::new(204, "")
            }
        } else {
            crate::runtime::http::HttpResponse::new(204, "")
        }
    } else {
        crate::runtime::http::HttpResponse::not_found()
    };

    stream
        .write_all(res.to_http_string().as_bytes())
        .map_err(|e| format!("net_http_session_serve_once write: {e}"))?;
    Ok(Value::Number(res.status))
}

#[cfg(target_arch = "wasm32")]
fn net_http_session_serve_once_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    Err("net_http_session_serve_once unavailable on wasm32".into())
}

pub fn game_globals(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("requestAnimationFrame", request_animation_frame_native),
        ("cancelAnimationFrame", cancel_animation_frame_native),
        ("kb_on_frame", kb_on_frame_native),
        ("game_tick", game_tick_native),
        ("game_run", game_run_native),
        ("game_surface_create", game_surface_create_native),
        ("game_surface_create_3d", game_surface_create_3d_native),
        ("game_present", game_present_native),
        ("input_key_down", input_key_down_native),
        ("input_key_up", input_key_up_native),
        ("input_pointer_move", input_pointer_move_native),
        ("input_pointer_down", input_pointer_down_native),
        ("input_pointer_up", input_pointer_up_native),
        ("input_poll", input_poll_native),
        ("input_is_down", input_is_down_native),
        ("game_info", game_info_native),
        ("editor_scene_gpu_viewport", editor_scene_gpu_viewport_native),
        ("game_gpu_shadow_render", game_gpu_shadow_render_native),
        ("game_gpu_shadow_sample", game_gpu_shadow_sample_native),
        ("net_http_hub_reset", net_http_hub_reset_native),
        ("net_http_hub_push", net_http_hub_push_native),
        ("net_http_hub_poll", net_http_hub_poll_native),
        ("net_http_session_serve_once", net_http_session_serve_once_native),
        ("xr_host_info", xr_host_info_native),
        ("xr_host_present", xr_host_present_native),
        ("xr_ffi_probe", xr_ffi_probe_native),
        ("xr_bind_headset", xr_bind_headset_native),
        ("xr_hmd_driver_present", xr_hmd_driver_present_native),
        ("xr_compositor_open", xr_compositor_open_native),
        ("xr_compositor_submit", xr_compositor_submit_native),
        ("xr_compositor_poll", xr_compositor_poll_native),
        ("xr_request_session", xr_request_session_native),
        (
            "xr_compositor_process_spawn",
            xr_compositor_process_spawn_native,
        ),
        (
            "xr_compositor_process_tick",
            xr_compositor_process_tick_native,
        ),
        (
            "xr_compositor_process_stop",
            xr_compositor_process_stop_native,
        ),
        (
            "xr_compositor_process_status",
            xr_compositor_process_status_native,
        ),
        (
            "xr_loader_end_frame_status",
            xr_loader_end_frame_status_native,
        ),
        ("xr_raf_bind", xr_raf_bind_native),
        (
            "xr_request_animation_frame",
            xr_request_animation_frame_native,
        ),
        (
            "xr_cancel_animation_frame",
            xr_cancel_animation_frame_native,
        ),
        ("xr_raf_tick", xr_raf_tick_native),
        ("xr_raf_status", xr_raf_status_native),
        ("xr_create_swapchain", xr_create_swapchain_native),
        ("xr_wait_frame", xr_wait_frame_native),
        ("xr_acquire_swapchain_image", xr_acquire_swapchain_image_native),
        ("xr_release_swapchain_image", xr_release_swapchain_image_native),
        ("xr_end_frame", xr_end_frame_native),
        ("xr_compose_hmd", xr_compose_hmd_native),
        ("gltf_load_json", gltf::gltf_load_json_native),
        ("image_decode_png", image_png::image_decode_png_native),
        ("asset_watch", hot_reload::asset_watch_native),
        ("asset_poll", hot_reload::asset_poll_native),
        ("host_read_bytes", host_read_bytes_native),
    ];
    for (name, f) in fns {
        env.set((*name).into(), Value::NativeFunction(*f));
    }
}

fn host_read_bytes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("host_read_bytes(path) expects string".into()),
    };
    let bytes = std::fs::read(path).map_err(|e| format!("host_read_bytes({path}): {e}"))?;
    Ok(Value::Array(
        bytes.into_iter().map(|b| Value::Number(b as i64)).collect(),
    ))
}

pub fn reset_all() {
    frame::reset_for_tests();
    input::reset_for_tests();
    hot_reload::reset_for_tests();
    if let Ok(mut q) = NET_HTTP_HUB.lock() {
        q.clear();
    }
    xr_reset_runtime();
    xr_ffi::reset_for_tests();
}
