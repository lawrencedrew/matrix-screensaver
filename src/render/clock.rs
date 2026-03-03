use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::ttf::Font;
use rand::Rng;
use chrono::Local;
use chrono::Timelike;

pub struct CachedClockTexture<'a> {
    pub texture: sdl2::render::Texture<'a>,
    pub w: u32,
    pub h: u32,
}

pub struct ClockRenderer {
    last_second: u32,
}

impl ClockRenderer {
    pub fn new() -> Self {
        ClockRenderer { last_second: u32::MAX }
    }

    pub fn render<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        font: &Font,
        elapsed_secs: f32,
        rng: &mut impl Rng,
        cache: &mut Option<CachedClockTexture<'a>>,
    ) -> anyhow::Result<()> {
        let now = Local::now();
        let current_second = now.second();

        // Rebuild texture when the second changes
        if current_second != self.last_second || cache.is_none() {
            let time_str = now.format("%H:%M:%S").to_string();
            let surface = font
                .render(&time_str)
                .blended(Color::RGB(255, 255, 255))
                .map_err(|e| anyhow::anyhow!(e))?;
            let texture = texture_creator
                .create_texture_from_surface(&surface)
                .map_err(|e| anyhow::anyhow!(e))?;
            let q = texture.query();
            *cache = Some(CachedClockTexture { texture, w: q.width, h: q.height });
            self.last_second = current_second;
        }

        let Some(cached) = cache else { return Ok(()); };

        // Glitch: 0.3% chance per frame — replace one digit char with random Katakana
        let glitch_str: Option<String> = if rng.gen::<f32>() < 0.003 {
            let time_str = now.format("%H:%M:%S").to_string();
            let mut chars: Vec<char> = time_str.chars().collect();
            let digit_positions = [0usize, 1, 3, 4, 6, 7];
            let pos = digit_positions[rng.gen_range(0..digit_positions.len())];
            let katakana: Vec<char> = (0xFF66u32..=0xFF9F).filter_map(char::from_u32).collect();
            chars[pos] = katakana[rng.gen_range(0..katakana.len())];
            Some(chars.into_iter().collect())
        } else {
            None
        };

        let (sw, sh) = canvas.output_size().unwrap_or((1920, 1080));

        if let Some(ref gs) = glitch_str {
            // Build a one-shot texture for the glitch frame
            let surface = font
                .render(gs)
                .blended(Color::RGB(255, 255, 255))
                .map_err(|e| anyhow::anyhow!(e))?;
            let mut glitch_tex = texture_creator
                .create_texture_from_surface(&surface)
                .map_err(|e| anyhow::anyhow!(e))?;
            let q = glitch_tex.query();
            let cx = (sw as i32 - q.width as i32) / 2;
            let cy = (sh as i32 - q.height as i32) / 2;

            // Red ghost
            let red_surface = font
                .render(gs)
                .blended(Color::RGB(255, 255, 255))
                .map_err(|e| anyhow::anyhow!(e))?;
            let mut red_tex = texture_creator
                .create_texture_from_surface(&red_surface)
                .map_err(|e| anyhow::anyhow!(e))?;
            red_tex.set_color_mod(180, 0, 0);
            red_tex.set_alpha_mod(180);
            let _ = canvas.copy(&red_tex, None, Some(Rect::new(cx - 3, cy, q.width, q.height)));

            // Blue ghost
            let blue_surface = font
                .render(gs)
                .blended(Color::RGB(255, 255, 255))
                .map_err(|e| anyhow::anyhow!(e))?;
            let mut blue_tex = texture_creator
                .create_texture_from_surface(&blue_surface)
                .map_err(|e| anyhow::anyhow!(e))?;
            blue_tex.set_color_mod(0, 0, 180);
            blue_tex.set_alpha_mod(180);
            let _ = canvas.copy(&blue_tex, None, Some(Rect::new(cx + 3, cy, q.width, q.height)));

            // Green main
            let g = ((elapsed_secs * 2.0).sin().abs() * 30.0 + 225.0) as u8;
            glitch_tex.set_color_mod(0, g, 0);
            let _ = canvas.copy(&glitch_tex, None, Some(Rect::new(cx, cy, q.width, q.height)));

            return Ok(());
        }

        // Normal (non-glitch) path — use cached texture
        let tw = cached.w;
        let th = cached.h;
        let cx = (sw as i32 - tw as i32) / 2;
        let cy = (sh as i32 - th as i32) / 2;

        // Red ghost
        cached.texture.set_color_mod(180, 0, 0);
        cached.texture.set_alpha_mod(180);
        canvas.copy(&cached.texture, None, Some(Rect::new(cx - 3, cy, tw, th)))
            .map_err(|e| anyhow::anyhow!(e))?;

        // Blue ghost
        cached.texture.set_color_mod(0, 0, 180);
        cached.texture.set_alpha_mod(180);
        canvas.copy(&cached.texture, None, Some(Rect::new(cx + 3, cy, tw, th)))
            .map_err(|e| anyhow::anyhow!(e))?;

        // Green main with sine flicker
        let g = ((elapsed_secs * 2.0).sin().abs() * 30.0 + 225.0) as u8;
        cached.texture.set_color_mod(0, g, 0);
        cached.texture.set_alpha_mod(255);
        canvas.copy(&cached.texture, None, Some(Rect::new(cx, cy, tw, th)))
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }
}
