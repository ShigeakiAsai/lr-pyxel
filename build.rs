// build.rs — detects whatever Python version PyO3 actually links
// against (via pyo3-build-config, the same interpreter-discovery logic
// PyO3 itself uses), and exposes it as a compile-time env var so
// retro.rs's RTLD_GLOBAL re-dlopen() targets the correct
// libpythonX.Y.so instead of a hardcoded version.
//
// On Lakka (cross-compiled, PYO3_CROSS_PYTHON_VERSION=3.11 set in
// package.mk) this resolves to "3.11". On a native non-Lakka build
// (e.g. `cargo build` on Ubuntu 24.04, no cross-compile env vars set)
// it resolves to whatever `python3` on PATH actually is (e.g. "3.12"),
// with no code changes needed either place.

fn main() {
    let config = pyo3_build_config::get();
    println!(
        "cargo:rustc-env=LR_PYXEL_PYTHON_VERSION={}.{}",
        config.version.major, config.version.minor
    );
}
