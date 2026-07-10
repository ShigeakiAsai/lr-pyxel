# lr-pyxel

A libretro core that runs [Pyxel](https://github.com/kitao/pyxel) games on RetroArch/Lakka.

[日本語 README](README.ja.md)

> **Status**: v0.12.3 tagged, in active development.

---

## Overview

lr-pyxel embeds CPython and a headless Pyxel engine inside a libretro
core, allowing Pyxel games (`.py` and `.pyxapp`) to run on Lakka/RetroArch
on devices such as the Raspberry Pi 5. The embedded Python's version isn't
fixed at 3.11 — it's whatever `pyxel-core`'s own PyO3 dependency links
against at build time (3.11 on Lakka's cross-compile, but automatically
whatever a native build's local `python3` happens to be otherwise — see
[Build](#build)).

Both gamepad and mouse input are supported: `RETRO_DEVICE_JOYPAD` maps to
Pyxel's keyboard/gamepad API, and `RETRO_DEVICE_MOUSE` (position, left/
right/middle buttons, wheel) maps to Pyxel's mouse API.

There are two ways to launch content, and they support different file
types:

- **Launched with content** (loading a file directly as the core's
  content, e.g. via a RetroArch playlist): only **`.pyxapp`** is
  supported. `.pyxapp` is a self-contained packaged format, which is
  what RetroArch's direct-content-loading model expects — a bare `.py`
  file isn't a well-defined "piece of content" in the same way.
- **Launched with no content**: the built-in launcher starts instead,
  browsing a content folder ("ROMS_DIR", see below). **Only in this
  launcher** can you also run a loose **`.py`** script directly,
  alongside `.pyxapp` files, and navigate into subfolders (a `[folder]`
  entry enters it, `..` goes back up) — the launcher is just listing a
  folder, so both file types and nested folders are equally convenient
  there.

ROMS_DIR itself is resolved differently depending on the build:
- **Lakka builds** (the `lakka` Cargo feature): fixed at
  `/storage/roms/pyxel`, matching the `/storage/roms/<console>`
  convention every other core follows, so games are easy to find (e.g.
  over Samba). The launcher can't navigate above this folder.
- **Non-Lakka builds**: discovered at runtime via the libretro
  `RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY` call (falling back to
  `RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`, then a hardcoded default),
  since there's no equivalent established convention. The launcher can
  navigate the whole filesystem here, relying on OS permissions rather
  than an artificial boundary.

On Lakka builds, an in-core downloader (`downloader.pyxapp`) is embedded
in the core binary and auto-extracted to
`{system_dir}/pyxel/downloader.pyxapp` on first boot (a core-owned tool
location, resolved via `RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`,
separate from ROMS_DIR). It's launchable from the
`[Download new games]` entry at the top of the launcher's file list
(shown only at the ROMS_DIR root, not in subfolders) and can fetch
additional games over HTTP into ROMS_DIR. Non-Lakka builds don't embed
or auto-extract a downloader at all — but if a `downloader.pyxapp`
happens to exist directly in ROMS_DIR (Lakka or not), that copy is
always preferred over the embedded one — this lets an updated
downloader be dropped into ROMS_DIR (e.g. via a future self-update
mechanism) and take effect immediately, with no core rebuild/redeploy
needed.

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

A plain `cargo build --release` (no `--features lakka`) also works on a
native, non-Lakka Linux install with RetroArch (confirmed on Ubuntu
24.04) — see [Non-Lakka prerequisites](#non-lakka-prerequisites) below
for what needs to be installed first. Copy the resulting
`target/release/liblr_pyxel.so` to RetroArch's cores directory,
renaming it to `pyxel_libretro.so` (the `lib` prefix Cargo adds to
`cdylib` outputs isn't part of libretro's naming convention, so it needs
a manual rename either way — `package.mk`'s `makeinstall_target()` does
this same rename for Lakka builds).

### Non-Lakka prerequisites

Building outside the Lakka buildroot (which otherwise provides a
complete, self-contained cross-compile toolchain) needs a few packages
installed up front:

```bash
sudo apt install build-essential cmake clang libclang-dev python3-dev
```

- `build-essential`, `cmake`: needed by `rust-libretro-sys`'s `bindgen`-based
  build script (parses `libretro.h`) and by `pyxel-core`'s statically-linked
  SDL2 build.
- `clang`, `libclang-dev`: `bindgen` itself needs `libclang` to parse C headers.
- `python3-dev` (or the version-specific package, e.g. `python3.12-dev`):
  needed to link against `libpython3.X.so` — see below.

### Dependency notes

- `Cargo.toml` pins `pyxel-core` to the `lr-pyxel` branch of
  [ShigeakiAsai/pyxel](https://github.com/ShigeakiAsai/pyxel) (a fork of
  upstream Pyxel), **not** the default branch — the fork's `main` branch is
  kept clean for upstream contributions (see PR
  [kitao/pyxel#718](https://github.com/kitao/pyxel/pull/718)). After
  pulling changes to that branch, run `cargo update -p pyxel-core` before
  rebuilding.
- The `lakka` Cargo feature gates Lakka/LibreELEC-specific defaults (see
  [Overview](#overview)) and is **not enabled by default** — Lakka
  builds must opt in explicitly. `package.mk` passes `--features lakka`
  to `cargo build`; a plain `cargo build` (e.g. for a generic Linux
  RetroArch) gets the non-Lakka defaults instead.
- Networking (`pyxel.download_file()` / `pyxel.http_get()`) shells out to
  the system `curl` binary rather than linking libcurl, so the target
  device needs `curl` on its `PATH`.
- `retro_init()` re-`dlopen()`s `libpython3.X.so` with `RTLD_GLOBAL`
  (via the `libc` crate) before starting the embedded interpreter. This
  is needed because RetroArch loads this core (and in turn its
  `libpython3.X.so` dependency) without `RTLD_GLOBAL`, which otherwise
  leaves that library's symbols invisible to further `dlopen()` calls
  CPython itself makes when importing compiled extension modules — a
  well-known pitfall of embedding CPython inside a plugin-style shared
  library. Without this, some compiled standard-library extensions fail
  to load with `undefined symbol: ...` even though they work fine
  outside lr-pyxel.
- The exact Python version (`X` above) isn't hardcoded: `build.rs` uses
  the `pyo3-build-config` crate (the same interpreter-discovery logic
  PyO3 itself uses) to detect whatever Python `pyxel-core`'s PyO3
  dependency actually linked against, and bakes it into a
  `LR_PYXEL_PYTHON_VERSION` compile-time value. This is `3.11` on Lakka
  (cross-compiled, `package.mk` sets `PYO3_CROSS_PYTHON_VERSION=3.11`),
  but automatically becomes whatever a native build's local `python3` is
  otherwise (e.g. `3.12` on Ubuntu 24.04), with no code changes needed
  either place.

### Supported platforms

lr-pyxel has only been built and tested on Linux/POSIX RetroArch
(Lakka, and native desktop Linux). Windows, Android, and iOS aren't
supported, and there are no plans to add them — on those platforms,
installing [upstream Pyxel](https://github.com/kitao/pyxel) natively is
easier and more direct than going through a libretro core, which only
really earns its keep on platforms (like Lakka) where Pyxel can't
otherwise run at all.

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

---

## Known Issues

- Per-bank audio/graphics state (`sounds()`, `musics()`, `tones()`,
  `channels()` gain/detune) isn't reset when switching content, unlike
  the color palette / screen size / input state, which are. No
  concrete failure has been observed yet, but the same class of bug is
  possible.
- `Tilemap.blt()` (both the top-level `pyxel.bltm()` and the `Tilemap`
  instance method) only accepts an integer bank index as the source,
  not a `Tilemap` object — unlike `Image.blt()`, which accepts either.
- The Python API surface isn't independently verified complete against
  upstream Pyxel — several gaps (e.g. `pyxel.screen`, `pyxel.colors`/
  `pyxel.channels` list methods like `append()`/`from_list()`) have
  been found and fixed via real third-party games surfacing
  `AttributeError`s that plain testing hadn't caught. If a script hits
  one, please report it.

---

## Tested Samples

Confirmed working on real hardware (Raspberry Pi 5 / Lakka) or on a
native Linux RetroArch install:

- Official Pyxel examples: `01_hello_pyxel.py` – `05_color_palette.py`,
  `07_snake.py`, `11_offscreen.py`, `15_tiled_map_file.py`
- `mega_wing.pyxapp` (official example)
- `30sec_of_daylight.pyxapp` (1st Pyxel Jam winner)
- `laser-jetman.pyxapp`
- `cursed_caverns.pyxapp`
- `vortexion.pyxapp`
- `sarananda.pyxapp`
- `finardry.pyxapp`
- `Braveforce-LDV_Demo.pyxapp`
- `LastEmulator.pyxapp` (mouse-driven; confirmed at its native 720x480)
- `dungeon-antiqua.pyxapp`, `dungeon-antiqua2.pyxapp` (confirmed up to
  1024x960), `dungeon-antiqua-v2.pyxapp`

---

## License

MIT

---

## Credits

- [Pyxel](https://github.com/kitao/pyxel) by kitao
- [Lakka](https://www.lakka.tv/)
- [RetroArch](https://www.retroarch.com/)
