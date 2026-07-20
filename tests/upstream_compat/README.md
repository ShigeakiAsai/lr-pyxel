# Upstream Pyxel Compatibility Suite

This folder contains `lr_test_upstream_suite.py`: a harness that runs
[Pyxel](https://github.com/kitao/pyxel)'s own `pytest` test files —
`test_channel.py`, `test_audio.py`, `test_audio_render.py`,
`test_audio_semantics.py`, `test_errors.py`, `test_font.py`,
`test_graphics.py`, `test_image.py`, `test_input.py`, `test_math.py`,
`test_music.py`, `test_resize.py`, `test_resource_io.py`,
`test_sequences.py`, `test_sound.py`, `test_system.py`,
`test_tilemap.py`, `test_tone.py`, `test_utils.py` — **verbatim,
unmodified**, against lr-pyxel's embedded engine.

## The 19 test files are not bundled here

This folder only contains the harness (lr-pyxel's own code). The 19
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
press **X**. This fetches the 19 test files (see list above) plus
`_assertions.py` (a small upstream test helper some of them import —
see "Fake modules" below) from upstream Pyxel, and
`lr_test_upstream_suite.py` from this repo's `main` branch, straight
into `ROMS_DIR`. Once done, launch `lr_test_upstream_suite.py` from the
normal launcher like any other script.

Note: the harness itself fakes out `_assertions` in `sys.modules` (see
below), so `_assertions.py` doesn't actually need to be fetched or
present on disk for the suite to run — it's listed here only in case a
manual/non-Lakka setup wants the real upstream file for reference.

## Running elsewhere (native/non-Lakka builds)

Fetch the 19 `test_*.py` files from upstream Pyxel's `python/tests/`
directory yourself (matching the list above), place them next to
`lr_test_upstream_suite.py`, and run it directly — `pyxel run
lr_test_upstream_suite.py` or via a `.pyxapp`.

## Why a custom harness instead of `pytest` itself

Lakka has no `pip`, and lr-pyxel embeds CPython directly via PyO3 —
`pytest` itself isn't installed or installable in that environment. The
harness registers a minimal fake `pytest` module in `sys.modules`
(`pytest.approx`, `pytest.raises`, `pytest.fixture`, `pytest.mark.parametrize`)
before `exec`-ing each test file verbatim, then discovers and calls
every `Test*.test_*` method by hand, replicating just enough of
`conftest.py`'s autouse fixture (state reset between tests) and
pytest's builtin `capfd` fixture to run. `exec`'d files also get a
`__file__` entry in their namespace, since a couple of upstream files
use it at module scope (e.g. `Path(__file__).parent` to locate
reference assets) — without it, those files fail to even load.

`@pytest.mark.parametrize`-decorated tests are expanded into one
sub-result per row (`test_name[row_repr]`) rather than run once. A row
that also needs an actual unsupported fixture (e.g. a parametrized test
that takes `tmp_path` too) is still reported as skipped, same as any
other fixture-dependent test.

## Fake modules

Besides `pytest`, the harness also fakes upstream's own small shared
test helper module, `_assertions` (`python/tests/_assertions.py`),
which `test_errors.py`, `test_resize.py`, `test_resource_io.py`,
`test_sequences.py`, and `test_system.py` import for its
`raises_exact(exception_type, message)` context manager — a stricter
`pytest.raises()` that also asserts the exception's message matches
exactly. Without this fake, those five files fail to even load
(`No module named '_assertions'`), not just fail individual tests —
this was caught by this harness itself after upstream added the
module post-dating this suite's original version. Reimplemented here
on the same `_RaisesContext` the fake `pytest.raises()` already uses,
since the logic (type + exact message check) is identical either way.

## Known exclusions

Of upstream's test files, the following aren't run here:

- `test_input.py` was previously excluded too (its `set_btn()`-style
  test-only input-injection APIs were assumed unimplemented), but
  lr-pyxel added mouse/keyboard support later in development without
  this list being revisited — re-included now that those APIs exist.

- Files that spawn subprocesses expecting a standalone `import pyxel`
  (`test_apps.py`, `test_examples.py`, `test_run_examples.py`) or a
  dev server (`test_start_showcase.py`) don't apply to lr-pyxel's
  embedded-module architecture.
- `test_editor.py` and `test_cli.py` exercise `pyxel.cli`/the resource
  editor, unavailable headless.
- `test_import_hook.py`, `test_format_prose.py`, `test_generate_docs.py`,
  and `test_update_version.py` are the upstream repo's own dev-tooling
  tests (source-formatting checks, doc-generation scripts, version-bump
  scripts) — they validate upstream's own repository maintenance
  scripts, not the `pyxel` runtime API, so there's nothing for
  lr-pyxel's embedded engine to be compatible with here.
- `test_audio_render.py` mostly needs `tmp_path`/`update_references`
  fixtures this harness doesn't support, so most of its tests report as
  SKIPPED rather than running — it's included anyway since a few of its
  cases still execute, and SKIPs are harmless, informational entries
  rather than failures.

## Output

Results are shown on screen (scrollable with UP/DOWN) and written in
full to `test_results.txt` next to the test files.
