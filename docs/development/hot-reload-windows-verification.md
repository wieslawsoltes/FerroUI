# Native Windows verification

The hot reload acceptance commands were run natively on Windows in an interactive
desktop session, including callback patches, view builders, restart hand-off and
the ten catalog pages registered at the time. All 21 commands completed with exit
code 0. The feature remains experimental, with the limitations described in
[hot-reload.md](hot-reload.md). Later revisions need their own runs; these results
are not acceptance of every later path.

## Environment

- Windows 11 Pro 25H2 (build 26200), x64, MSVC.
- Rust and Cargo 1.93.0, dx (dioxus-cli) 0.7.10, Subsecond 0.7.10 (locked),
  Python 3.12.
- Visual Studio 2026 Community 18.4 (MSVC 19.50, toolset 14.50), Windows SDK
  10.0.26100.0, LLVM and libclang 21.1.8 (`LIBCLANG_PATH`).
- `RUSTUP_TOOLCHAIN=1.93.0`, `PYTHONUTF8=1`, `RUST_BACKTRACE=1`, four build jobs,
  no inherited `CARGO_TARGET_DIR`; Git with `core.autocrlf=false` and
  `core.eol=lf`. The native probes create fresh private applications and
  artifacts. Windows long-path support stayed disabled.

Some independent builds ran concurrently, so elapsed times include contention.
Render-completion measurements do not include physical display scan-out.

## Acceptance results

| Gate | Command | Exit | Elapsed (s) |
| --- | --- | ---: | ---: |
| 1 | `cargo test --locked -p ferroui-build --lib` | 0 | 126.359 |
| 2 | `cargo test --locked -p ferroui-base --features hot-reload --lib` | 0 | 158.891 |
| 3 | `python scripts/hot-reload/profile_probe.py` | 0 | 10.234 |
| 4 | `cargo test --locked -p ferroui-hot-reload -p ferroui-hot-patch --lib` | 0 | 211.500 |
| 5 | `cargo test --locked -p ferroui-markup-xaml --features hot-reload --lib` | 0 | 206.297 |
| 6 | `cargo test --locked -p ferroui-markup-xaml-loader --lib` | 0 | 189.640 |
| 7 | `python -m unittest discover -s scripts/hot-reload -p "test_*.py"` | 0 | 6.735 |
| 8 | `python scripts/hot-reload/probe.py` | 0 | 177.688 |
| 9 | `python scripts/hot-reload/desktop_patch_probe.py` | 0 | 586.407 |
| 10 | `python scripts/hot-reload/supervisor_probe.py` | 0 | 1912.218 |
| 11 | `cargo test --locked -p control-catalog --lib tests::compiled_markup` | 0 | 361.671 |
| 12 | `cargo test --locked -p control-catalog --features hot-reload --lib expander_refresh_releases` | 0 | 320.656 |
| 13 | `cargo test --locked -p control-catalog --features hot-reload --lib tests::hot_reload` | 0 | 54.719 |
| 14 | `cargo build --locked -p control-catalog-desktop -p hot-reload-demo --features control-catalog-desktop/hot-reload,hot-reload-demo/patch-control` | 0 | 336.188 |
| 15 | `python scripts/hot-reload/restart_state_probe.py` | 0 | 60.437 |
| 16 | `python scripts/hot-reload/markup_probe.py` | 0 | 1.625 |
| 17 | `python scripts/hot-reload/catalog_probe.py` | 0 | 182.234 |
| 18 | `python scripts/hot-reload/resource_probe.py` | 0 | 20.000 |
| 19 | `cargo check --locked -p control-catalog -p ferroui-hot-reload -p ferroui-hot-patch --features control-catalog/hot-reload --release` | 0 | 121.890 |
| 20 | `python scripts/android/convert-thread-locals.py --check` | 0 | 7.297 |
| 21 | `git diff --check` | 0 | 0.062 |

## Test counts and skips

- Gate 1: 25 passed.
- Gate 2: 4,376 passed.
- Gate 4: 19 patch and 74 reload tests passed.
- Gate 5: 204 passed.
- Gate 6: 404 passed, 1 ignored (the existing metadata diagnostic
  `the_well_known_types_resolve_over_the_registered_metadata`).
- Gate 7: 27 tests, 2 skipped.
- Gate 11: 226 passed; gate 12: 1 passed; gate 13: 3 passed.

The Python skips are the POSIX interrupt-delivery test and the internal-link
snapshot test, which needs the privilege to create symbolic links. The
explicit-input test leaves out its Windows link-creation part. The Windows Job
Object cleanup, assignment-failure and launch-gate tests run. Two Rust patch
tests are compiled only on Unix (symlink rejection and executable permissions).

After the watcher shutdown fix below, the catalog reload tests (four at that
point), 85 reload tests, 19 patch tests, the Python harness and the native
root-binding probe also passed on Windows; the full matrix was not rerun for that
revision.

## Native observations

- The standalone patch probe kept the process, the framework and type identity,
  the existing subscription and the counter through two accepted patches and
  through the initial, subsequent and panicking transaction rollbacks. Incompatible
  edits of stable-library inputs required a restart before a patch was installed.
- The desktop probe passed coherent Rust and XAML revisions, rollback after
  invalid markup and after a failed library, handler-only updates that kept the
  child and its revision, rejection of invalid derived markup, replacement of a
  live view builder, and a derived view kept through a later handler-only patch.
- The supervisor kept the running application after a failed replacement build
  and removed failed, retired and shut-down generations in order. The replacement
  received counter 7, the first command after start-up was a click that returned
  8, and the rest of the state envelope equalled the captured one.
- The placement probe restored a non-default placement, navigation and counter,
  and rejected invalid or frozen placement changes.
- The small fixture and the ten catalog pages (Border, Slider, ProgressBar,
  CheckBox, Canvas, RadioButton, TextBlock, Viewbox, Expander and ListBox) passed
  invalid-edit recovery, atomic saves and root-property removal where covered.
- The resource probe changed a child's static width from 611 to 612 and 613
  through resource revisions, refreshed the preserved root's static Theme, kept
  the last good child and theme when a key disappeared, and recovered through an
  atomic save, with unchanged window, root and view-model identities.
- Compiled and runtime parity ran without `runtime-markup`. The release and
  profile checks passed without enabling reload in release builds.

## Fixes found on Windows

- An integration run completed its assertions but deadlocked while worker threads
  shut down: the catalog's thread-local destructor joined its file watcher while
  the watcher waited in Windows thread shutdown (the loader lock; see the
  [Rust 1.93 thread-local notes](https://github.com/rust-lang/rust/blob/1.93.0/library/std/src/thread/local.rs#L101-L108)).
  The catalog session now belongs to the application's service scope and the
  thread-local holds only a weak reference; disposing of the scope stops its
  dispatcher timer and joins the watcher before thread-local teardown, and desktop
  exit shuts it down explicitly. The integration tests then finished normally,
  and a native ListBox exit check confirmed a live watcher and a normal close.
- The build-task test compares native paths, a compiler failure reports its
  complete output, the supervisor's private workspace sets LF line endings
  explicitly, and the supervisor uses a shorter temporary-directory prefix: a
  264-character Skia cache path failed in curl, 253 characters succeeded.
- LLVM 21.1.8 supplied libclang, which MSVC requires to be Clang 19 or newer.
- `FERROUI_TRACE_CLICKS` makes the sample record each counter change and whether
  its source was a native button activation or the probe's protocol. In the final
  supervisor run, the counters were exactly 1, 2, 3, 5, 7 and 8, all from the
  protocol.

## Unresolved observations

- One early supervisor run returned counter 14 instead of 8 without any input to
  the window. That run did not keep its captured state, and the cause is not
  known. Later complete runs passed with full state equality, and twelve start-ups
  of the retained binary passed with an immediate first click.
- One Rust compiler access violation during an early run is unexplained.
