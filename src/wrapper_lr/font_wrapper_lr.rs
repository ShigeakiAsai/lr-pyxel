//! font_wrapper_lr.rs — Font wrapper (pyxel.Font).
//!
//! Tracks upstream's font_wrapper.rs closely; no significant lr-pyxel-
//! specific behavior beyond the unsendable/thread-safety note below.

use pyo3::prelude::*;

#[pyclass(name = "Font", unsendable)]
pub struct PyFont {
    inner: pyxel_core::RcFont,
}

// RcFont is Rc<UnsafeCell<Font>>, neither Send nor Sync — matches
// every other Rc-backed wrapper in this file (Image/Tilemap/etc).
// `unsendable` (rather than the old `unsafe impl Send for PyFont {}`)
// tells PyO3 to panic at runtime if this is ever accessed from a
// different thread than it was created on, instead of asserting a
// guarantee this type doesn't actually uphold. Since PyO3 0.23,
// #[pyclass] additionally requires Sync (for free-threaded Python
// support) unless marked unsendable — Rc<UnsafeCell<_>> can't satisfy
// that either way, so unsendable is the only option here, same as it
// always should have been.

#[pymethods]
impl PyFont {
    #[new]
    #[pyo3(signature = (filename, font_size=None))]
    pub fn new(filename: &str, font_size: Option<f32>) -> PyResult<Self> {
        pyxel_core::Font::new(filename, font_size)
            .map(|inner| PyFont { inner })
            .map_err(pyo3::exceptions::PyException::new_err)
    }

    pub fn text_width(&self, s: &str) -> i32 {
        unsafe { (&mut *self.inner.as_ptr()).text_width(s) }
    }
}

impl PyFont {
    // Accessor for other modules (e.g. graphics_wrapper_lr's text()),
    // matching every other pyclass's .rc() convention in this codebase
    // — `inner` itself stays private since PyO3 exposes struct fields
    // to Python only via explicit #[getter]s, not by field visibility.
    pub fn rc(&self) -> &pyxel_core::RcFont {
        &self.inner
    }
}
