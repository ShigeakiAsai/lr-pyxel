//! music_wrapper_lr.rs — Music wrapper (pyxel.Music, pyxel.musics[n]),
//! plus Music.seqs (PyMusicSeq/PyMusicSeqs — the per-channel sequence
//! lists).
//!
//! Tracks upstream's music_wrapper.rs closely. Music.seqs needs
//! richer support than the other Seq-like wrappers: upstream's own
//! tests exercise real slice access/assignment on the OUTER list
//! (e.g. msc.seqs[2:0] = [[7]] to insert a channel), which is
//! deliberately left unsupported elsewhere (colors/channels/etc. —
//! see the v1.0 known-limitations note) but which Music.seqs
//! specifically requires.

use pyo3::prelude::*;
use crate::*;

// ---------------------------------------------------------------------------
// Music bank wrapper (pyxel.musics[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Music.seqs — full "Seqs" protocol (nested list: one Vec<u32> per channel)
// ---------------------------------------------------------------------------
// Music.seqs needs richer support than the other Seq-like wrappers:
// upstream's own tests exercise real slice access/assignment on the
// OUTER list (e.g. msc.seqs[2:0] = [[7]] to insert a channel), which we
// deliberately left unsupported elsewhere (colors/channels/etc. — see
// the v1.0 known-limitations note) but which Music.seqs specifically
// requires. Two classes: PyMusicSeq (one channel's Vec<u32>, live) and
// PyMusicSeqs (the outer list of channels, live).

// -- Inner: one channel's sequence --
#[pyclass(name = "MusicSeq", unsendable)]
pub struct PyMusicSeq {
    parent: pyxel_core::RcMusic,
    channel: usize,
}

#[pymethods]
impl PyMusicSeq {
    pub fn __len__(&self) -> usize {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].len()
    }

    pub fn __getitem__(&self, idx: i64) -> PyResult<u32> {
        {
            let guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let v = &guard.seqs[self.channel];
            let len = v.len() as i64;
            let i = if idx < 0 { idx + len } else { idx };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
            }
            Ok(v[i as usize])
        }
    }

    pub fn __setitem__(&self, idx: i64, val: u32) -> PyResult<()> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let v = &mut guard.seqs[self.channel];
            let len = v.len() as i64;
            let i = if idx < 0 { idx + len } else { idx };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
            }
            v[i as usize] = val;
            Ok(())
        }
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let v = &mut guard.seqs[self.channel];
            let len = v.len() as i64;
            let i = if idx < 0 { idx + len } else { idx };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
            }
            v.remove(i as usize);
            Ok(())
        }
    }

    pub fn append(&self, val: u32) {
        { self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner).seqs[self.channel].push(val); }
    }

    pub fn insert(&self, idx: usize, val: u32) {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let v = &mut guard.seqs[self.channel];
            let idx = idx.min(v.len());
            v.insert(idx, val);
        }
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<u32> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let v = &mut guard.seqs[self.channel];
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

    pub fn extend(&self, vals: Vec<u32>) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].extend(vals); }
    }

    pub fn clear(&self) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].clear(); }
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel])
    }

    pub fn __bool__(&self) -> bool {
        !(&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].is_empty()
    }

    pub fn __reversed__(&self) -> Vec<u32> {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].iter().rev().copied().collect()
    }

    pub fn __iadd__(&self, vals: Vec<u32>) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].extend(vals); }
    }

    pub fn __eq__(&self, other: Vec<u32>) -> bool {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel] == other
    }

    pub fn to_list(&self) -> Vec<u32> {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[self.channel].clone()
    }
}

// -- Outer: the list of channels --
#[pyclass(name = "MusicSeqs", unsendable)]
pub struct PyMusicSeqs {
    parent: pyxel_core::RcMusic,
}

#[pymethods]
impl PyMusicSeqs {
    pub fn __len__(&self) -> usize {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.len()
    }

    // int -> a live PyMusicSeq view; slice -> a plain nested list
    // (matching upstream: msc.seqs[0:2] == [[0], [1]], not wrapper
    // objects — slicing reads a snapshot, same as any Python list
    // slice).
    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        {
            let len = (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.len();
            if let Ok(i) = idx.extract::<i64>() {
                let l = len as i64;
                let i = if i < 0 { i + l } else { i };
                if i < 0 || i >= l {
                    return Err(pyo3::exceptions::PyIndexError::new_err("music channel index out of range"));
                }
                return Ok(PyMusicSeq { parent: self.parent.clone(), channel: i as usize }.into_pyobject(py)?.into_any().unbind());
            }
            if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                let indices = slice.indices(len as isize)?;
                let mut result = Vec::new();
                let mut i = indices.start;
                if indices.step > 0 {
                    while i < indices.stop {
                        result.push((&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[i as usize].clone());
                        i += indices.step;
                    }
                } else if indices.step < 0 {
                    while i > indices.stop {
                        result.push((&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs[i as usize].clone());
                        i += indices.step;
                    }
                }
                return Ok(result.into_pyobject(py)?.into_any().unbind());
            }
            Err(pyo3::exceptions::PyTypeError::new_err("music channel index must be an int or slice"))
        }
    }

    // int -> replace one channel's whole sequence; slice (step=1 only)
    // -> splice channels in/out/replace, matching Python's own list
    // slice-assignment semantics (e.g. seqs[2:0] = [[7]] inserts a new
    // channel at position 2, since slice 2:0 is an empty range).
    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let music = &mut *guard;
            if let Ok(i) = idx.extract::<i64>() {
                let len = music.seqs.len() as i64;
                let i = if i < 0 { i + len } else { i };
                if i < 0 || i >= len {
                    return Err(pyo3::exceptions::PyIndexError::new_err("music channel index out of range"));
                }
                let new_seq = val.extract::<Vec<u32>>()?;
                music.seqs[i as usize] = new_seq;
                return Ok(());
            }
            if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                let len = music.seqs.len() as i64;
                let indices = slice.indices(len as isize)?;
                if indices.step != 1 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "extended slices (step != 1) are not supported for Music.seqs assignment"
                    ));
                }
                let replacement = val.extract::<Vec<Vec<u32>>>()?;
                let start = indices.start.max(0) as usize;
                let stop = indices.stop.max(indices.start) as usize;
                music.seqs.splice(start..stop, replacement);
                return Ok(());
            }
            Err(pyo3::exceptions::PyTypeError::new_err("music channel index must be an int or slice"))
        }
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let music = &mut *guard;
            let len = music.seqs.len() as i64;
            if let Ok(i) = idx.extract::<i64>() {
                let i = if i < 0 { i + len } else { i };
                if i < 0 || i >= len {
                    return Err(pyo3::exceptions::PyIndexError::new_err("music channel index out of range"));
                }
                music.seqs.remove(i as usize);
                return Ok(());
            }
            if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
                let indices = slice.indices(len as isize)?;
                if indices.step != 1 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "extended slices (step != 1) are not supported for Music.seqs deletion"
                    ));
                }
                let start = indices.start.max(0) as usize;
                let stop = indices.stop.max(indices.start) as usize;
                music.seqs.drain(start..stop);
                return Ok(());
            }
            Err(pyo3::exceptions::PyTypeError::new_err("music channel index must be an int or slice"))
        }
    }

    pub fn append(&self, val: Vec<u32>) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.push(val); }
    }

    pub fn insert(&self, idx: usize, val: Vec<u32>) {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let seqs = &mut guard.seqs;
            let idx = idx.min(seqs.len());
            seqs.insert(idx, val);
        }
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<Vec<u32>> {
        {
            let mut guard = self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let seqs = &mut guard.seqs;
            let len = seqs.len() as i64;
            if len == 0 {
                return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty list"));
            }
            let i = idx.unwrap_or(-1);
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
            }
            Ok(seqs.remove(i as usize))
        }
    }

    pub fn extend(&self, vals: Vec<Vec<u32>>) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.extend(vals); }
    }

    pub fn clear(&self) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.clear(); }
    }

    pub fn __repr__(&self) -> String {
        format!("Seqs{:?}", (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs)
    }

    pub fn __bool__(&self) -> bool {
        !(&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.is_empty()
    }

    pub fn __reversed__(&self) -> Vec<Vec<u32>> {
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.iter().rev().cloned().collect()
    }

    pub fn __iadd__(&self, vals: Vec<Vec<u32>>) {
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.extend(vals); }
    }

    // Deprecated aliases for the whole-list bulk operations.
    pub fn from_list(&self, vals: Vec<Vec<u32>>) {
        warn_deprecated_once("MusicSeqs.from_list", "Seqs.from_list() is deprecated. Use slice assignment instead.");
        { (&mut *self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs = vals; }
    }

    pub fn to_list(&self) -> Vec<Vec<u32>> {
        warn_deprecated_once("MusicSeqs.to_list", "Seqs.to_list() is deprecated. Use list(seq) instead.");
        (&*self.parent.lock().unwrap_or_else(std::sync::PoisonError::into_inner)).seqs.clone()
    }
}

// ---------------------------------------------------------------------------
// Music wrapper (music_wrapper.rs)
// ---------------------------------------------------------------------------

// Same reasoning as SoundRef above. The Bank(usize)/Owned(RcMusic)
// split's Bank variant was removed after it caused a real crash: a
// lazily-resolved Bank(i) reference re-indexed into a possibly-changed
// bank later and could panic out-of-bounds — a Rust panic crossing
// the retro_run() FFI boundary aborts the whole process rather than
// raising a catchable Python exception. Every construction site now
// clones the Arc eagerly instead (matching upstream's own official
// binding), so a PyMusic always holds an independent, already-resolved
// handle.
#[pyclass(name = "Music", unsendable)]
pub struct PyMusic {
    music: pyxel_core::RcMusic,
}

impl PyMusic {
    pub fn rc(&self) -> pyxel_core::RcMusic {
        self.music.clone()
    }

    // Constructor for other modules — see PySound::from_rc() in
    // sound_wrapper_lr.rs for the reasoning (same pattern).
    pub(crate) fn from_rc(music: pyxel_core::RcMusic) -> Self {
        PyMusic { music }
    }
}

#[pymethods]
impl PyMusic {
    // Same reasoning as PySound::new() above.
    #[new]
    pub fn new() -> Self {
        PyMusic { music: pyxel_core::Music::new() }
    }

    #[getter]
    pub fn seqs(&self) -> PyMusicSeqs {
        PyMusicSeqs { parent: self.rc().clone() }
    }

    #[setter]
    pub fn set_seqs(&self, seqs: Vec<Vec<u32>>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set(&seqs); }
    }

    // Deprecated: snds_list (alias for seqs, getter only)
    #[getter]
    pub fn snds_list(&self) -> PyMusicSeqs {
        warn_deprecated_once("Music.snds_list", "Music.snds_list[ch] is deprecated. Use Music.seqs[ch] instead.");
        self.seqs()
    }

    // set(seq0, seq1, ...) — each arg is a list of sound indices for that channel
    // Also accepts a single Vec<Vec<u32>> for compatibility
    #[pyo3(signature = (*args))]
    pub fn set(&self, args: pyo3::Bound<'_, pyo3::types::PyTuple>) -> PyResult<()> {
        let seqs: Vec<Vec<u32>> = if args.len() == 1 {
            let first = args.get_item(0)?;
            if let Ok(v) = first.extract::<Vec<Vec<u32>>>() {
                v
            } else if let Ok(v) = first.extract::<Vec<u32>>() {
                vec![v]
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err("Invalid argument"));
            }
        } else {
            let mut seqs = Vec::new();
            for i in 0..args.len() {
                let item = args.get_item(i)?;
                let seq = item.extract::<Vec<u32>>()?;
                seqs.push(seq);
            }
            seqs
        };
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set(&seqs); }
        Ok(())
    }
}

#[pyclass(name = "MusicList")]
pub struct PyMusicList;

#[pymethods]
impl PyMusicList {
    pub fn __len__(&self) -> usize {
        pyxel_core::musics().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let len = pyxel_core::musics().len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PyMusic { music: pyxel_core::musics()[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PyMusic { music: pyxel_core::musics()[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PyMusic { music: pyxel_core::musics()[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("music index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyMusic>) -> PyResult<()> {
        let len = pyxel_core::musics().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                String::from("list index out of range")
            ));
        }
        pyxel_core::musics()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut musics = pyxel_core::musics();
        let len = musics.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("music index out of range"));
            }
            musics.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for musics deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            musics.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("music index must be an int or slice"))
    }

    pub fn __repr__(&self) -> String {
        format!("[Music; {}]", pyxel_core::musics().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::musics().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyMusic> {
        pyxel_core::musics().iter().rev()
            .map(|rc| PyMusic { music: rc.clone() })
            .collect()
    }

    pub fn append(&self, music: pyo3::PyRef<PyMusic>) {
        pyxel_core::musics().push(music.rc().clone());
    }

    pub fn insert(&self, idx: usize, music: pyo3::PyRef<PyMusic>) {
        let mut musics = pyxel_core::musics();
        let idx = idx.min(musics.len());
        musics.insert(idx, music.rc().clone());
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyMusic>>) {
        for m in &items {
            pyxel_core::musics().push(m.rc().clone());
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PyMusic>>) {
        self.extend(items);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PyMusic> {
        let mut musics = pyxel_core::musics();
        let len = musics.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty musics list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PyMusic { music: musics.remove(i as usize) })
    }

    pub fn clear(&self) {
        pyxel_core::musics().clear();
    }
}

