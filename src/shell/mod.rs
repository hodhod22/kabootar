//! Native Kabootar desktop shell — real OS window with pixel compositor.

#[cfg(feature = "shell")]
mod gpu_window;

#[cfg(feature = "shell")]
use crate::value::Environment;
#[cfg(feature = "shell")]
use std::cell::RefCell;

#[cfg(feature = "shell")]
thread_local! {
    static SHELL_ENV: RefCell<Option<Environment>> = RefCell::new(None);
}

#[cfg(feature = "shell")]
pub fn run_desktop() -> Result<(), String> {
    boot_desktop_frame()?;

    #[cfg(feature = "gpu")]
    {
        if let Ok(()) = gpu_window::run() {
            return Ok(());
        }
    }

    run_softbuffer_standalone()
}

#[cfg(feature = "shell")]
fn boot_desktop_frame() -> Result<(), String> {
    use crate::evaluator::{create_global_env, eval_source};

    const BOOT: &str = r#"
        import "kos/boot"
        platform_use("kabootar");
        kb_set_backend("gpu");
        let win = os_window_create("Kabootar OS", 960, 540);
        os_display_register(win, "Kabootar Desktop", 960, 540);
        kb_viewport(960, 540);
        // H6c: session policy (apps seed, Start, theme, mount) in kos/boot
        let kosShell = bootKosSession();
        if kosShell == null {
            import "kstyle/parse"
            parseAndApply("body { display:flex; flex-direction:column; padding:32px; background:#292a2d; color:#e8eaed; gap:16px; }
              h1 { font-size:36px; color:#8ab4f8; } .card { background:#35363a; padding:20px; border-radius:12px; }");
            let ui = kml("<html><body><h1>Kabootar OS</h1><div class='card'><p>Native desktop — GPU compositor when available.</p></div></body></html>");
            kb_mount(ui);
            kb_paint();
        }
    "#;

    let mut env = create_global_env();
    eval_source(BOOT, &mut env).map_err(|e| format!("boot failed: {e}"))?;
    SHELL_ENV.with(|slot| {
        *slot.borrow_mut() = Some(env);
    });
    Ok(())
}

/// Map host pointer click → kb_click → drainKosEvents → remount/paint.
#[cfg(feature = "shell")]
pub(crate) fn shell_pointer_click(x: f64, y: f64) -> Result<(), String> {
    use crate::evaluator::eval_source;
    use crate::runtime::game;

    game::pointer_down(x, y);
    SHELL_ENV.with(|slot| {
        let mut guard = slot.borrow_mut();
        let env = guard
            .as_mut()
            .ok_or_else(|| "shell env not booted".to_string())?;
        let code = format!(
            r#"
            import "kos/boot"
            if kosShell != null && kosShell != undefined {{
                kosShell = handleShellClick(kosShell, {x}, {y});
            }} else {{
                kb_click({x}, {y});
            }}
            "#
        );
        eval_source(&code, env).map_err(|e| format!("shell click: {e}"))?;
        Ok(())
    })
}

#[cfg(feature = "shell")]
fn run_softbuffer_standalone() -> Result<(), String> {
    use crate::runtime::frame_buffer;
    use crate::runtime::game;
    use softbuffer::{Context, Surface};
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use winit::event::{ElementState, Event, MouseButton, WindowEvent};
    use winit::event_loop::EventLoop;
    use winit::keyboard::PhysicalKey;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Kabootar OS")
            .with_inner_size(winit::dpi::LogicalSize::new(960, 540))
            .build(&event_loop)
            .map_err(|e| format!("window: {e}"))?,
    );

    let context = Context::new(window.clone()).map_err(|e| format!("softbuffer ctx: {e}"))?;
    let mut surface =
        Surface::new(&context, window.clone()).map_err(|e| format!("surface: {e}"))?;

    let blit = |window: &winit::window::Window,
                surface: &mut Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>|
     -> Result<(), String> {
        let (w, h, rgba) = frame_buffer::last_frame_pixels().ok_or("no compositor frame")?;
        let size = window.inner_size();
        let sw = size.width;
        let sh = size.height;
        surface
            .resize(
                NonZeroU32::new(sw.max(1)).unwrap(),
                NonZeroU32::new(sh.max(1)).unwrap(),
            )
            .map_err(|e| format!("resize: {e}"))?;
        let mut buf = surface.buffer_mut().map_err(|e| format!("buffer: {e}"))?;
        let fw = w.max(1) as usize;
        let fh = h.max(1) as usize;
        for (i, px) in buf.iter_mut().enumerate() {
            let sx = i % sw as usize;
            let sy = i / sw as usize;
            let fx = sx * fw / sw as usize;
            let fy = sy * fh / sh as usize;
            let idx = (fy * fw + fx) * 4;
            if idx + 3 < rgba.len() {
                let r = rgba[idx] as u32;
                let g = rgba[idx + 1] as u32;
                let b = rgba[idx + 2] as u32;
                *px = (r << 16) | (g << 8) | b;
            } else {
                *px = 0x202124;
            }
        }
        buf.present().map_err(|e| format!("present: {e}"))?;
        Ok(())
    };

    blit(&window, &mut surface)?;
    let mut last_frame = Instant::now();
    let mut cursor = (0.0_f64, 0.0_f64);

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::RedrawRequested => {
                        SHELL_ENV.with(|slot| {
                            if let Some(env) = slot.borrow_mut().as_mut() {
                                let _ = game::shell_step(env);
                            }
                        });
                        let _ = blit(&window, &mut surface);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let PhysicalKey::Code(code) = event.physical_key {
                            let key = winit_key_label(code);
                            match event.state {
                                ElementState::Pressed => game::key_down(key),
                                ElementState::Released => game::key_up(key),
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor = (position.x, position.y);
                        game::pointer_move(position.x, position.y);
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let (x, y) = cursor;
                        let _ = shell_pointer_click(x, y);
                        window.request_redraw();
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let (x, y) = cursor;
                        game::pointer_up(x, y);
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    if last_frame.elapsed() >= Duration::from_millis(16) {
                        last_frame = Instant::now();
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        })
        .map_err(|e| format!("loop: {e}"))?;
    Ok(())
}

#[cfg(feature = "shell")]
fn winit_key_label(code: winit::keyboard::KeyCode) -> &'static str {
    use winit::keyboard::KeyCode;
    match code {
        KeyCode::ArrowLeft => "ArrowLeft",
        KeyCode::ArrowRight => "ArrowRight",
        KeyCode::ArrowUp => "ArrowUp",
        KeyCode::ArrowDown => "ArrowDown",
        KeyCode::Space => "Space",
        KeyCode::Escape => "Escape",
        KeyCode::KeyW => "KeyW",
        KeyCode::KeyA => "KeyA",
        KeyCode::KeyS => "KeyS",
        KeyCode::KeyD => "KeyD",
        _ => "Unknown",
    }
}

#[cfg(not(feature = "shell"))]
pub fn run_desktop() -> Result<(), String> {
    Err("Kabootar shell requires --features shell".into())
}
