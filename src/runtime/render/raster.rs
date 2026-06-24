//! CPU/GPU-ready pixel compositor — rasterizes layout trees to RGBA buffers.

use crate::runtime::render::canvas2d;
use crate::runtime::kabootar_dom::DomNode;
use crate::runtime::kstyle::{compute_style, Stylesheet};
use crate::runtime::render::layout::LayoutBox;
use crate::runtime::render::text::{paint_text, TextStyle};

#[derive(Debug, Clone)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    /// Packed 0xAARRGGBB pixels, row-major.
    pub pixels: Vec<u32>,
}

impl PixelBuffer {
    pub fn new(width: u32, height: u32, clear: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![clear; (width * height) as usize],
        }
    }

    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for px in &self.pixels {
            let a = ((px >> 24) & 0xff) as u8;
            let r = ((px >> 16) & 0xff) as u8;
            let g = ((px >> 8) & 0xff) as u8;
            let b = (px & 0xff) as u8;
            out.extend_from_slice(&[r, g, b, a]);
        }
        out
    }
}

pub fn rasterize_tree(
    root: &DomNode,
    layout: &LayoutBox,
    sheet: &Stylesheet,
    width: u32,
    height: u32,
) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height, parse_color("#202124"));
    raster_walk(root, layout, sheet, &mut buf, &compute_style(&root.tag, &root.attributes, sheet));
    buf
}

fn raster_walk(
    node: &DomNode,
    layout: &LayoutBox,
    sheet: &Stylesheet,
    buf: &mut PixelBuffer,
    inherited: &crate::runtime::kstyle::ComputedStyle,
) {
    if node.tag == "#text" {
        let style = compute_style("span", &node.attributes, sheet);
        let merged = merge_text_style(inherited, &style);
        let text_style = TextStyle::from_computed(&merged, None);
        if let Some(tl) = &layout.text_layout {
            paint_text(buf, tl, layout.x as f32, layout.y as f32, &text_style);
        }
        return;
    }

    let style = compute_style(&node.tag, &node.attributes, sheet);
    let bg = parse_color(if style.background == "transparent" {
        "#00000000"
    } else {
        &style.background
    });
    if (bg >> 24) != 0 {
        fill_round_rect(
            buf,
            layout.x as i32,
            layout.y as i32,
            layout.w as i32,
            layout.h as i32,
            parse_px_simple(&style.border_radius, 0),
            bg,
        );
    }

    if node.tag == "canvas" {
        canvas2d::blit_dom_canvas(
            buf,
            node.id,
            layout.x as i32,
            layout.y as i32,
            layout.w as i32,
            layout.h as i32,
        );
        return;
    }

    for (child, child_layout) in node.children.iter().zip(layout.children.iter()) {
        raster_walk(child, child_layout, sheet, buf, &style);
    }
}

fn merge_text_style(
    parent: &crate::runtime::kstyle::ComputedStyle,
    node: &crate::runtime::kstyle::ComputedStyle,
) -> crate::runtime::kstyle::ComputedStyle {
    let mut m = parent.clone();
    if node.color != "#e8eaed" {
        m.color = node.color.clone();
    }
    if node.font_size != "16px" {
        m.font_size = node.font_size.clone();
    }
    if node.font_weight != "400" {
        m.font_weight = node.font_weight.clone();
    }
    if node.line_height != "normal" {
        m.line_height = node.line_height.clone();
    }
    if node.white_space != "normal" {
        m.white_space = node.white_space.clone();
    }
    m
}

fn fill_round_rect(buf: &mut PixelBuffer, x: i32, y: i32, w: i32, h: i32, _r: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(buf.width as i32);
    let y1 = (y + h).min(buf.height as i32);
    for py in y0..y1 {
        for px in x0..x1 {
            buf.pixels[py as usize * buf.width as usize + px as usize] = color;
        }
    }
}

pub fn parse_color(s: &str) -> u32 {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            }
            8 => {
                let a = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                let r = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[6..8], 16).unwrap_or(0);
                ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            }
            _ => 0xff202124,
        }
    } else {
        0xff202124
    }
}

fn parse_px_simple(s: &str, default: i32) -> i32 {
    let s = s.trim();
    if s.ends_with("px") {
        s[..s.len() - 2].trim().parse().unwrap_or(default)
    } else {
        default
    }
}
