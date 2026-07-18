//! variable_wrapper_lr.rs — Module-level __getattr__ for dynamic
//! variables (width/height/frame_count/mouse_x/mouse_y/mouse_wheel/
//! colors/screen/cursor/font/images/tilemaps/sounds/musics/tones/
//! channels).
//!
//! Mirrors upstream's variable_wrapper.rs __getattr__ approach:
//! variables that change every frame (frame_count, mouse_x/y, etc.)
//! are returned dynamically instead of being set once at module init
//! time.

use pyo3::prelude::*;
use crate::*;

// PyColors — live view onto pyxel_core::colors() (the palette).
// Upstream has no dedicated colors_wrapper.rs file either — pyxel.colors
// is generated there via a shared generic bank-list macro referenced
// from this same variable_wrapper.rs, not a hand-written class. Kept
// here (rather than its own colors_wrapper_lr.rs) to mirror that:
// this is the only place upstream ever mentions Colors at all.

// ---------------------------------------------------------------------------
// Colors wrapper — live view onto pyxel_core::colors() (the palette)
// ---------------------------------------------------------------------------
// Previously `pyxel.colors` (via __getattr__) returned a brand new PyList
// copied from pyxel_core::colors() on every access. That meant writes
// like `pyxel.colors[:] = PALETTE` or `pyxel.colors[i] = 0x123456`
// mutated only that disposable copy and were silently lost — the actual
// global palette was never updated. This class instead reads/writes
// pyxel_core::colors() directly, matching the existing ChannelList
// pattern for single-index vs. full-slice assignment.
#[pyclass(name = "Colors")]
pub struct PyColors;

#[pymethods]
impl PyColors {
    pub fn __len__(&self) -> usize {
        pyxel_core::colors().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let colors = pyxel_core::colors();
        let len = colors.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("list index out of range"));
            }
            return Ok(colors[i as usize].into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(colors[i as usize]);
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(colors[i as usize]);
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("colors index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(idx_i64) = idx.extract::<i64>() {
            // Single index assignment: colors[i] = 0xRRGGBB
            let v = val.extract::<u32>()?;
            let mut colors = pyxel_core::colors();
            let len = colors.len() as i64;
            let i = if idx_i64 < 0 { idx_i64 + len } else { idx_i64 };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("list index out of range"));
            }
            colors[i as usize] = v;
        } else if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            // Slice assignment: colors[a:b] = [...]. Previously this
            // branch just did `*pyxel_core::colors() = items`,
            // replacing the ENTIRE list regardless of the slice's
            // actual start/stop — colors[0:2] = [x, y] happened to
            // look correct (colors[0]/[1] matched) but silently wiped
            // out every other entry, and colors[2:0] = [x] (an
            // empty-range "insert") produced a 1-element list instead
            // of inserting. Now uses the same indices()+splice()
            // pattern as Music.seqs, matching Python's own list
            // slice-assignment semantics (can change length; an empty
            // range inserts rather than replaces — confirmed via
            // upstream's own
            // test_reversed_step_one_slice_assignment_inserts).
            let mut colors_ref = pyxel_core::colors();
            let len = colors_ref.len() as i64;
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for colors assignment"
                ));
            }
            let replacement = val.extract::<Vec<u32>>()?;
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            colors_ref.splice(start..stop, replacement);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err("colors index must be an int or slice"));
        }
        Ok(())
    }

    // List-like growth methods. pyxel.colors is documented as a plain
    // list of the palette's colors, so scripts reasonably treat it like
    // one (e.g. growing it to 256 entries after loading a shorter
    // .pyxpal file with pyxel.colors.append(0)).
    pub fn append(&self, val: u32) {
        pyxel_core::colors().push(val);
    }

    pub fn insert(&self, idx: usize, val: u32) {
        let mut colors = pyxel_core::colors();
        let idx = idx.min(colors.len());
        colors.insert(idx, val);
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut colors = pyxel_core::colors();
        let len = colors.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("list index out of range"));
            }
            colors.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for colors deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            colors.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("colors index must be an int or slice"))
    }

    pub fn __repr__(&self) -> String {
        format!("Colors{:?}", pyxel_core::colors())
    }

    pub fn __eq__(&self, other: pyo3::Bound<'_, pyo3::PyAny>) -> bool {
        if let Ok(other_vec) = other.extract::<Vec<u32>>() {
            *pyxel_core::colors() == other_vec
        } else {
            false
        }
    }

    pub fn __add__(&self, other: Vec<u32>) -> Vec<u32> {
        let mut result = pyxel_core::colors().clone();
        result.extend(other);
        result
    }

    pub fn __mul__(&self, n: usize) -> Vec<u32> {
        let colors = pyxel_core::colors();
        let mut result = Vec::with_capacity(colors.len() * n);
        for _ in 0..n {
            result.extend(colors.iter().copied());
        }
        result
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::colors().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<u32> {
        pyxel_core::colors().iter().rev().copied().collect()
    }

    pub fn __iadd__(&self, vals: Vec<u32>) {
        pyxel_core::colors().extend(vals);
    }

    pub fn extend(&self, vals: Vec<u32>) {
        pyxel_core::colors().extend(vals);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<u32> {
        let mut colors = pyxel_core::colors();
        let len = colors.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty colors list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(colors.remove(i as usize))
    }

    pub fn clear(&self) {
        pyxel_core::colors().clear();
    }

    // Alternate spelling some scripts use instead of colors[:] = [...]
    // for a full bulk replace (e.g. the Mandelbrot palette-extension
    // example, and dungeon-antiqua2.pyxapp's config loader). Same
    // semantics as the slice-assignment branch of __setitem__.
    pub fn from_list(&self, items: Vec<u32>) {
        *pyxel_core::colors() = items;
    }

    pub fn to_list(&self) -> Vec<u32> {
        pyxel_core::colors().clone()
    }
}


// ---------------------------------------------------------------------------
// Module-level __getattr__ for dynamic variables (variable_wrapper.rs)
// ---------------------------------------------------------------------------
// This mirrors pyxel-binding's variable_wrapper.rs __getattr__ approach:
// variables that change every frame (frame_count, mouse_x/y, etc.) are
// returned dynamically instead of being set once at module init time.

#[pyfunction]
pub fn __getattr__(py: Python, name: &str) -> PyResult<Py<PyAny>> {
    let value: Py<PyAny> = match name {
        // System
        "width"       => (*pyxel_core::width()).into_pyobject(py)?.into_any().unbind(),
        "height"      => (*pyxel_core::height()).into_pyobject(py)?.into_any().unbind(),
        "frame_count" => unsafe { LR_FRAME_COUNT }.into_pyobject(py)?.into_any().unbind(),
        // Input
        "mouse_x"     => (*pyxel_core::mouse_x()).into_pyobject(py)?.into_any().unbind(),
        "mouse_y"     => (*pyxel_core::mouse_y()).into_pyobject(py)?.into_any().unbind(),
        "mouse_wheel" => (*pyxel_core::mouse_wheel()).into_pyobject(py)?.into_any().unbind(),
        // Graphics
        "colors"   => PyColors.into_pyobject(py)?.into_any().unbind(),
        "screen"   => PyImage::from_rc(pyxel_core::screen().clone()).into_pyobject(py)?.into_any().unbind(),
        // Missing entirely until now (pyxel.screen was added in
        // v0.11.2, but these two built-in image banks were
        // overlooked) — the mouse cursor sprite and the built-in
        // font glyph atlas, both exposed upstream as plain Image
        // instances alongside pyxel.screen.
        "cursor"   => PyImage::from_rc(pyxel_core::cursor_image().clone()).into_pyobject(py)?.into_any().unbind(),
        "font"     => PyImage::from_rc(pyxel_core::font_image().clone()).into_pyobject(py)?.into_any().unbind(),
        "images"   => PyImageList.into_pyobject(py)?.into_any().unbind(),
        "tilemaps" => PyTilemapList.into_pyobject(py)?.into_any().unbind(),
        // Audio
        "sounds"   => PySoundList.into_pyobject(py)?.into_any().unbind(),
        "musics"   => PyMusicList.into_pyobject(py)?.into_any().unbind(),
        "tones"    => PyToneList.into_pyobject(py)?.into_any().unbind(),
        "channels" => PyChannelList.into_pyobject(py)?.into_any().unbind(),
        _ => return Err(pyo3::exceptions::PyAttributeError::new_err(
            format!("module 'pyxel' has no attribute '{name}'")
        )),
    };
    Ok(value)
}

