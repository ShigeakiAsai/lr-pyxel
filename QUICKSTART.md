# lr-pyxel Quickstart (no build required)

Want to run [Pyxel](https://github.com/kitao/pyxel) games through
RetroArch, without installing Lakka and without building anything
yourself? This is for you.

## 1. Download

Go to the [Releases page](https://github.com/ShigeakiAsai/lr-pyxel/releases)
and grab the file matching your CPU (Linux only — see the note below):

- Most PCs/laptops (Intel/AMD): `pyxel_libretro-x86_64.so`
- ARM-based machines (Raspberry Pi, etc.): `pyxel_libretro-aarch64.so`

> **Note**: these builds are for RetroArch running on a regular
> desktop/ARM Linux install — this is *not* the Lakka build. If you're
> already running Lakka, see the main [README](README.md) instead.

## 2. Rename it

Rename the downloaded file to `pyxel_libretro.so` (drop the
`-x86_64`/`-aarch64` part). This isn't strictly required — RetroArch
will happily load a core under any filename if you pick it manually —
but it keeps things tidy and matches what most guides expect.

## 3. Copy it into RetroArch's cores folder

Where that is depends on your Linux install:

- **Flatpak**: `~/.var/app/org.libretro.RetroArch/config/retroarch/cores/`
- **Native package**: often `~/.config/retroarch/cores/` or `/usr/lib/retroarch/cores/` — check RetroArch's own Directory settings if unsure

> lr-pyxel only targets Linux/POSIX RetroArch (see [Known
> Limitations](README.md#known-limitations) in the main README) —
> there's no Windows build, and none is planned.

## 4. Run a Pyxel game

In RetroArch: **Load Core → select `pyxel_libretro.so` → Load Content**
→ pick a `.pyxapp` file.

Don't have one handy? Try any of the official
[Pyxel examples](https://github.com/kitao/pyxel/tree/main/python/pyxel/examples),
or see the [Pyxel User Examples](https://kitao.github.io/pyxel-user-examples/)
gallery for community-made games.

## Something not working?

Check the [FAQ](FAQ.md) first — it covers the most common gotchas
(missing modules, games that don't work here even though they work on
upstream Pyxel, etc.). Still stuck? Open an
[Issue](https://github.com/ShigeakiAsai/lr-pyxel/issues).
