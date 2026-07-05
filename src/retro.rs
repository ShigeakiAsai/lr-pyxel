//! retro.rs — libretro C API entry points for lr-pyxel

use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};
use pyo3::prelude::*;

use crate::*;

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
    // Always write stubs (even on re-init) so they're up to date
    const MATH_PY:   &str = include_str!("../math.py");
    const RANDOM_PY: &str = include_str!("../random.py");
    let stub_dir = std::path::Path::new("/tmp/lr-pyxel-stdlib");
    let _ = std::fs::create_dir_all(stub_dir);
    let _ = std::fs::write(stub_dir.join("math.py"),   MATH_PY);
    let _ = std::fs::write(stub_dir.join("random.py"), RANDOM_PY);

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

    video::build_palette_lut();
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
// ---------------------------------------------------------------------------
// Static analysis: extract pyxel.init() arguments from script
// ---------------------------------------------------------------------------

// Parse pyxel.init(w, h, ..., fps=N, ...) from a Python script.
// Returns (width, height, fps) if found, None otherwise.
fn parse_pyxel_init(script: &str) -> Option<(u32, u32, u32)> {
    // Build variable map from simple assignments (VAR = NUMBER)
    let mut var_map: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for line in script.lines() {
        let line = line.trim();
        if line.starts_with('#') { continue; }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let val_str = line[eq_pos+1..].trim();
            if key.chars().all(|c| c.is_alphanumeric() || c == '_') && !key.is_empty() {
                if let Ok(n) = val_str.parse::<u32>() {
                    var_map.insert(key, n);
                }
            }
        }
    }

    // Find pyxel.init( and extract content
    let init_pos = script.find("pyxel.init(")?;
    let after = &script[init_pos + "pyxel.init(".len()..];
    let mut depth = 1;
    let mut end = 0;
    for (i, c) in after.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => { depth -= 1; if depth == 0 { end = i; break; } }
            _ => {}
        }
    }
    let args_str = &after[..end];

    // Helper: resolve value (literal or variable)
    let resolve = |s: &str| -> Option<u32> {
        let s = s.trim().trim_end_matches(',').trim();
        s.parse::<u32>().ok().or_else(|| var_map.get(s).copied())
    };

    // Extract each argument line by line
    let mut w: Option<u32> = None;
    let mut h: Option<u32> = None;
    let mut fps: Option<u32> = None;
    let mut positional = 0;

    for line in args_str.lines() {
        let line = line.trim().trim_end_matches(',').trim();
        if line.is_empty() { continue; }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let val = resolve(&line[eq_pos+1..]);
            match key {
                "w" | "width"  => w = val,
                "h" | "height" => h = val,
                "fps"          => fps = val,
                _ => {}
            }
        } else {
            match positional {
                0 => w = resolve(line),
                1 => h = resolve(line),
                3 => fps = resolve(line),
                _ => {}
            }
            positional += 1;
        }
    }

    Some((
        w.unwrap_or(128),
        h.unwrap_or(128),
        fps.unwrap_or(30),
    ))
}


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

    // Static analysis: parse pyxel.init() args BEFORE running the script
    // to set the correct screen size and fps (problem⑤)
    if let Ok(code) = std::fs::read_to_string(&script_path) {
        if let Some((w, h, fps)) = parse_pyxel_init(&code) {
            eprintln!("[lr-pyxel] parsed init: w={w} h={h} fps={fps}");
            GAME_W   = w;
            GAME_H   = h;
            GAME_FPS = fps;
        } else {
            eprintln!("[lr-pyxel] parse_pyxel_init: not found, using defaults");
        }
    }

    // Notify RetroArch of geometry with parsed size
    if let Some(env) = ENVIRON_CB {
        let geometry = rust_libretro_sys::retro_game_geometry {
            base_width:   GAME_W,
            base_height:  GAME_H,
            max_width:    256,
            max_height:   256,
            aspect_ratio: GAME_W as f32 / GAME_H as f32,
        };
        env(37, &geometry as *const _ as *mut c_void);
    }

    Python::with_gil(|py| {
        // Drop previous game callbacks inside GIL to avoid double-free
        PY_UPDATE = None;
        PY_DRAW   = None;

        // Reset frame counters for new content
        RETRO_FRAME_COUNT = 0;
        LR_FRAME_COUNT    = 0;
        audio::PREV_BUTTONS = 0;

        // Clear cached modules from previous game to prevent import conflicts.
        // Without this, modules like 'constants' from game A would be reused
        // when game B tries to import its own 'constants' module.
        if let Ok(sys) = pyo3::Python::import_bound(py, "sys") {
            if let Ok(modules) = sys.getattr("modules") {
                if let Ok(modules_dict) = modules.downcast_into::<pyo3::types::PyDict>() {
                    // Keep only stdlib and built-in modules, remove game modules
                    let keys_to_remove: Vec<String> = modules_dict
                        .keys()
                        .iter()
                        .filter_map(|k| k.extract::<String>().ok())
                        .filter(|k| {
                            !k.starts_with('_')
                                && !matches!(k.as_str(),
                                    "sys" | "builtins" | "pyxel" | "os" | "os.path"
                                    | "io" | "abc" | "types" | "typing" | "functools"
                                    | "collections" | "itertools" | "operator"
                                    | "re" | "enum" | "warnings" | "weakref"
                                )
                        })
                        .collect();
                    for key in keys_to_remove {
                        let _ = modules_dict.del_item(key);
                    }
                }
            }
        }

        // Stop all audio and reset BlipBuf to prevent previous content's
        // audio from bleeding into the next content (problem②)
        if PYXEL_READY {
            pyxel_core::pyxel().stop_all_channels();
        }
        if let Some(ref mut blip) = BLIP_BUF {
            *blip = blip_buf::BlipBuf::new(1024);
            blip.set_rates(
                pyxel_core::AUDIO_CLOCK_RATE as f64,
                pyxel_core::AUDIO_SAMPLE_RATE as f64,
            );
        }

        // Add game directory to sys.path and set as working directory.
        // First, remove any previous game directories from sys.path to prevent
        // module name conflicts between different games (problem: laser-jetman
        // importing cursed_caverns' constants.py)
        let sys     = py.import_bound("sys").expect("failed to import sys");
        let syspath = sys.getattr("path").unwrap();
        let syspath = syspath.downcast_into::<pyo3::types::PyList>().unwrap();

        // Remove all /tmp/lr-pyxel/ entries from sys.path
        let mut i = 0;
        while i < syspath.len() {
            if let Ok(s) = syspath.get_item(i).and_then(|item| item.extract::<String>()) {
                if s.contains("/tmp/lr-pyxel/") || s.contains("\\tmp\\lr-pyxel\\") {
                    let _ = syspath.del_item(i);
                    continue;
                }
            }
            i += 1;
        }

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
        video::submit_fallback_frame();
        return;
    }

    // 4. Frame sync: increment RetroArch counter and determine if game should update.
    //    RetroArch drives at 60fps; game may target 30fps (or other).
    //    Only run update()/draw()/flip_screen()/audio when it's the game's turn.
    RETRO_FRAME_COUNT += 1;
    let step = (FPS / GAME_FPS).max(1) as u64;
    let should_update = RETRO_FRAME_COUNT % step == 0;

    if unsafe { PY_UPDATE.is_some() || PY_DRAW.is_some() } {
        if should_update {
            // Increment lr-pyxel's frame_count (returned by pyxel.frame_count)
            LR_FRAME_COUNT += 1;

            Python::with_gil(|py| {
                if let Some(ref update) = PY_UPDATE {
                    if let Err(e) = update.call0(py) { e.print(py); }
                }
                if let Some(ref draw) = PY_DRAW {
                    if let Err(e) = draw.call0(py) { e.print(py); }
                }
            });

            // 5. Advance Pyxel's internal audio clock only when game updates.
            //    This keeps audio speed in sync with game speed.
            pyxel_core::pyxel().flip_screen();

            // 6. Inject input AFTER flip_screen()
            audio::inject_input(buttons);

            // 8. Render and submit audio samples (only on game frames)
            audio::submit_audio_frame();
        }
    } else {
        // No game loaded — light blue placeholder
        pyxel_core::pyxel().clear(11);
    }

    // 7. Submit framebuffer to RetroArch every frame to keep display smooth
    video::submit_pyxel_frame();
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// → moved to video.rs

// → PREV_BUTTONS and inject_input moved to audio.rs

// → moved to video.rs

// → moved to video.rs

// → submit_audio_frame moved to audio.rs

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
