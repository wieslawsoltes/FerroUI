# Dependencies

The selected SDK is the parent FerroUI repository. The six shipping manifests use
relative paths only; no workstation checkout or private SDK is required. Core
depends on the real FerroUI.System notifications, serde/serde_json, indexmap, flate2,
parking_lot, uuid, sys-locale, time and Jiff. Rendering, native input, scheduling,
clipboard and reusable markup use the actual FerroUI crates.

Native locale services use system ICU on Linux/macOS, NLS on Windows, and the
existing Android/Emscripten integrations. Linux requires `libicu-dev` and pkg-config.
The browser ICU data/version remains explicitly pinned by its launcher. Optional
platform launchers retain their SDK/toolchain requirements; desktop integration
does not certify those runtimes.

Rust registry dependencies are locked. Reference license records are retained
under reference/ with source/resource provenance; these records retain their
original inventory scope. New SDK versions and distribution packages require their
own dependency/license audit. No default test needs .NET or public font downloads.
