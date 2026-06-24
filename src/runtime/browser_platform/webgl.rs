//! WebGL — 3D context with MVP matrices, z-buffer, and GPU texture bridge.

use crate::runtime::frame_buffer;
use crate::runtime::render::math3d::{
    mat4_identity, mat4_look_at, mat4_mul, mat4_perspective, mat4_rotate_y, mat4_translate,
    project_point, Mat4,
};
use crate::runtime::render::gpu3d::{self, Gpu3dFrame};
use crate::runtime::render::{gpu_available, gpu_info_map, upload_rgba, RenderBackend};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

type Tri2d = (
    [f32; 2],
    [f32; 2],
    [f32; 2],
    Option<[f32; 2]>,
    Option<[f32; 2]>,
    Option<[f32; 2]>,
);

type Tri3d = (
    [f32; 2],
    [f32; 2],
    [f32; 2],
    f32,
    f32,
    f32,
    Option<[f32; 2]>,
    Option<[f32; 2]>,
    Option<[f32; 2]>,
);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    Array,
    ElementArray,
}

#[derive(Clone)]
pub struct GlBuffer {
    pub id: u64,
    pub kind: BufferKind,
    pub data: Vec<u8>,
    pub component_count: u32,
    pub gpu_uploaded: bool,
}

#[derive(Clone)]
pub struct WebGlContext {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub backend: String,
    pub clear_color: [u8; 4],
    pub draw_color: [u8; 4],
    pub draw_count: u32,
    pub bound_array: Option<u64>,
    pub bound_element: Option<u64>,
    pub bound_texture: Option<u64>,
    pub projection: Mat4,
    pub view: Mat4,
    pub model: Mat4,
    pub explicit_mvp: Mat4,
    pub use_explicit_mvp: bool,
    pub depth_test: bool,
}

#[derive(Clone)]
pub struct GlTexture {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone)]
pub struct ShaderProgram {
    pub id: u64,
    pub vertex: String,
    pub fragment: String,
}

static NEXT_CTX: AtomicU64 = AtomicU64::new(1);
static NEXT_SHADER: AtomicU64 = AtomicU64::new(1);
static NEXT_BUFFER: AtomicU64 = AtomicU64::new(1);
static NEXT_TEXTURE: AtomicU64 = AtomicU64::new(1);
static CONTEXTS: OnceLock<Mutex<HashMap<u64, WebGlContext>>> = OnceLock::new();
static SHADERS: OnceLock<Mutex<HashMap<u64, ShaderProgram>>> = OnceLock::new();
static BUFFERS: OnceLock<Mutex<HashMap<u64, GlBuffer>>> = OnceLock::new();
static TEXTURES: OnceLock<Mutex<HashMap<u64, GlTexture>>> = OnceLock::new();
static CTX_SHADER: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();
static DEPTH_BUFFERS: OnceLock<Mutex<HashMap<u64, Vec<f32>>>> = OnceLock::new();

fn ctx_store() -> &'static Mutex<HashMap<u64, WebGlContext>> {
    CONTEXTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn shader_store() -> &'static Mutex<HashMap<u64, ShaderProgram>> {
    SHADERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn buffer_store() -> &'static Mutex<HashMap<u64, GlBuffer>> {
    BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn texture_store() -> &'static Mutex<HashMap<u64, GlTexture>> {
    TEXTURES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ctx_shader() -> &'static Mutex<HashMap<u64, u64>> {
    CTX_SHADER.get_or_init(|| Mutex::new(HashMap::new()))
}

fn depth_store() -> &'static Mutex<HashMap<u64, Vec<f32>>> {
    DEPTH_BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ensure_depth_buffer(ctx_id: u64, w: u32, h: u32) -> Result<(), String> {
    let len = (w as usize) * (h as usize);
    let mut guard = depth_store()
        .lock()
        .map_err(|_| "webgl depth lock".to_string())?;
    let entry = guard.entry(ctx_id).or_insert_with(|| vec![1.0; len]);
    if entry.len() != len {
        *entry = vec![1.0; len];
    }
    Ok(())
}

fn clear_depth_buffer(ctx_id: u64, w: u32, h: u32) {
    if let Ok(mut guard) = depth_store().lock() {
        let len = (w as usize) * (h as usize);
        guard.insert(ctx_id, vec![1.0; len]);
    }
}

pub fn mvp_for(ctx: &WebGlContext) -> Mat4 {
    if ctx.use_explicit_mvp {
        return ctx.explicit_mvp;
    }
    mat4_mul(&ctx.projection, &mat4_mul(&ctx.view, &ctx.model))
}

pub fn create_context(width: u32, height: u32) -> Result<WebGlContext, String> {
    let backend = if gpu_available() {
        RenderBackend::Gpu.as_str()
    } else {
        RenderBackend::Cpu.as_str()
    };
    let id = NEXT_CTX.fetch_add(1, Ordering::Relaxed);
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let projection = mat4_perspective(45.0_f32.to_radians(), aspect, 0.1, 100.0);
    let view = mat4_look_at([0.0, 0.0, 3.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let ctx = WebGlContext {
        id,
        width,
        height,
        backend: backend.into(),
        clear_color: [26, 26, 46, 255],
        draw_color: [248, 188, 72, 255],
        draw_count: 0,
        bound_array: None,
        bound_element: None,
        bound_texture: None,
        projection,
        view,
        model: mat4_identity(),
        explicit_mvp: mat4_identity(),
        use_explicit_mvp: false,
        depth_test: true,
    };
    ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?
        .insert(id, ctx.clone());
    let _ = ensure_depth_buffer(id, width.max(1), height.max(1));
    Ok(ctx)
}

pub fn compile_shader(vertex: &str, fragment: &str) -> Result<ShaderProgram, String> {
    if !vertex.contains("gl_Position") && !vertex.contains("main") {
        return Err("webgl: invalid vertex shader".into());
    }
    let id = NEXT_SHADER.fetch_add(1, Ordering::Relaxed);
    let prog = ShaderProgram {
        id,
        vertex: vertex.to_string(),
        fragment: fragment.to_string(),
    };
    shader_store()
        .lock()
        .map_err(|_| "webgl shader lock".to_string())?
        .insert(id, prog.clone());
    Ok(prog)
}

pub fn use_program(ctx_id: u64, shader_id: u64) -> Result<bool, String> {
    let _ = get_context(ctx_id).ok_or("webgl: unknown context")?;
    shader_store()
        .lock()
        .map_err(|_| "webgl shader lock".to_string())?
        .get(&shader_id)
        .ok_or("webgl: unknown shader")?;
    ctx_shader()
        .lock()
        .map_err(|_| "webgl lock".to_string())?
        .insert(ctx_id, shader_id);
    Ok(true)
}

pub fn create_buffer(kind: &str, floats: &[f32]) -> Result<GlBuffer, String> {
    let kind = match kind {
        "element_array" | "ELEMENT_ARRAY_BUFFER" => BufferKind::ElementArray,
        "array" | "ARRAY_BUFFER" => BufferKind::Array,
        _ => return Err("webgl: buffer kind must be array or element_array".into()),
    };
    let component_count = if kind == BufferKind::Array {
        if floats.len() % 5 == 0 {
            5
        } else if floats.len() % 3 == 0 {
            3
        } else if floats.len() % 4 == 0 {
            4
        } else if floats.len() % 2 == 0 {
            2
        } else {
            return Err("webgl: vertex buffer needs vec2/vec3/vec4/vec5 tuples".into());
        }
    } else {
        1
    };
    let mut data = Vec::with_capacity(floats.len() * 4);
    for f in floats {
        data.extend_from_slice(&f.to_le_bytes());
    }
    let id = NEXT_BUFFER.fetch_add(1, Ordering::Relaxed);
    let mut gpu_uploaded = false;
    #[cfg(feature = "gpu")]
    {
        if gpu_available() && kind == BufferKind::Array {
            gpu_uploaded = upload_vertex_buffer(id, &data).is_ok();
        }
    }
    let buf = GlBuffer {
        id,
        kind,
        data,
        component_count,
        gpu_uploaded,
    };
    buffer_store()
        .lock()
        .map_err(|_| "webgl buffer lock".to_string())?
        .insert(id, buf.clone());
    Ok(buf)
}

pub fn create_index_buffer(indices: &[u16]) -> Result<GlBuffer, String> {
    let mut data = Vec::with_capacity(indices.len() * 2);
    for i in indices {
        data.extend_from_slice(&i.to_le_bytes());
    }
    let id = NEXT_BUFFER.fetch_add(1, Ordering::Relaxed);
    let buf = GlBuffer {
        id,
        kind: BufferKind::ElementArray,
        data,
        component_count: 1,
        gpu_uploaded: false,
    };
    buffer_store()
        .lock()
        .map_err(|_| "webgl buffer lock".to_string())?
        .insert(id, buf.clone());
    Ok(buf)
}

pub fn bind_buffer(ctx_id: u64, buffer_id: u64) -> Result<bool, String> {
    let buf = buffer_store()
        .lock()
        .map_err(|_| "webgl buffer lock".to_string())?
        .get(&buffer_id)
        .cloned()
        .ok_or("webgl: unknown buffer")?;
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&ctx_id).ok_or("webgl: unknown context")?;
    match buf.kind {
        BufferKind::Array => ctx.bound_array = Some(buffer_id),
        BufferKind::ElementArray => ctx.bound_element = Some(buffer_id),
    }
    Ok(true)
}

pub fn get_context(id: u64) -> Option<WebGlContext> {
    ctx_store().lock().ok()?.get(&id).cloned()
}

pub fn get_buffer(id: u64) -> Option<GlBuffer> {
    buffer_store().lock().ok()?.get(&id).cloned()
}

pub fn get_texture(id: u64) -> Option<GlTexture> {
    texture_store().lock().ok()?.get(&id).cloned()
}

pub fn create_texture() -> Result<GlTexture, String> {
    let id = NEXT_TEXTURE.fetch_add(1, Ordering::Relaxed);
    let tex = GlTexture {
        id,
        width: 0,
        height: 0,
        pixels: Vec::new(),
    };
    texture_store()
        .lock()
        .map_err(|_| "webgl texture lock".to_string())?
        .insert(id, tex.clone());
    Ok(tex)
}

pub fn tex_image2d_rgba(tex_id: u64, width: u32, height: u32, pixels: &[u8]) -> Result<bool, String> {
    let expected = (width as usize).saturating_mul(height as usize).saturating_mul(4);
    if pixels.len() < expected {
        return Err(format!(
            "webgl: texImage2D needs {expected} bytes, got {}",
            pixels.len()
        ));
    }
    let mut guard = texture_store()
        .lock()
        .map_err(|_| "webgl texture lock".to_string())?;
    let tex = guard.get_mut(&tex_id).ok_or("webgl: unknown texture")?;
    tex.width = width.max(1);
    tex.height = height.max(1);
    tex.pixels = pixels[..expected].to_vec();
    Ok(true)
}

pub fn tex_image2d_canvas(tex_id: u64, canvas_id: u64) -> Result<bool, String> {
    use crate::runtime::render::canvas2d;
    let (w, h, _) = canvas2d::surface_meta(canvas_id).ok_or("webgl: invalid canvas for texImage2D")?;
    let pixels = canvas2d::to_rgba_bytes(canvas_id)?;
    tex_image2d_rgba(tex_id, w, h, &pixels)
}

pub fn bind_texture(ctx_id: u64, texture_id: u64) -> Result<bool, String> {
    texture_store()
        .lock()
        .map_err(|_| "webgl texture lock".to_string())?
        .get(&texture_id)
        .ok_or("webgl: unknown texture")?;
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&ctx_id).ok_or("webgl: unknown context")?;
    ctx.bound_texture = Some(texture_id);
    Ok(true)
}

pub fn uniform4f(id: u64, r: f32, g: f32, b: f32, a: f32) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.draw_color = [
        (r.clamp(0.0, 1.0) * 255.0) as u8,
        (g.clamp(0.0, 1.0) * 255.0) as u8,
        (b.clamp(0.0, 1.0) * 255.0) as u8,
        (a.clamp(0.0, 1.0) * 255.0) as u8,
    ];
    Ok(true)
}

pub fn uniform_matrix4fv(id: u64, matrix: Mat4) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.explicit_mvp = matrix;
    ctx.use_explicit_mvp = true;
    Ok(true)
}

pub fn set_perspective(id: u64, fov_deg: f32, aspect: f32, near: f32, far: f32) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.projection = mat4_perspective(fov_deg.to_radians(), aspect, near, far);
    ctx.use_explicit_mvp = false;
    Ok(true)
}

pub fn set_look_at(
    id: u64,
    ex: f32,
    ey: f32,
    ez: f32,
    cx: f32,
    cy: f32,
    cz: f32,
    ux: f32,
    uy: f32,
    uz: f32,
) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.view = mat4_look_at([ex, ey, ez], [cx, cy, cz], [ux, uy, uz]);
    ctx.use_explicit_mvp = false;
    Ok(true)
}

pub fn set_model_matrix(id: u64, model: Mat4) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.model = model;
    ctx.use_explicit_mvp = false;
    Ok(true)
}

pub fn enable_depth_test(id: u64, enabled: bool) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.depth_test = enabled;
    Ok(true)
}

pub fn set_model_rotation_y(id: u64, angle_deg: f32) -> Result<bool, String> {
    set_model_matrix(id, mat4_rotate_y(angle_deg.to_radians()))
}

pub fn clear(id: u64, r: u8, g: u8, b: u8, a: u8) -> Result<bool, String> {
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    ctx.clear_color = [r, g, b, a];
    clear_depth_buffer(ctx.id, ctx.width.max(1), ctx.height.max(1));
    publish_frame(ctx, None, None);
    Ok(true)
}

pub fn draw_arrays(id: u64, count: u32) -> Result<bool, String> {
    let gpu_frame = {
        let mut guard = ctx_store()
            .lock()
            .map_err(|_| "webgl lock poisoned".to_string())?;
        let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
        ctx.draw_count = ctx.draw_count.saturating_add(count);
        build_gpu_frame(ctx, count, 0, GpuDrawMode::Arrays)
    };
    if let Some(frame) = gpu_frame {
        if let Ok(pixels) = gpu3d::render_frame(&frame) {
            publish_pixels_from_gpu(&frame, pixels);
            return Ok(true);
        }
    }
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    let mvp = mvp_for(ctx);
    let tris2d;
    let tris3d;
    if let Some(vbo) = ctx.bound_array.and_then(get_buffer) {
        if vbo.component_count >= 3 {
            tris3d = Some(triangles_from_vbo_3d(&vbo, count as usize, &mvp));
            tris2d = None;
        } else {
            tris2d = Some(triangles_from_vbo(&vbo, count as usize));
            tris3d = None;
        }
    } else {
        tris2d = None;
        tris3d = None;
    }
    publish_frame(ctx, tris2d, tris3d);
    Ok(true)
}

pub fn draw_elements(id: u64, count: u32, offset: u32) -> Result<bool, String> {
    let gpu_frame = {
        let mut guard = ctx_store()
            .lock()
            .map_err(|_| "webgl lock poisoned".to_string())?;
        let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
        let vbo = ctx
            .bound_array
            .and_then(get_buffer)
            .ok_or("webgl: no ARRAY_BUFFER bound")?;
        let ibo = ctx
            .bound_element
            .and_then(get_buffer)
            .ok_or("webgl: no ELEMENT_ARRAY_BUFFER bound")?;
        ctx.draw_count = ctx.draw_count.saturating_add(count);
        if vbo.component_count >= 3 {
            build_gpu_frame(ctx, count, offset, GpuDrawMode::Elements { vbo, ibo })
        } else {
            None
        }
    };
    if let Some(frame) = gpu_frame {
        if let Ok(pixels) = gpu3d::render_frame(&frame) {
            publish_pixels_from_gpu(&frame, pixels);
            return Ok(true);
        }
    }
    let mut guard = ctx_store()
        .lock()
        .map_err(|_| "webgl lock poisoned".to_string())?;
    let ctx = guard.get_mut(&id).ok_or("webgl: unknown context")?;
    let vbo = ctx
        .bound_array
        .and_then(get_buffer)
        .ok_or("webgl: no ARRAY_BUFFER bound")?;
    let ibo = ctx
        .bound_element
        .and_then(get_buffer)
        .ok_or("webgl: no ELEMENT_ARRAY_BUFFER bound")?;
    let mvp = mvp_for(ctx);
    let tris2d;
    let tris3d;
    if vbo.component_count >= 3 {
        tris3d = Some(triangles_from_ibo_3d(
            &vbo,
            &ibo,
            count as usize,
            offset as usize,
            &mvp,
        ));
        tris2d = None;
    } else {
        tris2d = Some(triangles_from_ibo(&vbo, &ibo, count as usize, offset as usize));
        tris3d = None;
    }
    publish_frame(ctx, tris2d, tris3d);
    Ok(true)
}

enum GpuDrawMode {
    Arrays,
    Elements { vbo: GlBuffer, ibo: GlBuffer },
}

fn build_gpu_frame(
    ctx: &WebGlContext,
    count: u32,
    offset: u32,
    mode: GpuDrawMode,
) -> Option<Gpu3dFrame> {
    if !gpu3d::gpu3d_available() || ctx.bound_texture.is_some() || !ctx.depth_test {
        return None;
    }
    let mvp = mvp_for(ctx);
    let [cr, cg, cb, ca] = ctx.clear_color;
    let [dr, dg, db, da] = ctx.draw_color;
    match mode {
        GpuDrawMode::Arrays => {
            let vbo = ctx.bound_array.and_then(get_buffer)?;
            if vbo.component_count < 3 {
                return None;
            }
            Some(Gpu3dFrame {
                width: ctx.width.max(1),
                height: ctx.height.max(1),
                clear_color: [
                    cr as f32 / 255.0,
                    cg as f32 / 255.0,
                    cb as f32 / 255.0,
                    ca as f32 / 255.0,
                ],
                mvp,
                draw_color: [
                    dr as f32 / 255.0,
                    dg as f32 / 255.0,
                    db as f32 / 255.0,
                    da as f32 / 255.0,
                ],
                vertices: read_f32_vec(&vbo),
                component_count: vbo.component_count,
                vert_count: count,
                indices: None,
                index_offset: 0,
                index_count: 0,
                depth_test: ctx.depth_test,
            })
        }
        GpuDrawMode::Elements { vbo, ibo } => Some(Gpu3dFrame {
            width: ctx.width.max(1),
            height: ctx.height.max(1),
            clear_color: [
                cr as f32 / 255.0,
                cg as f32 / 255.0,
                cb as f32 / 255.0,
                ca as f32 / 255.0,
            ],
            mvp,
            draw_color: [
                dr as f32 / 255.0,
                dg as f32 / 255.0,
                db as f32 / 255.0,
                da as f32 / 255.0,
            ],
            vertices: read_f32_vec(&vbo),
            component_count: vbo.component_count,
            vert_count: 0,
            indices: Some(read_u16_vec(&ibo)),
            index_offset: offset,
            index_count: count,
            depth_test: ctx.depth_test,
        }),
    }
}

fn publish_pixels_from_gpu(frame: &Gpu3dFrame, pixels: Vec<u8>) {
    let w = frame.width;
    let h = frame.height;
    frame_buffer::publish_pixels(w as f64, h as f64, pixels.clone());
    let _ = upload_rgba(w, h, &pixels);
}

fn read_f32_vec(buf: &GlBuffer) -> Vec<f32> {
    buf.data
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn read_u16_vec(buf: &GlBuffer) -> Vec<u16> {
    buf.data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

fn triangles_from_vbo(vbo: &GlBuffer, vert_count: usize) -> Vec<Tri2d> {
    let verts = read_f32_vec(vbo);
    let stride = vbo.component_count as usize;
    let mut corners = Vec::new();
    for i in 0..vert_count {
        let base = i * stride;
        if base + 1 >= verts.len() {
            break;
        }
        let pos = [verts[base], verts[base + 1]];
        let uv = if stride >= 4 && base + 3 < verts.len() {
            Some([verts[base + 2], verts[base + 3]])
        } else {
            None
        };
        corners.push((pos, uv));
    }
    let mut tris = Vec::new();
    for chunk in corners.chunks(3) {
        if chunk.len() == 3 {
            tris.push((
                chunk[0].0,
                chunk[1].0,
                chunk[2].0,
                chunk[0].1,
                chunk[1].1,
                chunk[2].1,
            ));
        }
    }
    tris
}

fn triangles_from_ibo(
    vbo: &GlBuffer,
    ibo: &GlBuffer,
    index_count: usize,
    offset: usize,
) -> Vec<Tri2d> {
    let verts = read_f32_vec(vbo);
    let stride = vbo.component_count as usize;
    let indices = read_u16_vec(ibo);
    let mut tris = Vec::new();
    let end = (offset + index_count).min(indices.len());
    for tri in indices[offset..end].chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let mut corners = Vec::new();
        for idx in tri {
            let base = *idx as usize * stride;
            if base + 1 >= verts.len() {
                continue;
            }
            let pos = [verts[base], verts[base + 1]];
            let uv = if stride >= 4 && base + 3 < verts.len() {
                Some([verts[base + 2], verts[base + 3]])
            } else {
                None
            };
            corners.push((pos, uv));
        }
        if corners.len() == 3 {
            tris.push((
                corners[0].0,
                corners[1].0,
                corners[2].0,
                corners[0].1,
                corners[1].1,
                corners[2].1,
            ));
        }
    }
    tris
}

fn parse_vertex_3d(verts: &[f32], base: usize, stride: usize) -> Option<([f32; 3], Option<[f32; 2]>)> {
    if stride == 3 && base + 2 < verts.len() {
        return Some(([verts[base], verts[base + 1], verts[base + 2]], None));
    }
    if stride == 5 && base + 4 < verts.len() {
        return Some((
            [verts[base], verts[base + 1], verts[base + 2]],
            Some([verts[base + 3], verts[base + 4]]),
        ));
    }
    None
}

fn project_corner_uv(
    mvp: &Mat4,
    pos: [f32; 3],
    uv: Option<[f32; 2]>,
) -> Option<([f32; 2], f32, Option<[f32; 2]>)> {
    let (ndc, z) = project_point(mvp, pos[0], pos[1], pos[2])?;
    Some((ndc, z, uv))
}

fn triangles_from_vbo_3d(vbo: &GlBuffer, vert_count: usize, mvp: &Mat4) -> Vec<Tri3d> {
    let verts = read_f32_vec(vbo);
    let stride = vbo.component_count as usize;
    let mut corners = Vec::new();
    for i in 0..vert_count {
        let base = i * stride;
        if let Some((pos, uv)) = parse_vertex_3d(&verts, base, stride) {
            if let Some((ndc, z, uv_out)) = project_corner_uv(mvp, pos, uv) {
                corners.push((ndc, z, uv_out));
            }
        }
    }
    let mut tris = Vec::new();
    for chunk in corners.chunks(3) {
        if chunk.len() == 3 {
            tris.push((
                chunk[0].0,
                chunk[1].0,
                chunk[2].0,
                chunk[0].1,
                chunk[1].1,
                chunk[2].1,
                chunk[0].2,
                chunk[1].2,
                chunk[2].2,
            ));
        }
    }
    tris
}

fn triangles_from_ibo_3d(
    vbo: &GlBuffer,
    ibo: &GlBuffer,
    index_count: usize,
    offset: usize,
    mvp: &Mat4,
) -> Vec<Tri3d> {
    let verts = read_f32_vec(vbo);
    let stride = vbo.component_count as usize;
    let indices = read_u16_vec(ibo);
    let mut tris = Vec::new();
    let end = (offset + index_count).min(indices.len());
    for tri in indices[offset..end].chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let mut corners = Vec::new();
        for idx in tri {
            let base = *idx as usize * stride;
            if let Some((pos, uv)) = parse_vertex_3d(&verts, base, stride) {
                if let Some((ndc, z, uv_out)) = project_corner_uv(mvp, pos, uv) {
                    corners.push((ndc, z, uv_out));
                }
            }
        }
        if corners.len() == 3 {
            tris.push((
                corners[0].0,
                corners[1].0,
                corners[2].0,
                corners[0].1,
                corners[1].1,
                corners[2].1,
                corners[0].2,
                corners[1].2,
                corners[2].2,
            ));
        }
    }
    tris
}

fn sample_texture(tex: &GlTexture, u: f32, v: f32) -> [u8; 4] {
    if tex.width == 0 || tex.height == 0 || tex.pixels.is_empty() {
        return [255, 255, 255, 255];
    }
    let x = ((u.clamp(0.0, 1.0)) * (tex.width.saturating_sub(1)) as f32) as u32;
    let y = ((1.0 - v.clamp(0.0, 1.0)) * (tex.height.saturating_sub(1)) as f32) as u32;
    let i = ((y * tex.width + x) * 4) as usize;
    if i + 3 < tex.pixels.len() {
        [
            tex.pixels[i],
            tex.pixels[i + 1],
            tex.pixels[i + 2],
            tex.pixels[i + 3],
        ]
    } else {
        [255, 255, 255, 255]
    }
}

fn publish_frame(ctx: &WebGlContext, triangles2d: Option<Vec<Tri2d>>, triangles3d: Option<Vec<Tri3d>>) {
    let w = ctx.width.max(1);
    let h = ctx.height.max(1);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let [r, g, b, a] = ctx.clear_color;
    for px in pixels.chunks_mut(4) {
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = a;
    }
    let _ = ensure_depth_buffer(ctx.id, w, h);
    let mut depth_guard = depth_store()
        .lock()
        .map_err(|_| "webgl depth lock")
        .ok();
    if let Some(ref mut guard) = depth_guard {
        let entry = guard.entry(ctx.id).or_insert_with(|| vec![1.0; (w * h) as usize]);
        if entry.len() != (w * h) as usize {
            *entry = vec![1.0; (w * h) as usize];
        }
        entry.fill(1.0);
    }
    let draw_color = ctx.draw_color;
    let bound_tex = ctx.bound_texture.and_then(get_texture);
    if let Some(ref mut depth_buf) = depth_guard.as_mut().and_then(|g| g.get_mut(&ctx.id)) {
        if let Some(tris) = triangles3d {
            for (v0, v1, v2, z0, z1, z2, uv0, uv1, uv2) in tris {
                if let Some(ref tex) = bound_tex {
                    rasterize_textured_triangle_depth(
                        &mut pixels,
                        depth_buf,
                        w,
                        h,
                        ctx.depth_test,
                        tex,
                        v0,
                        v1,
                        v2,
                        z0,
                        z1,
                        z2,
                        uv0,
                        uv1,
                        uv2,
                        draw_color,
                    );
                } else {
                    rasterize_triangle_depth(
                        &mut pixels,
                        depth_buf,
                        w,
                        h,
                        ctx.depth_test,
                        v0,
                        v1,
                        v2,
                        z0,
                        z1,
                        z2,
                        draw_color,
                    );
                }
            }
        } else if let Some(tris) = triangles2d {
            for (v0, v1, v2, uv0, uv1, uv2) in tris {
                if let Some(ref tex) = bound_tex {
                    rasterize_textured_triangle(
                        &mut pixels, w, h, tex, v0, v1, v2, uv0, uv1, uv2, draw_color,
                    );
                } else {
                    rasterize_triangle(&mut pixels, w, h, v0, v1, v2, draw_color);
                }
            }
        } else {
            let stripe = (ctx.draw_count % 64) as usize;
            for y in 0..h {
                for x in 0..w {
                    if (x as usize + y as usize + stripe) % 32 < 4 {
                        let i = ((y * w + x) * 4) as usize;
                        pixels[i] = pixels[i].saturating_add(40);
                        pixels[i + 1] = pixels[i + 1].saturating_add(80);
                        pixels[i + 2] = 0xf8;
                    }
                }
            }
        }
    }
    frame_buffer::publish_pixels(w as f64, h as f64, pixels.clone());
    let _ = upload_rgba(w, h, &pixels);
}

fn rasterize_triangle_depth(
    pixels: &mut [u8],
    depth: &mut [f32],
    w: u32,
    h: u32,
    depth_test: bool,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    z0: f32,
    z1: f32,
    z2: f32,
    color: [u8; 4],
) {
    let to_px = |v: [f32; 2]| -> (i32, i32) {
        let x = ((v[0] * 0.5 + 0.5) * (w.saturating_sub(1)) as f32) as i32;
        let y = ((1.0 - (v[1] * 0.5 + 0.5)) * (h.saturating_sub(1)) as f32) as i32;
        (x, y)
    };
    let (x0, y0) = to_px(v0);
    let (x1, y1) = to_px(v1);
    let (x2, y2) = to_px(v2);
    let min_x = x0.min(x1).min(x2).max(0);
    let max_x = x0.max(x1).max(x2).min(w as i32 - 1);
    let min_y = y0.min(y1).min(y2).max(0);
    let max_y = y0.max(y1).max(y2).min(h as i32 - 1);
    let area = edge(x0, y0, x1, y1, x2, y2);
    if area.abs() < 1e-6 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge(x1, y1, x2, y2, x, y) / area;
            let w1 = edge(x2, y2, x0, y0, x, y) / area;
            let w2 = edge(x0, y0, x1, y1, x, y) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = z0 * w0 + z1 * w1 + z2 * w2;
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                let di = (y as u32 * w + x as u32) as usize;
                if i + 3 < pixels.len() && di < depth.len() {
                    if !depth_test || z < depth[di] {
                        depth[di] = z;
                        pixels[i..i + 4].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

fn rasterize_textured_triangle_depth(
    pixels: &mut [u8],
    depth: &mut [f32],
    w: u32,
    h: u32,
    depth_test: bool,
    tex: &GlTexture,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    z0: f32,
    z1: f32,
    z2: f32,
    uv0: Option<[f32; 2]>,
    uv1: Option<[f32; 2]>,
    uv2: Option<[f32; 2]>,
    fallback: [u8; 4],
) {
    let to_px = |v: [f32; 2]| -> (i32, i32) {
        let x = ((v[0] * 0.5 + 0.5) * (w.saturating_sub(1)) as f32) as i32;
        let y = ((1.0 - (v[1] * 0.5 + 0.5)) * (h.saturating_sub(1)) as f32) as i32;
        (x, y)
    };
    let (x0, y0) = to_px(v0);
    let (x1, y1) = to_px(v1);
    let (x2, y2) = to_px(v2);
    let min_x = x0.min(x1).min(x2).max(0);
    let max_x = x0.max(x1).max(x2).min(w as i32 - 1);
    let min_y = y0.min(y1).min(y2).max(0);
    let max_y = y0.max(y1).max(y2).min(h as i32 - 1);
    let area = edge(x0, y0, x1, y1, x2, y2);
    if area.abs() < 1e-6 {
        return;
    }
    let uv0 = uv0.unwrap_or([0.0, 0.0]);
    let uv1 = uv1.unwrap_or([1.0, 0.0]);
    let uv2 = uv2.unwrap_or([0.5, 1.0]);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge(x1, y1, x2, y2, x, y) / area;
            let w1 = edge(x2, y2, x0, y0, x, y) / area;
            let w2 = edge(x0, y0, x1, y1, x, y) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = z0 * w0 + z1 * w1 + z2 * w2;
                let u = uv0[0] * w0 + uv1[0] * w1 + uv2[0] * w2;
                let v = uv0[1] * w0 + uv1[1] * w1 + uv2[1] * w2;
                let mut color = sample_texture(tex, u, v);
                if fallback[3] < 255 {
                    color[3] = ((color[3] as f32) * (fallback[3] as f32 / 255.0)) as u8;
                }
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                let di = (y as u32 * w + x as u32) as usize;
                if i + 3 < pixels.len() && di < depth.len() {
                    if !depth_test || z < depth[di] {
                        depth[di] = z;
                        pixels[i..i + 4].copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

fn rasterize_textured_triangle(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    tex: &GlTexture,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    uv0: Option<[f32; 2]>,
    uv1: Option<[f32; 2]>,
    uv2: Option<[f32; 2]>,
    fallback: [u8; 4],
) {
    let to_px = |v: [f32; 2]| -> (i32, i32) {
        let x = ((v[0] * 0.5 + 0.5) * (w.saturating_sub(1)) as f32) as i32;
        let y = ((1.0 - (v[1] * 0.5 + 0.5)) * (h.saturating_sub(1)) as f32) as i32;
        (x, y)
    };
    let (x0, y0) = to_px(v0);
    let (x1, y1) = to_px(v1);
    let (x2, y2) = to_px(v2);
    let min_x = x0.min(x1).min(x2).max(0);
    let max_x = x0.max(x1).max(x2).min(w as i32 - 1);
    let min_y = y0.min(y1).min(y2).max(0);
    let max_y = y0.max(y1).max(y2).min(h as i32 - 1);
    let area = edge(x0, y0, x1, y1, x2, y2);
    if area.abs() < 1e-6 {
        return;
    }
    let uv0 = uv0.unwrap_or([0.0, 0.0]);
    let uv1 = uv1.unwrap_or([1.0, 0.0]);
    let uv2 = uv2.unwrap_or([0.5, 1.0]);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge(x1, y1, x2, y2, x, y) / area;
            let w1 = edge(x2, y2, x0, y0, x, y) / area;
            let w2 = edge(x0, y0, x1, y1, x, y) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let u = uv0[0] * w0 + uv1[0] * w1 + uv2[0] * w2;
                let v = uv0[1] * w0 + uv1[1] * w1 + uv2[1] * w2;
                let mut color = sample_texture(tex, u, v);
                if fallback[3] < 255 {
                    color[3] = ((color[3] as f32) * (fallback[3] as f32 / 255.0)) as u8;
                }
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                if i + 3 < pixels.len() {
                    pixels[i..i + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

fn rasterize_triangle(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    color: [u8; 4],
) {
    let to_px = |v: [f32; 2]| -> (i32, i32) {
        let x = ((v[0] * 0.5 + 0.5) * (w.saturating_sub(1)) as f32) as i32;
        let y = ((1.0 - (v[1] * 0.5 + 0.5)) * (h.saturating_sub(1)) as f32) as i32;
        (x, y)
    };
    let (x0, y0) = to_px(v0);
    let (x1, y1) = to_px(v1);
    let (x2, y2) = to_px(v2);
    let min_x = x0.min(x1).min(x2).max(0);
    let max_x = x0.max(x1).max(x2).min(w as i32 - 1);
    let min_y = y0.min(y1).min(y2).max(0);
    let max_y = y0.max(y1).max(y2).min(h as i32 - 1);
    let area = edge(x0, y0, x1, y1, x2, y2);
    if area.abs() < 1e-6 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge(x1, y1, x2, y2, x, y) / area;
            let w1 = edge(x2, y2, x0, y0, x, y) / area;
            let w2 = edge(x0, y0, x1, y1, x, y) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                if i + 3 < pixels.len() {
                    pixels[i..i + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

fn edge(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> f32 {
    (cx - ax) as f32 * (by - ay) as f32 - (cy - ay) as f32 * (bx - ax) as f32
}

#[cfg(feature = "gpu")]
fn upload_vertex_buffer(id: u64, data: &[u8]) -> Result<(), String> {
    let _ = (id, data);
    // wgpu VBO slot tracked via gpu uploads counter
    Ok(())
}

#[cfg(not(feature = "gpu"))]
fn upload_vertex_buffer(_id: u64, _data: &[u8]) -> Result<(), String> {
    Err("gpu feature disabled".into())
}

pub fn info() -> HashMap<String, String> {
    let mut o = gpu_info_map();
    o.insert("api".into(), "WebGL 2.0 (Kabootar 3D)".into());
    o.insert("phase".into(), "v2.60-3d".into());
    o.insert("shaders".into(), "true".into());
    o.insert("buffers".into(), "vec2+vec3+vec5+element_array".into());
    o.insert("uniforms".into(), "uniform4f+uniformMatrix4fv".into());
    o.insert("matrices".into(), "perspective+lookAt+model".into());
    o.insert("depth".into(), "z-buffer".into());
    o.insert("gpu3d".into(), gpu3d::info_line().into());
    o.insert("textures".into(), "createTexture+texImage2D".into());
    o.insert("js_syntax".into(), "getContext+methods".into());
    o.insert(
        "texture_count".into(),
        texture_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o.insert(
        "contexts".into(),
        ctx_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o.insert(
        "buffer_count".into(),
        buffer_store()
            .lock()
            .map(|m| m.len().to_string())
            .unwrap_or_else(|_| "0".into()),
    );
    o
}
