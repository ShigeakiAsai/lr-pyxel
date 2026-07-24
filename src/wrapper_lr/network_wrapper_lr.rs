//! network_wrapper_lr.rs — Network functions (via the `ureq` crate,
//! using rustls — no OpenSSL/libcurl cross-compilation involved).
//!
//! Upstream has no analog for this at all — desktop Pyxel has no
//! built-in HTTP/network API. This exists purely for lr-pyxel's
//! libretro/handheld embedding (e.g. fetching remote content from
//! within a game).
//!
//! Both functions are prefixed `lr_` (lr_download_file, lr_http_get)
//! rather than left bare — unlike the file-naming `_lr` suffix used
//! throughout wrapper_lr/, this is Python-visible API a script author
//! actually types, and an unprefixed pyxel.download_file() would look
//! exactly like a standard Pyxel function until it broke on desktop
//! Pyxel with AttributeError.
//!
//! Previously shelled out to the system `curl` binary, specifically
//! to avoid cross-compiling libcurl/OpenSSL for the target device
//! (see git history for the original rationale). Replaced with the
//! `ureq` crate instead: its `rustls` backend (ureq's own default
//! feature) is a pure-Rust TLS stack with no C library to cross-
//! compile at all — the original problem this design was avoiding
//! simply doesn't apply to it. Also removes the runtime dependency on
//! `curl` actually being present on the target's PATH, and gives
//! finer-grained error information than a bare process exit code.

use std::io::Read;

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Network functions (via ureq / rustls)
// ---------------------------------------------------------------------------
// Both release the GIL for the duration of the blocking network call
// (py.detach). Without this, a game calling these from a background
// Python thread (e.g. downloader.py) would still freeze the main
// update()/draw() loop, since PyO3 holds the GIL across the FFI call
// by default and only one Python thread can run at a time regardless.
//
// http_status_as_error(false): matches the previous curl-based
// implementation's behavior exactly — a 4xx/5xx response is not
// treated as a failure here, the response body is still returned/
// saved as-is. Scripts that care about the status code get it via
// whatever the response body itself says (e.g. an API's own JSON
// error payload), same as before. Without this, ureq's own default
// (treating any non-2xx as an Err) would be a silent behavior change
// from what downloader.py and any other existing script already
// expects.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

// lr_download_file(url, save_path) -> bool
// Downloads `url` to `save_path`, following redirects (ureq's own
// default, matching curl's -L). Returns True on success, False
// otherwise — network failure, unwritable save_path, and non-2xx
// responses (see http_status_as_error above — an HTTP error still
// gets its response body streamed to save_path, and this still
// reports True) are the only distinctions preserved from before: this
// function was already documented as "doesn't raise on HTTP/network
// failure, check the return value", which only genuinely covered
// network-level failures even under curl (a 404 page saved
// successfully to disk was already "True" under curl -s too, since
// curl itself doesn't fail its own exit code on HTTP-level errors
// without -f).
#[pyfunction]
pub fn lr_download_file(py: Python<'_>, url: &str, save_path: &str) -> PyResult<bool> {
    let url = url.to_owned();
    let save_path = save_path.to_owned();
    let ok = py.detach(move || -> bool {
        let Ok(mut response) = agent().get(&url).call() else { return false; };
        let Ok(mut file) = std::fs::File::create(&save_path) else { return false; };
        let mut reader = response.body_mut().as_reader();
        std::io::copy(&mut reader, &mut file).is_ok()
    });
    Ok(ok)
}

// lr_http_get(url) -> str
// Fetches `url` and returns the response body decoded as UTF-8
// (lossy — invalid byte sequences are replaced, never raises on
// that). Raises OSError only if the request itself could not be made
// at all (DNS failure, connection refused, TLS handshake failure,
// etc.) — matches the previous curl-based version's "only raises if
// the curl process itself could not be spawned" scope as closely as
// the two underlying mechanisms allow.
#[pyfunction]
pub fn lr_http_get(py: Python<'_>, url: &str) -> PyResult<String> {
    let url = url.to_owned();
    let result = py.detach(move || -> Result<Vec<u8>, String> {
        let mut response = agent().get(&url).call().map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        response.body_mut().as_reader().read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        Ok(buf)
    });
    match result {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) => Err(pyo3::exceptions::PyException::new_err(e)),
    }
}
