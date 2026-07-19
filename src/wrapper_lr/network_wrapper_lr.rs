//! network_wrapper_lr.rs — Network functions (shells out to the
//! `curl` CLI binary).
//!
//! Upstream has no analog for this at all — desktop Pyxel has no
//! built-in HTTP/network API. This exists purely for lr-pyxel's
//! libretro/handheld embedding (e.g. fetching remote content from
//! within a game), implemented by shelling out to `curl` rather than
//! linking an HTTP client crate.
//!
//! Both functions are prefixed `lr_` (lr_download_file, lr_http_get)
//! rather than left bare — unlike the file-naming `_lr` suffix used
//! throughout wrapper_lr/, this is Python-visible API a script author
//! actually types, and an unprefixed pyxel.download_file() would look
//! exactly like a standard Pyxel function until it broke on desktop
//! Pyxel with AttributeError.

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Network functions (shells out to the `curl` CLI binary)
// ---------------------------------------------------------------------------
// Lakka's embedded Python lacks _socket.so / _ssl.so, so networking can't
// be done from Python (urllib etc.). These wrappers shell out to the
// system `curl` binary instead of linking libcurl into the core, which
// avoids cross-compiling libcurl/OpenSSL for the target device.
//
// Both release the GIL for the duration of the blocking curl call
// (py.detach). Without this, a game calling these from a background
// Python thread (e.g. downloader.py) would still freeze the main
// update()/draw() loop, since PyO3 holds the GIL across the FFI call
// by default and only one Python thread can run at a time regardless.

// lr_download_file(url, save_path) -> bool
// Downloads `url` to `save_path` via `curl -L -s -o save_path url`.
// Returns True on success (curl exit code 0), False otherwise.
// Does not raise on HTTP/network failure — check the return value.
#[pyfunction]
pub fn lr_download_file(py: Python<'_>, url: &str, save_path: &str) -> PyResult<bool> {
    let url = url.to_owned();
    let save_path = save_path.to_owned();
    let ok = py.detach(move || {
        std::process::Command::new("curl")
            .args(["-L", "-s", "-o", &save_path, &url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });
    Ok(ok)
}

// lr_http_get(url) -> str
// Fetches `url` via `curl -L -s url` and returns stdout decoded as UTF-8
// (lossy — invalid byte sequences are replaced, never raises on that).
// Raises OSError only if the `curl` process itself could not be spawned.
#[pyfunction]
pub fn lr_http_get(py: Python<'_>, url: &str) -> PyResult<String> {
    let url = url.to_owned();
    let output = py.detach(move || {
        std::process::Command::new("curl")
            .args(["-L", "-s", &url])
            .output()
    });
    match output {
        Ok(o) => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(e) => Err(pyo3::exceptions::PyException::new_err(e.to_string())),
    }
}

