//! sound_wrapper_lr.rs — Sound wrapper (pyxel.Sound, pyxel.sounds[n]).
//!
//! Tracks upstream's sound_wrapper.rs closely. PySoundNotes/
//! PySoundTones/PySoundVolumes/PySoundEffects (the live-view list
//! types behind Sound.notes/.tones/.volumes/.effects) are generated
//! here via the define_live_list! macro (defined in
//! wrapper_lr/utils_lr.rs, #[macro_export]'d so it's reachable via
//! `use crate::*;` below with no extra import needed).

use pyo3::prelude::*;
use crate::*;

define_live_list!(PySoundNotes,   "SoundNotes",   pyxel_core::RcSound, pyxel_core::SoundNote,   notes);
define_live_list!(PySoundTones,   "SoundTones",   pyxel_core::RcSound, pyxel_core::SoundTone,   tones);
define_live_list!(PySoundVolumes, "SoundVolumes", pyxel_core::RcSound, pyxel_core::SoundVolume, volumes);
define_live_list!(PySoundEffects, "SoundEffects", pyxel_core::RcSound, pyxel_core::SoundEffect, effects);

// ---------------------------------------------------------------------------
// Sound bank wrapper (pyxel.sounds[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Sound wrapper (sound_wrapper.rs)
// ---------------------------------------------------------------------------

// Previously a Bank(usize)/Owned(RcSound) enum, split so that
// PySoundList.pop() could return a detached, standalone Sound (no
// longer tied to any bank position). The Bank(usize) lazy-index
// variant was removed after it caused a real crash: capturing
// `list(pyxel.sounds)`, mutating the bank (e.g. `.clear()`), then
// using an old Bank(i) reference re-indexed into the now-different
// bank and panicked out-of-bounds — a Rust panic crossing the
// retro_run() FFI boundary aborts the whole process rather than
// raising a catchable Python exception. Every construction site now
// clones the Arc eagerly instead (matching upstream's own official
// binding, which does the same at __getitem__ time), so a PySound
// always holds an independent, already-resolved handle — no stale
// index ever gets re-resolved. Arc::clone() is just an atomic
// refcount bump, not a deep copy, so this is effectively free; the
// old Bank(usize) variant didn't actually save that cost anyway,
// since almost every method already calls rc() (which cloned in
// both branches) at least once.
#[pyclass(name = "Sound", unsendable)]
pub struct PySound {
    sound: pyxel_core::RcSound,
}

impl PySound {
    pub fn rc(&self) -> pyxel_core::RcSound {
        self.sound.clone()
    }

    // Constructor for other modules that need to wrap an existing
    // RcSound in a fresh PySound (e.g. wrappers.rs's deprecated
    // sound() accessor and the screen/cursor/font __getattr__
    // entries) — `sound` itself stays private, matching PyImage's
    // from_rc()/rc() pair.
    pub(crate) fn from_rc(sound: pyxel_core::RcSound) -> Self {
        PySound { sound }
    }
}

#[pymethods]
impl PySound {
    // Missing entirely until now — pyxel.Sound() is a documented
    // upstream constructor for a standalone sound, not just a
    // bank-indexed pyxel.sounds[i]. Made possible by the Bank/Owned
    // enum split already added in v0.14.0's Seq protocol work.
    #[new]
    pub fn new() -> Self {
        PySound { sound: pyxel_core::Sound::new() }
    }

    #[getter]
    pub fn notes(&self) -> PySoundNotes {
        PySoundNotes { parent: self.rc().clone() }
    }

    #[setter(notes)]
    pub fn set_notes_list(&self, notes: Vec<pyxel_core::SoundNote>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).notes = notes; }
    }

    #[getter]
    pub fn tones(&self) -> PySoundTones {
        PySoundTones { parent: self.rc().clone() }
    }

    #[setter(tones)]
    pub fn set_tones_list(&self, tones: Vec<pyxel_core::SoundTone>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).tones = tones; }
    }

    #[getter]
    pub fn volumes(&self) -> PySoundVolumes {
        PySoundVolumes { parent: self.rc().clone() }
    }

    #[setter(volumes)]
    pub fn set_volumes_list(&self, volumes: Vec<pyxel_core::SoundVolume>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).volumes = volumes; }
    }

    #[getter]
    pub fn effects(&self) -> PySoundEffects {
        PySoundEffects { parent: self.rc().clone() }
    }

    #[setter(effects)]
    pub fn set_effects_list(&self, effects: Vec<pyxel_core::SoundEffect>) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).effects = effects; }
    }

    #[getter]
    pub fn speed(&self) -> pyxel_core::SoundSpeed {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).speed
    }

    #[setter]
    pub fn set_speed(&self, speed: pyxel_core::SoundSpeed) -> PyResult<()> {
        pyxel_core::Sound::validate_speed(speed).map_err(pyo3::exceptions::PyValueError::new_err)?;
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).speed = speed; }
        Ok(())
    }

    pub fn set(&self, notes: &str, tones: &str, volumes: &str, effects: &str, speed: pyxel_core::SoundSpeed) -> PyResult<()> {
        // Explicit pre-check, matching upstream's own binding exactly:
        // the underlying pyxel_core::Sound::set() already calls
        // validate_speed() internally too, but that error would map to
        // the generic PyException below (same as notes/tones/volumes/
        // effects errors), not ValueError — upstream's own test suite
        // expects speed=0 specifically to raise ValueError, so this
        // needs its own explicit check ahead of the general call.
        pyxel_core::Sound::validate_speed(speed).map_err(pyo3::exceptions::PyValueError::new_err)?;
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner))
                .set(notes, tones, volumes, effects, speed)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_notes(&self, notes: &str) -> PyResult<()> {
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set_notes(notes)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_tones(&self, tones: &str) -> PyResult<()> {
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set_tones(tones)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_volumes(&self, volumes: &str) -> PyResult<()> {
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set_volumes(volumes)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_effects(&self, effects: &str) -> PyResult<()> {
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).set_effects(effects)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (code=None))]
    pub fn mml(&self, code: Option<&str>) -> PyResult<()> {
        {
            let rc = self.rc();
            let mut guard = rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let snd = &mut *guard;
            match code {
                None => { snd.clear_mml(); Ok(()) }
                Some(c) => {
                    match snd.set_mml(c) {
                        Ok(()) => Ok(()),
                        Err(new_err) => {
                            // The old MML dialect (predates the current
                            // mml() grammar) uses syntax the new parser
                            // rejects (e.g. a bare 'x'/'X' token) — fall
                            // back to the legacy parser instead of
                            // raising, with a deprecation warning, for
                            // backward compatibility with scripts
                            // written against the old dialect. Uses its
                            // own key, separate from calling old_mml()
                            // directly — upstream tests each as its own
                            // independent "first time this session"
                            // warning.
                            match snd.old_mml(c) {
                                Ok(()) => {
                                    warn_deprecated_once(
                                        "Sound.mml.old_syntax",
                                        "Old MML syntax is deprecated. Use new syntax instead."
                                    );
                                    Ok(())
                                }
                                Err(_) => Err(pyo3::exceptions::PyException::new_err(new_err)),
                            }
                        }
                    }
                }
            }
        }
    }

    // Deprecated: old_mml (legacy MML dialect, predates the current
    // mml() syntax)
    pub fn old_mml(&self, code: &str) -> PyResult<()> {
        warn_deprecated_once("Sound.old_mml", "Sound.old_mml(code) is deprecated. Use Sound.mml(code) instead.");
        {
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).old_mml(code)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (filename=None))]
    pub fn pcm(&self, filename: Option<&str>) -> PyResult<()> {
        {
            let rc = self.rc();
            let mut guard = rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let snd = &mut *guard;
            match filename {
                None => { snd.clear_pcm(); Ok(()) }
                Some(f) => snd.load_pcm(f).map_err(pyo3::exceptions::PyException::new_err)
            }
        }
    }

    pub fn total_sec(&self) -> Option<f32> {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).total_seconds()
    }

    // Missing entirely until now — pyxel_core::Sound::save() itself
    // was always implemented (renders `sec` seconds of this Sound's
    // audio and writes it out as a real file, optionally transcoding
    // via ffmpeg), but lr-pyxel's own binding never wired it up.
    // Clones the Sound's data out from behind the lock before calling
    // save() — matches upstream's own binding (sound_wrapper.rs)
    // exactly, likely so this Sound's lock isn't held for the
    // duration of what can be a slow operation (rendering audio, then
    // file I/O and possibly an ffmpeg subprocess).
    #[pyo3(signature = (filename, sec, ffmpeg=None))]
    pub fn save(&self, filename: &str, sec: f32, ffmpeg: Option<bool>) -> PyResult<()> {
        let sound = (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).clone();
        sound.save(filename, sec, ffmpeg)
            .map_err(pyo3::exceptions::PyException::new_err)
    }
}

#[pyclass(name = "SoundList")]
pub struct PySoundList;

#[pymethods]
impl PySoundList {
    pub fn __len__(&self) -> usize {
        pyxel_core::sounds().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let len = pyxel_core::sounds().len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PySound { sound: pyxel_core::sounds()[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PySound { sound: pyxel_core::sounds()[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PySound { sound: pyxel_core::sounds()[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("sound index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PySound>) -> PyResult<()> {
        let len = pyxel_core::sounds().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                String::from("list index out of range")
            ));
        }
        pyxel_core::sounds()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut sounds = pyxel_core::sounds();
        let len = sounds.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("sound index out of range"));
            }
            sounds.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for sounds deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            sounds.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("sound index must be an int or slice"))
    }

    pub fn __repr__(&self) -> String {
        format!("[Sound; {}]", pyxel_core::sounds().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::sounds().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PySound> {
        pyxel_core::sounds().iter().rev()
            .map(|rc| PySound { sound: rc.clone() })
            .collect()
    }

    pub fn append(&self, sound: pyo3::PyRef<PySound>) {
        pyxel_core::sounds().push(sound.rc().clone());
    }

    pub fn insert(&self, idx: usize, sound: pyo3::PyRef<PySound>) {
        let mut sounds = pyxel_core::sounds();
        let idx = idx.min(sounds.len());
        sounds.insert(idx, sound.rc().clone());
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PySound>>) {
        for s in &items {
            pyxel_core::sounds().push(s.rc().clone());
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PySound>>) {
        self.extend(items);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PySound> {
        let mut sounds = pyxel_core::sounds();
        let len = sounds.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty sounds list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PySound { sound: sounds.remove(i as usize) })
    }

    pub fn clear(&self) {
        pyxel_core::sounds().clear();
    }
}

