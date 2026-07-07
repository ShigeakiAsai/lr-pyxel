# lr-pyxel

A libretro core that runs [Pyxel](https://github.com/kitao/pyxel) games on RetroArch/Lakka.

[日本語 README](README.ja.md)

> **Status**: v0.8.4 tagged, v0.9.0 in development (documentation & release prep).

---

## Overview

lr-pyxel embeds CPython 3.11 and a headless Pyxel engine inside a libretro core,
allowing Pyxel games (`.py` and `.pyxapp`) to run on Lakka/RetroArch
on devices such as the Raspberry Pi 5.

There are two ways to launch content, and they support different file
types:

- **Launched with content** (loading a file directly as the core's
  content, e.g. via a RetroArch playlist): only **`.pyxapp`** is
  supported. `.pyxapp` is a self-contained packaged format, which is
  what RetroArch's direct-content-loading model expects — a bare `.py`
  file isn't a well-defined "piece of content" in the same way.
- **Launched with no content**: the built-in launcher starts instead,
  browsing `/storage/roms/pyxel`. **Only in this launcher** can you
  also run a loose **`.py`** script directly, alongside `.pyxapp`
  files — the launcher is just listing a folder, so both are equally
  convenient there.

An in-core downloader (`downloader.pyxapp`), launchable from the
`[Download new games]` entry at the top of the launcher's file list,
can fetch additional games over HTTP into that same folder. Downloaded
games are saved to `/storage/roms/pyxel` and can then be selected from
the launcher.

---

## Supported Content

| Format | Launched directly with content | Launched via the built-in launcher (no content) |
|--------|:---:|:---:|
| `.pyxapp` (packaged app) | ✅ | ✅ |
| `.py` (single script) | ❌ | ✅ |

---

## Build

lr-pyxel is built as a package inside a Lakka/LibreELEC buildroot checkout,
cross-compiled for the target device (currently developed against
Raspberry Pi 5 / aarch64).

```bash
# From the root of your Lakka-LibreELEC checkout:
DISTRO=Lakka PROJECT=RPi DEVICE=RPi5 ARCH=aarch64 scripts/clean pyxel
DISTRO=Lakka PROJECT=RPi DEVICE=RPi5 ARCH=aarch64 scripts/build pyxel
```

The resulting core is installed to `usr/lib/libretro/pyxel_libretro.so`
inside the package's `install_pkg` output.

### Dependency notes

- `Cargo.toml` pins `pyxel-core` to the `lr-pyxel` branch of
  [ShigeakiAsai/pyxel](https://github.com/ShigeakiAsai/pyxel) (a fork of
  upstream Pyxel), **not** the default branch — the fork's `main` branch is
  kept clean for upstream contributions (see PR
  [kitao/pyxel#718](https://github.com/kitao/pyxel/pull/718)). After
  pulling changes to that branch, run `cargo update -p pyxel-core` before
  rebuilding.
- Networking (`pyxel.download_file()` / `pyxel.http_get()`) shells out to
  the system `curl` binary rather than linking libcurl, so the target
  device needs `curl` on its `PATH`.
- Lakka's embedded Python lacks several compiled standard-library
  extensions (`_socket`, `_struct`, `_random`, ...); `math`, `random`, and
  `struct` are replaced with pure-Python stubs written to
  `/tmp/lr-pyxel-stdlib` at boot. Some other compiled stdlib extensions
  may still fail to load with an ABI mismatch (`undefined symbol: ...`) —
  see [Known Issues](#known-issues).

---

## Known Limitations

These script patterns can't run under lr-pyxel's frame-driven
`retro_run()` model. Since v0.8.2, both fail safely: an on-screen
RetroArch notification is shown and the core returns to the launcher,
rather than crashing or hanging.

- **`pyxel.flip()`-based games** (e.g. `99_flip_animation.py`): the
  classic `while True: ... pyxel.flip()` main loop pattern doesn't fit
  libretro's model, where the frontend calls `retro_run()` once per
  frame rather than the game driving its own loop. `pyxel.flip()` now
  raises immediately instead of silently no-op'ing (which previously
  hung the whole RetroArch process, since the infinite loop never
  yielded back to Rust).
- **`pyxel.cli` / app launcher** (e.g. `17_app_launcher.py`): the Pyxel
  CLI and its own app-switching mechanism aren't available headless;
  `import pyxel.cli` fails with `ModuleNotFoundError`, which is caught
  and bounces back to the launcher.
- **Mouse input**: not implemented — `retro_run()` only polls
  `RETRO_DEVICE_JOYPAD`, never `RETRO_DEVICE_MOUSE`, so `mouse_x`/
  `mouse_y` never move. `pyxel.mouse(True)` is forced to stay hidden
  rather than show a static, non-functional cursor. Planned for v2.0.0.

---

## Known Issues

- A handful of compiled Python standard-library extensions may fail to
  load with `undefined symbol: ...` (an ABI mismatch between Lakka's
  system Python 3.11 build and lr-pyxel's embedded PyO3 Python 3.11
  build — same version number, different binary layout). Confirmed so
  far: `_contextvars`, hit by
  [sarananda.pyxapp](https://github.com/kadoyan/sarananda), which fails
  to start entirely because of it. The long-term fix is a fully
  self-contained embedded Python (e.g. via
  [python-build-standalone](https://github.com/astral-sh/python-build-standalone))
  that never touches the system's `lib-dynload`; planned for v2.0.0.
- Per-bank audio/graphics state (`sounds()`, `musics()`, `tones()`,
  `channels()` gain/detune) isn't reset when switching content, unlike
  the color palette / screen size / input state, which are. No
  concrete failure has been observed yet, but the same class of bug is
  possible.
- `Tilemap.blt()` (both the top-level `pyxel.bltm()` and the `Tilemap`
  instance method) only accepts an integer bank index as the source,
  not a `Tilemap` object — unlike `Image.blt()`, which accepts either.

---

## Tested Samples

Confirmed working on real hardware (Raspberry Pi 5 / Lakka):

- Official Pyxel examples: `01_hello_pyxel.py` – `05_color_palette.py`,
  `07_snake.py`, `11_offscreen.py`, `15_tiled_map_file.py`
- `mega_wing.pyxapp` (official example)
- `30sec_of_daylight.pyxapp` (1st Pyxel Jam winner)
- `laser-jetman.pyxapp`
- `cursed_caverns.pyxapp`
- `vortexion.pyxapp`

---

## License

MIT

---

## Credits

- [Pyxel](https://github.com/kitao/pyxel) by kitao
- [Lakka](https://www.lakka.tv/)
- [RetroArch](https://www.retroarch.com/)
