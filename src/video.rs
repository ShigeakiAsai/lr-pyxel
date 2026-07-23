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
    // one row occupies in memory). Since v0.11.x, the canvas is actually
    // resized to match each game's requested dimensions via
    // pyxel_core::pyxel().set_screen_size() (previously it stayed fixed
    // at its boot size forever, with only the separate width()/height()
    // globals updated per game — that mismatch corrupted the image into
    // diagonal/scanline garbage whenever a game's width didn't equal the
    // canvas's fixed physical size). Reading the canvas's own reported
    // width here means this code doesn't need to know or care whether
    // the canvas is fixed-size or resized per game — it's always correct
    // either way.
    let src_w = (*screen_rc.as_ptr()).width() as usize;
    let dst_w = (GAME_W as usize).min(src_w);
    let dst_h = (GAME_H as usize).min(*height() as usize);

    let src: *const u8 = (*screen_rc.as_ptr()).data_ptr() as *const u8;

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

/// Feeds the current screen buffer into lr-pyxel's own, independent
/// Screencast instance (LR_SCREENCAST — see its declaration in
/// lib.rs for why this exists separately from pyxel-core's own
/// internal one). No-op if no script has requested capture_sec (i.e.
/// LR_SCREENCAST is None).
///
/// Mirrors pyxel-core's own capture_screen() (resource.rs) as closely
/// as an external crate can: same width/height/frame_count source
/// (pyxel_core::frame_count(), not lr-pyxel's own LR_FRAME_COUNT —
/// Screencast's frame-delay math is keyed on this counter, so using a
/// different one here would desync GIF frame timing from what
/// pyxel-core's own screen_delay() expects). The one unavoidable
/// difference: pyxel-core's own version reaches Image's private
/// canvas.data field directly, which isn't accessible from outside
/// the crate — data_ptr() (already public, already used by
/// submit_pyxel_frame() above) stands in for it instead.
pub unsafe fn capture_lr_screencast_frame() {
    let Some(ref mut screencast) = crate::LR_SCREENCAST else { return; };

    let screen_rc = screen();
    let w = (*screen_rc.as_ptr()).width();
    let h = (*screen_rc.as_ptr()).height();
    let src: *const u8 = (*screen_rc.as_ptr()).data_ptr() as *const u8;
    let pixel_count = (w * h) as usize;
    let image = std::slice::from_raw_parts(src, pixel_count);

    let palette = pyxel_core::colors();

    screencast.capture(w, h, image, &palette, *pyxel_core::frame_count());
}
