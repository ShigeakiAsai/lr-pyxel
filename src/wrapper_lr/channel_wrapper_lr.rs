//! channel_wrapper_lr.rs — Channel wrapper (pyxel.Channel,
//! pyxel.channels[n]).
//!
//! Tracks upstream's channel_wrapper.rs closely. References PySound
//! (Channel.play() accepts a Sound instance or list of them) via
//! PySound::rc(), reached through the usual `use crate::*;` +
//! crate-root re-export pattern used throughout wrapper_lr/.
//!
//! Includes PyChannelList (pyxel.channels[n], the bank accessor) —
//! found living inside the old wrappers.rs's "Colors wrapper" section
//! while splitting that section out, apparently just from how the
//! file grew over time rather than any deliberate grouping. Moved
//! here since it belongs with PyChannel, not Colors, and constructs
//! PyChannel directly (needs same-file access to its private field).

use pyo3::prelude::*;
use crate::*;

// ---------------------------------------------------------------------------
// Channel wrapper (channel_wrapper.rs)
// ---------------------------------------------------------------------------

// Channel needs to support two distinct identities: pyxel.channels[i]
// (a live view onto one of the real, shared channel banks) and
// pyxel.Channel() (documented upstream as "Create a new Channel
// instance" — a genuinely independent, standalone object, unconnected
// to any bank). Previously PyChannel only ever stored a bank index,
// and Channel::new() hardcoded bank 0 — meaning EVERY px.Channel()
// call was secretly a view onto the SAME real bank-0 channel, not an
// independent object at all. Found via dungeon-antiqua2.pyxapp's
// Sounds.set_volume(), which builds a list of several px.Channel()
// instances (each meant to hold its own gain) and hands them to
// pyxel.channels.from_list() — since every one of those "independent"
// instances was actually the same shared bank-0 storage, only the
// LAST one's gain survived, silently discarding the others. Mirrors
// ImageSource's Index(u32)/Image(RcImage) split in tilemap.rs.
//
// This used to be a Bank(usize)/Owned(RcChannel) enum, resolving a
// bank index lazily on each rc() call. The Bank(usize) variant was
// removed after it caused a real crash: capturing `list(pyxel.channels)`,
// mutating the bank shape, then using an old Bank(i) reference
// re-indexed into the now-different bank and panicked out-of-bounds —
// a Rust panic crossing the retro_run() FFI boundary aborts the whole
// process rather than raising a catchable Python exception. Every
// construction site now clones the Arc eagerly instead (matching
// upstream's own official binding, which does the same at
// __getitem__ time), so a PyChannel always holds an independent,
// already-resolved handle — no stale index ever gets re-resolved.
// Arc::clone() is just an atomic refcount bump, not a deep copy, so
// this is effectively free.
#[pyclass(name = "Channel", unsendable)]
pub struct PyChannel {
    channel: pyxel_core::RcChannel,
}

impl PyChannel {
    pub fn rc(&self) -> pyxel_core::RcChannel {
        self.channel.clone()
    }

    // Constructor for other modules — see PySound::from_rc() above
    // for the reasoning (same pattern, different type).
    pub(crate) fn from_rc(channel: pyxel_core::RcChannel) -> Self {
        PyChannel { channel }
    }
}

#[pymethods]
impl PyChannel {
    #[new]
    pub fn new() -> Self {
        PyChannel { channel: pyxel_core::Channel::new() }
    }

    #[getter]
    pub fn gain(&self) -> pyxel_core::ChannelGain {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).gain
    }

    #[setter]
    pub fn set_gain(&self, gain: pyxel_core::ChannelGain) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).gain = gain; }
    }

    #[getter]
    pub fn detune(&self) -> pyxel_core::ChannelDetune {
        (&*self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).detune
    }

    #[setter]
    pub fn set_detune(&self, detune: pyxel_core::ChannelDetune) {
        { (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).detune = detune; }
    }

    // See the top-level play() function's comment for why snd needs to
    // accept int/list[int]/Sound/list[Sound]/MML-string, not just a
    // bare u32 index.
    #[pyo3(signature = (snd, sec=None, r#loop=None, resume=None, tick=None))]
    pub fn play(&self, snd: pyo3::Bound<'_, pyo3::PyAny>, sec: Option<f32>, r#loop: Option<bool>, resume: Option<bool>, tick: Option<f32>) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let should_loop   = r#loop.unwrap_or(false);
            let should_resume = resume.unwrap_or(false);
            if tick.is_some() {
                warn_deprecated_once("Channel.play.tick", "tick option of Channel.play is deprecated. Use sec option instead.");
            }
            let sec = tick.map(|t| t / 120.0).or(sec);
            let rc = self.rc();
            let mut guard = rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let channel = &mut *guard;
            if let Ok(idx) = snd.extract::<u32>() {
                let sound = pyxel_core::sounds().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("snd must be a valid sound index"))?;
                channel.play_sound(sound, sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            } else if let Ok(seq) = snd.extract::<Vec<u32>>() {
                let pyxel_sounds = pyxel_core::sounds();
                let mut sounds = Vec::with_capacity(seq.len());
                for idx in seq {
                    sounds.push(
                        pyxel_sounds.get(idx as usize).cloned()
                            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("snd must contain only valid sound indices"))?
                    );
                }
                channel.play(sounds, sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            } else if let Ok(mml) = snd.extract::<String>() {
                // Same reasoning as the top-level play() fix above:
                // MML syntax errors map to the generic Exception, not
                // ValueError, matching upstream's own binding.
                channel.play_mml(&mml, sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyException::new_err)?;
            } else if let Ok(snd_ref) = snd.extract::<pyo3::PyRef<PySound>>() {
                channel.play(vec![snd_ref.rc().clone()], sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            } else if let Ok(snd_refs) = snd.extract::<Vec<pyo3::PyRef<PySound>>>() {
                let sounds: Vec<_> = snd_refs.iter().map(|s| s.rc().clone()).collect();
                channel.play(sounds, sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "snd must be int, list[int], Sound, list[Sound], or str"
                ));
            }
        }
        Ok(())
    }

    pub fn stop(&self) {
        unsafe {
            if PYXEL_READY {
                (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).stop();
            }
        }
    }

    pub fn play_pos(&self) -> Option<(u32, f32)> {
        unsafe {
            if !PYXEL_READY { return None; }
            (&mut *self.rc().lock().unwrap_or_else(std::sync::PoisonError::into_inner)).play_position()
        }
    }
}

#[pyclass(name = "ChannelList")]
pub struct PyChannelList;

#[pymethods]
impl PyChannelList {
    // Bounds now check the Vec's actual (possibly grown/shrunk) length
    // rather than the fixed NUM_CHANNELS default — upstream's own tests
    // (test_append_to_global_channels) confirm channels can grow past
    // the default count via append()/insert(), so NUM_CHANNELS is only
    // ever the *starting* size, not a hard ceiling.
    pub fn __len__(&self) -> usize {
        pyxel_core::channels().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let len = pyxel_core::channels().len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PyChannel { channel: pyxel_core::channels()[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PyChannel { channel: pyxel_core::channels()[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PyChannel { channel: pyxel_core::channels()[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("channel index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(idx_i64) = idx.extract::<i64>() {
            // Single index assignment: channels[n] = channel
            let len = pyxel_core::channels().len() as i64;
            let i = if idx_i64 < 0 { idx_i64 + len } else { idx_i64 };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("channel index out of range"));
            }
            let i = i as usize;
            let ch = val.extract::<pyo3::PyRef<PyChannel>>()?;
            {
                let src_rc = ch.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let dst_rc = pyxel_core::channels()[i].clone();
                let mut dst_guard = dst_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        } else if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            // Slice assignment: channels[a:b] = [ch0, ch1, ...].
            // Previously this ignored the slice's actual start/stop
            // and always copied into existing slots starting from
            // position 0 (same class of bug PyColors' __setitem__ had
            // before v0.17.0 — channels[0:2] = [...] happened to look
            // right since slots 0/1 matched, but channels[2:4] = [...]
            // would silently write into slots 0/1 instead). Now
            // properly respects the slice range and can change the
            // list's length via splice(), matching Python's own list
            // slice-assignment semantics. Unlike PyColors (a plain
            // Vec<u32>, spliced directly), each replacement slot here
            // must be a freshly created Channel with copied gain/
            // detune fields (matching append()/insert()'s existing
            // "copy values into the bank" semantics), not an alias of
            // the argument's own Rc — each channel bank slot keeps
            // its own independent identity.
            let len = pyxel_core::channels().len() as i64;
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for channels assignment"
                ));
            }
            let items = val.extract::<Vec<pyo3::PyRef<PyChannel>>>()?;
            let mut fresh_channels = Vec::with_capacity(items.len());
            for ch in &items {
                let fresh = pyxel_core::Channel::new();
                {
                    let src_rc = ch.rc();
                    let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let src = &*src_guard;
                    let mut dst_guard = fresh.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let dst = &mut *dst_guard;
                    dst.gain   = src.gain;
                    dst.detune = src.detune;
                }
                fresh_channels.push(fresh);
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            pyxel_core::channels().splice(start..stop, fresh_channels);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err("channel index must be an int or slice"));
        }
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut channels = pyxel_core::channels();
        let len = channels.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("channel index out of range"));
            }
            channels.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for channels deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            channels.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("channel index must be an int or slice"))
    }

    pub fn __repr__(&self) -> String {
        format!("[Channel; {}]", pyxel_core::channels().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::channels().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyChannel> {
        (0..pyxel_core::channels().len()).rev()
            .map(|i| PyChannel { channel: pyxel_core::channels()[i].clone() })
            .collect()
    }

    // append/insert copy gain/detune into a brand-new real bank slot
    // (matching __setitem__'s existing "copy values into the bank"
    // semantics, rather than aliasing the argument's own Rc) — each
    // channel bank stays its own independent identity.
    #[pyo3(signature = (channel=None))]
    pub fn append(&self, channel: Option<pyo3::PyRef<PyChannel>>) {
        let fresh = pyxel_core::Channel::new();
        if let Some(ch) = channel {
            {
                let src_rc = ch.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let mut dst_guard = fresh.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        }
        pyxel_core::channels().push(fresh);
    }

    #[pyo3(signature = (idx, channel=None))]
    pub fn insert(&self, idx: usize, channel: Option<pyo3::PyRef<PyChannel>>) {
        let fresh = pyxel_core::Channel::new();
        if let Some(ch) = channel {
            {
                let src_rc = ch.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let mut dst_guard = fresh.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        }
        let mut channels = pyxel_core::channels();
        let idx = idx.min(channels.len());
        channels.insert(idx, fresh);
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyChannel>>) {
        for ch in &items {
            let fresh = pyxel_core::Channel::new();
            {
                let src_rc = ch.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let mut dst_guard = fresh.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
            pyxel_core::channels().push(fresh);
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PyChannel>>) {
        self.extend(items);
    }

    // Removes and returns the bank at idx as a standalone, independent
    // Channel object (an Owned snapshot of its gain/detune at the time
    // of removal) — once popped, it's no longer tied to any bank
    // position, matching how a plain Python list.pop() detaches the
    // returned item from the list.
    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PyChannel> {
        let mut channels = pyxel_core::channels();
        let len = channels.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty channels list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        let removed = channels.remove(i as usize);
        Ok(PyChannel { channel: removed })
    }

    pub fn clear(&self) {
        pyxel_core::channels().clear();
    }

    // List-like bulk methods, matching the existing slice-assignment
    // semantics (channels[:] = [...]). Some scripts build a fresh list
    // of standalone Channel() objects and hand the whole thing over at
    // once via from_list() rather than slice syntax — found in
    // dungeon-antiqua2.pyxapp's Sounds.set_volume(), which constructs
    // one px.Channel() per bank with the desired gain and calls
    // px.channels.from_list(channels).
    pub fn from_list(&self, items: Vec<pyo3::PyRef<PyChannel>>) {
        for (i, ch) in items.iter().enumerate() {
            if i >= pyxel_core::channels().len() { break; }
            {
                let src_rc = ch.rc();
                let src_guard = src_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let src = &*src_guard;
                let dst_rc = pyxel_core::channels()[i].clone();
                let mut dst_guard = dst_rc.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let dst = &mut *dst_guard;
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        }
    }

    pub fn to_list(&self) -> Vec<PyChannel> {
        (0..pyxel_core::channels().len())
            .map(|i| PyChannel { channel: pyxel_core::channels()[i].clone() })
            .collect()
    }
}

