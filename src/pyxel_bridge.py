# SPDX-License-Identifier: MIT
# Copyright (c) 2026-present Yasai-san
#
# pyxel_bridge.py
# Bridge between the lr-pyxel libretro core and the Pyxel game engine.
# Called from Rust via PyO3 once per frame.

import importlib.util
import struct
import pyxel

# -- screen dimensions (must match retro_get_system_av_info) ------------------
SCREEN_W = 128
SCREEN_H = 128

# -- internal state -----------------------------------------------------------
_game_update = None   # update() function of the loaded game
_game_draw   = None   # draw() function of the loaded game
_initialized = False

# Pre-built palette lookup table: palette index -> RGB565 (u16)
_palette_rgb565: list[int] = []

# -- public API (called from Rust) --------------------------------------------

def init() -> bool:
    """Initialize Pyxel in headless mode. Called once from retro_init()."""
    global _initialized, _palette_rgb565

    try:
        pyxel.init(SCREEN_W, SCREEN_H, fps=60)
        _build_palette()
        _initialized = True
        return True
    except Exception as e:
        print(f"[pyxel_bridge] init error: {e}")
        return False


def load_game(path: str) -> bool:
    """
    Load a Pyxel game from a .py file.
    Called from retro_load_game() with the ROM file path.
    """
    global _game_update, _game_draw

    if not _initialized:
        print("[pyxel_bridge] not initialized")
        return False

    try:
        spec   = importlib.util.spec_from_file_location("_pyxel_game", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        _game_update = getattr(module, "update", None)
        _game_draw   = getattr(module, "draw",   None)

        if _game_update is None or _game_draw is None:
            print("[pyxel_bridge] game must define update() and draw()")
            return False

        return True
    except Exception as e:
        print(f"[pyxel_bridge] load_game error: {e}")
        return False


def unload_game() -> None:
    """Called from retro_unload_game()."""
    global _game_update, _game_draw
    _game_update = None
    _game_draw   = None


def set_input(buttons: int) -> None:
    """
    Inject libretro joypad button state into Pyxel.
    buttons is a bitmask of RETRO_DEVICE_ID_JOYPAD_* bits.

    Bit mapping (matches libretro joypad):
      bit 0  B       -> KEY_Z
      bit 1  Y       -> KEY_X
      bit 4  UP      -> KEY_UP
      bit 5  DOWN    -> KEY_DOWN
      bit 6  LEFT    -> KEY_LEFT
      bit 7  RIGHT   -> KEY_RIGHT
      bit 8  A       -> KEY_A
      bit 9  X       -> KEY_S
      bit 2  SELECT  -> KEY_ESCAPE
      bit 3  START   -> KEY_RETURN
    """
    BUTTON_MAP = {
        0:  pyxel.KEY_Z,
        1:  pyxel.KEY_X,
        4:  pyxel.KEY_UP,
        5:  pyxel.KEY_DOWN,
        6:  pyxel.KEY_LEFT,
        7:  pyxel.KEY_RIGHT,
        8:  pyxel.KEY_A,
        9:  pyxel.KEY_S,
        2:  pyxel.KEY_ESCAPE,
        3:  pyxel.KEY_RETURN,
    }
    for bit, key in BUTTON_MAP.items():
        pressed = bool(buttons & (1 << bit))
        pyxel.set_btn(key, pressed)


def run_frame() -> None:
    """Advance Pyxel by one frame. Called from retro_run()."""
    if _game_update:
        _game_update()
    if _game_draw:
        _game_draw()
    # Flush Pyxel's internal render pipeline without opening a window
    pyxel.flip()


def get_framebuffer() -> bytes:
    """
    Return the current screen as RGB565 bytes (2 bytes per pixel, row-major).
    Uses the pre-built palette LUT for fast conversion.
    """
    src  = pyxel.screen.data_ptr()   # palette index per pixel (u8)
    lut  = _palette_rgb565
    out  = bytearray(SCREEN_W * SCREEN_H * 2)
    i    = 0
    for idx in src:
        rgb565 = lut[idx]
        out[i]     =  rgb565 & 0xFF          # low byte
        out[i + 1] = (rgb565 >> 8) & 0xFF    # high byte
        i += 2
    return bytes(out)


# -- internal helpers ---------------------------------------------------------

def _build_palette() -> None:
    """Build a lookup table from Pyxel palette index to RGB565."""
    global _palette_rgb565
    _palette_rgb565 = []
    for rgb888 in pyxel.colors:
        r = (rgb888 >> 16) & 0xFF
        g = (rgb888 >>  8) & 0xFF
        b =  rgb888        & 0xFF
        rgb565 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)
        _palette_rgb565.append(rgb565)
