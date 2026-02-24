# Visual Effects & Packaging Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add fade-in, variable column density, glitch effect, and wake animation to the screensaver; add AUR PKGBUILD, .deb packaging, and GitHub Actions CI release workflow.

**Architecture:** Visual effects are all in `src/render/mod.rs` and `src/render/matrix.rs`. No config or idle detection changes needed. Packaging files go in `pkg/aur/` and `Cargo.toml`. CI goes in `.github/workflows/`.

**Tech Stack:** Rust, SDL2, cargo-deb, GitHub Actions

---

### Task 1: Fade In

**Files:**
- Modify: `src/render/mod.rs` (render loop, ~lines 133–165)

The screensaver currently pops in instantly. Apply a 2-second fade by multiplying brightness by a ramp factor computed from `startup_time` (already tracked in the render loop).

**Step 1: Locate the brightness/color block in the render loop**

In `src/render/mod.rs`, find this block inside `for col in cols.iter_mut()`:

```rust
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
```

**Step 2: Replace it with fade-aware version**

```rust
let raw_brightness = col.brightness_at(dist);
if raw_brightness == 0 {
    continue;
}
let fade = (startup_time.elapsed().as_secs_f32() / 2.0).min(1.0);
let brightness = (raw_brightness as f32 * fade) as u8;
if brightness == 0 {
    continue;
}
let ch = chars[rng.gen_range(0..chars.len())];
let color = if dist == 0 {
    Color::RGB(
        (200.0 * fade) as u8,
        (255.0 * fade) as u8,
        (200.0 * fade) as u8,
    )
} else {
    Color::RGB(
        (base_color.r as f32 * brightness as f32 / 255.0) as u8,
        (base_color.g as f32 * brightness as f32 / 255.0) as u8,
        (base_color.b as f32 * brightness as f32 / 255.0) as u8,
    )
};
```

**Step 3: Build and check**

```bash
cd ~/matrix-screensaver && source ~/.cargo/env && cargo build --release 2>&1
```
Expected: no errors.

**Step 4: Smoke test**

```bash
systemctl --user stop matrix-screensaver
~/.local/bin/matrix-screensaver --timeout 3 &
# wait 3 seconds, observe fade-in over 2 seconds
# press any key to dismiss
```

**Step 5: Run tests**

```bash
source ~/.cargo/env && cargo test 2>&1
```
Expected: all pass.

**Step 6: Commit**

```bash
git add src/render/mod.rs
git commit -m "feat: fade in over 2 seconds on startup"
```

---

### Task 2: Variable Column Density + Wider Trail Range

**Files:**
- Modify: `src/render/mod.rs` (column init, ~lines 62–78, and rng init ~line 96)
- Modify: `src/render/matrix.rs` (`Column::new` and `update`, trail_len range)

Currently all column slots are filled and trail_len is `8..20`. Widen to `4..24` and skip ~30% of columns.

**Step 1: Move rng init before the display loop**

In `src/render/mod.rs`, find these lines (currently after the display loop and glyph cache setup, around line 96):

```rust
    use rand::Rng;
    let mut rng = rand::thread_rng();
```

Move them to just before the `for i in 0..num_displays` loop:

```rust
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // One borderless window per display, positioned at each display's bounds
    let num_displays = video.num_video_displays().unwrap_or(1) as usize;
```

**Step 2: Apply 30% density skip in column init**

Find:
```rust
        columns_per_display.push((0..cols).map(|x| Column::new(x, rows, config.speed)).collect());
```

Replace with:
```rust
        columns_per_display.push(
            (0..cols)
                .filter(|_| rng.gen::<f32>() > 0.3)
                .map(|x| Column::new(x, rows, config.speed))
                .collect(),
        );
```

**Step 3: Widen trail_len range in matrix.rs**

In `src/render/matrix.rs`, `Column::new` has:
```rust
trail_len: rng.gen_range(8..20),
```

Change to:
```rust
trail_len: rng.gen_range(4..24),
```

Also in `update()`, the reset line has:
```rust
self.trail_len = rng.gen_range(8..20);
```

Change to:
```rust
self.trail_len = rng.gen_range(4..24);
```

**Step 4: Build and check**

```bash
source ~/.cargo/env && cargo build --release 2>&1
```

**Step 5: Run tests**

```bash
source ~/.cargo/env && cargo test 2>&1
```
Expected: all pass (trail_len range change doesn't break the positive test).

**Step 6: Commit**

```bash
git add src/render/mod.rs src/render/matrix.rs
git commit -m "feat: variable column density (70%) and wider trail range (4-24)"
```

---

### Task 3: Glitch Effect

**Files:**
- Modify: `src/render/mod.rs` (inside the column render loop)

Per column, per frame: 0.5% chance to flash a random cell at full white brightness, independent of the trail.

**Step 1: Add glitch block after the trail loop**

In `src/render/mod.rs`, after the `for dist in 0..=(col.trail_len + 1)` block (still inside `for col in cols.iter_mut()`), add:

```rust
            // Glitch: 0.5% chance per frame to flash a random cell white
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
                }
            }
```

Note: `let _ =` intentionally ignores the copy error here — a glitch failing to render is fine.

**Step 2: Build and check**

```bash
source ~/.cargo/env && cargo build --release 2>&1
```

**Step 3: Run tests**

```bash
source ~/.cargo/env && cargo test 2>&1
```

**Step 4: Commit**

```bash
git add src/render/mod.rs
git commit -m "feat: glitch effect — 0.5% per-column per-frame random white flash"
```

---

### Task 4: Wake Animation

**Files:**
- Modify: `src/render/mod.rs` (event loop and render loop)

On dismiss, instead of breaking immediately: set `exit_requested`, multiply column speed 5x for 600ms, then exit.

**Step 1: Add exit_requested before the main loop**

Just before `'running: loop {`, add:

```rust
    let mut exit_requested: Option<Instant> = None;
```

**Step 2: Replace break_running events with exit_requested**

Find:
```rust
            match event {
                Event::Quit { .. }
                | Event::KeyDown { .. }
                | Event::MouseButtonDown { .. } => break 'running,
                Event::MouseMotion { .. }
                    if startup_time.elapsed() > Duration::from_millis(500) =>
                {
                    break 'running
                }
                _ => {}
            }
```

Replace with:
```rust
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
```

**Step 3: Add speed multiplier and exit check after frame timing**

After `last_frame = after_sleep;`, add:

```rust
        let speed_mult = if exit_requested.is_some() { 5.0_f32 } else { 1.0_f32 };
        if let Some(t) = exit_requested {
            if t.elapsed() > Duration::from_millis(600) {
                break 'running;
            }
        }
```

**Step 4: Apply speed_mult to column update**

Find:
```rust
                col.update(delta);
```

Replace with:
```rust
                col.update(delta * speed_mult);
```

**Step 5: Build and check**

```bash
source ~/.cargo/env && cargo build --release 2>&1
```

**Step 6: Run tests**

```bash
source ~/.cargo/env && cargo test 2>&1
```

**Step 7: Deploy and smoke test**

```bash
systemctl --user stop matrix-screensaver
cp target/release/matrix-screensaver ~/.local/bin/matrix-screensaver
~/.local/bin/matrix-screensaver --timeout 3 &
# wait 3 seconds, then press a key — rain should speed up and exit
```

**Step 8: Commit**

```bash
git add src/render/mod.rs
git commit -m "feat: wake animation — rain accelerates 5x for 600ms before exit"
```

---

### Task 5: AUR PKGBUILD

**Files:**
- Create: `pkg/aur/PKGBUILD`

**Note:** `Cargo.lock` must be committed for `--locked` to work. Check it's not in `.gitignore`:
```bash
cat ~/matrix-screensaver/.gitignore
```
If `Cargo.lock` is listed, remove it. Binary crates should commit their lockfile.

**Step 1: Create the PKGBUILD**

```bash
mkdir -p ~/matrix-screensaver/pkg/aur
```

Create `pkg/aur/PKGBUILD`:

```bash
# Maintainer: Lawrence Drew
pkgname=matrix-screensaver
pkgver=0.1.0
pkgrel=1
pkgdesc="Matrix-style falling character rain screensaver for Linux"
arch=('x86_64')
url="https://github.com/lawrencedrew/matrix-screensaver"
license=('MIT')
depends=('sdl2' 'sdl2_ttf' 'noto-fonts-cjk')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 "$pkgname.service" "$pkgdir/usr/lib/systemd/user/$pkgname.service"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
```

**Step 2: Commit**

```bash
git add pkg/aur/PKGBUILD
git commit -m "pkg: add AUR PKGBUILD"
```

---

### Task 6: .deb Packaging

**Files:**
- Modify: `Cargo.toml` (add `[package.metadata.deb]`)

Uses `cargo-deb` (installed as a tool, not a dependency).

**Step 1: Add deb metadata to Cargo.toml**

At the bottom of `Cargo.toml`, add:

```toml
[package.metadata.deb]
maintainer = "Lawrence Drew"
copyright = "2026, Lawrence Drew"
extended-description = "Matrix-style falling character rain screensaver for Linux. Supports X11 and Wayland."
depends = "$auto, fonts-noto-cjk"
section = "x11"
priority = "optional"
assets = [
    ["target/release/matrix-screensaver", "usr/bin/", "755"],
    ["matrix-screensaver.service", "usr/lib/systemd/user/", "644"],
    ["README.md", "usr/share/doc/matrix-screensaver/README.md", "644"],
]
```

**Step 2: Install cargo-deb and test the build**

```bash
source ~/.cargo/env && cargo install cargo-deb
cargo deb 2>&1
```
Expected: `target/debian/matrix-screensaver_0.1.0_amd64.deb` created.

**Step 3: Verify the .deb contents**

```bash
dpkg -c target/debian/matrix-screensaver_*.deb
```
Expected: see `./usr/bin/matrix-screensaver`, `./usr/lib/systemd/user/matrix-screensaver.service`, `./usr/share/doc/matrix-screensaver/README.md`.

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "pkg: add cargo-deb metadata for .deb packaging"
```

---

### Task 7: GitHub Actions Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`

Triggers on `v*` tags. Builds the binary, produces a `.deb` and a `.tar.gz`, uploads both to a GitHub Release.

**Step 1: Create the workflow**

```bash
mkdir -p ~/matrix-screensaver/.github/workflows
```

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libsdl2-dev libsdl2-ttf-dev

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build release binary
        run: cargo build --release

      - name: Install cargo-deb
        run: cargo install cargo-deb

      - name: Build .deb
        run: cargo deb

      - name: Create tarball
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          mkdir dist
          cp target/release/matrix-screensaver matrix-screensaver.service README.md LICENSE dist/
          tar -czf matrix-screensaver-${VERSION}-linux-x86_64.tar.gz -C dist .

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            target/debian/*.deb
            matrix-screensaver-*.tar.gz
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: GitHub Actions release workflow on v* tags"
```

**Step 3: Push and verify workflow appears**

```bash
git push
```

Then check `https://github.com/lawrencedrew/matrix-screensaver/actions` — the workflow will appear once a `v*` tag is pushed.

**Step 4: Deploy updated service and set back to 1 minute**

```bash
systemctl --user stop matrix-screensaver
cp target/release/matrix-screensaver ~/.local/bin/matrix-screensaver
systemctl --user start matrix-screensaver
```
