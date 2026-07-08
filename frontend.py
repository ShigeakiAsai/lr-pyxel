# frontend.py - lr-pyxel content browser
# Runs when no content is loaded (content-less boot)
import pyxel
import os

# ROMS_DIR / ROOT_LOCKED are set by lr-pyxel (Rust side) via environment
# variables, based on a compile-time feature flag (Lakka vs. generic
# Linux builds — see v0.11.x de-Lakka-ification). Fall back to Lakka
# defaults if unset (e.g. running frontend.py standalone).
ROMS_DIR = os.path.expanduser(os.environ.get("LR_PYXEL_ROMS_DIR", "/storage/roms/pyxel"))
# On Lakka, ROMS_DIR is a hard root the launcher can't navigate above.
# On a general Linux install, the whole filesystem is navigable instead,
# relying on OS permissions rather than an artificial boundary.
ROOT_LOCKED = os.environ.get("LR_PYXEL_ROMS_ROOT_LOCKED", "1") == "1"
EXTS = (".py", ".pyxapp")

DOWNLOADER_ENTRY = "[Download new games]"
# The preinstalled downloader lives at {system_dir}/pyxel/downloader.pyxapp
# (embedded in the core binary, auto-extracted on first boot — a
# core-owned tool location, separate from ROMS_DIR/user content; see
# v0.11.x de-Lakka-ification). If a copy also exists directly in
# ROMS_DIR, prefer that instead — this lets a newer downloader be
# dropped into ROMS_DIR (e.g. via a future self-update mechanism)
# and take effect immediately, with no core rebuild/redeploy needed.
_SYSTEM_DOWNLOADER_PATH = os.environ.get(
    "LR_PYXEL_DOWNLOADER_PATH", os.path.join(ROMS_DIR, "downloader.pyxapp")
)
_ROMS_DIR_DOWNLOADER_PATH = os.path.join(ROMS_DIR, "downloader.pyxapp")
if os.path.exists(_ROMS_DIR_DOWNLOADER_PATH):
    DOWNLOADER_PATH = _ROMS_DIR_DOWNLOADER_PATH
else:
    DOWNLOADER_PATH = _SYSTEM_DOWNLOADER_PATH

current_dir = ROMS_DIR


def _is_at_root():
    return os.path.normpath(current_dir) == os.path.normpath(ROMS_DIR)


def scan_entries():
    entries = []

    # ".." parent-directory entry, hidden at ROMS_DIR when root-locked,
    # and hidden at the real filesystem root either way (nowhere to go).
    parent = os.path.dirname(current_dir.rstrip("/")) or "/"
    show_parent = parent != current_dir
    if ROOT_LOCKED and _is_at_root():
        show_parent = False
    if show_parent:
        entries.append("..")

    # Only offer the downloader at the ROMS_DIR root, not in subfolders
    if _is_at_root() and os.path.exists(DOWNLOADER_PATH):
        entries.append(DOWNLOADER_ENTRY)

    try:
        names = sorted(os.listdir(current_dir))
        dirs, files = [], []
        for name in names:
            if name.startswith("."):
                continue
            full = os.path.join(current_dir, name)
            if os.path.isdir(full):
                dirs.append(name)
            elif name.endswith(EXTS) and not name.startswith("_") and name != "downloader.pyxapp":
                files.append(name)
        entries.extend(f"[{d}]" for d in dirs)
        entries.extend(files)
    except Exception:
        pass

    return entries


entries = scan_entries()
cursor = 0
MAX_VISIBLE = 12
scroll = 0

# Auto-repeat for UP/DOWN cursor movement: wait REPEAT_HOLD frames after
# the initial press, then repeat every REPEAT_RATE frames at a constant
# interval (no acceleration).
REPEAT_HOLD = 20
REPEAT_RATE = 4


def _enter_dir(new_dir):
    global current_dir, entries, cursor, scroll
    current_dir = new_dir
    entries = scan_entries()
    cursor = 0
    scroll = 0


def update():
    global cursor, scroll

    # Ignore input for first 10 frames to avoid button carry-over from
    # whatever content we just returned from (e.g. downloader.pyxapp
    # calling pyxel.load_content(None) while A is still held/pressed).
    if pyxel.frame_count < 10:
        return

    if pyxel.btnp(pyxel.KEY_UP, REPEAT_HOLD, REPEAT_RATE) or \
       pyxel.btnp(pyxel.GAMEPAD1_BUTTON_DPAD_UP, REPEAT_HOLD, REPEAT_RATE):
        cursor = max(0, cursor - 1)
        if cursor < scroll:
            scroll = cursor

    if pyxel.btnp(pyxel.KEY_DOWN, REPEAT_HOLD, REPEAT_RATE) or \
       pyxel.btnp(pyxel.GAMEPAD1_BUTTON_DPAD_DOWN, REPEAT_HOLD, REPEAT_RATE):
        cursor = min(len(entries) - 1, cursor + 1)
        if cursor >= scroll + MAX_VISIBLE:
            scroll = cursor - MAX_VISIBLE + 1

    # Select the highlighted entry
    if entries and (pyxel.btnp(pyxel.KEY_RETURN) or pyxel.btnp(pyxel.GAMEPAD1_BUTTON_A)):
        name = entries[cursor]
        if name == DOWNLOADER_ENTRY:
            pyxel.load_content(DOWNLOADER_PATH)
        elif name == "..":
            _enter_dir(os.path.dirname(current_dir.rstrip("/")) or "/")
        elif name.startswith("[") and name.endswith("]"):
            _enter_dir(os.path.join(current_dir, name[1:-1]))
        else:
            pyxel.load_content(os.path.join(current_dir, name))


def draw():
    pyxel.cls(0)
    pyxel.text(2, 2, "lr-pyxel", 5)
    pyxel.text(70, 2, "SELECT CONTENT", 13)
    pyxel.line(0, 10, 127, 10, 5)

    if not entries:
        pyxel.text(10, 60, "No files found in", 13)
        pyxel.text(10, 68, current_dir[:24], 6)
        return

    for i in range(MAX_VISIBLE):
        idx = scroll + i
        if idx >= len(entries):
            break
        y = 14 + i * 9
        name = entries[idx]
        if idx == cursor:
            pyxel.rect(0, y - 1, 128, 9, 1)
            col = 10 if name == DOWNLOADER_ENTRY else 7
            pyxel.text(4, y, name[:20], col)
        else:
            if name == DOWNLOADER_ENTRY:
                col = 11
            elif name == "..":
                col = 12
            elif name.startswith("[") and name.endswith("]"):
                col = 9
            elif name.endswith(".pyxapp"):
                col = 6
            else:
                col = 13
            pyxel.text(4, y, name[:20], col)

    # Scrollbar
    if len(entries) > MAX_VISIBLE:
        bar_h = max(4, MAX_VISIBLE * MAX_VISIBLE // len(entries))
        bar_y = 14 + scroll * (MAX_VISIBLE * 9 - bar_h) // (len(entries) - MAX_VISIBLE)
        pyxel.rect(125, 14, 2, MAX_VISIBLE * 9, 1)
        pyxel.rect(125, bar_y, 2, bar_h, 5)

    pyxel.line(0, 117, 127, 117, 5)
    pyxel.text(2, 119, "UP/DOWN:select  A:launch", 5)
