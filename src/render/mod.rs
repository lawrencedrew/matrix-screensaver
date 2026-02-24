pub mod matrix;

use std::collections::HashMap;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::time::{Duration, Instant};
use crate::config::Config;
use matrix::{Column, charset_chars};

const CELL_W: i32 = 14;
const CELL_H: i32 = 18;

// Fix 3: Safe parse_color that validates hex string length
pub fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return Color::RGB(0, 255, 0); // default green
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color::RGB(r, g, b)
}

// Fix 1: Pre-render all glyphs once into a cache keyed by char
fn build_glyph_cache<'a>(
    font: &sdl2::ttf::Font,
    chars: &[char],
    texture_creator: &'a TextureCreator<WindowContext>,
) -> anyhow::Result<HashMap<char, Texture<'a>>> {
    let mut cache = HashMap::new();
    for &ch in chars {
        let surface = font
            .render_char(ch)
            .blended(Color::RGB(255, 255, 255))
            .map_err(|e| anyhow::anyhow!(e))?;
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| anyhow::anyhow!(e))?;
        cache.insert(ch, texture);
    }
    Ok(cache)
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

    // Fix 1: Build glyph cache once before the main loop
    let mut glyph_cache = build_glyph_cache(&font, &chars, &texture_creator)?;

    // Fix 4: Record startup time so we can ignore early MouseMotion events
    let startup_time = Instant::now();

    'running: loop {
        // Fix 2: Process events at the TOP of every loop iteration
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                // Fix 4: Only exit on MouseMotion after 500 ms grace period
                Event::MouseMotion { .. }
                    if startup_time.elapsed() > Duration::from_millis(500) =>
                {
                    break 'running
                }
                _ => {}
            }
        }

        // Fix 2: Frame-rate throttle without skipping event processing
        let now = Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        let delta = now.duration_since(last_frame).as_secs_f32().max(0.001);
        last_frame = Instant::now();

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

                // Fix 1: Use cached texture with color modulation instead of
                // allocating a new texture per cell per frame
                if let Some(texture) = glyph_cache.get_mut(&ch) {
                    texture.set_color_mod(color.r, color.g, color.b);
                    let dst = Rect::new(
                        col.col_x * CELL_W,
                        cell_y * CELL_H,
                        CELL_W as u32,
                        CELL_H as u32,
                    );
                    canvas.copy(texture, None, Some(dst)).map_err(|e| anyhow::anyhow!(e))?;
                }
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
