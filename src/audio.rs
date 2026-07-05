//! audio.rs — libretro audio output and input injection for lr-pyxel

use crate::{
    AUDIO_BATCH_CB, BLIP_BUF, AUDIO_SAMPLES_PER_FRAME,
    KEY_Z, KEY_X, KEY_S, KEY_RETURN, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_A,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
};

/// Previous frame's button bitmask — used to detect edges (press/release)
pub static mut PREV_BUTTONS: u32 = 0;

/// Render Pyxel's audio and submit to RetroArch's audio batch callback.
pub unsafe fn submit_audio_frame() {
    let Some(ref mut blip) = BLIP_BUF else { return; };
    let Some(audio_cb)     = AUDIO_BATCH_CB else { return; };

    let mut mono = [0i16; AUDIO_SAMPLES_PER_FRAME];
    pyxel_core::Audio::render_samples(pyxel_core::channels(), blip, &mut mono);

    let mut stereo = [0i16; AUDIO_SAMPLES_PER_FRAME * 2];
    for (i, &s) in mono.iter().enumerate() {
        stereo[i * 2]     = s;
        stereo[i * 2 + 1] = s;
    }

    audio_cb(stereo.as_ptr(), AUDIO_SAMPLES_PER_FRAME);
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
