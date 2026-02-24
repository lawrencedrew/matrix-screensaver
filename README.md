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
