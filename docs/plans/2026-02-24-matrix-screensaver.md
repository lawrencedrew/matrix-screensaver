# Matrix Screensaver Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust binary that detects system idle across X11/Wayland/GNOME/KDE and displays a Matrix-style falling character rain screensaver.

**Architecture:** A single async Tokio binary with a trait-based idle detection system (three backends tried in order: Wayland `ext-idle-notify-v1`, D-Bus `org.freedesktop.ScreenSaver`, X11 MIT-SCREEN-SAVER). On idle, opens a fullscreen SDL2 window rendering matrix rain; on wake, closes it.

**Tech Stack:** Rust, Tokio, SDL2, wayland-client, wayland-protocols, zbus, x11rb, serde+toml, rand

---

## Task 1: Cargo project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Initialize the project**

```bash
cd ~/matrix-screensaver
cargo init --name matrix-screensaver
```

**Step 2: Replace Cargo.toml with full dependencies**

```toml
[package]
name = "matrix-screensaver"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "matrix-screensaver"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
sdl2 = { version = "0.37", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
wayland-client = "0.31"
wayland-protocols = { version = "0.32", features = ["staging"] }
zbus = { version = "4", default-features = false, features = ["tokio"] }
x11rb = { version = "0.13", features = ["screensaver"] }
rand = "0.8"
anyhow = "1"
async-trait = "0.1"
```

**Step 3: Verify it compiles**

```bash
cargo check
```

Expected: no errors (warnings about unused deps are fine).

**Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: scaffold cargo project with dependencies"
```

---

## Task 2: Config module

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Write failing test**

Add to `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.idle_timeout_secs, 300);
        assert_eq!(cfg.fps, 30);
        assert!((cfg.speed - 1.0).abs() < f32::EPSILON);
        assert_eq!(cfg.charset, Charset::Katakana);
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
            idle_timeout_secs = 60
            fps = 20
            speed = 2.0
            charset = "mixed"
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.idle_timeout_secs, 60);
        assert_eq!(cfg.fps, 20);
        assert!((cfg.speed - 2.0).abs() < f32::EPSILON);
        assert_eq!(cfg.charset, Charset::Mixed);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test config
```

Expected: compile error — `Config`, `Charset` not defined.

**Step 3: Implement config.rs**

```rust
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Charset {
    Katakana,
    Latin,
    Digits,
    Mixed,
}

impl Default for Charset {
    fn default() -> Self {
        Charset::Katakana
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_timeout")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub charset: Charset,
}

fn default_timeout() -> u64 { 300 }
fn default_color() -> String { "#00ff00".to_string() }
fn default_fps() -> u32 { 30 }
fn default_speed() -> f32 { 1.0 }

impl Default for Config {
    fn default() -> Self {
        Config {
            idle_timeout_secs: default_timeout(),
            color: default_color(),
            fps: default_fps(),
            speed: default_speed(),
            charset: Charset::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        config_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(home).join(".config")).unwrap_or_default().into()
        });
    Some(base.join("matrix-screensaver").join("config.toml"))
}
```

Add to `src/main.rs`:

```rust
mod config;

fn main() {
    let _cfg = config::Config::load();
}
```

**Step 4: Run tests**

```bash
cargo test config
```

Expected: 2 tests PASS.

**Step 5: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add config module with TOML parsing and defaults"
```

---

## Task 3: Matrix rain data structures

**Files:**
- Create: `src/render/mod.rs`
- Create: `src/render/matrix.rs`

**Step 1: Write failing tests**

Create `src/render/matrix.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_advances() {
        let mut col = Column::new(0, 24, 1.0);
        let initial_head = col.head_y;
        col.update(1.0);
        assert!(col.head_y > initial_head || col.head_y == 0.0);
    }

    #[test]
    fn test_trail_length_positive() {
        let col = Column::new(0, 24, 1.0);
        assert!(col.trail_len > 0);
    }

    #[test]
    fn test_cell_fade() {
        // Cell at distance 0 from head = full brightness
        // Cell at distance > trail_len = 0 brightness
        let col = Column::new(0, 24, 1.0);
        let full = col.brightness_at(0);
        let gone = col.brightness_at(col.trail_len + 5);
        assert!(full > 0);
        assert_eq!(gone, 0);
    }

    #[test]
    fn test_charset_katakana() {
        let chars = charset_chars(&crate::config::Charset::Katakana);
        // All chars should be in the katakana half-width range
        for c in &chars {
            let cp = *c as u32;
            assert!(cp >= 0xFF66 && cp <= 0xFF9F, "unexpected char: U+{:04X}", cp);
        }
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test render
```

Expected: compile error.

**Step 3: Implement matrix.rs**

```rust
use rand::Rng;
use crate::config::Charset;

pub fn charset_chars(charset: &Charset) -> Vec<char> {
    match charset {
        Charset::Katakana => (0xFF66u32..=0xFF9F).filter_map(char::from_u32).collect(),
        Charset::Latin => ('A'..='Z').chain('a'..='z').collect(),
        Charset::Digits => ('0'..='9').collect(),
        Charset::Mixed => {
            let mut v: Vec<char> = (0xFF66u32..=0xFF9F).filter_map(char::from_u32).collect();
            v.extend('A'..='Z');
            v.extend('0'..='9');
            v
        }
    }
}

pub struct Column {
    pub col_x: i32,
    pub head_y: f32,
    pub trail_len: usize,
    height_cells: i32,
    speed: f32,
    active: bool,
    delay: f32,
}

impl Column {
    pub fn new(col_x: i32, height_cells: i32, speed: f32) -> Self {
        let mut rng = rand::thread_rng();
        Column {
            col_x,
            head_y: -(rng.gen_range(1..height_cells) as f32),
            trail_len: rng.gen_range(8..20),
            height_cells,
            speed: speed * rng.gen_range(0.5f32..1.5),
            active: true,
            delay: 0.0,
        }
    }

    pub fn update(&mut self, delta_secs: f32) {
        if self.delay > 0.0 {
            self.delay -= delta_secs;
            return;
        }
        self.head_y += self.speed * delta_secs * 15.0;
        if self.head_y - self.trail_len as f32 > self.height_cells as f32 {
            // Reset
            let mut rng = rand::thread_rng();
            self.head_y = -(rng.gen_range(1..5) as f32);
            self.trail_len = rng.gen_range(8..20);
            self.speed = rng.gen_range(0.5f32..1.5);
            self.delay = rng.gen_range(0.0f32..2.0);
        }
    }

    /// Returns brightness 0-255 for the cell at `cell_y`. Head char returns 255.
    pub fn brightness_at(&self, distance_from_head: usize) -> u8 {
        if distance_from_head == 0 {
            return 255; // head: white
        }
        if distance_from_head > self.trail_len {
            return 0;
        }
        let ratio = 1.0 - (distance_from_head as f32 / self.trail_len as f32);
        (ratio * 200.0) as u8
    }

    pub fn is_head_at(&self, cell_y: i32) -> bool {
        cell_y == self.head_y as i32
    }
}
```

Create `src/render/mod.rs`:

```rust
pub mod matrix;
```

Add to `src/main.rs`:

```rust
mod render;
```

**Step 4: Run tests**

```bash
cargo test render
```

Expected: 4 tests PASS.

**Step 5: Commit**

```bash
git add src/render/
git commit -m "feat: add matrix rain column data structures with tests"
```

---

## Task 4: SDL2 rendering window

**Files:**
- Modify: `src/render/mod.rs`

**Step 1: Implement the renderer**

SDL2 rendering is visual — we test it compiles and runs without panicking. Add to `src/render/mod.rs`:

```rust
pub mod matrix;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::ttf;
use std::time::{Duration, Instant};
use crate::config::{Config, Charset};
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
    let ttf_ctx = ttf::init().map_err(|e| anyhow::anyhow!(e))?;

    let display_mode = video.desktop_display_mode(0).map_err(|e| anyhow::anyhow!(e))?;
    let width = display_mode.w as u32;
    let height = display_mode.h as u32;

    let window = video
        .window("matrix-screensaver", width, height)
        .fullscreen_desktop()
        .build()?;

    let mut canvas = window.into_canvas().accelerated().build()?;
    let texture_creator = canvas.texture_creator();

    // Embed a fallback font or use a system monospace font
    // For now, use SDL2_ttf with a known system path
    let font_path = find_monospace_font()?;
    let font = ttf_ctx.load_font(font_path, 16).map_err(|e| anyhow::anyhow!(e))?;

    let cols = (width as i32) / CELL_W;
    let rows = (height as i32) / CELL_H;
    let chars = charset_chars(&config.charset);
    let base_color = parse_color(&config.color);
    let frame_duration = Duration::from_secs_f32(1.0 / config.fps as f32);

    let mut columns: Vec<Column> = (0..cols)
        .map(|x| Column::new(x, rows, config.speed))
        .collect();

    let mut event_pump = sdl.event_pump().map_err(|e| anyhow::anyhow!(e))?;
    let mut last_frame = Instant::now();
    let mut rng = rand::thread_rng();

    // Hide cursor
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
        let delta = now.duration_since(last_frame).as_secs_f32();
        if delta < frame_duration.as_secs_f32() {
            std::thread::sleep(frame_duration - now.duration_since(last_frame));
            continue;
        }
        last_frame = now;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        use rand::Rng;
        for col in &mut columns {
            col.update(delta);
            let head_cell = col.head_y as i32;
            for dist in 0..=(col.trail_len + 1) {
                let cell_y = head_cell - dist as i32;
                if cell_y < 0 || cell_y >= rows { continue; }
                let brightness = col.brightness_at(dist);
                if brightness == 0 { continue; }

                let ch = chars[rng.gen_range(0..chars.len())];
                let is_head = dist == 0;
                let color = if is_head {
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

                let dst = Rect::new(col.col_x * CELL_W, cell_y * CELL_H, CELL_W as u32, CELL_H as u32);
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
        if p.exists() { return Ok(p); }
    }
    anyhow::bail!("No monospace font found. Install fonts-dejavu-core or fonts-liberation.")
}
```

**Step 2: Verify it compiles**

```bash
cargo check
```

Expected: no errors. Install SDL2 dev if missing: `sudo apt install libsdl2-dev libsdl2-ttf-dev`

**Step 3: Quick smoke test** (visual)

Temporarily call the renderer from main:

```rust
fn main() {
    let cfg = config::Config::load();
    render::run_screensaver(&cfg).unwrap();
}
```

Run: `cargo run` — fullscreen matrix rain should appear. Press Escape to exit. Then revert main.rs to just the placeholder.

**Step 4: Commit**

```bash
git add src/render/mod.rs src/main.rs
git commit -m "feat: add SDL2 fullscreen matrix rain renderer"
```

---

## Task 5: IdleDetector trait

**Files:**
- Create: `src/idle/mod.rs`

**Step 1: Write the trait and event type**

```rust
// src/idle/mod.rs
use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;

pub mod wayland;
pub mod dbus;
pub mod x11;

#[derive(Debug, Clone, PartialEq)]
pub enum IdleEvent {
    Idle,
    Wake,
}

#[async_trait]
pub trait IdleDetector: Send + Sync {
    /// Returns true if this backend is available on the current system.
    async fn is_available(&self) -> bool;
    /// Block and send IdleEvent::Idle / IdleEvent::Wake on the channel.
    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()>;
}

/// Try backends in order, return the first available one.
pub async fn detect_backend() -> Box<dyn IdleDetector> {
    let backends: Vec<Box<dyn IdleDetector>> = vec![
        Box::new(wayland::WaylandIdleDetector),
        Box::new(dbus::DbusIdleDetector),
        Box::new(x11::X11IdleDetector),
    ];
    for backend in backends {
        if backend.is_available().await {
            return backend;
        }
    }
    panic!("No idle detection backend available. Ensure you are running X11 or a supported Wayland compositor.");
}
```

Add to `src/main.rs`:

```rust
mod idle;
```

**Step 2: Verify it compiles (with stub modules)**

Create empty stubs first — add `src/idle/wayland.rs`, `src/idle/dbus.rs`, `src/idle/x11.rs` each with:

```rust
// stub
```

Then run:

```bash
cargo check
```

**Step 3: Commit**

```bash
git add src/idle/
git commit -m "feat: add IdleDetector trait and backend selection"
```

---

## Task 6: Wayland idle backend

**Files:**
- Modify: `src/idle/wayland.rs`

The `ext-idle-notify-v1` protocol lets us register a timeout notification. When the compositor fires `idled`, we send `IdleEvent::Idle`. When it fires `resumed`, we send `IdleEvent::Wake`.

**Step 1: Implement wayland.rs**

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};

pub struct WaylandIdleDetector;

#[async_trait]
impl IdleDetector for WaylandIdleDetector {
    async fn is_available(&self) -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        use wayland_client::{Connection, Dispatch, QueueHandle, globals::registry_queue_init};
        use wayland_protocols::ext::idle_notify::v1::client::{
            ext_idle_notifier_v1::ExtIdleNotifierV1,
            ext_idle_notification_v1::{self, ExtIdleNotificationV1},
        };
        use wayland_client::protocol::wl_seat::WlSeat;

        let conn = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<AppState>(&conn)?;
        let qh = queue.handle();

        let notifier: ExtIdleNotifierV1 = globals.bind(&qh, 1..=1, ())?;
        let seat: WlSeat = globals.bind(&qh, 1..=8, ())?;

        let timeout_ms = (timeout_secs * 1000) as u32;
        let _notification = notifier.get_idle_notification(timeout_ms, &seat, &qh, ());

        let mut state = AppState { tx: tx.clone(), notifier };

        loop {
            queue.blocking_dispatch(&mut state)?;
        }
    }
}

struct AppState {
    tx: mpsc::Sender<IdleEvent>,
    notifier: wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1,
}

use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notifier_v1::ExtIdleNotifierV1,
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
};
use wayland_client::protocol::wl_seat::WlSeat;

impl Dispatch<ExtIdleNotificationV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                let _ = state.tx.blocking_send(IdleEvent::Idle);
            }
            ext_idle_notification_v1::Event::Resumed => {
                let _ = state.tx.blocking_send(IdleEvent::Wake);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for AppState {
    fn event(_: &mut Self, _: &ExtIdleNotifierV1, _: wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::Event, _: &(), _: &wayland_client::Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlSeat, ()> for AppState {
    fn event(_: &mut Self, _: &WlSeat, _: wayland_client::protocol::wl_seat::Event, _: &(), _: &wayland_client::Connection, _: &QueueHandle<Self>) {}
}

// Required for globals binding
wayland_client::delegate_noop!(AppState: ignore wayland_client::protocol::wl_registry::WlRegistry);
```

**Step 2: Verify compilation**

```bash
cargo check
```

Fix any import errors — wayland crate paths can be verbose. Check `wayland-protocols` docs for exact module paths if needed.

**Step 3: Commit**

```bash
git add src/idle/wayland.rs
git commit -m "feat: add Wayland ext-idle-notify-v1 idle backend"
```

---

## Task 7: D-Bus idle backend (GNOME / KDE)

**Files:**
- Modify: `src/idle/dbus.rs`

This backend polls `org.freedesktop.ScreenSaver.GetSessionIdleTime` (KDE) and falls back to `org.gnome.Mutter.IdleMonitor.GetIdletime`. Both return milliseconds since last activity.

**Step 1: Implement dbus.rs**

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};
use zbus::Connection;

pub struct DbusIdleDetector;

#[async_trait]
impl IdleDetector for DbusIdleDetector {
    async fn is_available(&self) -> bool {
        // Available if we can connect to session bus
        Connection::session().await.is_ok()
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        let conn = Connection::session().await?;
        let timeout_ms = timeout_secs * 1000;
        let poll_interval = tokio::time::Duration::from_secs(5);
        let mut was_idle = false;

        loop {
            tokio::time::sleep(poll_interval).await;
            let idle_ms = get_idle_ms(&conn).await.unwrap_or(0);

            if !was_idle && idle_ms >= timeout_ms {
                was_idle = true;
                let _ = tx.send(IdleEvent::Idle).await;
            } else if was_idle && idle_ms < timeout_ms {
                was_idle = false;
                let _ = tx.send(IdleEvent::Wake).await;
            }
        }
    }
}

async fn get_idle_ms(conn: &Connection) -> Result<u64> {
    // Try KDE / freedesktop screensaver first
    if let Ok(ms) = kde_idle_ms(conn).await {
        return Ok(ms);
    }
    // Try GNOME Mutter
    if let Ok(ms) = gnome_idle_ms(conn).await {
        return Ok(ms);
    }
    anyhow::bail!("No idle time source found on D-Bus")
}

async fn kde_idle_ms(conn: &Connection) -> Result<u64> {
    let reply: u32 = conn
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "GetSessionIdleTime",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(reply as u64)
}

async fn gnome_idle_ms(conn: &Connection) -> Result<u64> {
    let reply: u64 = conn
        .call_method(
            Some("org.gnome.Mutter.IdleMonitor"),
            "/org/gnome/Mutter/IdleMonitor/Core",
            Some("org.gnome.Mutter.IdleMonitor"),
            "GetIdletime",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(reply)
}
```

**Step 2: Verify compilation**

```bash
cargo check
```

**Step 3: Commit**

```bash
git add src/idle/dbus.rs
git commit -m "feat: add D-Bus idle backend for GNOME and KDE"
```

---

## Task 8: X11 idle backend

**Files:**
- Modify: `src/idle/x11.rs`

Uses `x11rb`'s screensaver extension `QueryInfo` to get milliseconds since last input.

**Step 1: Implement x11.rs**

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use anyhow::Result;
use super::{IdleDetector, IdleEvent};

pub struct X11IdleDetector;

#[async_trait]
impl IdleDetector for X11IdleDetector {
    async fn is_available(&self) -> bool {
        std::env::var("DISPLAY").is_ok()
    }

    async fn run(&self, timeout_secs: u64, tx: mpsc::Sender<IdleEvent>) -> Result<()> {
        use x11rb::connection::Connection;
        use x11rb::protocol::screensaver;

        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let timeout_ms = timeout_secs * 1000;
        let poll = tokio::time::Duration::from_secs(5);
        let mut was_idle = false;

        loop {
            tokio::time::sleep(poll).await;
            let info = screensaver::query_info(&conn, root)?.reply()?;
            let idle_ms = info.ms_since_user_input as u64;

            if !was_idle && idle_ms >= timeout_ms {
                was_idle = true;
                let _ = tx.send(IdleEvent::Idle).await;
            } else if was_idle && idle_ms < timeout_ms {
                was_idle = false;
                let _ = tx.send(IdleEvent::Wake).await;
            }
        }
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check
```

**Step 3: Commit**

```bash
git add src/idle/x11.rs
git commit -m "feat: add X11 MIT-SCREEN-SAVER idle backend"
```

---

## Task 9: Wire everything together in main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement main.rs**

```rust
mod config;
mod idle;
mod render;

use tokio::sync::mpsc;
use idle::IdleEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::load();
    let backend = idle::detect_backend().await;

    let (tx, mut rx) = mpsc::channel::<IdleEvent>(8);

    let timeout = config.idle_timeout_secs;
    tokio::spawn(async move {
        if let Err(e) = backend.run(timeout, tx).await {
            eprintln!("Idle backend error: {e}");
        }
    });

    while let Some(event) = rx.recv().await {
        match event {
            IdleEvent::Idle => {
                // Run screensaver on a blocking thread (SDL2 is not async)
                let cfg = config.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    render::run_screensaver(&cfg)
                });
                // Wait for screensaver to exit (user woke machine)
                if let Err(e) = handle.await? {
                    eprintln!("Screensaver error: {e}");
                }
            }
            IdleEvent::Wake => {
                // Already handled — SDL2 exits on mouse move / key press
            }
        }
    }

    Ok(())
}
```

**Step 2: Build release binary**

```bash
cargo build --release
```

Expected: binary at `target/release/matrix-screensaver`.

**Step 3: Smoke test**

Lower idle timeout temporarily in config:

```bash
mkdir -p ~/.config/matrix-screensaver
echo 'idle_timeout_secs = 10' > ~/.config/matrix-screensaver/config.toml
```

Run the binary, wait 10 seconds without touching the machine — screensaver should appear. Move the mouse — it should disappear.

```bash
./target/release/matrix-screensaver
```

Reset config after testing: `rm ~/.config/matrix-screensaver/config.toml`

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire idle detection to screensaver in main"
```

---

## Task 10: systemd service and README

**Files:**
- Create: `matrix-screensaver.service`
- Create: `README.md`

**Step 1: Write systemd unit**

```ini
[Unit]
Description=Matrix Screensaver
After=graphical-session.target
PartOf=graphical-session.target

[Service]
ExecStart=%h/.local/bin/matrix-screensaver
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
```

**Step 2: Write README.md**

```markdown
# matrix-screensaver

A Matrix-style falling character rain screensaver for Linux. Supports X11 and all major Wayland compositors (Sway, Hyprland, GNOME, KDE Plasma).

## Install

```bash
cargo build --release
cp target/release/matrix-screensaver ~/.local/bin/
```

Requires `libsdl2` and `libsdl2-ttf`. On Debian/Ubuntu:
```bash
sudo apt install libsdl2-2.0-0 libsdl2-ttf-2.0-0
```

## Autostart (systemd)

```bash
mkdir -p ~/.config/systemd/user
cp matrix-screensaver.service ~/.config/systemd/user/
systemctl --user enable --now matrix-screensaver
```

## Config

`~/.config/matrix-screensaver/config.toml`:

```toml
idle_timeout_secs = 300
color = "#00ff00"
fps = 30
speed = 1.0
charset = "katakana"  # katakana | latin | digits | mixed
```

## Idle Detection

Backends tried in order:
1. `ext-idle-notify-v1` (Wayland: Sway, Hyprland, KDE Plasma 6+)
2. `org.freedesktop.ScreenSaver` D-Bus (GNOME, KDE)
3. X11 MIT-SCREEN-SAVER extension

## License

MIT
```

**Step 3: Add LICENSE file**

```
MIT License

Copyright (c) 2026 Lawrence Drew

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

**Step 4: Commit**

```bash
git add matrix-screensaver.service README.md LICENSE
git commit -m "docs: add README, systemd service, and MIT license"
```

---

## Task 11: Create GitHub repo and push

**Step 1: Create repo**

```bash
~/.local/bin/gh repo create lawrencedrew/matrix-screensaver --public --description "Matrix-style falling character screensaver for Linux (X11 + Wayland)" --source . --push
```

Expected: repo created and code pushed to `github.com/lawrencedrew/matrix-screensaver`.
