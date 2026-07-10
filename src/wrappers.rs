//! wrappers.rs — Pyxel API wrappers (Python ↔ pyxel_core)

use std::cmp::Ordering;
use pyo3::prelude::*;
use crate::*;

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

fn warn_deprecated_once(key: &'static str, message: &str) {
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
            pyo3::Python::with_gil(|py| {
                if let Ok(builtins) = py.import_bound("builtins") {
                    if let Ok(print_fn) = builtins.getattr("print") {
                        let _ = print_fn.call1((format!("Warning: {message} is deprecated"),));
                    }
                }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Pyxel Python module — v0.4.0 minimal set
// ---------------------------------------------------------------------------

// -- drawing -----------------------------------------------------------------

#[pyfunction]
pub fn cls(color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().clear(color);
        }
    }
}

#[pyfunction]
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_rect(x, y, w, h, color);
        }
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, s, color, font=None))]
pub fn text(x: f32, y: f32, s: &str, color: u8, font: Option<pyo3::PyRef<PyFont>>) {
    unsafe {
        if PYXEL_READY {
            let font_ref = font.as_ref().map(|f| &f.inner);
            pyxel_core::pyxel().draw_text(x, y, s, color, font_ref);
        }
    }
}

#[pyfunction]
pub fn pset(x: f32, y: f32, color: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().set_pixel(x, y, color);
        }
    }
}

#[pyfunction]
pub fn pget(x: f32, y: f32) -> u8 {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().pixel(x, y)
        } else {
            0
        }
    }
}

// blt(x, y, img, u, v, w, h, colkey=None)
// Draws a width x height region starting at (u, v) of image bank `img`
// onto the screen at (x, y). `colkey` marks a transparent color index.
#[pyfunction]
#[pyo3(signature = (x, y, img, u, v, w, h, colkey=None, rotate=None, scale=None))]
#[allow(clippy::too_many_arguments)]
pub fn blt(x: f32, y: f32, img: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if let Ok(idx) = img.extract::<u32>() {
            pyxel_core::pyxel().draw_image(x, y, idx, u, v, w, h, colkey, rotate, scale);
        } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
            let src = pyimg.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.get();
            dst.draw_image(x, y, &src, u, v, w, h, colkey, rotate, scale);
        }
    }
    Ok(())
}

// bltm(x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None)
// Draws a region of tilemap bank `tm` onto the screen at (x, y).
// tm can be a bank index (u32) or a Tilemap instance — mirrors blt()'s
// existing int/Image handling, which bltm() previously lacked (only
// took a bank index), unlike upstream Pyxel's bltm/Tilemap.blt.
#[pyfunction]
#[pyo3(signature = (x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None))]
#[allow(clippy::too_many_arguments)]
pub fn bltm(x: f32, y: f32, tm: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if let Ok(idx) = tm.extract::<u32>() {
            pyxel_core::pyxel().draw_tilemap(x, y, idx, u, v, w, h, colkey, rotate, scale);
        } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
            let src = pytm.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.get();
            dst.draw_tilemap(x, y, &src, u, v, w, h, colkey, rotate, scale);
        }
    }
    Ok(())
}

// blt3d(x, y, w, h, img, pos, rot, fov=None, colkey=None)
// img can be a bank index (int) or an Image instance — previously only
// the index form was supported here, unlike the 2D blt(), which
// already handled both; confirmed via upstream's own test suite
// (test_blt3d_with_image_instance) that this is a real, documented gap
// rather than an intentional 3D-only restriction.
#[pyfunction]
#[pyo3(signature = (x, y, w, h, img, pos, rot, fov=None, colkey=None))]
#[allow(clippy::too_many_arguments)]
pub fn blt3d(
    x: f32, y: f32, w: f32, h: f32,
    img: pyo3::Bound<'_, pyo3::PyAny>,
    pos: (f32, f32, f32),
    rot: (f32, f32, f32),
    fov: Option<f32>,
    colkey: Option<u8>,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if let Ok(idx) = img.extract::<u32>() {
            pyxel_core::pyxel().draw_image_3d(x, y, w, h, idx, pos, rot, fov, colkey);
        } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
            let src = pyimg.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.get();
            dst.draw_image_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "img must be an image bank index (int) or an Image instance"
            ));
        }
    }
    Ok(())
}

// bltm3d(x, y, w, h, tm, pos, rot, fov=None, colkey=None)
// Same int-or-object handling as blt3d() above.
#[pyfunction]
#[pyo3(signature = (x, y, w, h, tm, pos, rot, fov=None, colkey=None))]
#[allow(clippy::too_many_arguments)]
pub fn bltm3d(
    x: f32, y: f32, w: f32, h: f32,
    tm: pyo3::Bound<'_, pyo3::PyAny>,
    pos: (f32, f32, f32),
    rot: (f32, f32, f32),
    fov: Option<f32>,
    colkey: Option<u8>,
) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        if let Ok(idx) = tm.extract::<u32>() {
            pyxel_core::pyxel().draw_tilemap_3d(x, y, w, h, idx, pos, rot, fov, colkey);
        } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
            let src = pytm.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.get();
            dst.draw_tilemap_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "tm must be a tilemap bank index (int) or a Tilemap instance"
            ));
        }
    }
    Ok(())
}

// Deprecated: pyxel.image(n) → use pyxel.images[n] instead
#[pyfunction]
pub fn image(img: u32) -> PyResult<PyImage> {
    warn_deprecated_once("image()", "pyxel.image() (use pyxel.images[n] instead)");
    if img as usize >= pyxel_core::NUM_IMAGES as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid image index"));
    }
    Ok(PyImage { image: pyxel_core::images()[img as usize].clone() })
}

// Deprecated: pyxel.tilemap(n) → use pyxel.tilemaps[n] instead
#[pyfunction]
#[pyo3(name = "tilemap")]
pub fn tilemap_fn(tm: u32) -> PyResult<PyTilemap> {
    warn_deprecated_once("tilemap()", "pyxel.tilemap() (use pyxel.tilemaps[n] instead)");
    if tm as usize >= pyxel_core::NUM_TILEMAPS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid tilemap index"));
    }
    Ok(PyTilemap { tilemap: pyxel_core::tilemaps()[tm as usize].clone() })
}

// image_load(bank, path, x=0, y=0, include_colors=False)
// Loads a PNG file into image bank `bank` at offset (x, y).
// Mirrors pyxel_core::Image::load(); the bank index must already exist
// (Pyxel pre-allocates NUM_IMAGES banks at init time).
#[pyfunction]
#[pyo3(signature = (bank, path, x=0, y=0, include_colors=false))]
pub fn image_load(bank: usize, path: &str, x: i32, y: i32, include_colors: bool) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let imgs = pyxel_core::images();
        let Some(rc_image) = imgs.get(bank) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "image bank {bank} does not exist"
            )));
        };
        // RcImage = Rc<UnsafeCell<Image>>; get a mutable reference via the cell
        let image: &mut pyxel_core::Image = &mut *rc_image.get();
        image
            .load(x, y, path, Some(include_colors))
            .map_err(pyo3::exceptions::PyOSError::new_err)
    }
}

// image_pset(bank, x, y, color)
// Sets a single pixel directly inside image bank `bank`, without going
// through the screen. Useful for hand-drawing a tiny sprite at runtime
// (e.g. for the blt() smoke test) without needing an external PNG.
#[pyfunction]
pub fn image_pset(bank: usize, x: f32, y: f32, color: u8) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY {
            return Ok(());
        }
        let imgs = pyxel_core::images();
        let Some(rc_image) = imgs.get(bank) else {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "image bank {bank} does not exist"
            )));
        };
        let image: &mut pyxel_core::Image = &mut *rc_image.get();
        image.set_pixel(x, y, color);
        Ok(())
    }
}

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

// ---------------------------------------------------------------------------
// Network functions (shells out to the `curl` CLI binary)
// ---------------------------------------------------------------------------
// Lakka's embedded Python lacks _socket.so / _ssl.so, so networking can't
// be done from Python (urllib etc.). These wrappers shell out to the
// system `curl` binary instead of linking libcurl into the core, which
// avoids cross-compiling libcurl/OpenSSL for the target device.
//
// Both release the GIL for the duration of the blocking curl call
// (py.allow_threads). Without this, a game calling these from a background
// Python thread (e.g. downloader.py) would still freeze the main
// update()/draw() loop, since PyO3 holds the GIL across the FFI call
// by default and only one Python thread can run at a time regardless.

/// download_file(url, save_path) -> bool
/// Downloads `url` to `save_path` via `curl -L -s -o save_path url`.
/// Returns True on success (curl exit code 0), False otherwise.
/// Does not raise on HTTP/network failure — check the return value.
#[pyfunction]
pub fn download_file(py: Python<'_>, url: &str, save_path: &str) -> PyResult<bool> {
    let url = url.to_owned();
    let save_path = save_path.to_owned();
    let ok = py.allow_threads(move || {
        std::process::Command::new("curl")
            .args(["-L", "-s", "-o", &save_path, &url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });
    Ok(ok)
}

/// http_get(url) -> str
/// Fetches `url` via `curl -L -s url` and returns stdout decoded as UTF-8
/// (lossy — invalid byte sequences are replaced, never raises on that).
/// Raises OSError only if the `curl` process itself could not be spawned.
#[pyfunction]
pub fn http_get(py: Python<'_>, url: &str) -> PyResult<String> {
    let url = url.to_owned();
    let output = py.allow_threads(move || {
        std::process::Command::new("curl")
            .args(["-L", "-s", &url])
            .output()
    });
    match output {
        Ok(o) => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(e) => Err(pyo3::exceptions::PyException::new_err(e.to_string())),
    }
}

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
        let sound: &mut pyxel_core::Sound = &mut *rc_sound.get();
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
        let should_loop   = r#loop.unwrap_or(false);
        let should_resume = resume.unwrap_or(false);
        if tick.is_some() {
            warn_deprecated_once("play.tick", "play()'s tick argument (use sec instead)");
        }
        let sec = tick.map(|t| t / 120.0).or(sec);
        if let Ok(idx) = snd.extract::<u32>() {
            pyxel_core::pyxel().play_sound(ch, idx, sec, should_loop, should_resume);
        } else if let Ok(seq) = snd.extract::<Vec<u32>>() {
            pyxel_core::pyxel().play(ch, &seq, sec, should_loop, should_resume);
        } else if let Ok(mml) = snd.extract::<String>() {
            let _lock = pyxel_core::AudioLock::lock();
            let channel = &mut *pyxel_core::channels()[ch as usize].get();
            channel.play_mml(&mml, sec, should_loop, should_resume)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        } else if let Ok(snd_ref) = snd.extract::<pyo3::PyRef<PySound>>() {
            let _lock = pyxel_core::AudioLock::lock();
            let channel = &mut *pyxel_core::channels()[ch as usize].get();
            channel.play(vec![snd_ref.rc().clone()], sec, should_loop, should_resume);
        } else if let Ok(snd_refs) = snd.extract::<Vec<pyo3::PyRef<PySound>>>() {
            let sounds: Vec<_> = snd_refs.iter().map(|s| s.rc().clone()).collect();
            let _lock = pyxel_core::AudioLock::lock();
            let channel = &mut *pyxel_core::channels()[ch as usize].get();
            channel.play(sounds, sec, should_loop, should_resume);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "snd must be an int, list of ints, Sound, list of Sounds, or MML string"
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
pub fn playm(msc: u32, sec: Option<f32>, r#loop: Option<bool>, tick: Option<f32>) {
    unsafe {
        if PYXEL_READY {
            if tick.is_some() {
                warn_deprecated_once("playm.tick", "playm()'s tick argument (use sec instead)");
            }
            let sec = tick.map(|t| t / 120.0).or(sec);
            pyxel_core::pyxel().play_music(msc, sec, r#loop.unwrap_or(false));
        }
    }
}

// stop(ch=None)
#[pyfunction]
#[pyo3(signature = (ch=None))]
pub fn stop(ch: Option<u32>) {
    unsafe {
        if !PYXEL_READY { return; }
        match ch {
            Some(c) => pyxel_core::pyxel().stop_channel(c),
            None    => pyxel_core::pyxel().stop_all_channels(),
        }
    }
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
pub fn play_pos(ch: u32) -> Option<(u32, f32)> {
    unsafe {
        if !PYXEL_READY { return None; }
        pyxel_core::pyxel().play_position(ch)
    }
}

// Deprecated: pyxel.sound(n) → use pyxel.sounds[n]
#[pyfunction]
#[pyo3(name = "sound")]
pub fn sound_fn(snd: u32) -> PyResult<PySound> {
    warn_deprecated_once("sound()", "pyxel.sound() (use pyxel.sounds[n] instead)");
    if snd as usize >= pyxel_core::NUM_SOUNDS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid sound index"));
    }
    Ok(PySound { sound_ref: SoundRef::Bank(snd as usize) })
}

// Deprecated: pyxel.music(n) → use pyxel.musics[n]
#[pyfunction]
#[pyo3(name = "music")]
pub fn music_fn(msc: u32) -> PyResult<PyMusic> {
    warn_deprecated_once("music()", "pyxel.music() (use pyxel.musics[n] instead)");
    if msc as usize >= pyxel_core::NUM_MUSICS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid music index"));
    }
    Ok(PyMusic { music_ref: MusicRef::Bank(msc as usize) })
}

// Deprecated: pyxel.channel(n) → use pyxel.channels[n]
#[pyfunction]
#[pyo3(name = "channel")]
pub fn channel_fn(ch: u32) -> PyResult<PyChannel> {
    warn_deprecated_once("channel()", "pyxel.channel() (use pyxel.channels[n] instead)");
    if ch as usize >= pyxel_core::NUM_CHANNELS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid channel index"));
    }
    Ok(PyChannel { channel_ref: ChannelRef::Bank(ch as usize) })
}

// -- input -------------------------------------------------------------------

#[pyfunction]
pub fn btn(key: u32) -> bool {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().is_button_down(key)
        } else {
            false
        }
    }
}

#[pyfunction]
pub fn btnp(key: u32, hold: Option<u32>, repeat: Option<u32>) -> bool {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().is_button_pressed(key, hold, repeat)
        } else {
            false
        }
    }
}

// -- system ------------------------------------------------------------------


// init() previously only updated GAME_W/GAME_H bookkeeping, the
// Python-visible pyxel.width/height module attributes, and notified
// RetroArch via SET_GEOMETRY — but never actually resized the physical
// canvas (missing the pyxel_core::pyxel().set_screen_size() call). This
// meant the game's REAL init() call (with its actual runtime-computed
// w/h — which may differ from parse_pyxel_init()'s static pre-parse
// guess, e.g. when the real value depends on a conditional expression
// or other logic the static parser can't evaluate) correctly told
// RetroArch "expect WxH", but the underlying video stream stayed
// capped at whatever size the pre-parse guessed, silently truncating
// anything beyond that. Found via finardry.pyxapp:
// `height = 256 if MODE_SQUARE else 240; px.init(256, height, ...)` —
// the static parser can't evaluate the conditional, falls back to the
// default 128, and the real init() call's correct height (240) was
// never propagated to the actual canvas.
#[pyfunction]
#[pyo3(signature = (w, h, title=None, fps=None, quit_key=None,
                    display_scale=None, capture_scale=None,
                    capture_sec=None))]
pub fn init(
    w: u32, h: u32,
    title: Option<&str>, fps: Option<u32>, quit_key: Option<u32>,
    display_scale: Option<u32>, capture_scale: Option<u32>, capture_sec: Option<u32>,
) {
    let _ = (title, quit_key, display_scale, capture_scale, capture_sec);
    unsafe {
        // Save game-requested size and FPS
        GAME_W = w.max(1);
        GAME_H = h.max(1);
        GAME_FPS = fps.unwrap_or(30).clamp(1, 60);

        // Actually resize the physical canvas to match — this is the
        // authoritative source of truth (the script's real runtime
        // values), superseding whatever the pre-execution static parse
        // guessed. Also updates pyxel_core::width()/height().
        if PYXEL_READY {
            pyxel_core::pyxel().set_screen_size(GAME_W, GAME_H);
        }

        // Update pyxel.width/height module attributes to reflect game size
        Python::with_gil(|py| {
            if let Ok(m) = py.import_bound("pyxel") {
                let _ = m.setattr("width",  GAME_W);
                let _ = m.setattr("height", GAME_H);
            }
        });

        // Notify RetroArch of the game's actual screen geometry.
        // RETRO_ENVIRONMENT_SET_GEOMETRY (37) lets us change width/height
        // after init without restarting the core.
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   GAME_W,
                base_height:  GAME_H,
                max_width:    1024,
                max_height:   1024,
                aspect_ratio: GAME_W as f32 / GAME_H as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
    }
}

// run(update, draw) — caches the callbacks for the libretro frame loop.
// In normal Pyxel this starts the event loop; here it is the hook that
// lets class-based games (e.g. Game() → pyxel.run(self.update, self.draw))
// register their callbacks with the core.
#[pyfunction]
pub fn run(update: PyObject, draw: PyObject) {
    unsafe {
        PY_UPDATE = Some(update);
        PY_DRAW   = Some(draw);
    }
}

// -- key constants -----------------------------------------------------------

pub fn add_module_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Graphics
    m.add("NUM_COLORS",      pyxel_core::NUM_COLORS)?;
    m.add("NUM_IMAGES",      pyxel_core::NUM_IMAGES)?;
    m.add("IMAGE_SIZE",      pyxel_core::IMAGE_SIZE)?;
    m.add("NUM_TILEMAPS",    pyxel_core::NUM_TILEMAPS)?;
    m.add("TILEMAP_SIZE",    pyxel_core::TILEMAP_SIZE)?;
    m.add("TILE_SIZE",       pyxel_core::TILE_SIZE)?;
    m.add("COLOR_BLACK",     pyxel_core::COLOR_BLACK)?;
    m.add("COLOR_NAVY",      pyxel_core::COLOR_NAVY)?;
    m.add("COLOR_PURPLE",    pyxel_core::COLOR_PURPLE)?;
    m.add("COLOR_GREEN",     pyxel_core::COLOR_GREEN)?;
    m.add("COLOR_BROWN",     pyxel_core::COLOR_BROWN)?;
    m.add("COLOR_DARK_BLUE", pyxel_core::COLOR_DARK_BLUE)?;
    m.add("COLOR_LIGHT_BLUE",pyxel_core::COLOR_LIGHT_BLUE)?;
    m.add("COLOR_WHITE",     pyxel_core::COLOR_WHITE)?;
    m.add("COLOR_RED",       pyxel_core::COLOR_RED)?;
    m.add("COLOR_ORANGE",    pyxel_core::COLOR_ORANGE)?;
    m.add("COLOR_YELLOW",    pyxel_core::COLOR_YELLOW)?;
    m.add("COLOR_LIME",      pyxel_core::COLOR_LIME)?;
    m.add("COLOR_CYAN",      pyxel_core::COLOR_CYAN)?;
    m.add("COLOR_GRAY",      pyxel_core::COLOR_GRAY)?;
    m.add("COLOR_PINK",      pyxel_core::COLOR_PINK)?;
    m.add("COLOR_PEACH",     pyxel_core::COLOR_PEACH)?;
    m.add("FONT_WIDTH",      pyxel_core::FONT_WIDTH)?;
    m.add("FONT_HEIGHT",     pyxel_core::FONT_HEIGHT)?;
    // Audio
    m.add("NUM_CHANNELS",          pyxel_core::NUM_CHANNELS)?;
    m.add("NUM_TONES",             pyxel_core::NUM_TONES)?;
    m.add("NUM_SOUNDS",            pyxel_core::NUM_SOUNDS)?;
    m.add("NUM_MUSICS",            pyxel_core::NUM_MUSICS)?;
    m.add("TONE_TRIANGLE",         pyxel_core::TONE_TRIANGLE)?;
    m.add("TONE_SQUARE",           pyxel_core::TONE_SQUARE)?;
    m.add("TONE_PULSE",            pyxel_core::TONE_PULSE)?;
    m.add("TONE_NOISE",            pyxel_core::TONE_NOISE)?;
    m.add("EFFECT_NONE",           pyxel_core::EFFECT_NONE)?;
    m.add("EFFECT_SLIDE",          pyxel_core::EFFECT_SLIDE)?;
    m.add("EFFECT_VIBRATO",        pyxel_core::EFFECT_VIBRATO)?;
    m.add("EFFECT_FADEOUT",        pyxel_core::EFFECT_FADEOUT)?;
    m.add("EFFECT_HALF_FADEOUT",   pyxel_core::EFFECT_HALF_FADEOUT)?;
    m.add("EFFECT_QUARTER_FADEOUT",pyxel_core::EFFECT_QUARTER_FADEOUT)?;
    // Keyboard
    m.add("KEY_BACKSPACE",   pyxel_core::KEY_BACKSPACE)?;
    m.add("KEY_TAB",         pyxel_core::KEY_TAB)?;
    m.add("KEY_RETURN",      pyxel_core::KEY_RETURN)?;
    m.add("KEY_ESCAPE",      pyxel_core::KEY_ESCAPE)?;
    m.add("KEY_SPACE",       pyxel_core::KEY_SPACE)?;
    m.add("KEY_0",           pyxel_core::KEY_0)?;
    m.add("KEY_1",           pyxel_core::KEY_1)?;
    m.add("KEY_2",           pyxel_core::KEY_2)?;
    m.add("KEY_3",           pyxel_core::KEY_3)?;
    m.add("KEY_4",           pyxel_core::KEY_4)?;
    m.add("KEY_5",           pyxel_core::KEY_5)?;
    m.add("KEY_6",           pyxel_core::KEY_6)?;
    m.add("KEY_7",           pyxel_core::KEY_7)?;
    m.add("KEY_8",           pyxel_core::KEY_8)?;
    m.add("KEY_9",           pyxel_core::KEY_9)?;
    m.add("KEY_A",           pyxel_core::KEY_A)?;
    m.add("KEY_B",           pyxel_core::KEY_B)?;
    m.add("KEY_C",           pyxel_core::KEY_C)?;
    m.add("KEY_D",           pyxel_core::KEY_D)?;
    m.add("KEY_E",           pyxel_core::KEY_E)?;
    m.add("KEY_F",           pyxel_core::KEY_F)?;
    m.add("KEY_G",           pyxel_core::KEY_G)?;
    m.add("KEY_H",           pyxel_core::KEY_H)?;
    m.add("KEY_I",           pyxel_core::KEY_I)?;
    m.add("KEY_J",           pyxel_core::KEY_J)?;
    m.add("KEY_K",           pyxel_core::KEY_K)?;
    m.add("KEY_L",           pyxel_core::KEY_L)?;
    m.add("KEY_M",           pyxel_core::KEY_M)?;
    m.add("KEY_N",           pyxel_core::KEY_N)?;
    m.add("KEY_O",           pyxel_core::KEY_O)?;
    m.add("KEY_P",           pyxel_core::KEY_P)?;
    m.add("KEY_Q",           pyxel_core::KEY_Q)?;
    m.add("KEY_R",           pyxel_core::KEY_R)?;
    m.add("KEY_S",           pyxel_core::KEY_S)?;
    m.add("KEY_T",           pyxel_core::KEY_T)?;
    m.add("KEY_U",           pyxel_core::KEY_U)?;
    m.add("KEY_V",           pyxel_core::KEY_V)?;
    m.add("KEY_W",           pyxel_core::KEY_W)?;
    m.add("KEY_X",           pyxel_core::KEY_X)?;
    m.add("KEY_Y",           pyxel_core::KEY_Y)?;
    m.add("KEY_Z",           pyxel_core::KEY_Z)?;
    m.add("KEY_DELETE",      pyxel_core::KEY_DELETE)?;
    m.add("KEY_CAPSLOCK",    pyxel_core::KEY_CAPSLOCK)?;
    m.add("KEY_F1",          pyxel_core::KEY_F1)?;
    m.add("KEY_F2",          pyxel_core::KEY_F2)?;
    m.add("KEY_F3",          pyxel_core::KEY_F3)?;
    m.add("KEY_F4",          pyxel_core::KEY_F4)?;
    m.add("KEY_F5",          pyxel_core::KEY_F5)?;
    m.add("KEY_F6",          pyxel_core::KEY_F6)?;
    m.add("KEY_F7",          pyxel_core::KEY_F7)?;
    m.add("KEY_F8",          pyxel_core::KEY_F8)?;
    m.add("KEY_F9",          pyxel_core::KEY_F9)?;
    m.add("KEY_F10",         pyxel_core::KEY_F10)?;
    m.add("KEY_F11",         pyxel_core::KEY_F11)?;
    m.add("KEY_F12",         pyxel_core::KEY_F12)?;
    m.add("KEY_INSERT",      pyxel_core::KEY_INSERT)?;
    m.add("KEY_HOME",        pyxel_core::KEY_HOME)?;
    m.add("KEY_PAGEUP",      pyxel_core::KEY_PAGEUP)?;
    m.add("KEY_END",         pyxel_core::KEY_END)?;
    m.add("KEY_PAGEDOWN",    pyxel_core::KEY_PAGEDOWN)?;
    m.add("KEY_RIGHT",       pyxel_core::KEY_RIGHT)?;
    m.add("KEY_LEFT",        pyxel_core::KEY_LEFT)?;
    m.add("KEY_DOWN",        pyxel_core::KEY_DOWN)?;
    m.add("KEY_UP",          pyxel_core::KEY_UP)?;
    m.add("KEY_LCTRL",       pyxel_core::KEY_LCTRL)?;
    m.add("KEY_LSHIFT",      pyxel_core::KEY_LSHIFT)?;
    m.add("KEY_LALT",        pyxel_core::KEY_LALT)?;
    m.add("KEY_RCTRL",       pyxel_core::KEY_RCTRL)?;
    m.add("KEY_RSHIFT",      pyxel_core::KEY_RSHIFT)?;
    m.add("KEY_RALT",        pyxel_core::KEY_RALT)?;
    m.add("KEY_LGUI",        pyxel_core::KEY_LGUI)?;
    m.add("KEY_RGUI",        pyxel_core::KEY_RGUI)?;
    m.add("KEY_SHIFT",       pyxel_core::KEY_SHIFT)?;
    m.add("KEY_CTRL",        pyxel_core::KEY_CTRL)?;
    m.add("KEY_ALT",         pyxel_core::KEY_ALT)?;
    m.add("KEY_GUI",         pyxel_core::KEY_GUI)?;
    m.add("KEY_NONE",        pyxel_core::KEY_NONE)?;
    // Mouse
    m.add("MOUSE_POS_X",          pyxel_core::MOUSE_POS_X)?;
    m.add("MOUSE_POS_Y",          pyxel_core::MOUSE_POS_Y)?;
    m.add("MOUSE_WHEEL_X",        pyxel_core::MOUSE_WHEEL_X)?;
    m.add("MOUSE_WHEEL_Y",        pyxel_core::MOUSE_WHEEL_Y)?;
    m.add("MOUSE_BUTTON_LEFT",    pyxel_core::MOUSE_BUTTON_LEFT)?;
    m.add("MOUSE_BUTTON_MIDDLE",  pyxel_core::MOUSE_BUTTON_MIDDLE)?;
    m.add("MOUSE_BUTTON_RIGHT",   pyxel_core::MOUSE_BUTTON_RIGHT)?;
    m.add("MOUSE_BUTTON_X1",      pyxel_core::MOUSE_BUTTON_X1)?;
    m.add("MOUSE_BUTTON_X2",      pyxel_core::MOUSE_BUTTON_X2)?;
    // Gamepad 1
    m.add("GAMEPAD1_AXIS_LEFTX",        pyxel_core::GAMEPAD1_AXIS_LEFTX)?;
    m.add("GAMEPAD1_AXIS_LEFTY",        pyxel_core::GAMEPAD1_AXIS_LEFTY)?;
    m.add("GAMEPAD1_AXIS_RIGHTX",       pyxel_core::GAMEPAD1_AXIS_RIGHTX)?;
    m.add("GAMEPAD1_AXIS_RIGHTY",       pyxel_core::GAMEPAD1_AXIS_RIGHTY)?;
    m.add("GAMEPAD1_AXIS_TRIGGERLEFT",  pyxel_core::GAMEPAD1_AXIS_TRIGGERLEFT)?;
    m.add("GAMEPAD1_AXIS_TRIGGERRIGHT", pyxel_core::GAMEPAD1_AXIS_TRIGGERRIGHT)?;
    m.add("GAMEPAD1_BUTTON_A",             pyxel_core::GAMEPAD1_BUTTON_A)?;
    m.add("GAMEPAD1_BUTTON_B",             pyxel_core::GAMEPAD1_BUTTON_B)?;
    m.add("GAMEPAD1_BUTTON_X",             pyxel_core::GAMEPAD1_BUTTON_X)?;
    m.add("GAMEPAD1_BUTTON_Y",             pyxel_core::GAMEPAD1_BUTTON_Y)?;
    m.add("GAMEPAD1_BUTTON_BACK",          pyxel_core::GAMEPAD1_BUTTON_BACK)?;
    m.add("GAMEPAD1_BUTTON_GUIDE",         pyxel_core::GAMEPAD1_BUTTON_GUIDE)?;
    m.add("GAMEPAD1_BUTTON_START",         pyxel_core::GAMEPAD1_BUTTON_START)?;
    m.add("GAMEPAD1_BUTTON_LEFTSTICK",     pyxel_core::GAMEPAD1_BUTTON_LEFTSTICK)?;
    m.add("GAMEPAD1_BUTTON_RIGHTSTICK",    pyxel_core::GAMEPAD1_BUTTON_RIGHTSTICK)?;
    m.add("GAMEPAD1_BUTTON_LEFTSHOULDER",  pyxel_core::GAMEPAD1_BUTTON_LEFTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_RIGHTSHOULDER", pyxel_core::GAMEPAD1_BUTTON_RIGHTSHOULDER)?;
    m.add("GAMEPAD1_BUTTON_DPAD_UP",       pyxel_core::GAMEPAD1_BUTTON_DPAD_UP)?;
    m.add("GAMEPAD1_BUTTON_DPAD_DOWN",     pyxel_core::GAMEPAD1_BUTTON_DPAD_DOWN)?;
    m.add("GAMEPAD1_BUTTON_DPAD_LEFT",     pyxel_core::GAMEPAD1_BUTTON_DPAD_LEFT)?;
    m.add("GAMEPAD1_BUTTON_DPAD_RIGHT",    pyxel_core::GAMEPAD1_BUTTON_DPAD_RIGHT)?;
    // Gamepad 2
    m.add("GAMEPAD2_BUTTON_A",             pyxel_core::GAMEPAD2_BUTTON_A)?;
    m.add("GAMEPAD2_BUTTON_B",             pyxel_core::GAMEPAD2_BUTTON_B)?;
    m.add("GAMEPAD2_BUTTON_X",             pyxel_core::GAMEPAD2_BUTTON_X)?;
    m.add("GAMEPAD2_BUTTON_Y",             pyxel_core::GAMEPAD2_BUTTON_Y)?;
    m.add("GAMEPAD2_BUTTON_BACK",          pyxel_core::GAMEPAD2_BUTTON_BACK)?;
    m.add("GAMEPAD2_BUTTON_GUIDE",         pyxel_core::GAMEPAD2_BUTTON_GUIDE)?;
    m.add("GAMEPAD2_BUTTON_START",         pyxel_core::GAMEPAD2_BUTTON_START)?;
    m.add("GAMEPAD2_BUTTON_LEFTSHOULDER",  pyxel_core::GAMEPAD2_BUTTON_LEFTSHOULDER)?;
    m.add("GAMEPAD2_BUTTON_RIGHTSHOULDER", pyxel_core::GAMEPAD2_BUTTON_RIGHTSHOULDER)?;
    m.add("GAMEPAD2_BUTTON_DPAD_UP",       pyxel_core::GAMEPAD2_BUTTON_DPAD_UP)?;
    m.add("GAMEPAD2_BUTTON_DPAD_DOWN",     pyxel_core::GAMEPAD2_BUTTON_DPAD_DOWN)?;
    m.add("GAMEPAD2_BUTTON_DPAD_LEFT",     pyxel_core::GAMEPAD2_BUTTON_DPAD_LEFT)?;
    m.add("GAMEPAD2_BUTTON_DPAD_RIGHT",    pyxel_core::GAMEPAD2_BUTTON_DPAD_RIGHT)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Math functions
// ---------------------------------------------------------------------------
// Math functions (math_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyfunction] fn ceil(x: f32) -> i32 { pyxel_core::Pyxel::ceil(x) }
#[pyfunction] fn floor(x: f32) -> i32 { pyxel_core::Pyxel::floor(x) }
#[pyfunction] fn sqrt(x: f32) -> f32 { pyxel_core::Pyxel::sqrt(x) }
#[pyfunction] fn sin(deg: f32) -> f32 { pyxel_core::Pyxel::sin(deg) }
#[pyfunction] fn cos(deg: f32) -> f32 { pyxel_core::Pyxel::cos(deg) }
#[pyfunction] fn atan2(y: f32, x: f32) -> f32 { pyxel_core::Pyxel::atan2(y, x) }
#[pyfunction] fn rseed(seed: u32) { pyxel_core::Pyxel::random_seed(seed); }
#[pyfunction] fn rndi(a: i32, b: i32) -> i32 { pyxel_core::Pyxel::random_int(a, b) }
#[pyfunction] fn rndf(a: f32, b: f32) -> f32 { pyxel_core::Pyxel::random_float(a, b) }
#[pyfunction] fn nseed(seed: u32) { pyxel_core::Pyxel::noise_seed(seed); }

// clamp: returns int for int inputs, float for float inputs
#[pyfunction]
pub fn clamp(
    x: pyo3::Bound<'_, pyo3::PyAny>,
    lower: pyo3::Bound<'_, pyo3::PyAny>,
    upper: pyo3::Bound<'_, pyo3::PyAny>,
) -> PyResult<Py<pyo3::PyAny>> {
    let py = x.py();
    if let (Ok(xi), Ok(li), Ok(ui)) = (
        x.extract::<i64>(),
        lower.extract::<i64>(),
        upper.extract::<i64>(),
    ) {
        let (lo, hi) = if li < ui { (li, ui) } else { (ui, li) };
        let v = xi.clamp(lo, hi);
        return Ok(v.into_py(py));
    }
    let xf = x.extract::<f64>()?;
    let lf = lower.extract::<f64>()?;
    let uf = upper.extract::<f64>()?;
    let (lo, hi) = if lf < uf { (lf, uf) } else { (uf, lf) };
    Ok(xf.clamp(lo, hi).into_py(py))
}

// sgn: returns int for int inputs, float for float inputs
#[pyfunction]
pub fn sgn(x: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<Py<pyo3::PyAny>> {
    let py = x.py();
    if let Ok(xi) = x.extract::<i64>() {
        let v: i64 = match xi.cmp(&0) {
            Ordering::Greater => 1,
            Ordering::Less => -1,
            Ordering::Equal => 0,
        };
        return Ok(v.into_py(py));
    }
    let xf = x.extract::<f64>()?;
    let v: f64 = match xf.partial_cmp(&0.0) {
        Some(Ordering::Greater) => 1.0,
        Some(Ordering::Less) => -1.0,
        _ => 0.0,
    };
    Ok(v.into_py(py))
}

#[pyfunction]
#[pyo3(signature = (x, y=None, z=None))]
pub fn noise(x: f32, y: Option<f32>, z: Option<f32>) -> f32 {
    pyxel_core::Pyxel::noise(x, y.unwrap_or(0.0), z.unwrap_or(0.0))
}

// ---------------------------------------------------------------------------
// Drawing functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_line(x1, y1, x2, y2, color); } }
}
#[pyfunction]
pub fn rectb(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_rect_border(x, y, w, h, color); } }
}
#[pyfunction]
pub fn circ(x: f32, y: f32, r: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle(x, y, r, color); } }
}
#[pyfunction]
pub fn circb(x: f32, y: f32, r: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle_border(x, y, r, color); } }
}
#[pyfunction]
pub fn elli(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse(x, y, w, h, color); } }
}
#[pyfunction]
pub fn ellib(x: f32, y: f32, w: f32, h: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse_border(x, y, w, h, color); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn tri(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle(x1, y1, x2, y2, x3, y3, color); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn trib(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle_border(x1, y1, x2, y2, x3, y3, color); } }
}
#[pyfunction]
pub fn fill(x: f32, y: f32, color: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().flood_fill(x, y, color); } }
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None, w=None, h=None))]
pub fn clip(x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (x, y, w, h) {
            (Some(x), Some(y), Some(w), Some(h)) => pyxel_core::pyxel().set_clip_rect(x, y, w, h),
            _ => pyxel_core::pyxel().reset_clip_rect(),
        }
    }
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None))]
pub fn camera(x: Option<f32>, y: Option<f32>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (x, y) {
            (Some(x), Some(y)) => pyxel_core::pyxel().set_camera(x, y),
            _ => pyxel_core::pyxel().reset_camera(),
        }
    }
}
#[pyfunction]
#[pyo3(signature = (col1=None, col2=None))]
pub fn pal(col1: Option<u8>, col2: Option<u8>) {
    unsafe {
        if !PYXEL_READY { return; }
        match (col1, col2) {
            (Some(c1), Some(c2)) => pyxel_core::pyxel().map_color(c1, c2),
            _ => pyxel_core::pyxel().reset_color_map(),
        }
    }
}
#[pyfunction]
pub fn dither(alpha: f32) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_dithering(alpha); } }
}

// ---------------------------------------------------------------------------
// Input functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn btnr(key: u32) -> bool {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().is_button_released(key) } else { false } }
}
#[pyfunction]
pub fn btnv(key: u32) -> i32 {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().button_value(key) } else { 0 } }
}
#[pyfunction]
pub fn mouse(visible: bool) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_mouse_visible(visible); } }
}

#[pyfunction]
pub fn set_btn(key: u32, state: bool) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_button_state(key, state); } }
}

#[pyfunction]
pub fn set_btnv(key: u32, val: i32) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_button_value(key, val); } }
}

#[pyfunction]
pub fn set_mouse_pos(x: f32, y: f32) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_mouse_position(x, y); } }
}

#[pyfunction]
pub fn set_input_text(text: &str) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_input_text(text); } }
}

#[pyfunction]
pub fn set_dropped_files(files: Vec<String>) {
    unsafe {
        if PYXEL_READY {
            let refs: Vec<&str> = files.iter().map(String::as_str).collect();
            pyxel_core::pyxel().set_dropped_files(&refs);
        }
    }
}



// ---------------------------------------------------------------------------
// System functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn quit() {
    unsafe {
        if let Some(env) = ENVIRON_CB {
            env(rust_libretro_sys::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut());
        }
    }
}

// show() — renders one frame and waits (used in scripts without a run loop).
// We cache a no-op update/draw so retro_run() keeps displaying the
// already-rendered frame instead of falling back to the placeholder.
#[pyfunction]
#[allow(static_mut_refs)]
pub fn show() {
    unsafe {
        if !PYXEL_READY { return; }
        Python::with_gil(|py| {
            // Create no-op lambda and cache as update/draw
            let noop = py.eval_bound("lambda: None", None, None).unwrap();
            if PY_UPDATE.is_none() {
                PY_UPDATE = Some(noop.clone().into());
            }
            if PY_DRAW.is_none() {
                PY_DRAW = Some(noop.into());
            }
        });
    }
}

// flip() — advances one frame manually (used instead of pyxel.run()).
// Unsupported in libretro (framing is driven by retro_run()); raises
// instead of no-op'ing so flip()-based main loops fail fast (see below).
#[pyfunction]
pub fn flip() -> PyResult<()> {
    // Previously a silent no-op. Scripts using the flip()-based main loop
    // pattern (`while True: ... pyxel.flip()`, e.g. 99_flip_animation.py)
    // never call back into Rust between flip() calls, so with flip() doing
    // nothing the loop never terminates — it spins forever inside the
    // single py.run_bound() call that runs the script, permanently
    // hanging retro_run() (RetroArch itself freezes, no crash, no error).
    // Raising here instead lets the loop's first flip() call unwind back
    // out with a clear, actionable message instead of a silent hang.
    Err(pyo3::exceptions::PyRuntimeError::new_err(
        "pyxel.flip() is not supported in lr-pyxel (libretro build). \
         Games driven by a `while True: ... pyxel.flip()` main loop can't \
         run under libretro's frame-driven retro_run() model — only \
         pyxel.run(update, draw) is supported here."
    ))
}

// system_wrapper.rs additions
// Window/display settings are no-ops in headless libretro mode

#[pyfunction]
pub fn reset() {
    // In libretro, reset = reload current content
    // For now this is a no-op; future: trigger RETRO_ENVIRONMENT_RESET
}

/// Load a content file from the frontend browser.
/// Called by frontend.py when the user selects a file.
/// Pass None or empty string to return to the frontend.
#[pyfunction]
#[pyo3(signature = (path=None))]
pub fn load_content(path: Option<&str>) -> PyResult<()> {
    unsafe {
        crate::PENDING_CONTENT = Some(path.unwrap_or("").to_string());
    }
    Ok(())
}

#[pyfunction]
pub fn title(_title: &str) {
    // no-op in headless mode
}

#[pyfunction]
#[pyo3(signature = (data, scale, colkey=None))]
pub fn icon(data: Vec<String>, scale: u32, colkey: Option<u8>) {
    let _ = (data, scale, colkey);
    // no-op in headless mode
}

#[pyfunction]
pub fn perf_monitor(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn integer_scale(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn screen_mode(_scr: u32) {
    // no-op in headless mode
}

#[pyfunction]
pub fn fullscreen(_enabled: bool) {
    // no-op in headless mode
}

#[pyfunction]
pub fn resize(width: u32, height: u32) -> PyResult<()> {
    if width == 0 || height == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "width and height must be greater than 0",
        ));
    }
    unsafe {
        GAME_W = width;
        GAME_H = height;
        // Actually resize the physical canvas — previously this only
        // updated our own GAME_W/GAME_H tracking and RetroArch's
        // reported geometry, leaving the real screen canvas at
        // whatever size it was before. Same class of bug as init()'s
        // missing set_screen_size() call (v0.11.3), just in a
        // different function we hadn't touched with that fix. Any
        // script calling pyxel.resize() at runtime would have its
        // rendering silently truncated to the old size.
        if PYXEL_READY {
            pyxel_core::pyxel().set_screen_size(width, height);
        }
        // pyxel.width/height are frozen as static module attributes by
        // init() (see there for why) — once set, a static attribute
        // takes precedence over __getattr__ permanently, so without
        // this, pyxel.width/height would report the size at launch
        // forever, never reflecting a runtime resize() call, even
        // though pyxel_core's own width()/height() (and everything
        // reading them internally) update correctly.
        Python::with_gil(|py| {
            if let Ok(m) = py.import_bound("pyxel") {
                let _ = m.setattr("width",  width);
                let _ = m.setattr("height", height);
            }
        });
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   width,
                base_height:  height,
                // Was hardcoded to 256, stale since v0.11.3 raised the
                // actual ceiling (SCREEN_W/SCREEN_H and every other
                // max_width/max_height site) to 1024.
                max_width:    1024,
                max_height:   1024,
                aspect_ratio: width as f32 / height as f32,
            };
            env(37, &geometry as *const _ as *mut c_void);
        }
    }
    Ok(())
}

// -- module registration -----------------------------------------------------

// ---------------------------------------------------------------------------
// Image bank wrapper (pyxel.images[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Image wrapper (image_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Image", unsendable)]
pub struct PyImage {
    image: pyxel_core::RcImage,
}

impl PyImage {
    pub fn rc(&self) -> &pyxel_core::RcImage {
        &self.image
    }
}

#[pymethods]
impl PyImage {
    #[new]
    pub fn new(width: u32, height: u32) -> Self {
        // Previously this ignored width/height and always aliased the
        // fixed bank-0 image, so every pyxel.Image(w, h) instance shared
        // the same underlying canvas (see problem: dynamic Image creation
        // not supported, e.g. 11_offscreen.py). pyxel_core::Image::new()
        // allocates a genuinely independent image, not tied to any of the
        // fixed NUM_IMAGES banks.
        PyImage { image: pyxel_core::Image::new(width, height) }
    }

    #[staticmethod]
    #[pyo3(signature = (filename, include_colors=None, incl_colors=None))]
    pub fn from_image(filename: &str, include_colors: Option<bool>, incl_colors: Option<bool>) -> PyResult<Self> {
        // incl_colors is the deprecated alias for include_colors.
        if incl_colors.is_some() {
            warn_deprecated_once("Image.from_image.incl_colors", "incl_colors (use include_colors instead)");
        }
        let include_colors = include_colors.or(incl_colors);
        // pyxel_core::Image::load() does NOT resize its target canvas — it
        // just blits the loaded file into the existing (fixed-size) canvas,
        // clipped to its bounds. pyxel_core::Image::from_image() is the
        // correct function here: it creates a brand new image already
        // sized to match the loaded file.
        unsafe {
            if !PYXEL_READY {
                return Err(pyo3::exceptions::PyRuntimeError::new_err("Pyxel not initialized"));
            }
        }
        let image = pyxel_core::Image::from_image(filename, include_colors)
            .map_err(pyo3::exceptions::PyException::new_err)?;
        Ok(PyImage { image })
    }

    #[getter]
    pub fn width(&self) -> u32 {
        unsafe { (&*self.rc().get()).width() }
    }

    #[getter]
    pub fn height(&self) -> u32 {
        unsafe { (&*self.rc().get()).height() }
    }

    // data_ptr() -> ctypes array of c_uint8
    // Returns the image's raw pixel buffer as a live ctypes view (no
    // copy) — one byte per pixel, palette index 0-255, row-major,
    // width*height bytes total. Used by scripts that need bulk pixel
    // access faster than pset()/pget() one at a time (e.g. procedural
    // noise effects).
    pub fn data_ptr(&self, py: Python) -> PyResult<PyObject> {
        unsafe {
            let img = &mut *self.rc().get();
            let size = (img.width() * img.height()) as usize;
            let ptr = img.data_ptr() as usize;
            let ctypes = py.import_bound("ctypes")?;
            let c_uint8 = ctypes.getattr("c_uint8")?;
            let array_type = c_uint8.call_method1("__mul__", (size,))?;
            let array = array_type.call_method1("from_address", (ptr,))?;
            Ok(array.into())
        }
    }

    pub fn set(&self, x: i32, y: i32, data: Vec<String>) {
        unsafe {
            let img = &mut *self.rc().get();
            let refs: Vec<&str> = data.iter().map(String::as_str).collect();
            img.set(x, y, &refs);
        }
    }

    #[pyo3(signature = (x, y, filename, include_colors=None, incl_colors=None))]
    pub fn load(&self, x: i32, y: i32, filename: &str, include_colors: Option<bool>, incl_colors: Option<bool>) -> PyResult<()> {
        if incl_colors.is_some() {
            warn_deprecated_once("Image.load.incl_colors", "incl_colors (use include_colors instead)");
        }
        let include_colors = include_colors.or(incl_colors);
        unsafe {
            let img = &mut *self.rc().get();
            img.load(x, y, filename, include_colors)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn save(&self, filename: &str, scale: u32) -> PyResult<()> {
        unsafe {
            let img = &*self.rc().get();
            img.save(filename, scale)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (x=None, y=None, w=None, h=None))]
    pub fn clip(&self, x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) -> PyResult<()> {
        unsafe {
            let img = &mut *self.rc().get();
            if let (Some(x), Some(y), Some(w), Some(h)) = (x, y, w, h) {
                img.set_clip_rect(x, y, w, h);
            } else {
                img.reset_clip_rect();
            }
        }
        Ok(())
    }

    #[pyo3(signature = (x=None, y=None))]
    pub fn camera(&self, x: Option<f32>, y: Option<f32>) -> PyResult<()> {
        unsafe {
            let img = &mut *self.rc().get();
            if let (Some(x), Some(y)) = (x, y) {
                img.set_camera(x, y);
            } else {
                img.reset_camera();
            }
        }
        Ok(())
    }

    #[pyo3(signature = (col1=None, col2=None))]
    pub fn pal(&self, col1: Option<u8>, col2: Option<u8>) -> PyResult<()> {
        unsafe {
            let img = &mut *self.rc().get();
            if let (Some(c1), Some(c2)) = (col1, col2) {
                img.map_color(c1, c2);
            } else {
                img.reset_color_map();
            }
        }
        Ok(())
    }

    pub fn dither(&self, alpha: f32) {
        unsafe { (&mut *self.rc().get()).set_dithering(alpha); }
    }

    pub fn cls(&self, col: u8) {
        unsafe { (&mut *self.rc().get()).clear(col); }
    }

    pub fn pget(&self, x: f32, y: f32) -> u8 {
        unsafe { (&*self.rc().get()).pixel(x, y) }
    }

    pub fn pset(&self, x: f32, y: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).set_pixel(x, y, col); }
    }

    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_line(x1, y1, x2, y2, col); }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_rect(x, y, w, h, col); }
    }

    pub fn rectb(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_rect_border(x, y, w, h, col); }
    }

    pub fn circ(&self, x: f32, y: f32, r: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_circle(x, y, r, col); }
    }

    pub fn circb(&self, x: f32, y: f32, r: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_circle_border(x, y, r, col); }
    }

    pub fn elli(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_ellipse(x, y, w, h, col); }
    }

    pub fn ellib(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_ellipse_border(x, y, w, h, col); }
    }

    pub fn tri(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_triangle(x1, y1, x2, y2, x3, y3, col); }
    }

    pub fn trib(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_triangle_border(x1, y1, x2, y2, x3, y3, col); }
    }

    pub fn fill(&self, x: f32, y: f32, col: u8) {
        unsafe { (&mut *self.rc().get()).flood_fill(x, y, col); }
    }

    #[pyo3(signature = (x, y, img, u, v, w, h, colkey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn blt(&self, x: f32, y: f32, img: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32,
           colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = img.extract::<u32>() {
                pyxel_core::images().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid image index"))?
            } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
                pyimg.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "img must be an image bank index (int) or an Image instance"
                ));
            };
            let dst = &mut *self.rc().get();
            dst.draw_image(x, y, &src, u, v, w, h, colkey, rotate, scale);
        }
        Ok(())
    }

    // Same int-or-object handling as Tilemap.blt() above.
    #[pyo3(signature = (x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn bltm(&self, x: f32, y: f32, tm: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32,
            colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = tm.extract::<u32>() {
                pyxel_core::tilemaps().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid tilemap index"))?
            } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
                pytm.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "tm must be a tilemap bank index (int) or a Tilemap instance"
                ));
            };
            let dst = &mut *self.rc().get();
            dst.draw_tilemap(x, y, &src, u, v, w, h, colkey, rotate, scale);
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, s, col))]
    pub fn text(&self, x: f32, y: f32, s: &str, col: u8) {
        unsafe { (&mut *self.rc().get()).draw_text(x, y, s, col, None); }
    }
}

#[pyclass(name = "ImageList")]
pub struct PyImageList;

#[pymethods]
impl PyImageList {
    pub fn __len__(&self) -> usize {
        pyxel_core::images().len()
    }

    pub fn __getitem__(&self, idx: i64) -> PyResult<PyImage> {
        let images = pyxel_core::images();
        let len = images.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("image bank index {idx} out of range")
            ));
        }
        Ok(PyImage { image: images[i as usize].clone() })
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyImage>) -> PyResult<()> {
        let images = pyxel_core::images();
        let len = images.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("image bank index {idx} out of range")
            ));
        }
        // Replace the bank's underlying image outright (Rc clone: shares
        // the same canvas as `val`), rather than copying pixels into the
        // existing fixed-size bank canvas. The old pixel-copy approach
        // silently clipped anything wider/taller than the bank's current
        // size (e.g. loading a >256px-wide tileset PNG into image bank 0
        // would truncate everything past x=256/y=256).
        pyxel_core::images()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let images = pyxel_core::images();
        let len = images.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("image index out of range"));
        }
        images.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Image; {}]", pyxel_core::images().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::images().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyImage> {
        pyxel_core::images().iter().rev()
            .map(|rc| PyImage { image: rc.clone() })
            .collect()
    }

    // Unlike Channel/Tone's append()/insert() (which copy values into a
    // fresh bank slot), Image banks are swappable resources — append()/
    // insert() push the given Image's own Rc directly, same as
    // __setitem__ above. No default size exists to fall back on, so
    // (unlike Channel()/Tone()) an Image argument is required here.
    pub fn append(&self, image: pyo3::PyRef<PyImage>) {
        pyxel_core::images().push(image.rc().clone());
    }

    pub fn insert(&self, idx: usize, image: pyo3::PyRef<PyImage>) {
        let images = pyxel_core::images();
        let idx = idx.min(images.len());
        images.insert(idx, image.rc().clone());
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyImage>>) {
        for img in &items {
            pyxel_core::images().push(img.rc().clone());
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PyImage>>) {
        self.extend(items);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PyImage> {
        let images = pyxel_core::images();
        let len = images.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty images list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PyImage { image: images.remove(i as usize) })
    }

    pub fn clear(&self) {
        pyxel_core::images().clear();
    }
}

// ---------------------------------------------------------------------------
// Sound bank wrapper (pyxel.sounds[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Sound wrapper (sound_wrapper.rs)
// ---------------------------------------------------------------------------

// Same Bank/Owned split as ChannelRef/ToneRef — needed so that
// PySoundList.pop() can return a detached, standalone Sound (no longer
// tied to any bank position). The #[new] standalone constructor
// (pyxel.Sound()) itself is a separate, not-yet-done item (upstream
// documents it but it's still missing here) — this enum groundwork
// just makes pop() implementable now; adding #[new] later is a small
// follow-up once this exists.
enum SoundRef {
    Bank(usize),
    Owned(pyxel_core::RcSound),
}

#[pyclass(name = "Sound", unsendable)]
pub struct PySound {
    sound_ref: SoundRef,
}

impl PySound {
    pub fn rc(&self) -> &pyxel_core::RcSound {
        match &self.sound_ref {
            SoundRef::Bank(i) => &pyxel_core::sounds()[*i],
            SoundRef::Owned(rc) => rc,
        }
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
        PySound { sound_ref: SoundRef::Owned(pyxel_core::Sound::new()) }
    }

    #[getter]
    pub fn notes(&self) -> Vec<pyxel_core::SoundNote> {
        unsafe { (&*self.rc().get()).notes.clone() }
    }

    #[setter(notes)]
    pub fn set_notes_list(&self, notes: Vec<pyxel_core::SoundNote>) {
        unsafe { (&mut *self.rc().get()).notes = notes; }
    }

    #[getter]
    pub fn tones(&self) -> Vec<pyxel_core::SoundTone> {
        unsafe { (&*self.rc().get()).tones.clone() }
    }

    #[setter(tones)]
    pub fn set_tones_list(&self, tones: Vec<pyxel_core::SoundTone>) {
        unsafe { (&mut *self.rc().get()).tones = tones; }
    }

    #[getter]
    pub fn volumes(&self) -> Vec<pyxel_core::SoundVolume> {
        unsafe { (&*self.rc().get()).volumes.clone() }
    }

    #[setter(volumes)]
    pub fn set_volumes_list(&self, volumes: Vec<pyxel_core::SoundVolume>) {
        unsafe { (&mut *self.rc().get()).volumes = volumes; }
    }

    #[getter]
    pub fn effects(&self) -> Vec<pyxel_core::SoundEffect> {
        unsafe { (&*self.rc().get()).effects.clone() }
    }

    #[setter(effects)]
    pub fn set_effects_list(&self, effects: Vec<pyxel_core::SoundEffect>) {
        unsafe { (&mut *self.rc().get()).effects = effects; }
    }

    #[getter]
    pub fn speed(&self) -> pyxel_core::SoundSpeed {
        unsafe { (&*self.rc().get()).speed }
    }

    #[setter]
    pub fn set_speed(&self, speed: pyxel_core::SoundSpeed) {
        unsafe { (&mut *self.rc().get()).speed = speed; }
    }

    pub fn set(&self, notes: &str, tones: &str, volumes: &str, effects: &str, speed: pyxel_core::SoundSpeed) -> PyResult<()> {
        unsafe {
            (&mut *self.rc().get())
                .set(notes, tones, volumes, effects, speed)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_notes(&self, notes: &str) -> PyResult<()> {
        unsafe {
            (&mut *self.rc().get()).set_notes(notes)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_tones(&self, tones: &str) -> PyResult<()> {
        unsafe {
            (&mut *self.rc().get()).set_tones(tones)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_volumes(&self, volumes: &str) -> PyResult<()> {
        unsafe {
            (&mut *self.rc().get()).set_volumes(volumes)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn set_effects(&self, effects: &str) -> PyResult<()> {
        unsafe {
            (&mut *self.rc().get()).set_effects(effects)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (code=None))]
    pub fn mml(&self, code: Option<&str>) -> PyResult<()> {
        unsafe {
            let snd = &mut *self.rc().get();
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
                                        "the old MML syntax (use the current mml() syntax instead)"
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
        warn_deprecated_once("Sound.old_mml", "Sound.old_mml() (use Sound.mml() instead)");
        unsafe {
            (&mut *self.rc().get()).old_mml(code)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (filename=None))]
    pub fn pcm(&self, filename: Option<&str>) -> PyResult<()> {
        unsafe {
            let snd = &mut *self.rc().get();
            match filename {
                None => { snd.clear_pcm(); Ok(()) }
                Some(f) => snd.load_pcm(f).map_err(pyo3::exceptions::PyException::new_err)
            }
        }
    }

    pub fn total_sec(&self) -> Option<f32> {
        unsafe { (&*self.rc().get()).total_seconds() }
    }
}

#[pyclass(name = "SoundList")]
pub struct PySoundList;

#[pymethods]
impl PySoundList {
    pub fn __len__(&self) -> usize {
        pyxel_core::sounds().len()
    }

    pub fn __getitem__(&self, idx: i64) -> PyResult<PySound> {
        let len = pyxel_core::sounds().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("sound bank index {idx} out of range")
            ));
        }
        Ok(PySound { sound_ref: SoundRef::Bank(i as usize) })
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PySound>) -> PyResult<()> {
        let len = pyxel_core::sounds().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("sound bank index {idx} out of range")
            ));
        }
        pyxel_core::sounds()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let sounds = pyxel_core::sounds();
        let len = sounds.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("sound index out of range"));
        }
        sounds.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Sound; {}]", pyxel_core::sounds().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::sounds().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PySound> {
        pyxel_core::sounds().iter().rev()
            .map(|rc| PySound { sound_ref: SoundRef::Owned(rc.clone()) })
            .collect()
    }

    pub fn append(&self, sound: pyo3::PyRef<PySound>) {
        pyxel_core::sounds().push(sound.rc().clone());
    }

    pub fn insert(&self, idx: usize, sound: pyo3::PyRef<PySound>) {
        let sounds = pyxel_core::sounds();
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
        let sounds = pyxel_core::sounds();
        let len = sounds.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty sounds list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PySound { sound_ref: SoundRef::Owned(sounds.remove(i as usize)) })
    }

    pub fn clear(&self) {
        pyxel_core::sounds().clear();
    }
}

// ---------------------------------------------------------------------------
// Tilemap bank wrapper (pyxel.tilemaps[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tilemap wrapper (tilemap_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Tilemap", unsendable)]
pub struct PyTilemap {
    tilemap: pyxel_core::RcTilemap,
}

impl PyTilemap {
    pub fn rc(&self) -> &pyxel_core::RcTilemap {
        &self.tilemap
    }
}

#[pymethods]
impl PyTilemap {
    // Missing entirely until now — pyxel.Tilemap(width, height, img) is
    // a documented upstream constructor for a standalone tilemap, not
    // just a bank-indexed pyxel.tilemaps[i]. img can be an image bank
    // index (int) or an Image instance, matching ImageSource's two
    // variants.
    #[new]
    pub fn new(width: u32, height: u32, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<Self> {
        let imgsrc = if let Ok(idx) = img.extract::<u32>() {
            pyxel_core::ImageSource::Index(idx)
        } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
            pyxel_core::ImageSource::Image(pyimg.rc().clone())
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "img must be an image bank index (int) or an Image instance"
            ));
        };
        Ok(PyTilemap { tilemap: pyxel_core::Tilemap::new(width, height, imgsrc) })
    }

    #[staticmethod]
    pub fn from_tmx(filename: &str, layer: u32) -> PyResult<Self> {
        // Same fix as Image::from_image: Tilemap::load() does NOT resize
        // its target canvas, it only blits into the existing (fixed-size)
        // one. pyxel_core::Tilemap::from_tmx() is the correct function
        // here — it creates a brand new tilemap already sized to match
        // the loaded TMX layer.
        unsafe {
            if !PYXEL_READY {
                return Err(pyo3::exceptions::PyRuntimeError::new_err("Pyxel not initialized"));
            }
        }
        let tilemap = pyxel_core::Tilemap::from_tmx(filename, layer)
            .map_err(pyo3::exceptions::PyException::new_err)?;
        Ok(PyTilemap { tilemap })
    }

    #[getter]
    pub fn width(&self) -> u32 {
        unsafe { (&*self.rc().get()).width() }
    }

    #[getter]
    pub fn height(&self) -> u32 {
        unsafe { (&*self.rc().get()).height() }
    }

    // imgsrc can be read/written as either a bank index (int) or an
    // Image instance — previously only the int form worked in either
    // direction. Confirmed via upstream's own tests (test_imgsrc_read_write_image,
    // test_tilemap_wrong_imgsrc_type) that this bidirectional support
    // is expected, not an int-only design.
    #[getter]
    pub fn imgsrc(&self, py: pyo3::Python) -> pyo3::PyObject {
        use pyo3::IntoPy;
        unsafe {
            match &(&*self.rc().get()).imgsrc {
                pyxel_core::ImageSource::Index(i) => (*i).into_py(py),
                pyxel_core::ImageSource::Image(rc) => {
                    pyo3::Py::new(py, PyImage { image: rc.clone() })
                        .map(|obj| obj.into_py(py))
                        .unwrap_or_else(|_| py.None())
                }
            }
        }
    }

    #[setter]
    pub fn set_imgsrc(&self, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        unsafe {
            let imgsrc = if let Ok(idx) = img.extract::<u32>() {
                pyxel_core::ImageSource::Index(idx)
            } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
                pyxel_core::ImageSource::Image(pyimg.rc().clone())
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "imgsrc must be an image bank index (int) or an Image instance"
                ));
            };
            (&mut *self.rc().get()).imgsrc = imgsrc;
        }
        Ok(())
    }

    // Deprecated: refimg (alias for imgsrc, raw pass-through — returns
    // whatever form imgsrc itself would: an int if set as an index, or
    // an Image if set as an instance). getter/setter use distinct keys,
    // same reasoning as Tone.waveform/noise above.
    #[getter]
    pub fn refimg(&self, py: pyo3::Python) -> pyo3::PyObject {
        warn_deprecated_once("Tilemap.refimg.get", "Tilemap.refimg (use Tilemap.imgsrc instead)");
        self.imgsrc(py)
    }

    #[setter]
    pub fn set_refimg(&self, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        warn_deprecated_once("Tilemap.refimg.set", "Tilemap.refimg (use Tilemap.imgsrc instead)");
        self.set_imgsrc(img)
    }

    // Deprecated: image. Unlike refimg, this ALWAYS resolves to a real
    // Image instance even if the tilemap's imgsrc was set as a plain
    // bank index — image's original (pre-imgsrc) semantics were always
    // "the actual Image content", never a raw index. Confirmed via
    // upstream's own test (constructs Tilemap(8, 8, 0) — an int index —
    // then asserts isinstance(tm.image, pyxel.Image)).
    #[getter]
    pub fn image(&self, py: pyo3::Python) -> pyo3::PyObject {
        use pyo3::IntoPy;
        warn_deprecated_once("Tilemap.image.get", "Tilemap.image (use Tilemap.imgsrc instead)");
        unsafe {
            match &(&*self.rc().get()).imgsrc {
                pyxel_core::ImageSource::Index(i) => {
                    let rc = pyxel_core::images()[*i as usize].clone();
                    pyo3::Py::new(py, PyImage { image: rc })
                        .map(|obj| obj.into_py(py))
                        .unwrap_or_else(|_| py.None())
                }
                pyxel_core::ImageSource::Image(rc) => {
                    pyo3::Py::new(py, PyImage { image: rc.clone() })
                        .map(|obj| obj.into_py(py))
                        .unwrap_or_else(|_| py.None())
                }
            }
        }
    }

    #[setter]
    pub fn set_image(&self, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        warn_deprecated_once("Tilemap.image.set", "Tilemap.image (use Tilemap.imgsrc instead)");
        self.set_imgsrc(img)
    }

    pub fn set(&self, x: i32, y: i32, data: Vec<String>) {
        unsafe {
            let tm = &mut *self.rc().get();
            let refs: Vec<&str> = data.iter().map(String::as_str).collect();
            tm.set(x, y, &refs);
        }
    }

    pub fn load(&self, x: i32, y: i32, filename: &str, layer: u32) -> PyResult<()> {
        unsafe {
            let tm = &mut *self.rc().get();
            tm.load(x, y, filename, layer)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (x=None, y=None, w=None, h=None))]
    pub fn clip(&self, x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) -> PyResult<()> {
        unsafe {
            let tm = &mut *self.rc().get();
            if let (Some(x), Some(y), Some(w), Some(h)) = (x, y, w, h) {
                tm.set_clip_rect(x, y, w, h);
            } else {
                tm.reset_clip_rect();
            }
        }
        Ok(())
    }

    #[pyo3(signature = (x=None, y=None))]
    pub fn camera(&self, x: Option<f32>, y: Option<f32>) -> PyResult<()> {
        unsafe {
            let tm = &mut *self.rc().get();
            if let (Some(x), Some(y)) = (x, y) {
                tm.set_camera(x, y);
            } else {
                tm.reset_camera();
            }
        }
        Ok(())
    }

    pub fn cls(&self, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).clear(tile); }
    }

    pub fn pget(&self, x: f32, y: f32) -> (u16, u16) {
        unsafe { (&*self.rc().get()).tile(x, y) }
    }

    pub fn pset(&self, x: f32, y: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).set_tile(x, y, tile); }
    }

    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_line(x1, y1, x2, y2, tile); }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_rect(x, y, w, h, tile); }
    }

    pub fn rectb(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_rect_border(x, y, w, h, tile); }
    }

    pub fn circ(&self, x: f32, y: f32, r: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_circle(x, y, r, tile); }
    }

    pub fn circb(&self, x: f32, y: f32, r: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_circle_border(x, y, r, tile); }
    }

    pub fn elli(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_ellipse(x, y, w, h, tile); }
    }

    pub fn ellib(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_ellipse_border(x, y, w, h, tile); }
    }

    pub fn tri(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_triangle(x1, y1, x2, y2, x3, y3, tile); }
    }

    pub fn trib(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).draw_triangle_border(x1, y1, x2, y2, x3, y3, tile); }
    }

    pub fn fill(&self, x: f32, y: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().get()).flood_fill(x, y, tile); }
    }

    pub fn collide(&self, x: f32, y: f32, w: f32, h: f32, dx: f32, dy: f32, walls: Vec<(u16, u16)>) -> (f32, f32) {
        unsafe { (&*self.rc().get()).collide(x, y, w, h, dx, dy, &walls) }
    }

    // tm can be a bank index (int) or a Tilemap instance — previously
    // only the index form was supported here, unlike Image.blt() (and
    // the top-level bltm()/PyImage.bltm(), which already handled both).
    #[pyo3(signature = (x, y, tm, u, v, w, h, tilekey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn blt(&self, x: f32, y: f32, tm: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32,
           tilekey: Option<(u16, u16)>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = tm.extract::<u32>() {
                pyxel_core::tilemaps().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid tilemap index"))?
            } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
                pytm.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "tm must be a tilemap bank index (int) or a Tilemap instance"
                ));
            };
            let dst = &mut *self.rc().get();
            dst.draw_tilemap(x, y, &src, u, v, w, h, tilekey, rotate, scale);
        }
        Ok(())
    }
}

#[pyclass(name = "TilemapList")]
pub struct PyTilemapList;

#[pymethods]
impl PyTilemapList {
    pub fn __len__(&self) -> usize {
        pyxel_core::tilemaps().len()
    }

    pub fn __getitem__(&self, idx: i64) -> PyResult<PyTilemap> {
        let len = pyxel_core::tilemaps().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tilemap bank index {idx} out of range")
            ));
        }
        Ok(PyTilemap { tilemap: pyxel_core::tilemaps()[i as usize].clone() })
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyTilemap>) -> PyResult<()> {
        let len = pyxel_core::tilemaps().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tilemap bank index {idx} out of range")
            ));
        }
        // Same fix as ImageList::__setitem__: replace the bank outright
        // instead of copying tiles into the existing fixed-size canvas,
        // which silently truncated maps larger than the current bank size.
        pyxel_core::tilemaps()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let tilemaps = pyxel_core::tilemaps();
        let len = tilemaps.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("tilemap index out of range"));
        }
        tilemaps.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Tilemap; {}]", pyxel_core::tilemaps().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::tilemaps().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyTilemap> {
        pyxel_core::tilemaps().iter().rev()
            .map(|rc| PyTilemap { tilemap: rc.clone() })
            .collect()
    }

    pub fn append(&self, tilemap: pyo3::PyRef<PyTilemap>) {
        pyxel_core::tilemaps().push(tilemap.rc().clone());
    }

    pub fn insert(&self, idx: usize, tilemap: pyo3::PyRef<PyTilemap>) {
        let tilemaps = pyxel_core::tilemaps();
        let idx = idx.min(tilemaps.len());
        tilemaps.insert(idx, tilemap.rc().clone());
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyTilemap>>) {
        for t in &items {
            pyxel_core::tilemaps().push(t.rc().clone());
        }
    }

    pub fn __iadd__(&self, items: Vec<pyo3::PyRef<PyTilemap>>) {
        self.extend(items);
    }

    #[pyo3(signature = (idx=None))]
    pub fn pop(&self, idx: Option<i64>) -> PyResult<PyTilemap> {
        let tilemaps = pyxel_core::tilemaps();
        let len = tilemaps.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty tilemaps list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PyTilemap { tilemap: tilemaps.remove(i as usize) })
    }

    pub fn clear(&self) {
        pyxel_core::tilemaps().clear();
    }
}

// ---------------------------------------------------------------------------
// Font wrapper (font_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Font")]
pub struct PyFont {
    inner: pyxel_core::RcFont,
}

// RcFont is Rc<UnsafeCell<Font>> which is not Send by default.
// We are single-threaded in the libretro context so this is safe.
unsafe impl Send for PyFont {}

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
        unsafe { (&mut *self.inner.get()).text_width(s) }
    }
}

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
enum ChannelRef {
    Bank(usize),
    Owned(pyxel_core::RcChannel),
}

#[pyclass(name = "Channel", unsendable)]
pub struct PyChannel {
    channel_ref: ChannelRef,
}

impl PyChannel {
    pub fn rc(&self) -> &pyxel_core::RcChannel {
        match &self.channel_ref {
            ChannelRef::Bank(i) => &pyxel_core::channels()[*i],
            ChannelRef::Owned(rc) => rc,
        }
    }
}

#[pymethods]
impl PyChannel {
    #[new]
    pub fn new() -> Self {
        PyChannel { channel_ref: ChannelRef::Owned(pyxel_core::Channel::new()) }
    }

    #[getter]
    pub fn gain(&self) -> pyxel_core::ChannelGain {
        unsafe { (&*self.rc().get()).gain }
    }

    #[setter]
    pub fn set_gain(&self, gain: pyxel_core::ChannelGain) {
        unsafe { (&mut *self.rc().get()).gain = gain; }
    }

    #[getter]
    pub fn detune(&self) -> pyxel_core::ChannelDetune {
        unsafe { (&*self.rc().get()).detune }
    }

    #[setter]
    pub fn set_detune(&self, detune: pyxel_core::ChannelDetune) {
        unsafe { (&mut *self.rc().get()).detune = detune; }
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
                warn_deprecated_once("Channel.play.tick", "Channel.play()'s tick argument (use sec instead)");
            }
            let sec = tick.map(|t| t / 120.0).or(sec);
            let channel = &mut *self.rc().get();
            if let Ok(idx) = snd.extract::<u32>() {
                let sound = pyxel_core::sounds().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid sound index"))?;
                channel.play_sound(sound, sec, should_loop, should_resume);
            } else if let Ok(seq) = snd.extract::<Vec<u32>>() {
                let pyxel_sounds = pyxel_core::sounds();
                let mut sounds = Vec::with_capacity(seq.len());
                for idx in seq {
                    sounds.push(
                        pyxel_sounds.get(idx as usize).cloned()
                            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid sound index"))?
                    );
                }
                channel.play(sounds, sec, should_loop, should_resume);
            } else if let Ok(mml) = snd.extract::<String>() {
                channel.play_mml(&mml, sec, should_loop, should_resume)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            } else if let Ok(snd_ref) = snd.extract::<pyo3::PyRef<PySound>>() {
                channel.play(vec![snd_ref.rc().clone()], sec, should_loop, should_resume);
            } else if let Ok(snd_refs) = snd.extract::<Vec<pyo3::PyRef<PySound>>>() {
                let sounds: Vec<_> = snd_refs.iter().map(|s| s.rc().clone()).collect();
                channel.play(sounds, sec, should_loop, should_resume);
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "snd must be an int, list of ints, Sound, list of Sounds, or MML string"
                ));
            }
        }
        Ok(())
    }

    pub fn stop(&self) {
        unsafe {
            if PYXEL_READY {
                (&mut *self.rc().get()).stop();
            }
        }
    }

    pub fn play_pos(&self) -> Option<(u32, f32)> {
        unsafe {
            if !PYXEL_READY { return None; }
            (&mut *self.rc().get()).play_position()
        }
    }
}

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

    pub fn __getitem__(&self, idx: i64) -> PyResult<u32> {
        let colors = pyxel_core::colors();
        let len = colors.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("color index out of range"));
        }
        Ok(colors[i as usize])
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(idx_i64) = idx.extract::<i64>() {
            // Single index assignment: colors[i] = 0xRRGGBB
            let v = val.extract::<u32>()?;
            let colors = pyxel_core::colors();
            let len = colors.len() as i64;
            let i = if idx_i64 < 0 { idx_i64 + len } else { idx_i64 };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("color index out of range"));
            }
            colors[i as usize] = v;
        } else {
            // Slice assignment: colors[:] = [0x.., 0x.., ...]
            let items = val.extract::<Vec<u32>>()?;
            *pyxel_core::colors() = items;
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
        let colors = pyxel_core::colors();
        let idx = idx.min(colors.len());
        colors.insert(idx, val);
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let colors = pyxel_core::colors();
        let len = colors.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("color index out of range"));
        }
        colors.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", pyxel_core::colors())
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
        let colors = pyxel_core::colors();
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

    pub fn __getitem__(&self, idx: i64) -> PyResult<PyChannel> {
        let len = pyxel_core::channels().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("channel index {idx} out of range")
            ));
        }
        Ok(PyChannel { channel_ref: ChannelRef::Bank(i as usize) })
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
            unsafe {
                let src = &*ch.rc().get();
                let dst = &mut *pyxel_core::channels()[i].get();
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        } else {
            // Slice assignment: channels[:] = [ch0, ch1, ...]
            let items = val.extract::<Vec<pyo3::PyRef<PyChannel>>>()?;
            for (i, ch) in items.iter().enumerate() {
                if i >= pyxel_core::channels().len() { break; }
                unsafe {
                    let src = &*ch.rc().get();
                    let dst = &mut *pyxel_core::channels()[i].get();
                    dst.gain   = src.gain;
                    dst.detune = src.detune;
                }
            }
        }
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let channels = pyxel_core::channels();
        let len = channels.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("channel index out of range"));
        }
        channels.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Channel; {}]", pyxel_core::channels().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::channels().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyChannel> {
        (0..pyxel_core::channels().len()).rev()
            .map(|i| PyChannel { channel_ref: ChannelRef::Bank(i) })
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
            unsafe {
                let src = &*ch.rc().get();
                let dst = &mut *fresh.get();
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
            unsafe {
                let src = &*ch.rc().get();
                let dst = &mut *fresh.get();
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        }
        let channels = pyxel_core::channels();
        let idx = idx.min(channels.len());
        channels.insert(idx, fresh);
    }

    pub fn extend(&self, items: Vec<pyo3::PyRef<PyChannel>>) {
        for ch in &items {
            let fresh = pyxel_core::Channel::new();
            unsafe {
                let src = &*ch.rc().get();
                let dst = &mut *fresh.get();
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
        let channels = pyxel_core::channels();
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
        Ok(PyChannel { channel_ref: ChannelRef::Owned(removed) })
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
            unsafe {
                let src = &*ch.rc().get();
                let dst = &mut *pyxel_core::channels()[i].get();
                dst.gain   = src.gain;
                dst.detune = src.detune;
            }
        }
    }

    pub fn to_list(&self) -> Vec<PyChannel> {
        (0..pyxel_core::channels().len())
            .map(|i| PyChannel { channel_ref: ChannelRef::Bank(i) })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tone wrapper (tone_wrapper.rs)
// ---------------------------------------------------------------------------

// Same fix as PyChannel/ChannelRef above, for the same reason: Tone()
// is documented upstream as "Create a new Tone instance" (a genuinely
// independent object), but previously always hardcoded bank 0 — every
// px.Tone() was secretly a view onto the same shared bank-0 tone.
enum ToneRef {
    Bank(usize),
    Owned(pyxel_core::RcTone),
}

#[pyclass(name = "Tone", unsendable)]
pub struct PyTone {
    tone_ref: ToneRef,
}

impl PyTone {
    pub fn rc(&self) -> &pyxel_core::RcTone {
        match &self.tone_ref {
            ToneRef::Bank(i) => &pyxel_core::tones()[*i],
            ToneRef::Owned(rc) => rc,
        }
    }
}

#[pymethods]
impl PyTone {
    #[new]
    pub fn new() -> Self {
        PyTone { tone_ref: ToneRef::Owned(pyxel_core::Tone::new()) }
    }

    #[getter]
    pub fn mode(&self) -> u32 {
        unsafe { (&*self.rc().get()).mode.into() }
    }

    #[setter]
    pub fn set_mode(&self, mode: u32) {
        unsafe { (&mut *self.rc().get()).mode = pyxel_core::ToneMode::from(mode); }
    }

    #[getter]
    pub fn sample_bits(&self) -> u32 {
        unsafe { (&*self.rc().get()).sample_bits }
    }

    #[setter]
    pub fn set_sample_bits(&self, sample_bits: u32) {
        unsafe { (&mut *self.rc().get()).sample_bits = sample_bits; }
    }

    #[getter]
    pub fn gain(&self) -> pyxel_core::ToneGain {
        unsafe { (&*self.rc().get()).gain }
    }

    #[setter]
    pub fn set_gain(&self, gain: pyxel_core::ToneGain) {
        unsafe { (&mut *self.rc().get()).gain = gain; }
    }

    #[getter]
    pub fn wavetable(&self) -> Vec<pyxel_core::ToneSample> {
        unsafe { (&*self.rc().get()).wavetable.clone() }
    }

    #[setter]
    pub fn set_wavetable(&self, wavetable: Vec<pyxel_core::ToneSample>) {
        unsafe { (&mut *self.rc().get()).wavetable = wavetable; }
    }

    // Deprecated: waveform (alias for wavetable). getter/setter use
    // distinct keys — upstream tests each as an independently "first
    // time this session" warning, in separate test functions, so a
    // single shared key (where whichever ran first consumed the only
    // warning) made the second one silently stop firing.
    #[getter]
    pub fn waveform(&self) -> Vec<pyxel_core::ToneSample> {
        warn_deprecated_once("Tone.waveform.get", "Tone.waveform (use Tone.wavetable instead)");
        self.wavetable()
    }

    #[setter]
    pub fn set_waveform(&self, waveform: Vec<pyxel_core::ToneSample>) {
        warn_deprecated_once("Tone.waveform.set", "Tone.waveform (use Tone.wavetable instead)");
        self.set_wavetable(waveform);
    }

    // Deprecated: noise (alias for mode). Same getter/setter key split
    // as waveform above.
    #[getter]
    pub fn noise(&self) -> u32 {
        warn_deprecated_once("Tone.noise.get", "Tone.noise (use Tone.mode instead)");
        self.mode()
    }

    #[setter]
    pub fn set_noise(&self, mode: u32) {
        warn_deprecated_once("Tone.noise.set", "Tone.noise (use Tone.mode instead)");
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
        unsafe {
            let src = &*src.rc().get();
            let dst = &mut *fresh.get();
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

    pub fn __getitem__(&self, idx: i64) -> PyResult<PyTone> {
        let len = pyxel_core::tones().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tone bank index {idx} out of range")
            ));
        }
        Ok(PyTone { tone_ref: ToneRef::Bank(i as usize) })
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
            unsafe {
                let src = &*tone.rc().get();
                let dst = &mut *pyxel_core::tones()[i].get();
                dst.mode        = src.mode;
                dst.sample_bits = src.sample_bits;
                dst.gain        = src.gain;
                dst.wavetable   = src.wavetable.clone();
            }
        } else {
            // Slice assignment: tones[:] = [t0, t1, ...]
            let items = val.extract::<Vec<pyo3::PyRef<PyTone>>>()?;
            for (i, tone) in items.iter().enumerate() {
                if i >= pyxel_core::tones().len() { break; }
                unsafe {
                    let src = &*tone.rc().get();
                    let dst = &mut *pyxel_core::tones()[i].get();
                    dst.mode        = src.mode;
                    dst.sample_bits = src.sample_bits;
                    dst.gain        = src.gain;
                    dst.wavetable   = src.wavetable.clone();
                }
            }
        }
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let tones = pyxel_core::tones();
        let len = tones.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("tone index out of range"));
        }
        tones.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Tone; {}]", pyxel_core::tones().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::tones().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyTone> {
        (0..pyxel_core::tones().len()).rev()
            .map(|i| PyTone { tone_ref: ToneRef::Bank(i) })
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
        let tones = pyxel_core::tones();
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
        let tones = pyxel_core::tones();
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
        Ok(PyTone { tone_ref: ToneRef::Owned(removed) })
    }

    pub fn clear(&self) {
        pyxel_core::tones().clear();
    }
}

// ---------------------------------------------------------------------------
// Music bank wrapper (pyxel.musics[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Music wrapper (music_wrapper.rs)
// ---------------------------------------------------------------------------

// Same reasoning as SoundRef above.
enum MusicRef {
    Bank(usize),
    Owned(pyxel_core::RcMusic),
}

#[pyclass(name = "Music", unsendable)]
pub struct PyMusic {
    music_ref: MusicRef,
}

impl PyMusic {
    pub fn rc(&self) -> &pyxel_core::RcMusic {
        match &self.music_ref {
            MusicRef::Bank(i) => &pyxel_core::musics()[*i],
            MusicRef::Owned(rc) => rc,
        }
    }
}

#[pymethods]
impl PyMusic {
    // Same reasoning as PySound::new() above.
    #[new]
    pub fn new() -> Self {
        PyMusic { music_ref: MusicRef::Owned(pyxel_core::Music::new()) }
    }

    #[getter]
    pub fn seqs(&self) -> Vec<Vec<u32>> {
        unsafe { (&*self.rc().get()).seqs.clone() }
    }

    #[setter]
    pub fn set_seqs(&self, seqs: Vec<Vec<u32>>) {
        unsafe { (&mut *self.rc().get()).set(&seqs); }
    }

    // Deprecated: snds_list (alias for seqs, getter only)
    #[getter]
    pub fn snds_list(&self) -> Vec<Vec<u32>> {
        warn_deprecated_once("Music.snds_list", "Music.snds_list (use Music.seqs instead)");
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
        unsafe { (&mut *self.rc().get()).set(&seqs); }
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

    pub fn __getitem__(&self, idx: i64) -> PyResult<PyMusic> {
        let len = pyxel_core::musics().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("music bank index {idx} out of range")
            ));
        }
        Ok(PyMusic { music_ref: MusicRef::Bank(i as usize) })
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyMusic>) -> PyResult<()> {
        let len = pyxel_core::musics().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("music bank index {idx} out of range")
            ));
        }
        pyxel_core::musics()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: i64) -> PyResult<()> {
        let musics = pyxel_core::musics();
        let len = musics.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("music index out of range"));
        }
        musics.remove(i as usize);
        Ok(())
    }

    pub fn __repr__(&self) -> String {
        format!("[Music; {}]", pyxel_core::musics().len())
    }

    pub fn __bool__(&self) -> bool {
        !pyxel_core::musics().is_empty()
    }

    pub fn __reversed__(&self) -> Vec<PyMusic> {
        pyxel_core::musics().iter().rev()
            .map(|rc| PyMusic { music_ref: MusicRef::Owned(rc.clone()) })
            .collect()
    }

    pub fn append(&self, music: pyo3::PyRef<PyMusic>) {
        pyxel_core::musics().push(music.rc().clone());
    }

    pub fn insert(&self, idx: usize, music: pyo3::PyRef<PyMusic>) {
        let musics = pyxel_core::musics();
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
        let musics = pyxel_core::musics();
        let len = musics.len() as i64;
        if len == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop from empty musics list"));
        }
        let i = idx.unwrap_or(-1);
        let i = if i < 0 { i + len } else { i };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("pop index out of range"));
        }
        Ok(PyMusic { music_ref: MusicRef::Owned(musics.remove(i as usize)) })
    }

    pub fn clear(&self) {
        pyxel_core::musics().clear();
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
        "width"       => (*pyxel_core::width()).into_py(py),
        "height"      => (*pyxel_core::height()).into_py(py),
        "frame_count" => unsafe { LR_FRAME_COUNT }.into_py(py),
        // Input
        "mouse_x"     => (*pyxel_core::mouse_x()).into_py(py),
        "mouse_y"     => (*pyxel_core::mouse_y()).into_py(py),
        "mouse_wheel" => (*pyxel_core::mouse_wheel()).into_py(py),
        // Graphics
        "colors"   => PyColors.into_py(py),
        "screen"   => PyImage { image: pyxel_core::screen().clone() }.into_py(py),
        // Missing entirely until now (pyxel.screen was added in
        // v0.11.2, but these two built-in image banks were
        // overlooked) — the mouse cursor sprite and the built-in
        // font glyph atlas, both exposed upstream as plain Image
        // instances alongside pyxel.screen.
        "cursor"   => PyImage { image: pyxel_core::cursor_image().clone() }.into_py(py),
        "font"     => PyImage { image: pyxel_core::font_image().clone() }.into_py(py),
        "images"   => PyImageList.into_py(py),
        "tilemaps" => PyTilemapList.into_py(py),
        // Audio
        "sounds"   => PySoundList.into_py(py),
        "musics"   => PyMusicList.into_py(py),
        "tones"    => PyToneList.into_py(py),
        "channels" => PyChannelList.into_py(py),
        _ => return Err(pyo3::exceptions::PyAttributeError::new_err(
            format!("module 'pyxel' has no attribute '{name}'")
        )),
    };
    Ok(value)
}


#[pymodule]
pub fn pyxel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Drawing
    m.add_function(wrap_pyfunction!(cls,         m)?)?;
    m.add_function(wrap_pyfunction!(rect,        m)?)?;
    m.add_function(wrap_pyfunction!(rectb,       m)?)?;
    m.add_function(wrap_pyfunction!(text,        m)?)?;
    m.add_function(wrap_pyfunction!(pset,        m)?)?;
    m.add_function(wrap_pyfunction!(pget,        m)?)?;
    m.add_function(wrap_pyfunction!(line,        m)?)?;
    m.add_function(wrap_pyfunction!(circ,        m)?)?;
    m.add_function(wrap_pyfunction!(circb,       m)?)?;
    m.add_function(wrap_pyfunction!(elli,        m)?)?;
    m.add_function(wrap_pyfunction!(ellib,       m)?)?;
    m.add_function(wrap_pyfunction!(tri,         m)?)?;
    m.add_function(wrap_pyfunction!(trib,        m)?)?;
    m.add_function(wrap_pyfunction!(fill,        m)?)?;
    m.add_function(wrap_pyfunction!(clip,        m)?)?;
    m.add_function(wrap_pyfunction!(camera,      m)?)?;
    m.add_function(wrap_pyfunction!(pal,         m)?)?;
    m.add_function(wrap_pyfunction!(dither,      m)?)?;
    m.add_function(wrap_pyfunction!(blt,         m)?)?;
    m.add_function(wrap_pyfunction!(bltm,        m)?)?;
    m.add_function(wrap_pyfunction!(blt3d,       m)?)?;
    m.add_function(wrap_pyfunction!(bltm3d,      m)?)?;
    m.add_function(wrap_pyfunction!(image,       m)?)?;
    m.add_function(wrap_pyfunction!(tilemap_fn,  m)?)?;
    m.add_function(wrap_pyfunction!(image_load,  m)?)?;
    m.add_function(wrap_pyfunction!(image_pset,  m)?)?;
    m.add_function(wrap_pyfunction!(load,             m)?)?;
    m.add_function(wrap_pyfunction!(save,             m)?)?;
    m.add_function(wrap_pyfunction!(load_pal,         m)?)?;
    m.add_function(wrap_pyfunction!(save_pal,         m)?)?;
    m.add_function(wrap_pyfunction!(screenshot,       m)?)?;
    m.add_function(wrap_pyfunction!(screencast,       m)?)?;
    m.add_function(wrap_pyfunction!(reset_screencast, m)?)?;
    m.add_function(wrap_pyfunction!(user_data_dir,    m)?)?;
    // Network
    m.add_function(wrap_pyfunction!(download_file, m)?)?;
    m.add_function(wrap_pyfunction!(http_get,      m)?)?;
    // Input
    m.add_function(wrap_pyfunction!(btn,         m)?)?;
    m.add_function(wrap_pyfunction!(btnp,        m)?)?;
    m.add_function(wrap_pyfunction!(btnr,             m)?)?;
    m.add_function(wrap_pyfunction!(btnv,             m)?)?;
    m.add_function(wrap_pyfunction!(mouse,            m)?)?;
    m.add_function(wrap_pyfunction!(set_btn,          m)?)?;
    m.add_function(wrap_pyfunction!(set_btnv,         m)?)?;
    m.add_function(wrap_pyfunction!(set_mouse_pos,    m)?)?;
    m.add_function(wrap_pyfunction!(set_input_text,   m)?)?;
    m.add_function(wrap_pyfunction!(set_dropped_files,m)?)?;
    // Audio
    m.add_function(wrap_pyfunction!(sound_set,   m)?)?;
    m.add_function(wrap_pyfunction!(play,        m)?)?;
    m.add_function(wrap_pyfunction!(gen_bgm,     m)?)?;
    m.add_function(wrap_pyfunction!(playm,       m)?)?;
    m.add_function(wrap_pyfunction!(stop,        m)?)?;
    m.add_function(wrap_pyfunction!(play_pos,    m)?)?;
    m.add_function(wrap_pyfunction!(sound_fn,    m)?)?;
    m.add_function(wrap_pyfunction!(music_fn,    m)?)?;
    m.add_function(wrap_pyfunction!(channel_fn,  m)?)?;
    // Math
    m.add_function(wrap_pyfunction!(ceil,        m)?)?;
    m.add_function(wrap_pyfunction!(floor,       m)?)?;
    m.add_function(wrap_pyfunction!(clamp,       m)?)?;
    m.add_function(wrap_pyfunction!(sgn,         m)?)?;
    m.add_function(wrap_pyfunction!(sqrt,        m)?)?;
    m.add_function(wrap_pyfunction!(sin,         m)?)?;
    m.add_function(wrap_pyfunction!(cos,         m)?)?;
    m.add_function(wrap_pyfunction!(atan2,       m)?)?;
    m.add_function(wrap_pyfunction!(rseed,       m)?)?;
    m.add_function(wrap_pyfunction!(rndi,        m)?)?;
    m.add_function(wrap_pyfunction!(rndf,        m)?)?;
    m.add_function(wrap_pyfunction!(nseed,       m)?)?;
    m.add_function(wrap_pyfunction!(noise,       m)?)?;
    // System (system_wrapper.rs)
    m.add_function(wrap_pyfunction!(quit,         m)?)?;
    m.add_function(wrap_pyfunction!(load_content, m)?)?;
    m.add_function(wrap_pyfunction!(reset,        m)?)?;
    m.add_function(wrap_pyfunction!(show,         m)?)?;
    m.add_function(wrap_pyfunction!(flip,         m)?)?;
    m.add_function(wrap_pyfunction!(title,        m)?)?;
    m.add_function(wrap_pyfunction!(icon,         m)?)?;
    m.add_function(wrap_pyfunction!(perf_monitor, m)?)?;
    m.add_function(wrap_pyfunction!(integer_scale,m)?)?;
    m.add_function(wrap_pyfunction!(screen_mode,  m)?)?;
    m.add_function(wrap_pyfunction!(fullscreen,   m)?)?;
    m.add_function(wrap_pyfunction!(resize,       m)?)?;
    m.add_function(wrap_pyfunction!(init,         m)?)?;
    m.add_function(wrap_pyfunction!(run,          m)?)?;
    // width/height as module attributes
    // Dynamic variables via __getattr__ (variable_wrapper.rs approach)
    // width, height, frame_count, mouse_x/y, colors, images, tilemaps,
    // sounds, musics are all returned dynamically by __getattr__
    m.add_function(wrap_pyfunction!(__getattr__, m)?)?;

    // Constants (constant_wrapper.rs)
    add_module_constants(m)?;

    // Register pyclass types
    m.add_class::<PyImage>()?;
    m.add_class::<PyFont>()?;
    m.add_class::<PyColors>()?;
    m.add_class::<PyImageList>()?;
    m.add_class::<PySound>()?;
    m.add_class::<PySoundList>()?;
    m.add_class::<PyMusic>()?;
    m.add_class::<PyMusicList>()?;
    m.add_class::<PyTilemap>()?;
    m.add_class::<PyTilemapList>()?;
    m.add_class::<PyChannel>()?;
    m.add_class::<PyChannelList>()?;
    m.add_class::<PyTone>()?;
    m.add_class::<PyToneList>()?;

    Ok(())
}


