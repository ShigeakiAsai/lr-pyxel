//! resource_wrapper_lr.rs — Resource functions (save/load/screenshot/
//! screencast/reset_screencast/user_data_dir).
//!
//! Tracks upstream's resource_wrapper.rs closely for most functions.

use pyo3::prelude::*;
use crate::*;

// ---------------------------------------------------------------------------
// Resource functions (resource_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (filename, exclude_images=None, exclude_tilemaps=None, exclude_sounds=None, exclude_musics=None, excl_images=None, excl_tilemaps=None, excl_sounds=None, excl_musics=None))]
#[allow(clippy::too_many_arguments)]
pub fn load(
    filename: &str,
    exclude_images: Option<bool>,
    exclude_tilemaps: Option<bool>,
    exclude_sounds: Option<bool>,
    exclude_musics: Option<bool>,
    excl_images: Option<bool>,
    excl_tilemaps: Option<bool>,
    excl_sounds: Option<bool>,
    excl_musics: Option<bool>,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if excl_images.is_some() || excl_tilemaps.is_some() || excl_sounds.is_some() || excl_musics.is_some() {
            warn_deprecated_once("load.excl_*", "excl_* arguments (use exclude_* instead)");
        }
        let ei = excl_images.or(exclude_images);
        let et = excl_tilemaps.or(exclude_tilemaps);
        let es = excl_sounds.or(exclude_sounds);
        let em = excl_musics.or(exclude_musics);
        pyxel_core::pyxel()
            .load_resource(filename, ei, et, es, em)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
#[pyo3(signature = (filename, exclude_images=None, exclude_tilemaps=None, exclude_sounds=None, exclude_musics=None, excl_images=None, excl_tilemaps=None, excl_sounds=None, excl_musics=None))]
#[allow(clippy::too_many_arguments)]
pub fn save(
    filename: &str,
    exclude_images: Option<bool>,
    exclude_tilemaps: Option<bool>,
    exclude_sounds: Option<bool>,
    exclude_musics: Option<bool>,
    excl_images: Option<bool>,
    excl_tilemaps: Option<bool>,
    excl_sounds: Option<bool>,
    excl_musics: Option<bool>,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if excl_images.is_some() || excl_tilemaps.is_some() || excl_sounds.is_some() || excl_musics.is_some() {
            warn_deprecated_once("save.excl_*", "excl_* arguments (use exclude_* instead)");
        }
        let ei = excl_images.or(exclude_images);
        let et = excl_tilemaps.or(exclude_tilemaps);
        let es = excl_sounds.or(exclude_sounds);
        let em = excl_musics.or(exclude_musics);
        pyxel_core::pyxel()
            .save_resource(filename, ei, et, es, em)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
pub fn load_pal(filename: &str) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().load_palette(filename)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
pub fn save_pal(filename: &str) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().save_palette(filename)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
#[pyo3(signature = (filename=None, scale=None))]
pub fn screenshot(filename: Option<&str>, scale: Option<u32>) -> PyResult<()> {
    // filename=None: pyxel-core's own default (join_desktop_path(),
    // resource.rs) resolves via the `directories` crate's UserDirs —
    // the exact same OS-user-directory mechanism user_data_dir()
    // relies on, with the same Lakka limitation (no meaningful "home"/
    // "Desktop" concept to resolve there). Compute a sensible default
    // ourselves instead — see default_capture_filename() below.
    let owned_filename = filename.map(str::to_string)
        .or_else(|| default_capture_filename("screenshot_directory"));
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().save_screenshot(owned_filename.as_deref(), scale)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
#[pyo3(signature = (filename=None, scale=None))]
pub fn screencast(filename: Option<&str>, scale: Option<u32>) -> PyResult<()> {
    // filename=None: same join_desktop_path()/UserDirs caveat and fix
    // as screenshot() above, reading recording_output_directory
    // instead — RetroArch's own separate convention for recorded
    // media vs still screenshots.
    let owned_filename = filename.map(str::to_string)
        .or_else(|| default_capture_filename("recording_output_directory"));
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().save_screencast(owned_filename.as_deref(), scale)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

// Builds a `{dir}/pyxel-{timestamp}` default path for screenshot()/
// screencast() when a script omits filename.
//
// Reads retroarch.cfg fresh on every call (rather than caching a
// value from retro_init()/content-load time) deliberately: RetroArch
// only rewrites this file on an explicit save (menu save, or exit
// with config_save_on_exit), so there's no way to see a change made
// in RetroArch's own menu but not yet saved — but re-reading here
// means a change that *was* saved takes effect on the very next
// screenshot()/screencast() call, not just after the next content
// load or core restart. The file itself is small and this isn't a
// hot per-frame path, so the extra read is cheap.
//
// `cfg_key` is "screenshot_directory" or "recording_output_directory".
// A real path in retroarch.cfg is used directly; an absent/empty/
// literal "default" value falls back to CURRENT_CONTENT_PATH's own
// parent directory, matching RetroArch's real semantics (screenshots/
// recordings land next to the loaded content) rather than a single
// hardcoded guess.
fn default_capture_filename(cfg_key: &str) -> Option<String> {
    let dir = resolve_capture_dir(cfg_key);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = std::fs::create_dir_all(&dir);
    Some(format!("{dir}/pyxel-{timestamp}"))
}

fn resolve_capture_dir(cfg_key: &str) -> String {
    #[cfg(feature = "lakka")]
    {
        const LAKKA_HOME: &str = "/storage";

        // Looks for `key = "value"` in retroarch.cfg's simple
        // line-based format, ignoring commented-out (#) lines.
        // Confirmed on a real device: Lakka's own retroarch.cfg
        // writes these as `"~/screenshots"`/`"~/recordings"` — `~` is
        // a *shell* expansion, not something any Rust filesystem API
        // understands natively, so it's expanded here by hand.
        // Lakka's own home directory is /storage (confirmed:
        // screenshots/recordings/savefiles all exist directly under
        // /storage/ already, exactly matching what `~/screenshots`
        // etc. are supposed to resolve to) — hardcoded rather than
        // read from $HOME, same as ROMS_DIR's own hardcoded
        // /storage/roms/pyxel elsewhere (this is the Lakka-specific
        // branch; a general Linux install falls through to the
        // content-directory fallback below instead).
        // RETROARCH_CFG_PATH is None if retro_init()'s own existence
        // check already found no file there — skips this read
        // entirely rather than repeating a doomed-to-fail filesystem
        // open on every single screenshot()/screencast() call (see
        // RETROARCH_CFG_PATH's own declaration in lib.rs).
        if let Some(cfg_path) = unsafe { (*std::ptr::addr_of!(RETROARCH_CFG_PATH)).clone() } {
            if let Ok(contents) = std::fs::read_to_string(&cfg_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with('#') { continue; }
                    let Some((k, v)) = line.split_once('=') else { continue; };
                    if k.trim() != cfg_key { continue; }
                    let v = v.trim().trim_matches('"');
                    if v.is_empty() || v == "default" { break; }
                    if let Some(rest) = v.strip_prefix("~/") {
                        return format!("{LAKKA_HOME}/{rest}");
                    } else if v == "~" {
                        return LAKKA_HOME.to_string();
                    }
                    return v.to_string();
                }
            }
        }
    }
    #[cfg(not(feature = "lakka"))]
    { let _ = cfg_key; }

    // "default" (RetroArch resolves this to the loaded content's own
    // directory), the key was absent, or the file couldn't be read —
    // fall back to CURRENT_CONTENT_PATH's own parent directory,
    // matching RetroArch's real semantics rather than a single
    // hardcoded guess. Non-Lakka builds always land here, same as a
    // desktop Pyxel game saving screenshots next to the script.
    unsafe {
        (*std::ptr::addr_of!(CURRENT_CONTENT_PATH)).clone()
            .as_deref()
            .map(std::path::Path::new)
            .and_then(std::path::Path::parent)
            .map(|p| p.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ".".to_string())
    }
}

#[pyfunction]
pub fn reset_screencast() {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().reset_screencast(); } }
}

#[pyfunction]
pub fn user_data_dir(vendor_name: &str, app_name: &str) -> PyResult<String> {
    unsafe {
        if !PYXEL_READY { return Ok(String::new()); }
        pyxel_core::pyxel().user_data_dir(vendor_name, app_name)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

