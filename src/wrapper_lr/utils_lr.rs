//! utils_lr.rs — Shared infrastructure for lr-pyxel's Python wrapper
//! layer, split out of what used to be one large wrappers.rs.
//!
//! Everything here is lr-pyxel's own original implementation, not a
//! port of any single upstream pyxel-binding file — upstream doesn't
//! have a single-file analog for the deprecation-warning mechanism or
//! the live-list macro below (its own utils.rs covers different
//! ground: cast_pyany!, validate_index!/invalid_index_error! with a
//! different implementation, and PyO3-version-specific error helpers).
//! The macros here are #[macro_export]'d so every other *_wrapper_lr.rs
//! file picks them up automatically via its existing `use crate::*;`,
//! with no additional per-file import needed.

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Deprecation warnings
// ---------------------------------------------------------------------------
// Upstream Pyxel prints a message containing "deprecated" to stdout the
// FIRST time a deprecated API is used, and never again for that same
// item in the same process (upstream's own tests rely on exactly this:
// "warning fires only once per session, so test both APIs in order").
// Tracked as a plain global HashSet of string keys, one entry per
// distinct deprecated item — not reset on content switch, since
// upstream's "session" scope is the whole process lifetime, not a
// single script run.
static mut WARNED_DEPRECATIONS: Option<std::collections::HashSet<&'static str>> = None;

pub fn warn_deprecated_once(key: &'static str, message: &str) {
    unsafe {
        let ptr = std::ptr::addr_of_mut!(WARNED_DEPRECATIONS);
        let set = (*ptr).get_or_insert_with(std::collections::HashSet::new);
        if set.insert(key) {
            // Route through Python's own print() (writing to sys.stdout)
            // rather than Rust's println! (which writes directly to the
            // OS-level stdout file descriptor, bypassing Python's stdio
            // layer entirely). Found via a test harness built to run
            // upstream Pyxel's own pytest suite inside lr-pyxel: a
            // Python-level capfd-style stdout redirection
            // (contextlib.redirect_stdout) never saw these warnings at
            // all, even though they showed up fine in journalctl (which
            // captures the raw process stdout) — println!'s output
            // never passed through sys.stdout for Python-level
            // redirection to intercept in the first place.
            //
            // `message` is printed verbatim (no added wrapper text) —
            // upstream's own deprecation_warning! macro is just
            // `println!($msg)`, and upstream's own test suite asserts
            // on the captured stdout matching that exact literal
            // string (e.g. "Tone.noise is deprecated. Use Tone.mode
            // instead.\n"), so every call site below passes the exact
            // upstream wording, not a paraphrase.
            pyo3::Python::attach(|py| {
                if let Ok(builtins) = py.import("builtins") {
                    if let Ok(print_fn) = builtins.getattr("print") {
                        let _ = print_fn.call1((message,));
                    }
                }
            });
        }
    }
}

// Mirrors upstream's own validate_index!/invalid_index_error! macros
// (crates/pyxel-binding/src/utils.rs) — same wording ("$parameter must
// be a valid $resource index"), same bounds check. Several places in
// this file indexed pyxel_core::images()/tilemaps()/channels()/
// sounds()/musics() directly with a caller-supplied index and no
// bounds check at all — an out-of-range index (e.g.
// pyxel.blt(0, 0, 999, ...)) then panicked on the raw Vec index
// instead of raising a Python exception. A Rust panic raised from
// PyO3-called code crosses the retro_run() FFI boundary and aborts
// the whole process rather than being catchable, so every one of
// these needs the same guard upstream already has.
#[macro_export]
macro_rules! validate_index {
    ($index:expr, $len:expr, $parameter:literal, $resource:literal) => {
        if ($index as usize) >= $len {
            return Err(pyo3::exceptions::PyValueError::new_err(
                concat!($parameter, " must be a valid ", $resource, " index")
            ));
        }
    };
    ($index:expr, $len:expr, $parameter:literal, $resource:literal, list) => {
        if ($index as usize) >= $len {
            return Err(pyo3::exceptions::PyValueError::new_err(
                concat!($parameter, " must contain only valid ", $resource, " indices")
            ));
        }
    };
}

// ---------------------------------------------------------------------------
// Live list views (fixes "getter returns a copy" for Sound.notes/tones/
// volumes/effects and Tone.wavetable)
// ---------------------------------------------------------------------------
// Previously these getters returned a plain Vec<T> — a snapshot copy.
// `snd.notes.append(60)` silently mutated that throwaway copy and never
// touched the real underlying field, since Python has no way to write
// a mutation back through a plain returned list. Each of the 5 fields
// below needs the same fix: a dedicated wrapper class holding a
// reference back to the parent Sound/Tone, implementing the list
// protocol by reading/writing the real field directly on every call.
// One macro generates all 5, parameterized by wrapper name, parent Rc
// type, element type (must be Copy — all 5 are: i8/u8/u32), and field
// name.
#[macro_export]
macro_rules! define_live_list {
    ($wrapper:ident, $py_name:literal, $parent_rc:ty, $elem:ty, $field:ident) => {
        #[pyclass(name = $py_name, unsendable)]
        pub struct $wrapper {
            parent: $parent_rc,
        }

        #[pymethods]
        impl $wrapper {
            pub fn __len__(&self) -> usize {
                {
                    self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner).$field.len()
                }
            }

            pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
                {
                    let guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let v = &guard.$field;
                    let len = v.len() as i64;
                    if let Ok(i) = idx.extract::<i64>() {
                        let i = if i < 0 { i + len } else { i };
                        if i < 0 || i >= len {
                            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                        }
                        return Ok(v[i as usize].into_pyobject(py)?.into_any().unbind());
                    }
                    if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                        let indices = slice.indices(len as isize)?;
                        let mut result = Vec::new();
                        let mut i = indices.start;
                        if indices.step > 0 {
                            while i < indices.stop {
                                result.push(v[i as usize]);
                                i += indices.step;
                            }
                        } else if indices.step < 0 {
                            while i > indices.stop {
                                result.push(v[i as usize]);
                                i += indices.step;
                            }
                        }
                        return Ok(result.into_pyobject(py)?.into_any().unbind());
                    }
                    Err(pyo3::exceptions::PyTypeError::new_err("index must be an int or slice"))
                }
            }

            pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
                {
                    let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let v = &mut guard.$field;
                    let len = v.len() as i64;
                    if let Ok(i) = idx.extract::<i64>() {
                        let i = if i < 0 { i + len } else { i };
                        if i < 0 || i >= len {
                            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                        }
                        v[i as usize] = val.extract::<$elem>()?;
                        return Ok(());
                    }
                    if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                        // Slice assignment (e.g. tone.wavetable[:] = [...]),
                        // confirmed needed by upstream's own
                        // 14_synthesizer.py example — this whole macro
                        // (Sound.notes/tones/volumes/effects,
                        // Tone.wavetable) was int-only before, discovered
                        // when that official example hit exactly this
                        // TypeError. Same indices()+splice() pattern as
                        // the top-level bank lists (colors etc.).
                        let indices = slice.indices(len as isize)?;
                        if indices.step != 1 {
                            return Err(pyo3::exceptions::PyValueError::new_err(
                                "extended slices (step != 1) are not supported for assignment"
                            ));
                        }
                        let replacement = val.extract::<Vec<$elem>>()?;
                        let start = indices.start.max(0) as usize;
                        let stop = indices.stop.max(indices.start) as usize;
                        v.splice(start..stop, replacement);
                        return Ok(());
                    }
                    Err(pyo3::exceptions::PyTypeError::new_err("index must be an int or slice"))
                }
            }

            pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
                {
                    let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let v = &mut guard.$field;
                    let len = v.len() as i64;
                    if let Ok(i) = idx.extract::<i64>() {
                        let i = if i < 0 { i + len } else { i };
                        if i < 0 || i >= len {
                            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
                        }
                        v.remove(i as usize);
                        return Ok(());
                    }
                    if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                        let indices = slice.indices(len as isize)?;
                        if indices.step != 1 {
                            return Err(pyo3::exceptions::PyValueError::new_err(
                                "extended slices (step != 1) are not supported for deletion"
                            ));
                        }
                        let start = indices.start.max(0) as usize;
                        let stop = indices.stop.max(indices.start) as usize;
                        v.drain(start..stop);
                        return Ok(());
                    }
                    Err(pyo3::exceptions::PyTypeError::new_err("index must be an int or slice"))
                }
            }

            pub fn append(&self, val: $elem) {
                { self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner).$field.push(val); }
            }

            pub fn insert(&self, idx: usize, val: $elem) {
                {
                    let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let v = &mut guard.$field;
                    let idx = idx.min(v.len());
                    v.insert(idx, val);
                }
            }

            #[pyo3(signature = (idx=None))]
            pub fn pop(&self, idx: Option<i64>) -> PyResult<$elem> {
                {
                    let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let v = &mut guard.$field;
                    let len = v.len() as i64;
                    if len == 0 {
                        return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty list"));
                    }
                    let i = idx.unwrap_or(-1);
                    let i = if i < 0 { i + len } else { i };
                    if i < 0 || i >= len {
                        return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
                    }
                    Ok(v.remove(i as usize))
                }
            }

            pub fn extend(&self, vals: Vec<$elem>) {
                { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.extend(vals); }
            }

            pub fn clear(&self) {
                { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.clear(); }
            }

            pub fn __repr__(&self) -> String {
                { format!("{:?}", (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field) }
            }

            pub fn __bool__(&self) -> bool {
                { !(&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.is_empty() }
            }

            pub fn __reversed__(&self) -> Vec<$elem> {
                { (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.iter().rev().copied().collect() }
            }

            pub fn __iadd__(&self, vals: Vec<$elem>) {
                { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.extend(vals); }
            }

            pub fn __eq__(&self, other: Vec<$elem>) -> bool {
                { (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field == other }
            }

            pub fn to_list(&self) -> Vec<$elem> {
                { (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).$field.clone() }
            }
        }
    };
}

