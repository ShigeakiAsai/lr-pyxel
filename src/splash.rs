//! splash.rs — lr-pyxel splash screen

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Draw the splash screen onto Pyxel's internal buffer.
pub fn draw() {
    let px = pyxel_core::pyxel();
    px.clear(0);

    // Colors matched directly from the official Pyxel logo
    // (pyxel_logo_152x64.png), sampled pixel-by-pixel: top face =
    // PEACH, left face = PINK, right face = CYAN. Drawn in that order
    // (top, then left, then right) so the shared center vertical edge
    // — where left and right faces meet — ends up in the right
    // face's color, matching the logo's own apparent priority (right
    // face outline shows on top at that shared edge).
    let top_col: u8 = 15;   // PEACH (beige)
    let left_col: u8 = 14;  // PINK
    let right_col: u8 = 12; // CYAN (light blue)

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

    for d in 0..2u32 {
        let d = d as f32;
        // Top face's own 2 uppermost edges only — the lower-left and
        // lower-right edges of the top rhombus are reassigned to the
        // left/right face passes below, so each side face's own draw
        // pass fully owns every line touching its shared vertices,
        // instead of relying on a differently-shaped line from
        // another face happening to cover the same pixel (this is
        // what caused stray top-face pixels at the (lx,my)/(rx,my)
        // joints before).
        px.draw_line(cx+d, ty+d, rx+d, my+d, top_col);
        px.draw_line(lx+d, my+d, cx+d, ty+d, top_col);
    }
    for d in 0..2u32 {
        let d = d as f32;
        // Left face: the top rhombus's lower-left edge (reassigned
        // from top, so this pass owns the (lx,my) joint), its own
        // vertical edge, its bottom edge, and the shared center
        // vertical edge. The two truly-vertical segments use a
        // horizontal-only thickness offset (d added to x only) —
        // diagonally offsetting a vertical line's thickness (d added
        // to both x and y) makes its second copy start one row late,
        // which is what left a stray top-face pixel at (cx,by) before.
        px.draw_line(cx+d, by+d, lx+d, my+d, left_col);
        px.draw_line(lx+d, my, lx+d, bly, left_col);
        px.draw_line(lx+d, bly+d, cx+d, bcy+d, left_col);
        px.draw_line(cx+d, by, cx+d, bcy, left_col);
    }
    for d in 0..2u32 {
        let d = d as f32;
        // Right face: same idea as left, drawn last so it wins on the
        // shared center edge.
        px.draw_line(rx+d, my+d, cx+d, by+d, right_col);
        px.draw_line(rx+d, my, rx+d, bry, right_col);
        px.draw_line(rx+d, bry+d, cx+d, bcy+d, right_col);
        px.draw_line(cx+d, by, cx+d, bcy, right_col);
    }

    // Small white "shine" highlights at each vertex, matching the
    // official logo (pyxel_logo_152x64.png) — a single accent pixel
    // at the top point, both upper peaks, both lower peaks, the
    // bottom point, and where the front center edge begins.
    let hl_col: u8 = 7; // WHITE
    px.draw_rect(cx, ty, 2.0, 2.0, hl_col);
    px.draw_rect(lx, my, 2.0, 2.0, hl_col);
    px.draw_rect(rx, my, 2.0, 2.0, hl_col);
    px.draw_rect(lx, bly, 2.0, 2.0, hl_col);
    px.draw_rect(rx, bry, 2.0, 2.0, hl_col);
    px.draw_rect(cx, bcy, 2.0, 2.0, hl_col);
    px.draw_rect(cx, by, 2.0, 2.0, hl_col);

    // --- "libretro" text with black outline ---
    // Position: left-aligned to pyxel, above it
    let lx2 = 17.0f32;
    let ly2 = 55.0f32;
    draw_text_outlined(px, lx2, ly2, "libretro", 13);

    // --- "Pyxel" in 4x pixel-art letters, centered in cube ---
    // Cube center x = 64, pyxel width = 5*12 + 4*2 = 68px → start x = 64-34 = 30
    // Cube center y ≈ 58+34 = 92? Let's place at y=52 (between top and bottom)
    let px_start_x = 33i32;
    let px_start_y = 65i32;
    // Paint the text's own background black first, so the cube's
    // center vertical line (running straight through this area,
    // e.g. the front edge at cx=64) doesn't show through the "Pyxel"
    // lettering. Sized to the text's actual footprint (5 letters *
    // 3 cols * 4px + 4 gaps * 2px = 68px wide, 6 rows * 4px = 24px
    // tall — one extra row versus the original all-caps design, for
    // the "y" descender) plus 1px padding on each side for the
    // outline's own ±1px offset.
    px.draw_rect(
        (px_start_x - 1) as f32,
        (px_start_y - 1) as f32,
        70.0,
        26.0,
        0,
    );
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

/// Draw "Pyxel" in 4x pixel font with black outline.
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

/// Draw "Pyxel" in 4x hand-crafted pixel font.
///
/// Each letter is 6 rows tall: row 0 is ascender space (used only by
/// "P" and "l"), row 4 is the shared baseline, and row 5 is descender
/// space (used only by "y"). "x" and "e" sit at x-height (rows 1-4),
/// shorter than "P"/"l" — matching the official Pyxel logo's exact
/// glyph shapes (pyxel_logo_152x64.png), not the all-caps PYXEL this
/// used to draw.
fn draw_pyxel_text(px: &pyxel_core::Pyxel, start_x: i32, start_y: i32, col: u8) {
    let s = 4i32;
    let gap = 2i32;

    let letters: &[&[u8]] = &[
        &[0b110, 0b101, 0b110, 0b100, 0b100, 0b000], // P
        &[0b000, 0b101, 0b101, 0b011, 0b001, 0b010], // y
        &[0b000, 0b101, 0b010, 0b010, 0b101, 0b000], // x
        &[0b000, 0b011, 0b101, 0b110, 0b011, 0b000], // e
        &[0b110, 0b010, 0b010, 0b010, 0b111, 0b000], // l
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
