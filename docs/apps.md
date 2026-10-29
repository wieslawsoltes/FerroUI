# Applications in repositories of their own

A FerroUI application that is larger than a sample of this repository lives in a repository of
its own (Solitaire, Ember and FerroDesign in the [FerroUI organization](https://github.com/FerroUI)).
Such a repository has FerroUI as a git submodule and reuses its build setup and its CI from this
repository, so that a new application is cheap to set up and builds exactly as the framework does:

| What | Where it comes from |
|---|---|
| The framework crates | path dependencies into the submodule (`FerroUI/src/...`) |
| Workspace dependencies and their pinned versions | `[workspace.dependencies]` of FerroUI's `Cargo.toml`, copied by `scripts/apps/sync-workspace.py` |
| Profiles (release, fast-release, dist, browser with its per-package levels) | the `[profile.*]` sections of FerroUI's `Cargo.toml`, copied by the same script |
| The cargo configuration of the browser target | FerroUI's `.cargo/config.toml`, included by the application's `.cargo/config.toml` |
| Compiler cache and build-tool optimisation | FerroUI's opt-in `.cargo/compiler-cache.toml` and `.cargo/build-tools.toml` |
| The browser site | `scripts/build-browser.sh` with `FERROUI_BROWSER_WORKSPACE` |
| CI and GitHub Pages | the reusable workflow `.github/workflows/app.yml` |

## 1. Layout

```
MyApp/
  FerroUI/                     the submodule (https://github.com/wieslawsoltes/FerroUI.git)
  .cargo/config.toml           includes FerroUI/.cargo/config.toml
  .github/workflows/ci.yml     calls FerroUI's app.yml
  Cargo.toml                   the workspace, with the FerroUI blocks
  Cargo.lock                   committed (CI builds with --locked)
  src/... or MyApp/...         the crates of the application
  MyApp.Browser/               the browser host: a binary package with wwwroot/
  README.md, LICENSE, NOTICE.md
```

The submodule must be at `FerroUI/`: the browser job of the reusable workflow uses the browser
toolchain action of the submodule at that path, and the conventions job leaves that directory out.

```
git submodule add https://github.com/wieslawsoltes/FerroUI.git FerroUI
```

## 2. The workspace manifest

The application's root `Cargo.toml` is a workspace of its own. Cargo takes neither profiles nor
workspace dependencies from another workspace, so FerroUI's are copied into it, between markers,
by `scripts/apps/sync-workspace.py`. The script writes the blocks when they are missing; the CI of
the application fails when they differ from the pinned submodule
(`sync-workspace.py --check`). Nothing in the blocks is edited by hand.

```toml
[workspace]
resolver = "2"
members = ["MyApp", "MyApp.Desktop", "MyApp.Browser"]
exclude = ["FerroUI"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.89"
license = "MIT"
repository = "https://github.com/FerroUI/MyApp"

[workspace.dependencies]
# >>> FerroUI workspace dependencies (written by FerroUI/scripts/apps/sync-workspace.py ...)
# <<< FerroUI workspace dependencies
# The dependencies of the application itself follow the block.
serde = { version = "1", features = ["derive"] }

# >>> FerroUI profiles (written by FerroUI/scripts/apps/sync-workspace.py ...)
# <<< FerroUI profiles
```

`exclude = ["FerroUI"]` is required: cargo makes every path dependency below the workspace root a
member of the workspace, and the framework crates belong to FerroUI's workspace.

```
python3 FerroUI/scripts/apps/sync-workspace.py           # write the blocks
python3 FerroUI/scripts/apps/sync-workspace.py --check   # what CI runs
```

- **Dependencies.** The block is FerroUI's `[workspace.dependencies]` with every `path` made
  relative to the application (`FerroUI/src/FerroUI.Base`). The crates of the application state
  `ferroui-base = { workspace = true }`, exactly as the samples of this repository do, and take
  third-party crates that FerroUI pins (for example `wasm-bindgen`, whose command-line tool the
  browser toolchain installs at that exact version) from the same block. Samples-only helper crates
  of this repository (`FerroUI/samples/SampleSupport`, `SampleBuild`, `SampleTesting`) are path
  dependencies.
- **Profiles.** `release` (thin LTO), `fast-release` (for running locally), `dist` (what is shipped
  and measured) and `browser` (size-optimised, with the per-package optimisation levels of the
  framework's hot crates; docs/porting/browser-platform.md, section 18). Cargo warns about a
  per-package override whose package the application does not use (for example the Vello crates);
  the warning is harmless.
- **Lock file.** Start `Cargo.lock` from FerroUI's, so that the application resolves the same
  versions the framework is tested with, then let cargo prune and complete it:

  ```
  cp FerroUI/Cargo.lock Cargo.lock
  cargo metadata --format-version 1 > /dev/null
  ```

## 3. Cargo configuration

`.cargo/config.toml` of the application includes FerroUI's (the link settings of the
`wasm32-unknown-emscripten` target):

```toml
include = ["../FerroUI/.cargo/config.toml"]
```

`include` needs a recent cargo: the version the browser toolchain is pinned to
(`scripts/browser/setup.sh`) reads it, older versions ignore the key (which matters only for the
browser target). `scripts/build-browser.sh` sees the include and does not pass the file
a second time; without it, the script passes the file with `--config`.

The opt-in files of FerroUI work for the application as they do here
(docs/build-performance-profile-tuning.md):

```
cargo --config FerroUI/.cargo/compiler-cache.toml build --locked   # sccache as rustc wrapper
cargo --config FerroUI/.cargo/build-tools.toml build --locked      # optimised source scanner
```

## 4. Conventions

The rules of the port hold in the application as well, and its CI checks them:

- The name of the upstream framework appears only in `docs/`, `scripts/` and `NOTICE.md` files.
- Thread-local variables are declared with `ferroui_base::ferro_thread_local!`, never with
  `thread_local!` (docs/porting/android-platform.md, section 5.1).
- Every cargo command of CI runs with `--locked`.
- `NOTICE.md` keeps the license and copyright of the work the application is derived from; the
  application itself is MIT (`LICENSE`).

## 5. The browser host

The browser host is a binary package with a `wwwroot/` directory next to its manifest, like
`samples/ControlCatalog.Browser` and the hosts in the application repositories: a `#[wasm_bindgen]`
entry point that starts the application with `start_browser_app`, and a host page whose script
loads the module through `ferroui-loader.js` and calls the entry point. The site is built with the
script of the submodule:

```
source ~/browser-tools/env.sh     # the pinned toolchain (FerroUI/scripts/browser/setup.sh)
FERROUI_BROWSER_WORKSPACE=$PWD FerroUI/scripts/build-browser.sh my-app-browser
```

The site is written to `target/browser/my-app-browser`; serve that directory with any static web
server.

## 6. CI and GitHub Pages

The application's only workflow calls the reusable workflow of this repository:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}

jobs:
  app:
    uses: wieslawsoltes/FerroUI/.github/workflows/app.yml@main
    with:
      test: |
        -p my-app
      check: |
        -p my-app-desktop
      browser-package: my-app-browser
```

All jobs run on `ubuntu-latest`:

| Job | What it does |
|---|---|
| Conventions | the upstream name, `ferro_thread_local!`, the FerroUI blocks of `Cargo.toml` against the submodule |
| Build and test | the stable toolchain, the Rust build cache (saved by runs of main only), mold as linker, the system packages for fonts and an X server; each line of `check` is a `cargo check --locked`, each line of `test` a `cargo test --locked` under `xvfb-run`, then `script` |
| Browser site | the pinned browser toolchain (FerroUI's `browser-toolchain` action), `build-browser.sh` with `FERROUI_BROWSER_WORKSPACE`, then `browser-script` (with `SITE`), the module sizes and the Pages artifact |
| Deploy to GitHub Pages | runs of main only, after the other jobs passed |

Inputs: `test`, `check`, `apt-packages` (extra Ubuntu packages), `script`, `browser-package`,
`browser-features`, `browser-script`, `pages` (default `true`). The actions are pinned by commit, as
in the workflows of this repository.

GitHub Pages is enabled once, with GitHub Actions as its source:

```
gh api -X POST repos/FerroUI/MyApp/pages -f build_type=workflow
```

The site is then published at `https://ferroui.github.io/MyApp/` by every push to main.

## 7. Updating FerroUI

```
git -C FerroUI fetch origin && git -C FerroUI checkout origin/main
python3 FerroUI/scripts/apps/sync-workspace.py
cargo metadata --format-version 1 > /dev/null     # updates Cargo.lock for the new framework
git add FerroUI Cargo.toml Cargo.lock && git commit
```

An application may pin the submodule to a branch of a pull request of FerroUI while it needs a
change that is not merged yet, and moves back to main when the pull request is merged.
