//! audio.rs — libretro audio output for lr-pyxel
//!
//! Input handling (button mapping, mouse polling) moved to input.rs in
//! v0.12.2 — this file previously held both, even though its name only
//! ever matched submit_audio_frame(). Purely a file reorganization; no
//! behavior changes.

use crate::{AUDIO_BATCH_CB, BLIP_BUF};

/// Sample accumulator to handle 22050/60 = 367.5 (alternates 367/368)
static mut SAMPLE_ACCUMULATOR: f32 = 0.0;

/// Render Pyxel's audio and submit to RetroArch's audio batch callback.
/// Called every retro_run() frame (60fps) regardless of game fps.
pub unsafe fn submit_audio_frame() {
    let Some(ref mut blip) = BLIP_BUF else { return; };
    let Some(audio_cb)     = AUDIO_BATCH_CB else { return; };

    // Accumulator: absorb the 0.5-sample-per-frame rounding error
    // 22050 / 60 = 367.5 → alternates between 367 and 368 samples
    SAMPLE_ACCUMULATOR += 22050.0 / 60.0;
    let samples = SAMPLE_ACCUMULATOR.floor() as usize;
    SAMPLE_ACCUMULATOR -= samples as f32;

    let mut mono = vec![0i16; samples];
    pyxel_core::Audio::render_samples(pyxel_core::channels(), blip, &mut mono);

    let mut stereo = vec![0i16; samples * 2];
    for (i, &s) in mono.iter().enumerate() {
        stereo[i * 2]     = s;
        stereo[i * 2 + 1] = s;
    }

    audio_cb(stereo.as_ptr(), samples);
}
