//! pyxel_singleton_lr.rs — intentionally empty.
//!
//! Upstream's pyxel_singleton.rs is a 3-line indirection:
//!
//!     pub fn pyxel() -> std::cell::RefMut<'static, pyxel::Pyxel> {
//!         pyxel::pyxel()
//!     }
//!
//! lr-pyxel has never needed this extra layer — every *_wrapper_lr.rs
//! file just calls `pyxel_core::pyxel()` directly wherever it needs
//! the global Pyxel instance, the same way upstream's own indirection
//! ultimately does internally. This file exists only so the file list
//! under wrapper_lr/ maps 1:1 onto upstream's pyxel-binding/src/ file
//! list; it deliberately contains no code.
