# Render Countdown Text into Tray Icon

**Date:** 2026-04-11
**Status:** Approved
**Problem:** macOS 26 (Tahoe) breaks `TrayIcon::set_title()` — the call succeeds but no text renders next to the icon. The progress bar icon still displays. This is a known Tauri/macOS regression (tauri-apps/tauri#13770).

**Solution:** Render the countdown text ("3h 42m", "42m") directly into the icon's RGBA pixel buffer using a hand-coded 5x7 bitmap font. Bypass `set_title()` entirely.

## Icon Layout

Variable width, fixed 32px height:

```
[4px pad] [bar 24x20] [4px gap] [text ~50px] [4px pad]
```

- **Bar:** Unchanged from current design. 24px wide, 20px tall (with 2px border, 4px vertical padding). Colour-coded: green (<70%), amber (70-89%), red (>=90%).
- **Gap:** 4px transparent space between bar and text.
- **Text:** 5x7 bitmap glyphs scaled 2x to 10x14 pixels, vertically centred. 2px inter-character spacing.
- **Total width examples:** "3h 42m" = 6 glyphs = ~86px. "42m" = 3 glyphs = ~56px. No text (expired) = 32px (current icon).

macOS menu bar uses @2x icons, so pixel dimensions display at half in points (e.g. 86px icon = ~43pt).

## Bitmap Font

Hand-coded pixel arrays for 13 characters: `0123456789hm `. Each glyph is a 5x7 grid stored as `[u8; 7]` where each byte encodes 5 pixels as bits. Scaled 2x at render time (10x14 actual pixels). No external font files or crate dependencies.

## Text Colour

`(220, 220, 220, 255)` — light grey. Reads well against the macOS dark menu bar. Since `icon_as_template` is `false`, we control the colour directly.

## Icon Cache

Current cache key: `u8` (quantised utilisation, 0-20 in 5% steps).

New cache key: `(u8, Option<u16>)` — quantised utilisation + countdown in whole minutes. `None` = no active window (bar-only icon).

Bounded: utilisation has 21 values, countdown changes once per minute and is evicted when utilisation changes. Practical maximum ~21-30 entries at any time. Each entry is a leaked `&'static [u8]` of variable size (max ~350 bytes for widest icon).

## API Changes

### `icon.rs`

- `render_icon_rgba(quantised_util: f64)` signature changes to `render_icon_rgba(quantised_util: f64, countdown: Option<&str>)`.
- `generate_usage_icon(utilisation: f64)` signature changes to `generate_usage_icon(utilisation: f64, countdown: Option<&str>)`.
- New private constants/functions: `GLYPHS` array, `glyph_for_char()`, `render_text()`.
- `ICON_WIDTH` becomes a function of whether text is present (or a computed constant per call).

### `lib.rs`

- `update_tray_title()` is removed. All callers replaced with icon regeneration that includes countdown text.
- `start_countdown_ticker()` regenerates the icon (with text) every 60s instead of calling `set_title`.
- `start_polling()` success path passes countdown to `generate_usage_icon`.
- Initial fetch in `setup` passes countdown to `generate_usage_icon`.
- `format_countdown()` stays — it now feeds the icon renderer instead of `set_title`.
- When `format_countdown` returns "---" (expired/no window), pass `None` to `generate_usage_icon` to get the bar-only 32px icon.

### Removal

- All calls to `tray.set_title()` are removed.
- The diagnostic `log::debug!("Tray title -> ...")` line (added during this debugging session) is removed.

## Expired / No-Window State

When `five_hour` is `None`, `resets_at` is `None`, or the window has expired: render the 32x32 bar-only icon at 0% utilisation. No text. This is visually cleaner than showing "---".

## Testing

Existing `icon.rs` tests continue to work (they test `render_icon_rgba` with the new `None` countdown parameter).

New tests:
- `text_adds_width` — icon with countdown text is wider than 32px.
- `no_text_is_32px` — icon with `None` countdown is 32x32.
- `text_pixels_are_nonzero` — the text region contains non-transparent pixels.
- `glyph_coverage` — all 13 characters (`0-9`, `h`, `m`, ` `) have defined glyphs.

## Out of Scope

- Anti-aliasing or sub-pixel rendering. The 2x scaled bitmap font is sharp enough at menu bar size.
- Light mode adaptation. The current icon already uses `icon_as_template(false)` with fixed colours. Light grey text on the dark macOS menu bar is readable. If a light-mode fix is needed later, it's a separate change.
- Tooltip changes. `update_tray_tooltip()` remains unchanged.
