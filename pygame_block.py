# pygame_block.py — installed once at interpreter startup (retro_init()),
# not per-game-load.
#
# lr-pyxel's audio output goes through the libretro audio_batch_cb.
# RetroArch itself holds the machine's sole ALSA PCM device (see the
# fuser -v /dev/snd/* investigation), so pygame.mixer's own SDL2 audio
# device — opened from inside the same RetroArch process — has nowhere
# to go: pygame.mixer.init() reports success, but the resulting audio
# stream is empty. A game that "successfully" imports a real pygame
# from the system's site-packages therefore goes silent instead of
# falling back to Pyxel's own PCM output.
#
# Until lr-pyxel implements a proper pygame.mixer-compatible bridge into
# the libretro audio callback (tracked as a future enhancement — see
# project notes on 44.1/48kHz passthrough), we deliberately make
# `import pygame` fail exactly as if pygame were not installed at all.
# Games with an existing `try: import pygame / except ImportError:`
# fallback (e.g. LastEmulator) then take the Pyxel-PCM path instead of
# silently going nowhere.
#
# Only the `pygame` package itself is blocked — other site-packages
# modules (numpy, etc.) are left completely untouched.

import sys


class _LrPyxelBlockedModuleFinder:
    """meta_path finder that makes specific module names behave as if
    they were never installed, by raising the same ModuleNotFoundError
    a real absence would raise."""

    BLOCKED = frozenset({"pygame"})

    def find_spec(self, fullname, path, target=None):
        root = fullname.split(".", 1)[0]
        if root in self.BLOCKED:
            raise ModuleNotFoundError(f"No module named {fullname!r}")
        return None


# Idempotent: retro_init() may run more than once in a session (see the
# retro_deinit() comment about RetroArch re-calling retro_init() on
# content switch), so avoid stacking duplicate finders.
if not any(isinstance(f, _LrPyxelBlockedModuleFinder) for f in sys.meta_path):
    sys.meta_path.insert(0, _LrPyxelBlockedModuleFinder())
