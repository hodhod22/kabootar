//! Game runtime — frame loop, input, unified surface (v2.59).

mod frame;
pub mod gltf;
mod hot_reload;
pub mod image_png;
mod input;
mod surface;

use crate::value::{Environment, Value};
use std::collections::HashMap;

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

fn f64_arg(args: &[Value], i: usize) -> Result<f64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n as f64),
        Some(Value::Float(f)) => Ok(*f),
        _ => Err("expected number".into()),
    }
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
}
