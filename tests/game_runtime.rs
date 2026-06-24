//! Game runtime — frame loop, input, unified surface.

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::value::format_value;

fn eval(code: &str) -> String {
    kabootar::runtime::game::reset_all();
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn game_tick_increments_frame_counter() {
    let out = eval(r##"game_tick()["frame"]"##);
    assert_eq!(out, "1");
}

#[test]
fn request_animation_frame_invokes_callback() {
    let out = eval(
        r##"
        fn run() {
            fn on_frame(dt) { input_key_down("Space") }
            requestAnimationFrame(on_frame)
            game_tick()
        }
        run()
        input_is_down("Space")
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn game_run_executes_multiple_frames() {
    let out = eval(
        r##"
        fn run() {
            fn frame3(dt) { input_key_down("KeyA") }
            fn frame2(dt) {
                input_key_down("KeyS")
                requestAnimationFrame(frame3)
            }
            fn frame1(dt) {
                input_key_down("KeyD")
                requestAnimationFrame(frame2)
            }
            requestAnimationFrame(frame1)
            game_run(10)
        }
        run()
        len(input_poll()["down"])
    "##,
    );
    assert_eq!(out, "3");
}

#[test]
fn input_poll_keyboard() {
    let out = eval(
        r##"
        input_key_down("ArrowLeft")
        input_key_down("ArrowLeft")
        let e = input_poll()
        len(e["pressed"]) + len(e["down"])
    "##,
    );
    assert_eq!(out, "2");
}

#[test]
fn input_is_down() {
    let out = eval(
        r##"
        input_key_down("Space")
        input_is_down("Space")
    "##,
    );
    assert_eq!(out, "true");
}

#[test]
fn game_surface_create_and_present() {
    let out = eval(
        r##"
        platform_use("kabootar")
        let surf = game_surface_create(64, 64)
        let ctx = surf["ctx"]
        ctx.fillStyle = "#ff0000"
        ctx.fillRect(0, 0, 32, 32)
        surf.present()
        surf["layer"]
    "##,
    );
    assert_eq!(out, "kabootar");
}

#[test]
fn game_surface_frame_loop_present() {
    let out = eval(
        r##"
        let surf = game_surface_create(40, 40)
        let ctx = surf["ctx"]
        fn run() {
            fn on_frame(dt) {
                ctx.fillStyle = "#00ff00"
                ctx.fillRect(0, 0, 40, 40)
                surf.present()
            }
            requestAnimationFrame(on_frame)
            game_tick()
        }
        run()
        surf["width"]
    "##,
    );
    assert_eq!(out, "40");
}

#[test]
fn game_info_reports_api() {
    let info = eval("game_info()");
    assert!(info.contains("kabootar-game"));
    assert!(info.contains("3d"));
}

#[test]
fn game_surface_create_3d_and_present() {
    let out = eval(
        r##"
        let surf = game_surface_create_3d(64, 64);
        let gl = surf["gl"];
        gl.clearColor(20, 30, 40, 255);
        let vbo = gl.createBuffer("array", [-0.9, -0.9, 0.9, -0.9, 0.0, 0.9]);
        gl.bindBuffer(vbo);
        gl.drawArrays(3);
        surf.present();
        surf["mode"]
    "##,
    );
    assert_eq!(out, "3d");
}
