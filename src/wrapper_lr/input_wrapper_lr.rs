//! input_wrapper_lr.rs — Input functions (btn/btnp/btnr/btnv/mouse/
//! set_btn/set_btnv/set_mouse_pos/set_input_text/set_dropped_files).
//!
//! This is a substantial rework versus upstream's input_wrapper.rs,
//! not a close port: upstream reads real keyboard/mouse/gamepad state
//! via SDL2, while lr-pyxel receives input through libretro's
//! RETRO_DEVICE_KEYBOARD callback and RETROK_* keycodes instead. The
//! actual keycode-value constants (KEY_A, KEY_UP, etc.) are re-
//! exported from pyxel_core directly in lib.rs and registered into
//! the Python module by constant_wrapper_lr.rs, not defined here.

use pyo3::prelude::*;
use crate::*;

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
#[pyo3(signature = (key, hold=None, repeat=None))]
pub fn btnp(key: u32, hold: Option<u32>, repeat: Option<u32>) -> bool {
    unsafe {
        if PYXEL_READY {
            pyxel_core::pyxel().is_button_pressed(key, hold, repeat)
        } else {
            false
        }
    }
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
pub fn set_dropped_files(files: pyo3::Bound<'_, pyo3::PyAny>) -> PyResult<()> {
    // Same manual-validation pattern as Image.set()/Tilemap.set(), for
    // the same reason: PyO3's automatic Vec<String> extraction
    // produces a different, version-dependent auto-generated message
    // than upstream's own binding.
    let items: Vec<String> = files.extract().map_err(|_| {
        let type_name = files.get_type().name()
            .map(|n| n.to_string())
            .unwrap_or_else(|_| "object".to_string());
        pyo3::exceptions::PyTypeError::new_err(format!(
            "'{type_name}' object is not an instance of 'Sequence'"
        ))
    })?;
    unsafe {
        if PYXEL_READY {
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            pyxel_core::pyxel().set_dropped_files(&refs);
        }
    }
    Ok(())
}



