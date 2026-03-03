# Clock Feature Design — 2026-03-03

## Summary

Add a centered green digital clock showing HH:MM:SS to the matrix screensaver, rendered with a matrix-style glitch aesthetic (chromatic aberration + brightness flicker + occasional digit randomisation).

## Architecture

The clock renderer lives in a new module `src/render/clock.rs`. It is called from the main render loop in `run_screensaver()` after drawing the rain columns, so the clock always renders on top of the rain. Only the primary display (index 0) shows the clock.

## Components

### `ClockRenderer` struct

Fields:
- `texture: Option<Texture>` — cached texture of the current time string; rebuilt once per second
- `last_second: u32` — tracks when the displayed second last changed

Methods:
- `ClockRenderer::new() -> Self`
- `render(&mut self, canvas, texture_creator, font, rng) -> anyhow::Result<()>`

### Font loading

Load the existing monospace font a second time at **72pt** alongside the 16pt matrix font. If this fails, the screensaver continues without the clock (non-fatal, logged to stderr).

### Per-frame render (`render`)

1. Get current local time; format as `"HH:MM:SS"`.
2. If second has changed, rebuild the texture via `font.render(text).blended(white)`.
3. Optionally apply digit glitch: 0.3% chance per frame, replace one random character with a Katakana char (from the charset).
4. Draw three passes:
   - **Red ghost**: `set_color_mod(180, 0, 0)`, position offset `(-3, 0)` from centre
   - **Blue ghost**: `set_color_mod(0, 0, 180)`, position offset `(+3, 0)` from centre
   - **Main green**: `set_color_mod(r_green, g_green, 0)` where `g_green = (|sin(elapsed * 2.0)| * 30.0 + 225.0) as u8`

### Centering

Use SDL2 `texture.query()` to get `(w, h)`, then place at:
```
x = (screen_w - w) / 2
y = (screen_h - h) / 2
```

## Data Flow

```
system time → format "HH:MM:SS"
           → rebuild texture if second changed
           → per-frame glitch override (0.3% chance)
           → render red ghost (−3px)
           → render blue ghost (+3px)
           → render green text (centred, flickering brightness)
```

## Error Handling

Font and texture errors propagate via `anyhow::Result`. Clock font load failure is non-fatal: log to stderr and skip clock rendering.

## Testing

No unit tests for the renderer (SDL2 not headlessly testable). Existing matrix column tests are unaffected.

## Files Changed

| File | Change |
|------|--------|
| `src/render/clock.rs` | New module — `ClockRenderer` |
| `src/render/mod.rs` | Load 72pt clock font; instantiate `ClockRenderer`; call `render` in main loop |
