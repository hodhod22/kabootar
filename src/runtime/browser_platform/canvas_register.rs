//! Canvas 2D native registration.

use crate::runtime::render::canvas2d;
use crate::runtime::kabootar_dom::DomNode;
use crate::value::{Environment, Value};
use std::collections::HashMap;

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

fn expect_f64(args: &[Value], i: usize) -> Result<f64, String> {
    match args.get(i) {
        Some(Value::Number(n)) => Ok(*n as f64),
        Some(Value::Float(f)) => Ok(*f),
        _ => Err("expected number".into()),
    }
}

fn canvas_id_arg(args: &[Value], i: usize) -> Result<u64, String> {
    match args.get(i) {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        Some(Value::Object(o)) => match o.get("id") {
            Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
            _ => Err("canvas context object missing id".into()),
        },
        _ => Err("expected canvas id or context object".into()),
    }
}

fn dom_node(args: &[Value], i: usize) -> Result<DomNode, String> {
    match args.get(i) {
        Some(Value::KabootarDom(n)) => Ok(n.clone()),
        _ => Err("expected KabootarDom node".into()),
    }
}

fn canvas_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(map_to_object(canvas2d::info()))
}

fn map_to_object(m: HashMap<String, String>) -> Value {
    Value::Object(m.into_iter().map(|(k, v)| (k, Value::String(v))).collect())
}

fn canvas_create_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let w = args.get(0).and_then(|v| match v {
        Value::Number(n) => Some(*n as u32),
        _ => None,
    }).unwrap_or(300);
    let h = args.get(1).and_then(|v| match v {
        Value::Number(n) => Some(*n as u32),
        _ => None,
    }).unwrap_or(150);
    let id = canvas2d::create(w, h)?;
    canvas_value(id)
}

fn canvas_bind_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let node = dom_node(args, 0)?;
    let w = node
        .attributes
        .get("width")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(300);
    let h = node
        .attributes
        .get("height")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(150);
    let id = canvas2d::bind_dom(node.id, w, h)?;
    canvas_value(id)
}

fn canvas_get_context_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let kind = args.get(1).map(|v| match v {
        Value::String(s) => s.as_str(),
        _ => "2d",
    }).unwrap_or("2d");

    let (w, h) = if args.first().is_some_and(|v| matches!(v, Value::KabootarDom(_))) {
        let node = dom_node(args, 0)?;
        let w = node.attributes.get("width").and_then(|s| s.parse().ok()).unwrap_or(300);
        let h = node.attributes.get("height").and_then(|s| s.parse().ok()).unwrap_or(150);
        (w, h)
    } else {
        let id = canvas_id_arg(args, 0)?;
        let (w, h, _) = canvas2d::surface_meta(id).ok_or("invalid canvas")?;
        (w, h)
    };

    if super::webgl_register::is_webgl_kind(kind) {
        return super::webgl_register::create_gl_context(w, h, kind, "native", None);
    }

    if kind != "2d" {
        return Err(format!("unsupported getContext(\"{kind}\")"));
    }

    let id = if args.first().is_some_and(|v| matches!(v, Value::KabootarDom(_))) {
        let node = dom_node(args, 0)?;
        canvas2d::bind_dom(node.id, w, h)?
    } else {
        canvas_id_arg(args, 0)?
    };
    canvas_value(id)
}

fn canvas_value(id: u64) -> Result<Value, String> {
    let (w, h, dom_id) = canvas2d::surface_meta(id).ok_or("invalid canvas")?;
    let mut o = HashMap::new();
    o.insert("__kab_ctx".into(), Value::Bool(true));
    o.insert("id".into(), Value::Number(id as i64));
    o.insert("width".into(), Value::Number(w as i64));
    o.insert("height".into(), Value::Number(h as i64));
    o.insert("kind".into(), Value::String("2d".into()));
    o.insert("layer".into(), Value::String("native".into()));
    if let Some(d) = dom_id {
        o.insert("dom_id".into(), Value::Number(d as i64));
    }
    attach_native_ctx_methods(&mut o);
    Ok(Value::Object(o))
}

/// Build a native 2D context object for an existing canvas surface id.
pub fn native_canvas_context(id: u64) -> Result<Value, String> {
    canvas_value(id)
}

fn attach_native_ctx_methods(o: &mut HashMap<String, Value>) {
    o.insert("fillRect".into(), Value::NativeFunction(canvas_fill_rect_native));
    o.insert("strokeRect".into(), Value::NativeFunction(canvas_stroke_rect_native));
    o.insert("clearRect".into(), Value::NativeFunction(canvas_clear_rect_native));
    o.insert("fillText".into(), Value::NativeFunction(canvas_fill_text_native));
    o.insert("measureText".into(), Value::NativeFunction(canvas_measure_text_native));
    o.insert("beginPath".into(), Value::NativeFunction(canvas_begin_path_native));
    o.insert("moveTo".into(), Value::NativeFunction(canvas_move_to_native));
    o.insert("lineTo".into(), Value::NativeFunction(canvas_line_to_native));
    o.insert("arc".into(), Value::NativeFunction(canvas_arc_native));
    o.insert("closePath".into(), Value::NativeFunction(canvas_close_path_native));
    o.insert("fill".into(), Value::NativeFunction(canvas_fill_native));
    o.insert("stroke".into(), Value::NativeFunction(canvas_stroke_native));
    o.insert("save".into(), Value::NativeFunction(canvas_save_native));
    o.insert("restore".into(), Value::NativeFunction(canvas_restore_native));
    o.insert("translate".into(), Value::NativeFunction(canvas_translate_native));
    o.insert("scale".into(), Value::NativeFunction(canvas_scale_native));
    o.insert("rotate".into(), Value::NativeFunction(canvas_rotate_native));
    o.insert("drawImage".into(), Value::NativeFunction(canvas_draw_image_native));
    o.insert("getImageData".into(), Value::NativeFunction(canvas_get_image_data_native));
    o.insert("putImageData".into(), Value::NativeFunction(canvas_put_image_data_native));
    o.insert("setTransform".into(), Value::NativeFunction(canvas_set_transform_native));
    o.insert("transform".into(), Value::NativeFunction(canvas_transform_native));
    o.insert("resetTransform".into(), Value::NativeFunction(canvas_reset_transform_native));
    o.insert("rect".into(), Value::NativeFunction(canvas_rect_native));
    o.insert("quadraticCurveTo".into(), Value::NativeFunction(canvas_quadratic_curve_to_native));
    o.insert("bezierCurveTo".into(), Value::NativeFunction(canvas_bezier_curve_to_native));
    o.insert("clip".into(), Value::NativeFunction(canvas_clip_native));
    o.insert("toDataURL".into(), Value::NativeFunction(canvas_to_data_url_native));
}

fn canvas_fill_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::fill_rect(id, expect_f64(args, 1)?, expect_f64(args, 2)?, expect_f64(args, 3)?, expect_f64(args, 4)?)?;
    Ok(Value::Null)
}

fn canvas_stroke_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::stroke_rect(id, expect_f64(args, 1)?, expect_f64(args, 2)?, expect_f64(args, 3)?, expect_f64(args, 4)?)?;
    Ok(Value::Null)
}

fn canvas_clear_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::clear_rect(id, expect_f64(args, 1)?, expect_f64(args, 2)?, expect_f64(args, 3)?, expect_f64(args, 4)?)?;
    Ok(Value::Null)
}

fn canvas_set_fill_style_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_fill_style(id, &expect_str(args, 1, "fillStyle")?)?;
    Ok(Value::Null)
}

fn canvas_set_stroke_style_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_stroke_style(id, &expect_str(args, 1, "strokeStyle")?)?;
    Ok(Value::Null)
}

fn canvas_set_global_alpha_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_global_alpha(id, expect_f64(args, 1)?)?;
    Ok(Value::Null)
}

fn canvas_set_line_width_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_line_width(id, expect_f64(args, 1)?)?;
    Ok(Value::Null)
}

fn canvas_set_font_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_font(id, &expect_str(args, 1, "font")?)?;
    Ok(Value::Null)
}

fn canvas_save_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::save(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_restore_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::restore(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_translate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::translate(id, expect_f64(args, 1)?, expect_f64(args, 2)?)?;
    Ok(Value::Null)
}

fn canvas_scale_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::scale(id, expect_f64(args, 1)?, expect_f64(args, 2)?)?;
    Ok(Value::Null)
}

fn canvas_rotate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::rotate(id, expect_f64(args, 1)?)?;
    Ok(Value::Null)
}

fn canvas_begin_path_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::begin_path(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_move_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::move_to(id, expect_f64(args, 1)?, expect_f64(args, 2)?)?;
    Ok(Value::Null)
}

fn canvas_line_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::line_to(id, expect_f64(args, 1)?, expect_f64(args, 2)?)?;
    Ok(Value::Null)
}

fn canvas_arc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::arc(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
        expect_f64(args, 5)?,
        args.get(6).and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        }).unwrap_or(false),
    )?;
    Ok(Value::Null)
}

fn canvas_close_path_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::close_path(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_fill_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::fill(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_stroke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::stroke(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_fill_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::fill_text(id, &expect_str(args, 1, "text")?, expect_f64(args, 2)?, expect_f64(args, 3)?)?;
    Ok(Value::Null)
}

fn canvas_measure_text_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    let (w, h) = canvas2d::measure_text_size(id, &expect_str(args, 1, "text")?)?;
    let mut o = HashMap::new();
    o.insert("width".into(), Value::Float(w as f64));
    o.insert("height".into(), Value::Float(h as f64));
    Ok(Value::Object(o))
}

fn canvas_create_linear_gradient_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::create_linear_gradient(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
    )?;
    Ok(Value::Null)
}

fn canvas_gradient_add_color_stop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::gradient_add_color_stop(id, expect_f64(args, 1)?, &expect_str(args, 2, "color")?)?;
    Ok(Value::Null)
}

fn canvas_draw_image_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let dst = canvas_id_arg(args, 0)?;
    let src = canvas_id_arg(args, 1)?;
    match args.len() {
        4 => {
            canvas2d::draw_image_xy(dst, src, expect_f64(args, 2)?, expect_f64(args, 3)?)?;
        }
        6 => {
            canvas2d::draw_image(
                dst,
                src,
                expect_f64(args, 2)?,
                expect_f64(args, 3)?,
                expect_f64(args, 4)?,
                expect_f64(args, 5)?,
            )?;
        }
        n if n >= 10 => {
            canvas2d::draw_image_rect(
                dst,
                src,
                expect_f64(args, 2)?,
                expect_f64(args, 3)?,
                expect_f64(args, 4)?,
                expect_f64(args, 5)?,
                expect_f64(args, 6)?,
                expect_f64(args, 7)?,
                expect_f64(args, 8)?,
                expect_f64(args, 9)?,
            )?;
        }
        _ => {
            canvas2d::draw_image(
                dst,
                src,
                expect_f64(args, 2)?,
                expect_f64(args, 3)?,
                expect_f64(args, 4).unwrap_or(1.0),
                expect_f64(args, 5).unwrap_or(1.0),
            )?;
        }
    }
    Ok(Value::Null)
}

fn canvas_get_image_data_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    let x = expect_num(args, 1)? as i32;
    let y = expect_num(args, 2)? as i32;
    let w = expect_num(args, 3)? as i32;
    let h = expect_num(args, 4)? as i32;
    let data = canvas2d::get_image_data(id, x, y, w, h)?;
    let mut o = HashMap::new();
    o.insert("width".into(), Value::Number(w as i64));
    o.insert("height".into(), Value::Number(h as i64));
    o.insert(
        "data".into(),
        Value::Array(data.into_iter().map(|b| Value::Number(b as i64)).collect()),
    );
    Ok(Value::Object(o))
}

fn canvas_put_image_data_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    let (data, w, h) = match args.get(1) {
        Some(Value::Object(o)) => {
            let w = match o.get("width") {
                Some(Value::Number(n)) => *n as i32,
                _ => return Err("putImageData: ImageData.width required".into()),
            };
            let h = match o.get("height") {
                Some(Value::Number(n)) => *n as i32,
                _ => return Err("putImageData: ImageData.height required".into()),
            };
            let data = match o.get("data") {
                Some(Value::Array(items)) => items
                    .iter()
                    .map(|v| match v {
                        Value::Number(n) => Ok::<u8, String>(*n as u8),
                        _ => Err("putImageData: data must be number array".into()),
                    })
                    .collect::<Result<Vec<u8>, String>>()?,
                _ => return Err("putImageData: ImageData.data required".into()),
            };
            (data, w, h)
        }
        Some(Value::Array(items)) => {
            let w = expect_num(args, 4).unwrap_or(0) as i32;
            let h = expect_num(args, 5).unwrap_or(0) as i32;
            let data = items
                .iter()
                .map(|v| match v {
                    Value::Number(n) => Ok::<u8, String>(*n as u8),
                    _ => Err("putImageData: data must be number array".into()),
                })
                .collect::<Result<Vec<u8>, String>>()?;
            (data, w, h)
        }
        _ => return Err("putImageData expects ImageData object".into()),
    };
    let dx = expect_num(args, 2)? as i32;
    let dy = expect_num(args, 3)? as i32;
    canvas2d::put_image_data(id, &data, dx, dy, w, h)?;
    Ok(Value::Null)
}

fn canvas_set_transform_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::set_transform(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
        expect_f64(args, 5)?,
        expect_f64(args, 6)?,
    )?;
    Ok(Value::Null)
}

fn canvas_rect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::rect_path(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
    )?;
    Ok(Value::Null)
}

fn canvas_transform_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::transform_multiply(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
        expect_f64(args, 5)?,
        expect_f64(args, 6)?,
    )?;
    Ok(Value::Null)
}

fn canvas_reset_transform_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::reset_transform(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_quadratic_curve_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::quadratic_curve_to(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
    )?;
    Ok(Value::Null)
}

fn canvas_bezier_curve_to_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    canvas2d::bezier_curve_to(
        id,
        expect_f64(args, 1)?,
        expect_f64(args, 2)?,
        expect_f64(args, 3)?,
        expect_f64(args, 4)?,
        expect_f64(args, 5)?,
        expect_f64(args, 6)?,
    )?;
    Ok(Value::Null)
}

fn canvas_clip_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    canvas2d::clip_path(canvas_id_arg(args, 0)?)?;
    Ok(Value::Null)
}

fn canvas_to_data_url_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    let mime = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => "image/png",
    };
    Ok(Value::String(canvas2d::to_data_url(id, mime)?))
}

fn canvas_to_pixels_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = canvas_id_arg(args, 0)?;
    let bytes = canvas2d::to_rgba_bytes(id)?;
    Ok(Value::Array(bytes.into_iter().map(|b| Value::Number(b as i64)).collect()))
}

pub fn register_canvas(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("canvas_info", canvas_info_native),
        ("canvas_create", canvas_create_native),
        ("canvas_bind", canvas_bind_native),
        ("canvas_get_context", canvas_get_context_native),
        ("canvas_fill_rect", canvas_fill_rect_native),
        ("canvas_stroke_rect", canvas_stroke_rect_native),
        ("canvas_clear_rect", canvas_clear_rect_native),
        ("canvas_set_fill_style", canvas_set_fill_style_native),
        ("canvas_set_stroke_style", canvas_set_stroke_style_native),
        ("canvas_set_global_alpha", canvas_set_global_alpha_native),
        ("canvas_set_line_width", canvas_set_line_width_native),
        ("canvas_set_font", canvas_set_font_native),
        ("canvas_save", canvas_save_native),
        ("canvas_restore", canvas_restore_native),
        ("canvas_translate", canvas_translate_native),
        ("canvas_scale", canvas_scale_native),
        ("canvas_rotate", canvas_rotate_native),
        ("canvas_begin_path", canvas_begin_path_native),
        ("canvas_move_to", canvas_move_to_native),
        ("canvas_line_to", canvas_line_to_native),
        ("canvas_arc", canvas_arc_native),
        ("canvas_close_path", canvas_close_path_native),
        ("canvas_fill", canvas_fill_native),
        ("canvas_stroke", canvas_stroke_native),
        ("canvas_fill_text", canvas_fill_text_native),
        ("canvas_measure_text", canvas_measure_text_native),
        ("canvas_create_linear_gradient", canvas_create_linear_gradient_native),
        ("canvas_gradient_add_color_stop", canvas_gradient_add_color_stop_native),
        ("canvas_draw_image", canvas_draw_image_native),
        ("canvas_get_image_data", canvas_get_image_data_native),
        ("canvas_put_image_data", canvas_put_image_data_native),
        ("canvas_set_transform", canvas_set_transform_native),
        ("canvas_transform", canvas_transform_native),
        ("canvas_reset_transform", canvas_reset_transform_native),
        ("canvas_rect", canvas_rect_native),
        ("canvas_quadratic_curve_to", canvas_quadratic_curve_to_native),
        ("canvas_bezier_curve_to", canvas_bezier_curve_to_native),
        ("canvas_clip", canvas_clip_native),
        ("canvas_to_data_url", canvas_to_data_url_native),
        ("canvas_to_pixels", canvas_to_pixels_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
