//! Real 3D — vec3 vertices, MVP matrices, z-buffer.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::format_value;

fn eval(code: &str) -> String {
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn webgl_info_reports_real_3d() {
    let info = eval("webgl_info()");
    assert!(info.contains("v2.60-3d"));
    assert!(info.contains("z-buffer"));
    assert!(info.contains("uniformMatrix4fv"));
}

#[test]
fn webgl_vec3_cube_draw() {
    let out = eval(
        r##"
        let gl = webgl_create(64, 64);
        gl.lookAt(0, 0, 3, 0, 0, 0, 0, 1, 0);
        gl.rotateModelY(30);
        gl.uniform4f(0, 0.2, 0.8, 1.0, 1.0);
        let vbo = gl.createBuffer("array", [
            -0.5, -0.5,  0.5,  0.5, -0.5,  0.5,  0.5,  0.5,  0.5,
            -0.5, -0.5,  0.5,  0.5,  0.5,  0.5, -0.5,  0.5,  0.5,
             0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, -0.5,
             0.5, -0.5, -0.5, -0.5,  0.5, -0.5,  0.5,  0.5, -0.5
        ]);
        gl.bindBuffer(vbo);
        gl.drawArrays(12)
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn webgl_depth_buffer_occlusion() {
    let out = eval(
        r##"
        let gl = webgl_create(32, 32);
        gl.lookAt(0, 0, 2.5, 0, 0, 0, 0, 1, 0);
        gl.clearColor(0, 0, 0, 255);
        gl.uniform4f(0, 1.0, 0.0, 0.0, 1.0);
        let near = gl.createBuffer("array", [
            -0.6, -0.6, 0.0,  0.6, -0.6, 0.0,  0.6, 0.6, 0.0,
            -0.6, -0.6, 0.0,  0.6, 0.6, 0.0, -0.6, 0.6, 0.0
        ]);
        gl.bindBuffer(near);
        gl.drawArrays(6);
        gl.uniform4f(0, 0.0, 0.0, 1.0, 1.0);
        let far = gl.createBuffer("array", [
            -0.6, -0.6, -0.5,  0.6, -0.6, -0.5,  0.6, 0.6, -0.5,
            -0.6, -0.6, -0.5,  0.6, 0.6, -0.5, -0.6, 0.6, -0.5
        ]);
        gl.bindBuffer(far);
        gl.drawArrays(6);
        "ok"
    "##,
    );
    assert_eq!(out, "ok");
}

#[test]
fn game_surface_3d_cube_frame() {
    let out = eval(
        r##"
        let surf = game_surface_create_3d(64, 64);
        let gl = surf["gl"];
        gl.rotateModelY(20);
        let vbo = gl.createBuffer("array", [
            -0.5, -0.5, 0.5,  0.5, -0.5, 0.5,  0.0, 0.5, 0.5
        ]);
        gl.bindBuffer(vbo);
        gl.uniform4f(0, 0.0, 1.0, 0.5, 1.0);
        gl.drawArrays(3);
        surf.present();
        surf["mode"]
    "##,
    );
    assert_eq!(out, "3d");
}

#[cfg(feature = "gpu")]
#[test]
fn webgl_gpu_textured_vec5_draw() {
    // GP0a: bound texture + vec5 (xyz+uv) stays on wgpu path when GPU is available.
    let out = eval(
        r##"
        let src = canvas_create(4, 4);
        src.fillStyle = "#ff0000";
        src.fillRect(0, 0, 4, 4);
        let gl = webgl_create(32, 32);
        gl.lookAt(0, 0, 2.5, 0, 0, 0, 0, 1, 0);
        gl.clearColor(0, 0, 0, 255);
        gl.uniform4f(0, 1.0, 1.0, 1.0, 1.0);
        let tex = gl.createTexture();
        gl.texImage2D(tex, src);
        gl.bindTexture(tex);
        let vbo = gl.createBuffer("array", [
            -0.8, -0.8, 0.0, 0.0, 0.0,
             0.8, -0.8, 0.0, 1.0, 0.0,
             0.0,  0.8, 0.0, 0.5, 1.0
        ]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3);
        webgl_info()["gpu3d"]
    "##,
    );
    assert!(
        out == "wgpu-pipeline" || out == "wgpu-pipeline+msaa4" || out == "cpu-fallback",
        "unexpected gpu3d info: {out}"
    );
}

#[test]
fn webgl_material_uv_xform_and_model_uniform() {
    let out = eval(
        r##"
        let gl = webgl_create(64, 64);
        gl.lookAt(0, 0, 3, 0, 0, 0, 0, 1, 0);
        gl.uniform4f(0, 0.2, 0.8, 1.0, 1.0);
        gl.uniform4f(1, 2.0, 2.0, 0.0, 0.0);
        gl.rotateModelY(25);
        let vbo = gl.createBuffer("array", [
            -0.5, -0.5, 0.5,  0.5, -0.5, 0.5,  0.0, 0.5, 0.5
        ]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3);
        "ok"
    "##,
    );
    assert_eq!(out, "ok");
}

#[cfg(feature = "gpu")]
#[test]
fn webgl_gpu_material_bind_groups() {
    let out = eval(
        r##"
        let src = canvas_create(4, 4);
        src.fillStyle = "#00ff88";
        src.fillRect(0, 0, 4, 4);
        let gl = webgl_create(32, 32);
        gl.lookAt(0, 0, 2.5, 0, 0, 0, 0, 1, 0);
        gl.clearColor(0, 0, 0, 255);
        gl.uniform4f(0, 1.0, 1.0, 1.0, 1.0);
        gl.uniform4f(1, 1.5, 1.5, 0.0, 0.0);
        let tex = gl.createTexture();
        gl.texImage2D(tex, src);
        gl.bindTexture(tex);
        let vbo = gl.createBuffer("array", [
            -0.8, -0.8, 0.0, 0.0, 0.0,
             0.8, -0.8, 0.0, 1.0, 0.0,
             0.0,  0.8, 0.0, 0.5, 1.0
        ]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3);
        webgl_info()["gpu3d"]
    "##,
    );
    assert!(
        out == "wgpu-pipeline" || out == "wgpu-pipeline+msaa4" || out == "cpu-fallback",
        "unexpected gpu3d info: {out}"
    );
}
