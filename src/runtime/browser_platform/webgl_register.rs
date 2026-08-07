//! WebGL context objects with JS-syntax methods (`gl.drawElements`, …).

use super::webgl;
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn gl_id_from_receiver(args: &[Value]) -> Result<u64, String> {
    let receiver = args.first().ok_or("missing WebGL context receiver")?;
    let Value::Object(map) = receiver else {
        return Err("expected WebGL context object".into());
    };
    match map.get("id") {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("WebGL context missing id".into()),
    }
}

fn f64_arg(args: &[Value], i: usize) -> Result<f64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n as f64),
        Some(Value::Float(f)) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

fn num_arg(args: &[Value], i: usize) -> Result<i64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::Float(f)) => Ok(*f as i64),
        _ => Err("expected number".into()),
    }
}

fn str_arg(args: &[Value], i: usize) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err("expected string".into()),
    }
}

fn float_array_from_value(v: Option<&Value>) -> Result<Vec<f32>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    if crate::runtime::shared_memory::is_float32_array(v) {
        return crate::runtime::shared_memory::float32_array_to_f32_vec(v);
    }
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as f32),
                Value::Float(f) => Ok(*f as f32),
                _ => Err("buffer data expects numbers".into()),
            })
            .collect(),
        Value::Number(n) => Ok(vec![*n as f32]),
        Value::Float(f) => Ok(vec![*f as f32]),
        _ => Err("buffer data expects array of numbers or Float32Array".into()),
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

fn gl_clear_color_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let r = num_arg(args, 1)? as u8;
    let g = num_arg(args, 2)? as u8;
    let b = num_arg(args, 3)? as u8;
    let a = args.get(4).and_then(|v| match v {
        Value::Number(n) => Some(*n as u8),
        Value::Float(f) => Some(*f as u8),
        _ => None,
    }).unwrap_or(255);
    webgl::clear(id, r, g, b, a)?;
    Ok(Value::Null)
}

fn gl_clear_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    gl_clear_color_native(args, env)
}

fn gl_draw_arrays_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let count = if args.len() >= 4 {
        num_arg(args, 3)? as u32
    } else {
        num_arg(args, 1)? as u32
    };
    Ok(Value::Bool(webgl::draw_arrays(id, count)?))
}

fn gl_draw_arrays_instanced_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    // gl.drawArraysInstanced(count, instances) or WebGL-like (mode, first, count, primcount)
    let (count, instances) = if args.len() >= 5 {
        (num_arg(args, 3)? as u32, num_arg(args, 4)? as u32)
    } else {
        (num_arg(args, 1)? as u32, num_arg(args, 2).unwrap_or(1) as u32)
    };
    Ok(Value::Bool(webgl::draw_arrays_instanced(id, count, instances)?))
}

fn gl_draw_elements_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let count = if args.len() >= 5 {
        num_arg(args, 2)? as u32
    } else {
        num_arg(args, 1)? as u32
    };
    let offset = if args.len() >= 5 {
        num_arg(args, 4)? as u32
    } else {
        num_arg(args, 2).unwrap_or(0) as u32
    };
    Ok(Value::Bool(webgl::draw_elements(id, count, offset)?))
}

fn gl_draw_elements_instanced_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    // gl.drawElementsInstanced(count, offset, instances)
    let count = num_arg(args, 1)? as u32;
    let offset = num_arg(args, 2).unwrap_or(0) as u32;
    let instances = num_arg(args, 3).unwrap_or(1) as u32;
    Ok(Value::Bool(webgl::draw_elements_instanced(
        id, count, offset, instances,
    )?))
}

fn gl_bind_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let buffer_id = if args.len() >= 3 {
        num_arg(args, 2)? as u64
    } else {
        num_arg(args, 1)? as u64
    };
    Ok(Value::Bool(webgl::bind_buffer(id, buffer_id)?))
}

fn gl_create_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kind = if args.len() >= 3 {
        str_arg(args, 1)?
    } else {
        "array".into()
    };
    let data_idx = if args.len() >= 3 { 2 } else { 1 };
    let floats = float_array_from_value(args.get(data_idx))?;
    let buf = webgl::create_buffer(&kind, &floats)?;
    Ok(Value::Number(buf.id as i64))
}

fn gl_create_index_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let indices = u16_array_from_value(args.get(1))?;
    let buf = webgl::create_index_buffer(&indices)?;
    Ok(Value::Number(buf.id as i64))
}

fn gl_compile_shader_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let vert = str_arg(args, 1)?;
    let frag = str_arg(args, 2)?;
    let prog = webgl::compile_shader(&vert, &frag)?;
    Ok(Value::Number(prog.id as i64))
}

fn gl_use_program_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let shader_id = num_arg(args, 1)? as u64;
    Ok(Value::Bool(webgl::use_program(id, shader_id)?))
}

fn gl_uniform4f_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let loc = num_arg(args, 1)? as i32;
    let r = f64_arg(args, 2)? as f32;
    let g = f64_arg(args, 3)? as f32;
    let b = f64_arg(args, 4)? as f32;
    let a = f64_arg(args, 5)? as f32;
    Ok(Value::Bool(webgl::uniform4f(id, loc, r, g, b, a)?))
}

fn mat4_from_value(v: &Value) -> Result<[f32; 16], String> {
    let Value::Array(items) = v else {
        return Err("uniformMatrix4fv expects 16-element array".into());
    };
    if items.len() != 16 {
        return Err("uniformMatrix4fv expects 16 floats".into());
    }
    let mut m = [0.0f32; 16];
    for (i, item) in items.iter().enumerate() {
        m[i] = match item {
            Value::Number(n) => *n as f32,
            Value::Float(f) => *f as f32,
            _ => return Err("matrix elements must be numbers".into()),
        };
    }
    Ok(m)
}

fn gl_uniform_matrix4fv_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    // args: [self, matrix] → loc 0; [self, loc, matrix] → loc
    let (loc, matrix) = if args.len() >= 3 {
        let loc = num_arg(args, 1)? as i32;
        let matrix = mat4_from_value(args.get(2).ok_or("uniformMatrix4fv(loc, matrix)")?)?;
        (loc, matrix)
    } else {
        let matrix = mat4_from_value(args.get(1).ok_or("uniformMatrix4fv(matrix)")?)?;
        (0, matrix)
    };
    Ok(Value::Bool(webgl::uniform_matrix4fv_at(id, loc, matrix)?))
}

fn gl_perspective_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let fov = f64_arg(args, 1)? as f32;
    let aspect = f64_arg(args, 2)? as f32;
    let near = f64_arg(args, 3)? as f32;
    let far = f64_arg(args, 4)? as f32;
    Ok(Value::Bool(webgl::set_perspective(id, fov, aspect, near, far)?))
}

fn gl_look_at_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    Ok(Value::Bool(webgl::set_look_at(
        id,
        f64_arg(args, 1)? as f32,
        f64_arg(args, 2)? as f32,
        f64_arg(args, 3)? as f32,
        f64_arg(args, 4)? as f32,
        f64_arg(args, 5)? as f32,
        f64_arg(args, 6)? as f32,
        f64_arg(args, 7)? as f32,
        f64_arg(args, 8)? as f32,
        f64_arg(args, 9)? as f32,
    )?))
}

fn gl_rotate_model_y_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let angle = f64_arg(args, 1)? as f32;
    Ok(Value::Bool(webgl::set_model_rotation_y(id, angle)?))
}

fn gl_enable_depth_test_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let enabled = matches!(args.get(1), Some(Value::Bool(true)));
    Ok(Value::Bool(webgl::enable_depth_test(id, enabled)?))
}

fn texture_id_from_value(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64),
        Value::Object(map) => match map.get("id") {
            Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
            _ => Err("texture object missing id".into()),
        },
        _ => Err("expected texture id or object".into()),
    }
}

fn rgba_bytes_from_value(v: &Value) -> Result<Vec<u8>, String> {
    // P2: Uint8Array zero-copy staging (same path as PNG decode).
    if crate::runtime::shared_memory::is_uint8_array(v) {
        return crate::runtime::shared_memory::uint8_array_to_vec(v);
    }
    match v {
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Number(n) => Ok(*n as u8),
                _ => Err("texImage2D pixel data expects bytes".into()),
            })
            .collect(),
        _ => Err("texImage2D expects RGBA byte array or Uint8Array".into()),
    }
}

fn canvas_id_from_value(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64),
        Value::Object(map) if matches!(map.get("__kab_ctx"), Some(Value::Bool(true))) => {
            match map.get("id") {
                Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
                _ => Err("canvas context missing id".into()),
            }
        }
        _ => Err("texImage2D source must be canvas context".into()),
    }
}

fn gl_create_texture_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let tex = webgl::create_texture()?;
    Ok(texture_value(&tex))
}

fn gl_bind_texture_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let tex = texture_id_from_value(
        args.get(1).ok_or("bindTexture(texture) expects texture")?,
    )?;
    Ok(Value::Bool(webgl::bind_texture(id, tex)?))
}

fn gl_tex_image2d_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let tex = texture_id_from_value(
        args.get(1).ok_or("texImage2D(texture, …) expects texture")?,
    )?;
    if args.len() == 3 {
        let src = args.get(2).ok_or("texImage2D(texture, source)")?;
        let canvas_id = canvas_id_from_value(src)?;
        webgl::tex_image2d_canvas(tex, canvas_id)?;
        return Ok(texture_value(
            &webgl::get_texture(tex).ok_or("webgl: texture missing after upload")?,
        ));
    }
    let w = num_arg(args, 2)? as u32;
    let h = num_arg(args, 3)? as u32;
    let pixels = rgba_bytes_from_value(args.get(4).ok_or("texImage2D needs pixel data")?)?;
    webgl::tex_image2d_rgba(tex, w, h, &pixels)?;
    Ok(texture_value(
        &webgl::get_texture(tex).ok_or("webgl: texture missing after upload")?,
    ))
}

fn texture_value(tex: &webgl::GlTexture) -> Value {
    let mut o = HashMap::new();
    o.insert("__kab_gl_tex".into(), Value::Bool(true));
    o.insert("id".into(), Value::Number(tex.id as i64));
    o.insert("width".into(), Value::Number(tex.width as i64));
    o.insert("height".into(), Value::Number(tex.height as i64));
    Value::Object(o)
}

fn framebuffer_value(fb: &webgl::GlFramebuffer) -> Value {
    let mut o = HashMap::new();
    o.insert("__kab_gl_fbo".into(), Value::Bool(true));
    o.insert("id".into(), Value::Number(fb.id as i64));
    o.insert("complete".into(), Value::Bool(fb.complete));
    if let Some(tid) = fb.color_texture {
        o.insert("colorTexture".into(), Value::Number(tid as i64));
    }
    Value::Object(o)
}

fn fb_id_from_value(v: &Value) -> Result<u64, String> {
    match v {
        Value::Number(n) if *n > 0 => Ok(*n as u64),
        Value::Object(map) => match map.get("id") {
            Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
            _ => Err("framebuffer object missing id".into()),
        },
        _ => Err("expected framebuffer id or object".into()),
    }
}

fn gl_create_framebuffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    Ok(framebuffer_value(&webgl::create_framebuffer()?))
}

fn gl_bind_framebuffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = gl_id_from_receiver(args)?;
    let fb = match args.get(1) {
        None | Some(Value::Null) | Some(Value::Undefined) => None,
        Some(v) => Some(fb_id_from_value(v)?),
    };
    Ok(Value::Bool(webgl::bind_framebuffer(id, fb)?))
}

fn gl_framebuffer_texture_2d_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = gl_id_from_receiver(args)?;
    // gl.framebufferTexture2D(fb, texture) or (target, attachment, texTarget, texture, level)
    let (fb, tex) = if args.len() >= 3 {
        // method: self, fb, tex  OR self, target, attachment, textarget, tex, level
        if args.len() >= 6 {
            (
                fb_id_from_value(args.get(1).ok_or("framebuffer")?)?,
                texture_id_from_value(args.get(4).ok_or("texture")?)?,
            )
        } else {
            (
                fb_id_from_value(args.get(1).ok_or("framebuffer")?)?,
                texture_id_from_value(args.get(2).ok_or("texture")?)?,
            )
        }
    } else {
        return Err("framebufferTexture2D(fb, texture)".into());
    };
    Ok(Value::Bool(webgl::framebuffer_texture_2d(fb, tex)?))
}

fn gl_check_framebuffer_status_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = gl_id_from_receiver(args)?;
    let fb = fb_id_from_value(args.get(1).ok_or("checkFramebufferStatus(fb)")?)?;
    Ok(Value::String(webgl::check_framebuffer_status(fb)?.into()))
}

fn gl_compile_shader_from_files_native(
    args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    // Method: (self, vert, frag) or flat: (vert, frag)
    let (vert, frag) = if args.first().is_some_and(|v| matches!(v, Value::Object(m) if m.contains_key("__kab_gl_ctx")))
    {
        (
            match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("compileShaderFromFiles(vert, frag)".into()),
            },
            match args.get(2) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("compileShaderFromFiles(vert, frag)".into()),
            },
        )
    } else {
        (
            match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("compileShaderFromFiles(vert, frag)".into()),
            },
            match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("compileShaderFromFiles(vert, frag)".into()),
            },
        )
    };
    let prog = webgl::compile_shader_from_files(&vert, &frag)?;
    let mut o = HashMap::new();
    o.insert("id".into(), Value::Number(prog.id as i64));
    o.insert("vertex".into(), Value::String(prog.vertex));
    o.insert("fragment".into(), Value::String(prog.fragment));
    Ok(Value::Object(o))
}

fn hashmap_str_to_value(m: std::collections::HashMap<String, String>) -> Value {
    Value::Object(m.into_iter().map(|(k, v)| (k, Value::String(v))).collect())
}

fn gl_load_wgsl_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _id = gl_id_from_receiver(args)?;
    let kind = str_arg(args, 1)?;
    let source = str_arg(args, 2)?;
    Ok(hashmap_str_to_value(crate::runtime::render::gpu3d::load_wgsl(
        &kind, &source, None,
    )?))
}

fn gl_load_wgsl_from_file_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _id = gl_id_from_receiver(args)?;
    let kind = str_arg(args, 1)?;
    let path = str_arg(args, 2)?;
    Ok(hashmap_str_to_value(
        crate::runtime::render::gpu3d::load_wgsl_from_file(&kind, &path)?,
    ))
}

fn attach_gl_methods(o: &mut HashMap<String, Value>) {
    o.insert("clearColor".into(), Value::NativeFunction(gl_clear_color_native));
    o.insert("clear".into(), Value::NativeFunction(gl_clear_native));
    o.insert("drawArrays".into(), Value::NativeFunction(gl_draw_arrays_native));
    o.insert(
        "drawArraysInstanced".into(),
        Value::NativeFunction(gl_draw_arrays_instanced_native),
    );
    o.insert("drawElements".into(), Value::NativeFunction(gl_draw_elements_native));
    o.insert(
        "drawElementsInstanced".into(),
        Value::NativeFunction(gl_draw_elements_instanced_native),
    );
    o.insert("bindBuffer".into(), Value::NativeFunction(gl_bind_buffer_native));
    o.insert("createBuffer".into(), Value::NativeFunction(gl_create_buffer_native));
    o.insert(
        "createIndexBuffer".into(),
        Value::NativeFunction(gl_create_index_buffer_native),
    );
    o.insert("compileShader".into(), Value::NativeFunction(gl_compile_shader_native));
    o.insert("useProgram".into(), Value::NativeFunction(gl_use_program_native));
    o.insert("uniform4f".into(), Value::NativeFunction(gl_uniform4f_native));
    o.insert(
        "uniformMatrix4fv".into(),
        Value::NativeFunction(gl_uniform_matrix4fv_native),
    );
    o.insert("perspective".into(), Value::NativeFunction(gl_perspective_native));
    o.insert("lookAt".into(), Value::NativeFunction(gl_look_at_native));
    o.insert("rotateModelY".into(), Value::NativeFunction(gl_rotate_model_y_native));
    o.insert("enableDepthTest".into(), Value::NativeFunction(gl_enable_depth_test_native));
    o.insert("createTexture".into(), Value::NativeFunction(gl_create_texture_native));
    o.insert("bindTexture".into(), Value::NativeFunction(gl_bind_texture_native));
    o.insert("texImage2D".into(), Value::NativeFunction(gl_tex_image2d_native));
    o.insert("createFramebuffer".into(), Value::NativeFunction(gl_create_framebuffer_native));
    o.insert("bindFramebuffer".into(), Value::NativeFunction(gl_bind_framebuffer_native));
    o.insert(
        "framebufferTexture2D".into(),
        Value::NativeFunction(gl_framebuffer_texture_2d_native),
    );
    o.insert(
        "checkFramebufferStatus".into(),
        Value::NativeFunction(gl_check_framebuffer_status_native),
    );
    o.insert(
        "compileShaderFromFiles".into(),
        Value::NativeFunction(gl_compile_shader_from_files_native),
    );
    o.insert("loadWgsl".into(), Value::NativeFunction(gl_load_wgsl_native));
    o.insert(
        "loadWgslFromFile".into(),
        Value::NativeFunction(gl_load_wgsl_from_file_native),
    );
    o.insert("TEXTURE_2D".into(), Value::String("texture_2d".into()));
    o.insert("ARRAY_BUFFER".into(), Value::String("array".into()));
    o.insert(
        "ELEMENT_ARRAY_BUFFER".into(),
        Value::String("element_array".into()),
    );
    o.insert("FRAMEBUFFER".into(), Value::String("framebuffer".into()));
    o.insert(
        "COLOR_ATTACHMENT0".into(),
        Value::String("color_attachment0".into()),
    );
}

/// Build a JS-style WebGL context object for an existing context id.
pub fn gl_context_value(
    ctx: &webgl::WebGlContext,
    kind: &str,
    layer: &str,
    host_gl_ctx_id: Option<u64>,
) -> Value {
    let mut o = HashMap::new();
    o.insert("__kab_gl_ctx".into(), Value::Bool(true));
    o.insert("id".into(), Value::Number(ctx.id as i64));
    o.insert("width".into(), Value::Number(ctx.width as i64));
    o.insert("height".into(), Value::Number(ctx.height as i64));
    o.insert("kind".into(), Value::String(kind.into()));
    o.insert("layer".into(), Value::String(layer.into()));
    o.insert("backend".into(), Value::String(ctx.backend.clone()));
    o.insert("depth_test".into(), Value::Bool(ctx.depth_test));
    if let Some(hid) = host_gl_ctx_id {
        o.insert("host_gl_ctx_id".into(), Value::Number(hid as i64));
    }
    attach_gl_methods(&mut o);
    Value::Object(o)
}

/// Create WebGL context for canvas `getContext("webgl"|"webgl2")`.
pub fn create_gl_context(
    width: u32,
    height: u32,
    kind: &str,
    layer: &str,
    host_gl_ctx_id: Option<u64>,
) -> Result<Value, String> {
    let ctx = webgl::create_context(width, height)?;
    Ok(gl_context_value(&ctx, kind, layer, host_gl_ctx_id))
}

pub fn is_webgl_kind(kind: &str) -> bool {
    matches!(
        kind,
        "webgl" | "webgl2" | "experimental-webgl" | "webgl2-compute"
    )
}
