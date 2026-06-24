//! HTML Canvas 2D — advanced offscreen rendering + compositor blit.

use crate::runtime::render::raster::{parse_color, PixelBuffer};
use crate::runtime::render::text::{layout_text, measure_text, paint_text, TextStyle, WhiteSpace};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static SURFACES: OnceLock<Mutex<HashMap<u64, CanvasSurface>>> = OnceLock::new();
static DOM_LINK: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();

fn surfaces() -> &'static Mutex<HashMap<u64, CanvasSurface>> {
    SURFACES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dom_link() -> &'static Mutex<HashMap<u64, u64>> {
    DOM_LINK.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Copy)]
struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Transform {
    fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    fn multiply(&self, o: &Transform) -> Transform {
        Transform {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }

    fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    fn rotate(rad: f64) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct LinearGradient {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    stops: Vec<(f64, u32)>,
}

#[derive(Debug, Clone)]
enum PaintStyle {
    Color(u32),
    Gradient(LinearGradient),
}

#[derive(Debug, Clone)]
enum PathCmd {
    Move(f64, f64),
    Line(f64, f64),
    Arc {
        cx: f64,
        cy: f64,
        r: f64,
        start: f64,
        end: f64,
        ccw: bool,
    },
    Close,
}

#[derive(Debug, Clone)]
struct ContextState {
    transform: Transform,
    fill_style: PaintStyle,
    stroke_style: PaintStyle,
    global_alpha: f32,
    line_width: f64,
    font_size: f32,
    font_family: String,
    text_baseline: String,
}

impl Default for ContextState {
    fn default() -> Self {
        Self {
            transform: Transform::identity(),
            fill_style: PaintStyle::Color(0xff000000),
            stroke_style: PaintStyle::Color(0xff000000),
            global_alpha: 1.0,
            line_width: 1.0,
            font_size: 16.0,
            font_family: "sans-serif".into(),
            text_baseline: "alphabetic".into(),
        }
    }
}

#[derive(Debug, Clone)]
struct CanvasSurface {
    pub id: u64,
    pub dom_id: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub buffer: PixelBuffer,
    pub state: ContextState,
    pub state_stack: Vec<ContextState>,
    pub path: Vec<PathCmd>,
    pub path_start: Option<(f64, f64)>,
    pub current: (f64, f64),
    pub dirty: bool,
}

impl CanvasSurface {
    fn new(id: u64, width: u32, height: u32, dom_id: Option<u64>) -> Self {
        Self {
            id,
            dom_id,
            width,
            height,
            buffer: PixelBuffer::new(width.max(1), height.max(1), 0x00000000),
            state: ContextState::default(),
            state_stack: Vec::new(),
            path: Vec::new(),
            path_start: None,
            current: (0.0, 0.0),
            dirty: false,
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

pub fn info() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("api".into(), "canvas-2d-advanced".into());
    m.insert("version".into(), "1.0".into());
    m.insert(
        "features".into(),
        "paths,gradients,transforms,text,imageData,drawImage,compositor".into(),
    );
    m
}

pub fn create(width: u32, height: u32) -> Result<u64, String> {
    let w = width.clamp(1, 8192);
    let h = height.clamp(1, 8192);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    surfaces()
        .lock()
        .map_err(|_| "canvas lock poisoned".to_string())?
        .insert(id, CanvasSurface::new(id, w, h, None));
    Ok(id)
}

pub fn bind_dom(dom_id: u64, width: u32, height: u32) -> Result<u64, String> {
    let w = width.clamp(1, 8192);
    let h = height.clamp(1, 8192);
    if let Some(existing) = dom_link()
        .lock()
        .map_err(|_| "canvas lock poisoned".to_string())?
        .get(&dom_id)
        .copied()
    {
        if let Some(s) = surfaces()
            .lock()
            .map_err(|_| "canvas lock poisoned".to_string())?
            .get_mut(&existing)
        {
            s.width = w;
            s.height = h;
            s.buffer = PixelBuffer::new(w, h, 0x00000000);
            s.dirty = true;
            return Ok(existing);
        }
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let surface = CanvasSurface::new(id, w, h, Some(dom_id));
    dom_link()
        .lock()
        .map_err(|_| "canvas lock poisoned".to_string())?
        .insert(dom_id, id);
    surfaces()
        .lock()
        .map_err(|_| "canvas lock poisoned".to_string())?
        .insert(id, surface);
    Ok(id)
}

pub fn canvas_id_for_dom(dom_id: u64) -> Option<u64> {
    dom_link().lock().ok()?.get(&dom_id).copied()
}

pub fn surface_meta(id: u64) -> Option<(u32, u32, Option<u64>)> {
    let s = surfaces().lock().ok()?.get(&id)?.clone();
    Some((s.width, s.height, s.dom_id))
}

/// Blit canvas backing store into compositor buffer (scaled to layout box).
pub fn blit_dom_canvas(
    buf: &mut PixelBuffer,
    dom_id: u64,
    dst_x: i32,
    dst_y: i32,
    dst_w: i32,
    dst_h: i32,
) {
    let Some(canvas_id) = canvas_id_for_dom(dom_id) else {
        return;
    };
    let Ok(guard) = surfaces().lock() else {
        return;
    };
    let Some(surface) = guard.get(&canvas_id) else {
        return;
    };
    blit_scaled(buf, &surface.buffer, dst_x, dst_y, dst_w, dst_h);
}

fn blit_scaled(dst: &mut PixelBuffer, src: &PixelBuffer, x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 || src.width == 0 || src.height == 0 {
        return;
    }
    for row in 0..h {
        let sy = (row as u32 * src.height) / h as u32;
        for col in 0..w {
            let sx = (col as u32 * src.width) / w as u32;
            let px = src.pixels[sy as usize * src.width as usize + sx as usize];
            let dx = x + col;
            let dy = y + row;
            if dx >= 0 && dy >= 0 && dx < dst.width as i32 && dy < dst.height as i32 {
                blend_pixel(dst, dx, dy, px);
            }
        }
    }
}

fn with_surface<F, T>(id: u64, f: F) -> Result<T, String>
where
    F: FnOnce(&mut CanvasSurface) -> Result<T, String>,
{
    let mut guard = surfaces().lock().map_err(|_| "canvas lock poisoned".to_string())?;
    let surface = guard.get_mut(&id).ok_or("invalid canvas id")?;
    f(surface)
}

pub fn clear_rect(id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    with_surface(id, |s| {
        fill_rect_pixels(&mut s.buffer, x, y, w, h, 0x00000000, &s.state.transform);
        s.mark_dirty();
        Ok(())
    })
}

pub fn fill_rect(id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    with_surface(id, |s| {
        let color = resolve_fill(&s.state, x, y, w, h)?;
        fill_rect_pixels(&mut s.buffer, x, y, w, h, color, &s.state.transform);
        s.mark_dirty();
        Ok(())
    })
}

pub fn stroke_rect(id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    with_surface(id, |s| {
        let lw = s.state.line_width;
        let color = resolve_stroke(&s.state, x, y, w, h)?;
        stroke_line(
            &mut s.buffer,
            &s.state.transform,
            x,
            y,
            x + w,
            y,
            lw,
            color,
            s.state.global_alpha,
        );
        stroke_line(
            &mut s.buffer,
            &s.state.transform,
            x + w,
            y,
            x + w,
            y + h,
            lw,
            color,
            s.state.global_alpha,
        );
        stroke_line(
            &mut s.buffer,
            &s.state.transform,
            x + w,
            y + h,
            x,
            y + h,
            lw,
            color,
            s.state.global_alpha,
        );
        stroke_line(
            &mut s.buffer,
            &s.state.transform,
            x,
            y + h,
            x,
            y,
            lw,
            color,
            s.state.global_alpha,
        );
        s.mark_dirty();
        Ok(())
    })
}

pub fn set_fill_style(id: u64, color: &str) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.fill_style = PaintStyle::Color(parse_color(color));
        Ok(())
    })
}

pub fn set_stroke_style(id: u64, color: &str) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.stroke_style = PaintStyle::Color(parse_color(color));
        Ok(())
    })
}

pub fn set_global_alpha(id: u64, alpha: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.global_alpha = alpha.clamp(0.0, 1.0) as f32;
        Ok(())
    })
}

pub fn set_line_width(id: u64, w: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.line_width = w.max(0.0);
        Ok(())
    })
}

pub fn set_font(id: u64, spec: &str) -> Result<(), String> {
    with_surface(id, |s| {
        let (size, family) = parse_font(spec);
        s.state.font_size = size;
        s.state.font_family = family;
        Ok(())
    })
}

pub fn save(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state_stack.push(s.state.clone());
        Ok(())
    })
}

pub fn restore(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        if let Some(st) = s.state_stack.pop() {
            s.state = st;
        }
        Ok(())
    })
}

pub fn translate(id: u64, tx: f64, ty: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.transform = s.state.transform.multiply(&Transform::translate(tx, ty));
        Ok(())
    })
}

pub fn scale(id: u64, sx: f64, sy: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.transform = s.state.transform.multiply(&Transform::scale(sx, sy));
        Ok(())
    })
}

pub fn rotate(id: u64, rad: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.transform = s.state.transform.multiply(&Transform::rotate(rad));
        Ok(())
    })
}

pub fn set_transform(
    id: u64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.transform = Transform { a, b, c, d, e, f };
        Ok(())
    })
}

pub fn begin_path(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.clear();
        s.path_start = None;
        Ok(())
    })
}

pub fn move_to(id: u64, x: f64, y: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.push(PathCmd::Move(x, y));
        s.path_start = Some((x, y));
        s.current = (x, y);
        Ok(())
    })
}

pub fn line_to(id: u64, x: f64, y: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.push(PathCmd::Line(x, y));
        s.current = (x, y);
        Ok(())
    })
}

pub fn rect_path(id: u64, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.push(PathCmd::Move(x, y));
        s.path.push(PathCmd::Line(x + w, y));
        s.path.push(PathCmd::Line(x + w, y + h));
        s.path.push(PathCmd::Line(x, y + h));
        s.path.push(PathCmd::Close);
        s.current = (x, y);
        Ok(())
    })
}

pub fn arc(
    id: u64,
    cx: f64,
    cy: f64,
    r: f64,
    start: f64,
    end: f64,
    ccw: bool,
) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.push(PathCmd::Arc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        });
        let (ex, ey) = arc_endpoint(cx, cy, r, end);
        s.current = (ex, ey);
        Ok(())
    })
}

pub fn close_path(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        s.path.push(PathCmd::Close);
        if let Some((x, y)) = s.path_start {
            s.current = (x, y);
        }
        Ok(())
    })
}

pub fn fill(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        let polys = flatten_path(&s.path);
        let bounds = path_bounds(&polys);
        let color_base = resolve_fill(&s.state, bounds.0, bounds.1, bounds.2, bounds.3)?;
        for poly in polys {
            if poly.len() < 3 {
                continue;
            }
            fill_polygon(&mut s.buffer, &poly, &s.state.transform, color_base, s.state.global_alpha);
        }
        s.mark_dirty();
        Ok(())
    })
}

pub fn stroke(id: u64) -> Result<(), String> {
    with_surface(id, |s| {
        let segments = flatten_segments(&s.path);
        let bounds = segment_bounds(&segments);
        let color = resolve_stroke(&s.state, bounds.0, bounds.1, bounds.2, bounds.3)?;
        let lw = s.state.line_width;
        for (x0, y0, x1, y1) in segments {
            stroke_line(
                &mut s.buffer,
                &s.state.transform,
                x0,
                y0,
                x1,
                y1,
                lw,
                color,
                s.state.global_alpha,
            );
        }
        s.mark_dirty();
        Ok(())
    })
}

pub fn fill_text(id: u64, text: &str, x: f64, y: f64) -> Result<(), String> {
    with_surface(id, |s| {
        let (tx, ty) = s.state.transform.apply(x, y);
        let color = resolve_fill(&s.state, x, y, 1.0, 1.0)?;
        let style = TextStyle {
            font_size: s.state.font_size,
            line_height: 1.2,
            max_width: None,
            white_space: WhiteSpace::Nowrap,
            color,
        };
        let layout = layout_text(text, &style);
        let baseline_off = match s.state.text_baseline.as_str() {
            "top" => 0.0,
            "middle" => layout.height * 0.5,
            "bottom" => layout.height,
            _ => s.state.font_size * 0.8,
        };
        paint_text(
            &mut s.buffer,
            &layout,
            tx as f32,
            (ty - baseline_off as f64) as f32,
            &style,
        );
        s.mark_dirty();
        Ok(())
    })
}

pub fn measure_text_size(id: u64, text: &str) -> Result<(f32, f32), String> {
    with_surface(id, |s| {
        let style = TextStyle {
            font_size: s.state.font_size,
            line_height: 1.2,
            max_width: None,
            white_space: WhiteSpace::Nowrap,
            color: 0xff000000,
        };
        Ok(measure_text(text, &style))
    })
}

pub fn create_linear_gradient(
    id: u64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Result<(), String> {
    with_surface(id, |s| {
        s.state.fill_style = PaintStyle::Gradient(LinearGradient {
            x0,
            y0,
            x1,
            y1,
            stops: Vec::new(),
        });
        Ok(())
    })
}

pub fn gradient_add_color_stop(id: u64, offset: f64, color: &str) -> Result<(), String> {
    with_surface(id, |s| {
        if let PaintStyle::Gradient(ref mut g) = s.state.fill_style {
            g.stops.push((offset.clamp(0.0, 1.0), parse_color(color)));
            g.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            Ok(())
        } else {
            Err("no active gradient on fillStyle".into())
        }
    })
}

pub fn get_image_data(id: u64, x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    with_surface(id, |s| {
        let w = w.max(0) as u32;
        let h = h.max(0) as u32;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            for col in 0..w {
                let px = sample(&s.buffer, x + col as i32, y + row as i32);
                let a = ((px >> 24) & 0xff) as u8;
                let r = ((px >> 16) & 0xff) as u8;
                let g = ((px >> 8) & 0xff) as u8;
                let b = (px & 0xff) as u8;
                out.extend_from_slice(&[r, g, b, a]);
            }
        }
        Ok(out)
    })
}

pub fn put_image_data(id: u64, data: &[u8], x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    with_surface(id, |s| {
        let w = w.max(0) as u32;
        let h = h.max(0) as u32;
        if data.len() < (w * h * 4) as usize {
            return Err("putImageData: data too small".into());
        }
        let mut i = 0usize;
        for row in 0..h {
            for col in 0..w {
                let r = data[i];
                let g = data[i + 1];
                let b = data[i + 2];
                let a = data[i + 3];
                i += 4;
                let px = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
                put_pixel(&mut s.buffer, x + col as i32, y + row as i32, px);
            }
        }
        s.mark_dirty();
        Ok(())
    })
}

pub fn draw_image(
    dst_id: u64,
    src_id: u64,
    dx: f64,
    dy: f64,
    dw: f64,
    dh: f64,
) -> Result<(), String> {
    let src = surfaces()
        .lock()
        .map_err(|_| "canvas lock poisoned".to_string())?
        .get(&src_id)
        .ok_or("drawImage: invalid source canvas")?
        .buffer
        .clone();
    with_surface(dst_id, |s| {
        let (tx, ty) = s.state.transform.apply(dx, dy);
        blit_scaled(
            &mut s.buffer,
            &src,
            tx.round() as i32,
            ty.round() as i32,
            dw.max(1.0) as i32,
            dh.max(1.0) as i32,
        );
        s.mark_dirty();
        Ok(())
    })
}

pub fn to_rgba_bytes(id: u64) -> Result<Vec<u8>, String> {
    with_surface(id, |s| Ok(s.buffer.to_rgba_bytes()))
}

// --- raster helpers ---

fn parse_font(spec: &str) -> (f32, String) {
    let parts: Vec<&str> = spec.split_whitespace().collect();
    let mut size = 16.0f32;
    let mut family = "sans-serif".to_string();
    for p in parts {
        if p.ends_with("px") {
            size = p[..p.len() - 2].parse().unwrap_or(16.0);
        } else if !matches!(p, "bold" | "italic" | "normal" | "12px" | "16px") && !p.contains("px")
        {
            family = p.to_string();
        } else if p.contains("px") {
            size = p.replace("px", "").parse().unwrap_or(16.0);
        }
    }
    (size, family)
}

fn resolve_fill(state: &ContextState, x: f64, y: f64, w: f64, h: f64) -> Result<u32, String> {
    match &state.fill_style {
        PaintStyle::Color(c) => Ok(apply_alpha(*c, state.global_alpha)),
        PaintStyle::Gradient(g) => Ok(sample_gradient(g, x + w * 0.5, y + h * 0.5)),
    }
}

fn resolve_stroke(state: &ContextState, x: f64, y: f64, w: f64, h: f64) -> Result<u32, String> {
    match &state.stroke_style {
        PaintStyle::Color(c) => Ok(apply_alpha(*c, state.global_alpha)),
        PaintStyle::Gradient(g) => Ok(sample_gradient(g, x + w * 0.5, y + h * 0.5)),
    }
}

fn apply_alpha(color: u32, alpha: f32) -> u32 {
    let a = ((color >> 24) & 0xff) as f32 / 255.0;
    let na = (a * alpha * 255.0).round() as u32;
    (color & 0x00ffffff) | (na << 24)
}

fn sample_gradient(g: &LinearGradient, x: f64, y: f64) -> u32 {
    let dx = g.x1 - g.x0;
    let dy = g.y1 - g.y0;
    let len2 = dx * dx + dy * dy;
    let t = if len2 < 1e-9 {
        0.0
    } else {
        ((x - g.x0) * dx + (y - g.y0) * dy) / len2
    };
    let t = t.clamp(0.0, 1.0);
    if g.stops.is_empty() {
        return 0xff000000;
    }
    if g.stops.len() == 1 {
        return g.stops[0].1;
    }
    for win in g.stops.windows(2) {
        if t >= win[0].0 && t <= win[1].0 {
            let span = win[1].0 - win[0].0;
            let u = if span < 1e-9 { 0.0 } else { (t - win[0].0) / span };
            return lerp_color(win[0].1, win[1].1, u as f32);
        }
    }
    g.stops.last().map(|s| s.1).unwrap_or(0xff000000)
}

fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ar = ((a >> 16) & 0xff) as f32;
    let ag = ((a >> 8) & 0xff) as f32;
    let ab = (a & 0xff) as f32;
    let aa = ((a >> 24) & 0xff) as f32;
    let br = ((b >> 16) & 0xff) as f32;
    let bg = ((b >> 8) & 0xff) as f32;
    let bb = (b & 0xff) as f32;
    let ba = ((b >> 24) & 0xff) as f32;
    let r = (ar + (br - ar) * t).round() as u32;
    let g = (ag + (bg - ag) * t).round() as u32;
    let bl = (ab + (bb - ab) * t).round() as u32;
    let al = (aa + (ba - aa) * t).round() as u32;
    (al << 24) | (r << 16) | (g << 8) | bl
}

fn fill_rect_pixels(buf: &mut PixelBuffer, x: f64, y: f64, w: f64, h: f64, color: u32, tf: &Transform) {
    let corners = [
        tf.apply(x, y),
        tf.apply(x + w, y),
        tf.apply(x + w, y + h),
        tf.apply(x, y + h),
    ];
    let min_x = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min).floor() as i32;
    let min_y = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min).floor() as i32;
    let max_x = corners.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    let max_y = corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    for py in min_y..max_y {
        for px in min_x..max_x {
            if point_in_transformed_rect(px as f64 + 0.5, py as f64 + 0.5, x, y, w, h, tf) {
                blend_pixel(buf, px, py, color);
            }
        }
    }
}

fn point_in_transformed_rect(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64, tf: &Transform) -> bool {
    let det = tf.a * tf.d - tf.b * tf.c;
    if det.abs() < 1e-12 {
        return false;
    }
    let lx = px - tf.e;
    let ly = py - tf.f;
    let ux = (tf.d * lx - tf.c * ly) / det;
    let uy = (-tf.b * lx + tf.a * ly) / det;
    ux >= x && ux <= x + w && uy >= y && uy <= y + h
}

fn stroke_line(
    buf: &mut PixelBuffer,
    tf: &Transform,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    width: f64,
    color: u32,
    alpha: f32,
) {
    let (ax, ay) = tf.apply(x0, y0);
    let (bx, by) = tf.apply(x1, y1);
    let col = apply_alpha(color, alpha);
    let hw = (width * 0.5).max(0.5);
    let steps = ((bx - ax).hypot(by - ay)).ceil() as i32 + 1;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let cx = ax + (bx - ax) * t;
        let cy = ay + (by - ay) * t;
        for dy in -(hw as i32)..=(hw as i32) {
            for dx in -(hw as i32)..=(hw as i32) {
                blend_pixel(buf, (cx + dx as f64).round() as i32, (cy + dy as f64).round() as i32, col);
            }
        }
    }
}

fn fill_polygon(buf: &mut PixelBuffer, poly: &[(f64, f64)], tf: &Transform, color: u32, alpha: f32) {
    let col = apply_alpha(color, alpha);
    let pts: Vec<(f64, f64)> = poly.iter().map(|(x, y)| tf.apply(*x, *y)).collect();
    if pts.len() < 3 {
        return;
    }
    let min_y = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min).floor() as i32;
    let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    let min_x = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min).floor() as i32;
    let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max).ceil() as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_polygon(x as f64 + 0.5, y as f64 + 0.5, &pts) {
                blend_pixel(buf, x, y, col);
            }
        }
    }
}

fn point_in_polygon(x: f64, y: f64, poly: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn flatten_path(path: &[PathCmd]) -> Vec<Vec<(f64, f64)>> {
    let mut polys = Vec::new();
    let mut current = Vec::new();
    let mut cursor = (0.0, 0.0);
    let mut start = (0.0, 0.0);

    for cmd in path {
        match cmd {
            PathCmd::Move(x, y) => {
                if current.len() >= 3 {
                    polys.push(current);
                }
                current = vec![(*x, *y)];
                cursor = (*x, *y);
                start = (*x, *y);
            }
            PathCmd::Line(x, y) => {
                if current.is_empty() {
                    current.push(cursor);
                }
                current.push((*x, *y));
                cursor = (*x, *y);
            }
            PathCmd::Arc {
                cx,
                cy,
                r,
                start: sa,
                end,
                ccw,
            } => {
                let segs = tessellate_arc(*cx, *cy, *r, *sa, *end, *ccw);
                for (x, y) in segs {
                    if current.is_empty() {
                        current.push(cursor);
                    }
                    current.push((x, y));
                    cursor = (x, y);
                }
            }
            PathCmd::Close => {
                if !current.is_empty() {
                    current.push(start);
                    polys.push(current);
                    current = Vec::new();
                    cursor = start;
                }
            }
        }
    }
    if current.len() >= 3 {
        polys.push(current);
    }
    polys
}

fn flatten_segments(path: &[PathCmd]) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    let mut cursor = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    for cmd in path {
        match cmd {
            PathCmd::Move(x, y) => {
                cursor = (*x, *y);
                start = (*x, *y);
            }
            PathCmd::Line(x, y) => {
                out.push((cursor.0, cursor.1, *x, *y));
                cursor = (*x, *y);
            }
            PathCmd::Arc {
                cx,
                cy,
                r,
                start: sa,
                end,
                ccw,
            } => {
                let segs = tessellate_arc(*cx, *cy, *r, *sa, *end, *ccw);
                let mut prev = cursor;
                for (x, y) in segs {
                    out.push((prev.0, prev.1, x, y));
                    prev = (x, y);
                }
                cursor = prev;
            }
            PathCmd::Close => {
                out.push((cursor.0, cursor.1, start.0, start.1));
                cursor = start;
            }
        }
    }
    out
}

fn tessellate_arc(cx: f64, cy: f64, r: f64, start: f64, end: f64, ccw: bool) -> Vec<(f64, f64)> {
    let mut a0 = start;
    let mut a1 = end;
    if ccw {
        if a1 > a0 {
            a1 -= 2.0 * PI;
        }
    } else if a1 < a0 {
        a1 += 2.0 * PI;
    }
    let delta = a1 - a0;
    let steps = ((delta.abs() * r) / 4.0).ceil().max(8.0) as usize;
    (0..=steps)
        .map(|i| {
            let t = a0 + delta * (i as f64 / steps as f64);
            (cx + t.cos() * r, cy + t.sin() * r)
        })
        .collect()
}

fn arc_endpoint(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    (cx + angle.cos() * r, cy + angle.sin() * r)
}

fn path_bounds(polys: &[Vec<(f64, f64)>]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for poly in polys {
        for (x, y) in poly {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
    }
    if !min_x.is_finite() {
        return (0.0, 0.0, 1.0, 1.0);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

fn segment_bounds(segs: &[(f64, f64, f64, f64)]) -> (f64, f64, f64, f64) {
    if segs.is_empty() {
        return (0.0, 0.0, 1.0, 1.0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x0, y0, x1, y1) in segs {
        min_x = min_x.min(*x0).min(*x1);
        min_y = min_y.min(*y0).min(*y1);
        max_x = max_x.max(*x0).max(*x1);
        max_y = max_y.max(*y0).max(*y1);
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

fn sample(buf: &PixelBuffer, x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= buf.width as i32 || y >= buf.height as i32 {
        return 0;
    }
    buf.pixels[y as usize * buf.width as usize + x as usize]
}

fn put_pixel(buf: &mut PixelBuffer, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 || x >= buf.width as i32 || y >= buf.height as i32 {
        return;
    }
    buf.pixels[y as usize * buf.width as usize + x as usize] = color;
}

fn blend_pixel(buf: &mut PixelBuffer, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 || x >= buf.width as i32 || y >= buf.height as i32 {
        return;
    }
    let i = y as usize * buf.width as usize + x as usize;
    let dst = buf.pixels[i];
    let sa = ((color >> 24) & 0xff) as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 {
        buf.pixels[i] = color;
        return;
    }
    let sr = ((color >> 16) & 0xff) as u32;
    let sg = ((color >> 8) & 0xff) as u32;
    let sb = (color & 0xff) as u32;
    let da = ((dst >> 24) & 0xff) as u32;
    let dr = ((dst >> 16) & 0xff) as u32;
    let dg = ((dst >> 8) & 0xff) as u32;
    let db = (dst & 0xff) as u32;
    let inv = 255 - sa;
    let a = sa + da * inv / 255;
    let r = (sr * sa + dr * inv) / 255;
    let g = (sg * sa + dg * inv) / 255;
    let b = (sb * sa + db * inv) / 255;
    buf.pixels[i] = (a << 24) | (r << 16) | (g << 8) | b;
}
