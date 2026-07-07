//! video.rs — libretro video output for lr-pyxel
//!
//! Converts Pyxel's internal pixel buffer (palette-indexed) to RGB565
//! and submits it to RetroArch via the video callback.

use std::os::raw::{c_uint, c_void};
use pyxel_core::{colors, screen, height};

use crate::{VIDEO_CB, PALETTE_RGB565, SCREEN_W, SCREEN_H, GAME_W, GAME_H};

/// Build a 256-entry palette lookup table (Pyxel Rgb24 → RGB565).
pub unsafe fn build_palette_lut() {
    let pal = colors();
    for (i, &rgb24) in pal.iter().enumerate().take(256) {
        let r = ((rgb24 >> 16) & 0xFF) as u16;
        let g = ((rgb24 >>  8) & 0xFF) as u16;
        let b = ( rgb24        & 0xFF) as u16;
        PALETTE_RGB565[i] = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
    }
}

/// Submit the current Pyxel screen buffer to RetroArch.
pub unsafe fn submit_pyxel_frame() {
    let screen_rc = screen();

    // src_w must be the ACTUAL physical canvas stride (how many pixels
    // one row occupies in memory), not the per-game logical width now
    // reported by pyxel_core::width(). The canvas itself is a fixed
    // SCREEN_W x SCREEN_H buffer allocated once at boot and never
    // resized — only pyxel_core::width()/height() are updated per game
    // (so Python's pyxel.width/height report the right value). Using
    // the logical width here previously desynced the row stride from
    // the real memory layout, corrupting the image into diagonal/
    // scanline garbage whenever GAME_W no longer equaled 512.
    let src_w = (*screen_rc.get()).width() as usize;
    let dst_w = (GAME_W as usize).min(src_w);
    let dst_h = (GAME_H as usize).min(*height() as usize);

    let src: *const u8 = (*screen_rc.get()).data_ptr() as *const u8;

    let mut fb = vec![0u16; dst_w * dst_h];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let idx = y * src_w + x;
            fb[y * dst_w + x] = PALETTE_RGB565[*src.add(idx) as usize];
        }
    }

    if let Some(video) = VIDEO_CB {
        video(fb.as_ptr() as *const c_void, dst_w as c_uint, dst_h as c_uint, dst_w * 2);
    }
}

/// Submit a solid-color fallback frame (shown before any content is loaded).
pub unsafe fn submit_fallback_frame() {
    const GREEN: u16 = 0x07E0;
    let fb = vec![GREEN; (SCREEN_W * SCREEN_H) as usize];
    if let Some(video) = VIDEO_CB {
        video(fb.as_ptr() as *const c_void, SCREEN_W, SCREEN_H, (SCREEN_W * 2) as usize);
    }
}
