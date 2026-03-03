use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::ttf::Font;
use rand::Rng;

pub struct ClockRenderer {
    last_second: u32,
    cached_texture_key: Option<String>,
}

impl ClockRenderer {
    pub fn new() -> Self {
        ClockRenderer {
            last_second: u32::MAX,
            cached_texture_key: None,
        }
    }

    pub fn render<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        font: &Font,
        elapsed_secs: f32,
        rng: &mut impl Rng,
        textures: &mut Option<CachedClockTexture<'a>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct CachedClockTexture<'a> {
    pub texture: Texture<'a>,
    pub w: u32,
    pub h: u32,
}
