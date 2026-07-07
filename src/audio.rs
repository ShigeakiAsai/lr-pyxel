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

/// libretro joypad bit -> Pyxel key/button mapping, shared by
/// inject_input() and reset_all_button_states().
const KEY_MAP: &[(u32, u32)] = &[
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

/// Force every tracked key/button into pyxel_core's "released" state.
///
/// pyxel_core's Input.key_states map (inside the single long-lived Pyxel
/// instance — pyxel_core::init() only ever runs once per process) is
/// never cleared between content switches. If a button was still held
/// down at the exact moment SELECT triggered a core shutdown, retro_run()
/// returns before that frame's inject_input() runs, so that key's state
/// is never transitioned to Released — it stays stuck at Pressed,
/// tagged with that old session's absolute frame_count.
///
/// pyxel_core's btnp(key, hold, repeat) auto-repeat check is:
///   elapsed = current_frame_count - (stored_frame_count + hold)
///   fires when elapsed >= 0 && elapsed % repeat == 0
/// Since frame_count() is reset to 0 on every content switch (see
/// reset_color_palette() and friends) but the stale stored_frame_count
/// is not, `elapsed` starts very negative and then counts back up as the
/// new session's frame_count increases — eventually crossing zero and
/// firing a phantom auto-repeat press, even though nothing is actually
/// held down. This explained the "A-button press fires on its own a few
/// seconds after the launcher starts" bug: a stuck D-pad/A key from the
/// previous session's SELECT-interrupted press.
///
/// Calling set_button_state(key, false) for every mapped key writes a
/// fresh "Released" entry (with the *current* frame_count) over any
/// stale stuck entry, so no leftover state can survive a content switch.
pub unsafe fn reset_all_button_states() {
    if !crate::PYXEL_READY { return; }
    let px = pyxel_core::pyxel();
    for &(_, key) in KEY_MAP {
        px.set_button_state(key, false);
    }
}

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
    let px = pyxel_core::pyxel();
    let changed = buttons ^ PREV_BUTTONS;
    for &(bit, key) in KEY_MAP {
        let mask = 1u32 << bit;
        if changed & mask != 0 {
            px.set_button_state(key, buttons & mask != 0);
        }
    }
    PREV_BUTTONS = buttons;
}
