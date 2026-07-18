//! audio_wrapper_lr.rs — Audio functions (sound_set, play/playm/stop/
//! play_pos, and the deprecated sound()/music()/channel() bank
//! accessors).
//!
//! Tracks upstream's audio_wrapper.rs closely for play/playm/stop/
//! play_pos and the deprecated accessors. sound_set() has no upstream
//! analog (a standalone-function shortcut for writing a sound bank's
//! MML data by index, mirroring pyxel_core::Sound::set() directly
//! rather than going through a Sound instance).

use pyo3::prelude::*;
use crate::*;

// -- sound -------------------------------------------------------------------

// sound_set(no, notes, tones, volumes, effects, speed)
// Writes MML-style note/tone/volume/effect strings into sound bank `no`,
// mirroring pyxel_core::Sound::set(). Must be called once (e.g. at module
// load time) before play()/play_sound() can use that bank.
#[pyfunction]
pub fn sound_set(
    no: usize,
    notes: &str,
    tones: &str,
    volumes: &str,
    effects: &str,
    speed: u16,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let snds = pyxel_core::sounds();
        let Some(rc_sound) = snds.get(no) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "sound bank {no} does not exist"
            )));
        };
        let mut guard = rc_sound.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let sound: &mut pyxel_core::Sound = &mut *guard;
        sound
            .set(notes, tones, volumes, effects, speed)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }
}

// ---------------------------------------------------------------------------
// Audio functions (audio_wrapper.rs)
// ---------------------------------------------------------------------------

// play(ch, snd, sec=None, loop=False, resume=False, tick=None)
// snd can be a sound index (u32), a list of sound indices (Vec<u32>), a
// Sound instance, a list of Sound instances, or a raw MML string played
// directly on this channel (bypassing the sound bank entirely) — see
// the official API reference: "snd can be a sound number, a list, a
// Sound instance, a list of Sounds, or an MML string". The string case
// was missing entirely here (a documented, real upstream feature, not
// an edge case) — found via Braveforce-LDV_Demo.pyxapp's BGM playback
// (`pyxel.play(ch, mml_string)`), which failed silently from Python's
// perspective (caught by the game's own try/except with no visible
// symptom beyond "no BGM"); SFX in the same game go through
// Sound.mml() + an int index instead, which already worked, so nothing
// pointed at play() itself until the two were compared side by side.
// tick is an alternate way to specify the start position (1 tick = 1/120
// sec), restored in upstream Pyxel after being replaced by `sec` in 2.4.
// If both are given, tick takes precedence.
#[pyfunction]
#[pyo3(signature = (ch, snd, sec=None, r#loop=None, resume=None, tick=None))]
pub fn play(ch: u32, snd: pyo3::Bound<'_, pyo3::PyAny>, sec: Option<f32>, r#loop: Option<bool>, resume: Option<bool>, tick: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        // Bounds-check the channel index before any pyxel_core call —
        // every branch below indexes pyxel_core::channels()[ch] (some
        // directly here, some inside pyxel_core itself) with no
        // bounds-checking of its own, so an out-of-range ch previously
        // caused a raw Rust panic instead of a catchable Python
        // exception — and a panic inside PyO3-called Rust code aborts
        // the whole process (RetroArch included), not just the script.
        // Found via test_play_invalid_channel (pyxel.play(999, 0)),
        // which crashed RetroArch entirely rather than raising
        // cleanly.
        validate_index!(ch, pyxel_core::channels().len(), "ch", "channel");
        let should_loop   = r#loop.unwrap_or(false);
        let should_resume = resume.unwrap_or(false);
        if tick.is_some() {
            warn_deprecated_once("play.tick", "tick option of pyxel.play is deprecated. Use sec option instead.");
        }
        let sec = tick.map(|t| t / 120.0).or(sec);
        if let Ok(idx) = snd.extract::<u32>() {
            validate_index!(idx, pyxel_core::sounds().len(), "snd", "sound");
            pyxel_core::pyxel().play_sound(ch, idx, sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        } else if let Ok(seq) = snd.extract::<Vec<u32>>() {
            if seq.iter().any(|&i| i as usize >= pyxel_core::sounds().len()) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "snd must contain only valid sound indices"
                ));
            }
            pyxel_core::pyxel().play(ch, &seq, sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        } else if let Ok(mml) = snd.extract::<String>() {
            let _lock = pyxel_core::AudioLock::lock();
            let rc_channel = pyxel_core::channels()[ch as usize].clone();
            let mut guard = rc_channel.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let channel = &mut *guard;
            // MML syntax errors specifically map to the generic
            // Exception, not ValueError — matches upstream's own
            // binding (the (String, {...}) branch there uses
            // PyException::new_err, unlike every other branch here).
            channel.play_mml(&mml, sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyException::new_err)?;
        } else if let Ok(snd_ref) = snd.extract::<pyo3::PyRef<PySound>>() {
            let _lock = pyxel_core::AudioLock::lock();
            let rc_channel = pyxel_core::channels()[ch as usize].clone();
            let mut guard = rc_channel.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let channel = &mut *guard;
            channel.play(vec![snd_ref.rc().clone()], sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        } else if let Ok(snd_refs) = snd.extract::<Vec<pyo3::PyRef<PySound>>>() {
            let sounds: Vec<_> = snd_refs.iter().map(|s| s.rc().clone()).collect();
            let _lock = pyxel_core::AudioLock::lock();
            let rc_channel = pyxel_core::channels()[ch as usize].clone();
            let mut guard = rc_channel.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let channel = &mut *guard;
            channel.play(sounds, sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "snd must be int, list[int], Sound, list[Sound], or str"
            ));
        }
        Ok(())
    }
}

// playm(msc, sec=None, loop=False, tick=None)
// tick is an alternate way to specify the start position (1 tick = 1/120
// sec); if both sec and tick are given, tick takes precedence.
#[pyfunction]
#[pyo3(signature = (msc, sec=None, r#loop=None, tick=None))]
pub fn playm(msc: u32, sec: Option<f32>, r#loop: Option<bool>, tick: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        validate_index!(msc, pyxel_core::musics().len(), "msc", "music");
        if tick.is_some() {
            warn_deprecated_once("playm.tick", "tick option of pyxel.playm is deprecated. Use sec option instead.");
        }
        let sec = tick.map(|t| t / 120.0).or(sec);
        pyxel_core::pyxel().play_music(msc, sec, r#loop.unwrap_or(false))
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
    }
    Ok(())
}

// stop(ch=None)
#[pyfunction]
#[pyo3(signature = (ch=None))]
pub fn stop(ch: Option<u32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        match ch {
            Some(c) => {
                validate_index!(c, pyxel_core::channels().len(), "ch", "channel");
                pyxel_core::pyxel().stop_channel(c);
            }
            None => pyxel_core::pyxel().stop_all_channels(),
        }
    }
    Ok(())
}

// gen_bgm(preset, transp, instr, seed, play=None)
// Procedurally generates a 4-channel BGM (one MML string per channel) from
// (preset, transpose, instrumentation, seed). If play is true, immediately
// assigns and plays the generated MML on channels 0-3 (looping); either
// way, the generated MML strings are returned.
#[pyfunction]
#[pyo3(signature = (preset, transp, instr, seed, play=None))]
pub fn gen_bgm(preset: i32, transp: i32, instr: i32, seed: u64, play: Option<bool>) -> PyResult<Vec<String>> {
    unsafe {
        if !PYXEL_READY { return Ok(Vec::new()); }
        Ok(pyxel_core::pyxel().gen_bgm(preset, transp, instr, seed, play))
    }
}

// play_pos(ch)
#[pyfunction]
pub fn play_pos(ch: u32) -> PyResult<Option<(u32, f32)>> {
    unsafe {
        if !PYXEL_READY { return Ok(None); }
        validate_index!(ch, pyxel_core::channels().len(), "ch", "channel");
        Ok(pyxel_core::pyxel().play_position(ch))
    }
}

// Deprecated: pyxel.sound(n) → use pyxel.sounds[n]
#[pyfunction]
#[pyo3(name = "sound")]
pub fn sound_fn(snd: u32) -> PyResult<PySound> {
    warn_deprecated_once("sound()", "pyxel.sound(snd) is deprecated. Use pyxel.sounds[snd] instead.");
    validate_index!(snd, pyxel_core::NUM_SOUNDS as usize, "snd", "sound");
    Ok(PySound::from_rc(pyxel_core::sounds()[snd as usize].clone()))
}

// Deprecated: pyxel.music(n) → use pyxel.musics[n]
#[pyfunction]
#[pyo3(name = "music")]
pub fn music_fn(msc: u32) -> PyResult<PyMusic> {
    warn_deprecated_once("music()", "pyxel.music(msc) is deprecated. Use pyxel.musics[msc] instead.");
    validate_index!(msc, pyxel_core::NUM_MUSICS as usize, "msc", "music");
    Ok(PyMusic::from_rc(pyxel_core::musics()[msc as usize].clone()))
}

// Deprecated: pyxel.channel(n) → use pyxel.channels[n]
#[pyfunction]
#[pyo3(name = "channel")]
pub fn channel_fn(ch: u32) -> PyResult<PyChannel> {
    warn_deprecated_once("channel()", "pyxel.channel(ch) is deprecated. Use pyxel.channels[ch] instead.");
    validate_index!(ch, pyxel_core::NUM_CHANNELS as usize, "ch", "channel");
    Ok(PyChannel::from_rc(pyxel_core::channels()[ch as usize].clone()))
}

