//! audio.rs — libretro audio output and input injection for lr-pyxel

use crate::{
    AUDIO_BATCH_CB, BLIP_BUF, GAME_W, GAME_H,
    KEY_Z, KEY_X, KEY_RETURN, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
    MOUSE_BUTTON_LEFT, MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT, MOUSE_WHEEL_Y,
};

/// Previous frame's button bitmask — used to detect edges (press/release)
pub static mut PREV_BUTTONS: u32 = 0;

/// Accumulated absolute mouse position. RETRO_DEVICE_MOUSE only reports
/// *relative* motion (dx, dy) per poll, unlike RETRO_DEVICE_JOYPAD's
/// buttons, so — unlike inject_input() — inject_mouse_input() has to
/// keep its own running position across frames rather than translating
/// a fresh absolute value every time.
pub static mut MOUSE_ACCUM_X: i32 = 0;
pub static mut MOUSE_ACCUM_Y: i32 = 0;

/// Previous frame's mouse button bitmask (bit0=left, bit1=right,
/// bit2=middle) — used the same way as PREV_BUTTONS, to only call
/// set_button_state() on an actual change. Calling it unconditionally
/// every frame a button stays held re-stamps pyxel_core's stored
/// press-frame to "now" each time, making btnp() (a "just pressed this
/// frame" check) fire every single frame instead of once — the mouse
/// looked like it was auto-repeating/rapid-firing clicks as a result.
static mut PREV_MOUSE_BUTTONS: u32 = 0;

/// Sample accumulator to handle 22050/60 = 367.5 (alternates 367/368)
static mut SAMPLE_ACCUMULATOR: f32 = 0.0;

/// libretro joypad bit -> Pyxel key/button mapping, shared by
/// inject_input() and reset_all_button_states().
const KEY_MAP: &[(u32, u32)] = &[
    (0,  KEY_Z),
    (1,  KEY_X),
    (3,  KEY_RETURN),
    (4,  KEY_UP),
    (5,  KEY_DOWN),
    (6,  KEY_LEFT),
    (7,  KEY_RIGHT),
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
    // Mouse buttons are just as susceptible to the same stuck-press bug
    // (a SELECT-interrupted click leaves a stale "Pressed" entry), so
    // reset them here too. Also recenter the accumulated cursor position
    // for the new content, rather than carrying over wherever the
    // previous game's cursor happened to be.
    px.set_button_state(MOUSE_BUTTON_LEFT,   false);
    px.set_button_state(MOUSE_BUTTON_RIGHT,  false);
    px.set_button_state(MOUSE_BUTTON_MIDDLE, false);
    PREV_MOUSE_BUTTONS = 0;
    // Not centering on GAME_W/GAME_H here: this runs before the new
    // content's dimensions are set, so those globals would still
    // reflect the previous game's size. (0, 0) is always valid
    // regardless of size.
    MOUSE_ACCUM_X = 0;
    MOUSE_ACCUM_Y = 0;
    px.set_mouse_position(0.0, 0.0);
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

/// Poll RETRO_DEVICE_MOUSE and feed the result into pyxel_core's mouse
/// state (position, buttons, wheel). Call once per retro_run() frame,
/// alongside inject_input(). Safe to call even if no mouse is
/// physically connected — RetroArch just reports all-zero deltas/
/// buttons in that case, so mouse_x/mouse_y simply never move and
/// mouse buttons/wheel stay at rest.
pub unsafe fn inject_mouse_input(state: unsafe extern "C" fn(u32, u32, u32, u32) -> i16) {
    // libretro.h device/ID constants for RETRO_DEVICE_MOUSE. Defined
    // locally (rather than relying on rust_libretro_sys's exact naming)
    // since these are simple, stable values straight from the spec.
    const RETRO_DEVICE_MOUSE: u32      = 2;
    const ID_MOUSE_X: u32              = 0;
    const ID_MOUSE_Y: u32              = 1;
    const ID_MOUSE_LEFT: u32           = 2;
    const ID_MOUSE_RIGHT: u32          = 3;
    const ID_MOUSE_WHEELUP: u32        = 4;
    const ID_MOUSE_WHEELDOWN: u32      = 5;
    const ID_MOUSE_MIDDLE: u32         = 6;

    let poll = |id: u32| state(0, RETRO_DEVICE_MOUSE, 0, id);

    // Position: RETRO_DEVICE_MOUSE reports relative motion since the
    // last poll, not an absolute position (unlike a real desktop mouse
    // cursor) — accumulate it ourselves, clamped to the current game's
    // screen bounds, matching how a cursor can't leave the window.
    let dx = i32::from(poll(ID_MOUSE_X));
    let dy = i32::from(poll(ID_MOUSE_Y));
    if dx != 0 || dy != 0 {
        MOUSE_ACCUM_X = (MOUSE_ACCUM_X + dx).clamp(0, GAME_W as i32 - 1);
        MOUSE_ACCUM_Y = (MOUSE_ACCUM_Y + dy).clamp(0, GAME_H as i32 - 1);
        pyxel_core::pyxel().set_mouse_position(MOUSE_ACCUM_X as f32, MOUSE_ACCUM_Y as f32);
    }

    // Buttons: only call set_button_state() when a button's state
    // actually changed since last frame — see PREV_MOUSE_BUTTONS's doc
    // comment for why calling it unconditionally every frame broke
    // btnp() into firing continuously while a button was held.
    let px = pyxel_core::pyxel();
    let mouse_buttons =
        (u32::from(poll(ID_MOUSE_LEFT)   != 0))
        | (u32::from(poll(ID_MOUSE_RIGHT)  != 0) << 1)
        | (u32::from(poll(ID_MOUSE_MIDDLE) != 0) << 2);
    let changed = mouse_buttons ^ PREV_MOUSE_BUTTONS;
    if changed & 0b001 != 0 { px.set_button_state(MOUSE_BUTTON_LEFT,   mouse_buttons & 0b001 != 0); }
    if changed & 0b010 != 0 { px.set_button_state(MOUSE_BUTTON_RIGHT,  mouse_buttons & 0b010 != 0); }
    if changed & 0b100 != 0 { px.set_button_state(MOUSE_BUTTON_MIDDLE, mouse_buttons & 0b100 != 0); }
    PREV_MOUSE_BUTTONS = mouse_buttons;

    // Wheel: WHEELUP/WHEELDOWN each report a click count since the last
    // poll (not a held state), so combine them into a signed delta.
    // pyxel_core resets mouse_wheel to 0 at the start of every input
    // frame on its own, so we only need to report *this* frame's delta.
    let wheel_delta = i32::from(poll(ID_MOUSE_WHEELUP)) - i32::from(poll(ID_MOUSE_WHEELDOWN));
    if wheel_delta != 0 {
        pyxel_core::pyxel().set_button_value(MOUSE_WHEEL_Y, wheel_delta);
    }
}
