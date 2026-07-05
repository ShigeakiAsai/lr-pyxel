# lr-pyxel

A libretro core that runs [Pyxel](https://github.com/kitao/pyxel) games on RetroArch/Lakka.

> **Status**: Work in progress — v0.5.x series complete, v0.6.x in development.

---

## Overview

lr-pyxel embeds CPython 3.11 and a headless Pyxel engine inside a libretro core,
allowing Pyxel games (`.py` and `.pyxapp`) to run on Lakka/RetroArch
on devices such as the Raspberry Pi 5.

---

## Supported Content

| Format | Support |
|--------|---------|
| `.py` (single script) | ✅ |
| `.pyxapp` (packaged app) | ✅ |

---

## Known Limitations

The following are **structural limitations** of the libretro architecture
and cannot be resolved:

- **`pyxel.flip()`-based games** (e.g. `99_flip_animation.py`):
  The infinite `while True: pyxel.flip()` loop is incompatible with
  libretro's frame-driven `retro_run()` model.

- **`pyxel.cli` / app launcher** (e.g. `17_app_launcher.py`):
  The Pyxel CLI and launcher are not available in the libretro environment.

---

## Build

> (To be documented after release features are finalized)

---

## License

MIT

---

## Credits

- [Pyxel](https://github.com/kitao/pyxel) by kitao
- [Lakka](https://www.lakka.tv/)
- [RetroArch](https://www.retroarch.com/)
