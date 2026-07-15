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

## Q. My game's save data keeps resetting (or crashing)

A. lr-pyxel extracts a `.pyxapp` to a fresh temporary directory on
*every* load — not just once per RetroArch session, but every single
time that content is loaded, even reloading the exact same `.pyxapp`
without restarting RetroArch triggers a new extraction. A game that
reads/writes its own save or config file with a bare relative path
(e.g. `open("save.json", "w")`) instead of upstream Pyxel's own
`user_data_dir(vendor_name, app_name)` mechanism will hit one of three
symptoms, depending on how it's written:

- **(a) A template save file is bundled in the `.pyxapp`.** The game
  resets to that template's contents on every load — confirmed
  on-device: even reloading the same `.pyxapp` without restarting
  RetroArch was enough to reset a high score, no RetroArch or system
  restart needed.
- **(b) The game handles a missing file gracefully** (e.g.
  `try`/`except FileNotFoundError`, falling back to a default). Same
  reset-every-time behavior as (a), just without needing a bundled
  template — no crash, but nothing persists either.
- **(c) Neither of the above.** The first read of a file that was
  never bundled and has no fallback logic raises (e.g.
  `FileNotFoundError`) — since a *fresh* extraction happens on every
  load, this isn't a one-time failure that then works — it **crashes
  on every single load**.

### For game authors

If your game needs save data to actually persist under lr-pyxel, use
upstream Pyxel's own
[`user_data_dir(vendor_name, app_name)`](https://github.com/kitao/pyxel)
mechanism instead of a bare relative path. That mechanism resolves to
a real, persistent, per-application directory rather than the
game's own temporary extraction folder, and is confirmed to persist
correctly across reloads under lr-pyxel.

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
