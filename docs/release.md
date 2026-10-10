# Releasing FerroUI

How the crates of the workspace are published to crates.io: which crates, in which order, at which version, what each package holds, how the release workflow runs and what has to be true before it is run. Nothing has been published yet; section 12 lists what the owner decides before the first release.

Publishing cannot be taken back. A version of a crate can be yanked (Cargo then no longer selects it for new builds) but never removed or replaced, and the name of a crate belongs to the account that published it first. Everything below is arranged so that a release is rehearsed completely before anything is uploaded.

## 1. What is published

**The rule.** A member of the workspace is published unless its manifest says `publish = false`. No list of crates is kept anywhere: the scripts, the release workflow, the check of the continuous integration and the crate table of the README derive the set and its order from `cargo metadata` (`scripts/release/crates.py`).

```sh
scripts/release/crates.py order         # the published crates, in the order they are published
scripts/release/crates.py unpublished   # the crates that are never published
scripts/release/crates.py check         # what a published crate has to state
```

### 1.1 The published crates

The state on 2026-10-10, in the order of publication. The size is the size of the package (`.crate`, compressed); crates.io accepts 10 MiB. The dry run prints the current table.

| # | Crate | Directory | Group | Package |
|---|---|---|---|---|
| 1 | `ferroui-build-scan` | `src/FerroUI.Build.Scan` | Markup and build | 138 KiB |
| 2 | `ferroui-base` | `src/FerroUI.Base` | Core | 2389 KiB |
| 3 | `ferroui-metal` | `src/FerroUI.Metal` | Render backends and text | 12 KiB |
| 4 | `ferroui-microcom` | `src/FerroUI.MicroCom` | Platforms | 15 KiB |
| 5 | `ferroui-remote-protocol` | `src/FerroUI.Remote.Protocol` | Tools and designer support | 77 KiB |
| 6 | `ferroui-controls` | `src/FerroUI.Controls` | Controls and themes | 1611 KiB |
| 7 | `ferroui-fonts-inter` | `src/FerroUI.Fonts.Inter` | Controls and themes | 932 KiB |
| 8 | `ferroui-harfbuzz` | `src/HarfBuzz/FerroUI.HarfBuzz` | Render backends and text | 17 KiB |
| 9 | `ferroui-markup` | `src/Markup/FerroUI.Markup` | Markup and build | 56 KiB |
| 10 | `ferroui-markup-xaml` | `src/Markup/FerroUI.Markup.Xaml` | Markup and build | 133 KiB |
| 11 | `ferroui-opengl` | `src/FerroUI.OpenGL` | Render backends and text | 78 KiB |
| 12 | `ferroui-skia` | `src/Skia/FerroUI.Skia` | Render backends and text | 234 KiB |
| 13 | `microcom-codegen` | `src/tools/MicroCom.CodeGenerator` | Tools and designer support | 42 KiB |
| 14 | `xamlx` | `external/XamlX/src/XamlX` | Markup and build | 127 KiB |
| 15 | `ferroui-markup-xaml-loader` | `src/Markup/FerroUI.Markup.Xaml.Loader` | Markup and build | 465 KiB |
| 16 | `ferroui-build` | `src/FerroUI.Build.Tasks` | Markup and build | 81 KiB |
| 17 | `ferroui-designer-support` | `src/FerroUI.DesignerSupport` | Tools and designer support | 74 KiB |
| 18 | `ferroui-dialogs` | `src/FerroUI.Dialogs` | Controls and themes | 136 KiB |
| 19 | `ferroui-native` | `src/FerroUI.Native` | Platforms | 207 KiB |
| 20 | `ferroui-desktop` | `src/FerroUI.Desktop` | Platforms | 20 KiB |
| 21 | `ferroui-themes-fluent` | `src/FerroUI.Themes.Fluent` | Controls and themes | 550 KiB |
| 22 | `ferroui-browser` | `src/Browser/FerroUI.Browser` | Platforms | 231 KiB |
| 23 | `ferroui-themes-simple` | `src/FerroUI.Themes.Simple` | Controls and themes | 349 KiB |
| 24 | `ferroui-controls-color-picker` | `src/FerroUI.Controls.ColorPicker` | Controls and themes | 114 KiB |
| 25 | `ferroui-headless` | `src/Headless/FerroUI.Headless` | Platforms | 66 KiB |
| 26 | `ferroui-vello` | `src/Vello/FerroUI.Vello` | Render backends and text | 319 KiB |

The build-time crates are part of the set because published crates name them: `ferroui-build-scan` is a build dependency of the base library, the controls, the XAML run-time library and the OpenGL contracts (it exports the type model of a crate); `ferroui-build` is a build dependency of every crate with compiled markup (the dialogs, the themes, the colour picker) and of an application that compiles its markup; `microcom-codegen` is a build dependency of the macOS platform. `xamlx`, the port of an external library under `external/XamlX`, is a dependency of the loader, the build crate and the designer support. The workspace has no procedural macro crate.

Every one of the 26 was checked in the same way: packaged, and built from its package, by `scripts/release/publish.sh --dry-run` (section 8). `cargo publish --dry-run -p <crate>` cannot be used for this before the first release: it resolves the dependencies of the package in the registry, where the other crates of the workspace are not yet. `cargo package --workspace` (Cargo 1.90 and later) resolves them among the packages it has just assembled instead, which is what the dry run uses, and so every crate got the full check, not only the ones without dependencies in the workspace.

### 1.2 The crates that are never published

Each states `publish = false`.

| Crate | Directory | What it is |
|---|---|---|
| `control-catalog` | `samples/ControlCatalog` | Sample application |
| `control-catalog-desktop` | `samples/ControlCatalog.Desktop` | Desktop entry point of the sample |
| `control-catalog-browser` | `samples/ControlCatalog.Browser` | Browser entry point of the sample |
| `mini-mvvm` | `samples/MiniMvvm` | View-model library of the samples |
| `ferroui-markup-xaml-tests` | `tests/FerroUI.Markup.Xaml.UnitTests` | Test crate |
| `xaml-include-fixture-theme` | `tests/XamlIncludeFixture/Theme` | Test fixture |
| `xaml-include-fixture-application` | `tests/XamlIncludeFixture/Application` | Test fixture |
| `ferroui-render-backends-comparison` | `tests/FerroUI.RenderBackends.Comparison` | Comparison harness of the render backends |
| `ferroui-render-tests` | `tests/FerroUI.RenderTests` | Upstream's render tests against its expected images |

### 1.3 Dependency order

A crate is published after every crate of the workspace its package depends on: normal dependencies, build dependencies, and development dependencies that have a version. The last kind counts because Cargo resolves the development dependencies of a package when it assembles it, so they have to be in the registry too.

That makes a cycle through development dependencies a problem that normal builds do not have. There was one: the Skia backend tests with the Inter font collection, and the font collection tests with the Skia backend. Cargo leaves a development dependency out of the uploaded manifest when it has a path and no version, so `ferroui-fonts-inter` names `ferroui-skia` by its path alone (not `workspace = true`, which would bring the version). `crates.py order` fails with the crates involved and this remedy when a new cycle appears.

### 1.4 How a new crate joins

1. Add it to `members` of the root manifest, with `version.workspace = true` and the other inherited keys of an existing crate (`edition`, `rust-version`, `license`, `repository`, `homepage`, `readme`, `keywords`, `categories`), its own one-line `description`, and `[package.metadata.release] group = "<core|controls|markup|rendering|platforms|tools>"`.
2. A crate that is not for crates.io (a sample, a test crate, a fixture, a benchmark, a tool of the repository) states `publish = false`, and nothing else is needed.
3. A crate that is published gets its entry in `[workspace.dependencies]` with a path and the pinned version, as the others, and is depended on with `workspace = true`. In its directory, `ln -s <relative path to the root>/LICENSE LICENSE` and `ln -s <relative path to the root>/NOTICE.md NOTICE-PROJECT.md` (section 5).
4. Run `scripts/release/readme-crates-table.sh` and `scripts/release/publish.sh --dry-run --no-verify`. The second is what the continuous integration runs; it says what is missing.

A published crate must not depend on an unpublished one, except through a development dependency that has a path and no version.

## 2. Names

Every name was looked up on 2026-10-10 in the sparse index of crates.io (`https://index.crates.io/...`, which answers 404 for a name no crate has) and with `cargo search`. crates.io treats `-` and `_` as the same character in a name.

| Name | State |
|---|---|
| The 24 `ferroui-*` crates of section 1.1 | free |
| `xamlx` | free |
| `microcom-codegen` | free |
| `ferroui` (the bare name) | free |
| `ferro-ui`, `ferro_ui` | free |
| `ferroui-windows`, `ferroui-win32`, `ferroui-linux`, `ferroui-x11` (candidates for the platform crates in progress) | free |

No name is taken, so nothing blocks on names. Nothing was reserved: a name is claimed by the first publish and by nothing else, and until then anyone can take it.

Two names are not under the `ferroui-` prefix, and the owner may prefer otherwise (section 12): `xamlx` is the name of the external library the crate is a port of, and `microcom-codegen` is named after the interop library whose design it follows. Publishing them claims those names on crates.io for this project. The alternatives are `ferroui-xamlx` and `ferroui-microcom-codegen`; a rename touches the manifests, the `use` paths and the profile overrides of the root manifest.

### 2.1 The bare name `ferroui`

`ferroui` is free and is the name people will try first. A facade crate is proposed, and not built yet, because what it exports is a decision about the public face of the project:

- A crate `ferroui` at `src/FerroUI` with no code of its own: it re-exports the common crates under short names (`ferroui::base`, `ferroui::controls`, `ferroui::markup`, ...) and a prelude of the types of a minimal application.
- Features choose the parts, so that an application names one dependency:

  | Feature | Brings | Default |
  |---|---|---|
  | `desktop` | `ferroui-desktop` (platform detection, Skia, HarfBuzz) | yes |
  | `fluent` | `ferroui-themes-fluent` | yes |
  | `simple` | `ferroui-themes-simple` | no |
  | `xaml` | `ferroui-markup-xaml` and the run-time loader | no |
  | `color-picker`, `dialogs` | the libraries of those names | no |
  | `browser` | `ferroui-browser` | no |
  | `vello`, `vello-hybrid`, `vello-gpu` | `ferroui-vello` with its modes | no |
  | `headless` | `ferroui-headless` | no |

- It is pinned to the other crates like every crate of the workspace, so `ferroui = "=0.1.0-preview.1"` selects one consistent set, which is the main use of it during the previews.

Until it exists, the name stays unclaimed. If the facade is not wanted for the first preview, the owner can still decide to publish it with the first preview as a thin crate that only re-exports, to hold the name.

## 3. Version scheme

**Semantic versioning, one version for the whole workspace, previews as pre-releases.** The previews are `0.1.0-preview.1`, `0.1.0-preview.2`, ... and then `0.1.0`. In the order of semantic versioning `0.1.0-preview.1 < 0.1.0-preview.2 < 0.1.0`; the numeric identifier after `preview.` is compared as a number, so `preview.10` follows `preview.9`.

Why pre-release identifiers and not `0.1.0`, `0.1.1`, ...: Cargo treats `0.1.x` versions as compatible with one another, and a preview is not compatible with the one before it. A pre-release says so, and Cargo never selects a pre-release unless the requirement names one: `ferroui-base = "0.1"` does not match `0.1.0-preview.1`. Nobody gets a preview by accident, and when `0.1.0` is published, requirements written for it never fall back to a preview.

**The crates of the workspace are pinned to one another exactly.** A requirement that names a pre-release without `=` (`"0.1.0-preview.1"`) is a caret requirement: it matches that preview, every later preview of `0.1.0`, and `0.1.x` releases. Cargo would then be free to build `ferroui-controls 0.1.0-preview.1` against `ferroui-base 0.1.0-preview.3`, which was never built or tested together and, between previews, will not compile. So every dependency between crates of the workspace is `version = "=<the version of the workspace>"`: the crates of a release resolve only as the set they were released as. An application does the same and names one version for every FerroUI crate (README, "Installation").

The exact pins stay when `0.1.0` is released. They can be relaxed to caret requirements once the crates keep semantic versioning towards one another individually, which is a later decision.

**One place.** The version is stated in `[workspace.package]` of the root manifest, which every member inherits (`version.workspace = true`), and in the `version` keys of the path dependencies in `[workspace.dependencies]`, which every member uses (`workspace = true`). One script writes both and verifies the result:

```sh
scripts/release/set-version.sh 0.1.0-preview.2
```

It rewrites the root manifest, runs `cargo metadata` (which also brings `Cargo.lock` up to date), and fails if a crate does not have the new version or a dependency between crates of the workspace is not pinned to it. The check of the continuous integration fails for the same reasons, so a crate that states its own version or its own requirement is found when it is added.

A version that was published is never reused. If a release stops half way and cannot be completed (section 8.3), the next attempt is the next preview number.

## 4. Metadata

`crates.py check` requires of every published crate: `description`, `license`, `repository`, `readme`, `keywords` (at most five) and `categories`, and the group of the README table. `homepage`, `readme`, `keywords` and `categories` are stated once in `[workspace.package]` and inherited; the build crates, the interop crates and `xamlx` state keywords and categories of their own. The `readme` is the README of the repository: Cargo copies a readme from outside the package directory into the package.

The descriptions are one line each and do not name the upstream project. The rule of the port holds for manifests as for sources: the upstream name appears only in `docs/`, `scripts/`, the README and the `NOTICE` files.

Nine crates had no description and received one: `ferroui-base`, `ferroui-controls`, `ferroui-markup`, `ferroui-desktop`, `ferroui-harfbuzz`, `ferroui-skia`, `ferroui-native`, `ferroui-microcom` and `microcom-codegen`.

No crate has a git dependency. The `links` keys (the crates that export a type model or compiled markup to the build scripts of their dependents) are unique, which `crates.py check` verifies.

## 5. Licence and notices

Every crate is `license = "MIT"`. Cargo accepts that key alone, but the licence text and the attributions have to travel with the code: the project is a port, the notices of the upstream projects live in the `NOTICE` files, and the component-level notices refer to the notice at the root of the repository. A package holds only files of its own directory, so every published crate directory has two symbolic links, which `cargo package` follows:

| In the package | Is |
|---|---|
| `LICENSE` | the `LICENSE` of the repository |
| `NOTICE-PROJECT.md` | the `NOTICE.md` of the repository (the project-wide attributions and licence texts) |
| `NOTICE.md` | the notice of the crate itself, where it has one (and the notices next to the code they cover, in its subdirectories) |

For `xamlx`, whose notice is at `external/XamlX/NOTICE.md`, above the package directory, `NOTICE.md` of the package is a link to it. The package of `ferroui-native` also holds `native/NOTICE.md`, the notice of the native library.

The links are needed for packaging only. A checkout without symbolic links (Git for Windows without the permission to create them) builds and tests as before; packages are assembled on Linux and macOS.

`crates.py check` fails for a published crate without the two links.

## 6. What a package holds

A published crate is built from its package: the files of its own directory, minus what `exclude` leaves out. Three crates read files of other directories, one package was too large, and two crates could not be ordered. What changed:

| Crate | Problem | Change |
|---|---|---|
| `ferroui-skia` | 32 MB of test fonts and images under `test_assets/` (the package was 33 MB) | `exclude = ["test_assets/"]`. Only the tests read them (`#[cfg(test)]` modules); the render backends' tests in other crates that reach them by a relative path are tests too. The package is 234 KiB. |
| `ferroui-base` | 1.2 MB of test fonts under `test_assets/` | `exclude = ["test_assets/"]`, for the same reason: only `#[cfg(test)]` code embeds them. The `testing` features of the base library and the controls, which other crates enable for their own tests, read no file. |
| `ferroui-native` | The build script compiled `../../native/FerroUI.Native` | `src/FerroUI.Native/native` is a symbolic link to that directory; the build script reads through it (and falls back to the path in the repository where the link is not a directory), and the package holds the native sources, the headers, the patches and the notice of the native library. |
| `ferroui-remote-protocol` | `#[path = "../FerroUI.Base/input/key.rs"]`: the two key enumerations are compiled from the sources of the base library | Copies under `src/FerroUI.Remote.Protocol/input/`, and a test (`the_key_enumerations_are_the_files_of_the_base_library`) that fails when a copy differs from its original. Not symbolic links: this crate is built on every platform, including checkouts without them. |
| `ferroui-themes-simple` | The build script embedded `../FerroUI.Themes.Fluent/Strings/InvariantResources.xaml` | The document is a file of the crate, `Strings/InvariantResources.xaml`, written by the converter from the same upstream document (`scripts/sync-simple-theme.sh`, whose `--check` keeps it current). |
| `ferroui-fonts-inter` | Development dependency cycle with `ferroui-skia` | Section 1.3. |

The other build scripts stay inside their package: the type model export (`ferroui-build-scan`) and the markup compiler (`ferroui-build`) read the sources and documents of the crate that is being built from `CARGO_MANIFEST_DIR`, and the models of its dependencies from the paths Cargo hands over (`DEP_<CRATE>_XAML_XAMLMETA`). The Skia build script compiles one file of its own directory for the browser target.

The tests are in the packages (their sources are small) but are not built from them: `cargo package` builds the library of a package, and the test suites are run from the repository. No `include_bytes!` or `include_str!` outside test code names an excluded file; the dry run proves it by building every package.

### 6.1 Native build requirements

What someone who depends on the published crates needs, beside Rust 1.89:

- **Skia** (`ferroui-skia`, through `skia-safe`): its build downloads a prebuilt Skia for the target and feature set, or builds Skia from source when there is none (which needs Python, a C++ toolchain, network access and a long time). The feature sets FerroUI asks for have prebuilt binaries on macOS (Graphite on Metal) and for the browser (Ganesh on WebGL). Nothing of Skia is in the package.
- **HarfBuzz** (`ferroui-harfbuzz`, through `harfbuzz-sys` with its bundled sources): built from source, needs a C++ compiler.
- **The macOS platform** (`ferroui-native`): the build script generates the interop bindings from `frn.idl` and compiles the Objective-C++ library with the system compiler; the Xcode command line tools are needed. Everything it compiles is in the package.
- **The browser platform** (`ferroui-browser`): Rust 1.93 or later, the `wasm32-unknown-emscripten` target, Emscripten, and the `wasm-bindgen` command-line tool at the exact version the workspace pins (`docs/porting/browser-platform.md`). The script module of the host page (`webapp/`) is in the package; the scripts that assemble a site (`scripts/build-browser.sh`) are in the repository and not in any package, which is a known limitation of the preview.

## 7. docs.rs

docs.rs builds the documentation of every published version on a Linux host without network access. What is expected, none of it verified on docs.rs itself (that is only possible after a publish):

| Crates | Expectation |
|---|---|
| The crates without native dependencies (the base library, controls, markup, themes, dialogs, build crates, protocol, designer support, `ferroui-metal`, `ferroui-opengl`, `ferroui-microcom`, `xamlx`, `microcom-codegen`) | Build. The build scripts of the themes compile their markup, which takes a few minutes and needs no network. |
| `ferroui-native` | `[package.metadata.docs.rs]` sets the macOS targets (the crate is empty elsewhere). On docs.rs the build script generates the bindings and skips the native library (`DOCS_RS`), since the host has no compiler for Objective-C++ against the macOS SDK and documentation links nothing. Expected to build. |
| `ferroui-vello` | Documented with the `hybrid` and `gpu` features for `aarch64-apple-darwin`, the target whose window they draw into. Its dependencies are Rust only. Expected to build. |
| `ferroui-harfbuzz`, `ferroui-headless` | Depend on the HarfBuzz build from source, which needs the C++ compiler of the docs.rs image and no network. Expected to build for the default Linux target. |
| `ferroui-skia`, `ferroui-desktop`, `ferroui-browser` | Depend on the build of `skia-bindings`, which without network cannot download Skia. The Skia bindings are documented on docs.rs themselves, so their build has a way to do without the binaries there; whether it carries through to a dependent crate with the features FerroUI enables is not known. **May fail**; if it does, the remedy is a docs.rs feature set or target for these three crates, in the next preview. `ferroui-browser` is documented for the Linux host, not for `wasm32-unknown-emscripten`, which docs.rs cannot build (no Emscripten). |

A failed documentation build does not affect the crate on crates.io: it shows as a failed badge in the README table. The first preview is the test.

## 8. Scripts and workflow

| File | What it does |
|---|---|
| `scripts/release/crates.py` | The set, the order, the checks, the version, the README table, and the question whether crates.io has a version (the sparse index; no token). |
| `scripts/release/set-version.sh <version>` | Section 3. |
| `scripts/release/readme-crates-table.sh [--check]` | Regenerates the crate table of the README between its markers. |
| `scripts/release/publish.sh --dry-run` or `--publish` | The one list of what a release does, for the workflow and for a maintainer. |
| `.github/workflows/release.yml` | The release workflow. |
| `.github/workflows/ci.yml`, job `packaging` | Keeps the crates publishable. |

### 8.1 The dry run

```sh
scripts/release/publish.sh --dry-run              # package every crate and build each from its package
scripts/release/publish.sh --dry-run --no-verify  # package only (what the continuous integration runs)
```

It runs `crates.py check` and the check of the README table, packages the published crates with `cargo package --workspace` (the unpublished ones excluded by the rule), builds each package unless `--no-verify` is given, and prints every package with its size and whether crates.io has that version. It fails for a package above 10 MiB. It needs Cargo 1.90 or later and publishes nothing; it does not read the token. `--allow-dirty` packages a working tree with changes that are not committed.

### 8.2 The check of the continuous integration

The job `packaging` of `ci.yml` runs `publish.sh --dry-run --no-verify` on Linux for every pull request and push that changes more than documentation. It compiles nothing: it installs the toolchain, resolves the dependencies and assembles 26 packages. Locally the script takes about half a minute with a warm registry cache, most of it the resolution of each package; on a runner about two minutes with the installation of the toolchain and a cold cache (an estimate: the job has not run on a runner yet). It catches a crate without metadata, a dependency that is not pinned, a dependency of a published crate on an unpublished one, a dependency cycle, a package above the size limit, a missing notice and a stale README table. It does not catch a package that no longer builds (a build script that reaches outside its package again): that is the full dry run, which the release workflow runs, and which is worth running by hand after a change of a build script.

### 8.3 The release workflow

`release.yml` is started by hand (`workflow_dispatch`, with the `version` and `dry_run`, which is on by default) or by a pushed tag `v<version>`, which is a real release. Two jobs:

**`verify`** (macOS, the platform every crate builds on with its real backend):

1. Checks out the commit and installs the toolchain (the same pinned action as the continuous integration).
2. Compares the tag or the input with the version of the workspace and fails if they differ. The workflow never sets a version: that is a commit.
3. Requires an entry for the version in `CHANGELOG.md`.
4. For a real release, requires a successful run of the CI workflow for the commit. This covers the browser jobs, which are not repeated here.
5. Runs the tests of the workspace (`cargo test --workspace --locked --no-fail-fast --lib --tests`, the command of the CI). They are run here and not taken from the CI because the macOS job of the CI runs for pull requests only, not for a push to main: the commit of a release has not necessarily been tested as it is.
6. Runs the full dry run: every crate is packaged and built from its package.

**`publish`** (Linux; only for a real release, only after `verify`, in the environment `crates-io`):

1. Runs `publish.sh --publish --no-verify` with `CARGO_REGISTRY_TOKEN` taken from the repository secret into the environment of that one step. The token is never printed or passed on a command line. `--no-verify` because `verify` has just built every crate from the same packages; building them again on the runner that waits for crates.io adds nothing.
2. Creates the GitHub release `v<version>` with generated notes (marked as a pre-release for a preview), unless it exists.

**Order and waiting.** The crates are published one at a time in the order of section 1.3 with `cargo publish -p <crate>`. Cargo waits after an upload until the index has the new version before it returns (since Cargo 1.66; the toolchain of the workflow is the current stable, and the dry run requires 1.90 anyway). Because that wait has a timeout, the script also asks the index itself before it goes on to the next crate.

**Running it again.** Before each crate the script asks the sparse index whether that version exists and skips the crate if it does. A release that stopped (a network error, the timeout of the job) is started again and continues where it stopped. If the index cannot be reached the script stops instead of guessing.

**Rate limits.** crates.io lets an account publish a small burst of new crates (five, at the time of writing) and then one new crate every ten minutes; new versions of existing crates have a far higher limit. The first release creates 26 new crates, so it takes about three and a half hours unless crates.io raises the limit for the account, which its team does on request (help@crates.io). The script waits and tries again when a publish is refused for this reason, and the job has a timeout of 350 minutes; if the job ends first, it is run again and continues. Later previews are not affected.

**If a crate cannot be published.** The crates before it in the order are on crates.io at that version and stay there. Fix the cause and run again if the fix does not change a crate that was already uploaded; otherwise yank the uploaded crates of that version (`cargo yank`, by the owner) and release the next preview number.

## 9. What "ready" means: the checklist of a preview

Before the release:

- [ ] The CI is green on `main`, including a run of the macOS job on the merged state (dispatch the CI workflow by hand on `main`; it does not run for a push).
- [ ] The tracking pages are current: `docs/porting/CRITICAL-PATH.md` and the generated `docs/porting/TRACKING.md`.
- [ ] The README states the status honestly: what works on which platform (macOS desktop and the browser; Windows and Linux in progress; the Vello backend experimental), and that the release is a preview.
- [ ] `CHANGELOG.md` has the entry of the version, with what it contains and its known limitations, and the date.
- [ ] `scripts/release/set-version.sh <version>` was run if the version changed, and the installation section of the README names the version.
- [ ] `scripts/release/readme-crates-table.sh --check` passes.
- [ ] The release workflow was run with `dry_run` on for the commit, and passed.
- [ ] For the first release only: the decisions of section 12 are made.

The release:

- [ ] Run the release workflow with `dry_run` off, or push the tag `v<version>` for the commit.
- [ ] Check that crates.io lists every crate of section 1.1 at the version (the last lines of the job say how many were published and how many skipped).

After the release:

- [ ] At the first release only: remove the note between the `release-note` markers of the README ("Upcoming"), and the sentence of its status paragraph that says nothing has been published.
- [ ] Set the date of the entry in `CHANGELOG.md` and point its link at the tag.
- [ ] Look at the docs.rs badges of the README table after an hour (section 7) and note the failures for the next preview.
- [ ] Set the next version (`scripts/release/set-version.sh 0.1.0-preview.<N+1>`), so that `main` never carries a version that is on crates.io with other contents.

## 10. A preview that turns out broken

Yank every crate of that version (the owner, with `cargo yank --version <version> <crate>` for each crate of section 1.1) and release the next preview. A yanked version stays downloadable for builds that have it in a lock file. Nothing is ever removed.

## 11. What is not done yet

- The facade crate `ferroui` (section 2.1).
- Per-crate README files: every package carries the README of the repository.
- The site assembly scripts of the browser platform are not in a package (section 6.1).
- The documentation builds on docs.rs are unverified (section 7).

None of these blocks a preview.

## 12. Decisions before the first release

1. **Go.** The first publish creates 26 crates under the owner's account for good. Everything up to it is rehearsed by the dry run.
2. **The names `xamlx` and `microcom-codegen`**, or `ferroui-xamlx` and `ferroui-microcom-codegen` (section 2).
3. **The bare name `ferroui`**: publish a facade with the first preview, a thin placeholder that re-exports, or leave it unclaimed for now (section 2.1).
4. **The rate limit**: ask crates.io to raise the limit of new crates for the account before the first release, or accept a first release of about three and a half hours (section 8.3).
5. **The environment `crates-io`** of the repository: add required reviewers to it, so that the job that publishes waits for an approval. Without that setting the environment exists and protects nothing.
6. **The token**: `CARGO_REGISTRY_TOKEN` should be a token scoped to publishing new crates and new versions, with no other scope. After the first release it can be replaced by trusted publishing of crates.io for this repository and workflow, which needs no stored token (a crate has to exist before it can be configured).
7. **Whether the unfinished platforms are in the first preview.** Crates added to the workspace before the release (the Windows and Linux platforms in progress) are published with it unless they say `publish = false`. A crate that is not ready to be claimed and seen should say so until it is.
8. **The date and the wording of the changelog entry.**
