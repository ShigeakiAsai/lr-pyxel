//! input.rs — libretro joypad/mouse input injection for lr-pyxel
//!
//! Split out of audio.rs (v0.12.2): that file's name only ever matched
//! its submit_audio_frame() function — everything else here (button
//! mapping, edge detection, mouse polling) is input handling that
//! happened to share a file with audio because both get driven once
//! per retro_run() frame. Purely a file reorganization; no behavior
//! changes.

use crate::{
    GAME_W, GAME_H,
    KEY_Z, KEY_X, KEY_RETURN, KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
    KEY_DELETE,
    KEY_INSERT, KEY_HOME, KEY_END, KEY_PAGEUP, KEY_PAGEDOWN,
    KEY_F1, KEY_F2, KEY_F3, KEY_F4, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F9, KEY_F10, KEY_F11, KEY_F12,
    KEY_CAPSLOCK,
    KEY_RSHIFT, KEY_LSHIFT, KEY_RCTRL, KEY_LCTRL, KEY_RALT, KEY_LALT, KEY_RGUI, KEY_LGUI,
    GAMEPAD1_BUTTON_A, GAMEPAD1_BUTTON_B, GAMEPAD1_BUTTON_X,
    GAMEPAD1_BUTTON_BACK, GAMEPAD1_BUTTON_START,
    GAMEPAD1_BUTTON_DPAD_UP, GAMEPAD1_BUTTON_DPAD_DOWN,
    GAMEPAD1_BUTTON_DPAD_LEFT, GAMEPAD1_BUTTON_DPAD_RIGHT,
    GAMEPAD1_BUTTON_LEFTSHOULDER, GAMEPAD1_BUTTON_RIGHTSHOULDER,
    MOUSE_BUTTON_LEFT, MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT, MOUSE_WHEEL_Y,
};
// Not re-exported at lr-pyxel's own crate root (crate::) — only the
// handful of KEY_* constants the pre-existing joypad-alias KEY_MAP
// needed were ever re-exported there. Pulled directly from pyxel_core
// instead of also threading a new re-export through lib.rs.
use pyxel_core::{
    KEY_CLEAR, KEY_PAUSE,
    KEY_KP_0, KEY_KP_1, KEY_KP_2, KEY_KP_3, KEY_KP_4, KEY_KP_5, KEY_KP_6, KEY_KP_7, KEY_KP_8, KEY_KP_9,
    KEY_KP_PERIOD, KEY_KP_DIVIDE, KEY_KP_MULTIPLY, KEY_KP_MINUS, KEY_KP_PLUS, KEY_KP_ENTER, KEY_KP_EQUALS,
    KEY_F13, KEY_F14, KEY_F15,
    KEY_NUMLOCKCLEAR, KEY_SCROLLLOCK,
    KEY_HELP, KEY_PRINTSCREEN, KEY_SYSREQ, KEY_MENU, KEY_POWER, KEY_UNDO,
    KEY_MUTE, KEY_VOLUMEDOWN, KEY_VOLUMEUP,
};

/// Previous frame's keyboard key states, keyed by RETROK_* id — used to
/// detect edges (press/release) the same way PREV_BUTTONS/
/// PREV_MOUSE_BUTTONS do, only calling set_button_state() on an actual
/// change (see PREV_MOUSE_BUTTONS's doc comment for why: calling it
/// unconditionally every frame a key stays held breaks btnp() into
/// firing every frame instead of once).
static mut PREV_KEYBOARD_KEYS: [bool; RETROK_TABLE.len()] = [false; RETROK_TABLE.len()];

/// libretro RETROK_* -> pyxel_core KEY_* mapping.
///
/// pyxel_core's KEY_* constants are SDL2 SDL_Keycode values verbatim
/// (confirmed against pyxel_core's own SDL2 event-polling code:
/// `sdl_event.key.keysym.sym as Key`). libretro's RETROK_* enum, by
/// contrast, still uses SDL 1.2-era linear numbering for special keys
/// (confirmed against libretro.h) — the two only agree by coincidence
/// in the printable-ASCII range (both ultimately trace back to plain
/// ASCII), and diverge completely for everything past it (SDL2 moved
/// special keys to a 0x4000_00XX-prefixed range; libretro kept the old
/// 256+ linear scheme). Two ASCII-range values are exceptions to even
/// that partial agreement — RETROK_CLEAR(12) and RETROK_PAUSE(19) both
/// landed on control-character values SDL2 doesn't use the same way —
/// so this table is built as one explicit, entry-by-entry mapping
/// covering every RETROK_* value pyxel_core has an equivalent for,
/// rather than a passthrough-plus-exceptions scheme that would be
/// easier to get subtly wrong.
///
/// Keys with no reasonable pyxel_core equivalent (MODE, COMPOSE, EURO,
/// OEM_102, the BROWSER_*/MEDIA_*/LAUNCH_* multimedia keys other than
/// the 3 volume ones) are simply omitted — btn()/btnp() on an
/// unrequested key already correctly returns False/never-pressed with
/// no entry needed.
const RETROK_TABLE: &[(u32, u32)] = &[
    // ASCII-range exceptions (see doc comment above)
    (12,  KEY_CLEAR),
    (19,  KEY_PAUSE),
    // Printable ASCII + a few control keys: RETROK_* value is
    // numerically identical to pyxel_core's KEY_* value throughout
    // this range (confirmed entry-by-entry against libretro.h and
    // key.rs), so each pair is just (n, n) — except DELETE, included
    // here for the same reason even though it's technically outside
    // "printable" ASCII.
    (8, 8), (9, 9), (13, 13), (27, 27), (32, 32), (33, 33), (34, 34), (35, 35),
    (36, 36), (37, 37), (38, 38), (39, 39), (40, 40), (41, 41), (42, 42), (43, 43),
    (44, 44), (45, 45), (46, 46), (47, 47), (48, 48), (49, 49), (50, 50), (51, 51),
    (52, 52), (53, 53), (54, 54), (55, 55), (56, 56), (57, 57), (58, 58), (59, 59),
    (60, 60), (61, 61), (62, 62), (63, 63), (64, 64), (65, 65), (66, 66), (67, 67),
    (68, 68), (69, 69), (70, 70), (71, 71), (72, 72), (73, 73), (74, 74), (75, 75),
    (76, 76), (77, 77), (78, 78), (79, 79), (80, 80), (81, 81), (82, 82), (83, 83),
    (84, 84), (85, 85), (86, 86), (87, 87), (88, 88), (89, 89), (90, 90), (91, 91),
    (92, 92), (93, 93), (94, 94), (95, 95), (96, 96), (97, 97), (98, 98), (99, 99),
    (100, 100), (101, 101), (102, 102), (103, 103), (104, 104), (105, 105), (106, 106), (107, 107),
    (108, 108), (109, 109), (110, 110), (111, 111), (112, 112), (113, 113), (114, 114), (115, 115),
    (116, 116), (117, 117), (118, 118), (119, 119), (120, 120), (121, 121), (122, 122), (123, 123),
    (124, 124), (125, 125), (126, 126), (127, KEY_DELETE),
    // Keypad
    (256, KEY_KP_0), (257, KEY_KP_1), (258, KEY_KP_2), (259, KEY_KP_3), (260, KEY_KP_4),
    (261, KEY_KP_5), (262, KEY_KP_6), (263, KEY_KP_7), (264, KEY_KP_8), (265, KEY_KP_9),
    (266, KEY_KP_PERIOD), (267, KEY_KP_DIVIDE), (268, KEY_KP_MULTIPLY),
    (269, KEY_KP_MINUS), (270, KEY_KP_PLUS), (271, KEY_KP_ENTER), (272, KEY_KP_EQUALS),
    // Navigation
    (273, KEY_UP), (274, KEY_DOWN), (275, KEY_RIGHT), (276, KEY_LEFT),
    (277, KEY_INSERT), (278, KEY_HOME), (279, KEY_END), (280, KEY_PAGEUP), (281, KEY_PAGEDOWN),
    // Function keys
    (282, KEY_F1), (283, KEY_F2), (284, KEY_F3), (285, KEY_F4), (286, KEY_F5), (287, KEY_F6),
    (288, KEY_F7), (289, KEY_F8), (290, KEY_F9), (291, KEY_F10), (292, KEY_F11), (293, KEY_F12),
    (294, KEY_F13), (295, KEY_F14), (296, KEY_F15),
    // Lock keys
    (300, KEY_NUMLOCKCLEAR), (301, KEY_CAPSLOCK), (302, KEY_SCROLLLOCK),
    // Modifiers
    (303, KEY_RSHIFT), (304, KEY_LSHIFT), (305, KEY_RCTRL), (306, KEY_LCTRL),
    (307, KEY_RALT), (308, KEY_LALT), (309, KEY_RGUI), (310, KEY_LGUI),
    (311, KEY_LGUI), (312, KEY_RGUI),
    // Misc
    (315, KEY_HELP), (316, KEY_PRINTSCREEN), (317, KEY_SYSREQ), (318, KEY_PAUSE),
    (319, KEY_MENU), (320, KEY_POWER), (322, KEY_UNDO),
    // Volume (the only BROWSER_*/MEDIA_*/LAUNCH_* range keys pyxel_core
    // has an equivalent for)
    (331, KEY_MUTE), (332, KEY_VOLUMEDOWN), (333, KEY_VOLUMEUP),
];

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
    // Mouse visibility (pyxel.mouse(True/False)) isn't reset by
    // pyxel_core itself on content switch either — a game that shows
    // its cursor (e.g. LastEmulator.pyxapp) leaves it visible for
    // whatever runs next, including the launcher (which never calls
    // pyxel.mouse() at all, since it predates mouse support). Default
    // to hidden, matching pyxel_core's own initial state.
    px.set_mouse_visible(false);

    // Keyboard keys are susceptible to the exact same stuck-press bug
    // as joypad buttons and mouse buttons above (a SELECT-interrupted
    // keypress leaves a stale "Pressed" entry for the new content).
    for &(_, key) in RETROK_TABLE {
        px.set_button_state(key, false);
    }
    PREV_KEYBOARD_KEYS = [false; RETROK_TABLE.len()];
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

/// Poll RETRO_DEVICE_KEYBOARD and feed the result into pyxel_core's key
/// states. Call once per retro_run() frame, alongside inject_input()
/// and inject_mouse_input(). Same edge-detection pattern as
/// inject_mouse_input()'s button handling: only call set_button_state()
/// on an actual change, not every frame a key stays held (see
/// PREV_MOUSE_BUTTONS's doc comment for why unconditional calls break
/// btnp()). Safe to call even with no physical keyboard attached —
/// RetroArch just reports every key as unpressed in that case.
///
/// Unlike RETRO_DEVICE_JOYPAD's 16-bit bitmask (one poll call covers
/// every button) or RETRO_DEVICE_MOUSE's handful of fixed IDs,
/// RETRO_DEVICE_KEYBOARD has no bitmask form — each key must be polled
/// individually by its RETROK_* id, one input_state() call per key
/// per frame. Only the keys in RETROK_TABLE are polled (not the full
/// RETROK_* range), since those are the only ones with a pyxel_core
/// KEY_* equivalent to report in the first place.
pub unsafe fn inject_keyboard_input(state: unsafe extern "C" fn(u32, u32, u32, u32) -> i16) {
    const RETRO_DEVICE_KEYBOARD: u32 = 3;
    let px = pyxel_core::pyxel();
    for (i, &(retrok, key)) in RETROK_TABLE.iter().enumerate() {
        let pressed = state(0, RETRO_DEVICE_KEYBOARD, 0, retrok) != 0;
        if pressed != PREV_KEYBOARD_KEYS[i] {
            px.set_button_state(key, pressed);
            PREV_KEYBOARD_KEYS[i] = pressed;
        }
    }
}
