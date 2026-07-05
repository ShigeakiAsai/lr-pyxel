//! splash.rs — lr-pyxel splash screen
//!
//! Displays for SPLASH_FRAMES after content load:
//!   - Isometric cube outline (background, dark gray)
//!   - "libretro" text (small, left-aligned to "pyxel")
//!   - "pyxel" in 4x pixel-art letters (white, centered)
//!   - "Powered by Lakka" (bottom right)
//!   - version number (bottom left)

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Draw the splash screen onto Pyxel's internal buffer.
pub fn draw() {
    let px = pyxel_core::pyxel();

    // Background
    px.clear(0);

    let col: u8 = 5; // dark gray

    // --- Isometric cube outline (background) ---
    let cx = 64.0f32;  // center x (128px screen / 2)
    let ty = 15.0f32;  // top point y
    let my = 33.0f32;  // middle y (top face bottom / left+right peak)
    let by = 51.0f32;  // bottom of top face = center vertical
    let lx = 22.0f32;  // left x
    let rx = 106.0f32; // right x
    let bly = 105.0f32; // bottom left y
    let bry = 105.0f32; // bottom right y
    let bcy = 123.0f32; // bottom center y

    // Top face
    px.draw_line(cx, ty, rx, my, col);
    px.draw_line(rx, my, cx, by, col);
    px.draw_line(cx, by, lx, my, col);
    px.draw_line(lx, my, cx, ty, col);

    // Left face
    px.draw_line(lx, my, lx, bly, col);
    px.draw_line(lx, bly, cx, bcy, col);
    px.draw_line(cx, by, cx, bcy, col);

    // Right face
    px.draw_line(rx, my, rx, bry, col);
    px.draw_line(rx, bry, cx, bcy, col);

    // --- "libretro" text (left-aligned to pyxel, color 13 = gray) ---
    px.draw_text(14.0, 42.0, "libretro", 13, None);

    // --- "pyxel" in 4x pixel-art letters (white = color 7) ---
    draw_pyxel_text(px, 14, 50, 7);

    // --- "Powered by Lakka" bottom right (color 5) ---
    px.draw_text(50.0, 120.0, "Powered by Lakka", 5, None);

    // --- Version bottom left (color 5) ---
    px.draw_text(2.0, 120.0, VERSION, 5, None);
}

/// Draw "pyxel" in 4x hand-crafted pixel font.
fn draw_pyxel_text(px: &pyxel_core::Pyxel, start_x: i32, start_y: i32, col: u8) {
    let s = 4i32;  // scale
    let gap = 2i32; // gap between letters

    let letters: &[&[u8]] = &[
        // P: 111 101 111 100 100
        &[0b111, 0b101, 0b111, 0b100, 0b100],
        // Y: 101 101 011 001 110
        &[0b101, 0b101, 0b011, 0b001, 0b110],
        // X: 101 101 010 101 101
        &[0b101, 0b101, 0b010, 0b101, 0b101],
        // E: 111 100 110 100 111
        &[0b111, 0b100, 0b110, 0b100, 0b111],
        // L: 100 100 100 100 111
        &[0b100, 0b100, 0b100, 0b100, 0b111],
    ];

    let mut lx = start_x;
    for rows in letters {
        for (ry, &row) in rows.iter().enumerate() {
            for cx in 0..3i32 {
                let bit = (row >> (2 - cx)) & 1;
                if bit == 1 {
                    let rx = lx + cx * s;
                    let ry2 = start_y + ry as i32 * s;
                    px.draw_rect(rx as f32, ry2 as f32, (s-1) as f32, (s-1) as f32, col);
                }
            }
        }
        lx += 3 * s + gap;
    }
}
