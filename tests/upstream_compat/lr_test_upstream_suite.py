import contextlib
import inspect
import io
import sys
import types

import pyxel

# 160x120 matches upstream's own conftest.py pyxel_init fixture (the
# size every one of their tests implicitly assumes, e.g.
# test_system.py's `assert pyxel.width == 160`). Running the actual
# test files at any other size produces false failures on every
# width/height-sensitive assertion — confirmed on-device: at 280x220,
# test_width/test_height/test_screen_is_image all failed purely
# because of this mismatch, not any real lr-pyxel gap. Resized larger
# below, after all tests have finished, purely for a more readable
# results screen — the actual test execution happens at 160x120.
pyxel.init(160, 120, title="lr-pyxel upstream compat suite")

# Upstream Pyxel's own pytest files (python/tests/*.py), run verbatim
# (no modifications) against lr-pyxel's embedded engine. See the
# "Known Issues" section of README.md — this suite is what backs that
# claim. Excluded from the full upstream set: files that spawn
# subprocesses expecting a standalone `import pyxel`, exercise
# `pyxel.cli`, or are the repo's own dev-tooling/meta tests (doc
# generation, source-formatting checks, version-bump scripts) — see
# README.md's "Known exclusions" for the full rationale per file.
#
# test_input.py was previously excluded too (its set_btn()-style
# test-only input-injection APIs were assumed unimplemented), but
# lr-pyxel added mouse/keyboard support (set_btn/set_btnv/
# set_mouse_pos/set_input_text/set_dropped_files, input_wrapper_lr.rs)
# later in development without this exclusion list being revisited —
# re-added now that those APIs actually exist.
TEST_FILES = [
    "test_channel.py",
    "test_tone.py",
    "test_sound.py",
    "test_music.py",
    "test_tilemap.py",
    "test_math.py",
    "test_utils.py",
    "test_audio.py",
    "test_audio_semantics.py",
    "test_audio_render.py",
    "test_errors.py",
    "test_font.py",
    "test_graphics.py",
    "test_image.py",
    "test_input.py",
    "test_resize.py",
    "test_resource_io.py",
    "test_sequences.py",
    "test_system.py",
]

# --- Fake "pytest" module -----------------------------------------------
# Registered in sys.modules BEFORE each test file is exec'd, so their
# `import pytest` and pytest.approx()/raises()/fixture usage are
# satisfied without needing pytest itself (not installed under Lakka's
# embedded Python) or editing any test file.
class _Approx:
    def __init__(self, value, abs=None, rel=None):
        self.value = value
        self.abs = abs if abs is not None else 1e-6
        self.rel = rel

    def __eq__(self, other):
        return abs(other - self.value) <= self.abs

    def __repr__(self):
        return f"approx({self.value})"


class _RaisesContext:
    def __init__(self, expected_exception, match=None):
        self.expected_exception = expected_exception
        self.match = match
        self.value = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, tb):
        if exc_type is None:
            raise AssertionError(f"DID NOT RAISE {self.expected_exception}")
        if not issubclass(exc_type, self.expected_exception):
            return False
        if self.match is not None:
            import re
            if not re.search(self.match, str(exc_value)):
                raise AssertionError(
                    f"Regex pattern {self.match!r} does not match {str(exc_value)!r}"
                )
        self.value = exc_value
        return True


def _raises(expected_exception, match=None):
    return _RaisesContext(expected_exception, match=match)


def _fixture(*args, **kwargs):
    # No-op decorator: @pytest.fixture and @pytest.fixture(...) both
    # just return the wrapped function unchanged. Fixtures beyond
    # conftest.py's autouse ones aren't actually invoked by this
    # harness, so a local @pytest.fixture-decorated function just sits
    # unused in the test module's namespace.
    if len(args) == 1 and callable(args[0]) and not kwargs:
        return args[0]

    def decorator(fn):
        return fn
    return decorator


# --- Fake "pytest.mark.parametrize" -------------------------------------
# A handful of upstream test files (test_errors.py, test_resource_io.py,
# test_system.py) use @pytest.mark.parametrize to run one test body
# against a table of inputs. Previously the fake pytest module had no
# `mark` attribute at all, so merely *importing* one of these files
# raised AttributeError on the decorator line — the whole file failed
# to load, not just the parametrized tests. This stores each
# decorated function's (names, argvalues) on the function object
# itself; run_test_file()'s discovery loop below expands it into one
# sub-result per row instead of calling the method directly.
def _parametrize(argnames, argvalues, ids=None, **_ignored_kwargs):
    names = (
        [n.strip() for n in argnames.split(",")]
        if isinstance(argnames, str)
        else list(argnames)
    )

    def decorator(fn):
        fn._pytest_parametrize = (names, list(argvalues), ids)
        return fn
    return decorator


class _Mark:
    parametrize = staticmethod(_parametrize)


_fake_pytest = types.ModuleType("pytest")
_fake_pytest.approx = _Approx
_fake_pytest.raises = _raises
_fake_pytest.fixture = _fixture
_fake_pytest.mark = _Mark()
sys.modules["pytest"] = _fake_pytest


# --- Fake "_assertions" module -------------------------------------------
# Registered the same way as the fake "pytest" module above. Upstream
# added this small shared helper (python/tests/_assertions.py) after
# this harness was first written — test_errors.py, test_resize.py,
# test_resource_io.py, test_sequences.py, and test_system.py all do
# `from _assertions import raises_exact`, so without this, those five
# files fail to even load ("No module named '_assertions'"), not just
# fail individual tests.
#
# raises_exact() is upstream's own stricter version of pytest.raises():
# it additionally asserts the raised exception's message matches
# exactly, not just its type. Reimplemented here on top of the same
# _RaisesContext used for the fake pytest.raises() above, since the
# logic (type + exact message check) is identical either way.
@contextlib.contextmanager
def _raises_exact(exception_type, message):
    with _raises(exception_type) as exc_info:
        yield exc_info
    if type(exc_info.value) is not exception_type:
        raise AssertionError(
            f"DID NOT RAISE exactly {exception_type} (raised {type(exc_info.value)} instead)"
        )
    if str(exc_info.value) != message:
        raise AssertionError(
            f"Exception message {str(exc_info.value)!r} != expected {message!r}"
        )


_fake_assertions = types.ModuleType("_assertions")
_fake_assertions.raises_exact = _raises_exact
sys.modules["_assertions"] = _fake_assertions


# --- Minimal per-test reset, matching conftest.py's autouse fixture ----
def reset_pyxel_state():
    # Restore the canvas size upstream's own conftest.py fixture
    # assumes for every single test (160x120) — confirmed necessary
    # on-device: test_resize.py legitimately changes pyxel.width/
    # height as part of what it's testing, and without resetting it
    # back here, that change leaked into every later test file (in
    # TEST_FILES order, test_system.py runs after test_resize.py),
    # making test_system.py's own width/height assertions fail for a
    # reason that had nothing to do with test_system.py itself.
    pyxel.resize(160, 120)
    pyxel.clip()
    pyxel.camera()
    pyxel.pal()
    pyxel.dither(1.0)
    pyxel.rseed(0)
    pyxel.nseed(0)


# --- Minimal capfd-like fixture (real pytest builtin, not something
# conftest.py defines) — captures stdout printed during a single test
# via redirection, exposed the same way pytest's capfd.readouterr() is:
# an object with .out/.err attributes.
class _CaptureResult:
    def __init__(self, out, err):
        self.out = out
        self.err = err


class _Capfd:
    def __init__(self):
        self._buf = io.StringIO()

    def readouterr(self):
        value = self._buf.getvalue()
        self._buf.truncate(0)
        self._buf.seek(0)
        return _CaptureResult(value, "")


def run_test_file(filename):
    file_results = []
    try:
        with open(filename, encoding="utf-8") as f:
            source = f.read()
    except OSError as e:
        return [(f"{filename} (FILE)", False, str(e))]

    # Some upstream files (e.g. test_audio_render.py) use `__file__` at
    # module scope (Path(__file__).parent) to locate reference assets.
    # exec()'d code has no `__file__` unless the namespace supplies one.
    namespace = {"__file__": filename}
    try:
        exec(compile(source, filename, "exec"), namespace)
    except Exception as e:
        return [(f"{filename} (LOAD)", False, str(e))]

    for name, obj in list(namespace.items()):
        if name.startswith("Test") and isinstance(obj, type):
            instance = obj()
            for method_name in dir(instance):
                if method_name.startswith("test_"):
                    method = getattr(instance, method_name)
                    label = f"{filename[:-3]}.{name}.{method_name}"
                    parametrize = getattr(method, "_pytest_parametrize", None)
                    if parametrize is not None:
                        file_results.extend(
                            _run_parametrized(method, parametrize, label)
                        )
                        continue
                    reset_pyxel_state()
                    params = list(inspect.signature(method).parameters)
                    unsupported = [p for p in params if p != "capfd"]
                    try:
                        if unsupported:
                            file_results.append(
                                (label, None, f"unsupported fixture(s): {unsupported}")
                            )
                            continue
                        if "capfd" in params:
                            capfd = _Capfd()
                            with contextlib.redirect_stdout(capfd._buf):
                                method(capfd)
                        else:
                            method()
                        file_results.append((label, True, ""))
                    except Exception as e:
                        file_results.append((label, False, str(e)))
    return file_results


def _run_parametrized(method, parametrize, label):
    # Expands one @pytest.mark.parametrize-decorated test into one
    # sub-result per row. A row can still combine parametrize'd names
    # with an actual (unsupported) fixture like tmp_path — e.g.
    # test_resource_io.py's test_unreadable_legacy_entry_is_not_treated
    # _as_missing takes both `tmp_path` and the parametrized `entry` —
    # so any leftover parameter that isn't one of the parametrize names
    # and isn't capfd still marks that row as skipped, same as a
    # plain (non-parametrized) test would be.
    names, argvalues, ids = parametrize
    params = list(inspect.signature(method).parameters)
    leftover = [p for p in params if p not in names and p != "capfd"]
    results = []
    for row_index, values in enumerate(argvalues):
        row_id = ids[row_index] if ids else repr(values)
        row_label = f"{label}[{row_id}]"
        if leftover:
            results.append(
                (row_label, None, f"unsupported fixture(s): {leftover}")
            )
            continue
        reset_pyxel_state()
        kwargs = {names[0]: values} if len(names) == 1 else dict(zip(names, values))
        try:
            if "capfd" in params:
                capfd = _Capfd()
                with contextlib.redirect_stdout(capfd._buf):
                    method(capfd=capfd, **kwargs)
            else:
                method(**kwargs)
            results.append((row_label, True, ""))
        except Exception as e:
            results.append((row_label, False, str(e)))
    return results


all_results = []
for fname in TEST_FILES:
    all_results.extend(run_test_file(fname))

# All test files have finished running at 160x120 (matching upstream's
# own expectations — see the pyxel.init() comment above). Resize now,
# purely for a more readable results-browsing screen; nothing past
# this point touches pyxel.width/height in a way any test asserts on.
pyxel.resize(280, 220)

passed = sum(1 for _, ok, _ in all_results if ok is True)
failed = sum(1 for _, ok, _ in all_results if ok is False)
skipped = sum(1 for _, ok, _ in all_results if ok is None)
failures = [(n, e) for n, ok, e in all_results if ok is False]

# Write full results to a plain text file next to the test files, so
# they can be reviewed via `cat`/a text editor instead of scrolling a
# tiny in-game screen.
with open("test_results.txt", "w", encoding="utf-8") as f:
    f.write(
        f"TOTAL: {passed} passed, {failed} failed, {skipped} skipped "
        f"(of {len(all_results)} tests across {len(TEST_FILES)} files)\n\n"
    )
    f.write("=== FAILURES ===\n")
    for name, ok, err in all_results:
        if ok is False:
            f.write(f"NG   {name}\n     {err}\n")
    f.write("\n=== SKIPPED ===\n")
    for name, ok, err in all_results:
        if ok is None:
            f.write(f"SKIP {name}\n     {err}\n")
    f.write("\n=== ALL RESULTS ===\n")
    for name, ok, err in all_results:
        label = "OK" if ok is True else ("SKIP" if ok is None else "NG")
        f.write(f"{label:4} {name}\n")

scroll = 0
MAX_LINES = 24


def update():
    global scroll
    if pyxel.btnp(pyxel.KEY_Q):
        pyxel.quit()
    if pyxel.btnp(pyxel.KEY_DOWN) or pyxel.btnp(pyxel.GAMEPAD1_BUTTON_DPAD_DOWN):
        scroll = min(scroll + 1, max(0, len(failures) - 1))
    if pyxel.btnp(pyxel.KEY_UP) or pyxel.btnp(pyxel.GAMEPAD1_BUTTON_DPAD_UP):
        scroll = max(scroll - 1, 0)


def draw():
    pyxel.cls(1)
    pyxel.text(4, 4, f"TOTAL: {passed} passed, {failed} failed, {skipped} skipped", 7)
    pyxel.text(4, 12, f"(of {len(all_results)} tests across {len(TEST_FILES)} files)", 6)
    pyxel.text(4, 20, "Full results written to test_results.txt", 10)
    y = 32
    if not failures:
        pyxel.text(4, y, "No failures!", 11)
    else:
        pyxel.text(4, y, "Failures (UP/DOWN to scroll):", 8)
        y += 10
        for name, err in failures[scroll:scroll + MAX_LINES]:
            pyxel.text(4, y, name[:44], 8)
            y += 7
            pyxel.text(8, y, err[:44], 8)
            y += 8


pyxel.run(update, draw)
