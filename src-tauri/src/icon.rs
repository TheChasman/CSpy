use tauri::image::Image;

/// Generate a dynamic usage icon: hollow rectangle with coloured fill based on utilisation.
/// Renders at 32×32 for Retina crispness. macOS menu bar expects @2x icons.
pub fn generate_usage_icon(utilisation: f64) -> Image<'static> {
    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    const BORDER: u32 = 2;   // 2px outline for visibility at @2x
    const PADDING: u32 = 4;  // Vertical padding so it's not a full square

    // Clamp utilisation to 0.0-1.0
    let util = utilisation.max(0.0).min(1.0);

    // Determine fill colour based on utilisation
    let fill_color: (u8, u8, u8) = if util >= 0.90 {
        (248, 113, 113) // Red: #f87171
    } else if util >= 0.70 {
        (251, 191, 36)  // Amber: #fbbf24
    } else {
        (74, 222, 128)  // Green: #4ade80
    };

    // Dark outline for contrast on light menu bar
    let outline_color: (u8, u8, u8) = (60, 60, 60); // Dark grey

    // Interior dimensions (inside border, inside padding)
    let inner_left = BORDER;
    let inner_right = WIDTH - BORDER;
    let inner_top = PADDING;
    let inner_bottom = HEIGHT - PADDING;
    let inner_width = inner_right - inner_left - 2 * BORDER; // space inside the border
    let fill_width = ((inner_width as f64 * util) as u32).min(inner_width);

    let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel_idx = ((y * WIDTH + x) * 4) as usize;

            let (r, g, b, a) = if y < inner_top || y >= inner_bottom {
                // Above/below the bar — fully transparent
                (0, 0, 0, 0)
            } else if x < inner_left || x >= inner_right
                || y < inner_top + BORDER || y >= inner_bottom - BORDER {
                // Border region
                (outline_color.0, outline_color.1, outline_color.2, 255)
            } else {
                // Interior
                let inner_x = x - inner_left - BORDER;
                if inner_x < fill_width {
                    // Filled: colour
                    (fill_color.0, fill_color.1, fill_color.2, 255)
                } else {
                    // Hollow: semi-transparent to show the bar outline without looking solid
                    (180, 180, 180, 80)
                }
            };

            rgba[pixel_idx] = r;
            rgba[pixel_idx + 1] = g;
            rgba[pixel_idx + 2] = b;
            rgba[pixel_idx + 3] = a;
        }
    }

    let rgba_static: &'static [u8] = Box::leak(rgba.into_boxed_slice());
    Image::new(rgba_static, WIDTH, HEIGHT)
}
