# Visual Effects & Packaging Design

**Goal:** Add fade-in, variable column density, glitch effect, and wake animation to the screensaver; add AUR PKGBUILD, .deb packaging, and GitHub Actions CI.

**Architecture:** All visual changes are isolated to `src/render/` — no changes to idle detection or config loading. Packaging files live in `pkg/`. CI lives in `.github/workflows/`.

---

## Visual Effects

### Fade In
- Track `startup_elapsed` from when `run_screensaver` is called
- Multiply all brightness values by `(elapsed_secs / 2.0).min(1.0)`
- Applied in the render loop before color calculation

### Variable Column Density
- At init, randomly skip ~30% of column slots (sparse columns Vec)
- Widen trail length range: 5–20 cells (currently fixed)
- Speed variance already exists but can be widened too

### Glitch Effect
- Per-column, ~0.5% chance per frame to flash a random cell as full-brightness white
- Applied after normal brightness calculation, independent of trail position

### Wake Animation
- On any dismiss event (key, click, mouse motion), set `exiting = true` instead of breaking immediately
- While `exiting`: multiply all column speeds by 5x
- After 600ms, break the loop and exit

---

## Packaging

### AUR PKGBUILD (`pkg/aur/PKGBUILD`)
- Sources GitHub release tarball by version tag
- Builds with `cargo build --release`
- Installs binary to `/usr/bin/` and service file to `/usr/lib/systemd/user/`
- `depends`: sdl2, sdl2_ttf, noto-fonts-cjk

### .deb (`[package.metadata.deb]` in Cargo.toml)
- Uses `cargo-deb` crate (dev dependency)
- Includes binary + systemd service file
- `depends`: libsdl2-2.0-0, libsdl2-ttf-2.0-0, fonts-noto-cjk

### GitHub Actions (`.github/workflows/release.yml`)
- Triggers on `v*` tags
- Steps:
  1. `cargo build --release` for Linux x86_64
  2. `cargo deb` to produce `.deb`
  3. Create `.tar.gz` of binary + service + README
  4. Create GitHub Release and upload both artifacts
