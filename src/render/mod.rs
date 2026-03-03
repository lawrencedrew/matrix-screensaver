pub mod matrix;
pub mod clock;

use std::collections::HashMap;
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::time::{Duration, Instant};
use crate::config::Config;
use matrix::{Column, charset_chars};
use clock::{ClockRenderer, CachedClockTexture};

const CELL_W: i32 = 14;
const CELL_H: i32 = 18;

pub fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return Color::RGB(0, 255, 0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color::RGB(r, g, b)
}

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

    let (font_path, font_index) = find_monospace_font()?;
    let font = ttf_ctx.load_font_at_index(&font_path, font_index, 16).map_err(|e| anyhow::anyhow!(e))?;
    let clock_font_result = ttf_ctx.load_font_at_index(&font_path, font_index, 72);
    let clock_font = match clock_font_result {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("matrix-screensaver: clock font load failed, clock disabled: {e}");
            None
        }
    };

    let chars = charset_chars(&config.charset);
    let base_color = parse_color(&config.color);
    let frame_duration = Duration::from_secs_f32(1.0 / config.fps as f32);

    let num_displays = video.num_video_displays().unwrap_or(1) as usize;

    // One window per physical monitor. SDL2's set_fullscreen(Desktop) goes
    // through Mutter's proper fullscreen path and tells GNOME Shell to retract
    // its panel. _NET_WM_BYPASS_COMPOSITOR=1 then makes Mutter render the window
    // directly to the framebuffer, bypassing the compositor layer: the panel
    // stays hidden, there is no compositor snapshot caching, and the window
    // appears live on all virtual desktops automatically.
    let mut canvases: Vec<Canvas<Window>> = Vec::new();
    let mut columns_per_canvas: Vec<Vec<Column>> = Vec::new();

    use rand::Rng;
    let mut rng = rand::thread_rng();

    for i in 0..num_displays {
        let bounds = video.display_bounds(i as i32).map_err(|e| anyhow::anyhow!(e))?;
        let cols = bounds.width() as i32 / CELL_W;
        let rows = bounds.height() as i32 / CELL_H;

        let title = format!("matrix-screensaver-{i}");
        let window = video
            .window(&title, bounds.width(), bounds.height())
            .position(bounds.x(), bounds.y())
            .borderless()
            .build()?;
        let canvas = window.into_canvas().accelerated().build()?;

        columns_per_canvas.push(
            (0..cols)
                .filter(|_| rng.gen::<f32>() > 0.3)
                .map(|x| Column::new(x, rows, config.speed))
                .collect(),
        );
        canvases.push(canvas);
    }

    let total_canvases = canvases.len();

    let texture_creators: Vec<TextureCreator<WindowContext>> =
        canvases.iter().map(|c| c.texture_creator()).collect();

    let mut glyph_caches: Vec<HashMap<char, Texture>> = texture_creators
        .iter()
        .map(|tc| build_glyph_cache(&font, &chars, tc))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut clock_renderers: Vec<ClockRenderer> =
        (0..total_canvases).map(|_| ClockRenderer::new()).collect();
    let mut clock_texture_caches: Vec<Option<CachedClockTexture>> =
        (0..total_canvases).map(|_| None).collect();

    let mut event_pump = sdl.event_pump().map_err(|e| anyhow::anyhow!(e))?;

    // Let the WM register our windows before requesting fullscreen.
    std::thread::sleep(Duration::from_millis(300));

    // SDL2's set_fullscreen goes through Mutter's proper state machine,
    // which correctly hides the GNOME Shell panel and positions the window
    // at (0,0) covering the full monitor.
    for canvas in &mut canvases {
        let _ = canvas.window_mut().set_fullscreen(sdl2::video::FullscreenType::Desktop);
    }

    // After fullscreen is applied, set _NET_WM_BYPASS_COMPOSITOR=1 so Mutter
    // renders our windows directly to the framebuffer rather than through the
    // compositor pipeline. This keeps the panel hidden on all virtual desktops
    // and eliminates compositor snapshot caching.
    std::thread::sleep(Duration::from_millis(200));
    set_bypass_compositor(num_displays);

    let mut last_frame = Instant::now();
    let startup_time = Instant::now();

    sdl.mouse().show_cursor(false);

    let mut exit_requested: Option<Instant> = None;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { .. }
                | Event::MouseButtonDown { .. } => {
                    if exit_requested.is_none() {
                        exit_requested = Some(Instant::now());
                    }
                }
                Event::MouseMotion { .. }
                    if startup_time.elapsed() > Duration::from_millis(500) =>
                {
                    if exit_requested.is_none() {
                        exit_requested = Some(Instant::now());
                    }
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        let after_sleep = Instant::now();
        let delta = after_sleep.duration_since(last_frame).as_secs_f32().max(0.001);
        last_frame = after_sleep;

        let speed_mult = if exit_requested.is_some() { 5.0_f32 } else { 1.0_f32 };
        if let Some(t) = exit_requested {
            if t.elapsed() > Duration::from_millis(600) {
                break 'running;
            }
        }

        for (idx, canvas) in canvases.iter_mut().enumerate() {
            let cols = &mut columns_per_canvas[idx];
            let glyph_cache = &mut glyph_caches[idx];
            let rows = canvas.output_size().map(|(_, h)| h as i32).unwrap_or(1080) / CELL_H;

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();

            let fade = (startup_time.elapsed().as_secs_f32() / 2.0).min(1.0);
            let head_color = Color::RGB(
                (200.0 * fade) as u8,
                (255.0 * fade) as u8,
                (200.0 * fade) as u8,
            );
            for col in cols.iter_mut() {
                col.update(delta * speed_mult);
                let head_cell = col.head_y as i32;
                for dist in 0..=col.trail_len {
                    let cell_y = head_cell - dist as i32;
                    if cell_y < 0 || cell_y >= rows {
                        continue;
                    }
                    let raw_brightness = col.brightness_at(dist);
                    if raw_brightness == 0 {
                        continue;
                    }
                    let brightness = (raw_brightness as f32 * fade) as u8;
                    if brightness == 0 {
                        continue;
                    }
                    let ch = chars[rng.gen_range(0..chars.len())];
                    let color = if dist == 0 {
                        head_color
                    } else {
                        Color::RGB(
                            (base_color.r as f32 * brightness as f32 / 255.0) as u8,
                            (base_color.g as f32 * brightness as f32 / 255.0) as u8,
                            (base_color.b as f32 * brightness as f32 / 255.0) as u8,
                        )
                    };
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
                if rng.gen::<f32>() < 0.005 {
                    let glitch_y = rng.gen_range(0..rows);
                    let ch = chars[rng.gen_range(0..chars.len())];
                    if let Some(texture) = glyph_cache.get_mut(&ch) {
                        texture.set_color_mod(255, 255, 255);
                        let dst = Rect::new(
                            col.col_x * CELL_W,
                            glitch_y * CELL_H,
                            CELL_W as u32,
                            CELL_H as u32,
                        );
                        let _ = canvas.copy(texture, None, Some(dst));
                        texture.set_color_mod(base_color.r, base_color.g, base_color.b);
                    }
                }
            }

            if let Some(ref cf) = clock_font {
                if let Err(e) = clock_renderers[idx].render(
                    canvas,
                    &texture_creators[idx],
                    cf,
                    startup_time.elapsed().as_secs_f32(),
                    &mut rng,
                    &mut clock_texture_caches[idx],
                ) {
                    eprintln!("matrix-screensaver: clock render error: {e}");
                }
            }

            canvas.present();
        }
    }

    sdl.mouse().show_cursor(true);
    Ok(())
}

/// Set _NET_WM_BYPASS_COMPOSITOR=1 on each screensaver window.
/// This tells Mutter to render the window directly to the framebuffer,
/// bypassing the compositor pipeline that carries GNOME Shell's panel overlay,
/// and eliminating snapshot caching across virtual desktops.
fn set_bypass_compositor(num_displays: usize) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::wrapper::ConnectionExt as _;

    let Ok((conn, screen_num)) = x11rb::connect(None) else { return; };
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let intern = |name: &[u8]| -> Option<u32> {
        conn.intern_atom(false, name).ok()?.reply().ok()
            .and_then(|r| if r.atom != 0 { Some(r.atom) } else { None })
    };

    let Some(net_client_list) = intern(b"_NET_CLIENT_LIST")          else { return; };
    let Some(net_wm_name)     = intern(b"_NET_WM_NAME")              else { return; };
    let Some(bypass_atom)     = intern(b"_NET_WM_BYPASS_COMPOSITOR") else { return; };
    let Some(utf8_string)     = intern(b"UTF8_STRING")               else { return; };

    let windows: Vec<u32> = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, 10240)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().map(|it| it.collect()))
        .unwrap_or_default();

    for win in windows {
        let name: Vec<u8> = conn
            .get_property(false, win, net_wm_name, utf8_string, 0, 512)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.value)
            .unwrap_or_default();

        let name_str = String::from_utf8_lossy(&name);
        let Some(suffix) = name_str.strip_prefix("matrix-screensaver-") else { continue; };
        let Ok(idx) = suffix.parse::<usize>() else { continue; };
        if idx >= num_displays { continue; }

        let _ = conn.change_property32(
            PropMode::REPLACE, win, bypass_atom, AtomEnum::CARDINAL, &[1u32],
        );
    }

    let _ = conn.flush();
}

fn find_monospace_font() -> anyhow::Result<(std::path::PathBuf, u32)> {
    let candidates: &[(&str, u32)] = &[
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 5),
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 5),
        ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 5),
        ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 0),
        ("/usr/share/fonts/TTF/DejaVuSansMono.ttf", 0),
        ("/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf", 0),
        ("/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf", 0),
        ("/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf", 0),
    ];
    for &(path, index) in candidates {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok((p, index));
        }
    }
    anyhow::bail!(
        "No monospace font found. Install fonts-noto-cjk, fonts-dejavu-core, or fonts-liberation."
    )
}
