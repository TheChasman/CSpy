use std::collections::HashMap;
use std::sync::Mutex;
use tauri::image::Image;

pub(crate) const ICON_WIDTH: u32 = 32;
pub(crate) const ICON_HEIGHT: u32 = 32;

/// Cache of rendered icon buffers, keyed by quantised utilisation (0–20 = 5% steps).
/// Maximum 21 entries × 4 KiB = 84 KiB total — bounded, no unbounded leak.
static ICON_CACHE: Mutex<Option<HashMap<u8, &'static [u8]>>> = Mutex::new(None);

/// 5x7 bitmap font for countdown text. Each glyph is 7 rows of 5 bits.
/// Bit 4 = leftmost pixel, bit 0 = rightmost pixel.
const GLYPH_0: [u8; 7] = [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110];
const GLYPH_1: [u8; 7] = [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110];
const GLYPH_2: [u8; 7] = [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111];
const GLYPH_3: [u8; 7] = [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110];
const GLYPH_4: [u8; 7] = [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010];
const GLYPH_5: [u8; 7] = [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110];
const GLYPH_6: [u8; 7] = [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110];
const GLYPH_7: [u8; 7] = [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000];
const GLYPH_8: [u8; 7] = [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110];
const GLYPH_9: [u8; 7] = [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110];
const GLYPH_H: [u8; 7] = [0b10000, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001];
const GLYPH_M: [u8; 7] = [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101];

/// Return the 5x7 glyph for a character, or None for space/unknown.
fn glyph_for_char(ch: char) -> Option<&'static [u8; 7]> {
    match ch {
        '0' => Some(&GLYPH_0),
        '1' => Some(&GLYPH_1),
        '2' => Some(&GLYPH_2),
        '3' => Some(&GLYPH_3),
        '4' => Some(&GLYPH_4),
        '5' => Some(&GLYPH_5),
        '6' => Some(&GLYPH_6),
        '7' => Some(&GLYPH_7),
        '8' => Some(&GLYPH_8),
        '9' => Some(&GLYPH_9),
        'h' => Some(&GLYPH_H),
        'm' => Some(&GLYPH_M),
        ' ' => None,
        _ => None,
    }
}

/// Glyph render width at 2x scale.
const GLYPH_RENDER_W: u32 = 10;
/// Glyph render height at 2x scale.
const GLYPH_RENDER_H: u32 = 14;
/// Pixels between glyphs.
const CHAR_GAP: u32 = 1;
/// Pixels for a space character.
const SPACE_WIDTH: u32 = 6;

/// Calculate the total pixel width of rendered text.
fn text_pixel_width(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut width: u32 = 0;
    let mut first = true;
    for ch in text.chars() {
        if !first && ch != ' ' {
            width += CHAR_GAP;
        }
        first = false;
        if ch == ' ' {
            width += SPACE_WIDTH;
        } else {
            width += GLYPH_RENDER_W;
        }
    }
    width
}

/// Render countdown text into an RGBA buffer at the given x offset.
/// Glyphs are drawn at 2x scale, vertically centred in ICON_HEIGHT.
fn render_text_into(
    rgba: &mut [u8],
    buf_width: u32,
    x_start: u32,
    text: &str,
    colour: (u8, u8, u8, u8),
) {
    let text_h = GLYPH_RENDER_H;
    let y_offset = (ICON_HEIGHT - text_h) / 2;

    let mut cursor_x = x_start;
    let mut first = true;

    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += SPACE_WIDTH;
            first = false;
            continue;
        }
        if !first {
            cursor_x += CHAR_GAP;
        }
        first = false;

        if let Some(glyph) = glyph_for_char(ch) {
            for glyph_row in 0..7u32 {
                let row_bits = glyph[glyph_row as usize];
                for glyph_col in 0..5u32 {
                    if (row_bits >> (4 - glyph_col)) & 1 == 1 {
                        for dy in 0..2u32 {
                            for dx in 0..2u32 {
                                let px = cursor_x + glyph_col * 2 + dx;
                                let py = y_offset + glyph_row * 2 + dy;
                                if px < buf_width && py < ICON_HEIGHT {
                                    let idx = ((py * buf_width + px) * 4) as usize;
                                    rgba[idx] = colour.0;
                                    rgba[idx + 1] = colour.1;
                                    rgba[idx + 2] = colour.2;
                                    rgba[idx + 3] = colour.3;
                                }
                            }
                        }
                    }
                }
            }
            cursor_x += GLYPH_RENDER_W;
        }
    }
}

/// Render raw RGBA bytes for a 32×32 usage icon at the given utilisation level.
/// Pure function — no caching, no tauri dependency. Used directly by tests.
pub(crate) fn render_icon_rgba(quantised_util: f64) -> Vec<u8> {
    const BORDER: u32 = 2;
    const PADDING: u32 = 4;

    let fill_color: (u8, u8, u8) = if quantised_util >= 0.90 {
        (248, 113, 113) // Red: #f87171
    } else if quantised_util >= 0.70 {
        (251, 191, 36)  // Amber: #fbbf24
    } else {
        (74, 222, 128)  // Green: #4ade80
    };

    let outline_color: (u8, u8, u8) = (60, 60, 60);

    let inner_left = BORDER;
    let inner_right = ICON_WIDTH - BORDER;
    let inner_top = PADDING;
    let inner_bottom = ICON_HEIGHT - PADDING;
    let inner_width = inner_right - inner_left - 2 * BORDER;
    let fill_width = ((inner_width as f64 * quantised_util) as u32).min(inner_width);

    let mut rgba = vec![0u8; (ICON_WIDTH * ICON_HEIGHT * 4) as usize];

    for y in 0..ICON_HEIGHT {
        for x in 0..ICON_WIDTH {
            let pixel_idx = ((y * ICON_WIDTH + x) * 4) as usize;

            let (r, g, b, a) = if y < inner_top || y >= inner_bottom {
                (0, 0, 0, 0)
            } else if x < inner_left + BORDER || x >= inner_right - BORDER
                || y < inner_top + BORDER || y >= inner_bottom - BORDER {
                (outline_color.0, outline_color.1, outline_color.2, 255)
            } else {
                let inner_x = x - inner_left - BORDER;
                if inner_x < fill_width {
                    (fill_color.0, fill_color.1, fill_color.2, 255)
                } else {
                    (180, 180, 180, 80)
                }
            };

            rgba[pixel_idx] = r;
            rgba[pixel_idx + 1] = g;
            rgba[pixel_idx + 2] = b;
            rgba[pixel_idx + 3] = a;
        }
    }

    rgba
}

/// Generate a dynamic usage icon: hollow rectangle with coloured fill based on utilisation.
/// Renders at 32×32 for Retina crispness. macOS menu bar expects @2x icons.
///
/// Icons are cached by quantised utilisation (5% steps) so each unique level
/// is only rendered once. The leaked buffers are bounded to ~84 KiB total.
pub fn generate_usage_icon(utilisation: f64) -> Image<'static> {
    let util = utilisation.clamp(0.0, 1.0);
    let key = (util * 20.0).round() as u8;

    {
        let mut guard = ICON_CACHE.lock().unwrap();
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(rgba_ref) = cache.get(&key) {
            return Image::new(rgba_ref, ICON_WIDTH, ICON_HEIGHT);
        }
    }

    let quantised_util = key as f64 / 20.0;
    let rgba = render_icon_rgba(quantised_util);

    let rgba_static: &'static [u8] = Box::leak(rgba.into_boxed_slice());

    let mut guard = ICON_CACHE.lock().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    cache.insert(key, rgba_static);

    Image::new(rgba_static, ICON_WIDTH, ICON_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
        let idx = ((y * ICON_WIDTH + x) * 4) as usize;
        (rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3])
    }

    fn count_interior_pixels_with_rgb(rgba: &[u8], rgb: (u8, u8, u8)) -> u32 {
        let mut count = 0;
        for y in 6..26 {
            for x in 4..28 {
                let (r, g, b, _) = pixel_at(rgba, x, y);
                if (r, g, b) == rgb {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn dimensions_are_32x32() {
        let rgba = render_icon_rgba(0.5);
        assert_eq!(rgba.len(), (32 * 32 * 4) as usize);
    }

    #[test]
    fn zero_percent_has_no_fill_pixels() {
        let rgba = render_icon_rgba(0.0);
        let green = count_interior_pixels_with_rgb(&rgba, (74, 222, 128));
        assert_eq!(green, 0, "0% should have no green fill pixels");
    }

    #[test]
    fn fifty_percent_uses_green() {
        let rgba = render_icon_rgba(0.5);
        let green = count_interior_pixels_with_rgb(&rgba, (74, 222, 128));
        let grey = count_interior_pixels_with_rgb(&rgba, (180, 180, 180));
        assert!(green > 0, "50% should have green fill pixels");
        assert!(grey > 0, "50% should have empty grey pixels too");
    }

    #[test]
    fn seventy_percent_uses_amber() {
        let rgba = render_icon_rgba(0.70);
        let amber = count_interior_pixels_with_rgb(&rgba, (251, 191, 36));
        assert!(amber > 0, "70% should use amber fill");
    }

    #[test]
    fn ninety_percent_uses_red() {
        let rgba = render_icon_rgba(0.90);
        let red = count_interior_pixels_with_rgb(&rgba, (248, 113, 113));
        assert!(red > 0, "90% should use red fill");
    }

    #[test]
    fn hundred_percent_fills_entire_interior() {
        let rgba = render_icon_rgba(1.0);
        let grey = count_interior_pixels_with_rgb(&rgba, (180, 180, 180));
        assert_eq!(grey, 0, "100% should have no empty grey pixels in interior");
    }

    #[test]
    fn padding_rows_are_transparent() {
        let rgba = render_icon_rgba(0.5);
        for y in 0..4 {
            for x in 0..ICON_WIDTH {
                let (_, _, _, a) = pixel_at(&rgba, x, y);
                assert_eq!(a, 0, "pixel ({x},{y}) in top padding should be transparent");
            }
        }
        for y in 28..ICON_HEIGHT {
            for x in 0..ICON_WIDTH {
                let (_, _, _, a) = pixel_at(&rgba, x, y);
                assert_eq!(a, 0, "pixel ({x},{y}) in bottom padding should be transparent");
            }
        }
    }

    #[test]
    fn glyph_coverage_all_countdown_chars() {
        for ch in "0123456789hm ".chars() {
            assert!(
                glyph_for_char(ch).is_some() || ch == ' ',
                "missing glyph for '{ch}'"
            );
        }
    }

    #[test]
    fn glyphs_are_5x7() {
        for ch in "0123456789hm".chars() {
            let glyph = glyph_for_char(ch).unwrap();
            assert_eq!(glyph.len(), 7, "glyph for '{ch}' should have 7 rows");
            for (row_idx, row) in glyph.iter().enumerate() {
                assert!(
                    *row < 32,
                    "glyph '{ch}' row {row_idx} uses more than 5 bits: {row:#010b}"
                );
            }
        }
    }

    #[test]
    fn text_width_single_digit() {
        assert_eq!(text_pixel_width("5m"), 21);
    }

    #[test]
    fn text_width_hours_and_mins() {
        assert_eq!(text_pixel_width("3h 42m"), 60);
    }

    #[test]
    fn text_width_empty() {
        assert_eq!(text_pixel_width(""), 0);
    }

    #[test]
    fn render_text_produces_nonzero_pixels() {
        let width: u32 = 40;
        let height: u32 = 32;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        render_text_into(&mut rgba, width, 0, "5m", (220, 220, 220, 255));
        let has_visible = rgba.chunks(4).any(|px| px[3] > 0);
        assert!(has_visible, "render_text_into should produce visible pixels");
    }

    #[test]
    fn render_text_respects_x_offset() {
        let width: u32 = 80;
        let height: u32 = 32;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        render_text_into(&mut rgba, width, 40, "1m", (220, 220, 220, 255));
        for y in 0..height {
            for x in 0..40u32 {
                let idx = ((y * width + x) * 4 + 3) as usize;
                assert_eq!(rgba[idx], 0, "pixel ({x},{y}) before offset should be transparent");
            }
        }
        let has_visible_after = (0..height).any(|y| {
            (40..width).any(|x| rgba[((y * width + x) * 4 + 3) as usize] > 0)
        });
        assert!(has_visible_after, "should have visible pixels after x=40");
    }
}
