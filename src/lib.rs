// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

mod video;
mod audio;
mod input;
mod retro;
mod splash;

// lr-pyxel's Python-wrapper layer, split out of what used to be one
// large wrappers.rs, grouped under src/wrapper_lr/ (see that
// directory's wrapper_lr.rs for the per-file breakdown and naming
// rationale).
mod wrapper_lr;
use wrapper_lr::{
    utils_lr, font_wrapper_lr, math_wrapper_lr,
    image_wrapper_lr, sound_wrapper_lr, tilemap_wrapper_lr,
    channel_wrapper_lr,
    tone_wrapper_lr, music_wrapper_lr, resource_wrapper_lr,
    network_wrapper_lr, audio_wrapper_lr, graphics_wrapper_lr, input_wrapper_lr,
    constant_wrapper_lr,
    system_wrapper_lr, variable_wrapper_lr,
};

// Re-exported at the crate root so every *_wrapper_lr module can keep
// using its existing `use crate::*;` to reach these, the same way it
// already does for PYXEL_READY/PY_UPDATE/etc. below.
// validate_index!/define_live_list! (in utils_lr) don't need a
// re-export here — #[macro_export] already puts them at the crate
// root automatically, macro_rules! is special that way.
pub use utils_lr::warn_deprecated_once;
pub use font_wrapper_lr::PyFont;
pub use math_wrapper_lr::{ceil, floor, sqrt, sin, cos, atan2, rseed, rndi, rndf, nseed, clamp, sgn, noise};
pub use image_wrapper_lr::{PyImage, PyImageList};
pub use sound_wrapper_lr::{PySound, PySoundList, PySoundNotes, PySoundTones, PySoundVolumes, PySoundEffects};
pub use tilemap_wrapper_lr::{PyTilemap, PyTilemapList};
pub use channel_wrapper_lr::{PyChannel, PyChannelList};
pub use variable_wrapper_lr::{__getattr__, PyColors};
pub use tone_wrapper_lr::{PyTone, PyToneList, PyToneWavetable};
pub use music_wrapper_lr::{PyMusicSeq, PyMusicSeqs, PyMusic, PyMusicList};
pub use resource_wrapper_lr::{load, save, load_pal, save_pal, screenshot, screencast, reset_screencast, user_data_dir};
pub use network_wrapper_lr::{lr_download_file, lr_http_get};
pub use audio_wrapper_lr::{sound_set, play, playm, stop, gen_bgm, play_pos, sound_fn, music_fn, channel_fn};
pub use graphics_wrapper_lr::{
    cls, rect, text, pset, pget, blt, bltm, blt3d, bltm3d, image, tilemap_fn,
    image_load, image_pset, line, rectb, circ, circb, elli, ellib, tri, trib,
    fill, clip, camera, pal, dither,
};
pub use input_wrapper_lr::{
    btn, btnp, btnr, btnv, mouse, set_btn, set_btnv,
    set_mouse_pos, set_input_text, set_dropped_files,
};
pub use constant_wrapper_lr::add_module_constants;
pub use system_wrapper_lr::{
    init, run, quit, show, flip, reset, load_content, title, icon,
    perf_monitor, integer_scale, screen_mode, fullscreen, resize,
};

use std::os::raw::{c_uint, c_void};

use pyo3::prelude::*;
use pyo3::append_to_inittab;

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
    KEY_LGUI, KEY_RGUI, KEY_SHIFT, KEY_CTRL, KEY_ALT, KEY_GUI, KEY_NONE,
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
const SCREEN_W: u32 = 1024;
const SCREEN_H: u32 = 1024;
const FPS: u32      = 60;

// Game-requested FPS (set by pyxel.init(), default 30)
// Used to skip frames: a 30fps game runs update/draw every 2nd retro_run() call
static mut GAME_FPS: u32 = 30;

// RetroArch frame counter (incremented every retro_run())
static mut RETRO_FRAME_COUNT: u64 = 0;

// lr-pyxel managed frame_count (only incremented when game update runs)
static mut LR_FRAME_COUNT: u32 = 0;

// Deferred reset_all_button_states(): set true at content load/reset
// time instead of calling reset_all_button_states() immediately.
// pyxel_core's own internal frame_count() (distinct from
// LR_FRAME_COUNT above — see input_wrapper_lr.rs's __getattr__,
// which serves pyxel.frame_count from LR_FRAME_COUNT, never touching
// this one) only advances inside flip_screen(), which retro_run()
// only calls on a should_update frame. If
// reset_all_button_states() (which writes an explicit Released
// state, at whatever frame_count happens to be current, for every
// mapped key — see input.rs) ran immediately at content load, before
// flip_screen() had ever run even once, every one of those writes
// landed at the same frame_count that any same-session set_btn()
// call (test-only key-injection API) would also land on — and
// pyxel_core's own same-frame-transition detection then treated that
// as a Released-then-Pressed sequence, making btnr() misfire True
// for a key that was only ever pressed, never released. Confirmed by
// upstream's own test_input.py::TestSetButtonState::
// test_btnr_false_without_release, which doesn't call flip() at all.
// Deferring the actual reset to fire only after frame_count has
// genuinely advanced past its post-reset value sidesteps this
// without touching pyxel_core's frame_count (a value pyxel_core
// itself owns and manages) or its internals in any way — lr-pyxel's
// own design stance is to leave pyxel-core itself alone wherever
// possible.
static mut PENDING_BUTTON_RESET: bool = false;

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

// Note: fixed samples-per-frame constant removed — audio.rs now uses a
// running accumulator (SAMPLE_ACCUMULATOR) to handle the 22050/60 = 367.5
// non-integer sample rate instead of a single rounded constant.

// Splash screen: show for this many frames after content load
const SPLASH_FRAMES: u32 = 180; // 3 seconds @ 60fps
static mut SPLASH_COUNT: u32 = 0;

/// Content path requested by the frontend browser (set by pyxel.load_content())
pub static mut PENDING_CONTENT: Option<String> = None;

// ---------------------------------------------------------------------------
// Pyxel Python module registration
// ---------------------------------------------------------------------------
// Moved here from the old wrappers.rs (which no longer exists — this
// was its last remaining content besides __getattr__, which moved to
// wrapper_lr/variable_wrapper_lr.rs). Lives in lib.rs rather than its
// own file, matching upstream's own convention of keeping #[pymodule]
// in lib.rs.
// gil_used = true: opts out of free-threaded Python support (default
// since PyO3 0.28). Every pyclass in this module wraps Rc/RefCell-
// based resources (not Sync) and is marked `unsendable` accordingly —
// none of it is safe for genuinely concurrent access from multiple
// Python threads, so explicitly declaring "this module needs the GIL"
// is accurate rather than aspirational.
#[pymodule(gil_used = true)]
pub fn pyxel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
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
    m.add_function(wrap_pyfunction!(blt3d,       m)?)?;
    m.add_function(wrap_pyfunction!(bltm3d,      m)?)?;
    m.add_function(wrap_pyfunction!(image,       m)?)?;
    m.add_function(wrap_pyfunction!(tilemap_fn,  m)?)?;
    m.add_function(wrap_pyfunction!(image_load,  m)?)?;
    m.add_function(wrap_pyfunction!(image_pset,  m)?)?;
    m.add_function(wrap_pyfunction!(load,             m)?)?;
    m.add_function(wrap_pyfunction!(save,             m)?)?;
    m.add_function(wrap_pyfunction!(load_pal,         m)?)?;
    m.add_function(wrap_pyfunction!(save_pal,         m)?)?;
    m.add_function(wrap_pyfunction!(screenshot,       m)?)?;
    m.add_function(wrap_pyfunction!(screencast,       m)?)?;
    m.add_function(wrap_pyfunction!(reset_screencast, m)?)?;
    m.add_function(wrap_pyfunction!(user_data_dir,    m)?)?;
    // Network
    m.add_function(wrap_pyfunction!(lr_download_file, m)?)?;
    m.add_function(wrap_pyfunction!(lr_http_get,      m)?)?;
    // Input
    m.add_function(wrap_pyfunction!(btn,         m)?)?;
    m.add_function(wrap_pyfunction!(btnp,        m)?)?;
    m.add_function(wrap_pyfunction!(btnr,             m)?)?;
    m.add_function(wrap_pyfunction!(btnv,             m)?)?;
    m.add_function(wrap_pyfunction!(mouse,            m)?)?;
    m.add_function(wrap_pyfunction!(set_btn,          m)?)?;
    m.add_function(wrap_pyfunction!(set_btnv,         m)?)?;
    m.add_function(wrap_pyfunction!(set_mouse_pos,    m)?)?;
    m.add_function(wrap_pyfunction!(set_input_text,   m)?)?;
    m.add_function(wrap_pyfunction!(set_dropped_files,m)?)?;
    // Audio
    m.add_function(wrap_pyfunction!(sound_set,   m)?)?;
    m.add_function(wrap_pyfunction!(play,        m)?)?;
    m.add_function(wrap_pyfunction!(gen_bgm,     m)?)?;
    m.add_function(wrap_pyfunction!(playm,       m)?)?;
    m.add_function(wrap_pyfunction!(stop,        m)?)?;
    m.add_function(wrap_pyfunction!(play_pos,    m)?)?;
    m.add_function(wrap_pyfunction!(sound_fn,    m)?)?;
    m.add_function(wrap_pyfunction!(music_fn,    m)?)?;
    m.add_function(wrap_pyfunction!(channel_fn,  m)?)?;
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
    m.add_function(wrap_pyfunction!(load_content, m)?)?;
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
    m.add_class::<PyFont>()?;
    m.add_class::<PyColors>()?;
    m.add_class::<PyImageList>()?;
    m.add_class::<PySound>()?;
    m.add_class::<PySoundList>()?;
    m.add_class::<PyMusic>()?;
    m.add_class::<PyMusicList>()?;
    m.add_class::<PyTilemap>()?;
    m.add_class::<PyTilemapList>()?;
    m.add_class::<PyChannel>()?;
    m.add_class::<PyChannelList>()?;
    m.add_class::<PyTone>()?;
    m.add_class::<PyToneList>()?;

    Ok(())
}


