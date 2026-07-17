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
//
// PyO3 0.29 change: pyo3-build-config no longer resolves/inlines the
// interpreter config itself. Instead, pyo3-ffi's own build script
// resolves it and serializes it into the DEP_PYTHON_PYO3_CONFIG env
// var (visible here because this crate already depends on `pyo3`,
// which pulls in pyo3-ffi). InterpreterConfig::from_cargo_dep_env()
// reads that env var back out. This function is #[doc(hidden)] in
// pyo3-build-config (not officially stable API), but it's the same
// mechanism PyO3 itself relies on internally, matching the intent
// described in the 0.29 migration guide.

fn main() {
    let config = pyo3_build_config::InterpreterConfig::from_cargo_dep_env()
        .expect("DEP_PYTHON_PYO3_CONFIG is not set — is `pyo3` (or `pyo3-ffi`) a dependency?")
        .expect("failed to parse pyo3's interpreter config");
    println!(
        "cargo:rustc-env=LR_PYXEL_PYTHON_VERSION={}.{}",
        config.version.major, config.version.minor
    );
}
