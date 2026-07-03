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
    // Colors
    COLOR_BLACK, COLOR_NAVY, COLOR_PURPLE, COLOR_GREEN, COLOR_BROWN,
    COLOR_DARK_BLUE, COLOR_LIGHT_BLUE, COLOR_WHITE, COLOR_RED, COLOR_ORANGE,
    COLOR_YELLOW, COLOR_LIME, COLOR_CYAN, COLOR_GRAY, COLOR_PINK, COLOR_PEACH,
    // Keys
    KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9,
    KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J,
    KEY_K, KEY_L, KEY_M, KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T,
    KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z,
    KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
    KEY_RETURN, KEY_ESCAPE, KEY_SPACE, KEY_BACKSPACE, KEY_TAB,
    KEY_LSHIFT, KEY_RSHIFT, KEY_LCTRL, KEY_RCTRL, KEY_LALT, KEY_RALT,
    KEY_F1, KEY_F2, KEY_F3, KEY_F4, KEY_F5, KEY_F6,
    KEY_F7, KEY_F8, KEY_F9, KEY_F10, KEY_F11, KEY_F12,
    KEY_DELETE, KEY_CAPSLOCK, KEY_INSERT, KEY_HOME, KEY_PAGEUP, KEY_END, KEY_PAGEDOWN,
    // Mouse
    MOUSE_POS_X, MOUSE_POS_Y, MOUSE_WHEEL_X, MOUSE_WHEEL_Y,
    MOUSE_BUTTON_LEFT, MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT,
    MOUSE_BUTTON_X1, MOUSE_BUTTON_X2,
    // Gamepad 1
    GAMEPAD1_AXIS_LEFTX, GAMEPAD1_AXIS_LEFTY, GAMEPAD1_AXIS_RIGHTX, GAMEPAD1_AXIS_RIGHTY,
    GAMEPAD1_AXIS_TRIGGERLEFT, GAMEPAD1_AXIS_TRIGGERRIGHT,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X, GAMEPAD1_BUTTON_Y,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_GUIDE, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_LEFTSTICK, GAMEPAD1_BUTTON_RIGHTSTICK,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
    // Gamepad 2
    GAMEPAD2_BUTTON_A, GAMEPAD2_BUTTON_B, GAMEPAD2_BUTTON_X, GAMEPAD2_BUTTON_Y,
    GAMEPAD2_BUTTON_BACK, GAMEPAD2_BUTTON_GUIDE, GAMEPAD2_BUTTON_START,
    GAMEPAD2_BUTTON_LEFTSHOULDER, GAMEPAD2_BUTTON_RIGHTSHOULDER,
    GAMEPAD2_BUTTON_DPAD_UP, GAMEPAD2_BUTTON_DPAD_DOWN,
    GAMEPAD2_BUTTON_DPAD_LEFT, GAMEPAD2_BUTTON_DPAD_RIGHT,
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

// Game-requested screen size (set by pyxel.init(), default 128x128)
static mut GAME_W: u32 = 128;
static mut GAME_H: u32 = 128;

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
#[pyo3(signature = (ch, snd, r#loop=false, resume=false))]
fn play(ch: u32, snd: u32, r#loop: bool, resume: bool) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().play_sound(ch, snd, Some(0.0), r#loop, resume);
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
    let _ = (title, quit_key, display_scale, capture_scale, capture_sec);
    unsafe {
        // Save game-requested size and FPS
        GAME_W = w.max(1);
        GAME_H = h.max(1);
        GAME_FPS = fps.unwrap_or(30).clamp(1, 60);

        // Update pyxel.width/height module attributes to reflect game size
        Python::with_gil(|py| {
            if let Ok(m) = py.import_bound("pyxel") {
                let _ = m.setattr("width",  GAME_W);
                let _ = m.setattr("height", GAME_H);
            }
        });

        // Notify RetroArch of the game's actual screen geometry.
        // RETRO_ENVIRONMENT_SET_GEOMETRY (37) lets us change width/height
        // after init without restarting the core.
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   w,
                base_height:  h,
                max_width:    256,
                max_height:   256,
                aspect_ratio: w as f32 / h as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
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

fn add_module_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Graphics
    m.add("NUM_COLORS",      pyxel_core::NUM_COLORS)?;
    m.add("NUM_IMAGES",      pyxel_core::NUM_IMAGES)?;
    m.add("IMAGE_SIZE",      pyxel_core::IMAGE_SIZE)?;
    m.add("NUM_TILEMAPS",    pyxel_core::NUM_TILEMAPS)?;
    m.add("TILEMAP_SIZE",    pyxel_core::TILEMAP_SIZE)?;
    m.add("TILE_SIZE",       pyxel_core::TILE_SIZE)?;
    m.add("COLOR_BLACK",     pyxel_core::COLOR_BLACK)?;
    m.add("COLOR_NAVY",      pyxel_core::COLOR_NAVY)?;
    m.add("COLOR_PURPLE",    pyxel_core::COLOR_PURPLE)?;
    m.add("COLOR_GREEN",     pyxel_core::COLOR_GREEN)?;
    m.add("COLOR_BROWN",     pyxel_core::COLOR_BROWN)?;
    m.add("COLOR_DARK_BLUE", pyxel_core::COLOR_DARK_BLUE)?;
    m.add("COLOR_LIGHT_BLUE",pyxel_core::COLOR_LIGHT_BLUE)?;
    m.add("COLOR_WHITE",     pyxel_core::COLOR_WHITE)?;
    m.add("COLOR_RED",       pyxel_core::COLOR_RED)?;
    m.add("COLOR_ORANGE",    pyxel_core::COLOR_ORANGE)?;
    m.add("COLOR_YELLOW",    pyxel_core::COLOR_YELLOW)?;
    m.add("COLOR_LIME",      pyxel_core::COLOR_LIME)?;
    m.add("COLOR_CYAN",      pyxel_core::COLOR_CYAN)?;
    m.add("COLOR_GRAY",      pyxel_core::COLOR_GRAY)?;
    m.add("COLOR_PINK",      pyxel_core::COLOR_PINK)?;
    m.add("COLOR_PEACH",     pyxel_core::COLOR_PEACH)?;
    m.add("FONT_WIDTH",      pyxel_core::FONT_WIDTH)?;
    m.add("FONT_HEIGHT",     pyxel_core::FONT_HEIGHT)?;
    // Audio
    m.add("NUM_CHANNELS",          pyxel_core::NUM_CHANNELS)?;
    m.add("NUM_TONES",             pyxel_core::NUM_TONES)?;
    m.add("NUM_SOUNDS",            pyxel_core::NUM_SOUNDS)?;
    m.add("NUM_MUSICS",            pyxel_core::NUM_MUSICS)?;
    m.add("TONE_TRIANGLE",         pyxel_core::TONE_TRIANGLE)?;
    m.add("TONE_SQUARE",           pyxel_core::TONE_SQUARE)?;
    m.add("TONE_PULSE",            pyxel_core::TONE_PULSE)?;
    m.add("TONE_NOISE",            pyxel_core::TONE_NOISE)?;
    m.add("EFFECT_NONE",           pyxel_core::EFFECT_NONE)?;
    m.add("EFFECT_SLIDE",          pyxel_core::EFFECT_SLIDE)?;
    m.add("EFFECT_VIBRATO",        pyxel_core::EFFECT_VIBRATO)?;
    m.add("EFFECT_FADEOUT",        pyxel_core::EFFECT_FADEOUT)?;
    m.add("EFFECT_HALF_FADEOUT",   pyxel_core::EFFECT_HALF_FADEOUT)?;
    m.add("EFFECT_QUARTER_FADEOUT",pyxel_core::EFFECT_QUARTER_FADEOUT)?;
    // Keyboard
    m.add("KEY_BACKSPACE",   pyxel_core::KEY_BACKSPACE)?;
    m.add("KEY_TAB",         pyxel_core::KEY_TAB)?;
    m.add("KEY_RETURN",      pyxel_core::KEY_RETURN)?;
    m.add("KEY_ESCAPE",      pyxel_core::KEY_ESCAPE)?;
    m.add("KEY_SPACE",       pyxel_core::KEY_SPACE)?;
    m.add("KEY_0",           pyxel_core::KEY_0)?;
    m.add("KEY_1",           pyxel_core::KEY_1)?;
    m.add("KEY_2",           pyxel_core::KEY_2)?;
    m.add("KEY_3",           pyxel_core::KEY_3)?;
    m.add("KEY_4",           pyxel_core::KEY_4)?;
    m.add("KEY_5",           pyxel_core::KEY_5)?;
    m.add("KEY_6",           pyxel_core::KEY_6)?;
    m.add("KEY_7",           pyxel_core::KEY_7)?;
    m.add("KEY_8",           pyxel_core::KEY_8)?;
    m.add("KEY_9",           pyxel_core::KEY_9)?;
    m.add("KEY_A",           pyxel_core::KEY_A)?;
    m.add("KEY_B",           pyxel_core::KEY_B)?;
    m.add("KEY_C",           pyxel_core::KEY_C)?;
    m.add("KEY_D",           pyxel_core::KEY_D)?;
    m.add("KEY_E",           pyxel_core::KEY_E)?;
    m.add("KEY_F",           pyxel_core::KEY_F)?;
    m.add("KEY_G",           pyxel_core::KEY_G)?;
    m.add("KEY_H",           pyxel_core::KEY_H)?;
    m.add("KEY_I",           pyxel_core::KEY_I)?;
    m.add("KEY_J",           pyxel_core::KEY_J)?;
    m.add("KEY_K",           pyxel_core::KEY_K)?;
    m.add("KEY_L",           pyxel_core::KEY_L)?;
    m.add("KEY_M",           pyxel_core::KEY_M)?;
    m.add("KEY_N",           pyxel_core::KEY_N)?;
    m.add("KEY_O",           pyxel_core::KEY_O)?;
    m.add("KEY_P",           pyxel_core::KEY_P)?;
    m.add("KEY_Q",           pyxel_core::KEY_Q)?;
    m.add("KEY_R",           pyxel_core::KEY_R)?;
    m.add("KEY_S",           pyxel_core::KEY_S)?;
    m.add("KEY_T",           pyxel_core::KEY_T)?;
    m.add("KEY_U",           pyxel_core::KEY_U)?;
    m.add("KEY_V",           pyxel_core::KEY_V)?;
    m.add("KEY_W",           pyxel_core::KEY_W)?;
    m.add("KEY_X",           pyxel_core::KEY_X)?;
    m.add("KEY_Y",           pyxel_core::KEY_Y)?;
    m.add("KEY_Z",           pyxel_core::KEY_Z)?;
    m.add("KEY_DELETE",      pyxel_core::KEY_DELETE)?;
    m.add("KEY_CAPSLOCK",    pyxel_core::KEY_CAPSLOCK)?;
    m.add("KEY_F1",          pyxel_core::KEY_F1)?;
    m.add("KEY_F2",          pyxel_core::KEY_F2)?;
    m.add("KEY_F3",          pyxel_core::KEY_F3)?;
    m.add("KEY_F4",          pyxel_core::KEY_F4)?;
    m.add("KEY_F5",          pyxel_core::KEY_F5)?;
    m.add("KEY_F6",          pyxel_core::KEY_F6)?;
    m.add("KEY_F7",          pyxel_core::KEY_F7)?;
    m.add("KEY_F8",          pyxel_core::KEY_F8)?;
    m.add("KEY_F9",          pyxel_core::KEY_F9)?;
    m.add("KEY_F10",         pyxel_core::KEY_F10)?;
    m.add("KEY_F11",         pyxel_core::KEY_F11)?;
    m.add("KEY_F12",         pyxel_core::KEY_F12)?;
    m.add("KEY_INSERT",      pyxel_core::KEY_INSERT)?;
    m.add("KEY_HOME",        pyxel_core::KEY_HOME)?;
    m.add("KEY_PAGEUP",      pyxel_core::KEY_PAGEUP)?;
    m.add("KEY_END",         pyxel_core::KEY_END)?;
    m.add("KEY_PAGEDOWN",    pyxel_core::KEY_PAGEDOWN)?;
    m.add("KEY_RIGHT",       pyxel_core::KEY_RIGHT)?;
    m.add("KEY_LEFT",        pyxel_core::KEY_LEFT)?;
    m.add("KEY_DOWN",        pyxel_core::KEY_DOWN)?;
    m.add("KEY_UP",          pyxel_core::KEY_UP)?;
    m.add("KEY_LCTRL",       pyxel_core::KEY_LCTRL)?;
    m.add("KEY_LSHIFT",      pyxel_core::KEY_LSHIFT)?;
    m.add("KEY_LALT",        pyxel_core::KEY_LALT)?;
    m.add("KEY_RCTRL",       pyxel_core::KEY_RCTRL)?;
    m.add("KEY_RSHIFT",      pyxel_core::KEY_RSHIFT)?;
    m.add("KEY_RALT",        pyxel_core::KEY_RALT)?;
    // Mouse
    m.add("MOUSE_POS_X",          pyxel_core::MOUSE_POS_X)?;
    m.add("MOUSE_POS_Y",          pyxel_core::MOUSE_POS_Y)?;
    m.add("MOUSE_WHEEL_X",        pyxel_core::MOUSE_WHEEL_X)?;
    m.add("MOUSE_WHEEL_Y",        pyxel_core::MOUSE_WHEEL_Y)?;
    m.add("MOUSE_BUTTON_LEFT",    pyxel_core::MOUSE_BUTTON_LEFT)?;
    m.add("MOUSE_BUTTON_MIDDLE",  pyxel_core::MOUSE_BUTTON_MIDDLE)?;
    m.add("MOUSE_BUTTON_RIGHT",   pyxel_core::MOUSE_BUTTON_RIGHT)?;
    m.add("MOUSE_BUTTON_X1",      pyxel_core::MOUSE_BUTTON_X1)?;
    m.add("MOUSE_BUTTON_X2",      pyxel_core::MOUSE_BUTTON_X2)?;
    // Gamepad 1
    m.add("GAMEPAD1_AXIS_LEFTX",        pyxel_core::GAMEPAD1_AXIS_LEFTX)?;
    m.add("GAMEPAD1_AXIS_LEFTY",        pyxel_core::GAMEPAD1_AXIS_LEFTY)?;
    m.add("GAMEPAD1_AXIS_RIGHTX",       pyxel_core::GAMEPAD1_AXIS_RIGHTX)?;
    m.add("GAMEPAD1_AXIS_RIGHTY",       pyxel_core::GAMEPAD1_AXIS_RIGHTY)?;
    m.add("GAMEPAD1_AXIS_TRIGGERLEFT",  pyxel_core::GAMEPAD1_AXIS_TRIGGERLEFT)?;
    m.add("GAMEPAD1_AXIS_TRIGGERRIGHT", pyxel_core::GAMEPAD1_AXIS_TRIGGERRIGHT)?;
    m.add("GAMEPAD1_BUTTON_A",             pyxel_core::GAMEPAD1_BUTTON_A)?;
    m.add("GAMEPAD1_BUTTON_B",             pyxel_core::GAMEPAD1_BUTTON_B)?;
    m.add("GAMEPAD1_BUTTON_X",             pyxel_core::GAMEPAD1_BUTTON_X)?;
    m.add("GAMEPAD1_BUTTON_Y",             pyxel_core::GAMEPAD1_BUTTON_Y)?;
    m.add("GAMEPAD1_BUTTON_BACK",          pyxel_core::GAMEPAD1_BUTTON_BACK)?;
    m.add("GAMEPAD1_BUTTON_GUIDE",         pyxel_core::GAMEPAD1_BUTTON_GUIDE)?;
    m.add("GAMEPAD1_BUTTON_START",         pyxel_core::GAMEPAD1_BUTTON_START)?;
    m.add("GAMEPAD1_BUTTON_LEFTSTICK",     pyxel_core::GAMEPAD1_BUTTON_LEFTSTICK)?;
    m.add("GAMEPAD1_BUTTON_RIGHTSTICK",    pyxel_core::GAMEPAD1_BUTTON_RIGHTSTICK)?;
    m.add("GAMEPAD1_BUTTON_LEFTSHOULDER",  pyxel_core::GAMEPAD1_BUTTON_LEFTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_RIGHTSHOULDER", pyxel_core::GAMEPAD1_BUTTON_RIGHTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_DPAD_UP",       pyxel_core::GAMEPAD1_BUTTON_DPAD_UP)?;
    m.add("GAMEPAD1_BUTTON_DPAD_DOWN",     pyxel_core::GAMEPAD1_BUTTON_DPAD_DOWN)?;
    m.add("GAMEPAD1_BUTTON_DPAD_LEFT",     pyxel_core::GAMEPAD1_BUTTON_DPAD_LEFT)?;
    m.add("GAMEPAD1_BUTTON_DPAD_RIGHT",    pyxel_core::GAMEPAD1_BUTTON_DPAD_RIGHT)?;
    // Gamepad 2
    m.add("GAMEPAD2_BUTTON_A",             pyxel_core::GAMEPAD2_BUTTON_A)?;
    m.add("GAMEPAD2_BUTTON_B",             pyxel_core::GAMEPAD2_BUTTON_B)?;
    m.add("GAMEPAD2_BUTTON_X",             pyxel_core::GAMEPAD2_BUTTON_X)?;
    m.add("GAMEPAD2_BUTTON_Y",             pyxel_core::GAMEPAD2_BUTTON_Y)?;
    m.add("GAMEPAD2_BUTTON_BACK",          pyxel_core::GAMEPAD2_BUTTON_BACK)?;
    m.add("GAMEPAD2_BUTTON_GUIDE",         pyxel_core::GAMEPAD2_BUTTON_GUIDE)?;
    m.add("GAMEPAD2_BUTTON_START",         pyxel_core::GAMEPAD2_BUTTON_START)?;
    m.add("GAMEPAD2_BUTTON_LEFTSHOULDER",  pyxel_core::GAMEPAD2_BUTTON_LEFTSHOULDER)?;
    m.add("GAMEPAD2_BUTTON_RIGHTSHOULDER", pyxel_core::GAMEPAD2_BUTTON_RIGHTSHOULDER)?;
    m.add("GAMEPAD2_BUTTON_DPAD_UP",       pyxel_core::GAMEPAD2_BUTTON_DPAD_UP)?;
    m.add("GAMEPAD2_BUTTON_DPAD_DOWN",     pyxel_core::GAMEPAD2_BUTTON_DPAD_DOWN)?;
    m.add("GAMEPAD2_BUTTON_DPAD_LEFT",     pyxel_core::GAMEPAD2_BUTTON_DPAD_LEFT)?;
    m.add("GAMEPAD2_BUTTON_DPAD_RIGHT",    pyxel_core::GAMEPAD2_BUTTON_DPAD_RIGHT)?;
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

// system_wrapper.rs additions
// Window/display settings are no-ops in headless libretro mode

#[pyfunction]
fn reset() {
    // In libretro, reset = reload current content
    // For now this is a no-op; future: trigger RETRO_ENVIRONMENT_RESET
}

#[pyfunction]
fn title(_title: &str) {
    // no-op in headless mode
}

#[pyfunction]
#[pyo3(signature = (data, scale, colkey=None))]
fn icon(data: Vec<String>, scale: u32, colkey: Option<u8>) {
    let _ = (data, scale, colkey);
    // no-op in headless mode
}

#[pyfunction]
fn perf_monitor(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
fn integer_scale(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
fn screen_mode(_scr: u32) {
    // no-op in headless mode
}

#[pyfunction]
fn fullscreen(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
fn resize(width: u32, height: u32) -> PyResult<()> {
    if width == 0 || height == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "width and height must be greater than 0",
        ));
    }
    unsafe {
        GAME_W = width;
        GAME_H = height;
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   width,
                base_height:  height,
                max_width:    256,
                max_height:   256,
                aspect_ratio: width as f32 / height as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
    }
    Ok(())
}
#[pyfunction]
fn width_fn() -> u32 { *pyxel_core::width() }
#[pyfunction]
fn height_fn() -> u32 { *pyxel_core::height() }

// -- module registration -----------------------------------------------------

// ---------------------------------------------------------------------------
// Image bank wrapper (pyxel.images[n])
// ---------------------------------------------------------------------------

#[pyclass(name = "Image")]
struct PyImage {
    bank: usize,
}

#[pymethods]
impl PyImage {
    fn load(&self, x: i32, y: i32, filename: &str) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &mut *rc.get();
            img.load(x, y, filename, Some(false))
                .map_err(pyo3::exceptions::PyOSError::new_err)
        }
    }

    fn set(&self, x: i32, y: i32, data: Vec<String>) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &mut *rc.get();
            let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
            img.set(x, y, &data_refs);
            Ok(())
        }
    }

    fn pget(&self, x: f32, y: f32) -> u8 {
        unsafe {
            if !PYXEL_READY { return 0; }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &*rc.get();
            img.pixel(x, y)
        }
    }

    fn pset(&self, x: f32, y: f32, color: u8) {
        unsafe {
            if !PYXEL_READY { return; }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &mut *rc.get();
            img.set_pixel(x, y, color);
        }
    }

    #[getter]
    fn width(&self) -> u32 {
        unsafe {
            if !PYXEL_READY { return 0; }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &*rc.get();
            img.width()
        }
    }

    #[getter]
    fn height(&self) -> u32 {
        unsafe {
            if !PYXEL_READY { return 0; }
            let imgs = pyxel_core::images();
            let rc = &imgs[self.bank];
            let img = &*rc.get();
            img.height()
        }
    }
}

#[pyclass(name = "ImageList")]
struct PyImageList;

#[pymethods]
impl PyImageList {
    fn __getitem__(&self, idx: usize) -> PyResult<PyImage> {
        if idx >= pyxel_core::NUM_IMAGES as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("image bank index {idx} out of range")
            ));
        }
        Ok(PyImage { bank: idx })
    }
}

// ---------------------------------------------------------------------------
// Sound bank wrapper (pyxel.sounds[n])
// ---------------------------------------------------------------------------

#[pyclass(name = "Sound")]
struct PySound {
    bank: usize,
}

#[pymethods]
impl PySound {
    fn set(&self, notes: &str, tones: &str, volumes: &str, effects: &str, speed: u16) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let snds = pyxel_core::sounds();
            let rc = &snds[self.bank];
            let snd = &mut *rc.get();
            snd.set(notes, tones, volumes, effects, speed)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        }
    }

    fn mml(&self, code: &str) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let snds = pyxel_core::sounds();
            let rc = &snds[self.bank];
            let snd = &mut *rc.get();
            snd.set_mml(code)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        }
    }
}

#[pyclass(name = "SoundList")]
struct PySoundList;

#[pymethods]
impl PySoundList {
    fn __getitem__(&self, idx: usize) -> PyResult<PySound> {
        if idx >= pyxel_core::NUM_SOUNDS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("sound bank index {idx} out of range")
            ));
        }
        Ok(PySound { bank: idx })
    }
}

// ---------------------------------------------------------------------------
// Tilemap bank wrapper (pyxel.tilemaps[n])
// ---------------------------------------------------------------------------

#[pyclass(name = "Tilemap")]
struct PyTilemap {
    bank: usize,
}

#[pymethods]
impl PyTilemap {
    fn set(&self, x: i32, y: i32, data: Vec<String>) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let tms = pyxel_core::tilemaps();
            let rc = &tms[self.bank];
            let tm = &mut *rc.get();
            let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
            tm.set(x, y, &data_refs);
            Ok(())
        }
    }

    #[getter]
    fn imgsrc(&self) -> u32 {
        unsafe {
            if !PYXEL_READY { return 0; }
            let tms = pyxel_core::tilemaps();
            let rc = &tms[self.bank];
            let tm = &*rc.get();
            match &tm.imgsrc {
                pyxel_core::ImageSource::Index(i) => *i,
                _ => 0,
            }
        }
    }

    #[setter]
    fn set_imgsrc(&self, idx: u32) {
        unsafe {
            if !PYXEL_READY { return; }
            let tms = pyxel_core::tilemaps();
            let rc = &tms[self.bank];
            let tm = &mut *rc.get();
            tm.imgsrc = pyxel_core::ImageSource::Index(idx);
        }
    }

    fn pget(&self, x: f32, y: f32) -> (u16, u16) {
        unsafe {
            if !PYXEL_READY { return (0, 0); }
            let tms = pyxel_core::tilemaps();
            let rc = &tms[self.bank];
            let tm = &*rc.get();
            tm.tile(x, y)
        }
    }

    fn pset(&self, x: f32, y: f32, tile: (u16, u16)) {
        unsafe {
            if !PYXEL_READY { return; }
            let tms = pyxel_core::tilemaps();
            let rc = &tms[self.bank];
            let tm = &mut *rc.get();
            tm.set_tile(x, y, tile);
        }
    }

    #[pyo3(signature = (x, y, tm, u, v, w, h))]
    fn blt(&self, x: f32, y: f32, tm: usize, u: f32, v: f32, w: f32, h: f32) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let tms = pyxel_core::tilemaps();
            let src_rc = &tms[tm];
            let dst_rc = &tms[self.bank];
            let dst = &mut *dst_rc.get();
            dst.draw_tilemap(x, y, src_rc, u, v, w, h, None, None, None);
            Ok(())
        }
    }
}

#[pyclass(name = "TilemapList")]
struct PyTilemapList;

#[pymethods]
impl PyTilemapList {
    fn __getitem__(&self, idx: usize) -> PyResult<PyTilemap> {
        if idx >= pyxel_core::NUM_TILEMAPS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tilemap bank index {idx} out of range")
            ));
        }
        Ok(PyTilemap { bank: idx })
    }
}

// ---------------------------------------------------------------------------
// Music bank wrapper (pyxel.musics[n])
// ---------------------------------------------------------------------------

#[pyclass(name = "Music")]
struct PyMusic {
    bank: usize,
}

#[pymethods]
impl PyMusic {
    // set(ch0, ch1, ch2, ch3) — each arg is a list of sound indices for that channel
    #[pyo3(signature = (ch0=vec![], ch1=vec![], ch2=vec![], ch3=vec![]))]
    fn set(&self, ch0: Vec<u32>, ch1: Vec<u32>, ch2: Vec<u32>, ch3: Vec<u32>) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let mscs = pyxel_core::musics();
            let rc = &mscs[self.bank];
            let msc = &mut *rc.get();
            msc.set(&[ch0, ch1, ch2, ch3]);
            Ok(())
        }
    }
}

#[pyclass(name = "MusicList")]
struct PyMusicList;

#[pymethods]
impl PyMusicList {
    fn __getitem__(&self, idx: usize) -> PyResult<PyMusic> {
        if idx >= pyxel_core::NUM_MUSICS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("music bank index {idx} out of range")
            ));
        }
        Ok(PyMusic { bank: idx })
    }
}

// ---------------------------------------------------------------------------
// Module-level __getattr__ for dynamic variables (variable_wrapper.rs)
// ---------------------------------------------------------------------------
// This mirrors pyxel-binding's variable_wrapper.rs __getattr__ approach:
// variables that change every frame (frame_count, mouse_x/y, etc.) are
// returned dynamically instead of being set once at module init time.

#[pyfunction]
fn __getattr__(py: Python, name: &str) -> PyResult<Py<PyAny>> {
    let value: Py<PyAny> = match name {
        // System
        "width"       => (*pyxel_core::width()).into_py(py),
        "height"      => (*pyxel_core::height()).into_py(py),
        "frame_count" => (*pyxel_core::frame_count()).into_py(py),
        // Input
        "mouse_x"     => (*pyxel_core::mouse_x()).into_py(py),
        "mouse_y"     => (*pyxel_core::mouse_y()).into_py(py),
        "mouse_wheel" => (*pyxel_core::mouse_wheel()).into_py(py),
        // Graphics
        "colors"   => {
            let pal = pyxel_core::colors();
            pyo3::types::PyList::new_bound(py, pal.iter().copied()).into()
        },
        "images"   => PyImageList.into_py(py),
        "tilemaps" => PyTilemapList.into_py(py),
        // Audio
        "sounds"   => PySoundList.into_py(py),
        "musics"   => PyMusicList.into_py(py),
        _ => return Err(pyo3::exceptions::PyAttributeError::new_err(
            format!("module 'pyxel' has no attribute '{name}'")
        )),
    };
    Ok(value)
}

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
    // System (system_wrapper.rs)
    m.add_function(wrap_pyfunction!(quit,         m)?)?;
    m.add_function(wrap_pyfunction!(reset,        m)?)?;
    m.add_function(wrap_pyfunction!(show,         m)?)?;
    m.add_function(wrap_pyfunction!(flip,         m)?)?;
    m.add_function(wrap_pyfunction!(title,        m)?)?;
    m.add_function(wrap_pyfunction!(icon,         m)?)?;
    m.add_function(wrap_pyfunction!(perf_monitor, m)?)?;
    m.add_function(wrap_pyfunction!(integer_scale,m)?)?;
    m.add_function(wrap_pyfunction!(screen_mode,  m)?)?;
    m.add_function(wrap_pyfunction!(fullscreen,   m)?)?;
    m.add_function(wrap_pyfunction!(resize,       m)?)?;
    m.add_function(wrap_pyfunction!(init,         m)?)?;
    m.add_function(wrap_pyfunction!(run,          m)?)?;
    // width/height as module attributes
    // Dynamic variables via __getattr__ (variable_wrapper.rs approach)
    // width, height, frame_count, mouse_x/y, colors, images, tilemaps,
    // sounds, musics are all returned dynamically by __getattr__
    m.add_function(wrap_pyfunction!(__getattr__, m)?)?;

    // Constants (constant_wrapper.rs)
    add_module_constants(m)?;

    // Register pyclass types
    m.add_class::<PyImage>()?;
    m.add_class::<PyImageList>()?;
    m.add_class::<PySound>()?;
    m.add_class::<PySoundList>()?;
    m.add_class::<PyMusic>()?;
    m.add_class::<PyMusicList>()?;
    m.add_class::<PyTilemap>()?;
    m.add_class::<PyTilemapList>()?;

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
    let w = if GAME_W > 0 { GAME_W } else { SCREEN_W };
    let h = if GAME_H > 0 { GAME_H } else { SCREEN_H };
    (*info).geometry.base_width   = w;
    (*info).geometry.base_height  = h;
    (*info).geometry.max_width    = 256;
    (*info).geometry.max_height   = 256;
    (*info).geometry.aspect_ratio = w as f32 / h as f32;
    (*info).timing.fps            = f64::from(FPS);
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
    // Use 1024 to accommodate both 30fps (735 samples) and 60fps (368 samples)
    let mut blip = blip_buf::BlipBuf::new(1024);
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
    // Pyxel internal buffer is SCREEN_W x SCREEN_H (512x512).
    // We submit only the GAME_W x GAME_H portion that the game requested
    // via pyxel.init(), cropping the bottom-right if needed.
    let src_w = *width()  as usize;  // Pyxel internal width (128)
    let dst_w = (GAME_W as usize).min(src_w);
    let dst_h = (GAME_H as usize).min(*height() as usize);

    let screen_rc = screen();
    let src: *const u8 = (*screen_rc.get()).data_ptr() as *const u8;

    // Build output framebuffer row by row
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
