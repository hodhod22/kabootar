//! Kabootar Render Engine — layout, paint, raster compositor (layer 2 core).

mod backend;
pub mod canvas2d;
mod layout;
mod paint;
mod raster;
mod gpu;
pub mod gpu3d;
pub(crate) mod math3d;
mod text;

pub use math3d::{mat4_identity, mat4_look_at, mat4_mul, mat4_perspective, mat4_rotate_y, mat4_transform, mat4_translate, Mat4};

pub use backend::{active_backend, set_backend, RenderBackend};
pub use canvas2d::{bind_dom, blit_dom_canvas, canvas_id_for_dom, create, info as canvas_info, surface_meta, to_rgba_bytes};
pub use layout::{LayoutBox, LayoutEngine};
pub use paint::{paint_frame_html, paint_text_preview};
pub use raster::{PixelBuffer, rasterize_tree};
pub use gpu::{gpu_available, gpu_info_map, probe_gpu, upload_rgba};
pub use text::{layout_text, measure_text, text_layout_to_object, TextStyle, WhiteSpace};

use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kstyle::Stylesheet;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompositorFrame {
    pub width: i64,
    pub height: i64,
    pub html: String,
    pub text_preview: String,
    pub node_count: usize,
    pub layers: Vec<RenderLayer>,
    pub pixels_rgba: Vec<u8>,
    pub backend: String,
    pub gpu_handle: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RenderLayer {
    pub node_id: u64,
    pub tag: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub z: i32,
}

pub struct RenderEngine {
    pub viewport_w: f64,
    pub viewport_h: f64,
    pub stylesheet: Stylesheet,
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self {
            viewport_w: 1280.0,
            viewport_h: 720.0,
            stylesheet: Stylesheet::default(),
        }
    }
}

impl RenderEngine {
    pub fn with_viewport(w: f64, h: f64) -> Self {
        Self {
            viewport_w: w,
            viewport_h: h,
            ..Self::default()
        }
    }

    pub fn set_stylesheet(&mut self, sheet: Stylesheet) {
        self.stylesheet = sheet;
    }

    pub fn compose(&self, root: &DomNode) -> CompositorFrame {
        let layout = LayoutEngine::layout(root, &self.stylesheet, self.viewport_w);
        let layers = flatten_layers(&layout, 0);
        let html = paint_frame_html(root, &self.stylesheet, &layout, self.viewport_w, self.viewport_h);
        let text_preview = paint_text_preview(root, &layout);
        let node_count = count_nodes(root);
        let pb = rasterize_tree(
            root,
            &layout,
            &self.stylesheet,
            self.viewport_w as u32,
            self.viewport_h as u32,
        );
        let fw = self.viewport_w as u32;
        let fh = self.viewport_h as u32;
        let pixels_rgba = pb.to_rgba_bytes();
        let requested = active_backend();
        let (backend, gpu_handle) = if requested == RenderBackend::Gpu {
            match upload_rgba(fw, fh, &pixels_rgba) {
                Ok(id) => (RenderBackend::Gpu.as_str().to_string(), Some(id)),
                Err(_) => (RenderBackend::Cpu.as_str().to_string(), None),
            }
        } else {
            (RenderBackend::Cpu.as_str().to_string(), None)
        };
        CompositorFrame {
            width: self.viewport_w as i64,
            height: self.viewport_h as i64,
            html,
            text_preview,
            node_count,
            layers,
            pixels_rgba,
            backend,
            gpu_handle,
        }
    }
}

fn count_nodes(node: &DomNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn flatten_layers(boxes: &LayoutBox, z: i32) -> Vec<RenderLayer> {
    let mut out = vec![RenderLayer {
        node_id: boxes.node_id,
        tag: boxes.tag.clone(),
        x: boxes.x,
        y: boxes.y,
        w: boxes.w,
        h: boxes.h,
        z,
    }];
    for child in &boxes.children {
        out.extend(flatten_layers(child, z + 1));
    }
    out
}

pub fn frame_to_object(frame: &CompositorFrame) -> HashMap<String, crate::value::Value> {
    use crate::value::Value;
    let mut m = HashMap::new();
    m.insert("width".into(), Value::Number(frame.width));
    m.insert("height".into(), Value::Number(frame.height));
    m.insert("html".into(), Value::String(frame.html.clone()));
    m.insert("text".into(), Value::String(frame.text_preview.clone()));
    m.insert("nodes".into(), Value::Number(frame.node_count as i64));
    m.insert(
        "pixels".into(),
        Value::Number(frame.pixels_rgba.len() as i64),
    );
    m.insert("backend".into(), Value::String(frame.backend.clone()));
    m.insert(
        "gpu".into(),
        Value::Number(frame.gpu_handle.unwrap_or(0) as i64),
    );
    m.insert(
        "layers".into(),
        Value::Array(
            frame
                .layers
                .iter()
                .map(|l| {
                    let mut o = HashMap::new();
                    o.insert("id".into(), Value::Number(l.node_id as i64));
                    o.insert("tag".into(), Value::String(l.tag.clone()));
                    o.insert("x".into(), Value::Float(l.x));
                    o.insert("y".into(), Value::Float(l.y));
                    o.insert("w".into(), Value::Float(l.w));
                    o.insert("h".into(), Value::Float(l.h));
                    o.insert("z".into(), Value::Number(l.z as i64));
                    Value::Object(o)
                })
                .collect(),
        ),
    );
    m
}
