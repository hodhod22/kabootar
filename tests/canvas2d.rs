//! Advanced HTML Canvas 2D — paths, gradients, transforms, compositor.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::format_value;

fn eval(code: &str) -> String {
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn canvas_create_and_fill_rect() {
    let out = eval(
        r##"
        let ctx = canvas_create(64, 64);
        canvas_set_fill_style(ctx, "#ff0000");
        canvas_fill_rect(ctx, 0, 0, 32, 32);
        len(canvas_to_pixels(ctx))
    "##,
    );
    assert_eq!(out, "16384");
}

#[test]
fn canvas_paths_and_gradient() {
    assert_eq!(
        eval(
            r##"
            let ctx = canvas_create(100, 100);
            canvas_create_linear_gradient(ctx, 0, 0, 100, 0);
            canvas_gradient_add_color_stop(ctx, 0, "#000000");
            canvas_gradient_add_color_stop(ctx, 1, "#ffffff");
            canvas_fill_rect(ctx, 0, 0, 100, 100);
            canvas_set_stroke_style(ctx, "#00ff00");
            canvas_set_line_width(ctx, 4);
            canvas_begin_path(ctx);
            canvas_arc(ctx, 50, 50, 40, 0, 6.28, false);
            canvas_stroke(ctx);
            ctx["width"]
        "##,
        ),
        "100"
    );
}

#[test]
fn canvas_dom_compositor() {
    assert_eq!(
        eval(
            r##"
            let page = kdom_create("div");
            let c = kdom_create("canvas");
            kdom_set_attr(c, "width", "80");
            kdom_set_attr(c, "height", "60");
            kdom_append(page, c);
            let ctx = canvas_get_context(c, "2d");
            canvas_set_fill_style(ctx, "#3366cc");
            canvas_fill_rect(ctx, 0, 0, 80, 60);
            canvas_fill_text(ctx, "Hi", 10, 30);
            kb_mount(page);
            kb_viewport(200, 200);
            let frame = kb_paint();
            frame["width"] > 0
        "##,
        ),
        "true"
    );
}

#[test]
fn canvas_transform_and_draw_image() {
    assert_eq!(
        eval(
            r##"
            let a = canvas_create(20, 20);
            canvas_set_fill_style(a, "#ff00ff");
            canvas_fill_rect(a, 0, 0, 20, 20);
            let b = canvas_create(40, 40);
            canvas_draw_image(b, a, 5, 5, 20, 20);
            canvas_save(b);
            canvas_translate(b, 10, 10);
            canvas_scale(b, 2, 2);
            canvas_set_fill_style(b, "#0000ff");
            canvas_fill_rect(b, 0, 0, 5, 5);
            canvas_restore(b);
            b["height"]
        "##,
        ),
        "40"
    );
}

#[test]
fn canvas_js_syntax_fill_style_property() {
    let out = eval(
        r##"
        let ctx = canvas_create(64, 64);
        ctx.fillStyle = "#ff0000";
        ctx.fillRect(0, 0, 32, 32);
        ctx.fillStyle
    "##,
    );
    assert_eq!(out, "#ff0000");
}

#[test]
fn canvas_host_fill_style_property() {
    let out = eval(
        r##"
        let ctx = document.createElement("canvas").getContext("2d");
        ctx.fillStyle = "#3366cc";
        ctx.fillRect(0, 0, 10, 10);
        ctx.fillStyle
    "##,
    );
    assert_eq!(out, "#3366cc");
}

#[test]
fn canvas_info_reports_advanced_api() {
    let info = eval("canvas_info()");
    assert!(info.contains("canvas-2d-advanced"));
}

#[test]
fn canvas_js_syntax_fill_rect() {
    let out = eval(
        r##"
        let ctx = canvas_create(64, 64);
        canvas_set_fill_style(ctx, "#ff0000");
        ctx.fillRect(0, 0, 32, 32);
        len(canvas_to_pixels(ctx))
    "##,
    );
    assert_eq!(out, "16384");
}

#[test]
fn canvas_host_create_element_get_context() {
    let out = eval(
        r##"
        let canvas = document.createElement("canvas");
        let ctx = canvas.getContext("2d");
        canvas_set_fill_style(ctx, "#3366cc");
        ctx.fillRect(0, 0, 80, 60);
        ctx["layer"]
    "##,
    );
    assert_eq!(out, "host");
}

#[test]
fn canvas_host_info_reports_backend() {
    let info = eval("bp_info()");
    assert!(info.contains("host-canvas"));
    assert!(info.contains("translate"));
    assert!(info.contains("drawImage"));
}

#[test]
fn canvas_host_translate_and_draw_image() {
    let out = eval(
        r##"
        let src = canvas_create(10, 10);
        src.fillStyle = "#ff00ff";
        src.fillRect(0, 0, 10, 10);
        let ctx = document.createElement("canvas").getContext("2d");
        ctx.fillStyle = "#000000";
        ctx.fillRect(0, 0, 20, 20);
        ctx.translate(5, 5);
        ctx.drawImage(src, 0, 0, 10, 10);
        ctx.measureText("Hi")["width"] > 0
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn canvas_get_put_image_data_roundtrip() {
    let out = eval(
        r##"
        let ctx = canvas_create(8, 8);
        canvas_set_fill_style(ctx, "#ff0000");
        canvas_fill_rect(ctx, 0, 0, 4, 4);
        let img = canvas_get_image_data(ctx, 0, 0, 4, 4);
        let ctx2 = canvas_create(8, 8);
        canvas_put_image_data(ctx2, img, 2, 2);
        let check = canvas_get_image_data(ctx2, 2, 2, 1, 1);
        img["width"] == 4 && img["height"] == 4 && len(img["data"]) == 64
            && check["data"][0] == 255 && check["data"][1] == 0 && check["data"][2] == 0
        "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn canvas_set_transform_and_rect_path() {
    let out = eval(
        r##"
        let ctx = canvas_create(20, 20);
        canvas_set_transform(ctx, 1, 0, 0, 1, 5, 5);
        canvas_begin_path(ctx);
        canvas_rect(ctx, 0, 0, 4, 4);
        canvas_set_fill_style(ctx, "#00ff00");
        canvas_fill(ctx);
        let px = canvas_get_image_data(ctx, 5, 5, 1, 1);
        px["data"][1] == 255
        "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn canvas_curves_clip_and_to_data_url() {
    let out = eval(
        r##"
        let ctx = canvas_create(32, 32);
        canvas_begin_path(ctx);
        canvas_rect(ctx, 0, 0, 16, 16);
        canvas_clip(ctx);
        canvas_begin_path(ctx);
        canvas_move_to(ctx, 0, 16);
        canvas_quadratic_curve_to(ctx, 8, 0, 16, 16);
        canvas_bezier_curve_to(ctx, 20, 32, 28, 0, 32, 16);
        canvas_set_stroke_style(ctx, "#ffffff");
        canvas_stroke(ctx);
        let url = canvas_to_data_url(ctx, "image/png");
        string_starts_with(url, "data:image/png;base64,")
        "##,
    );
    assert_eq!(out, "true");
}
