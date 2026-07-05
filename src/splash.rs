//! splash.rs — lr-pyxel splash screen

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Draw the splash screen onto Pyxel's internal buffer.
pub fn draw() {
    let px = pyxel_core::pyxel();
    px.clear(0);

    let col: u8 = 5; // dark blue-gray

    // --- Isometric cube outline (thick lines = draw twice offset by 1) ---
    // 128x128 screen, cube centered at (64, 64)
    let cx = 64.0f32;
    let ty = 2.0f32;   // top point
    let my = 26.0f32;  // middle (left/right peak of top face)
    let by = 50.0f32;  // bottom of top face (center vertical start)
    let lx = 12.0f32;  // left x
    let rx = 116.0f32; // right x
    let bly = 98.0f32;  // bottom left y
    let bry = 98.0f32;  // bottom right y
    let bcy = 122.0f32; // bottom center y

    // Top face (thick = 3 overlapping lines)
    for d in 0..2u32 {
        let d = d as f32;
        px.draw_line(cx+d, ty+d, rx+d, my+d, col);
        px.draw_line(rx+d, my+d, cx+d, by+d, col);
        px.draw_line(cx+d, by+d, lx+d, my+d, col);
        px.draw_line(lx+d, my+d, cx+d, ty+d, col);
        // Left face
        px.draw_line(lx+d, my+d, lx+d, bly+d, col);
        px.draw_line(lx+d, bly+d, cx+d, bcy+d, col);
        px.draw_line(cx+d, by+d, cx+d, bcy+d, col);
        // Right face
        px.draw_line(rx+d, my+d, rx+d, bry+d, col);
        px.draw_line(rx+d, bry+d, cx+d, bcy+d, col);
    }

    // --- "libretro" text with black outline ---
    // Position: left-aligned to pyxel, above it
    let lx2 = 14.0f32;
    let ly2 = 52.0f32;
    draw_text_outlined(px, lx2, ly2, "libretro", 13);

    // --- "pyxel" in 4x pixel-art letters, centered in cube ---
    // Cube center x = 64, pyxel width = 5*12 + 4*2 = 68px → start x = 64-34 = 30
    // Cube center y ≈ 58+34 = 92? Let's place at y=52 (between top and bottom)
    let px_start_x = 30i32;
    let px_start_y = 62i32;
    draw_pyxel_text_outlined(px, px_start_x, px_start_y, 7);

    // --- Version bottom right ---
    let ver_str = format!("v{}", VERSION);
    let ver_x = 128.0 - (ver_str.len() as f32 * 4.0) - 2.0;
    draw_text_outlined(px, ver_x, 119.0, &ver_str, 7);
}

/// Draw text with 1px black outline.
fn draw_text_outlined(px: &pyxel_core::Pyxel, x: f32, y: f32, text: &str, col: u8) {
    // Draw outline in black (4 directions)
    for dx in [-1.0f32, 0.0, 1.0] {
        for dy in [-1.0f32, 0.0, 1.0] {
            if dx != 0.0 || dy != 0.0 {
                px.draw_text(x + dx, y + dy, text, 0, None);
            }
        }
    }
    // Draw text on top
    px.draw_text(x, y, text, col, None);
}

/// Draw "pyxel" in 4x pixel font with black outline.
fn draw_pyxel_text_outlined(px: &pyxel_core::Pyxel, start_x: i32, start_y: i32, col: u8) {
    // Draw black outline first (offset by 1 in all 8 directions)
    for dx in [-1i32, 0, 1] {
        for dy in [-1i32, 0, 1] {
            if dx != 0 || dy != 0 {
                draw_pyxel_text(px, start_x + dx, start_y + dy, 0);
            }
        }
    }
    // Draw colored text on top
    draw_pyxel_text(px, start_x, start_y, col);
}

/// Draw "pyxel" in 4x hand-crafted pixel font.
fn draw_pyxel_text(px: &pyxel_core::Pyxel, start_x: i32, start_y: i32, col: u8) {
    let s = 4i32;
    let gap = 2i32;

    let letters: &[&[u8]] = &[
        &[0b111, 0b101, 0b111, 0b100, 0b100], // P
        &[0b101, 0b101, 0b011, 0b001, 0b110], // Y
        &[0b101, 0b101, 0b010, 0b101, 0b101], // X
        &[0b111, 0b100, 0b110, 0b100, 0b111], // E
        &[0b100, 0b100, 0b100, 0b100, 0b111], // L
    ];

    let mut lx = start_x;
    for rows in letters {
        for (ry, &row) in rows.iter().enumerate() {
            for cx in 0..3i32 {
                if (row >> (2 - cx)) & 1 == 1 {
                    px.draw_rect(
                        (lx + cx * s) as f32,
                        (start_y + ry as i32 * s) as f32,
                        (s - 1) as f32,
                        (s - 1) as f32,
                        col,
                    );
                }
            }
        }
        lx += 3 * s + gap;
    }
}
