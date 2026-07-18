//! math_wrapper_lr.rs — Math functions (ceil/floor/sqrt/sin/cos/atan2/
//! rseed/rndi/rndf/nseed/clamp/sgn/noise).
//!
//! Tracks upstream's math_wrapper.rs closely; these are thin
//! pass-throughs to pyxel_core::Pyxel's own math methods, with no
//! lr-pyxel-specific behavior beyond ordinary PyO3 0.29 API updates
//! (clamp/sgn's Bound<PyAny> + into_pyobject()?.into_any().unbind()
//! pattern, used to avoid PyO3 auto-prefixing type-conversion errors
//! with "argument 'name': ").

use std::cmp::Ordering;
use pyo3::prelude::*;

#[pyfunction] pub fn ceil(x: f32) -> i32 { pyxel_core::Pyxel::ceil(x) }
#[pyfunction] pub fn floor(x: f32) -> i32 { pyxel_core::Pyxel::floor(x) }
#[pyfunction] pub fn sqrt(x: f32) -> f32 { pyxel_core::Pyxel::sqrt(x) }
#[pyfunction] pub fn sin(deg: f32) -> f32 { pyxel_core::Pyxel::sin(deg) }
#[pyfunction] pub fn cos(deg: f32) -> f32 { pyxel_core::Pyxel::cos(deg) }
#[pyfunction] pub fn atan2(y: f32, x: f32) -> f32 { pyxel_core::Pyxel::atan2(y, x) }
#[pyfunction] pub fn rseed(seed: u32) { pyxel_core::Pyxel::random_seed(seed); }
#[pyfunction] pub fn rndi(a: i32, b: i32) -> i32 { pyxel_core::Pyxel::random_int(a, b) }
#[pyfunction] pub fn rndf(a: f32, b: f32) -> f32 { pyxel_core::Pyxel::random_float(a, b) }
#[pyfunction] pub fn nseed(seed: u32) { pyxel_core::Pyxel::noise_seed(seed); }

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
        return Ok(v.into_pyobject(py)?.into_any().unbind());
    }
    let xf = x.extract::<f64>()?;
    let lf = lower.extract::<f64>()?;
    let uf = upper.extract::<f64>()?;
    let (lo, hi) = if lf < uf { (lf, uf) } else { (uf, lf) };
    Ok(xf.clamp(lo, hi).into_pyobject(py)?.into_any().unbind())
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
        return Ok(v.into_pyobject(py)?.into_any().unbind());
    }
    let xf = x.extract::<f64>()?;
    let v: f64 = match xf.partial_cmp(&0.0) {
        Some(Ordering::Greater) => 1.0,
        Some(Ordering::Less) => -1.0,
        _ => 0.0,
    };
    Ok(v.into_pyobject(py)?.into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (x, y=None, z=None))]
pub fn noise(x: f32, y: Option<f32>, z: Option<f32>) -> f32 {
    pyxel_core::Pyxel::noise(x, y.unwrap_or(0.0), z.unwrap_or(0.0))
}
