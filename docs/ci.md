# Continuous integration: which checks a pull request runs

A pull request runs only the checks that build, test or run a file it changes. The macOS
runners are the scarce resource (their jobs wait in a queue for hours when every pull request
asks for them), so the workflows that need one run only for the areas they test. A push to
main, the nightly runs and the runs started by hand (`gh workflow run <file> --ref <branch>`)
run everything, as before: no check of main was removed.

## 1. The CI workflow

`.github/workflows/ci.yml` has a first job, `Changed areas`, which decides which of the other
jobs a pull request needs (`scripts/ci/changed_areas.py`), and a last one, `CI result`, which
sums the run up in one check:

| Job | Runner | Runs on a pull request |
| --- | --- | --- |
| Changed areas | Linux | always |
| Source conventions and packages | Linux | always (cheap, and every change is checked by it) |
| API gate (cargo-semver-checks) | macOS | a file under `src/` changes (the rule of the gate) |
| Build and test (Linux, the workspace) | Linux | a package of the workspace that it builds changes |
| Build and test (macOS, the native platform) | macOS | a package of the native platform or one they build changes |
| Build, test and smoke run (Windows platform backend) | Windows | a package that the Windows job builds changes |
| Check (browser, wasm32-unknown-emscripten) | Linux | a package of the browser catalog changes |
| Build and test (Linux, X11 under Xvfb) | Linux | a package of the X11 platform or one it builds changes |
| CI result | Linux | always: fails when a job failed or was cancelled |

The rule is the same for every job. The script describes each job by the packages it builds
(the `-p` of its steps, with their features, and whether it builds their tests), and the target
it builds them for (Linux, macOS, Windows, `wasm32-unknown-emscripten`). From the manifests of
the workspace (`cargo metadata --no-deps`) it works out every package the job compiles: the
dependencies of the roots on paths, with the optional ones the features turn on and the
platform-specific ones of the target (`[target.'cfg(windows)'.dependencies]` is not compiled
by the macOS job). A changed file belongs to the package whose directory holds it, and also to
every package whose sources name it with a path (`#[path = "../../scripts/perf/x.rs"]`,
`include_str!("../../docs/porting/data/x.tsv")`, a test fixture under `tests/media`); a name
in a test file of a package only reaches the jobs that build the tests of that package. A job
runs when one of the packages of the changed files is in its set, or when the pull request
changes a script that the job runs itself:

| Changed path | Jobs of ci.yml beyond the conventions |
| --- | --- |
| `docs/**`, `*.md` (outside a package), another workflow | none |
| `.github/workflows/ci.yml`, `.github/actions/**`, `Cargo.toml`, `Cargo.lock`, `.cargo/**`, `scripts/ci/**`, `external/**` | every job |
| a file in no package and in no area below (for example a new top-level file) | every job |
| `src/FerroUI.Base`, `src/FerroUI.Controls`, `src/Markup`, `src/Skia`, `src/FerroUI.Metal`, the Fluent and WinUI themes, TreeDataGrid, LiveCharts, `samples/ControlCatalog` | every job, the API gate for those under `src/` (the catalog and the desktop entry point build them everywhere) |
| `src/Vello/**` | API gate, Linux, Windows (ferroui-win32 tests with Vello); plus `vello.yml` and `browser.yml` |
| `src/FerroUI.Native/**`, `native/FerroUI.Native/**` | API gate, Linux, macOS |
| `src/Windows/FerroUI.Win32/**` | API gate, Linux, Windows |
| `src/FerroUI.X11/**`, `src/FerroUI.Wayland/**` | API gate, Linux, X11 (plus `wayland.yml`) |
| `src/Browser/FerroUI.Browser/**` | API gate, Linux, browser (plus `browser.yml`) |
| `src/iOS/**`, `src/Android/**` | API gate, Linux (plus `ios.yml`, `android.yml`) |
| `src/FerroUI.Themes.MacOS/**`, `src/FerroUI.Themes.Material/**` | API gate, Linux (plus their theme workflows) |
| `src/FerroUI.Designer*/**`, `src/Pretext/**`, `src/ProTranslate/**`, `src/Dock/**`, `src/FerroUI.Controls.Charts/**`, `src/FerroUI.Rendering3D/**` | API gate, Linux (plus their own workflows) |
| `src/FerroUI.View/**` | API gate, Linux, Windows (the HelloWorld samples) |
| `samples/WinUIGallery/**` | Linux (plus `winui-gallery.yml`) |
| `components/**` | Linux (the FerroBoard component) |
| `scripts/x11/**` | X11 |
| `scripts/browser/**`, `scripts/build-browser.sh` | browser (plus `browser.yml`) |
| `scripts/windows/**`, `scripts/winui/**`, `scripts/winui-theme/**`, `scripts/text-quality.py` | Windows |
| `scripts/api-gate/**` | API gate |
| any other script (`scripts/perf`, `scripts/release`, ...) | none (the conventions run the ones they use) |

The table is what the script answers for those paths today; the script itself is the
reference, and it follows the manifests: a new dependency changes the jobs of a package without
an edit here. To ask it about files of a branch:

```sh
git diff --name-only origin/main... | python3 scripts/ci/changed_areas.py --files -
echo src/Vello/FerroUI.Vello/lib.rs | python3 scripts/ci/changed_areas.py --files -
```

It prints `job=true|false` for each job, and the reason for each (the first file and the
package that made the job run) on the standard error; in CI the reasons are the summary of the
`Changed areas` job. Every job runs when the event is not a pull request, when the list of
files is not complete (the API of GitHub lists at most 3000 files), when the manifests cannot be
read, or when the script fails. Its tests (`scripts/ci/test_changed_areas.py`) run in the
conventions job.

A job that a pull request does not need is skipped and shows as a skipped check. No check is
required by a branch protection rule of main today; if one is set, `CI result` is the check to
require: it passes when the jobs a pull request does not need were skipped, and fails when one
failed or was cancelled.

The Windows job keeps a rule of its own inside it: its long steps that test Windows alone (the
teardown loops, the HelloWorld samples, the IntegrationTestApp, GpuInterop, the WinUI theme
gallery) run on a pull request only when the Windows sources change.

A pull request that changes the documentation alone (`docs/**`, Markdown) does not start the CI
workflow, except for the data under `docs/` that tests read (`docs/porting/data/**`,
`docs/designer/samples/**`).

**A part of a stack.** A pull request with an open pull request based on its branch runs the changed-areas and conventions checks and leaves the build and test jobs to the top part, whose head contains its code. The jobs of a part based on another branch follow every file the stack changes up to that part (the comparison with `main`), not only its own diff; more than the 300 files the comparison lists runs every job. The check `Reduced CI (the pull request above runs the full checks)` marks such a run, and `scripts/stack/stack.py` does not land a top part whose latest run has it (`gh workflow run ci.yml --ref <branch>` runs the full checks on that head). A pull request based on the branch that is not a part of the stack carries the label `stack-independent`. See docs/stacked-pull-requests.md.

## 2. The workflows of one area

The other workflows start on a pull request only when it changes their own paths
(`on.pull_request.paths`), so a pull request shows no check for them otherwise. Each one also
runs every night on main or for a push to main, which is where a change of the framework
crates they are built on meets them:

| Workflow | Runners | A pull request starts it for | Also runs |
| --- | --- | --- | --- |
| `vello.yml` | macOS | `src/Vello`, `src/FerroUI.Rendering3D`, the Fluent and Simple themes, the classic shell of the catalog, the render backends comparison | nightly |
| `browser.yml` | Linux | `src/Browser`, `src/Vello`, `src/FerroUI.DevTools`, `src/FerroUI.Controls.Repeater`, `samples/ControlCatalog.Browser`, `scripts/browser`, the browser toolchain action | nightly |
| `ios.yml` | macOS | `src/iOS`, `samples/*.iOS`, `scripts/ios` | nightly |
| `android.yml` | Linux | `src/Android`, `samples/*.Android`, `scripts/android` | nightly |
| `wayland.yml` | Linux | `src/FerroUI.Wayland`, `scripts/wayland` | nightly |
| `linuxfb.yml` | Linux | `src/Linux`, `src/FerroUI.Mcp`, `scripts/linuxfb`, `scripts/test-mcp.py` | nightly |
| `designer.yml` | Linux, macOS, Windows | the designer crates, `samples/DesignerStudio`, `docs/designer/samples` | nightly |
| `pretext.yml` | Linux | `src/Pretext`, `scripts/pretext`, `docs/porting/pretext.md` | push to main (also for the text services of ferroui-base, the facade crate and the headless platform) |
| `protranslate.yml` | Linux, macOS, Windows | `src/ProTranslate`, `samples/ProTranslateDemo` | push to main (also for the binding sources of ferroui-base it uses) |
| `macos-theme.yml` | Linux, macOS 26 | `src/FerroUI.Themes.MacOS`, `scripts/macos-theme`, `scripts/themes` | nightly |
| `material-theme.yml` | Linux | `src/FerroUI.Themes.Material`, the animation and gesture sources it uses, `samples/MaterialThemeGallery`, its scripts | nightly |
| `build-performance.yml` | Linux | the build tooling (scanner, build tasks, XAML loader, `samples/SampleBuild`, `scripts/build-perf`, the browser build scripts, `.cargo/*.toml`) | by hand |
| `charts.yml` | Linux | `src/FerroUI.Controls.Charts`, the facade crate, `samples/ChartsDemo`, its data and scripts | by hand |
| `dock.yml` | Linux | `src/Dock`, `tests/Dock`, `samples/Dock`, `scripts/dock`, `docs/porting/dock` | push to main |
| `inking.yml` | Linux, macOS, Windows (x64 and ARM64) | `src/FerroUI.Controls.Inking`, its builders, pages, scripts and data | by hand |
| `livecharts.yml` | Linux, Windows | the LiveCharts crates, `samples/LiveChartsDemo`, its scripts and data | nightly |
| `rich-text.yml` | Linux, Windows | the oracle, `src/FerroUI.MicroCom`, its scripts and case tables | by hand |
| `winui-gallery.yml` | Linux | `samples/WinUIGallery`, the WinUI controls, theme controls and markup it shows | nightly |
| `winui-gallery-scan.yml` | Linux | `scripts/winui-gallery`, `scripts/port-status/rustscan.py`, `docs/porting/winui-gallery` | nightly |
| `build-output-label.yml` | Linux | the label `build-output-changes` (its job is skipped for any other label) | |

`designer.yml`, `pretext.yml`, `protranslate.yml` and `macos-theme.yml` started on pull
requests that changed the framework crates under them until November 2026 (ferroui-base, the
controls, the markup, Vello, the native platform). Those crates are tested by the CI workflow
on every pull request that changes them, the Linux job builds and tests the crates of these
workflows with them, and the nightly run (or the run of the push to main) runs the workflows
themselves.

A change of the root manifest or the lock file does not start `designer.yml`, `pretext.yml`,
`protranslate.yml` or `capi.yml` on a pull request either: nearly every new crate changes those
two files. The CI workflow tests the change on the pull request, and these workflows meet it in
the nightly run or the run of the push to main.

`nightly.yml` (Windows on ARM64, Wayland under sway), `perf.yml`, `pages.yml` (a push to main),
`release.yml`, `hot-reload.yml`, `skia-vulkan-windows.yml` and the reusable `app.yml` do not
run on pull requests.

## 3. Cancelled runs

Every workflow that runs on pull requests has a concurrency group per workflow and ref with
`cancel-in-progress: true`: a new push to a pull request cancels the run of the commit before
it. The nightly run of ci.yml has a group of its own, so that it and a run of a push to main do
not cancel each other; `pages.yml` and `release.yml` never cancel a running deployment.

## 4. What a few pull requests run

| Pull request | Runs |
| --- | --- |
| documentation alone (`docs/ci.md`) | nothing |
| Vello alone (`src/Vello/FerroUI.Vello/lib.rs`) | ci.yml: Changed areas, conventions, API gate (macOS), Linux, Windows, CI result; `vello.yml` (macOS), `browser.yml` |
| the macOS native platform (`src/FerroUI.Native`) | ci.yml: Changed areas, conventions, API gate, Linux, macOS, CI result |
| a script of the benchmarks (`scripts/perf/gate.py`) | ci.yml: Changed areas, conventions, CI result |
| the WinUI Gallery sample (`samples/WinUIGallery`) | ci.yml: Changed areas, conventions, Linux, CI result; `winui-gallery.yml` |
| ferroui-base, the lock file or a workflow of ci.yml | every job of ci.yml |
