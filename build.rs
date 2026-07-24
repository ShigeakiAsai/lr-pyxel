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
// pyo3_build_config::get() is the crate's own public, documented
// entry point for this ("Loads the configuration determined from the
// build environment... requires a direct dependency on at least one
// of pyo3 or pyo3-ffi" — matches this build.rs's own situation
// exactly). Previously called InterpreterConfig::from_cargo_dep_env()
// directly, which get() itself wraps internally — but that function
// went from merely-undocumented (0.21, still worked here) to
// pub(crate)-private (0.29, fails to even compile) between versions,
// confirmed by checking pyo3-build-config's own source rather than
// guessing. get() is the stable path that isn't going anywhere,
// since it's what pyo3-build-config itself documents as the intended
// way for a dependent build script to do exactly this.

fn main() {
    let config = pyo3_build_config::get();
    let version = config.version();
    println!(
        "cargo:rustc-env=LR_PYXEL_PYTHON_VERSION={}.{}",
        version.major, version.minor
    );
}
