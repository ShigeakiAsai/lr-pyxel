# Upstream Pyxel Compatibility Suite

This folder contains `lr_test_upstream_suite.py`: a harness that runs
[Pyxel](https://github.com/kitao/pyxel)'s own `pytest` test files —
`test_channel.py`, `test_audio.py`, `test_errors.py`, `test_font.py`,
`test_graphics.py`, `test_image.py`, `test_math.py`, `test_music.py`,
`test_resize.py`, `test_resource_io.py`, `test_sequences.py`,
`test_sound.py`, `test_system.py`, `test_tilemap.py`, `test_tone.py`,
`test_utils.py` — **verbatim, unmodified**, against lr-pyxel's embedded
engine.

## The 16 test files are not bundled here

This folder only contains the harness (lr-pyxel's own code). The 16
`test_*.py` files themselves are fetched at runtime, straight from
upstream Pyxel's own `main` branch
(`https://raw.githubusercontent.com/kitao/pyxel/main/python/tests/`),
copyright kitao and licensed under the
[MIT License](https://github.com/kitao/pyxel/blob/main/LICENSE) — not
lr-pyxel's code, and not a copy lr-pyxel maintains.

This is deliberate: this suite isn't meant to be run casually/often (in
which case a stable, pinned copy would matter for reproducibility) —
it's meant to also catch upstream Pyxel API drift between lr-pyxel
updates. A result that changes with no lr-pyxel code change is itself
the signal, not noise. If diagnosing a failure needs a stable target
instead, fetch a specific upstream commit/tag manually rather than
`main`.

## Running on Lakka: the hidden downloader combo

`downloader.pyxapp` (the in-core content downloader) has a hidden
combo, deliberately undocumented in its own UI: hold **L + R**, then
press **X**. This fetches the 16 test files from upstream Pyxel and
`lr_test_upstream_suite.py` from this repo's `main` branch, straight
into `ROMS_DIR`. Once done, launch `lr_test_upstream_suite.py` from the
normal launcher like any other script.

## Running elsewhere (native/non-Lakka builds)

Fetch the 16 `test_*.py` files from upstream Pyxel's `python/tests/`
directory yourself (matching the list above), place them next to
`lr_test_upstream_suite.py`, and run it directly — `pyxel run
lr_test_upstream_suite.py` or via a `.pyxapp`.

## Why a custom harness instead of `pytest` itself

Lakka has no `pip`, and lr-pyxel embeds CPython directly via PyO3 —
`pytest` itself isn't installed or installable in that environment. The
harness registers a minimal fake `pytest` module in `sys.modules`
(`pytest.approx`, `pytest.raises`, `pytest.fixture`) before `exec`-ing
each test file verbatim, then discovers and calls every `Test*.test_*`
method by hand, replicating just enough of `conftest.py`'s autouse
fixture (state reset between tests) and pytest's builtin `capfd`
fixture to run.

## Known exclusions

Of upstream's 22 test files, a handful aren't run here:

- Files that spawn subprocesses expecting a standalone `import pyxel`
  don't apply to lr-pyxel's embedded-module architecture.
- Files exercising `pyxel.cli` don't apply headless.
- `test_input.py` exclusively tests `pyxel.set_btn()`-style test-only
  input-injection APIs that lr-pyxel has no plan to implement (see
  "Known Limitations" in the main README) — running it would just
  report that same permanent, already-documented gap as ~20 failures,
  not new signal.

## Output

Results are shown on screen (scrollable with UP/DOWN) and written in
full to `test_results.txt` next to the test files.
