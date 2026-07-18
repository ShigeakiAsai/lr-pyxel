//! system_wrapper_lr.rs — System functions (init/run/show/flip/quit/
//! title/icon/fps/quit_key/perf_monitor/integer_scale/screen_mode/
//! load_content, and window/display settings that are no-ops in
//! headless libretro mode).
//!
//! Substantial rework versus upstream's system_wrapper.rs: upstream
//! drives its own window + event loop, while lr-pyxel is driven
//! externally by RetroArch's retro_run() and has no real window to
//! manage (window/display settings below are no-ops), plus lr-pyxel-
//! specific additions (show()'s no-op update/draw caching so
//! retro_run() keeps displaying the last frame; flip() intentionally
//! raising instead of no-op'ing, since libretro's framing model can't
//! support manual frame advancement; load_content() for the frontend
//! browser's file picker).

use pyo3::prelude::*;
use crate::*;

// -- system ------------------------------------------------------------------


// init() previously only updated GAME_W/GAME_H bookkeeping, the
// Python-visible pyxel.width/height module attributes, and notified
// RetroArch via SET_GEOMETRY — but never actually resized the physical
// canvas (missing the pyxel_core::pyxel().set_screen_size() call). This
// meant the game's REAL init() call (with its actual runtime-computed
// w/h — which may differ from parse_pyxel_init()'s static pre-parse
// guess, e.g. when the real value depends on a conditional expression
// or other logic the static parser can't evaluate) correctly told
// RetroArch "expect WxH", but the underlying video stream stayed
// capped at whatever size the pre-parse guessed, silently truncating
// anything beyond that. Found via finardry.pyxapp:
// `height = 256 if MODE_SQUARE else 240; px.init(256, height, ...)` —
// the static parser can't evaluate the conditional, falls back to the
// default 128, and the real init() call's correct height (240) was
// never propagated to the actual canvas.
#[pyfunction]
#[pyo3(signature = (width, height, title=None, caption=None, fps=None, quit_key=None,
                    display_scale=None, capture_scale=None,
                    capture_sec=None))]
#[allow(clippy::too_many_arguments)]
pub fn init(
    width: u32, height: u32,
    title: Option<&str>, caption: Option<&str>, fps: Option<u32>, quit_key: Option<u32>,
    display_scale: Option<u32>, capture_scale: Option<u32>, capture_sec: Option<u32>,
) -> PyResult<()> {
    // caption predates upstream's rename to title in an early Pyxel
    // version — some older scripts (e.g. this exact NyanCat sample)
    // still call init(..., caption="...") rather than title=. Found
    // via the SAME class of bug as w/h below: init()'s parameter names
    // must match upstream's documented ones exactly for keyword-
    // argument calls (pyxel.init(width=160, ...)) to work at all —
    // PyO3 matches keyword arguments against the Rust parameter names
    // themselves, not just position.
    if caption.is_some() {
        warn_deprecated_once("init.caption", "init()'s caption argument (use title instead)");
    }
    let title = title.or(caption);
    let _ = (title, quit_key, display_scale, capture_scale, capture_sec);
    unsafe {
        // Save game-requested size and FPS
        GAME_W = width.max(1);
        GAME_H = height.max(1);
        GAME_FPS = fps.unwrap_or(30).clamp(1, 60);

        // Actually resize the physical canvas to match — this is the
        // authoritative source of truth (the script's real runtime
        // values), superseding whatever the pre-execution static parse
        // guessed. Also updates pyxel_core::width()/height().
        if PYXEL_READY {
            pyxel_core::pyxel().set_screen_size(GAME_W, GAME_H)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        }

        // Update pyxel.width/height module attributes to reflect game size
        Python::attach(|py| {
            if let Ok(m) = py.import("pyxel") {
                let _ = m.setattr("width",  GAME_W);
                let _ = m.setattr("height", GAME_H);
            }
        });

        // Notify RetroArch of the game's actual screen geometry.
        // RETRO_ENVIRONMENT_SET_GEOMETRY (37) lets us change width/height
        // after init without restarting the core.
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   GAME_W,
                base_height:  GAME_H,
                max_width:    1024,
                max_height:   1024,
                aspect_ratio: GAME_W as f32 / GAME_H as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
    }
    Ok(())
}

// run(update, draw) — caches the callbacks for the libretro frame loop.
// In normal Pyxel this starts the event loop; here it is the hook that
// lets class-based games (e.g. Game() → pyxel.run(self.update, self.draw))
// register their callbacks with the core.
#[pyfunction]
pub fn run(update: Py<PyAny>, draw: Py<PyAny>) {
    unsafe {
        PY_UPDATE = Some(update);
        PY_DRAW   = Some(draw);
    }
}


// ---------------------------------------------------------------------------
// System functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn quit() {
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
pub fn show() {
    unsafe {
        if !PYXEL_READY { return; }
        Python::attach(|py| {
            // Create no-op lambda and cache as update/draw
            let noop = py.eval(c"lambda: None", None, None).unwrap();
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
// Unsupported in libretro (framing is driven by retro_run()); raises
// instead of no-op'ing so flip()-based main loops fail fast (see below).
#[pyfunction]
pub fn flip() -> PyResult<()> {
    // Previously a silent no-op. Scripts using the flip()-based main loop
    // pattern (`while True: ... pyxel.flip()`, e.g. 99_flip_animation.py)
    // never call back into Rust between flip() calls, so with flip() doing
    // nothing the loop never terminates — it spins forever inside the
    // single py.run() call that runs the script, permanently
    // hanging retro_run() (RetroArch itself freezes, no crash, no error).
    // Raising here instead lets the loop's first flip() call unwind back
    // out with a clear, actionable message instead of a silent hang.
    Err(pyo3::exceptions::PyRuntimeError::new_err(
        "pyxel.flip() is not supported in lr-pyxel (libretro build). \
         Games driven by a `while True: ... pyxel.flip()` main loop can't \
         run under libretro's frame-driven retro_run() model — only \
         pyxel.run(update, draw) is supported here."
    ))
}

// system_wrapper.rs additions
// Window/display settings are no-ops in headless libretro mode

#[pyfunction]
pub fn reset() {
    // In libretro, reset = reload current content
    // For now this is a no-op; future: trigger RETRO_ENVIRONMENT_RESET
}

/// Load a content file from the frontend browser.
/// Called by frontend.py when the user selects a file.
/// Pass None or empty string to return to the frontend.
#[pyfunction]
#[pyo3(signature = (path=None))]
pub fn load_content(path: Option<&str>) -> PyResult<()> {
    unsafe {
        crate::PENDING_CONTENT = Some(path.unwrap_or("").to_string());
    }
    Ok(())
}

#[pyfunction]
pub fn title(_title: &str) {
    // no-op in headless mode
}

#[pyfunction]
#[pyo3(signature = (data, scale, colkey=None))]
pub fn icon(data: pyo3::Bound<'_, pyo3::PyAny>, scale: u32, colkey: Option<u8>) -> PyResult<()> {
    // Same manual-validation pattern as Image.set()/Tilemap.set()/
    // set_dropped_files() — wrong-type `data` should still raise with
    // upstream's exact wording, even though this function is itself a
    // no-op in headless mode.
    let items: Vec<String> = data.extract().map_err(|_| {
        let type_name = data.get_type().name()
            .map(|n| n.to_string())
            .unwrap_or_else(|_| "object".to_string());
        pyo3::exceptions::PyTypeError::new_err(format!(
            "'{type_name}' object is not an instance of 'Sequence'"
        ))
    })?;
    // Previously this discarded `items`/`scale`/`colkey` and always
    // succeeded — content validation (empty rows, bad hex digits,
    // scale=0, dimension overflow, out-of-palette colors) never ran
    // at all. pyxel_core::Pyxel::set_icon() already does all of this
    // validation itself, then early-returns Ok(()) once past it if
    // is_headless() — i.e. it's already exactly the "validate for
    // real, then no-op the actual OS window icon" behavior this
    // function needs, so delegate to it directly rather than
    // reimplementing the checks here (matches upstream's own binding,
    // which does the same one-line delegation).
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().set_icon(&items, scale, colkey)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        }
    }
    Ok(())
}

#[pyfunction]
pub fn perf_monitor(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn integer_scale(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn screen_mode(_scr: u32) {
    // no-op in headless mode
}

#[pyfunction]
pub fn fullscreen(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn resize(width: u32, height: u32) -> PyResult<()> {
    if width == 0 || height == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "width and height must be greater than 0",
        ));
    }
    unsafe {
        GAME_W = width;
        GAME_H = height;
        // Actually resize the physical canvas — previously this only
        // updated our own GAME_W/GAME_H tracking and RetroArch's
        // reported geometry, leaving the real screen canvas at
        // whatever size it was before. Same class of bug as init()'s
        // missing set_screen_size() call (v0.11.3), just in a
        // different function we hadn't touched with that fix. Any
        // script calling pyxel.resize() at runtime would have its
        // rendering silently truncated to the old size.
        if PYXEL_READY {
            pyxel_core::pyxel().set_screen_size(width, height)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        }
        // pyxel.width/height are frozen as static module attributes by
        // init() (see there for why) — once set, a static attribute
        // takes precedence over __getattr__ permanently, so without
        // this, pyxel.width/height would report the size at launch
        // forever, never reflecting a runtime resize() call, even
        // though pyxel_core's own width()/height() (and everything
        // reading them internally) update correctly.
        Python::attach(|py| {
            if let Ok(m) = py.import("pyxel") {
                let _ = m.setattr("width",  width);
                let _ = m.setattr("height", height);
            }
        });
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   width,
                base_height:  height,
                // Was hardcoded to 256, stale since v0.11.3 raised the
                // actual ceiling (SCREEN_W/SCREEN_H and every other
                // max_width/max_height site) to 1024.
                max_width:    1024,
                max_height:   1024,
                aspect_ratio: width as f32 / height as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
    }
    Ok(())
}

