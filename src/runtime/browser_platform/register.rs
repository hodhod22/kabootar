//! Browser Platform native API registration.

use super::{canvas_register, devtools, extensions, pwa, wasm_guest, webgl, webrtc};
use crate::runtime::os::OsHandle;
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn map_to_object(m: HashMap<String, String>) -> Value {
    Value::Object(m.into_iter().map(|(k, v)| (k, Value::String(v))).collect())
}

fn platform_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut o = HashMap::new();
    o.insert("platform".into(), Value::String("kabootar-browser-v2".into()));
    o.insert("version".into(), Value::String("2.58.0".into()));
    o.insert("wasm".into(), map_to_object(wasm_guest::info()));
    o.insert("webgl".into(), map_to_object(webgl::info()));
    o.insert("canvas".into(), map_to_object(crate::runtime::render::canvas2d::info()));
    o.insert("host_canvas".into(), map_to_object(super::canvas_host::info()));
    o.insert("webrtc".into(), map_to_object(webrtc::info()));
    o.insert("devtools".into(), map_to_object(devtools::info()));
    o.insert("extensions".into(), map_to_object(extensions::info()));
    o.insert("pwa".into(), map_to_object(pwa::info()));
    Ok(Value::Object(o))
}

// --- WASM ---

fn wasm_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(wasm_guest::info()))
}

fn wasm_load_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_str(args, 0, "wasm_load(path)")?;
    let name = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("module").to_string());
    let os = env.get("os").ok_or("wasm_load requires os handle")?;
    let Value::OsHandle(handle) = os else {
        return Err("wasm_load requires os handle".into());
    };
    let bytes = read_os_bytes(&handle, &path)?;
    let module = wasm_guest::load_wasm(&name, bytes)?;
    Ok(wasm_module_value(&module))
}

fn wasm_run_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let export = expect_str(args, 1, "wasm_run(id, export, args)")?;
    let i32_args = wasm_args_from_value(args.get(2))?;
    let out = wasm_guest::run_export(id, &export, &i32_args)?;
    Ok(Value::Number(out as i64))
}

fn wasm_args_from_value(v: Option<&Value>) -> Result<Vec<i32>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as i32),
                _ => Err("wasm args must be numbers".into()),
            })
            .collect(),
        Value::Number(n) => Ok(vec![*n as i32]),
        _ => Err("wasm_run third arg must be array of numbers".into()),
    }
}

fn wasm_module_value(m: &wasm_guest::WasmModule) -> Value {
    let mut o = HashMap::new();
    o.insert("id".into(), Value::Number(m.id as i64));
    o.insert("name".into(), Value::String(m.name.clone()));
    o.insert(
        "exports".into(),
        Value::Array(m.exports.iter().map(|e| Value::String(e.clone())).collect()),
    );
    o.insert("size".into(), Value::Number(m.bytes.len() as i64));
    Value::Object(o)
}

// --- WebGL ---

fn webgl_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(webgl::info()))
}

fn webgl_create_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = args.get(0).and_then(num).unwrap_or(800) as u32;
    let h = args.get(1).and_then(num).unwrap_or(600) as u32;
    super::webgl_register::create_gl_context(w, h, "webgl", "native", None)
}

fn webgl_shader_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let vert = expect_str(args, 0, "webgl_shader(vertex, fragment)")?;
    let frag = expect_str(args, 1, "webgl_shader(vertex, fragment)")?;
    let prog = webgl::compile_shader(&vert, &frag)?;
    Ok(Value::Number(prog.id as i64))
}

fn webgl_shader_from_files_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let vert = expect_str(args, 0, "webgl_shader_from_files(vert, frag)")?;
    let frag = expect_str(args, 1, "webgl_shader_from_files(vert, frag)")?;
    let prog = webgl::compile_shader_from_files(&vert, &frag)?;
    Ok(Value::Number(prog.id as i64))
}

fn webgl_create_texture_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tex = webgl::create_texture()?;
    Ok(Value::Number(tex.id as i64))
}

fn webgl_bind_texture_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_num(args, 0)? as u64;
    let tex = expect_num(args, 1)? as u64;
    Ok(Value::Bool(webgl::bind_texture(ctx, tex)?))
}

fn webgl_tex_image2d_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tex = expect_num(args, 0)? as u64;
    let w = expect_num(args, 1)? as u32;
    let h = expect_num(args, 2)? as u32;
    let pixels = match args.get(3) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as u8),
                _ => Err("pixels must be bytes".into()),
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => return Err("webgl_tex_image2d(tex, w, h, pixels)".into()),
    };
    Ok(Value::Bool(webgl::tex_image2d_rgba(tex, w, h, &pixels)?))
}

fn webgl_create_framebuffer_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(webgl::create_framebuffer()?.id as i64))
}

fn webgl_bind_framebuffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_num(args, 0)? as u64;
    let fb = match args.get(1) {
        None | Some(Value::Null) | Some(Value::Undefined) => None,
        Some(Value::Number(n)) if *n > 0 => Some(*n as u64),
        _ => return Err("webgl_bind_framebuffer(ctx, fb|null)".into()),
    };
    Ok(Value::Bool(webgl::bind_framebuffer(ctx, fb)?))
}

fn webgl_framebuffer_texture_2d_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let fb = expect_num(args, 0)? as u64;
    let tex = expect_num(args, 1)? as u64;
    Ok(Value::Bool(webgl::framebuffer_texture_2d(fb, tex)?))
}

fn webgl_use_program_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx_id = expect_num(args, 0)? as u64;
    let shader_id = expect_num(args, 1)? as u64;
    Ok(Value::Bool(webgl::use_program(ctx_id, shader_id)?))
}

fn webgl_clear_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let r = args.get(1).and_then(num).unwrap_or(0) as u8;
    let g = args.get(2).and_then(num).unwrap_or(0) as u8;
    let b = args.get(3).and_then(num).unwrap_or(0) as u8;
    let a = args.get(4).and_then(num).unwrap_or(255) as u8;
    Ok(Value::Bool(webgl::clear(id, r, g, b, a)?))
}

fn webgl_draw_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let count = args.get(1).and_then(num).unwrap_or(3) as u32;
    Ok(Value::Bool(webgl::draw_arrays(id, count)?))
}

fn webgl_create_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kind = args
        .get(0)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("array");
    let floats = float_array_from_value(args.get(1))?;
    let buf = webgl::create_buffer(kind, &floats)?;
    Ok(Value::Number(buf.id as i64))
}

fn webgl_create_index_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let indices = u16_array_from_value(args.get(0))?;
    let buf = webgl::create_index_buffer(&indices)?;
    Ok(Value::Number(buf.id as i64))
}

fn webgl_bind_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx_id = expect_num(args, 0)? as u64;
    let buffer_id = expect_num(args, 1)? as u64;
    Ok(Value::Bool(webgl::bind_buffer(ctx_id, buffer_id)?))
}

fn webgl_draw_elements_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let count = args.get(1).and_then(num).unwrap_or(3) as u32;
    let offset = args.get(2).and_then(num).unwrap_or(0) as u32;
    Ok(Value::Bool(webgl::draw_elements(id, count, offset)?))
}

fn float_array_from_value(v: Option<&Value>) -> Result<Vec<f32>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as f32),
                Value::Float(f) => Ok(*f as f32),
                _ => Err("webgl buffer expects numbers".into()),
            })
            .collect(),
        Value::Number(n) => Ok(vec![*n as f32]),
        Value::Float(f) => Ok(vec![*f as f32]),
        _ => Err("webgl buffer expects array of numbers".into()),
    }
}

fn u16_array_from_value(v: Option<&Value>) -> Result<Vec<u16>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as u16),
                _ => Err("index buffer expects integers".into()),
            })
            .collect(),
        Value::Number(n) => Ok(vec![*n as u16]),
        _ => Err("index buffer expects array".into()),
    }
}

// --- WebRTC ---

fn webrtc_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(webrtc::info()))
}

fn webrtc_create_peer_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let peer = webrtc::create_peer()?;
    Ok(Value::Number(peer.id as i64))
}

fn webrtc_create_offer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    Ok(Value::String(webrtc::create_offer(id)?))
}

fn webrtc_set_remote_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let sdp = expect_str(args, 1, "webrtc_set_remote(id, sdp)")?;
    Ok(Value::Bool(webrtc::set_remote_description(id, &sdp)?))
}

fn webrtc_gather_ice_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let cands = webrtc::gather_ice_candidates(id)?;
    Ok(Value::Array(
        cands
            .into_iter()
            .map(|c| {
                let mut m = HashMap::new();
                m.insert("candidate".into(), Value::String(c.candidate));
                m.insert("sdpMid".into(), Value::String(c.sdp_mid));
                Value::Object(m)
            })
            .collect(),
    ))
}

fn webrtc_add_track_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let kind = args
        .get(1)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("audio");
    Ok(Value::String(webrtc::add_track(id, kind)?))
}

fn webrtc_stats_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    Ok(map_to_object(webrtc::get_stats(id)))
}

fn webrtc_configure_ice_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let json = expect_str(args, 0, "webrtc_configure_ice(json)")?;
    let servers = webrtc::parse_ice_servers_json(&json);
    webrtc::configure_ice_servers(servers);
    Ok(Value::Bool(true))
}

fn webrtc_send_rtp_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let track = expect_str(args, 1, "webrtc_send_rtp(peer, track, data)")?;
    let data = expect_str(args, 2, "webrtc_send_rtp(peer, track, data)")?;
    let len = webrtc::send_rtp(id, &track, data.as_bytes())?;
    Ok(Value::Number(len as i64))
}

fn webrtc_recv_rtp_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = expect_num(args, 0)? as u64;
    let pkts = webrtc::recv_rtp(id)?;
    Ok(Value::Array(
        pkts.into_iter()
            .map(|p| {
                let mut m = HashMap::new();
                m.insert("track".into(), Value::String(p.track_id));
                m.insert("ssrc".into(), Value::Number(p.ssrc as i64));
                m.insert("seq".into(), Value::Number(p.sequence as i64));
                m.insert("ts".into(), Value::Number(p.timestamp as i64));
                m.insert(
                    "payload".into(),
                    Value::String(String::from_utf8_lossy(&p.payload).into_owned()),
                );
                Value::Object(m)
            })
            .collect(),
    ))
}

// --- DevTools ---

fn devtools_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(devtools::info()))
}

fn devtools_log_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let level = args
        .first()
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("log");
    let msg = expect_str(args, 1, "devtools_log(level, msg)")?;
    let source = args
        .get(2)
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("kabootar");
    devtools::console_log(level, &msg, source);
    Ok(Value::Null)
}

fn devtools_dump_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Array(
        devtools::console_dump()
            .into_iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("level".into(), Value::String(e.level));
                m.insert("message".into(), Value::String(e.message));
                m.insert("source".into(), Value::String(e.source));
                Value::Object(m)
            })
            .collect(),
    ))
}

fn devtools_inspect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = match args.first() {
        Some(Value::KabootarDom(n)) => n,
        _ => return Err("devtools_inspect(node)".into()),
    };
    Ok(map_to_object(devtools::inspect_node(node)))
}

fn devtools_breakpoint_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let file = expect_str(args, 0, "devtools_breakpoint(file, line)")?;
    let line = expect_num(args, 1)? as u32;
    Ok(Value::Bool(devtools::debugger_breakpoint_set(&file, line)))
}

fn devtools_source_map_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let generated = expect_str(args, 0, "devtools_source_map(generated, original)")?;
    let original = expect_str(args, 1, "devtools_source_map(generated, original)")?;
    devtools::source_map_register(&generated, &original);
    Ok(Value::Null)
}

fn devtools_dom_tree_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if let Some(Value::KabootarDom(n)) = args.first() {
        return Ok(devtools::dom_tree_value(n));
    }
    let doc = env.get("kbrowser").and_then(|v| match v {
        Value::KabootarBrowser(b) => b.active_document().ok(),
        _ => None,
    });
    let doc = doc.ok_or("devtools_dom_tree: no active document")?;
    Ok(devtools::dom_tree_value(&doc))
}

// --- Extensions ---

fn ext_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(extensions::info()))
}

fn ext_install_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let manifest = expect_str(args, 0, "ext_install(manifest_json)")?;
    let ext = extensions::parse_manifest(&manifest)?;
    let mut o = HashMap::new();
    o.insert("id".into(), Value::Number(ext.id as i64));
    o.insert("name".into(), Value::String(ext.name));
    o.insert("version".into(), Value::String(ext.version));
    o.insert("enabled".into(), Value::Bool(ext.enabled));
    Ok(Value::Object(o))
}

fn ext_list_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Array(
        extensions::list_extensions()
            .into_iter()
            .map(|e| {
                let mut m = HashMap::new();
                m.insert("id".into(), Value::Number(e.id as i64));
                m.insert("name".into(), Value::String(e.name));
                m.insert("version".into(), Value::String(e.version));
                m.insert("enabled".into(), Value::Bool(e.enabled));
                Value::Object(m)
            })
            .collect(),
    ))
}

// --- PWA ---

fn pwa_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(pwa::info()))
}

fn pwa_parse_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let json = expect_str(args, 0, "pwa_parse(json)")?;
    let m = pwa::parse_manifest(&json)?;
    let mut o = HashMap::new();
    o.insert("name".into(), Value::String(m.name));
    o.insert("short_name".into(), Value::String(m.short_name));
    o.insert("start_url".into(), Value::String(m.start_url));
    o.insert("display".into(), Value::String(m.display));
    o.insert("theme_color".into(), Value::String(m.theme_color));
    Ok(Value::Object(o))
}

fn pwa_install_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let json = expect_str(args, 0, "pwa_install(manifest_json)")?;
    let m = pwa::parse_manifest(&json)?;
    let os = env.get("os").ok_or("pwa_install requires os handle")?;
    let Value::OsHandle(handle) = os else {
        return Err("pwa_install requires os handle".into());
    };
    Ok(Value::String(pwa::install_to_os(&m, &handle)?))
}

fn pwa_register_worker_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let scope = expect_str(args, 0, "pwa_register_worker(scope, script)")?;
    let script = expect_str(args, 1, "pwa_register_worker(scope, script)")?;
    Ok(Value::Bool(pwa::register_worker(&scope, &script)?))
}

fn pwa_fetch_cached_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let url = expect_str(args, 0, "pwa_fetch_cached(url)")?;
    Ok(pwa::fetch_cached(&url)
        .map(Value::String)
        .unwrap_or(Value::Null))
}

fn read_os_bytes(os: &OsHandle, path: &str) -> Result<Vec<u8>, String> {
    let s = os.read(path)?;
    Ok(s.into_bytes())
}

fn expect_str(args: &[Value], i: usize, name: &str) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects string")),
    }
}

fn expect_num(args: &[Value], i: usize) -> Result<i64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n),
        _ => Err("expected number".into()),
    }
}

fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

pub fn browser_platform_globals(env: &mut Environment) {
    env.set("bp_info".into(), Value::NativeFunction(platform_info_native));
    env.set("wasm_info".into(), Value::NativeFunction(wasm_info_native));
    env.set("wasm_load".into(), Value::NativeFunction(wasm_load_native));
    env.set("wasm_run".into(), Value::NativeFunction(wasm_run_native));
    env.set("webgl_info".into(), Value::NativeFunction(webgl_info_native));
    env.set("webgl_create".into(), Value::NativeFunction(webgl_create_native));
    env.set("webgl_shader".into(), Value::NativeFunction(webgl_shader_native));
    env.set(
        "webgl_shader_from_files".into(),
        Value::NativeFunction(webgl_shader_from_files_native),
    );
    env.set("webgl_use_program".into(), Value::NativeFunction(webgl_use_program_native));
    env.set("webgl_clear".into(), Value::NativeFunction(webgl_clear_native));
    env.set("webgl_draw".into(), Value::NativeFunction(webgl_draw_native));
    env.set("webgl_create_buffer".into(), Value::NativeFunction(webgl_create_buffer_native));
    env.set(
        "webgl_create_index_buffer".into(),
        Value::NativeFunction(webgl_create_index_buffer_native),
    );
    env.set("webgl_bind_buffer".into(), Value::NativeFunction(webgl_bind_buffer_native));
    env.set("webgl_draw_elements".into(), Value::NativeFunction(webgl_draw_elements_native));
    env.set(
        "webgl_create_texture".into(),
        Value::NativeFunction(webgl_create_texture_native),
    );
    env.set("webgl_bind_texture".into(), Value::NativeFunction(webgl_bind_texture_native));
    env.set("webgl_tex_image2d".into(), Value::NativeFunction(webgl_tex_image2d_native));
    env.set(
        "webgl_create_framebuffer".into(),
        Value::NativeFunction(webgl_create_framebuffer_native),
    );
    env.set(
        "webgl_bind_framebuffer".into(),
        Value::NativeFunction(webgl_bind_framebuffer_native),
    );
    env.set(
        "webgl_framebuffer_texture_2d".into(),
        Value::NativeFunction(webgl_framebuffer_texture_2d_native),
    );
    env.set("webrtc_info".into(), Value::NativeFunction(webrtc_info_native));
    env.set("webrtc_create_peer".into(), Value::NativeFunction(webrtc_create_peer_native));
    env.set("webrtc_create_offer".into(), Value::NativeFunction(webrtc_create_offer_native));
    env.set("webrtc_set_remote".into(), Value::NativeFunction(webrtc_set_remote_native));
    env.set("webrtc_gather_ice".into(), Value::NativeFunction(webrtc_gather_ice_native));
    env.set("webrtc_add_track".into(), Value::NativeFunction(webrtc_add_track_native));
    env.set("webrtc_stats".into(), Value::NativeFunction(webrtc_stats_native));
    env.set(
        "webrtc_configure_ice".into(),
        Value::NativeFunction(webrtc_configure_ice_native),
    );
    env.set("webrtc_send_rtp".into(), Value::NativeFunction(webrtc_send_rtp_native));
    env.set("webrtc_recv_rtp".into(), Value::NativeFunction(webrtc_recv_rtp_native));
    env.set("devtools_info".into(), Value::NativeFunction(devtools_info_native));
    env.set("devtools_log".into(), Value::NativeFunction(devtools_log_native));
    env.set("devtools_dump".into(), Value::NativeFunction(devtools_dump_native));
    env.set("devtools_inspect".into(), Value::NativeFunction(devtools_inspect_native));
    env.set("devtools_breakpoint".into(), Value::NativeFunction(devtools_breakpoint_native));
    env.set("devtools_source_map".into(), Value::NativeFunction(devtools_source_map_native));
    env.set("devtools_dom_tree".into(), Value::NativeFunction(devtools_dom_tree_native));
    env.set("ext_info".into(), Value::NativeFunction(ext_info_native));
    env.set("ext_install".into(), Value::NativeFunction(ext_install_native));
    env.set("ext_list".into(), Value::NativeFunction(ext_list_native));
    env.set("pwa_info".into(), Value::NativeFunction(pwa_info_native));
    env.set("pwa_parse".into(), Value::NativeFunction(pwa_parse_native));
    env.set("pwa_install".into(), Value::NativeFunction(pwa_install_native));
    env.set("pwa_register_worker".into(), Value::NativeFunction(pwa_register_worker_native));
    env.set("pwa_fetch_cached".into(), Value::NativeFunction(pwa_fetch_cached_native));
    canvas_register::register_canvas(env);
}
