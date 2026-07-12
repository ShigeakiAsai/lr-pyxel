# lr-pyxel FAQ

[日本語 FAQ](FAQ.ja.md) | [README](README.md)

---

## Q. My game shows "Missing module: xxx" and won't start

A. The game depends on a third-party Python module (e.g. `numpy`)
beyond `pyxel` itself, and lr-pyxel doesn't have it. lr-pyxel targets
running `pyxel` on its own — supporting arbitrary third-party modules
is out of scope. Whether to install one is up to you.

On Lakka, `pip` isn't available. Instead of `pip install`, you can try
placing the module's wheel contents in one of these locations, which
may make it work:

- **SSH**: extract into `/tmp/system/pyxel/site-packages/` (this is
  under `/tmp`, but it's RetroArch's own System Directory, persisted
  across RetroArch/system reboots — it doesn't get wiped)
- **File browser (Samba)**: `\\LAKKA\System\pyxel\site-packages`
  (e.g. from Windows Explorer — Samba needs to be enabled first, in
  Lakka's settings)

This isn't a supported, guaranteed-to-work feature of lr-pyxel — it's
offered as reference information only.

## Q. A specific game doesn't work. Who should I report it to?

A. First, check whether the same issue reproduces on upstream Pyxel
(a `pip install pyxel` environment).

- If it also happens on upstream Pyxel → report it to the game's own
  author
- If it only happens on lr-pyxel → report it to the
  [lr-pyxel issue tracker](https://github.com/ShigeakiAsai/lr-pyxel/issues)

Even a game that works fine on upstream Pyxel may not work on
lr-pyxel, due to lr-pyxel-specific constraints (no third-party module
support, save data persistence conditions, etc. — see
[Known Limitations](README.md#known-limitations) in the README).
