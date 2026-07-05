//! audio.rs — libretro audio output and input injection for lr-pyxel

use crate::{
    AUDIO_BATCH_CB, BLIP_BUF,
    KEY_Z, KEY_X, KEY_S, KEY_RETURN, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_A,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
};

/// Previous frame's button bitmask — used to detect edges (press/release)
pub static mut PREV_BUTTONS: u32 = 0;

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

/// Translate libretro joypad bitmask to Pyxel key states.
pub unsafe fn inject_input(buttons: u32) {
    const MAP: &[(u32, u32)] = &[
        (0,  KEY_Z),
        (1,  KEY_X),
        (2,  KEY_S),
        (3,  KEY_RETURN),
        (4,  KEY_UP),
        (5,  KEY_DOWN),
        (6,  KEY_LEFT),
        (7,  KEY_RIGHT),
        (8,  KEY_A),
        (9,  KEY_S),
        (0,  GAMEPAD1_BUTTON_B),
        (1,  GAMEPAD1_BUTTON_A),
        (2,  GAMEPAD1_BUTTON_BACK),
        (3,  GAMEPAD1_BUTTON_START),
        (4,  GAMEPAD1_BUTTON_DPAD_UP),
        (5,  GAMEPAD1_BUTTON_DPAD_DOWN),
        (6,  GAMEPAD1_BUTTON_DPAD_LEFT),
        (7,  GAMEPAD1_BUTTON_DPAD_RIGHT),
        (8,  GAMEPAD1_BUTTON_A),
        (9,  GAMEPAD1_BUTTON_X),
        (10, GAMEPAD1_BUTTON_LEFTSHOULDER),
        (11, GAMEPAD1_BUTTON_RIGHTSHOULDER),
    ];
    let px = pyxel_core::pyxel();
    let changed = buttons ^ PREV_BUTTONS;
    for &(bit, key) in MAP {
        let mask = 1u32 << bit;
        if changed & mask != 0 {
            px.set_button_state(key, buttons & mask != 0);
        }
    }
    PREV_BUTTONS = buttons;
}
