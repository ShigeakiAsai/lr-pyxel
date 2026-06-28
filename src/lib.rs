// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

use std::os::raw::{c_char, c_uint, c_void};

// Function pointer to video refresh callback provided by RetroArch
static mut VIDEO_CB: Option<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)> = None;

// Define core information visible in RetroArch/Lakka menus
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut c_void) {
    let info = info as *mut libretro_sys::SystemInfo;
    (*info).library_name = b"Pyxel\0".as_ptr() as *const c_char;
    (*info).library_version = b"0.1.0\0".as_ptr() as *const c_char;
    (*info).valid_extensions = b"py|pyxapp\0".as_ptr() as *const c_char;
    (*info).need_fullpath = false;
}

// Set up environment and negotiate pixel format with the frontend
#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(cb: unsafe extern "C" fn(c_uint, *mut c_void) -> bool) -> bool {
    // In libretro-sys 0.1.1, pixel format options are prefixed with PixelFormat
    let format = libretro_sys::PixelFormat::RGB565;
    cb(libretro_sys::ENVIRONMENT_SET_PIXEL_FORMAT, &format as *const _ as *mut c_void);
    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_video_refresh(cb: unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)) {
    VIDEO_CB = Some(cb);
}

// Load the game file (ROM)
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(_game: *const c_void) -> bool {
    true
}

// Main loop called every frame (60fps)
#[no_mangle]
pub unsafe extern "C" fn retro_run() {
    // Default Pyxel screen size dimensions
    const WIDTH: usize = 256;
    const HEIGHT: usize = 256;
    
    // RGB565 frame buffer filled with retro green (R=0, G=63, B=0 -> 0x07E0)
    static mut FRAME_BUFFER: [u16; WIDTH * HEIGHT] = [0x07E0; WIDTH * HEIGHT];

    if let Some(video_cb) = VIDEO_CB {
        // Pass the frame buffer data to RetroArch
        video_cb(
            FRAME_BUFFER.as_ptr() as *const c_void, 
            WIDTH as c_uint, 
            HEIGHT as c_uint, 
            WIDTH * 2 // Pitch: 256 pixels * 2 bytes per pixel
        );
    }
}

// Required boilerplate functions for Libretro API compliance
#[no_mangle] pub unsafe extern "C" fn retro_init() {}
#[no_mangle] pub unsafe extern "C" fn retro_deinit() {}
#[no_mangle] pub unsafe extern "C" fn retro_unload_game() {}
#[no_mangle] pub unsafe extern "C" fn retro_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_set_audio_sample(_cb: unsafe extern "C" fn(i16, i16)) {}
#[no_mangle] pub unsafe extern "C" fn retro_set_audio_sample_batch(_cb: unsafe extern "C" fn(*const i16, usize) -> usize) {}
#[no_mangle] pub unsafe extern "C" fn retro_set_input_poll(_cb: unsafe extern "C" fn()) {}
#[no_mangle] pub unsafe extern "C" fn retro_set_input_state(_cb: unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16) {}

#[no_mangle] 
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut c_void) {
    let info = info as *mut libretro_sys::SystemAVInfo;
    (*info).geometry.base_width = 256;
    (*info).geometry.base_height = 256;
    (*info).geometry.max_width = 256;
    (*info).geometry.max_height = 256;
    (*info).geometry.aspect_ratio = 1.0;
    (*info).timing.fps = 60.0;
    (*info).timing.sample_rate = 44100.0;
}

#[no_mangle] pub unsafe extern "C" fn retro_api_version() -> c_uint { libretro_sys::API_VERSION }
#[no_mangle] pub unsafe extern "C" fn retro_unserialize(_data: *const c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_serialize(_data: *mut c_void, _size: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_serialize_size() -> usize { 0 }
#[no_mangle] pub unsafe extern "C" fn retro_cheat_reset() {}
#[no_mangle] pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, _is_enabled: bool, _code: *const c_char) {}
#[no_mangle] pub unsafe extern "C" fn retro_load_game_special(_game_type: c_uint, _info: *const c_void, _num_info: usize) -> bool { false }
#[no_mangle] pub unsafe extern "C" fn retro_region() -> c_uint { libretro_sys::REGION_NTSC }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void { std::ptr::null_mut() }
#[no_mangle] pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> usize { 0 }
