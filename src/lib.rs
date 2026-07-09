// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

mod video;
mod audio;
mod retro;
mod wrappers;
mod splash;

// Re-export pyxel module fn so append_to_inittab!(pyxel) works in retro.rs
pub use wrappers::pyxel;

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
