//! Kabootar text layout — measurement, word wrap, line-height, TTF via fontdue when available.

use crate::runtime::kstyle::ComputedStyle;
use crate::runtime::render::raster::{parse_color, PixelBuffer};
use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle as FdTextStyle, WrapStyle,
};
use fontdue::Font;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteSpace {
    Normal,
    Nowrap,
    PreWrap,
}

#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub max_width: Option<f32>,
    pub white_space: WhiteSpace,
    pub color: u32,
}

impl TextStyle {
    pub fn from_computed(style: &ComputedStyle, max_width: Option<f32>) -> Self {
        let font_size = parse_px_f32(&style.font_size, 16.0);
        Self {
            font_size,
            line_height: parse_line_height(&style.line_height, font_size),
            max_width,
            white_space: parse_white_space(&style.white_space),
            color: parse_color(&style.color),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub ch: char,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct TextLayoutResult {
    pub width: f32,
    pub height: f32,
    pub lines: usize,
    pub glyphs: Vec<PositionedGlyph>,
    pub used_ttf: bool,
}

struct TextEngine {
    ttf: Option<Arc<Font>>,
    layout: Layout,
}

static ENGINE: OnceLock<Mutex<TextEngine>> = OnceLock::new();

fn engine() -> &'static Mutex<TextEngine> {
    ENGINE.get_or_init(|| {
        Mutex::new(TextEngine {
            ttf: load_external_font(),
            layout: Layout::new(CoordinateSystem::PositiveYDown),
        })
    })
}

fn load_external_font() -> Option<Arc<Font>> {
    if let Ok(path) = std::env::var("KABOOTAR_FONT") {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(f) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                return Some(Arc::new(f));
            }
        }
    }
    let assets = Path::new("assets/fonts/KabootarUI.ttf");
    if assets.exists() {
        if let Ok(meta) = std::fs::metadata(assets) {
            if meta.len() > 500 {
                if let Ok(bytes) = std::fs::read(assets) {
                    if let Ok(f) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                        return Some(Arc::new(f));
                    }
                }
            }
        }
    }
    None
}

pub fn layout_text(text: &str, style: &TextStyle) -> TextLayoutResult {
    let mut guard = engine().lock().expect("text engine lock");
    if let Some(font) = guard.ttf.clone() {
        if let Some(r) = layout_with_fontdue(text, style, font.as_ref(), &mut guard.layout) {
            return r;
        }
    }
    layout_builtin(text, style)
}

pub fn measure_text(text: &str, style: &TextStyle) -> (f32, f32) {
    let r = layout_text(text, style);
    (r.width, r.height)
}

pub fn paint_text(
    buf: &mut PixelBuffer,
    layout: &TextLayoutResult,
    origin_x: f32,
    origin_y: f32,
    style: &TextStyle,
) {
    if layout.used_ttf {
        if let Ok(guard) = engine().lock() {
            if let Some(font) = guard.ttf.as_ref() {
                paint_fontdue(buf, font.as_ref(), layout, origin_x, origin_y, style.color, style.font_size);
                return;
            }
        }
    }
    paint_builtin(buf, layout, origin_x, origin_y, style);
}

fn layout_with_fontdue(
    text: &str,
    style: &TextStyle,
    font: &Font,
    layout: &mut Layout,
) -> Option<TextLayoutResult> {
    let wrap = match style.white_space {
        WhiteSpace::Nowrap => None,
        _ => style.max_width,
    };
    let settings = LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: wrap,
        max_height: None,
        horizontal_align: HorizontalAlign::Left,
        vertical_align: fontdue::layout::VerticalAlign::Top,
        line_height: style.line_height,
        wrap_style: WrapStyle::Word,
        wrap_hard_breaks: true,
    };
    layout.reset(&settings);
    layout.append(&[font], &FdTextStyle::new(text, style.font_size, 0));
    let glyphs: Vec<PositionedGlyph> = layout
        .glyphs()
        .iter()
        .filter_map(|g| {
            char::from_u32(g.parent as u32).map(|ch| PositionedGlyph {
                ch,
                x: g.x,
                y: g.y,
            })
        })
        .collect();
    if glyphs.is_empty() && !text.trim().is_empty() {
        return None;
    }
    let width = glyphs
        .iter()
        .map(|g| g.x + advance(g.ch, style.font_size))
        .fold(0.0f32, f32::max);
    Some(TextLayoutResult {
        width: width.max(1.0),
        height: layout.height().max(style.font_size * style.line_height),
        lines: layout.lines().map(|l| l.len()).unwrap_or(1).max(1),
        glyphs,
        used_ttf: true,
    })
}

fn paint_fontdue(
    buf: &mut PixelBuffer,
    font: &Font,
    layout: &TextLayoutResult,
    ox: f32,
    oy: f32,
    color: u32,
    px: f32,
) {
    let (fr, fg, fb, fa) = rgba_parts(color);
    for g in &layout.glyphs {
        let (metrics, bitmap) = font.rasterize(g.ch, px);
        let gx = (ox + g.x + metrics.xmin as f32).round() as i32;
        let gy = (oy + g.y + metrics.ymin as f32).round() as i32;
        let w = metrics.width;
        let h = metrics.height;
        for row in 0..h {
            for col in 0..w {
                let alpha = bitmap[row * w + col];
                if alpha == 0 {
                    continue;
                }
                blend_pixel(buf, gx + col as i32, gy + row as i32, fr, fg, fb, fa, alpha);
            }
        }
    }
}

fn paint_builtin(buf: &mut PixelBuffer, layout: &TextLayoutResult, ox: f32, oy: f32, style: &TextStyle) {
    let scale = style.font_size / 16.0;
    for g in &layout.glyphs {
        draw_builtin_glyph(
            buf,
            (ox + g.x).round() as i32,
            (oy + g.y).round() as i32,
            g.ch,
            style.color,
            scale,
        );
    }
}

fn layout_builtin(text: &str, style: &TextStyle) -> TextLayoutResult {
    let line_h = style.font_size * style.line_height;
    let mut glyphs = Vec::new();
    let mut line_count = 0usize;
    let mut max_w = 0.0f32;
    let mut y = 0.0f32;

    let paragraphs: Vec<&str> = match style.white_space {
        WhiteSpace::PreWrap => text.split('\n').collect(),
        _ => vec![text],
    };

    for (pi, para) in paragraphs.iter().enumerate() {
        if pi > 0 {
            y += line_h;
            line_count += 1;
        }
        let word_lines: Vec<Vec<&str>> = match style.white_space {
            WhiteSpace::Nowrap => vec![para.split_whitespace().collect()],
            _ if style.max_width.is_some() => wrap_words(para, style.max_width.unwrap(), style.font_size),
            _ => vec![para.split_whitespace().collect()],
        };

        for (li, words) in word_lines.iter().enumerate() {
            if li > 0 {
                y += line_h;
                line_count += 1;
            }
            let mut x = 0.0f32;
            for (wi, word) in words.iter().enumerate() {
                if wi > 0 {
                    x += advance(' ', style.font_size);
                }
                for ch in word.chars() {
                    glyphs.push(PositionedGlyph { ch, x, y });
                    x += advance(ch, style.font_size);
                }
            }
            max_w = max_w.max(x);
            if !words.is_empty() || para.is_empty() {
                line_count += 1;
            }
        }
    }

    if line_count == 0 && !text.is_empty() {
        line_count = 1;
    }

    TextLayoutResult {
        width: max_w,
        height: if glyphs.is_empty() {
            line_h
        } else {
            y + line_h
        },
        lines: line_count.max(1),
        glyphs,
        used_ttf: false,
    }
}

fn wrap_words(para: &str, max_width: f32, font_size: f32) -> Vec<Vec<&str>> {
    let words: Vec<&str> = para.split_whitespace().collect();
    if words.is_empty() {
        return vec![vec![]];
    }
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut line_w = 0.0f32;
    let space = advance(' ', font_size);

    for word in words {
        let word_w = measure_word(word, font_size);
        let add = if current.is_empty() { word_w } else { space + word_w };
        if !current.is_empty() && line_w + add > max_width {
            lines.push(current);
            current = vec![word];
            line_w = word_w;
        } else {
            line_w += add;
            current.push(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn measure_word(word: &str, font_size: f32) -> f32 {
    word.chars().map(|c| advance(c, font_size)).sum()
}

pub fn advance(ch: char, font_size: f32) -> f32 {
    builtin_advance_16px(ch) * font_size / 16.0
}

fn builtin_advance_16px(ch: char) -> f32 {
    match ch {
        'i' | 'l' | 'j' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' => 4.0,
        'f' | 't' | 'r' => 5.0,
        'a' | 'c' | 'e' | 'g' | 'k' | 'o' | 's' | 'v' | 'x' | 'y' | 'z' => 7.0,
        'b' | 'd' | 'h' | 'n' | 'p' | 'q' | 'u' => 8.0,
        'm' | 'w' => 12.0,
        'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'K' | 'L' | 'N' | 'P' | 'R' | 'S' | 'T'
        | 'U' | 'V' | 'X' | 'Y' | 'Z' => 9.0,
        'M' | 'W' | 'O' | 'Q' => 11.0,
        '0'..='9' => 8.0,
        ' ' => 4.0,
        '-' | '+' | '=' => 6.0,
        _ => 8.0,
    }
}

fn draw_builtin_glyph(buf: &mut PixelBuffer, x: i32, y: i32, ch: char, color: u32, scale: f32) {
    let rows = builtin_glyph_rows(ch);
    let scale_i = scale.max(0.5).round() as i32;
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..7 {
            if bits & (1 << (6 - col)) != 0 {
                for sy in 0..scale_i {
                    for sx in 0..scale_i {
                        set_pixel(buf, x + col * scale_i + sx, y + row as i32 * scale_i + sy, color);
                    }
                }
            }
        }
    }
}

fn builtin_glyph_rows(ch: char) -> [u8; 12] {
    match ch {
        'A'..='Z' => letter_glyph((ch as u8) - b'A'),
        'a'..='z' => letter_glyph((ch as u8 - b'a') % 26),
        '0'..='9' => digit_glyph((ch as u8) - b'0'),
        ' ' => [0; 12],
        '.' => [0, 0, 0, 0, 0, 0, 0, 0, 0x18, 0x18, 0, 0],
        ':' => [0, 0x18, 0x18, 0, 0, 0, 0x18, 0x18, 0, 0, 0, 0],
        '-' => [0, 0, 0, 0, 0x7e, 0x7e, 0, 0, 0, 0, 0, 0],
        '!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0, 0x18, 0x18, 0, 0, 0, 0],
        _ => [0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0, 0, 0, 0],
    }
}

fn letter_glyph(i: u8) -> [u8; 12] {
    const GLYPHS: [[u8; 12]; 26] = [
        [0x3c, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x7c, 0x66, 0x66, 0x7c, 0x66, 0x66, 0x66, 0x7c, 0, 0, 0, 0],
        [0x3c, 0x66, 0x60, 0x60, 0x60, 0x60, 0x66, 0x3c, 0, 0, 0, 0],
        [0x78, 0x6c, 0x66, 0x66, 0x66, 0x66, 0x6c, 0x78, 0, 0, 0, 0],
        [0x7e, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x60, 0x7e, 0, 0, 0, 0],
        [0x7e, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x60, 0x60, 0, 0, 0, 0],
        [0x3c, 0x66, 0x60, 0x6e, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x66, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x66, 0, 0, 0, 0],
        [0x3c, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0, 0, 0, 0],
        [0x1e, 0x0c, 0x0c, 0x0c, 0x0c, 0x6c, 0x6c, 0x38, 0, 0, 0, 0],
        [0x66, 0x6c, 0x78, 0x70, 0x78, 0x6c, 0x66, 0x66, 0, 0, 0, 0],
        [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7e, 0, 0, 0, 0],
        [0x63, 0x77, 0x7f, 0x6b, 0x63, 0x63, 0x63, 0x63, 0, 0, 0, 0],
        [0x66, 0x76, 0x7e, 0x7e, 0x6e, 0x66, 0x66, 0x66, 0, 0, 0, 0],
        [0x3c, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60, 0x60, 0x60, 0, 0, 0, 0],
        [0x3c, 0x66, 0x66, 0x66, 0x66, 0x6a, 0x6c, 0x36, 0, 0, 0, 0],
        [0x7c, 0x66, 0x66, 0x7c, 0x6c, 0x66, 0x66, 0x66, 0, 0, 0, 0],
        [0x3c, 0x66, 0x60, 0x3c, 0x06, 0x06, 0x66, 0x3c, 0, 0, 0, 0],
        [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0, 0, 0, 0],
        [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x18, 0, 0, 0, 0],
        [0x63, 0x63, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x63, 0, 0, 0, 0],
        [0x66, 0x66, 0x3c, 0x18, 0x3c, 0x66, 0x66, 0x66, 0, 0, 0, 0],
        [0x66, 0x66, 0x66, 0x3c, 0x18, 0x18, 0x18, 0x18, 0, 0, 0, 0],
        [0x7e, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x60, 0x7e, 0, 0, 0, 0],
    ];
    GLYPHS[i as usize % 26]
}

fn digit_glyph(d: u8) -> [u8; 12] {
    const DIGITS: [[u8; 12]; 10] = [
        [0x3c, 0x66, 0x6e, 0x76, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7e, 0, 0, 0, 0],
        [0x3c, 0x66, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x7e, 0, 0, 0, 0],
        [0x3c, 0x66, 0x06, 0x1c, 0x06, 0x06, 0x66, 0x3c, 0, 0, 0, 0],
        [0x0c, 0x1c, 0x2c, 0x4c, 0x7e, 0x0c, 0x0c, 0x0c, 0, 0, 0, 0],
        [0x7e, 0x60, 0x7c, 0x06, 0x06, 0x06, 0x66, 0x3c, 0, 0, 0, 0],
        [0x3c, 0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x7e, 0x06, 0x0c, 0x18, 0x30, 0x30, 0x30, 0x30, 0, 0, 0, 0],
        [0x3c, 0x66, 0x66, 0x3c, 0x66, 0x66, 0x66, 0x3c, 0, 0, 0, 0],
        [0x3c, 0x66, 0x66, 0x66, 0x3e, 0x06, 0x06, 0x3c, 0, 0, 0, 0],
    ];
    DIGITS[d as usize % 10]
}

fn rgba_parts(color: u32) -> (u8, u8, u8, u8) {
    (
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
        ((color >> 24) & 0xff) as u8,
    )
}

fn blend_pixel(buf: &mut PixelBuffer, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8, cov: u8) {
    if x < 0 || y < 0 || x >= buf.width as i32 || y >= buf.height as i32 {
        return;
    }
    let alpha = (a as u16 * cov as u16 / 255) as u8;
    if alpha == 0 {
        return;
    }
    let idx = y as usize * buf.width as usize + x as usize;
    let dst = buf.pixels[idx];
    let (dr, dg, db, da) = rgba_parts(dst);
    let inv = 255 - alpha;
    let nr = (r as u16 * alpha as u16 + dr as u16 * inv as u16) / 255;
    let ng = (g as u16 * alpha as u16 + dg as u16 * inv as u16) / 255;
    let nb = (b as u16 * alpha as u16 + db as u16 * inv as u16) / 255;
    let na = (alpha as u16 + da as u16 * inv as u16 / 255).min(255) as u8;
    buf.pixels[idx] = ((na as u32) << 24) | ((nr as u32) << 16) | ((ng as u32) << 8) | nb as u32;
}

fn set_pixel(buf: &mut PixelBuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < buf.width as i32 && y < buf.height as i32 {
        buf.pixels[y as usize * buf.width as usize + x as usize] = color;
    }
}

fn parse_px_f32(s: &str, default: f32) -> f32 {
    let s = s.trim();
    if s.ends_with("px") {
        s[..s.len() - 2].trim().parse().unwrap_or(default)
    } else {
        s.parse().unwrap_or(default)
    }
}

fn parse_line_height(s: &str, font_size: f32) -> f32 {
    let s = s.trim();
    if s == "normal" || s.is_empty() {
        return 1.25;
    }
    if s.ends_with("px") {
        let px: f32 = s[..s.len() - 2].trim().parse().unwrap_or(font_size * 1.25);
        return (px / font_size).max(1.0);
    }
    if s.ends_with('%') {
        let pct: f32 = s[..s.len() - 1].trim().parse().unwrap_or(125.0);
        return (pct / 100.0).max(1.0);
    }
    s.parse::<f32>().unwrap_or(1.25).max(1.0)
}

fn parse_white_space(s: &str) -> WhiteSpace {
    match s.trim().to_ascii_lowercase().as_str() {
        "nowrap" => WhiteSpace::Nowrap,
        "pre-wrap" | "prewrap" => WhiteSpace::PreWrap,
        _ => WhiteSpace::Normal,
    }
}

pub fn text_layout_to_object(
    r: &TextLayoutResult,
) -> std::collections::HashMap<String, crate::value::Value> {
    use crate::value::Value;
    let mut m = std::collections::HashMap::new();
    m.insert("width".into(), Value::Float(r.width as f64));
    m.insert("height".into(), Value::Float(r.height as f64));
    m.insert("lines".into(), Value::Number(r.lines as i64));
    m.insert("glyphs".into(), Value::Number(r.glyphs.len() as i64));
    m.insert(
        "engine".into(),
        Value::String(if r.used_ttf { "ttf".into() } else { "builtin".into() }),
    );
    m
}
