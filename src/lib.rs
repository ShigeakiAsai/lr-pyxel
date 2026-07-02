// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};

use pyo3::prelude::*;
use pyo3::append_to_inittab;
use pyo3::types::PyModule;

#[allow(unused_imports)]
use pyxel_core::{
    colors, height, init as pyxel_init, screen, width,
    KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9,
    KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J,
    KEY_K, KEY_L, KEY_M, KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T,
    KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z,
    KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
    KEY_RETURN, KEY_ESCAPE, KEY_SPACE, KEY_BACKSPACE, KEY_TAB,
    KEY_LSHIFT, KEY_RSHIFT, KEY_LCTRL, KEY_RCTRL, KEY_LALT, KEY_RALT,
    KEY_F1, KEY_F2, KEY_F3, KEY_F4, KEY_F5, KEY_F6,
    KEY_F7, KEY_F8, KEY_F9, KEY_F10, KEY_F11, KEY_F12,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X, GAMEPAD1_BUTTON_Y,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
};

static mut VIDEO_CB:    Option<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)>   = None;
static mut INPUT_POLL:  Option<unsafe extern "C" fn()>                                        = None;
static mut INPUT_STATE: Option<unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16>  = None;
static mut ENVIRON_CB:  Option<unsafe extern "C" fn(c_uint, *mut c_void) -> bool>             = None;

// Screen dimensions
const SCREEN_W: u32 = 128;
const SCREEN_H: u32 = 128;
const FPS: u32      = 60;

// Game-requested FPS (set by pyxel.init(), default 30)
// Used to skip frames: a 30fps game runs update/draw every 2nd retro_run() call
static mut GAME_FPS: u32 = 30;

// Pre-built RGB565 palette LUT (256 entries)
static mut PALETTE_RGB565: [u16; 256] = [0u16; 256];

// True once pyxel_init() has succeeded
static mut PYXEL_READY: bool = false;

// Cached Python game callbacks
static mut PY_UPDATE: Option<Py<PyAny>> = None;
static mut PY_DRAW:   Option<Py<PyAny>> = None;

// Audio batch callback (libretro stereo PCM output)
static mut AUDIO_BATCH_CB: Option<unsafe extern "C" fn(*const i16, usize) -> usize> = None;

// BlipBuf for Pyxel audio rendering (22050 Hz, NTSC clock)
static mut BLIP_BUF: Option<blip_buf::BlipBuf> = None;

// Samples per frame at 22050 Hz / 60 fps (ceil)
const AUDIO_SAMPLES_PER_FRAME: usize = 368;

// ---------------------------------------------------------------------------
// Pyxel Python module — v0.4.0 minimal set
// ---------------------------------------------------------------------------

// -- drawing -----------------------------------------------------------------

#[pyfunction]
fn cls(color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().clear(color);
        }
    }
}

#[pyfunction]
fn rect(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_rect(x, y, w, h, color);
        }
    }
}

#[pyfunction]
fn text(x: f32, y: f32, s: &str, color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_text(x, y, s, color, None);
        }
    }
}

#[pyfunction]
fn pset(x: f32, y: f32, color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().set_pixel(x, y, color);
        }
    }
}

#[pyfunction]
fn pget(x: f32, y: f32) -> u8 {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().pixel(x, y)
        } else {
            0
        }
    }
}

// blt(x, y, img, u, v, w, h, colkey=None)
// Draws a width x height region starting at (u, v) of image bank `img`
// onto the screen at (x, y). `colkey` marks a transparent color index.
#[pyfunction]
#[pyo3(signature = (x, y, img, u, v, w, h, colkey=None, rotate=None, scale=None))]
#[allow(clippy::too_many_arguments)]
fn blt(x: f32, y: f32, img: u32, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_image(x, y, img, u, v, w, h, colkey, rotate, scale);
        }
    }
}

// bltm(x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None)
// Draws a region of tilemap bank `tm` onto the screen at (x, y).
#[pyfunction]
#[pyo3(signature = (x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None))]
#[allow(clippy::too_many_arguments)]
fn bltm(x: f32, y: f32, tm: u32, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_tilemap(x, y, tm, u, v, w, h, colkey, rotate, scale);
        }
    }
}

// image_load(bank, path, x=0, y=0, include_colors=False)
// Loads a PNG file into image bank `bank` at offset (x, y).
// Mirrors pyxel_core::Image::load(); the bank index must already exist
// (Pyxel pre-allocates NUM_IMAGES banks at init time).
#[pyfunction]
#[pyo3(signature = (bank, path, x=0, y=0, include_colors=false))]
fn image_load(bank: usize, path: &str, x: i32, y: i32, include_colors: bool) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let imgs = pyxel_core::images();
        let Some(rc_image) = imgs.get(bank) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "image bank {bank} does not exist"
            )));
        };
        // RcImage = Rc<UnsafeCell<Image>>; get a mutable reference via the cell
        let image: &mut pyxel_core::Image = &mut *rc_image.get();
        image
            .load(x, y, path, Some(include_colors))
            .map_err(pyo3::exceptions::PyOSError::new_err)
    }
}

// image_pset(bank, x, y, color)
// Sets a single pixel directly inside image bank `bank`, without going
// through the screen. Useful for hand-drawing a tiny sprite at runtime
// (e.g. for the blt() smoke test) without needing an external PNG.
#[pyfunction]
fn image_pset(bank: usize, x: f32, y: f32, color: u8) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let imgs = pyxel_core::images();
        let Some(rc_image) = imgs.get(bank) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "image bank {bank} does not exist"
            )));
        };
        let image: &mut pyxel_core::Image = &mut *rc_image.get();
        image.set_pixel(x, y, color);
        Ok(())
    }
}

// load(filename, excl_images=False, excl_tilemaps=False, excl_sounds=False, excl_musics=False)
// Loads a .pyxres resource file into the current Pyxel session.
#[pyfunction]
#[pyo3(signature = (filename, excl_images=false, excl_tilemaps=false, excl_sounds=false, excl_musics=false))]
fn load(
    filename: &str,
    excl_images: bool,
    excl_tilemaps: bool,
    excl_sounds: bool,
    excl_musics: bool,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel()
            .load_resource(
                filename,
                Some(excl_images),
                Some(excl_tilemaps),
                Some(excl_sounds),
                Some(excl_musics),
            )
            .map_err(pyo3::exceptions::PyOSError::new_err)
    }
}

// -- sound -------------------------------------------------------------------

// sound_set(no, notes, tones, volumes, effects, speed)
// Writes MML-style note/tone/volume/effect strings into sound bank `no`,
// mirroring pyxel_core::Sound::set(). Must be called once (e.g. at module
// load time) before play()/play_sound() can use that bank.
#[pyfunction]
fn sound_set(
    no: usize,
    notes: &str,
    tones: &str,
    volumes: &str,
    effects: &str,
    speed: u16,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let snds = pyxel_core::sounds();
        let Some(rc_sound) = snds.get(no) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "sound bank {no} does not exist"
            )));
        };
        let sound: &mut pyxel_core::Sound = &mut *rc_sound.get();
        sound
            .set(notes, tones, volumes, effects, speed)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }
}

// play(ch, snd, loop=False)
// Plays sound bank `snd` once (or looped) on channel `ch`.
#[pyfunction]
#[pyo3(signature = (ch, snd, r#loop=false))]
fn play(ch: u32, snd: u32, r#loop: bool) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().play_sound(ch, snd, Some(0.0), r#loop, false);
        }
    }
}

// playm(msc, loop=False)
// Plays music bank `msc`.
#[pyfunction]
#[pyo3(signature = (msc, r#loop=false))]
fn playm(msc: u32, r#loop: bool) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().play_music(msc, Some(0.0), r#loop);
        }
    }
}

// stop(ch=None)
// Stops playback on a single channel, or all channels if ch is omitted.
#[pyfunction]
#[pyo3(signature = (ch=None))]
fn stop(ch: Option<u32>) {
    unsafe {
        if !PYXEL_READY {
            return;
        }
        match ch {
            Some(c) => pyxel_core::pyxel().stop_channel(c),
            None => pyxel_core::pyxel().stop_all_channels(),
        }
    }
}

// -- input -------------------------------------------------------------------

#[pyfunction]
fn btn(key: u32) -> bool {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().is_button_down(key)
        } else {
            false
        }
    }
}

#[pyfunction]
fn btnp(key: u32, hold: Option<u32>, repeat: Option<u32>) -> bool {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().is_button_pressed(key, hold, repeat)
        } else {
            false
        }
    }
}

// -- system ------------------------------------------------------------------

#[pyfunction]
fn frame_count() -> u32 {
    *pyxel_core::frame_count()
}

// init() is a no-op: Pyxel is already initialized by retro_init()
#[pyfunction]
#[pyo3(signature = (w, h, title=None, fps=None, quit_key=None,
                    display_scale=None, capture_scale=None,
                    capture_sec=None))]
fn init(
    w: u32, h: u32,
    title: Option<&str>, fps: Option<u32>, quit_key: Option<u32>,
    display_scale: Option<u32>, capture_scale: Option<u32>, capture_sec: Option<u32>,
) {
    // Save game-requested FPS (default 30 if not specified)
    unsafe {
        GAME_FPS = fps.unwrap_or(30).clamp(1, 60);
    }
}

// run(update, draw) — caches the callbacks for the libretro frame loop.
// In normal Pyxel this starts the event loop; here it is the hook that
// lets class-based games (e.g. Game() → pyxel.run(self.update, self.draw))
// register their callbacks with the core.
#[pyfunction]
fn run(update: PyObject, draw: PyObject) {
    unsafe {
        PY_UPDATE = Some(update);
        PY_DRAW   = Some(draw);
    }
}

// -- key constants -----------------------------------------------------------

fn add_key_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("KEY_UP",        pyxel_core::KEY_UP)?;
    m.add("KEY_DOWN",      pyxel_core::KEY_DOWN)?;
    m.add("KEY_LEFT",      pyxel_core::KEY_LEFT)?;
    m.add("KEY_RIGHT",     pyxel_core::KEY_RIGHT)?;
    m.add("KEY_Z",         pyxel_core::KEY_Z)?;
    m.add("KEY_X",         pyxel_core::KEY_X)?;
    m.add("KEY_A",         pyxel_core::KEY_A)?;
    m.add("KEY_B",         pyxel_core::KEY_B)?;
    m.add("KEY_C",         pyxel_core::KEY_C)?;
    m.add("KEY_D",         pyxel_core::KEY_D)?;
    m.add("KEY_E",         pyxel_core::KEY_E)?;
    m.add("KEY_F",         pyxel_core::KEY_F)?;
    m.add("KEY_G",         pyxel_core::KEY_G)?;
    m.add("KEY_H",         pyxel_core::KEY_H)?;
    m.add("KEY_I",         pyxel_core::KEY_I)?;
    m.add("KEY_J",         pyxel_core::KEY_J)?;
    m.add("KEY_K",         pyxel_core::KEY_K)?;
    m.add("KEY_L",         pyxel_core::KEY_L)?;
    m.add("KEY_M",         pyxel_core::KEY_M)?;
    m.add("KEY_N",         pyxel_core::KEY_N)?;
    m.add("KEY_O",         pyxel_core::KEY_O)?;
    m.add("KEY_P",         pyxel_core::KEY_P)?;
    m.add("KEY_Q",         pyxel_core::KEY_Q)?;
    m.add("KEY_R",         pyxel_core::KEY_R)?;
    m.add("KEY_S",         pyxel_core::KEY_S)?;
    m.add("KEY_T",         pyxel_core::KEY_T)?;
    m.add("KEY_U",         pyxel_core::KEY_U)?;
    m.add("KEY_V",         pyxel_core::KEY_V)?;
    m.add("KEY_W",         pyxel_core::KEY_W)?;
    m.add("KEY_Y",         pyxel_core::KEY_Y)?;
    m.add("KEY_RETURN",    pyxel_core::KEY_RETURN)?;
    m.add("KEY_ESCAPE",    pyxel_core::KEY_ESCAPE)?;
    m.add("KEY_SPACE",     pyxel_core::KEY_SPACE)?;
    m.add("KEY_BACKSPACE", pyxel_core::KEY_BACKSPACE)?;
    m.add("KEY_TAB",       pyxel_core::KEY_TAB)?;
    m.add("KEY_LSHIFT",    pyxel_core::KEY_LSHIFT)?;
    m.add("KEY_RSHIFT",    pyxel_core::KEY_RSHIFT)?;
    m.add("KEY_LCTRL",     pyxel_core::KEY_LCTRL)?;
    m.add("KEY_RCTRL",     pyxel_core::KEY_RCTRL)?;
    m.add("KEY_LALT",      pyxel_core::KEY_LALT)?;
    m.add("KEY_RALT",      pyxel_core::KEY_RALT)?;
    m.add("KEY_0",         pyxel_core::KEY_0)?;
    m.add("KEY_1",         pyxel_core::KEY_1)?;
    m.add("KEY_2",         pyxel_core::KEY_2)?;
    m.add("KEY_3",         pyxel_core::KEY_3)?;
    m.add("KEY_4",         pyxel_core::KEY_4)?;
    m.add("KEY_5",         pyxel_core::KEY_5)?;
    m.add("KEY_6",         pyxel_core::KEY_6)?;
    m.add("KEY_7",         pyxel_core::KEY_7)?;
    m.add("KEY_8",         pyxel_core::KEY_8)?;
    m.add("KEY_9",         pyxel_core::KEY_9)?;
    m.add("KEY_F1",        pyxel_core::KEY_F1)?;
    m.add("KEY_F2",        pyxel_core::KEY_F2)?;
    m.add("KEY_F3",        pyxel_core::KEY_F3)?;
    m.add("KEY_F4",        pyxel_core::KEY_F4)?;
    m.add("KEY_F5",        pyxel_core::KEY_F5)?;
    m.add("KEY_F6",        pyxel_core::KEY_F6)?;
    m.add("KEY_F7",        pyxel_core::KEY_F7)?;
    m.add("KEY_F8",        pyxel_core::KEY_F8)?;
    m.add("KEY_F9",        pyxel_core::KEY_F9)?;
    m.add("KEY_F10",       pyxel_core::KEY_F10)?;
    m.add("KEY_F11",       pyxel_core::KEY_F11)?;
    m.add("KEY_F12",       pyxel_core::KEY_F12)?;
    Ok(())
}

fn add_color_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("COLOR_BLACK",      0u8)?;
    m.add("COLOR_NAVY",       1u8)?;
    m.add("COLOR_PURPLE",     2u8)?;
    m.add("COLOR_GREEN",      3u8)?;
    m.add("COLOR_BROWN",      4u8)?;
    m.add("COLOR_DARK_BLUE",  5u8)?;
    m.add("COLOR_LIGHT_BLUE", 6u8)?;
    m.add("COLOR_WHITE",      7u8)?;
    m.add("COLOR_RED",        8u8)?;
    m.add("COLOR_ORANGE",     9u8)?;
    m.add("COLOR_YELLOW",    10u8)?;
    m.add("COLOR_LIME",      11u8)?;
    m.add("COLOR_CYAN",      12u8)?;
    m.add("COLOR_GRAY",      13u8)?;
    m.add("COLOR_PINK",      14u8)?;
    m.add("COLOR_PEACH",     15u8)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Math functions
// ---------------------------------------------------------------------------

#[pyfunction] fn ceil(x: f32) -> i32 { pyxel_core::Pyxel::ceil(x) }
#[pyfunction] fn floor(x: f32) -> i32 { pyxel_core::Pyxel::floor(x) }
#[pyfunction] fn sqrt(x: f32) -> f32 { pyxel_core::Pyxel::sqrt(x) }
#[pyfunction] fn sin(deg: f32) -> f32 { pyxel_core::Pyxel::sin(deg) }
#[pyfunction] fn cos(deg: f32) -> f32 { pyxel_core::Pyxel::cos(deg) }
#[pyfunction] fn atan2(y: f32, x: f32) -> f32 { pyxel_core::Pyxel::atan2(y, x) }
#[pyfunction] fn sgn(x: f32) -> f32 { if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 } }
#[pyfunction] fn clamp(x: f32, lower: f32, upper: f32) -> f32 { x.clamp(lower, upper) }
#[pyfunction] fn rseed(seed: u32) { pyxel_core::Pyxel::random_seed(seed); }
#[pyfunction] fn rndi(a: i32, b: i32) -> i32 { pyxel_core::Pyxel::random_int(a, b) }
#[pyfunction] fn rndf(a: f32, b: f32) -> f32 { pyxel_core::Pyxel::random_float(a, b) }
#[pyfunction] fn nseed(seed: u32) { pyxel_core::Pyxel::noise_seed(seed); }

#[pyfunction]
#[pyo3(signature = (x, y=0.0, z=0.0))]
fn noise(x: f32, y: f32, z: f32) -> f32 { pyxel_core::Pyxel::noise(x, y, z) }

// ---------------------------------------------------------------------------
// Drawing functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
fn line(x1: f32, y1: f32, x2: f32, y2: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_line(x1, y1, x2, y2, color); } }
}
#[pyfunction]
fn rectb(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_rect_border(x, y, w, h, color); } }
}
#[pyfunction]
fn circ(x: f32, y: f32, r: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle(x, y, r, color); } }
}
#[pyfunction]
fn circb(x: f32, y: f32, r: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle_border(x, y, r, color); } }
}
#[pyfunction]
fn elli(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse(x, y, w, h, color); } }
}
#[pyfunction]
fn ellib(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse_border(x, y, w, h, color); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn tri(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle(x1, y1, x2, y2, x3, y3, color); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn trib(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle_border(x1, y1, x2, y2, x3, y3, color); } }
}
#[pyfunction]
fn fill(x: f32, y: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().flood_fill(x, y, color); } }
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None, w=None, h=None))]
fn clip(x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (x, y, w, h) {
            (Some(x), Some(y), Some(w), Some(h)) => pyxel_core::pyxel().set_clip_rect(x, y, w, h),
            _ => pyxel_core::pyxel().reset_clip_rect(),
        }
    }
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None))]
fn camera(x: Option<f32>, y: Option<f32>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (x, y) {
            (Some(x), Some(y)) => pyxel_core::pyxel().set_camera(x, y),
            _ => pyxel_core::pyxel().reset_camera(),
        }
    }
}
#[pyfunction]
#[pyo3(signature = (col1=None, col2=None))]
fn pal(col1: Option<u8>, col2: Option<u8>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (col1, col2) {
            (Some(c1), Some(c2)) => pyxel_core::pyxel().map_color(c1, c2),
            _ => pyxel_core::pyxel().reset_color_map(),
        }
    }
}
#[pyfunction]
fn dither(alpha: f32) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_dithering(alpha); } }
}

// ---------------------------------------------------------------------------
// Input functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
fn btnr(key: u32) -> bool {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().is_button_released(key) } else { false } }
}
#[pyfunction]
fn btnv(key: u32) -> i32 {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().button_value(key) } else { 0 } }
}
#[pyfunction]
fn mouse(visible: bool) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_mouse_visible(visible); } }
}

// ---------------------------------------------------------------------------
// System functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
fn quit() {
    unsafe {
        if let Some(env) = ENVIRON_CB {
            env(rust_libretro_sys::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut());
        }
    }
}

// show() — renders one frame and waits (used in scripts without a run loop).
// We cache a no-op update/draw so retro_run() keeps displaying the
// already-rendered frame instead of falling back to the placeholder.
#[pyfunction]
#[allow(static_mut_refs)]
fn show() {
    unsafe {
        if !PYXEL_READY { return; }
        Python::with_gil(|py| {
            // Create no-op lambda and cache as update/draw
            let noop = py.eval_bound("lambda: None", None, None).unwrap();
            if PY_UPDATE.is_none() {
                PY_UPDATE = Some(noop.clone().into());
            }
            if PY_DRAW.is_none() {
                PY_DRAW = Some(noop.into());
            }
        });
    }
}

// flip() — advances one frame manually (used instead of pyxel.run()).
// In libretro context this is a no-op: framing is driven by retro_run().
#[pyfunction]
fn flip() {}
#[pyfunction]
fn width_fn() -> u32 { *pyxel_core::width() }
#[pyfunction]
fn height_fn() -> u32 { *pyxel_core::height() }

// -- module registration -----------------------------------------------------

#[pymodule]
fn pyxel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Drawing
    m.add_function(wrap_pyfunction!(cls,         m)?)?;
    m.add_function(wrap_pyfunction!(rect,        m)?)?;
    m.add_function(wrap_pyfunction!(rectb,       m)?)?;
    m.add_function(wrap_pyfunction!(text,        m)?)?;
    m.add_function(wrap_pyfunction!(pset,        m)?)?;
    m.add_function(wrap_pyfunction!(pget,        m)?)?;
    m.add_function(wrap_pyfunction!(line,        m)?)?;
    m.add_function(wrap_pyfunction!(circ,        m)?)?;
    m.add_function(wrap_pyfunction!(circb,       m)?)?;
    m.add_function(wrap_pyfunction!(elli,        m)?)?;
    m.add_function(wrap_pyfunction!(ellib,       m)?)?;
    m.add_function(wrap_pyfunction!(tri,         m)?)?;
    m.add_function(wrap_pyfunction!(trib,        m)?)?;
    m.add_function(wrap_pyfunction!(fill,        m)?)?;
    m.add_function(wrap_pyfunction!(clip,        m)?)?;
    m.add_function(wrap_pyfunction!(camera,      m)?)?;
    m.add_function(wrap_pyfunction!(pal,         m)?)?;
    m.add_function(wrap_pyfunction!(dither,      m)?)?;
    m.add_function(wrap_pyfunction!(blt,         m)?)?;
    m.add_function(wrap_pyfunction!(bltm,        m)?)?;
    m.add_function(wrap_pyfunction!(image_load,  m)?)?;
    m.add_function(wrap_pyfunction!(image_pset,  m)?)?;
    m.add_function(wrap_pyfunction!(load,        m)?)?;
    // Input
    m.add_function(wrap_pyfunction!(btn,         m)?)?;
    m.add_function(wrap_pyfunction!(btnp,        m)?)?;
    m.add_function(wrap_pyfunction!(btnr,        m)?)?;
    m.add_function(wrap_pyfunction!(btnv,        m)?)?;
    m.add_function(wrap_pyfunction!(mouse,       m)?)?;
    // Audio
    m.add_function(wrap_pyfunction!(sound_set,   m)?)?;
    m.add_function(wrap_pyfunction!(play,        m)?)?;
    m.add_function(wrap_pyfunction!(playm,       m)?)?;
    m.add_function(wrap_pyfunction!(stop,        m)?)?;
    // Math
    m.add_function(wrap_pyfunction!(ceil,        m)?)?;
    m.add_function(wrap_pyfunction!(floor,       m)?)?;
    m.add_function(wrap_pyfunction!(clamp,       m)?)?;
    m.add_function(wrap_pyfunction!(sgn,         m)?)?;
    m.add_function(wrap_pyfunction!(sqrt,        m)?)?;
    m.add_function(wrap_pyfunction!(sin,         m)?)?;
    m.add_function(wrap_pyfunction!(cos,         m)?)?;
    m.add_function(wrap_pyfunction!(atan2,       m)?)?;
    m.add_function(wrap_pyfunction!(rseed,       m)?)?;
    m.add_function(wrap_pyfunction!(rndi,        m)?)?;
    m.add_function(wrap_pyfunction!(rndf,        m)?)?;
    m.add_function(wrap_pyfunction!(nseed,       m)?)?;
    m.add_function(wrap_pyfunction!(noise,       m)?)?;
    // System
    m.add_function(wrap_pyfunction!(frame_count, m)?)?;
    m.add_function(wrap_pyfunction!(quit,        m)?)?;
    m.add_function(wrap_pyfunction!(show,        m)?)?;
    m.add_function(wrap_pyfunction!(flip,        m)?)?;
    m.add_function(wrap_pyfunction!(width_fn,    m)?)?;
    m.add_function(wrap_pyfunction!(height_fn,   m)?)?;
    m.add_function(wrap_pyfunction!(init,        m)?)?;
    m.add_function(wrap_pyfunction!(run,         m)?)?;
    // width/height as module attributes
    m.add("width",  *pyxel_core::width())?;
    m.add("height", *pyxel_core::height())?;
    add_key_constants(m)?;
    add_color_constants(m)?;

    // Expose colors as a Python list (16 RGB24 values)
    // pyxel.colors[n] returns the RGB value of palette entry n
    {
        let pal = pyxel_core::colors();
        let color_list = pyo3::types::PyList::new_bound(_py, pal.iter().copied());
        m.add("colors", color_list)?;
    }

    // GAMEPAD constants
    m.add("GAMEPAD1_BUTTON_A",             pyxel_core::GAMEPAD1_BUTTON_A)?;
    m.add("GAMEPAD1_BUTTON_B",             pyxel_core::GAMEPAD1_BUTTON_B)?;
    m.add("GAMEPAD1_BUTTON_X",             pyxel_core::GAMEPAD1_BUTTON_X)?;
    m.add("GAMEPAD1_BUTTON_Y",             pyxel_core::GAMEPAD1_BUTTON_Y)?;
    m.add("GAMEPAD1_BUTTON_BACK",          pyxel_core::GAMEPAD1_BUTTON_BACK)?;
    m.add("GAMEPAD1_BUTTON_START",         pyxel_core::GAMEPAD1_BUTTON_START)?;
    m.add("GAMEPAD1_BUTTON_LEFTSHOULDER",  pyxel_core::GAMEPAD1_BUTTON_LEFTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_RIGHTSHOULDER", pyxel_core::GAMEPAD1_BUTTON_RIGHTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_DPAD_UP",       pyxel_core::GAMEPAD1_BUTTON_DPAD_UP)?;
    m.add("GAMEPAD1_BUTTON_DPAD_DOWN",     pyxel_core::GAMEPAD1_BUTTON_DPAD_DOWN)?;
    m.add("GAMEPAD1_BUTTON_DPAD_LEFT",     pyxel_core::GAMEPAD1_BUTTON_DPAD_LEFT)?;
    m.add("GAMEPAD1_BUTTON_DPAD_RIGHT",    pyxel_core::GAMEPAD1_BUTTON_DPAD_RIGHT)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Environment / pixel format
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(
    cb: unsafe extern "C" fn(c_uint, *mut c_void) -> bool,
) {
    ENVIRON_CB = Some(cb);

    let mut supported: u8 = 1;
    cb(
        rust_libretro_sys::RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME,
        &mut supported as *mut u8 as *mut c_void,
    );

    let format = rust_libretro_sys::retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565;
    cb(
        rust_libretro_sys::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
        &format as *const _ as *mut c_void,
    );
}

// ---------------------------------------------------------------------------
// Callback registration
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_set_video_refresh(
    cb: unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize),
) {
    VIDEO_CB = Some(cb);
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample(_cb: unsafe extern "C" fn(i16, i16)) {}

#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample_batch(
    cb: unsafe extern "C" fn(*const i16, usize) -> usize,
) -> usize {
    AUDIO_BATCH_CB = Some(cb);
    0
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_poll(cb: unsafe extern "C" fn()) {
    INPUT_POLL = Some(cb);
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_state(
    cb: unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16,
) {
    INPUT_STATE = Some(cb);
}

// ---------------------------------------------------------------------------
// System info
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut c_void) {
    let info = info as *mut rust_libretro_sys::retro_system_info;
    (*info).library_name     = b"Pyxel\0".as_ptr() as *const c_char;
    (*info).library_version  = b"0.5.0\0".as_ptr() as *const c_char;
    (*info).valid_extensions = b"py|pyxapp\0".as_ptr() as *const c_char;
    (*info).need_fullpath    = true;
    (*info).block_extract    = false;
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut c_void) {
    let info = info as *mut rust_libretro_sys::retro_system_av_info;
    (*info).geometry.base_width   = SCREEN_W;
    (*info).geometry.base_height  = SCREEN_H;
    (*info).geometry.max_width    = SCREEN_W;
    (*info).geometry.max_height   = SCREEN_H;
    (*info).geometry.aspect_ratio = 1.0;
    (*info).timing.fps            = f64::from(FPS);
    // NOTE: this declares the rate libretro expects from audio_batch_cb.
    // We are not yet feeding that callback (Pyxel/SDL2 currently renders
    // audio directly to ALSA in headless mode, bypassing libretro's audio
    // pipeline) — see CHANGELOG notes for v0.4.1 sound support.
    // Pyxel's internal mixer runs at AUDIO_SAMPLE_RATE = 22050 Hz.
    (*info).timing.sample_rate    = 22050.0;
}

// ---------------------------------------------------------------------------
// Init / deinit
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_init() {
    // Guard: only initialize once. RetroArch may call retro_init() again
    // when switching content without fully unloading the core.
    if PYXEL_READY {
        return;
    }

    // Register "pyxel" built-in module BEFORE Py_Initialize
    append_to_inittab!(pyxel);

    // Prevent SDL2 from grabbing the ALSA device directly.
    // Audio is routed through libretro's audio_batch_cb instead.
    std::env::set_var("SDL_AUDIODRIVER", "dummy");

    // Initialize Pyxel engine in headless mode
    pyxel_init(
        SCREEN_W, SCREEN_H,
        Some("lr-pyxel"),
        Some(FPS),
        None, None, None, None,
        Some(true),        // headless = true
    );

    // Initialize BlipBuf for audio rendering
    let mut blip = blip_buf::BlipBuf::new(AUDIO_SAMPLES_PER_FRAME as u32 * 2);
    blip.set_rates(
        pyxel_core::AUDIO_CLOCK_RATE as f64,
        pyxel_core::AUDIO_SAMPLE_RATE as f64,
    );
    BLIP_BUF = Some(blip);

    build_palette_lut();
    PYXEL_READY = true;

    // Start Python interpreter (after append_to_inittab)
    pyo3::prepare_freethreaded_python();
}

#[no_mangle]
pub unsafe extern "C" fn retro_deinit() {
    // Drop Py<PyAny> inside GIL to avoid double-free
    Python::with_gil(|_py| {
        PY_UPDATE = None;
        PY_DRAW   = None;
    });
    // NOTE: do NOT reset PYXEL_READY or BLIP_BUF here.
    // RetroArch may call retro_init() again after retro_deinit() when
    // switching content, and we guard retro_init() with PYXEL_READY.
}

// ---------------------------------------------------------------------------
// .pyxapp extraction
// ---------------------------------------------------------------------------

// Extract a .pyxapp (ZIP) file to a temporary directory and return the path
// to the startup script (.pyxapp_startup_script contains its relative path).
fn extract_pyxapp(pyxapp_path: &str) -> Option<String> {

    let file = std::fs::File::open(pyxapp_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // Extract to /tmp/lr-pyxel/<stem>/
    let stem = std::path::Path::new(pyxapp_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let extract_dir = std::path::PathBuf::from(format!("/tmp/lr-pyxel/{}", stem));
    std::fs::create_dir_all(&extract_dir).ok()?;

    // Security check: ensure no path traversal
    let extract_dir_abs = extract_dir.canonicalize().ok()?;
    for i in 0..archive.len() {
        let file = archive.by_index(i).ok()?;
        let target = extract_dir.join(file.name());
        let target_abs = if target.exists() {
            target.canonicalize().ok()?
        } else {
            // For files that don't exist yet, check parent
            let parent = target.parent()?;
            std::fs::create_dir_all(parent).ok()?;
            parent.canonicalize().ok()?.join(file.name().split('/').last()?)
        };
        if !target_abs.starts_with(&extract_dir_abs) {
            eprintln!("[lr-pyxel] Unsafe path in .pyxapp: {}", file.name());
            return None;
        }
    }

    // Extract all files
    archive.extract(&extract_dir).ok()?;

    // Find .pyxapp_startup_script in any subdirectory
    for entry in std::fs::read_dir(&extract_dir).ok()? {
        let entry = entry.ok()?;
        let subdir = entry.path();
        if !subdir.is_dir() { continue; }
        let startup_script_marker = subdir.join(".pyxapp_startup_script");
        if startup_script_marker.exists() {
            let script_rel = std::fs::read_to_string(&startup_script_marker).ok()?;
            let script_path = subdir.join(script_rel.trim());
            if script_path.exists() {
                return Some(script_path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Game load / unload
// ---------------------------------------------------------------------------

#[repr(C)]
struct RetroGameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn retro_load_game(game: *const c_void) -> bool {
    if game.is_null() {
        return true; // content-less boot
    }
    let info = &*(game as *const RetroGameInfo);
    if info.path.is_null() {
        return true;
    }

    let path = CStr::from_ptr(info.path).to_string_lossy().into_owned();

    // Resolve the actual .py script path.
    // For .pyxapp files: extract the ZIP and find the startup script.
    // For .py files: use the path directly.
    let script_path = if path.ends_with(".pyxapp") {
        match extract_pyxapp(&path) {
            Some(p) => p,
            None => {
                eprintln!("[lr-pyxel] Failed to extract .pyxapp: {}", path);
                return true;
            }
        }
    } else {
        path.clone()
    };

    Python::with_gil(|py| {
        // Drop previous game callbacks inside GIL to avoid double-free
        PY_UPDATE = None;
        PY_DRAW   = None;

        // Add game directory to sys.path and set as working directory
        let sys     = py.import_bound("sys").expect("failed to import sys");
        let syspath = sys.getattr("path").unwrap();
        let syspath = syspath.downcast_into::<pyo3::types::PyList>().unwrap();
        let game_dir = std::path::Path::new(&script_path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_string_lossy()
            .into_owned();
        syspath.insert(0, game_dir.clone()).unwrap();

        // Change working directory to the game directory so that relative
        // paths in the script (e.g. pyxel.load("assets/foo.pyxres")) resolve
        // correctly.
        let _ = std::env::set_current_dir(&game_dir);

        // Execute the game script
        let code    = std::fs::read_to_string(&script_path).unwrap_or_default();
        let globals = pyo3::types::PyDict::new_bound(py);

        match py.run_bound(&code, Some(&globals), None) {
            Ok(_) => {
                // If pyxel.run(update, draw) was called during script execution
                // (class-based games), PY_UPDATE/PY_DRAW are already set.
                // Only fall back to module-level update()/draw() if not set yet.
                if PY_UPDATE.is_none() {
                    PY_UPDATE = globals.get_item("update").ok()
                        .flatten()
                        .map(|f| f.into_py(py));
                }
                if PY_DRAW.is_none() {
                    PY_DRAW = globals.get_item("draw").ok()
                        .flatten()
                        .map(|f| f.into_py(py));
                }
            }
            Err(e) => {
                e.print(py);
            }
        }
    });

    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_unload_game() {
    Python::with_gil(|_py| {
        PY_UPDATE = None;
        PY_DRAW   = None;
    });
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn retro_run() {
    // 1. Poll input
    if let Some(poll) = INPUT_POLL {
        poll();
    }

    // 2. Collect joypad bitmask
    let mut buttons: u32 = 0;
    if let Some(state) = INPUT_STATE {
        for bit in 0u32..16 {
            if state(0, rust_libretro_sys::RETRO_DEVICE_JOYPAD, 0, bit) != 0 {
                buttons |= 1 << bit;
            }
        }
    }

    // 3. SELECT (bit 2) → shutdown
    if buttons & (1 << 2) != 0 {
        if let Some(env) = ENVIRON_CB {
            env(rust_libretro_sys::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut());
        }
        return;
    }

    if !PYXEL_READY {
        submit_fallback_frame();
        return;
    }

    // 4. Call Python game callbacks if loaded, otherwise show placeholder.
    //    For games running at less than 60fps, skip update()/draw() on
    //    intermediate frames so the game runs at its intended speed.
    //    e.g. GAME_FPS=30 → call update/draw every 2nd retro_run() call.
    let run_this_frame = unsafe {
        let fc = *pyxel_core::frame_count();
        let step = (FPS / GAME_FPS).max(1);
        fc % step == 0
    };

    if unsafe { PY_UPDATE.is_some() || PY_DRAW.is_some() } {
        if run_this_frame {
            Python::with_gil(|py| {
                if let Some(ref update) = PY_UPDATE {
                    if let Err(e) = update.call0(py) { e.print(py); }
                }
                if let Some(ref draw) = PY_DRAW {
                    if let Err(e) = draw.call0(py) { e.print(py); }
                }
            });
        }
    } else {
        // No game loaded — light blue placeholder
        pyxel_core::pyxel().clear(11);
    }

    // 5. Advance one Pyxel frame.
    //    flip_screen() calls start_input_frame() internally, resetting all key
    //    states. inject_input() must come AFTER this so the fresh input is
    //    registered in the new frame — preventing btnp() from firing every frame.
    pyxel_core::pyxel().flip_screen();

    // 6. Inject input AFTER flip_screen() so btnp() sees a single press
    inject_input(buttons);

    // Update pyxel.frame_count module attribute so scripts can use it
    // as either pyxel.frame_count (attribute) or pyxel.frame_count() (function)
    Python::with_gil(|py| {
        if let Ok(m) = py.import_bound("pyxel") {
            let fc = *pyxel_core::frame_count();
            let _ = m.setattr("frame_count", fc);
        }
    });

    // 7. Submit framebuffer to RetroArch
    submit_pyxel_frame();

    // 8. Render and submit audio samples to RetroArch
    submit_audio_frame();
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

unsafe fn build_palette_lut() {
    let pal = colors();
    for (i, &rgb24) in pal.iter().enumerate().take(256) {
        let r = ((rgb24 >> 16) & 0xFF) as u16;
        let g = ((rgb24 >>  8) & 0xFF) as u16;
        let b = ( rgb24        & 0xFF) as u16;
        PALETTE_RGB565[i] = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
    }
}

// Previous frame's button bitmask — used to detect edges (press/release)
static mut PREV_BUTTONS: u32 = 0;

unsafe fn inject_input(buttons: u32) {
    const MAP: &[(u32, u32)] = &[
        // libretro bit → Pyxel key
        // Bit 0 = B(cross),  1 = Y(square), 2 = Select, 3 = Start
        // Bit 4 = Up, 5 = Down, 6 = Left, 7 = Right
        // Bit 8 = A(circle), 9 = X(triangle), 10 = L, 11 = R
        (0,  KEY_Z),
        (1,  KEY_X),
        (2,  KEY_S),         // Select → KEY_S
        (3,  KEY_RETURN),
        (4,  KEY_UP),
        (5,  KEY_DOWN),
        (6,  KEY_LEFT),
        (7,  KEY_RIGHT),
        (8,  KEY_A),
        (9,  KEY_S),
        // GAMEPAD1_BUTTON_* mapping
        (0,  GAMEPAD1_BUTTON_B),
        (1,  GAMEPAD1_BUTTON_A),
        (2,  GAMEPAD1_BUTTON_BACK),
        (3,  GAMEPAD1_BUTTON_START),
        (4,  GAMEPAD1_BUTTON_DPAD_UP),
        (5,  GAMEPAD1_BUTTON_DPAD_DOWN),
        (6,  GAMEPAD1_BUTTON_DPAD_LEFT),
        (7,  GAMEPAD1_BUTTON_DPAD_RIGHT),
        (8,  GAMEPAD1_BUTTON_A),
        (9,  GAMEPAD1_BUTTON_X),
        (10, GAMEPAD1_BUTTON_LEFTSHOULDER),
        (11, GAMEPAD1_BUTTON_RIGHTSHOULDER),
    ];
    let px = pyxel_core::pyxel();
    let changed = buttons ^ PREV_BUTTONS;
    for &(bit, key) in MAP {
        let mask = 1u32 << bit;
        if changed & mask != 0 {
            px.set_button_state(key, buttons & mask != 0);
        }
    }
    PREV_BUTTONS = buttons;
}

unsafe fn submit_pyxel_frame() {
    let w = *width()  as usize;
    let h = *height() as usize;
    let pixels = w * h;

    let screen_rc = screen();
    let src: *const u8 = (*screen_rc.get()).data_ptr() as *const u8;

    let mut fb = vec![0u16; pixels];
    for i in 0..pixels {
        fb[i] = PALETTE_RGB565[*src.add(i) as usize];
    }

    if let Some(video) = VIDEO_CB {
        video(fb.as_ptr() as *const c_void, w as c_uint, h as c_uint, w * 2);
    }
}

unsafe fn submit_fallback_frame() {
    const GREEN: u16 = 0x07E0;
    let fb = vec![GREEN; (SCREEN_W * SCREEN_H) as usize];
    if let Some(video) = VIDEO_CB {
        video(fb.as_ptr() as *const c_void, SCREEN_W, SCREEN_H, (SCREEN_W * 2) as usize);
    }
}

unsafe fn submit_audio_frame() {
    let Some(ref mut blip) = BLIP_BUF else { return; };
    let Some(audio_cb)     = AUDIO_BATCH_CB else { return; };

    // Calculate samples needed per retro_run() call based on game FPS.
    // At 30fps we need 2x samples per call vs 60fps.
    let samples = (pyxel_core::AUDIO_SAMPLE_RATE / GAME_FPS) as usize;
    let samples = samples.min(AUDIO_SAMPLES_PER_FRAME * 2); // cap at 2x buffer

    // Render mono PCM from Pyxel's internal mixer
    let mut mono = vec![0i16; samples];
    pyxel_core::Audio::render_samples(pyxel_core::channels(), blip, &mut mono);

    // Convert mono → stereo interleaved (L/R identical) as libretro expects
    let mut stereo = vec![0i16; samples * 2];
    for (i, &s) in mono.iter().enumerate() {
        stereo[i * 2]     = s; // L
        stereo[i * 2 + 1] = s; // R
    }

    audio_cb(stereo.as_ptr(), samples);
}

// ---------------------------------------------------------------------------
// Required stubs
// ---------------------------------------------------------------------------

#[no_mangle] pub unsafe extern "C" fn retro_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_set_controller_port_device(_p: c_uint, _d: c_uint) {}
#[no_mangle] pub unsafe extern "C" fn retro_api_version() -> c_uint { rust_libretro_sys::RETRO_API_VERSION as c_uint }
#[no_mangle] pub unsafe extern "C" fn retro_serialize_size() -> usize { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_serialize(_d: *mut c_void, _s: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_unserialize(_d: *const c_void, _s: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_cheat_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_cheat_set(_i: c_uint, _e: bool, _c: *const c_char) {}
#[no_mangle] pub unsafe extern "C" fn retro_load_game_special(_t: c_uint, _i: *const c_void, _n: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_get_region() -> c_uint { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void { std::ptr::null_mut() }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> usize { 0 }
