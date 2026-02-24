pub mod matrix;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::{Duration, Instant};
use crate::config::Config;
use matrix::{Column, charset_chars};

const CELL_W: i32 = 14;
const CELL_H: i32 = 18;

pub fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color::RGB(r, g, b)
}

pub fn run_screensaver(config: &Config) -> anyhow::Result<()> {
    let sdl = sdl2::init().map_err(|e| anyhow::anyhow!(e))?;
    let video = sdl.video().map_err(|e| anyhow::anyhow!(e))?;
    let ttf_ctx = sdl2::ttf::init().map_err(|e| anyhow::anyhow!(e))?;

    let display_mode = video.desktop_display_mode(0).map_err(|e| anyhow::anyhow!(e))?;
    let width = display_mode.w as u32;
    let height = display_mode.h as u32;

    let window = video
        .window("matrix-screensaver", width, height)
        .fullscreen_desktop()
        .build()?;

    let mut canvas = window.into_canvas().accelerated().build()?;
    let texture_creator = canvas.texture_creator();

    let font_path = find_monospace_font()?;
    let font = ttf_ctx.load_font(font_path, 16).map_err(|e| anyhow::anyhow!(e))?;

    let cols = width as i32 / CELL_W;
    let rows = height as i32 / CELL_H;
    let chars = charset_chars(&config.charset);
    let base_color = parse_color(&config.color);
    let frame_duration = Duration::from_secs_f32(1.0 / config.fps as f32);

    let mut columns: Vec<Column> = (0..cols)
        .map(|x| Column::new(x, rows, config.speed))
        .collect();

    let mut event_pump = sdl.event_pump().map_err(|e| anyhow::anyhow!(e))?;
    let mut last_frame = Instant::now();

    use rand::Rng;
    let mut rng = rand::thread_rng();

    sdl.mouse().show_cursor(false);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                | Event::MouseMotion { .. } => break 'running,
                _ => {}
            }
        }

        let now = Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
            continue;
        }
        let delta = elapsed.as_secs_f32();
        last_frame = now;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        for col in &mut columns {
            col.update(delta);
            let head_cell = col.head_y as i32;
            for dist in 0..=(col.trail_len + 1) {
                let cell_y = head_cell - dist as i32;
                if cell_y < 0 || cell_y >= rows {
                    continue;
                }
                let brightness = col.brightness_at(dist);
                if brightness == 0 {
                    continue;
                }

                let ch = chars[rng.gen_range(0..chars.len())];
                let color = if dist == 0 {
                    Color::RGB(200, 255, 200)
                } else {
                    Color::RGB(
                        (base_color.r as f32 * brightness as f32 / 255.0) as u8,
                        (base_color.g as f32 * brightness as f32 / 255.0) as u8,
                        (base_color.b as f32 * brightness as f32 / 255.0) as u8,
                    )
                };

                let surface = font
                    .render_char(ch)
                    .blended(color)
                    .map_err(|e| anyhow::anyhow!(e))?;
                let texture = texture_creator
                    .create_texture_from_surface(&surface)
                    .map_err(|e| anyhow::anyhow!(e))?;

                let dst = Rect::new(
                    col.col_x * CELL_W,
                    cell_y * CELL_H,
                    CELL_W as u32,
                    CELL_H as u32,
                );
                canvas.copy(&texture, None, Some(dst)).map_err(|e| anyhow::anyhow!(e))?;
            }
        }

        canvas.present();
    }

    sdl.mouse().show_cursor(true);
    Ok(())
}

fn find_monospace_font() -> anyhow::Result<std::path::PathBuf> {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
    ];
    for path in &candidates {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }
    anyhow::bail!(
        "No monospace font found. Install fonts-dejavu-core or fonts-liberation."
    )
}
