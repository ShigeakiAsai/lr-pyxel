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
    (*info).geometry.max_width    = 512;
    (*info).geometry.max_height   = 512;
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
    const STRUCT_PY: &str = include_str!("../struct.py");
    let stub_dir = std::path::Path::new("/tmp/lr-pyxel-stdlib");
    let _ = std::fs::create_dir_all(stub_dir);
    let _ = std::fs::write(stub_dir.join("math.py"),   MATH_PY);
    let _ = std::fs::write(stub_dir.join("random.py"), RANDOM_PY);
    let _ = std::fs::write(stub_dir.join("struct.py"), STRUCT_PY);

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

    // Remove problematic .so modules from sys.modules so our stubs are loaded
    Python::with_gil(|py| {
        if let Ok(sys) = py.import_bound("sys") {
            if let Ok(path) = sys.getattr("path") {
                if let Ok(syspath) = path.downcast_into::<pyo3::types::PyList>() {
                    let _ = syspath.insert(0, "/tmp/lr-pyxel-stdlib");
                }
            }
            if let Ok(modules) = sys.getattr("modules") {
                if let Ok(d) = modules.downcast_into::<pyo3::types::PyDict>() {
                    let _ = d.del_item("math");
                    let _ = d.del_item("random");
                    let _ = d.del_item("struct");
                }
            }
        }
    });
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
// Splits a comma-separated argument list at top-level commas only,
// respecting nested parens/brackets/braces and quoted strings (so a
// keyword arg like `title="a, b"` or `pos=(1, 2)` isn't split apart).
// This lets pyxel.init(...) calls be parsed whether written all on one
// line or spread across several — the previous implementation assumed
// "one argument per line", which silently failed for the very common
// single-line style `pyxel.init(464, 256, title="...")` (the comma
// splitting was never done, so the whole call was treated as one
// argument and `w`/`h` were never found).
fn split_top_level_args(s: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let mut prev_was_backslash = false;
    let mut start = 0usize;

    for (idx, c) in s.char_indices() {
        if let Some(qc) = in_str {
            if c == qc && !prev_was_backslash {
                in_str = None;
            }
            prev_was_backslash = c == '\\' && !prev_was_backslash;
            continue;
        }
        prev_was_backslash = false;
        match c {
            '\'' | '"' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                args.push(s[start..idx].trim());
                start = idx + c.len_utf8();
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

// Default 16-color Pyxel palette (mirrors pyxel_core::settings::DEFAULT_COLORS).
// pyxel_core::colors() is a single process-wide list — pyxel_core::init()
// can only run once per process, so lr-pyxel never re-initializes it on
// content switches, it only resets its own state (GAME_W/H, frame counts,
// etc). Image.from_image(..., include_colors=True) permanently overwrites
// this shared palette, so without an explicit reset here, a game's custom
// palette leaks into whatever loads next (the launcher or another game).
const DEFAULT_PYXEL_COLORS: [u32; 16] = [
    0x0000_00, 0x2b33_5f, 0x7e20_72, 0x1995_9c, 0x8b48_52, 0x395c_98, 0xa9c1_ff, 0xeeee_ee,
    0xd418_6c, 0xd384_41, 0xe9c3_5b, 0x70c6_a9, 0x7696_de, 0xa3a3_a3, 0xff97_98, 0xedc7_b0,
];

unsafe fn reset_color_palette() {
    if PYXEL_READY {
        *pyxel_core::colors() = DEFAULT_PYXEL_COLORS.to_vec();
    }
}

// Show a short on-screen notification via RetroArch's own OSD message
// system (RETRO_ENVIRONMENT_SET_MESSAGE = 12), instead of building a
// custom in-Pyxel error screen. Used when a script fails to load (e.g.
// unsupported patterns like flip()-based main loops or `import pyxel.cli`)
// so the person actually sees *something* on screen, rather than the
// core silently bouncing back to the launcher with only a stderr
// traceback (invisible on a TV with no attached console).
unsafe fn show_retroarch_message(text: &str, frames: u32) {
    if let Some(env) = ENVIRON_CB {
        if let Ok(cmsg) = std::ffi::CString::new(text) {
            let message = rust_libretro_sys::retro_message {
                msg: cmsg.as_ptr(),
                frames,
            };
            env(6, &message as *const _ as *mut c_void); // RETRO_ENVIRONMENT_SET_MESSAGE
        }
    }
}

// Collects simple `VAR = NUMBER` top-level assignments from a chunk of
// Python source text into var_map. Used both for the entry script itself
// and for sibling .py files (see read_sibling_py_sources), so constants
// imported from another module (e.g. `from const import APP_WIDTH`) can
// still be resolved.
fn collect_int_vars(text: &str, var_map: &mut std::collections::HashMap<String, u32>) {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') { continue; }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let val_str = line[eq_pos+1..].trim();
            if key.chars().all(|c| c.is_alphanumeric() || c == '_') && !key.is_empty() {
                if let Ok(n) = val_str.parse::<u32>() {
                    var_map.insert(key.to_string(), n);
                }
            }
        }
    }
}

// Reads every sibling .py file next to the entry script (same directory),
// for cross-file constant resolution in parse_pyxel_init(). Packages with
// subdirectories (entities/, scenes/, etc.) aren't recursed into — this
// only covers the common "constants live in a sibling module" pattern.
fn read_sibling_py_sources(script_path: &str) -> Vec<String> {
    let mut sources = Vec::new();
    let script_path = std::path::Path::new(script_path);
    let Some(dir) = script_path.parent() else { return sources; };
    let Ok(entries) = std::fs::read_dir(dir) else { return sources; };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("py")
            && path.file_name() != script_path.file_name()
        {
            if let Ok(content) = std::fs::read_to_string(&path) {
                sources.push(content);
            }
        }
    }
    sources
}

fn parse_pyxel_init(script: &str, extra_sources: &[String]) -> Option<(u32, u32, u32)> {
    // Detect `import pyxel as ALIAS` so `ALIAS.init(...)` is found too —
    // searching only for the literal "pyxel.init(" silently misses scripts
    // like vortexion's main.py, which does `import pyxel as px` then
    // calls `px.init(...)`.
    let pyxel_name = {
        let mut name = "pyxel".to_string();
        for line in script.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("import pyxel as ") {
                let alias = rest.trim().split_whitespace().next()
                    .unwrap_or("").trim_end_matches(',');
                if !alias.is_empty() {
                    name = alias.to_string();
                    break;
                }
            }
        }
        name
    };
    let init_needle = format!("{pyxel_name}.init(");

    // Build variable map from simple assignments (VAR = NUMBER), scanning
    // both the entry script and any sibling .py files.
    let mut var_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    collect_int_vars(script, &mut var_map);
    for extra in extra_sources {
        collect_int_vars(extra, &mut var_map);
    }

    // Find <pyxel_name>.init( and extract content
    let init_pos = script.find(&init_needle)?;
    let after = &script[init_pos + init_needle.len()..];
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

    // Extract each argument, whether the call spans one line or several
    let mut w: Option<u32> = None;
    let mut h: Option<u32> = None;
    let mut fps: Option<u32> = None;
    let mut positional = 0;

    for arg in split_top_level_args(args_str) {
        if arg.is_empty() { continue; }

        if let Some(eq_pos) = arg.find('=') {
            let key = arg[..eq_pos].trim();
            let val = resolve(&arg[eq_pos+1..]);
            match key {
                "w" | "width"  => w = val,
                "h" | "height" => h = val,
                "fps"          => fps = val,
                _ => {}
            }
        } else {
            match positional {
                0 => w = resolve(arg),
                1 => h = resolve(arg),
                3 => fps = resolve(arg),
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
        // Content-less boot: show the splash screen first (see retro_run(),
        // which calls launch_frontend() once SPLASH_COUNT reaches
        // SPLASH_FRAMES). Previously this ran frontend.py immediately,
        // which set PY_UPDATE/PY_DRAW before retro_run() ever got a
        // chance to take the splash branch — so the splash was never
        // actually shown.
        PY_UPDATE = None;
        PY_DRAW   = None;
        SPLASH_COUNT = 0;
        // Reset game dimensions to default for frontend
        GAME_W   = 128;
        GAME_H   = 128;
        *pyxel_core::width()  = GAME_W;
        *pyxel_core::height() = GAME_H;
        GAME_FPS = 30;
        RETRO_FRAME_COUNT = 0;
        *pyxel_core::frame_count() = 0;
        LR_FRAME_COUNT    = 0;
        audio::PREV_BUTTONS = 0;
        audio::reset_all_button_states();
        reset_color_palette();
        // Stop audio from previous content
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
        // Reset geometry
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   128,
                base_height:  128,
                max_width:    512,
                max_height:   512,
                aspect_ratio: 1.0,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
        return true;
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
        let sibling_sources = read_sibling_py_sources(&script_path);
        if let Some((w, h, fps)) = parse_pyxel_init(&code, &sibling_sources) {
            eprintln!("[lr-pyxel] parsed init: w={w} h={h} fps={fps}");
            GAME_W   = w;
            GAME_H   = h;
            *pyxel_core::width()  = w;
            *pyxel_core::height() = h;
            GAME_FPS = fps;
        } else {
            eprintln!("[lr-pyxel] parse_pyxel_init: not found, using defaults");
        }

        // Update BlipBuf clock rate to match GAME_FPS.
        // Scale the source clock rate so that flip_screen() (called at GAME_FPS)
        // advances the audio at the correct speed.
        // AUDIO_CLOCK_RATE is designed for 60fps; scale it down for slower games.
        if let Some(ref mut blip) = BLIP_BUF {
            let scaled_clock = pyxel_core::AUDIO_CLOCK_RATE as f64
                * (GAME_FPS as f64 / FPS as f64);
            blip.set_rates(scaled_clock, pyxel_core::AUDIO_SAMPLE_RATE as f64);
        }
    }

    // Notify RetroArch of geometry with parsed size
    if let Some(env) = ENVIRON_CB {
        let geometry = rust_libretro_sys::retro_game_geometry {
            base_width:   GAME_W,
            base_height:  GAME_H,
            max_width:    512,
            max_height:   512,
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
        *pyxel_core::frame_count() = 0;
        LR_FRAME_COUNT    = 0;
        audio::PREV_BUTTONS = 0;
        audio::reset_all_button_states();
        reset_color_palette();

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
                show_retroarch_message(
                    "This app is not compatible with lr-pyxel (see log for details)",
                    240,
                );
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
    SPLASH_COUNT = 0;
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
            // LR_FRAME_COUNT is incremented AFTER update()/draw() run (not
            // before), so the very first call sees pyxel.frame_count == 0,
            // matching upstream Pyxel semantics. Incrementing beforehand
            // made the first update() see frame_count == 1, so any script
            // logic keyed on "every N frames starting from frame 0" (e.g.
            // 15_tiled_map_file.py's `if frame_count % 240 == 0`) only
            // fired once N frames had already elapsed, not immediately.
            Python::with_gil(|py| {
                if let Some(ref update) = PY_UPDATE {
                    if let Err(e) = update.call0(py) { e.print(py); }
                }
                if let Some(ref draw) = PY_DRAW {
                    if let Err(e) = draw.call0(py) { e.print(py); }
                }
            });
            LR_FRAME_COUNT += 1;

            pyxel_core::pyxel().flip_screen();
        }

    } else {
        // No game loaded or splash period — show splash screen
        if SPLASH_COUNT < SPLASH_FRAMES {
            SPLASH_COUNT += 1;
            splash::draw();
        } else {
            // Splash finished — launch the frontend now (deferred from
            // retro_load_game()'s content-less boot so the splash actually
            // gets a chance to show first).
            launch_frontend();
        }
    }

    // Always inject input every frame
    audio::inject_input(buttons);

    // Check if frontend requested a content load
    if let Some(path) = PENDING_CONTENT.take() {
        if path.is_empty() {
            // load_content(None) → return to frontend
            launch_frontend();
        } else {
            load_game_from_path(&path);
        }
    }

    // Always submit audio every frame (accumulator handles 367/368 alternation)
    audio::submit_audio_frame();

    // Rebuild the RGB565 palette LUT every frame before submitting.
    // pyxel_core::colors() (the global palette) can change at runtime —
    // e.g. Image.from_image(filename, include_colors=True) clears and
    // rebuilds it with colors from the loaded file. build_palette_lut()
    // was previously only called once at boot, so any color index added
    // after that (e.g. sprite colors beyond the default 16) stayed at
    // its zero-initialized (black) LUT entry forever. This is cheap
    // (256 entries) so redoing it every frame is not a concern.
    video::build_palette_lut();

    // Submit framebuffer to RetroArch every frame to keep display smooth
    video::submit_pyxel_frame();
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Launch the built-in frontend browser.
#[allow(static_mut_refs)]
unsafe fn launch_frontend() {
    const FRONTEND_PY: &str = include_str!("../frontend.py");
    PY_UPDATE = None;
    PY_DRAW   = None;
    GAME_W   = 128;
    GAME_H   = 128;
    *pyxel_core::width()  = GAME_W;
    *pyxel_core::height() = GAME_H;
    GAME_FPS = 30;
    RETRO_FRAME_COUNT = 0;
    *pyxel_core::frame_count() = 0;
    LR_FRAME_COUNT    = 0;
    audio::PREV_BUTTONS = 0;
    audio::reset_all_button_states();
    reset_color_palette();
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
    if let Some(env) = ENVIRON_CB {
        let geometry = rust_libretro_sys::retro_game_geometry {
            base_width:   128,
            base_height:  128,
            max_width:    512,
            max_height:   512,
            aspect_ratio: 1.0,
        };
        env(37, &geometry as *const _ as *mut c_void);
    }
    Python::with_gil(|py| {
        let globals = pyo3::types::PyDict::new_bound(py);
        let _ = globals.set_item("__name__", "__main__");
        match py.run_bound(FRONTEND_PY, Some(&globals), None) {
            Ok(_) => {
                if PY_UPDATE.is_none() {
                    PY_UPDATE = globals.get_item("update").ok()
                        .flatten().map(|f| f.into());
                }
                if PY_DRAW.is_none() {
                    PY_DRAW = globals.get_item("draw").ok()
                        .flatten().map(|f| f.into());
                }
            }
            Err(e) => { e.print(py); }
        }
    });
}

/// Load a game from a file path (called from frontend or PENDING_CONTENT).
#[allow(static_mut_refs)]
unsafe fn load_game_from_path(path: &str) {
    // Reset state
    PY_UPDATE = None;
    PY_DRAW   = None;
    RETRO_FRAME_COUNT = 0;
    *pyxel_core::frame_count() = 0;
    LR_FRAME_COUNT    = 0;
    audio::PREV_BUTTONS = 0;
    audio::reset_all_button_states();
    reset_color_palette();
    // Note: SPLASH_COUNT is NOT reset here; splash only shows on core-less boot

    // Stop audio
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

    // Resolve script path
    let script_path = if path.ends_with(".pyxapp") {
        match extract_pyxapp(path) {
            Some(p) => p,
            None => return,
        }
    } else {
        path.to_string()
    };

    // Static analysis
    if let Ok(code) = std::fs::read_to_string(&script_path) {
        let sibling_sources = read_sibling_py_sources(&script_path);
        if let Some((w, h, fps)) = parse_pyxel_init(&code, &sibling_sources) {
            eprintln!("[lr-pyxel] frontend launch: w={w} h={h} fps={fps}");
            GAME_W   = w;
            GAME_H   = h;
            *pyxel_core::width()  = w;
            *pyxel_core::height() = h;
            GAME_FPS = fps;
        }
    }

    // Notify RetroArch of geometry
    if let Some(env) = ENVIRON_CB {
        let geometry = rust_libretro_sys::retro_game_geometry {
            base_width:   GAME_W,
            base_height:  GAME_H,
            max_width:    512,
            max_height:   512,
            aspect_ratio: GAME_W as f32 / GAME_H as f32,
        };
        env(37, &geometry as *const _ as *mut c_void);
    }

    // Execute the script
    Python::with_gil(|py| {
        // Clear cached modules from the previous game to prevent import
        // conflicts. Previously this only removed math/random/struct
        // (the stub modules); any other same-named module left behind by
        // a prior game (e.g. a common convention like `game`, `scenes`,
        // `entities`, `constants`) would be silently reused from
        // sys.modules instead of being re-imported from the new game's
        // own files — no exception, just wrong code running (or, if the
        // reused module's shape didn't match what the new game expected,
        // it could stall before ever reaching pyxel.run()). This mirrors
        // the thorough cleanup already used in retro_load_game()'s
        // direct-game-load branch.
        if let Ok(sys) = py.import_bound("sys") {
            if let Ok(modules) = sys.getattr("modules") {
                if let Ok(modules_dict) = modules.downcast_into::<pyo3::types::PyDict>() {
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
            // Update sys.path
            if let Ok(path_attr) = sys.getattr("path") {
                if let Ok(syspath) = path_attr.downcast_into::<pyo3::types::PyList>() {
                    let game_dir = std::path::Path::new(&script_path)
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .to_string_lossy()
                        .into_owned();
                    let _ = syspath.insert(0, game_dir.clone());
                    let _ = std::env::set_current_dir(&game_dir);
                }
            }
        }
        if let Ok(code) = std::fs::read_to_string(&script_path) {
            let globals = pyo3::types::PyDict::new_bound(py);
            let _ = globals.set_item("__name__", "__main__");
            match py.run_bound(&code, Some(&globals), None) {
                Ok(_) => {
                    // pyxel.run() may have already set PY_UPDATE/PY_DRAW.
                    // Only fall back to globals if not set.
                    if PY_UPDATE.is_none() {
                        PY_UPDATE = globals.get_item("update").ok()
                            .flatten().map(|f| f.into());
                    }
                    if PY_DRAW.is_none() {
                        PY_DRAW = globals.get_item("draw").ok()
                            .flatten().map(|f| f.into());
                    }
                    // If still not set, use noop
                    if PY_UPDATE.is_none() {
                        let noop = py.eval_bound("lambda: None", None, None).unwrap();
                        PY_UPDATE = Some(noop.clone().into());
                        PY_DRAW   = Some(noop.into());
                    }
                }
                Err(e) => {
                    e.print(py);
                    show_retroarch_message(
                        "This app is not compatible with lr-pyxel (see log for details)",
                        240,
                    );
                }
            }
        }
    });
}

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
