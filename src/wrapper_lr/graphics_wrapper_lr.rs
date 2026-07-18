//! graphics_wrapper_lr.rs — Drawing functions (cls/rect/pset/text/
//! blt/bltm/blt3d/bltm3d/line/circ/elli/tri/fill/dither, and the
//! deprecated image()/tilemap() bank accessors).
//!
//! Tracks upstream's graphics_wrapper.rs closely.

use pyo3::prelude::*;
use crate::*;

// -- drawing -----------------------------------------------------------------

#[pyfunction]
pub fn cls(col: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().clear(col);
        }
    }
}

#[pyfunction]
pub fn rect(x: f32, y: f32, w: f32, h: f32, col: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().draw_rect(x, y, w, h, col);
        }
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, s, col, font=None))]
pub fn text(x: f32, y: f32, s: &str, col: u8, font: Option<pyo3::PyRef<PyFont>>) {
    unsafe {
        if PYXEL_READY {
            let font_ref = font.as_ref().map(|f| f.rc());
            pyxel_core::pyxel().draw_text(x, y, s, col, font_ref);
        }
    }
}

#[pyfunction]
pub fn pset(x: f32, y: f32, col: u8) {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().set_pixel(x, y, col);
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
            validate_index!(idx, pyxel_core::images().len(), "img", "image");
            pyxel_core::pyxel().draw_image(x, y, idx, u, v, w, h, colkey, rotate, scale);
        } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
            let src = pyimg.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.as_ptr();
            dst.draw_image(x, y, &src, u, v, w, h, colkey, rotate, scale);
        } else {
            // Previously fell through silently here (no else branch at
            // all) — an invalid img argument no-op'd instead of raising,
            // found via upstream's own test (pyxel.blt(0, 0,
            // "not_an_image", ...) expected a TypeError but nothing was
            // raised at all).
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "img must be int or Image"
            ));
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
            validate_index!(idx, pyxel_core::tilemaps().len(), "tm", "tilemap");
            pyxel_core::pyxel().draw_tilemap(x, y, idx, u, v, w, h, colkey, rotate, scale);
        } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
            let src = pytm.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.as_ptr();
            dst.draw_tilemap(x, y, &src, u, v, w, h, colkey, rotate, scale);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "tm must be int or Tilemap"
            ));
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
            validate_index!(idx, pyxel_core::images().len(), "img", "image");
            pyxel_core::pyxel().draw_image_3d(x, y, w, h, idx, pos, rot, fov, colkey);
        } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
            let src = pyimg.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.as_ptr();
            dst.draw_image_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "img must be int or Image"
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
            validate_index!(idx, pyxel_core::tilemaps().len(), "tm", "tilemap");
            pyxel_core::pyxel().draw_tilemap_3d(x, y, w, h, idx, pos, rot, fov, colkey);
        } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
            let src = pytm.rc().clone();
            let screen = pyxel_core::screen();
            let dst = &mut *screen.as_ptr();
            dst.draw_tilemap_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "tm must be int or Tilemap"
            ));
        }
    }
    Ok(())
}

// Deprecated: pyxel.image(n) → use pyxel.images[n] instead
#[pyfunction]
pub fn image(img: u32) -> PyResult<PyImage> {
    warn_deprecated_once("image()", "pyxel.image(img) is deprecated. Use pyxel.images[img] instead.");
    validate_index!(img, pyxel_core::NUM_IMAGES as usize, "img", "image");
    Ok(PyImage::from_rc(pyxel_core::images()[img as usize].clone()))
}

// Deprecated: pyxel.tilemap(n) → use pyxel.tilemaps[n] instead
#[pyfunction]
#[pyo3(name = "tilemap")]
pub fn tilemap_fn(tm: u32) -> PyResult<PyTilemap> {
    warn_deprecated_once("tilemap()", "pyxel.tilemap(tm) is deprecated. Use pyxel.tilemaps[tm] instead.");
    validate_index!(tm, pyxel_core::NUM_TILEMAPS as usize, "tm", "tilemap");
    Ok(PyTilemap::from_rc(pyxel_core::tilemaps()[tm as usize].clone()))
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
        let image: &mut pyxel_core::Image = &mut *rc_image.as_ptr();
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
        let image: &mut pyxel_core::Image = &mut *rc_image.as_ptr();
        image.set_pixel(x, y, color);
        Ok(())
    }
}


// ---------------------------------------------------------------------------
// Drawing functions (remaining)
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_line(x1, y1, x2, y2, col); } }
}
#[pyfunction]
pub fn rectb(x: f32, y: f32, w: f32, h: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_rect_border(x, y, w, h, col); } }
}
#[pyfunction]
pub fn circ(x: f32, y: f32, r: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle(x, y, r, col); } }
}
#[pyfunction]
pub fn circb(x: f32, y: f32, r: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_circle_border(x, y, r, col); } }
}
#[pyfunction]
pub fn elli(x: f32, y: f32, w: f32, h: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse(x, y, w, h, col); } }
}
#[pyfunction]
pub fn ellib(x: f32, y: f32, w: f32, h: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_ellipse_border(x, y, w, h, col); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn tri(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle(x1, y1, x2, y2, x3, y3, col); } }
}
#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn trib(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().draw_triangle_border(x1, y1, x2, y2, x3, y3, col); } }
}
#[pyfunction]
pub fn fill(x: f32, y: f32, col: u8) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().flood_fill(x, y, col); } }
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None, w=None, h=None))]
pub fn clip(x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        match (x, y, w, h) {
            (Some(x), Some(y), Some(w), Some(h)) => pyxel_core::pyxel().set_clip_rect(x, y, w, h),
            (None, None, None, None) => pyxel_core::pyxel().reset_clip_rect(),
            // Silently resetting on a partial argument set (e.g.
            // clip(10, 20), forgetting w/h) previously masked what was
            // almost certainly a script typo — now raises the same
            // way upstream does.
            _ => return Err(pyo3::exceptions::PyTypeError::new_err(
                "clip() takes 0 or 4 arguments"
            )),
        }
    }
    Ok(())
}
#[pyfunction]
#[pyo3(signature = (x=None, y=None))]
pub fn camera(x: Option<f32>, y: Option<f32>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        match (x, y) {
            (Some(x), Some(y)) => pyxel_core::pyxel().set_camera(x, y),
            (None, None) => pyxel_core::pyxel().reset_camera(),
            _ => return Err(pyo3::exceptions::PyTypeError::new_err(
                "camera() takes 0 or 2 arguments"
            )),
        }
    }
    Ok(())
}
#[pyfunction]
#[pyo3(signature = (col1=None, col2=None))]
pub fn pal(col1: Option<u8>, col2: Option<u8>) -> PyResult<()> {
    unsafe {
        if !PYXEL_READY { return Ok(()); }
        match (col1, col2) {
            (Some(c1), Some(c2)) => pyxel_core::pyxel().map_color(c1, c2),
            (None, None) => pyxel_core::pyxel().reset_color_map(),
            _ => return Err(pyo3::exceptions::PyTypeError::new_err(
                "pal() takes 0 or 2 arguments"
            )),
        }
    }
    Ok(())
}
#[pyfunction]
pub fn dither(alpha: f32) {
    unsafe { if PYXEL_READY { pyxel_core::pyxel().set_dithering(alpha); } }
}

