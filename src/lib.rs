// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};
use pyo3::prelude::*;
use pyo3::types::PyModule;

static mut VIDEO_CB:   Option<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)>    = None;
static mut INPUT_POLL: Option<unsafe extern "C" fn()>                                         = None;
static mut INPUT_STATE:Option<unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16>   = None;
static mut ENVIRON_CB: Option<unsafe extern "C" fn(c_uint, *mut c_void) -> bool>              = None;

// Screen dimensions — must match pyxel_bridge.SCREEN_W / SCREEN_H
const SCREEN_W: usize = 128;
const SCREEN_H: usize = 128;

// Fallback green frame shown before Pyxel initializes
const COLOR_GREEN: u16 = 0x07E0;

// Cached pyxel_bridge module handle
static mut BRIDGE: Option<Py<PyModule>> = None;

// ---------------------------------------------------------------------------
// Environment / pixel format
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(
    cb: unsafe extern "C" fn(c_uint, *mut c_void) -> bool,
) {
    ENVIRON_CB = Some(cb);

    // Must be called first so RetroArch shows "Start Core" without content
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
    _cb: unsafe extern "C" fn(*const i16, usize) -> usize,
) -> usize {
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
    (*info).library_version  = b"0.2.0\0".as_ptr() as *const c_char;
    (*info).valid_extensions = b"py\0".as_ptr()    as *const c_char;
    (*info).need_fullpath    = true;   // pass the .py file path as-is
    (*info).block_extract    = false;
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut c_void) {
    let info = info as *mut rust_libretro_sys::retro_system_av_info;
    (*info).geometry.base_width   = SCREEN_W as c_uint;
    (*info).geometry.base_height  = SCREEN_H as c_uint;
    (*info).geometry.max_width    = SCREEN_W as c_uint;
    (*info).geometry.max_height   = SCREEN_H as c_uint;
    (*info).geometry.aspect_ratio = 1.0;
    (*info).timing.fps            = 60.0;
    (*info).timing.sample_rate    = 44100.0;
}

// ---------------------------------------------------------------------------
// Init / deinit
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_init() {
    Python::with_gil(|py| {
        // Add the libretro core directory to sys.path so pyxel_bridge.py is found
        let sys  = py.import_bound("sys").expect("failed to import sys");
        let path = sys.getattr("path").unwrap();
        let path = path.downcast_into::<pyo3::types::PyList>().unwrap();
        path.insert(0, "/usr/lib/libretro").unwrap();

        match py.import_bound("pyxel_bridge") {
            Ok(module) => match module.call_method0("init") {
                Ok(_)  => { BRIDGE = Some(module.unbind()); }
                Err(e) => { e.print(py); }
            },
            Err(e) => { e.print(py); }
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn retro_deinit() {
    BRIDGE = None;
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
pub unsafe extern "C" fn retro_load_game(game: *const c_void) -> bool {
    // Allow content-less boot
    if game.is_null() {
        return true;
    }

    let info = &*(game as *const RetroGameInfo);
    if info.path.is_null() {
        return true;
    }

    let path = CStr::from_ptr(info.path).to_string_lossy().into_owned();

    if let Some(ref bridge) = BRIDGE {
        Python::with_gil(|py| {
            match bridge.bind(py).call_method1("load_game", (path,)) {
                Ok(result) => result.extract::<bool>().unwrap_or(false),
                Err(e)     => { e.print(py); false }
            }
        })
    } else {
        true // no bridge yet — content-less boot
    }
}

#[no_mangle]
pub unsafe extern "C" fn retro_unload_game() {
    if let Some(ref bridge) = BRIDGE {
        Python::with_gil(|py| {
            let _ = bridge.bind(py).call_method0("unload_game");
        });
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_run() {
    // 1. Poll input
    if let Some(poll) = INPUT_POLL {
        poll();
    }

    // 2. Collect button bitmask (16 joypad buttons)
    let mut buttons: u32 = 0;
    if let Some(state) = INPUT_STATE {
        for bit in 0u32..16 {
            if state(0, rust_libretro_sys::RETRO_DEVICE_JOYPAD, 0, bit) != 0 {
                buttons |= 1 << bit;
            }
        }
    }

    // 3. Check SELECT (bit 2) for shutdown
    if buttons & (1 << 2) != 0 {
        if let Some(env) = ENVIRON_CB {
            env(rust_libretro_sys::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut());
        }
        return;
    }

    // 4. Forward input and advance one frame via pyxel_bridge
    let fb_bytes: Option<Vec<u8>> = if let Some(ref bridge) = BRIDGE {
        Python::with_gil(|py| {
            let b = bridge.bind(py);
            let _ = b.call_method1("set_input", (buttons,));
            let _ = b.call_method0("run_frame");
            match b.call_method0("get_framebuffer") {
                Ok(fb) => fb.extract::<Vec<u8>>().ok(),
                Err(e) => { e.print(py); None }
            }
        })
    } else {
        None
    };

    // 5. Submit framebuffer to RetroArch
    if let Some(video) = VIDEO_CB {
        match fb_bytes {
            Some(ref fb) if fb.len() == SCREEN_W * SCREEN_H * 2 => {
                video(
                    fb.as_ptr() as *const c_void,
                    SCREEN_W as c_uint,
                    SCREEN_H as c_uint,
                    SCREEN_W * 2,
                );
            }
            _ => {
                // Fallback: show solid green until bridge is ready
                let fallback = vec![
                    (COLOR_GREEN & 0xFF) as u8,
                    ((COLOR_GREEN >> 8) & 0xFF) as u8,
                ]
                .repeat(SCREEN_W * SCREEN_H);
                video(
                    fallback.as_ptr() as *const c_void,
                    SCREEN_W as c_uint,
                    SCREEN_H as c_uint,
                    SCREEN_W * 2,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Required stubs
// ---------------------------------------------------------------------------

#[no_mangle] pub unsafe extern "C" fn retro_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}
#[no_mangle] pub unsafe extern "C" fn retro_api_version() -> c_uint { rust_libretro_sys::RETRO_API_VERSION as c_uint }
#[no_mangle] pub unsafe extern "C" fn retro_serialize_size() -> usize { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_cheat_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, _is_enabled: bool, _code: *const c_char) {}
#[no_mangle] pub unsafe extern "C" fn retro_load_game_special(_type: c_uint, _info: *const c_void, _num: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_get_region() -> c_uint { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void { std::ptr::null_mut() }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> usize { 0 }
