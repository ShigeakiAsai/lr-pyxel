// SPDX-License-Identifier: MIT
// Copyright (c) 2026-present Yasai-san

use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};

use pyo3::prelude::*;
use pyo3::append_to_inittab;
use pyo3::types::PyModule;

use pyxel_core::{
    colors, height, init as pyxel_init, screen, width,
    KEY_A, KEY_DOWN, KEY_LEFT, KEY_RETURN, KEY_RIGHT, KEY_S, KEY_UP, KEY_X, KEY_Z,
};

static mut VIDEO_CB:    Option<unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize)>   = None;
static mut INPUT_POLL:  Option<unsafe extern "C" fn()>                                        = None;
static mut INPUT_STATE: Option<unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16>  = None;
static mut ENVIRON_CB:  Option<unsafe extern "C" fn(c_uint, *mut c_void) -> bool>             = None;

// Screen dimensions
const SCREEN_W: u32 = 128;
const SCREEN_H: u32 = 128;
const FPS: u32      = 60;

// Pre-built RGB565 palette LUT (256 entries)
static mut PALETTE_RGB565: [u16; 256] = [0u16; 256];

// True once pyxel_init() has succeeded
static mut PYXEL_READY: bool = false;

// Cached Python game callbacks
static mut PY_UPDATE: Option<Py<PyAny>> = None;
static mut PY_DRAW:   Option<Py<PyAny>> = None;

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
#[pyo3(signature = (x, y, img, u, v, w, h, colkey=None))]
#[allow(clippy::too_many_arguments)]
fn blt(x: f32, y: f32, img: u32, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_image(x, y, img, u, v, w, h, colkey, None, None);
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
    let _ = (w, h, title, fps, quit_key, display_scale, capture_scale, capture_sec);
}

// run() is a no-op: frame loop is driven by retro_run()
#[pyfunction]
fn run(_update: PyObject, _draw: PyObject) {}

// -- key constants -----------------------------------------------------------

fn add_key_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("KEY_UP",     pyxel_core::KEY_UP)?;
    m.add("KEY_DOWN",   pyxel_core::KEY_DOWN)?;
    m.add("KEY_LEFT",   pyxel_core::KEY_LEFT)?;
    m.add("KEY_RIGHT",  pyxel_core::KEY_RIGHT)?;
    m.add("KEY_Z",      pyxel_core::KEY_Z)?;
    m.add("KEY_X",      pyxel_core::KEY_X)?;
    m.add("KEY_A",      pyxel_core::KEY_A)?;
    m.add("KEY_S",      pyxel_core::KEY_S)?;
    m.add("KEY_RETURN", pyxel_core::KEY_RETURN)?;
    m.add("KEY_ESCAPE", pyxel_core::KEY_ESCAPE)?;
    Ok(())
}

// -- module registration -----------------------------------------------------

#[pymodule]
fn pyxel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cls,         m)?)?;
    m.add_function(wrap_pyfunction!(rect,        m)?)?;
    m.add_function(wrap_pyfunction!(text,        m)?)?;
    m.add_function(wrap_pyfunction!(pset,        m)?)?;
    m.add_function(wrap_pyfunction!(pget,        m)?)?;
    m.add_function(wrap_pyfunction!(blt,         m)?)?;
    m.add_function(wrap_pyfunction!(image_load,  m)?)?;
    m.add_function(wrap_pyfunction!(image_pset,  m)?)?;
    m.add_function(wrap_pyfunction!(btn,         m)?)?;
    m.add_function(wrap_pyfunction!(btnp,        m)?)?;
    m.add_function(wrap_pyfunction!(frame_count, m)?)?;
    m.add_function(wrap_pyfunction!(init,        m)?)?;
    m.add_function(wrap_pyfunction!(run,         m)?)?;
    add_key_constants(m)?;
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
    (*info).library_version  = b"0.4.1\0".as_ptr() as *const c_char;
    (*info).valid_extensions = b"py\0".as_ptr()    as *const c_char;
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
    (*info).timing.sample_rate    = 44100.0;
}

// ---------------------------------------------------------------------------
// Init / deinit
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn retro_init() {
    // Register "pyxel" built-in module BEFORE Py_Initialize
    append_to_inittab!(pyxel);

    // Initialize Pyxel engine in headless mode
    pyxel_init(
        SCREEN_W, SCREEN_H,
        Some("lr-pyxel"),
        Some(FPS),
        None, None, None, None,
        Some(true),        // headless = true
    );

    build_palette_lut();
    PYXEL_READY = true;

    // Start Python interpreter (after append_to_inittab)
    pyo3::prepare_freethreaded_python();
}

#[no_mangle]
pub unsafe extern "C" fn retro_deinit() {
    PY_UPDATE   = None;
    PY_DRAW     = None;
    PYXEL_READY = false;
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
    if game.is_null() {
        return true; // content-less boot
    }
    let info = &*(game as *const RetroGameInfo);
    if info.path.is_null() {
        return true;
    }

    let path = CStr::from_ptr(info.path).to_string_lossy().into_owned();

    Python::with_gil(|py| {
        // Add game directory to sys.path
        let sys     = py.import_bound("sys").expect("failed to import sys");
        let syspath = sys.getattr("path").unwrap();
        let syspath = syspath.downcast_into::<pyo3::types::PyList>().unwrap();
        let game_dir = std::path::Path::new(&path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_string_lossy()
            .into_owned();
        syspath.insert(0, game_dir).unwrap();

        // Execute the game script
        let code    = std::fs::read_to_string(&path).unwrap_or_default();
        let globals = pyo3::types::PyDict::new_bound(py);

        match py.run_bound(&code, Some(&globals), None) {
            Ok(_) => {
                // Cache update() and draw() if defined at module level
                PY_UPDATE = globals.get_item("update").ok()
                    .flatten()
                    .map(|f| f.into_py(py));
                PY_DRAW = globals.get_item("draw").ok()
                    .flatten()
                    .map(|f| f.into_py(py));
            }
            Err(e) => {
                // Print the error but still return true: RetroArch already
                // committed to loading this core's content (need_fullpath=true
                // skipped its own file read), so returning false here only
                // produces a generic "Failed to load content" with no detail.
                // Printing here gives the real Python traceback in the log.
                e.print(py);
            }
        }
    });

    // Always report success once we've reached this point: the .py file
    // existed and was readable. Script errors are surfaced via e.print(py)
    // above and result in PY_UPDATE/PY_DRAW staying None, which falls back
    // to the placeholder screen in retro_run().
    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_unload_game() {
    PY_UPDATE = None;
    PY_DRAW   = None;
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

    // 4. Inject input into Pyxel
    inject_input(buttons);

    // 5. Call Python game callbacks if loaded, otherwise show placeholder
    if PY_UPDATE.is_some() || PY_DRAW.is_some() {
        Python::with_gil(|py| {
            if let Some(ref update) = PY_UPDATE {
                if let Err(e) = update.call0(py) { e.print(py); }
            }
            if let Some(ref draw) = PY_DRAW {
                if let Err(e) = draw.call0(py) { e.print(py); }
            }
        });
    } else {
        // No game loaded — light blue placeholder
        pyxel_core::pyxel().clear(11);
    }

    // 6. Advance one Pyxel frame
    pyxel_core::pyxel().flip_screen();

    // 7. Submit framebuffer to RetroArch
    submit_pyxel_frame();
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

unsafe fn inject_input(buttons: u32) {
    const MAP: &[(u32, u32)] = &[
        (0, KEY_Z),
        (1, KEY_X),
        (3, KEY_RETURN),
        (4, KEY_UP),
        (5, KEY_DOWN),
        (6, KEY_LEFT),
        (7, KEY_RIGHT),
        (8, KEY_A),
        (9, KEY_S),
    ];
    let px = pyxel_core::pyxel();
    for &(bit, key) in MAP {
        px.set_button_state(key, buttons & (1 << bit) != 0);
    }
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
