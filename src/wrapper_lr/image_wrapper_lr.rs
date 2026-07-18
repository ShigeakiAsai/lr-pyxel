//! image_wrapper_lr.rs — Image wrapper (pyxel.Image, pyxel.images[n]).
//!
//! Tracks upstream's image_wrapper.rs closely for most methods.
//! References PyTilemap (blt-family methods accept either an image
//! bank index or a Tilemap's backing image) — this is a genuine
//! circular reference between image_wrapper_lr.rs and
//! tilemap_wrapper_lr.rs, resolved the same way any two sibling
//! modules reference each other here: both types are re-exported at
//! the crate root (see lib.rs), and both files reach the other via
//! the `use crate::*;` below rather than a direct module-to-module
//! `use`.

use pyo3::prelude::*;
use crate::*;

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

    // Constructor for other modules (e.g. tilemap_wrapper_lr's
    // Tilemap.image/refimg getters, which wrap an existing RcImage in
    // a fresh PyImage instance) — `image` itself stays private so
    // construction goes through here rather than a public field,
    // matching the .rc() read-accessor's own reasoning.
    pub(crate) fn from_rc(image: pyxel_core::RcImage) -> Self {
        PyImage { image }
    }
}

#[pymethods]
impl PyImage {
    #[new]
    pub fn new(width: u32, height: u32) -> PyResult<Self> {
        // Previously this ignored width/height and always aliased the
        // fixed bank-0 image, so every pyxel.Image(w, h) instance shared
        // the same underlying canvas (see problem: dynamic Image creation
        // not supported, e.g. 11_offscreen.py). pyxel_core::Image::new()
        // allocates a genuinely independent image, not tied to any of the
        // fixed NUM_IMAGES banks.
        //
        // Uses try_new() (a Result), not new() — new() calls
        // .expect("image dimensions are too large") internally and
        // panics on oversized dimensions instead of returning an error.
        // A Rust panic raised from PyO3-called code crosses the
        // retro_run() FFI boundary and aborts the whole process rather
        // than raising a catchable Python exception (confirmed via
        // upstream's own test_canvas_constructor_rejects_oversized_dimensions,
        // which does exactly this: pyxel.Image(65536, 65536)). Matches
        // upstream's own official binding (image_wrapper.rs), which
        // uses try_new() + map_err(PyValueError::new_err) for the same
        // reason.
        pyxel_core::Image::try_new(width, height)
            .map(|image| PyImage { image })
            .map_err(pyo3::exceptions::PyValueError::new_err)
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
        unsafe { (&*self.rc().as_ptr()).width() }
    }

    #[getter]
    pub fn height(&self) -> u32 {
        unsafe { (&*self.rc().as_ptr()).height() }
    }

    // data_ptr() -> ctypes array of c_uint8
    // Returns the image's raw pixel buffer as a live ctypes view (no
    // copy) — one byte per pixel, palette index 0-255, row-major,
    // width*height bytes total. Used by scripts that need bulk pixel
    // access faster than pset()/pget() one at a time (e.g. procedural
    // noise effects).
    pub fn data_ptr(&self, py: Python) -> PyResult<Py<PyAny>> {
        unsafe {
            let img = &mut *self.rc().as_ptr();
            let size = (img.width() * img.height()) as usize;
            let ptr = img.data_ptr() as usize;
            let ctypes = py.import("ctypes")?;
            let c_uint8 = ctypes.getattr("c_uint8")?;
            let array_type = c_uint8.call_method1("__mul__", (size,))?;
            let array = array_type.call_method1("from_address", (ptr,))?;
            Ok(array.into())
        }
    }

    // Takes a raw PyAny (not Vec<String> directly) so the wrong-type
    // error matches upstream's exact wording ("'int' object is not an
    // instance of 'Sequence'") rather than PyO3's own auto-generated,
    // version-dependent message ("argument 'data': 'int' object
    // cannot be converted to 'Sequence'") that firing on Vec<String>'s
    // automatic extraction would otherwise produce. Confirmed via
    // upstream's own test_image_set_wrong_data_type.
    pub fn set(&self, x: i32, y: i32, data: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let items: Vec<String> = data.extract().map_err(|_| {
            let type_name = data.get_type().name()
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "object".to_string());
            pyo3::exceptions::PyTypeError::new_err(format!(
                "'{type_name}' object is not an instance of 'Sequence'"
            ))
        })?;
        unsafe {
            let img = &mut *self.rc().as_ptr();
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            img.set(x, y, &refs).map_err(pyo3::exceptions::PyValueError::new_err)?;
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, filename, include_colors=None, incl_colors=None))]
    pub fn load(&self, x: i32, y: i32, filename: &str, include_colors: Option<bool>, incl_colors: Option<bool>) -> PyResult<()> {
        if incl_colors.is_some() {
            warn_deprecated_once("Image.load.incl_colors", "incl_colors (use include_colors instead)");
        }
        let include_colors = include_colors.or(incl_colors);
        unsafe {
            let img = &mut *self.rc().as_ptr();
            img.load(x, y, filename, include_colors)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    pub fn save(&self, filename: &str, scale: u32) -> PyResult<()> {
        unsafe {
            let img = &*self.rc().as_ptr();
            img.save(filename, scale)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (x=None, y=None, w=None, h=None))]
    pub fn clip(&self, x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) -> PyResult<()> {
        unsafe {
            let img = &mut *self.rc().as_ptr();
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
            let img = &mut *self.rc().as_ptr();
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
            let img = &mut *self.rc().as_ptr();
            if let (Some(c1), Some(c2)) = (col1, col2) {
                img.map_color(c1, c2);
            } else {
                img.reset_color_map();
            }
        }
        Ok(())
    }

    pub fn dither(&self, alpha: f32) {
        unsafe { (&mut *self.rc().as_ptr()).set_dithering(alpha); }
    }

    pub fn cls(&self, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).clear(col); }
    }

    pub fn pget(&self, x: f32, y: f32) -> u8 {
        unsafe { (&*self.rc().as_ptr()).pixel(x, y) }
    }

    pub fn pset(&self, x: f32, y: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).set_pixel(x, y, col); }
    }

    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_line(x1, y1, x2, y2, col); }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_rect(x, y, w, h, col); }
    }

    pub fn rectb(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_rect_border(x, y, w, h, col); }
    }

    pub fn circ(&self, x: f32, y: f32, r: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_circle(x, y, r, col); }
    }

    pub fn circb(&self, x: f32, y: f32, r: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_circle_border(x, y, r, col); }
    }

    pub fn elli(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_ellipse(x, y, w, h, col); }
    }

    pub fn ellib(&self, x: f32, y: f32, w: f32, h: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_ellipse_border(x, y, w, h, col); }
    }

    pub fn tri(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_triangle(x1, y1, x2, y2, x3, y3, col); }
    }

    pub fn trib(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_triangle_border(x1, y1, x2, y2, x3, y3, col); }
    }

    pub fn fill(&self, x: f32, y: f32, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).flood_fill(x, y, col); }
    }

    #[pyo3(signature = (x, y, img, u, v, w, h, colkey=None, rotate=None, scale=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn blt(&self, x: f32, y: f32, img: pyo3::Bound<'_, pyo3::PyAny>, u: f32, v: f32, w: f32, h: f32,
           colkey: Option<u8>, rotate: Option<f32>, scale: Option<f32>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = img.extract::<u32>() {
                pyxel_core::images().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("img must be a valid image index"))?
            } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
                pyimg.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "img must be int or Image"
                ));
            };
            let dst = &mut *self.rc().as_ptr();
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
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("tm must be a valid tilemap index"))?
            } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
                pytm.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "tm must be int or Tilemap"
                ));
            };
            let dst = &mut *self.rc().as_ptr();
            dst.draw_tilemap(x, y, &src, u, v, w, h, colkey, rotate, scale);
        }
        Ok(())
    }

    // Missing entirely until now — only the top-level pyxel.blt3d()
    // (which always draws to the screen) existed. Image.blt3d() draws
    // into the calling Image instance itself, confirmed via upstream's
    // own test (draws into a standalone Image, not pyxel.screen).
    #[pyo3(signature = (x, y, w, h, img, pos, rot, fov=None, colkey=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn blt3d(&self, x: f32, y: f32, w: f32, h: f32, img: pyo3::Bound<'_, pyo3::PyAny>,
             pos: (f32, f32, f32), rot: (f32, f32, f32), fov: Option<f32>, colkey: Option<u8>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = img.extract::<u32>() {
                pyxel_core::images().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("img must be a valid image index"))?
            } else if let Ok(pyimg) = img.extract::<pyo3::PyRef<PyImage>>() {
                pyimg.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "img must be int or Image"
                ));
            };
            let dst = &mut *self.rc().as_ptr();
            dst.draw_image_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, w, h, tm, pos, rot, fov=None, colkey=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn bltm3d(&self, x: f32, y: f32, w: f32, h: f32, tm: pyo3::Bound<'_, pyo3::PyAny>,
              pos: (f32, f32, f32), rot: (f32, f32, f32), fov: Option<f32>, colkey: Option<u8>) -> PyResult<()> {
        unsafe {
            let src = if let Ok(idx) = tm.extract::<u32>() {
                pyxel_core::tilemaps().get(idx as usize)
                    .cloned()
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("tm must be a valid tilemap index"))?
            } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
                pytm.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "tm must be int or Tilemap"
                ));
            };
            let dst = &mut *self.rc().as_ptr();
            dst.draw_tilemap_3d(x, y, w, h, &src, pos, rot, fov, colkey);
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, s, col))]
    pub fn text(&self, x: f32, y: f32, s: &str, col: u8) {
        unsafe { (&mut *self.rc().as_ptr()).draw_text(x, y, s, col, None); }
    }
}

#[pyclass(name = "ImageList")]
pub struct PyImageList;

#[pymethods]
impl PyImageList {
    pub fn __len__(&self) -> usize {
        pyxel_core::images().len()
    }

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let images = pyxel_core::images();
        let len = images.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PyImage { image: images[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PyImage { image: images[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PyImage { image: images[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("image index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyImage>) -> PyResult<()> {
        let mut images = pyxel_core::images();
        let len = images.len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                String::from("list index out of range")
            ));
        }
        // Replace the bank's underlying image outright (Rc clone: shares
        // the same canvas as `val`), rather than copying pixels into the
        // existing fixed-size bank canvas. The old pixel-copy approach
        // silently clipped anything wider/taller than the bank's current
        // size (e.g. loading a >256px-wide tileset PNG into image bank 0
        // would truncate everything past x=256/y=256).
        images[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut images = pyxel_core::images();
        let len = images.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("image index out of range"));
            }
            images.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for images deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            images.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("image index must be an int or slice"))
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
        let mut images = pyxel_core::images();
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
        let mut images = pyxel_core::images();
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

