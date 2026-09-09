//! Offline frame capture for Kabootar-authored WebGL + Canvas productions.
//! Product visuals remain in `.kab`; this host only composites and encodes PNG frames.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::runtime::frame_buffer;
use png::{BitDepth, ColorType, Encoder};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

fn capture_call(
    env: &mut kabootar_lib::value::Environment,
    source: &str,
    expected_width: usize,
    expected_height: usize,
) -> Result<Vec<u8>, String> {
    frame_buffer::clear_frame();
    eval_source(source, env)?;
    let (width, height, pixels) =
        frame_buffer::last_frame_pixels().ok_or("Kabootar did not publish a frame")?;
    if width as usize != expected_width || height as usize != expected_height {
        return Err(format!(
            "unexpected frame size {width}x{height}; expected {expected_width}x{expected_height}"
        ));
    }
    Ok(pixels)
}

fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(pixels).map_err(|e| e.to_string())
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let source_path = PathBuf::from(args.next().ok_or(
        "usage: kabootar-capture <production.kab> <frames-dir> [frames] [fps]",
    )?);
    let output_dir = PathBuf::from(args.next().ok_or("frames directory is required")?);
    let frame_count: usize = args
        .next()
        .unwrap_or_else(|| "600".into())
        .parse()
        .map_err(|_| "frames must be an integer")?;
    let fps: f64 = args
        .next()
        .unwrap_or_else(|| "30".into())
        .parse()
        .map_err(|_| "fps must be a number")?;

    let source = fs::read_to_string(&source_path)
        .map_err(|e| format!("{}: {e}", source_path.display()))?;
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let mut env = create_global_env();
    eval_source(&source, &mut env)?;

    const GL_WIDTH: usize = 1200;
    const HUD_WIDTH: usize = 720;
    const HEIGHT: usize = 1080;
    const OUTPUT_WIDTH: usize = GL_WIDTH + HUD_WIDTH;

    for frame in 0..frame_count {
        let t = frame as f64 / fps;
        let mut gl = vec![0_u8; GL_WIDTH * HEIGHT * 4];
        for layer in 0..4 {
            let rendered = capture_call(
                &mut env,
                &format!("render_gl({t:.8}, {layer})"),
                GL_WIDTH,
                HEIGHT,
            )?;
            for (dst, src) in gl.chunks_exact_mut(4).zip(rendered.chunks_exact(4)) {
                dst[0] = dst[0].max(src[0]);
                dst[1] = dst[1].max(src[1]);
                dst[2] = dst[2].max(src[2]);
                dst[3] = 255;
            }
        }
        let hud = capture_call(
            &mut env,
            &format!("render_hud({t:.8})"),
            HUD_WIDTH,
            HEIGHT,
        )?;

        let mut combined = vec![0_u8; OUTPUT_WIDTH * HEIGHT * 4];
        for y in 0..HEIGHT {
            let dst = y * OUTPUT_WIDTH * 4;
            let hud_src = y * HUD_WIDTH * 4;
            let gl_src = y * GL_WIDTH * 4;
            combined[dst..dst + HUD_WIDTH * 4]
                .copy_from_slice(&hud[hud_src..hud_src + HUD_WIDTH * 4]);
            combined[dst + HUD_WIDTH * 4..dst + OUTPUT_WIDTH * 4]
                .copy_from_slice(&gl[gl_src..gl_src + GL_WIDTH * 4]);
        }

        let path = output_dir.join(format!("frame-{frame:05}.png"));
        write_png(&path, OUTPUT_WIDTH as u32, HEIGHT as u32, &combined)?;
        if frame % 30 == 0 {
            println!("Kabootar frame {frame}/{frame_count}");
        }
    }
    Ok(())
}
