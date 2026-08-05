//! GP5a — desktop ship smoke (`kabootar run` + GPU 3D path when available).

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::{format_value, Value};

#[test]
fn ship_desktop_3d_triangle_smoke() {
    kabootar_lib::runtime::game::reset_all();
    let mut env = create_global_env();
    std::env::set_var("KABOOTAR_COMPILE", "rust");
    std::env::set_var("KABOOTAR_VM", "host");
    let v = eval_source(
        r#"
        import "game/render"
        platform_use("kabootar")
        let surf = game_surface_create_3d(32, 32)
        let gl = surf["gl"]
        setColor(gl, 0.2, 0.8, 1.0, 1.0)
        let mesh = createMesh(gl, [-0.5, -0.5, 0.5, 0.5, -0.5, 0.5, 0.0, 0.5, 0.5])
        drawMesh(mesh)
        surf.present()
        surf["mode"]
        "#,
        &mut env,
    )
    .expect("ship smoke");
    assert!(matches!(v, Value::String(ref s) if s == "3d"), "got {v:?}");
}

#[cfg(feature = "gpu")]
#[test]
fn ship_desktop_gpu_info_when_available() {
    let mut env = create_global_env();
    let avail = format_value(&eval_source(r#"webgl_info()["gpu3d"]"#, &mut env).unwrap());
    if avail == "cpu-fallback" {
        return;
    }
    assert!(
        avail.starts_with("wgpu-pipeline"),
        "desktop ship expects wgpu when adapter present: {avail}"
    );
}
