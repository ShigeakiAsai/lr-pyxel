// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

use std::os::raw::{c_char, c_uint, c_void};

static mut VIDEO_CB:    Option<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)> = None;
static mut INPUT_POLL:  Option<unsafe extern "C" fn()>                                      = None;
static mut INPUT_STATE: Option<unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16> = None;
static mut ENVIRON_CB:  Option<unsafe extern "C" fn(c_uint, *mut c_void) -> bool>           = None;

// RGB565: white (0xFFFF) used for the exit flash
const COLOR_GREEN: u16 = 0x07E0;
const COLOR_WHITE: u16 = 0xFFFF;

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut c_void) {
    let info = info as *mut rust_libretro_sys::retro_system_info;
    (*info).library_name = b"Pyxel\0".as_ptr() as *const c_char;
    (*info).library_version = b"0.1.0\0".as_ptr() as *const c_char;
    
    // Fixed: Set extensions to empty and need_fullpath to true to ensure "Start Core" behaves correctly
    (*info).valid_extensions = b"\0".as_ptr() as *const c_char;
    (*info).need_fullpath = true;
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(cb: unsafe extern "C" fn(c_uint, *mut c_void) -> bool) -> bool {
    ENVIRON_CB = Some(cb);
    
    let format = rust_libretro_sys::retro_pixel_format_RETRO_PIXEL_FORMAT_RGB565;
    cb(rust_libretro_sys::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, &format as *const _ as *mut c_void);

    let support_no_game = true;
    cb(rust_libretro_sys::RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME, &support_no_game as *const _ as *mut c_void);

    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_video_refresh(cb: unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)) {
    VIDEO_CB = Some(cb);
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample(_cb: unsafe extern "C" fn(i16, i16)) {}
#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample_batch(_cb: unsafe extern "C" fn(*const i16, usize) -> usize) -> usize { 0 }

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_poll(cb: unsafe extern "C" fn()) {
    INPUT_POLL = Some(cb);
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_state(cb: unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16) {
    INPUT_STATE = Some(cb);
}

#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const c_void) -> bool {
    // Safely allow standalone boot even when game info pointer is null
    if game.is_null() {
        return true;
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_run() {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 256;

    if let Some(input_poll) = INPUT_POLL {
        input_poll();
    }

    let mut select_pressed = false;
    if let Some(input_state) = INPUT_STATE {
        // DEVICE_JOYPAD = 1, DEVICE_ID_JOYPAD_A = 0
        let state = input_state(0, 1, 0, 0);
        if state != 0 {
            select_pressed = true;
        }
    }

    let frame_buffer = if select_pressed {
        if let Some(environ_cb) = ENVIRON_CB {
            environ_cb(rust_libretro_sys::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut());
        }
        [COLOR_WHITE; WIDTH * HEIGHT]
    } else {
        [COLOR_GREEN; WIDTH * HEIGHT]
    };

    if let Some(video_cb) = VIDEO_CB {
        video_cb(
            frame_buffer.as_ptr() as *const c_void,
            WIDTH as c_uint,
            HEIGHT as c_uint,
            WIDTH * 2,
        );
    }
}

#[no_mangle] pub unsafe extern "C" fn retro_init() {}
#[no_mangle] pub unsafe extern "C" fn retro_deinit() {}
#[no_mangle] pub unsafe extern "C" fn retro_unload_game() {}
#[no_mangle] pub unsafe extern "C" fn retro_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut c_void) {
    let info = info as *mut rust_libretro_sys::retro_system_av_info;
    (*info).geometry.base_width = 256;
    (*info).geometry.base_height = 256;
    (*info).geometry.max_width = 256;
    (*info).geometry.max_height = 256;
    (*info).geometry.aspect_ratio = 1.0;
    (*info).timing.fps = 60.0;
    (*info).timing.sample_rate = 44100.0;
}

#[no_mangle] pub unsafe extern "C" fn retro_api_version() -> c_uint { rust_libretro_sys::RETRO_API_VERSION as c_uint }
#[no_mangle] pub unsafe extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_serialize_size() -> usize { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_cheat_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, _is_enabled: bool, _code: *const c_char) {}
#[no_mangle] pub unsafe extern "C" fn retro_load_game_special(_game_type: c_uint, _info: *const c_void, _num_info: usize) -> bool { false }

#[no_mangle] pub unsafe extern "C" fn retro_get_region() -> c_uint { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void { std::ptr::null_mut() }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> usize { 0 }
