# SPDX-License-Identifier: MIT
# Copyright (c) 2026-present Yasai-san
#
# pyxel_bridge.py
# Bridge between the lr-pyxel libretro core and the Pyxel game engine.
# When Pyxel is not available, a mock renderer is used for verification.

import importlib.util
import struct

# -- screen dimensions (must match retro_get_system_av_info) ------------------
SCREEN_W = 128
SCREEN_H = 128

# -- internal state -----------------------------------------------------------
_game_update   = None
_game_draw     = None
_initialized   = False
_use_mock      = False   # True when Pyxel is unavailable

# Pre-built palette lookup table: palette index -> RGB565 (u16)
_palette_rgb565: list = []

# Mock framebuffer: simple checkerboard pattern in RGB565
_mock_frame    = 0
_mock_fb: bytes = b""

# -- public API ---------------------------------------------------------------

def init() -> bool:
    """Initialize Pyxel (or mock) in headless mode. Called once from retro_init()."""
    global _initialized, _use_mock, _palette_rgb565, _mock_fb

    try:
        import pyxel
        pyxel.init(SCREEN_W, SCREEN_H, fps=60)
        _build_palette(pyxel)
        _use_mock = False
        print("[pyxel_bridge] Pyxel initialized successfully")
    except Exception as e:
        print(f"[pyxel_bridge] Pyxel unavailable ({e}), using mock renderer")
        _use_mock = True
        _palette_rgb565 = list(range(256))  # dummy LUT
        _mock_fb = _build_mock_fb()

    _initialized = True
    return True


def load_game(path: str) -> bool:
    """Load a Pyxel game from a .py file. Called from retro_load_game()."""
    global _game_update, _game_draw

    if not _initialized:
        print("[pyxel_bridge] not initialized")
        return False

    if _use_mock:
        print(f"[pyxel_bridge] mock mode: ignoring load_game({path})")
        return True

    try:
        import pyxel
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
    """Inject libretro joypad button state into Pyxel (or mock)."""
    if _use_mock:
        return  # mock ignores input

    try:
        import pyxel
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
            pyxel.set_btn(key, bool(buttons & (1 << bit)))
    except Exception as e:
        print(f"[pyxel_bridge] set_input error: {e}")


def run_frame() -> None:
    """Advance one frame. Called from retro_run()."""
    global _mock_frame, _mock_fb

    if _use_mock:
        _mock_frame += 1
        # Animate mock framebuffer every 30 frames
        if _mock_frame % 30 == 0:
            _mock_fb = _build_mock_fb(_mock_frame)
        return

    try:
        import pyxel
        if _game_update:
            _game_update()
        if _game_draw:
            _game_draw()
        pyxel.flip()
    except Exception as e:
        print(f"[pyxel_bridge] run_frame error: {e}")


def get_framebuffer() -> bytes:
    """Return current screen as RGB565 bytes (2 bytes per pixel, row-major)."""
    if _use_mock:
        return _mock_fb

    try:
        import pyxel
        src = pyxel.screen.data_ptr()
        lut = _palette_rgb565
        out = bytearray(SCREEN_W * SCREEN_H * 2)
        i   = 0
        for idx in src:
            rgb565     = lut[idx]
            out[i]     =  rgb565 & 0xFF
            out[i + 1] = (rgb565 >> 8) & 0xFF
            i += 2
        return bytes(out)
    except Exception as e:
        print(f"[pyxel_bridge] get_framebuffer error: {e}")
        return _mock_fb

# -- internal helpers ---------------------------------------------------------

def _build_palette(pyxel) -> None:
    """Build RGB565 lookup table from Pyxel palette."""
    global _palette_rgb565
    _palette_rgb565 = []
    for rgb888 in pyxel.colors:
        r = (rgb888 >> 16) & 0xFF
        g = (rgb888 >>  8) & 0xFF
        b =  rgb888        & 0xFF
        _palette_rgb565.append(((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3))


def _build_mock_fb(frame: int = 0) -> bytes:
    """
    Generate a simple animated checkerboard framebuffer in RGB565.
    Alternates between two colors every 30 frames.
    """
    COLOR_A: int = 0x07E0  # green
    COLOR_B: int = 0x001F  # blue
    phase = (frame // 30) % 2
    out   = bytearray(SCREEN_W * SCREEN_H * 2)
    i     = 0
    for y in range(SCREEN_H):
        for x in range(SCREEN_W):
            checker = (x // 16 + y // 16 + phase) % 2
            color   = COLOR_A if checker == 0 else COLOR_B
            out[i]     =  color & 0xFF
            out[i + 1] = (color >> 8) & 0xFF
            i += 2
    return bytes(out)
