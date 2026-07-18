//! wrapper_lr/ — lr-pyxel's own Python-wrapper layer over pyxel_core,
//! split by domain (mostly one file per upstream pyxel-binding file
//! of the same name, plus a handful of lr-pyxel-only additions with
//! no upstream analog — e.g. network_wrapper_lr.rs).
//!
//! Every file here is named with an `_lr` suffix rather than matching
//! upstream's own file name exactly (e.g. `font_wrapper_lr.rs`, not
//! `font_wrapper.rs`) — on purpose, so that in conversation or in a
//! commit referencing "image_wrapper.rs" there's no ambiguity about
//! whether that means upstream's own pyxel-binding source or this
//! crate's. Whether a given file's content closely tracks upstream's
//! same-domain file, or is a substantial rework for lr-pyxel's
//! libretro/headless embedding model, is noted in that file's own
//! header comment — the `_lr` suffix itself doesn't imply either way.

pub mod utils_lr;
pub mod pyxel_singleton_lr;
pub mod font_wrapper_lr;
pub mod math_wrapper_lr;
pub mod image_wrapper_lr;
pub mod sound_wrapper_lr;
pub mod tilemap_wrapper_lr;
pub mod channel_wrapper_lr;
pub mod tone_wrapper_lr;
pub mod music_wrapper_lr;
pub mod resource_wrapper_lr;
pub mod network_wrapper_lr;
pub mod audio_wrapper_lr;
pub mod graphics_wrapper_lr;
pub mod input_wrapper_lr;
pub mod constant_wrapper_lr;
pub mod system_wrapper_lr;
pub mod variable_wrapper_lr;
