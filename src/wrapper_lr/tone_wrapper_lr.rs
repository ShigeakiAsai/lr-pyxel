//! tone_wrapper_lr.rs — Tone wrapper (pyxel.Tone, pyxel.tones[n]).
//!
//! Tracks upstream's tone_wrapper.rs closely. PyToneWavetable (the
//! live-view list behind Tone.wavetable) is generated here via the
//! define_live_list! macro (defined in wrapper_lr/utils_lr.rs,
//! #[macro_export]'d so it's reachable via `use crate::*;` below with
//! no extra import needed) — mirrors sound_wrapper_lr.rs's
//! PySoundNotes/etc.

use pyo3::prelude::*;
use crate::*;

define_live_list!(PyToneWavetable, "ToneWavetable", pyxel_core::RcTone, pyxel_core::ToneSample, wavetable);

// ---------------------------------------------------------------------------
// Tone wrapper (tone_wrapper.rs)
// ---------------------------------------------------------------------------

// Same fix as PyChannel/ChannelRef above, for the same reason: Tone()
// is documented upstream as "Create a new Tone instance" (a genuinely
// independent object), but previously always hardcoded bank 0 — every
// px.Tone() was secretly a view onto the same shared bank-0 tone.
//
// Also carried the same Bank(usize)/Owned(RcTone) split as
// SoundRef/ChannelRef, removed for the same reason: a lazily-resolved
// Bank(i) reference re-indexed into a possibly-changed bank later
// (e.g. after `pyxel.tones.clear()`) and could panic out-of-bounds —
// a Rust panic crossing the retro_run() FFI boundary aborts the whole
// process rather than raising a catchable Python exception. Every
// construction site now clones the Arc eagerly instead (matching
// upstream's own official binding), so a PyTone always holds an
// independent, already-resolved handle.
#[pyclass(name = "Tone", unsendable)]
pub struct PyTone {
    tone: pyxel_core::RcTone,
}

impl PyTone {
    pub fn rc(&self) -> pyxel_core::RcTone {
        self.tone.clone()
    }
}

#[pymethods]
impl PyTone {
    #[new]
    pub fn new() -> Self {
        PyTone { tone: pyxel_core::Tone::new() }
    }

    #[getter]
    pub fn mode(&self) -> u32 {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).mode.into()
    }

    #[setter]
    pub fn set_mode(&self, mode: u32) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).mode = pyxel_core::ToneMode::from(mode); }
    }

    #[getter]
    pub fn sample_bits(&self) -> u32 {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).sample_bits
    }

    #[setter]
    pub fn set_sample_bits(&self, sample_bits: u32) -> PyResult<()> {
        // pyxel-core itself has no validate_sample_bits (unlike
        // Sound::validate_speed) — this range check only exists in
        // upstream's own binding layer (tone_wrapper.rs), so it has to
        // be replicated here rather than delegated to pyxel-core.
        if !(1..=pyxel_core::AUDIO_SAMPLE_BITS).contains(&sample_bits) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "sample_bits must be between 1 and {}",
                pyxel_core::AUDIO_SAMPLE_BITS
            )));
        }
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).sample_bits = sample_bits; }
        Ok(())
    }

    #[getter]
    pub fn gain(&self) -> pyxel_core::ToneGain {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).gain
    }

    #[setter]
    pub fn set_gain(&self, gain: pyxel_core::ToneGain) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).gain = gain; }
    }

    #[getter]
    pub fn wavetable(&self) -> PyToneWavetable {
        PyToneWavetable { parent: self.rc().clone() }
    }

    #[setter]
    pub fn set_wavetable(&self, wavetable: Vec<pyxel_core::ToneSample>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).wavetable = wavetable; }
    }

    // Deprecated: waveform (alias for wavetable). getter/setter use
    // distinct keys — upstream tests each as an independently "first
    // time this session" warning, in separate test functions, so a
    // single shared key (where whichever ran first consumed the only
    // warning) made the second one silently stop firing.
    #[getter]
    pub fn waveform(&self) -> PyToneWavetable {
        warn_deprecated_once("Tone.waveform.get", "Tone.waveform is deprecated. Use Tone.wavetable instead.");
        self.wavetable()
    }

    #[setter]
    pub fn set_waveform(&self, waveform: Vec<pyxel_core::ToneSample>) {
        warn_deprecated_once("Tone.waveform.set", "Tone.waveform is deprecated. Use Tone.wavetable instead.");
        self.set_wavetable(waveform);
    }

    // Deprecated: noise (alias for mode). Same getter/setter key split
    // as waveform above.
    #[getter]
    pub fn noise(&self) -> u32 {
        warn_deprecated_once("Tone.noise.get", "Tone.noise is deprecated. Use Tone.mode instead.");
        self.mode()
    }

    #[setter]
    pub fn set_noise(&self, mode: u32) {
        warn_deprecated_once("Tone.noise.set", "Tone.noise is deprecated. Use Tone.mode instead.");
        self.set_mode(mode);
    }
}

#[pyclass(name = "ToneList")]
pub struct PyToneList;

// Plain helper, kept outside #[pymethods] so PyO3 doesn't try to treat
// it as an exposed Python method (which caused it to be misinterpreted
// against the pyclass's own call signature).
impl PyToneList {
    fn copy_tone_into(src: &pyo3::PyRef<PyTone>, fresh: &pyxel_core::RcTone) {
        {
            let src_rc = src.rc();
            let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let src = &*src_guard;
            let mut dst_guard = fresh.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let dst = &mut *dst_guard;
            dst.mode        = src.mode;
            dst.sample_bits = src.sample_bits;
            dst.gain        = src.gain;
            dst.wavetable   = src.wavetable.clone();
        }
    }
}

#[pymethods]
impl PyToneList {
    pub fn __len__(&self) -> usize {
        pyxel_core::tones().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let len = pyxel_core::tones().len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PyTone { tone: pyxel_core::tones()[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PyTone { tone: pyxel_core::tones()[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PyTone { tone: pyxel_core::tones()[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("tone index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(idx_i64) = idx.extract::<i64>() {
            let len = pyxel_core::tones().len() as i64;
            let i = if idx_i64 < 0 { idx_i64 + len } else { idx_i64 };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("tone index out of range"));
            }
            let i = i as usize;
            let tone = val.extract::<pyo3::PyRef<PyTone>>()?;
            {
                let src_rc = tone.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let dst_rc = pyxel_core::tones()[i].clone();
                let mut dst_guard = dst_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.mode        = src.mode;
                dst.sample_bits = src.sample_bits;
                dst.gain        = src.gain;
                dst.wavetable   = src.wavetable.clone();
            }
        } else if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            // Slice assignment: tones[a:b] = [t0, t1, ...]. Same fix
            // as PyChannelList's __setitem__ — previously ignored the
            // slice's actual start/stop and always copied into
            // existing slots starting from position 0. Now properly
            // respects the slice range and can change the list's
            // length via splice(), using freshly created Tone slots
            // (matching append()/insert()'s existing "copy values
            // into the bank" semantics) rather than aliasing the
            // argument's own Rc.
            let len = pyxel_core::tones().len() as i64;
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for tones assignment"
                ));
            }
            let items = val.extract::<Vec<pyo3::PyRef<PyTone>>>()?;
            let mut fresh_tones = Vec::with_capacity(items.len());
            for tone in &items {
                let fresh = pyxel_core::Tone::new();
                Self::copy_tone_into(tone, &fresh);
                fresh_tones.push(fresh);
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            pyxel_core::tones().splice(start..stop, fresh_tones);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err("tone index must be an int or slice"));
        }
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut tones = pyxel_core::tones();
        let len = tones.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("tone index out of range"));
            }
            tones.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for tones deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            tones.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("tone index must be an int or slice"))
    }

    pub fn __repr__(&self) -> String {
        format!("[Tone; {}]", pyxel_core::tones().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::tones().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyTone> {
        (0..pyxel_core::tones().len()).rev()
            .map(|i| PyTone { tone: pyxel_core::tones()[i].clone() })
            .collect()
    }

    #[pyo3(signature = (tone=None))]
    pub fn append(&self, tone: Option<pyo3::PyRef<PyTone>>) {
        let fresh = pyxel_core::Tone::new();
        if let Some(t) = &tone { Self::copy_tone_into(t, &fresh); }
        pyxel_core::tones().push(fresh);
    }

    #[pyo3(signature = (idx, tone=None))]
    pub fn insert(&self, idx: usize, tone: Option<pyo3::PyRef<PyTone>>) {
        let fresh = pyxel_core::Tone::new();
        if let Some(t) = &tone { Self::copy_tone_into(t, &fresh); }
        let mut tones = pyxel_core::tones();
        let idx = idx.min(tones.len());
        tones.insert(idx, fresh);
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyTone>>) {
        for t in &items {
            let fresh = pyxel_core::Tone::new();
            Self::copy_tone_into(t, &fresh);
            pyxel_core::tones().push(fresh);
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PyTone>>) {
        self.extend(items);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PyTone> {
        let mut tones = pyxel_core::tones();
        let len = tones.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty tones list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        let removed = tones.remove(i as usize);
        Ok(PyTone { tone: removed })
    }

    pub fn clear(&self) {
        pyxel_core::tones().clear();
    }
}

