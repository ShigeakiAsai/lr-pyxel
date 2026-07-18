//! tilemap_wrapper_lr.rs — Tilemap wrapper (pyxel.Tilemap,
//! pyxel.tilemaps[n]).
//!
//! Tracks upstream's tilemap_wrapper.rs closely. References PyImage
//! (a Tilemap's backing image can be an Image bank index or an Image
//! instance) — a genuine circular reference with image_wrapper_lr.rs,
//! resolved via crate-root re-exports (see lib.rs) and `use crate::*;`
//! below, same as image_wrapper_lr.rs does for its own PyTilemap
//! references.

use pyo3::prelude::*;
use crate::*;

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

    // Constructor for other modules — see PySound::from_rc() above
    // for the reasoning (same pattern, different type).
    pub(crate) fn from_rc(tilemap: pyxel_core::RcTilemap) -> Self {
        PyTilemap { tilemap }
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
                "img must be int or Image"
            ));
        };
        // Uses try_new() (a Result), not new() — new() calls
        // .expect("tilemap dimensions are too large") internally and
        // panics on oversized dimensions instead of returning an
        // error. A Rust panic raised from PyO3-called code crosses the
        // retro_run() FFI boundary and aborts the whole process rather
        // than raising a catchable Python exception. Matches upstream's
        // own official binding (tilemap_wrapper.rs), which uses
        // try_new() + map_err(PyValueError::new_err) for the same
        // reason (see also PyImage::new() above).
        pyxel_core::Tilemap::try_new(width, height, imgsrc)
            .map(|tilemap| PyTilemap { tilemap })
            .map_err(pyo3::exceptions::PyValueError::new_err)
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
        unsafe { (&*self.rc().as_ptr()).width() }
    }

    #[getter]
    pub fn height(&self) -> u32 {
        unsafe { (&*self.rc().as_ptr()).height() }
    }

    // data_ptr() -> ctypes array of c_uint16
    // Returns the tilemap's raw tile buffer as a live ctypes view (no
    // copy) — two u16 values per tile (tile_id, color_modifier),
    // row-major, width*height*2 u16 entries total (row stride =
    // width*2). Same pattern as Image::data_ptr() above, mirrored
    // here for Tilemap — confirmed via upstream's own tests
    // (test_data_ptr_read/_write/_row_stride) that this is expected
    // to exist, not test-only scaffolding. Used by scripts that need
    // bulk tile access faster than pset()/pget() one at a time.
    pub fn data_ptr(&self, py: Python) -> PyResult<Py<PyAny>> {
        unsafe {
            let tm = &mut *self.rc().as_ptr();
            let size = (tm.width() * tm.height() * 2) as usize;
            let ptr = tm.data_ptr() as usize;
            let ctypes = py.import("ctypes")?;
            let c_uint16 = ctypes.getattr("c_uint16")?;
            let array_type = c_uint16.call_method1("__mul__", (size,))?;
            let array = array_type.call_method1("from_address", (ptr,))?;
            Ok(array.into())
        }
    }

    // imgsrc can be read/written as either a bank index (int) or an
    // Image instance — previously only the int form worked in either
    // direction. Confirmed via upstream's own tests (test_imgsrc_read_write_image,
    // test_tilemap_wrong_imgsrc_type) that this bidirectional support
    // is expected, not an int-only design.
    #[getter]
    pub fn imgsrc(&self, py: pyo3::Python) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        unsafe {
            Ok(match &(&*self.rc().as_ptr()).imgsrc {
                pyxel_core::ImageSource::Index(i) => (*i).into_pyobject(py)?.into_any().unbind(),
                pyxel_core::ImageSource::Image(rc) => {
                    pyo3::Py::new(py, PyImage::from_rc(rc.clone()))
                        .map(|obj| obj.into_any())
                        .unwrap_or_else(|_| py.None())
                }
            })
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
                    "imgsrc must be int or Image"
                ));
            };
            (&mut *self.rc().as_ptr()).imgsrc = imgsrc;
        }
        Ok(())
    }

    // Deprecated: refimg (alias for imgsrc, raw pass-through — returns
    // whatever form imgsrc itself would: an int if set as an index, or
    // an Image if set as an instance). getter/setter use distinct keys,
    // same reasoning as Tone.waveform/noise above.
    #[getter]
    pub fn refimg(&self, py: pyo3::Python) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        warn_deprecated_once("Tilemap.refimg.get", "Tilemap.refimg is deprecated. Use Tilemap.imgsrc instead.");
        self.imgsrc(py)
    }

    #[setter]
    pub fn set_refimg(&self, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        warn_deprecated_once("Tilemap.refimg.set", "Tilemap.refimg is deprecated. Use Tilemap.imgsrc instead.");
        self.set_imgsrc(img)
    }

    // Deprecated: image. Unlike refimg, this ALWAYS resolves to a real
    // Image instance even if the tilemap's imgsrc was set as a plain
    // bank index — image's original (pre-imgsrc) semantics were always
    // "the actual Image content", never a raw index. Confirmed via
    // upstream's own test (constructs Tilemap(8, 8, 0) — an int index —
    // then asserts isinstance(tm.image, pyxel.Image)).
    #[getter]
    pub fn image(&self, py: pyo3::Python) -> pyo3::Py<pyo3::PyAny> {
        warn_deprecated_once("Tilemap.image.get", "Tilemap.image is deprecated. Use Tilemap.imgsrc instead.");
        unsafe {
            match &(&*self.rc().as_ptr()).imgsrc {
                pyxel_core::ImageSource::Index(i) => {
                    let rc = pyxel_core::images()[*i as usize].clone();
                    pyo3::Py::new(py, PyImage::from_rc(rc))
                        .map(|obj| obj.into_any())
                        .unwrap_or_else(|_| py.None())
                }
                pyxel_core::ImageSource::Image(rc) => {
                    pyo3::Py::new(py, PyImage::from_rc(rc.clone()))
                        .map(|obj| obj.into_any())
                        .unwrap_or_else(|_| py.None())
                }
            }
        }
    }

    #[setter]
    pub fn set_image(&self, img: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        warn_deprecated_once("Tilemap.image.set", "Tilemap.image is deprecated. Use Tilemap.imgsrc instead.");
        self.set_imgsrc(img)
    }

    // Same manual-validation pattern as Image.set() above, for the
    // same reason: PyO3's automatic Vec<String> extraction produces a
    // different, version-dependent auto-generated message than
    // upstream's own binding. Not currently covered by an upstream
    // test the way Image.set() is, but fixed here too for consistency
    // rather than leaving an identical latent gap unaddressed.
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
            let tm = &mut *self.rc().as_ptr();
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            tm.set(x, y, &refs).map_err(pyo3::exceptions::PyValueError::new_err)?;
        }
        Ok(())
    }

    pub fn load(&self, x: i32, y: i32, filename: &str, layer: u32) -> PyResult<()> {
        unsafe {
            let tm = &mut *self.rc().as_ptr();
            tm.load(x, y, filename, layer)
                .map_err(pyo3::exceptions::PyException::new_err)
        }
    }

    #[pyo3(signature = (x=None, y=None, w=None, h=None))]
    pub fn clip(&self, x: Option<f32>, y: Option<f32>, w: Option<f32>, h: Option<f32>) -> PyResult<()> {
        unsafe {
            let tm = &mut *self.rc().as_ptr();
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
            let tm = &mut *self.rc().as_ptr();
            if let (Some(x), Some(y)) = (x, y) {
                tm.set_camera(x, y);
            } else {
                tm.reset_camera();
            }
        }
        Ok(())
    }

    pub fn cls(&self, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).clear(tile); }
    }

    pub fn pget(&self, x: f32, y: f32) -> (u16, u16) {
        unsafe { (&*self.rc().as_ptr()).tile(x, y) }
    }

    pub fn pset(&self, x: f32, y: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).set_tile(x, y, tile); }
    }

    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_line(x1, y1, x2, y2, tile); }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_rect(x, y, w, h, tile); }
    }

    pub fn rectb(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_rect_border(x, y, w, h, tile); }
    }

    pub fn circ(&self, x: f32, y: f32, r: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_circle(x, y, r, tile); }
    }

    pub fn circb(&self, x: f32, y: f32, r: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_circle_border(x, y, r, tile); }
    }

    pub fn elli(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_ellipse(x, y, w, h, tile); }
    }

    pub fn ellib(&self, x: f32, y: f32, w: f32, h: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_ellipse_border(x, y, w, h, tile); }
    }

    pub fn tri(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_triangle(x1, y1, x2, y2, x3, y3, tile); }
    }

    pub fn trib(&self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).draw_triangle_border(x1, y1, x2, y2, x3, y3, tile); }
    }

    pub fn fill(&self, x: f32, y: f32, tile: (u16, u16)) {
        unsafe { (&mut *self.rc().as_ptr()).flood_fill(x, y, tile); }
    }

    pub fn collide(&self, x: f32, y: f32, w: f32, h: f32, dx: f32, dy: f32, walls: Vec<(u16, u16)>) -> (f32, f32) {
        unsafe { (&*self.rc().as_ptr()).collide(x, y, w, h, dx, dy, &walls) }
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
                    .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("tm must be a valid tilemap index"))?
            } else if let Ok(pytm) = tm.extract::<pyo3::PyRef<PyTilemap>>() {
                pytm.rc().clone()
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "tm must be int or Tilemap"
                ));
            };
            let dst = &mut *self.rc().as_ptr();
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

    pub fn __getitem__(&self, py: pyo3::Python, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        let tilemaps = pyxel_core::tilemaps();
        let len = tilemaps.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    String::from("list index out of range")
                ));
            }
            return Ok(PyTilemap { tilemap: tilemaps[i as usize].clone() }.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            let mut result = Vec::new();
            let mut i = indices.start;
            if indices.step > 0 {
                while i < indices.stop {
                    result.push(PyTilemap { tilemap: tilemaps[i as usize].clone() });
                    i += indices.step;
                }
            } else if indices.step < 0 {
                while i > indices.stop {
                    result.push(PyTilemap { tilemap: tilemaps[i as usize].clone() });
                    i += indices.step;
                }
            }
            return Ok(result.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("tilemap index must be an int or slice"))
    }

    pub fn __setitem__(&self, idx: i64, val: pyo3::PyRef<PyTilemap>) -> PyResult<()> {
        let len = pyxel_core::tilemaps().len() as i64;
        let i = if idx < 0 { idx + len } else { idx };
        if i < 0 || i >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                String::from("list index out of range")
            ));
        }
        // Same fix as ImageList::__setitem__: replace the bank outright
        // instead of copying tiles into the existing fixed-size canvas,
        // which silently truncated maps larger than the current bank size.
        pyxel_core::tilemaps()[i as usize] = val.rc().clone();
        Ok(())
    }

    pub fn __delitem__(&self, idx: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
        let mut tilemaps = pyxel_core::tilemaps();
        let len = tilemaps.len() as i64;
        if let Ok(i) = idx.extract::<i64>() {
            let i = if i < 0 { i + len } else { i };
            if i < 0 || i >= len {
                return Err(pyo3::exceptions::PyIndexError::new_err("tilemap index out of range"));
            }
            tilemaps.remove(i as usize);
            return Ok(());
        }
        if let Ok(slice) = idx.cast::<pyo3::types::PySlice>() {
            let indices = slice.indices(len as isize)?;
            if indices.step != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "extended slices (step != 1) are not supported for tilemaps deletion"
                ));
            }
            let start = indices.start.max(0) as usize;
            let stop = indices.stop.max(indices.start) as usize;
            tilemaps.drain(start..stop);
            return Ok(());
        }
        Err(pyo3::exceptions::PyTypeError::new_err("tilemap index must be an int or slice"))
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
        let mut tilemaps = pyxel_core::tilemaps();
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
        let mut tilemaps = pyxel_core::tilemaps();
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

