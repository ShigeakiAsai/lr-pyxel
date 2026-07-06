# lr-pyxel

A libretro core that runs [Pyxel](https://github.com/kitao/pyxel) games on RetroArch/Lakka.

> **Status**: Work in progress — v0.7.0 tagged, v0.8.0 in development.

---

## v0.7.0 Highlights

- `pyxel.download_file()` / `pyxel.http_get()` — network access via the
  system `curl` binary (Lakka's embedded Python has no `_socket.so` /
  `_ssl.so`, so this replaces raw-socket/SSL Python code)
- In-core game downloader (`downloader.pyxapp`) rewritten to use the above
- Launcher (`frontend.py`) input carry-over fix when returning from
  sub-apps like the downloader
- Auto-repeat (hold/repeat, no acceleration) for D-pad/cursor navigation
  in the launcher and downloader
- Downloader "back to launcher" now responds to the physical B button
  (SELECT is reserved core-wide for shutdown, not app navigation)
- Build warning cleanup (`dead_code`, `static_mut_refs`)

Planned for v0.8.0: audio submission fix (send 22050/GAME_FPS samples
only on `should_update`, dropping the frame accumulator) and other
follow-up items.

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
