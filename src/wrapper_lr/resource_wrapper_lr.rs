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
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().save_screenshot(filename, scale)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyfunction]
#[pyo3(signature = (filename=None, scale=None))]
pub fn screencast(filename: Option<&str>, scale: Option<u32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        pyxel_core::pyxel().save_screencast(filename, scale)
            .map_err(pyo3::exceptions::PyException::new_err)
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

