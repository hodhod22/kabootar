//! WebGL via canvas getContext + JS-syntax methods.

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::format_value;

fn eval(code: &str) -> String {
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn canvas_get_context_webgl_native() {
    let out = eval(
        r##"
        let page = kdom_create("div");
        let c = kdom_create("canvas");
        kdom_set_attr(c, "width", "64");
        kdom_set_attr(c, "height", "64");
        kdom_append(page, c);
        let gl = canvas_get_context(c, "webgl2");
        gl["kind"]
    "##,
    );
    assert_eq!(out, "webgl2");
}

#[test]
fn webgl_js_syntax_draw_elements() {
    let out = eval(
        r##"
        let gl = webgl_create(64, 64);
        let sh = gl.compileShader(
            "void main() { gl_Position = vec4(0.0); }",
            "void main() { }"
        );
        gl.useProgram(sh);
        gl.clearColor(10, 20, 30, 255);
        let vbo = gl.createBuffer("array", [-0.8, -0.8, 0.8, -0.8, 0.0, 0.8]);
        let ibo = gl.createIndexBuffer([0, 1, 2]);
        gl.bindBuffer(vbo);
        gl.bindBuffer(ibo);
        gl.uniform4f(0, 1.0, 0.5, 0.0, 1.0);
        gl.drawElements(3, 0)
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn webgl_js_syntax_draw_arrays() {
    let out = eval(
        r##"
        let gl = webgl_create(32, 32);
        gl.clearColor(0, 0, 0, 255);
        let vbo = gl.createBuffer("array", [0.0, 0.5, -0.5, -0.5, 0.5, -0.5]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3)
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn canvas_host_get_context_webgl() {
    let out = eval(
        r##"
        let canvas = document.createElement("canvas");
        let gl = canvas.getContext("webgl");
        gl.clearColor(1, 2, 3, 255);
        gl["layer"]
    "##,
    );
    assert_eq!(out, "host");
}

#[test]
fn webgl_info_reports_js_syntax() {
    let info = eval("webgl_info()");
    assert!(info.contains("js_syntax"));
    assert!(info.contains("uniform4f"));
    assert!(info.contains("textures"));
}

#[test]
fn webgl_texture_tex_image2d_and_draw() {
    let out = eval(
        r##"
        let src = canvas_create(16, 16);
        src.fillStyle = "#ff0000";
        src.fillRect(0, 0, 16, 16);
        let gl = webgl_create(32, 32);
        let tex = gl.createTexture();
        gl.texImage2D(tex, src);
        gl.bindTexture(tex);
        let vbo = gl.createBuffer("array", [
            -1.0, -1.0, 0.0, 0.0,
            1.0, -1.0, 1.0, 0.0,
            0.0, 1.0, 0.5, 1.0
        ]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3)
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn webgl_tex_image2d_from_canvas() {
    let out = eval(
        r##"
        let src = canvas_create(8, 8);
        src.fillStyle = "#00ff00";
        src.fillRect(0, 0, 8, 8);
        let gl = webgl_create(16, 16);
        let tex = gl.createTexture();
        let uploaded = gl.texImage2D(tex, src);
        uploaded["width"]
    "##,
    );
    assert_eq!(out, "8");
}
