# frontend.py - lr-pyxel content browser
# Runs when no content is loaded (content-less boot)
import pyxel
import os

ROMS_DIR = "/storage/roms/pyxel"
EXTS = (".py", ".pyxapp")

def scan_files():
    files = []
    try:
        for name in sorted(os.listdir(ROMS_DIR)):
            if name.endswith(EXTS) and not name.startswith("_"):
                files.append(name)
    except Exception:
        pass
    return files

DOWNLOADER_ENTRY = "[Download new games]"
DOWNLOADER_PATH  = "/storage/roms/pyxel/downloader.pyxapp"

# Only show downloader entry if the .pyxapp exists
_has_downloader = os.path.exists(DOWNLOADER_PATH)
files = ([DOWNLOADER_ENTRY] if _has_downloader else []) + scan_files()
cursor = 0
MAX_VISIBLE = 12
scroll = 0

# Auto-repeat for UP/DOWN cursor movement: wait REPEAT_HOLD frames after
# the initial press, then repeat every REPEAT_RATE frames at a constant
# interval (no acceleration).
REPEAT_HOLD = 20
REPEAT_RATE = 4

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
        cursor = min(len(files) - 1, cursor + 1)
        if cursor >= scroll + MAX_VISIBLE:
            scroll = cursor - MAX_VISIBLE + 1

    # Launch selected content
    if files and (pyxel.btnp(pyxel.KEY_RETURN) or pyxel.btnp(pyxel.GAMEPAD1_BUTTON_A)):
        if files[cursor] == DOWNLOADER_ENTRY:
            pyxel.load_content(DOWNLOADER_PATH)
        else:
            path = ROMS_DIR + "/" + files[cursor]
            pyxel.load_content(path)

def draw():
    pyxel.cls(0)
    pyxel.text(2, 2, "lr-pyxel", 5)
    pyxel.text(70, 2, "SELECT CONTENT", 13)
    pyxel.line(0, 10, 127, 10, 5)

    if not files:
        pyxel.text(10, 60, "No files found in", 13)
        pyxel.text(10, 68, ROMS_DIR, 6)
        return

    for i in range(MAX_VISIBLE):
        idx = scroll + i
        if idx >= len(files):
            break
        y = 14 + i * 9
        name = files[idx]
        if idx == cursor:
            pyxel.rect(0, y - 1, 128, 9, 1)
            col = 10 if name == DOWNLOADER_ENTRY else 7
            pyxel.text(4, y, name[:20], col)
        else:
            if name == DOWNLOADER_ENTRY:
                col = 11
            elif name.endswith(".pyxapp"):
                col = 6
            else:
                col = 13
            pyxel.text(4, y, name[:20], col)

    # Scrollbar
    if len(files) > MAX_VISIBLE:
        bar_h = max(4, MAX_VISIBLE * MAX_VISIBLE // len(files))
        bar_y = 14 + scroll * (MAX_VISIBLE * 9 - bar_h) // (len(files) - MAX_VISIBLE)
        pyxel.rect(125, 14, 2, MAX_VISIBLE * 9, 1)
        pyxel.rect(125, bar_y, 2, bar_h, 5)

    pyxel.line(0, 117, 127, 117, 5)
    pyxel.text(2, 119, "UP/DOWN:select  A:launch", 5)
