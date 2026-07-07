//! wrappers.rs — Pyxel API wrappers (Python ↔ pyxel_core)

use std::cmp::Ordering;
use pyo3::prelude::*;
use crate::*;

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
#[pyfunction]
#[pyo3(signature = (x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None))]
#[allow(clippy::too_many_arguments)]
pub fn bltm(x: f32, y: f32, tm: u32, u: f32, v: f32, w: f32, h: f32, colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_tilemap(x, y, tm, u, v, w, h, colkey, rotate, scale);
        }
    }
}

// blt3d(x, y, w, h, img, pos, rot, fov=None, colkey=None)
#[pyfunction]
#[pyo3(signature = (x, y, w, h, img, pos, rot, fov=None, colkey=None))]
#[allow(clippy::too_many_arguments)]
pub fn blt3d(
    x: f32, y: f32, w: f32, h: f32,
    img: u32,
    pos: (f32, f32, f32),
    rot: (f32, f32, f32),
    fov: Option<f32>,
    colkey: Option<u8>,
) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_image_3d(x, y, w, h, img, pos, rot, fov, colkey);
        }
    }
}

// bltm3d(x, y, w, h, tm, pos, rot, fov=None, colkey=None)
#[pyfunction]
#[pyo3(signature = (x, y, w, h, tm, pos, rot, fov=None, colkey=None))]
#[allow(clippy::too_many_arguments)]
pub fn bltm3d(
    x: f32, y: f32, w: f32, h: f32,
    tm: u32,
    pos: (f32, f32, f32),
    rot: (f32, f32, f32),
    fov: Option<f32>,
    colkey: Option<u8>,
) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_tilemap_3d(x, y, w, h, tm, pos, rot, fov, colkey);
        }
    }
}

// Deprecated: pyxel.image(n) → use pyxel.images[n] instead
#[pyfunction]
pub fn image(img: u32) -> PyResult<PyImage> {
    if img as usize >= pyxel_core::NUM_IMAGES as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid image index"));
    }
    Ok(PyImage { image: pyxel_core::images()[img as usize].clone() })
}

// Deprecated: pyxel.tilemap(n) → use pyxel.tilemaps[n] instead
#[pyfunction]
#[pyo3(name = "tilemap")]
pub fn tilemap_fn(tm: u32) -> PyResult<PyTilemap> {
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
// snd can be a single sound index (u32) or a list of sound indices (Vec<u32>)
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
        let sec = tick.map(|t| t / 120.0).or(sec);
        if let Ok(idx) = snd.extract::<u32>() {
            pyxel_core::pyxel().play_sound(ch, idx, sec, should_loop, should_resume);
        } else if let Ok(seq) = snd.extract::<Vec<u32>>() {
            pyxel_core::pyxel().play(ch, &seq, sec, should_loop, should_resume);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "snd must be an int or list of ints"
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
    if snd as usize >= pyxel_core::NUM_SOUNDS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid sound index"));
    }
    Ok(PySound { bank: snd as usize })
}

// Deprecated: pyxel.music(n) → use pyxel.musics[n]
#[pyfunction]
#[pyo3(name = "music")]
pub fn music_fn(msc: u32) -> PyResult<PyMusic> {
    if msc as usize >= pyxel_core::NUM_MUSICS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid music index"));
    }
    Ok(PyMusic { bank: msc as usize })
}

// Deprecated: pyxel.channel(n) → use pyxel.channels[n]
#[pyfunction]
#[pyo3(name = "channel")]
pub fn channel_fn(ch: u32) -> PyResult<PyChannel> {
    if ch as usize >= pyxel_core::NUM_CHANNELS as usize {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid channel index"));
    }
    Ok(PyChannel { bank: ch as usize })
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


// init() is a no-op: Pyxel is already initialized by retro_init()
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
                base_width:   w,
                base_height:  h,
                max_width:    256,
                max_height:   256,
                aspect_ratio: w as f32 / h as f32,
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
    // Real mouse input isn't implemented yet (retro_run() only polls
    // RETRO_DEVICE_JOYPAD, never RETRO_DEVICE_MOUSE), so mouse_x/mouse_y
    // never move from their initial value. pyxel_core's own flip_screen()
    // still draws the cursor sprite at (mouse_x, mouse_y) whenever
    // visibility is on, which would show a static, non-functional cursor
    // stuck in place. Force it hidden regardless of what the script
    // requests until mouse input is actually wired up.
    let _ = visible;
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_mouse_visible(false); } }
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
        if let Some(env) = ENVIRON_CB {
            let geometry = rust_libretro_sys::retro_game_geometry {
                base_width:   width,
                base_height:  height,
                max_width:    256,
                max_height:   256,
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
    #[pyo3(signature = (filename, include_colors=None))]
    pub fn from_image(filename: &str, include_colors: Option<bool>) -> PyResult<Self> {
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

    pub fn set(&self, x: i32, y: i32, data: Vec<String>) {
        unsafe {
            let img = &mut *self.rc().get();
            let refs: Vec<&str> = data.iter().map(String::as_str).collect();
            img.set(x, y, &refs);
        }
    }

    #[pyo3(signature = (x, y, filename, include_colors=None))]
    pub fn load(&self, x: i32, y: i32, filename: &str, include_colors: Option<bool>) -> PyResult<()> {
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

    #[pyo3(signature = (x, y, tm, u, v, w, h, colkey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn bltm(&self, x: f32, y: f32, tm: u32, u: f32, v: f32, w: f32, h: f32,
            colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = pyxel_core::tilemaps().get(tm as usize)
                .cloned()
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid tilemap index"))?;
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
    pub fn __getitem__(&self, idx: usize) -> PyResult<PyImage> {
        if idx >= pyxel_core::NUM_IMAGES as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("image bank index {idx} out of range")
            ));
        }
        Ok(PyImage { image: pyxel_core::images()[idx].clone() })
    }

    pub fn __setitem__(&self, idx: usize, val: pyo3::PyRef<PyImage>) -> PyResult<()> {
        if idx >= pyxel_core::NUM_IMAGES as usize {
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
        pyxel_core::images()[idx] = val.rc().clone();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sound bank wrapper (pyxel.sounds[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Sound wrapper (sound_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Sound")]
pub struct PySound {
    bank: usize,
}

impl PySound {
    pub fn rc(&self) -> &pyxel_core::RcSound {
        &pyxel_core::sounds()[self.bank]
    }
}

#[pymethods]
impl PySound {
    #[getter]
    pub fn notes(&self) -> Vec<pyxel_core::SoundNote> {
        unsafe { (&*self.rc().get()).notes.clone() }
    }

    #[setter]
    pub fn set_notes_list(&self, notes: Vec<pyxel_core::SoundNote>) {
        unsafe { (&mut *self.rc().get()).notes = notes; }
    }

    #[getter]
    pub fn tones(&self) -> Vec<pyxel_core::SoundTone> {
        unsafe { (&*self.rc().get()).tones.clone() }
    }

    #[setter]
    pub fn set_tones_list(&self, tones: Vec<pyxel_core::SoundTone>) {
        unsafe { (&mut *self.rc().get()).tones = tones; }
    }

    #[getter]
    pub fn volumes(&self) -> Vec<pyxel_core::SoundVolume> {
        unsafe { (&*self.rc().get()).volumes.clone() }
    }

    #[setter]
    pub fn set_volumes_list(&self, volumes: Vec<pyxel_core::SoundVolume>) {
        unsafe { (&mut *self.rc().get()).volumes = volumes; }
    }

    #[getter]
    pub fn effects(&self) -> Vec<pyxel_core::SoundEffect> {
        unsafe { (&*self.rc().get()).effects.clone() }
    }

    #[setter]
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
                Some(c) => snd.set_mml(c).map_err(pyo3::exceptions::PyException::new_err)
            }
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
    pub fn __getitem__(&self, idx: usize) -> PyResult<PySound> {
        if idx >= pyxel_core::NUM_SOUNDS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("sound bank index {idx} out of range")
            ));
        }
        Ok(PySound { bank: idx })
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

    #[getter]
    pub fn imgsrc(&self) -> u32 {
        unsafe {
            match &(&*self.rc().get()).imgsrc {
                pyxel_core::ImageSource::Index(i) => *i,
                _ => 0,
            }
        }
    }

    #[setter]
    pub fn set_imgsrc(&self, idx: u32) {
        unsafe {
            (&mut *self.rc().get()).imgsrc = pyxel_core::ImageSource::Index(idx);
        }
    }

    // Deprecated: refimg
    #[getter]
    pub fn refimg(&self) -> u32 { self.imgsrc() }

    #[setter]
    pub fn set_refimg(&self, idx: u32) { self.set_imgsrc(idx); }

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

    #[pyo3(signature = (x, y, tm, u, v, w, h, tilekey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn blt(&self, x: f32, y: f32, tm: u32, u: f32, v: f32, w: f32, h: f32,
           tilekey: Option<(u16, u16)>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = pyxel_core::tilemaps().get(tm as usize)
                .cloned()
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid tilemap index"))?;
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
    pub fn __getitem__(&self, idx: usize) -> PyResult<PyTilemap> {
        if idx >= pyxel_core::NUM_TILEMAPS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tilemap bank index {idx} out of range")
            ));
        }
        Ok(PyTilemap { tilemap: pyxel_core::tilemaps()[idx].clone() })
    }

    pub fn __setitem__(&self, idx: usize, val: pyo3::PyRef<PyTilemap>) -> PyResult<()> {
        if idx >= pyxel_core::NUM_TILEMAPS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tilemap bank index {idx} out of range")
            ));
        }
        // Same fix as ImageList::__setitem__: replace the bank outright
        // instead of copying tiles into the existing fixed-size canvas,
        // which silently truncated maps larger than the current bank size.
        pyxel_core::tilemaps()[idx] = val.rc().clone();
        Ok(())
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

#[pyclass(name = "Channel")]
pub struct PyChannel {
    bank: usize,
}

impl PyChannel {
    pub fn rc(&self) -> &pyxel_core::RcChannel {
        &pyxel_core::channels()[self.bank]
    }
}

#[pymethods]
impl PyChannel {
    #[new]
    pub fn new() -> Self {
        PyChannel { bank: 0 }
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

    #[pyo3(signature = (snd, sec=None, r#loop=None, resume=None, tick=None))]
    pub fn play(&self, snd: u32, sec: Option<f32>, r#loop: Option<bool>, resume: Option<bool>, tick: Option<f32>) -> PyResult<()> {
        unsafe {
            if !PYXEL_READY { return Ok(()); }
            let sound = pyxel_core::sounds().get(snd as usize)
                .cloned()
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid sound index"))?;
            let ch = &mut *self.rc().get();
            let sec = tick.map(|t| t / 120.0).or(sec);
            ch.play_sound(sound, sec, r#loop.unwrap_or(false), resume.unwrap_or(false));
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

    pub fn __getitem__(&self, idx: usize) -> PyResult<u32> {
        pyxel_core::colors().get(idx).copied()
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("color index out of range"))
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(i) = idx.extract::<usize>() {
            // Single index assignment: colors[i] = 0xRRGGBB
            let v = val.extract::<u32>()?;
            let colors = pyxel_core::colors();
            if i >= colors.len() {
                return Err(pyo3::exceptions::PyIndexError::new_err("color index out of range"));
            }
            colors[i] = v;
        } else {
            // Slice assignment: colors[:] = [0x.., 0x.., ...]
            let items = val.extract::<Vec<u32>>()?;
            *pyxel_core::colors() = items;
        }
        Ok(())
    }
}

#[pyclass(name = "ChannelList")]
pub struct PyChannelList;

#[pymethods]
impl PyChannelList {
    pub fn __getitem__(&self, idx: usize) -> PyResult<PyChannel> {
        if idx >= pyxel_core::NUM_CHANNELS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("channel index {idx} out of range")
            ));
        }
        Ok(PyChannel { bank: idx })
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(i) = idx.extract::<usize>() {
            // Single index assignment: channels[n] = channel
            if i >= pyxel_core::NUM_CHANNELS as usize {
                return Err(pyo3::exceptions::PyIndexError::new_err("channel index out of range"));
            }
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
                if i >= pyxel_core::NUM_CHANNELS as usize { break; }
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
}

// ---------------------------------------------------------------------------
// Tone wrapper (tone_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Tone")]
pub struct PyTone {
    bank: usize,
}

impl PyTone {
    pub fn rc(&self) -> &pyxel_core::RcTone {
        &pyxel_core::tones()[self.bank]
    }
}

#[pymethods]
impl PyTone {
    #[new]
    pub fn new() -> Self {
        PyTone { bank: 0 }
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
}

#[pyclass(name = "ToneList")]
pub struct PyToneList;

#[pymethods]
impl PyToneList {
    pub fn __getitem__(&self, idx: usize) -> PyResult<PyTone> {
        if idx >= pyxel_core::NUM_TONES as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("tone bank index {idx} out of range")
            ));
        }
        Ok(PyTone { bank: idx })
    }

    pub fn __setitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>, val: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        if let Ok(i) = idx.extract::<usize>() {
            if i >= pyxel_core::NUM_TONES as usize {
                return Err(pyo3::exceptions::PyIndexError::new_err("tone index out of range"));
            }
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
                if i >= pyxel_core::NUM_TONES as usize { break; }
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
}

// ---------------------------------------------------------------------------
// Music bank wrapper (pyxel.musics[n])
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Music wrapper (music_wrapper.rs)
// ---------------------------------------------------------------------------

#[pyclass(name = "Music")]
pub struct PyMusic {
    bank: usize,
}

impl PyMusic {
    pub fn rc(&self) -> &pyxel_core::RcMusic {
        &pyxel_core::musics()[self.bank]
    }
}

#[pymethods]
impl PyMusic {
    #[getter]
    pub fn seqs(&self) -> Vec<Vec<u32>> {
        unsafe { (&*self.rc().get()).seqs.clone() }
    }

    #[setter]
    pub fn set_seqs(&self, seqs: Vec<Vec<u32>>) {
        unsafe { (&mut *self.rc().get()).set(&seqs); }
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
    pub fn __getitem__(&self, idx: usize) -> PyResult<PyMusic> {
        if idx >= pyxel_core::NUM_MUSICS as usize {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                format!("music bank index {idx} out of range")
            ));
        }
        Ok(PyMusic { bank: idx })
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


