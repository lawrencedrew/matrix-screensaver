# Matrix Screensaver — Design Document

Date: 2026-02-24

## Overview

A Linux screensaver that displays classic Matrix-style falling character rain. Implemented as a single Rust binary that auto-detects idle via multiple backends and works universally across X11 and Wayland compositors (wlroots, GNOME, KDE, etc.).

## Architecture

Single binary with three modules:

### `idle/` — Multi-backend idle detection

Trait-based design. Backends tried in order at startup; first one that initializes successfully is used.

1. **`wayland.rs`** — `ext-idle-notify-v1` Wayland protocol (covers wlroots compositors: Sway, Hyprland, River; and KDE Plasma 6+). Uses `wayland-client` crate.
2. **`dbus.rs`** — `org.freedesktop.ScreenSaver` D-Bus interface (covers GNOME and other DEs). Uses `zbus` crate.
3. **`x11.rs`** — X11 MIT-SCREEN-SAVER extension fallback. Uses `x11rb` crate.

Each backend emits two events: `Idle` and `Wake`. On `Idle` → open screensaver window. On `Wake` → close it.

### `render/` — Matrix rain animation

Rendered via SDL2 into a fullscreen window. SDL2 uses the native Wayland backend or Xlib automatically — same code path for both.

- **Columns:** each tracks an independent drop with random speed
- **Drop head:** bright white character, randomly changes each frame
- **Trail:** fades bright green → green → dark green → transparent over ~20 cells
- **Characters:** katakana (ｦ–ﾟ), latin letters, digits — randomly assigned per cell
- **Font:** monospace bitmap font embedded in binary (no system font dependency)
- **Frame rate:** capped at 30fps

### `config.rs` — Configuration

Reads `~/.config/matrix-screensaver/config.toml`. All fields optional with defaults.

```toml
idle_timeout_secs = 300   # default: 5 minutes
color = "#00ff00"          # default: green
fps = 30
speed = 1.0               # multiplier
charset = "katakana"       # "katakana" | "latin" | "digits" | "mixed"
```

## Project Structure

```
matrix-screensaver/
├── src/
│   ├── main.rs
│   ├── idle/
│   │   ├── mod.rs         # IdleDetector trait + backend selection
│   │   ├── wayland.rs     # ext-idle-notify-v1
│   │   ├── dbus.rs        # org.freedesktop.ScreenSaver
│   │   └── x11.rs         # MIT-SCREEN-SAVER
│   ├── render/
│   │   ├── mod.rs
│   │   └── matrix.rs      # rain animation logic
│   └── config.rs
├── Cargo.toml
├── matrix-screensaver.service   # systemd user service
├── README.md
├── LICENSE                      # MIT
└── docs/plans/
```

## Installation

**Build:** `cargo build --release` → single binary `target/release/matrix-screensaver`

**Runtime dependency:** SDL2 (dynamically linked, available on all distros)

**Manual install:** copy binary to `/usr/local/bin/`

**Autostart via systemd user service:**

```ini
[Unit]
Description=Matrix Screensaver

[Service]
ExecStart=/usr/local/bin/matrix-screensaver
Restart=on-failure

[Install]
WantedBy=default.target
```

Enable: `systemctl --user enable --now matrix-screensaver`

## Key Crates

- `wayland-client` — Wayland protocol implementation
- `zbus` — D-Bus (async)
- `x11rb` — X11 protocol
- `sdl2` — cross-platform graphics window
- `serde` + `toml` — config parsing
- `tokio` — async runtime for idle backends
