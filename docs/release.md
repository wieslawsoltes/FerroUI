# Releasing FerroUI

How the crates of the workspace are published to crates.io: which crates, in which order, at which version, what each package holds, how the release workflow runs and what has to be true before it is run. Four previews (`0.1.0-preview.1` to `0.1.0-preview.4`), the first release that is not a preview, `0.1.0` (tag `v0.1.0`, 38 crates), `0.2.0` (tag `v0.2.0`, 40 crates: the two crates of the TreeDataGrid joined), the preview `0.3.0-preview.1` (tag `v0.3.0-preview.1`, 48 crates: the eight crates of the reactive foundation joined) `0.3.0` (tag `v0.3.0`, the same 48 crates) and the preview `0.4.0-preview.1` (tag `v0.4.0-preview.1`, 50 crates: the two crates of the declarative API joined) are published; the workspace carries the next version. Section 12 records the decisions the owner made before the first release.

Publishing cannot be taken back. A version of a crate can be yanked (Cargo then no longer selects it for new builds) but never removed or replaced, and the name of a crate belongs to the account that published it first. Everything below is arranged so that a release is rehearsed completely before anything is uploaded.

## 1. What is published

**The rule.** A member of the workspace is published unless its manifest says `publish = false`. No list of crates is kept anywhere: the scripts, the release workflow, the check of the continuous integration and the crate table of the README derive the set and its order from `cargo metadata` (`scripts/release/crates.py`).

```sh
scripts/release/crates.py order         # the published crates, in the order they are published
scripts/release/crates.py unpublished   # the crates that are never published
scripts/release/crates.py check         # what a published crate has to state
```

### 1.1 The published crates

**Now (2026-10-16): 38 crates**, every one with code; `scripts/release/crates.py order` prints them in the order of publication, and the crate table of the README lists them by group. The table below is the state on 2026-10-10, the first 26; the paragraphs after it say how the others joined.

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
| 26 | `ferroui-ios` | `[package.metadata.docs.rs]` sets the iOS targets (the crate is empty elsewhere). The first preview failed there: the build script of `harfbuzz-sys` (through `ferroui-harfbuzz`) compiles HarfBuzz with `cc`, which for an Apple target asks `xcrun` for the iOS SDK, and the Linux host has neither; `harfbuzz-sys` has no `DOCS_RS` handling, and Skia's documentation bindings (which `skia-bindings` uses when `DOCS_RS` is set) have neither Graphite nor Metal, which `ferroui-skia` builds with on Apple targets. The crate needs neither for its documentation, so since 0.1.0-preview.2 the metadata sets the configuration `ferroui_docsrs` (`rustc-args` for Cargo and the compiler, `rustdoc-args` for the sources): the manifest declares `ferroui-harfbuzz` and `ferroui-skia` under `cfg(not(ferroui_docsrs))`, and `use_ios` selects them under the same configuration. Every other build is unchanged. Verified on Linux (Ubuntu 24.04 ARM64, `DOCS_RS=1`, the configuration `docsrs` for rustdoc): `cargo doc -p ferroui-ios --target aarch64-apple-ios` fails before the change with the error of docs.rs (stable and nightly, with and without the dependencies documented) and builds after it with the arguments of the metadata (stable and nightly, with and without the dependencies documented, and for `aarch64-apple-ios-sim`): 289 pages, the platform and `use_ios` among them. |
| `ferroui-vello` | `src/Vello/FerroUI.Vello` | Render backends and text | 319 KiB |

The build-time crates are part of the set because published crates name them: `ferroui-build-scan` is a build dependency of the base library, the controls, the XAML run-time library and the OpenGL contracts (it exports the type model of a crate); `ferroui-build` is a build dependency of every crate with compiled markup (the dialogs, the themes, the colour picker) and of an application that compiles its markup; `microcom-codegen` is a build dependency of the macOS platform. `xamlx`, the port of an external library under `external/XamlX`, is a dependency of the loader, the build crate and the designer support. The workspace has no procedural macro crate.

Joined since the table was made (2026-10-10): `ferroui-win32` (`src/Windows/FerroUI.Win32`, Platforms, 155 KiB), packaged by the dry run with the others; with `ferroui-x11` (`src/FerroUI.X11`, Platforms), which joined the same day, the dry run packages 28, and with `ferroui-freedesktop` (`src/FerroUI.FreeDesktop`, Platforms; the FreeDesktop services the Linux platforms share, a dependency of `ferroui-x11`) 29. Its default feature `angle` brings a dependency of Windows targets only (the crate `mozangle`), so the package builds on the hosts of the release like the rest. `ferroui-android` (`src/Android/FerroUI.Android`, Platforms) joined the same day (the dry run packages 31 with it, `ferroui-ios` and `ferroui-freedesktop`). Its dependencies on the renderer, the text shaper, `jni-sys` and `libc` are dependencies of Android targets only, and its package carries the Java sources of the backend (`java/`), which an application compiles with the SDK (`scripts/android/apk.sh`).

`ferroui-wayland` (`src/FerroUI.Wayland`, Platforms; the Wayland platform backend, 135 KiB) joined on 2026-10-10 as well: the dry run packages 32 with it.

The first preview added the five other names of section 2.2 as placeholders (37 packages). Each of the six placeholders is a real crate since 2026-10-15, at the same path and name: `ferroui` (the facade, section 2.1), `ferroui-wayland`, `ferroui-vulkan`, `ferroui-linuxframebuffer`, `ferroui-winui` and `ferroui-headless-vnc`. The set is still 37 crates. Its dependencies on the Wayland crates, `xkbcommon-dl` and `libc` are dependencies of Linux targets only, and none of them links a system library (they are opened at run time), so the package builds on the hosts of the release like the rest.

`ferroui-themes-winui` (`src/FerroUI.Themes.WinUI`, Controls and themes; the WinUI 3 theme, docs/porting/winui-theme.md) joined on 2026-10-16: 38 crates. It is an optional theme of the facade (`winui-theme`).

`ferroui-system` (`src/FerroUI.System`, Core; the parts of the .NET base class library that FerroUI and its reusable libraries share, docs/porting/reactive.md) joined on 2026-10-18 for `0.3.0-preview.1`: it has no dependency on the framework, `ferroui-base` depends on it and re-exports its items at the paths they had in the base library. It holds `ferro_thread_local!`, so it is published before every crate that declares thread-local variables. `ferroui-reactive` (`src/FerroUI.Reactive`, Core; the port of System.Reactive) and `ferroui-reactive-testing` (`src/FerroUI.Reactive.Testing`, Core; the port of Microsoft.Reactive.Testing) joined the same day; the unit tests of the port are the unpublished crate `ferroui-reactive-tests`.

`ferroui-splat` (`src/FerroUI.Splat`, Core; the port of Splat) and `ferroui-dynamic-data` (`src/FerroUI.DynamicData`, Core; the port of DynamicData) joined on 2026-10-19, and so did `ferroui-reactiveui` (`src/FerroUI.ReactiveUI`, Core; the port of ReactiveUI with the run time of ReactiveUI.Binding) and `ferroui-reactiveui-testing` (`src/FerroUI.ReactiveUI.Testing`, Core; the port of ReactiveUI.Testing). None of them depends on the framework. `ferroui-reactiveui-controls` (`src/FerroUI.ReactiveUI.Controls`, Controls; the integration of ReactiveUI with FerroUI) joined the same day; its unit tests are in the crate. Their unit tests are the unpublished crates `ferroui-splat-tests`, `ferroui-dynamic-data-tests`, `ferroui-reactiveui-tests` and `ferroui-reactiveui-testing-tests`.

`ferroui-view` (`src/FerroUI.View`, Controls and themes; the declarative API, docs/declarative-ui.md) and `ferroui-view-macros` (`src/FerroUI.View.Macros`, Controls and themes; its procedural macros `rsx!` and `#[component]`) joined on 2026-10-20 for `0.4.0-preview.1`; their tests are in `ferroui-view`, and the 7GUIs samples that use them are not published.

`ferroui-diagnostics` (`src/FerroUI.Diagnostics`, Controls and themes; the developer tools) joined on 2026-10-22. It compiles its markup in its build script (`links = "ferroui_diagnostics_xaml"`) and depends on the simple theme, the colour picker and the TreeDataGrid, so it is published after them; it is an optional part of the facade (`diagnostics`).

`ferroui-dependency-injection` (`src/FerroUI.DependencyInjection`, Core; the port of Microsoft.Extensions.DependencyInjection, docs/porting/dependency-injection.md) joined on 2026-10-22. It does not depend on the framework; its optional feature `splat` adds the adapter to Splat's resolver. Its unit tests are the unpublished crate `ferroui-dependency-injection-tests`.

`ferroui-inspect` (`src/FerroUI.Inspect`, Tools; the inspection core, docs/mcp.md) and `ferroui-mcp` (`src/FerroUI.Mcp`, Tools; the opt-in MCP server over it) joined on 2026-10-22. They are not part of the facade; an application depends on `ferroui-mcp` directly, as the ControlCatalog's `mcp` feature does. Their unit tests are in the crates.

`ferroui-devtools` (`src/FerroUI.DevTools`, Controls and themes; the opt-in Chrome DevTools Protocol endpoint, docs/devtools-cdp.md) joined the same day. It is built on the `target` module of `ferroui-inspect`, so the core is published before it and it before the facade, where it is the optional feature `devtools`. The native endpoint is its default feature `server`; the browser bridge is the feature `browser`, with the default features off. Its unit tests are in the crate.

`ferroui-controls-charts` (`src/FerroUI.Controls.Charts`, Controls and themes; the port of the public contract of WinUI Charts, docs/porting/winui-charts.md) joined with #282. It is an optional part of the facade (`charts`), and its sample `samples/ChartsDemo` is not published.

Every crate was checked in the same way: packaged, and built from its package, by `scripts/release/publish.sh --dry-run` (section 8). `cargo publish --dry-run -p <crate>` cannot be used for this before the first release: it resolves the dependencies of the package in the registry, where the other crates of the workspace are not yet. `cargo package --workspace` (Cargo 1.90 and later) resolves them among the packages it has just assembled instead, which is what the dry run uses, and so every crate got the full check, not only the ones without dependencies in the workspace.

### 1.2 The crates that are never published

Each states `publish = false`.

| Crate | Directory | What it is |
|---|---|---|
| `control-catalog` | `samples/ControlCatalog` | Sample application |
| `control-catalog-desktop` | `samples/ControlCatalog.Desktop` | Desktop entry point of the sample |
| `control-catalog-browser` | `samples/ControlCatalog.Browser` | Browser entry point of the sample |
| `mini-mvvm` | `samples/MiniMvvm` | View-model library of the samples |
| `render-demo`, `binding-demo`, `virtualization-demo`, `text-test-app`, `sandbox`, `app-without-lifetime`, `integration-test-app` | `samples/RenderDemo`, `samples/BindingDemo`, `samples/VirtualizationDemo`, `samples/TextTestApp`, `samples/Sandbox`, `samples/AppWithoutLifetime`, `samples/IntegrationTestApp` | Sample applications, each with its desktop entry point |
| `control-samples` | `samples/SampleControls` | Shared controls of the samples |
| `sample-build`, `sample-support`, `sample-testing` | `samples/SampleBuild`, `samples/SampleSupport`, `samples/SampleTesting` | Build script, markup tables and test harness of the small samples |
| `ferroui-leak-tests` | `tests/FerroUI.LeakTests` | Leak tests |
| `ferroui-benchmarks` | `tests/FerroUI.Benchmarks` | Benchmarks |
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

No name was taken then, so nothing blocked on names. A name is claimed by the first publish and by nothing else; the first preview claimed all 37 names below for the project. All 37 names of the first preview (the 31 crates with code and the six reserved names of section 2.2) were looked up again on 2026-10-10 before the release, and all were free.

Two names are not under the `ferroui-` prefix. The owner decided on 2026-10-10 to keep the names as they are (section 12): `xamlx` is the name of the external library the crate is a port of, and `microcom-codegen` is named after the interop library whose design it follows. Publishing them claims those names on crates.io for this project. The alternatives are `ferroui-xamlx` and `ferroui-microcom-codegen`; a rename touches the manifests, the `use` paths and the profile overrides of the root manifest.

### 2.1 The bare name `ferroui`: the facade crate

`ferroui` is the name people will try first. It was published with the first preview as an empty placeholder (section 2.2); for the second preview it is the facade crate, built as proposed here before (2026-10-15):

- **No code of its own.** `src/FerroUI/lib.rs` holds re-exports, a prelude of re-exports and documentation, and `#![forbid(unsafe_code)]`. Every module of the crate is one crate of the workspace, under the name of that crate without the `ferroui-` prefix (`-` becomes `_`), so the module of a crate never has to be looked up: `ferroui-base` is `ferroui::base`, `ferroui-themes-fluent` is `ferroui::themes_fluent`.
- **Features choose the parts**, so that an application names one dependency, `ferroui = "=0.1.0-preview.2"`:

  | Feature | Brings | Modules | Default |
  |---|---|---|---|
  | (none) | `ferroui-base`, `ferroui-controls`, `ferroui-markup` | `base`, `controls`, `markup` | always |
  | `desktop` | `ferroui-desktop` (platform detection) with Skia, HarfBuzz and the platform of the operating system: `ferroui-native` on macOS, `ferroui-win32` on Windows, `ferroui-x11` on Linux | `desktop`, `skia`, `harfbuzz`, `native` / `win32` / `x11` | yes |
  | `fluent` | `ferroui-themes-fluent` | `themes_fluent` | yes |
  | `simple` | `ferroui-themes-simple` | `themes_simple` | no |
  | `winui-theme` | `ferroui-themes-winui` | `themes_winui` | no |
  | `xaml` | `ferroui-markup-xaml` and the run-time loader (`ferroui-markup-xaml-loader` with its feature `runtime`) | `markup_xaml`, `markup_xaml_loader` | no |
  | `color-picker`, `dialogs` | `ferroui-controls-color-picker`, `ferroui-dialogs` | `controls_color_picker`, `dialogs` | no |
  | `charts` | `ferroui-controls-charts` (WinUI public-contract port) | `controls_charts` | no |
  | `devtools` | `ferroui-devtools`, `ferroui-inspect` (opt-in CDP inspection and automation) | `devtools` | no |
  | `diagnostics` | `ferroui-diagnostics` (added for 0.4) | `diagnostics` | no |
  | `treedatagrid` | `ferroui-controls-treedatagrid` and `ferroui-treedatagrid-core` (added for 0.2) | `controls_treedatagrid`, `treedatagrid_core` | no |
  | `browser` | `ferroui-browser`, with Skia and HarfBuzz | `browser`, `skia`, `harfbuzz` | no |
  | `vello`, `vello-hybrid`, `vello-gpu` | `ferroui-vello`; the second and third turn on its features `hybrid` and `gpu` | `vello` | no |
  | `headless` | `ferroui-headless` | `headless` | no |
  | `opengl`, `metal`, `vulkan` | the platform graphics contracts `ferroui-opengl`, `ferroui-metal`, `ferroui-vulkan` | `opengl`, `metal`, `vulkan` | no |

  The last row was not in the proposal. The contracts are what an application names when a control of its own draws with the graphics API (an OpenGL control, the import of external objects), and upstream's meta package brings them; they are small and of Rust alone, so they are features and not defaults. An application for the browser turns the default features off and names `browser` and a theme.
- **The prelude** (`ferroui::prelude`) holds what a minimal application names, which is what the minimal applications of the repository import (`src/FerroUI.Desktop/examples/hello_window.rs`, `src/FerroUI.Themes.Simple/examples/themed_window.rs`, the `App` and `Program` of the small samples): the declaration of an application class (`ferro_class!`, `ferro_impl_classes!`, `instantiate`, `Ref`, `FerroObjectImpl`, `Application`, `ApplicationImpl`, `ApplicationImplExt`, `NewApplication`), starting it (`AppBuilder`, and `AppBuilderDesktopExtensions` with `desktop`), the window and the common controls and panels (`Window`, `Control`, `Border`, `Button`, `CheckBox`, `TextBlock`, `TextBox`, `StackPanel`, `Grid`), the values of a layout (`Thickness`, `CornerRadius`, `HorizontalAlignment`, `VerticalAlignment`, `Orientation`, `Color`) and the theme of the features that are on (`FluentTheme`, `SimpleTheme`). Nothing else: the rest is reached through the modules, so that a glob import of the prelude does not shadow the names of an application.
- **Not in the facade.** The build-time crates (`ferroui-build`, `ferroui-build-scan`, `xamlx`): a build dependency cannot be re-exported, and the code the markup compiler writes names the crates it calls by their own names (`::ferroui_base::...`, `::ferroui_markup_xaml::...`), so a crate that compiles its markup depends on those crates directly. The tools and the designer support (`ferroui-designer-support`, `ferroui-remote-protocol`, `microcom-codegen`, `ferroui-mcp`), the interop runtime of the native platforms (`ferroui-microcom`), the font collection (`ferroui-fonts-inter`), the platforms that are opted into by name (`ferroui-wayland`, `ferroui-freedesktop`, `ferroui-linuxframebuffer`, `ferroui-ios`, `ferroui-android`), the WinUI integration (`ferroui-winui`) and the VNC server of the headless platform (`ferroui-headless-vnc`): an application names those crates beside the facade.
- **Upstream's counterpart** is the meta package `Avalonia` (`packages/Avalonia/Avalonia.csproj`): it references the core libraries of `build/CoreLibraries.props` (base, controls, designer support, OpenGL, Metal, Vulkan, dialogs, markup, XAML, MicroCom), the remote protocol, the build tasks, the source generators and the analyzers, and carries the props and targets of the build and the previewer. The desktop entry point and the themes are separate packages there (`Avalonia.Desktop`, `Avalonia.Themes.Fluent`); here they are features of the facade, on by default, because the facade is the one dependency of an application. MSBuild props and targets have no counterpart (Cargo has no build integration a library can bring), and the build tasks and generators are the build script of an application.
- It is pinned to the other crates like every crate of the workspace, so `ferroui = "=0.1.0-preview.2"` selects one consistent set, which is the main use of it during the previews. It depends on 22 crates of the workspace and is published after all of them (`crates.py order`).
- **Verified** (2026-10-15, macOS): the example `hello` (`src/FerroUI/examples/hello.rs`, a window with a text block and a button under the Fluent theme, written against the prelude alone) builds and runs (`FERROUI_SMOKE_EXIT_MS=2500 cargo run -p ferroui --example hello`: the window opens and closes, exit code 0); the two documentation tests of `lib.rs` pass (a complete application, compiled; the builder after platform detection, run); `cargo check -p ferroui` passes with no features and with every feature; the documentation builds without warnings with the features of docs.rs.

### 2.2 Reserved names

The owner decided on 2026-10-10 to reserve, with the first preview, the names of the parts that did not exist yet. Each was published with `0.1.0-preview.1` as a placeholder: a member of the workspace with a manifest, a crate comment that said it is a placeholder, no code and no dependencies, and a description on crates.io that started with "Reserved name". All six are real crates since 2026-10-15, at the same path and name, and are published with real descriptions from `0.1.0-preview.2` on; no crate of the workspace is a placeholder any more.

| Crate | Directory | Now |
|---|---|---|
| `ferroui` | `src/FerroUI` | the facade crate of section 2.1 |
| `ferroui-wayland` | `src/FerroUI.Wayland` | the Wayland platform (upstream `Avalonia.Wayland`; `docs/porting/wayland-platform.md`) |
| `ferroui-vulkan` | `src/FerroUI.Vulkan` | the Vulkan platform contracts (upstream `Avalonia.Vulkan`; `docs/porting/vulkan.md`) |
| `ferroui-linuxframebuffer` | `src/Linux/FerroUI.LinuxFramebuffer` | the Linux framebuffer platform (upstream `Avalonia.LinuxFramebuffer`; `docs/porting/linux-framebuffer-platform.md`) |
| `ferroui-winui` | `src/Windows/FerroUI.WinUI` | the WinUI integration (upstream `Avalonia.WinUI`; `docs/porting/winui.md`) |
| `ferroui-headless-vnc` | `src/Headless/FerroUI.Headless.Vnc` | the VNC server of the headless platform (upstream `Avalonia.Headless.Vnc`; `docs/porting/headless-vnc.md`) |

crates.io has no way to reserve a prefix: names are flat, and owning `ferroui` or any `ferroui-*` crate gives no right over other names that begin the same way. Publishing a name is the only protection. Not reserved: a name for the UI Automation bridge of Windows (a module of `ferroui-win32`, `automation`) and names for the upstream projects that have no counterpart in Rust (the interoperability with other .NET toolkits, the adapters of .NET test frameworks, the analyzers).

## 3. Version scheme

**Semantic versioning, one version for the whole workspace, previews as pre-releases.** The previews are `0.1.0-preview.1`, `0.1.0-preview.2`, ... and then `0.1.0`. In the order of semantic versioning `0.1.0-preview.1 < 0.1.0-preview.2 < 0.1.0`; the numeric identifier after `preview.` is compared as a number, so `preview.10` follows `preview.9`.

Why pre-release identifiers and not `0.1.0`, `0.1.1`, ...: Cargo treats `0.1.x` versions as compatible with one another, and a preview is not compatible with the one before it. A pre-release says so, and Cargo never selects a pre-release unless the requirement names one: `ferroui-base = "0.1"` does not match `0.1.0-preview.1`. Nobody gets a preview by accident, and when `0.1.0` is published, requirements written for it never fall back to a preview.

**The crates of the workspace are pinned to one another exactly.** A requirement that names a pre-release without `=` (`"0.1.0-preview.1"`) is a caret requirement: it matches that preview, every later preview of `0.1.0`, and `0.1.x` releases. Cargo would then be free to build `ferroui-controls 0.1.0-preview.1` against `ferroui-base 0.1.0-preview.3`, which was never built or tested together and, between previews, will not compile. So every dependency between crates of the workspace is `version = "=<the version of the workspace>"`: the crates of a release resolve only as the set they were released as. An application does the same and names one version for every FerroUI crate (README, "Installation").

After `0.1.0` the scheme repeats for the next minor version: its previews are `0.2.0-preview.1`, `0.2.0-preview.2`, ... and then `0.2.0`; a fix of `0.1.0` that keeps its public API would be `0.1.1`. `0.2.0` was released without a preview of it on crates.io (the workspace carried `0.2.0-preview.1` on `main` only); the work towards `0.3.0` was published as its preview `0.3.0-preview.1`, and `0.3.0` followed it with the same contents. The work towards `0.4.0` is published as its previews, from `0.4.0-preview.1`.

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

docs.rs builds the documentation of every published version on a Linux host without network access. **The first preview on docs.rs** (`https://docs.rs/crate/<crate>/0.1.0-preview.1/status.json`, read on 2026-10-15): 36 of the 37 crates built, the Skia backend, the desktop entry point, the browser platform and the facade among them; `ferroui-ios` failed. The six placeholders of section 2.2 had no code then, so the facade, Wayland, Vulkan, the framebuffer platform, WinUI and the VNC server are first documented with the second preview. What was expected before the first publish:

| Crates | Expectation |
|---|---|
| The crates without native dependencies (the base library, controls, markup, themes, dialogs, build crates, protocol, designer support, `ferroui-metal`, `ferroui-opengl`, `ferroui-microcom`, `xamlx`, `microcom-codegen`) | Build. The build scripts of the themes compile their markup, which takes a few minutes and needs no network. |
| `ferroui-native` | `[package.metadata.docs.rs]` sets the macOS targets (the crate is empty elsewhere). On docs.rs the build script generates the bindings and skips the native library (`DOCS_RS`), since the host has no compiler for Objective-C++ against the macOS SDK and documentation links nothing. Expected to build. |
| `ferroui-vello` | Documented with the `hybrid` and `gpu` features for `aarch64-apple-darwin`, the target whose window they draw into. Its dependencies are Rust only. Expected to build. |
| `ferroui-harfbuzz`, `ferroui-headless` | Depend on the HarfBuzz build from source, which needs the C++ compiler of the docs.rs image and no network. Expected to build for the default Linux target. |
| `ferroui-skia`, `ferroui-desktop`, `ferroui-browser`, `ferroui` | Depend on the build of `skia-bindings` (the facade through its default feature `desktop`; `[package.metadata.docs.rs]` adds the features that need nothing beyond the host), which without network cannot download Skia. The Skia bindings are documented on docs.rs themselves, so their build has a way to do without the binaries there; whether it carries through to a dependent crate with the features FerroUI enables is not known. **May fail**; if it does, the remedy is a docs.rs feature set or target for these crates, in the next preview (for the facade: documentation without the feature `desktop`). `ferroui-browser` is documented for the Linux host, not for `wasm32-unknown-emscripten`, which docs.rs cannot build (no Emscripten). |

A failed documentation build does not affect the crate on crates.io: it shows as a failed badge in the README table. Open for the next preview: the documentation build of `ferroui-ios`.

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

The job `packaging` of `ci.yml` runs `publish.sh --dry-run --no-verify` on Linux for every pull request and push that changes more than documentation. It compiles nothing: it installs the toolchain, resolves the dependencies and assembles the packages (37 since 2026-10-15). Locally the script takes about half a minute with a warm registry cache, most of it the resolution of each package; on a runner it is the job "Packages of the published crates" of every CI run (green, last on run 38769434562 of 2026-10-15). It catches a crate without metadata, a dependency that is not pinned, a dependency of a published crate on an unpublished one, a dependency cycle, a package above the size limit, a missing notice and a stale README table. It does not catch a package that no longer builds (a build script that reaches outside its package again): that is the full dry run, which the release workflow runs, and which is worth running by hand after a change of a build script.

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

## 9. What "ready" means: the checklist of a release

Before the release:

- [ ] The CI is green on `main`, including a run of the macOS job on the merged state (dispatch the CI workflow by hand on `main`; it does not run for a push).
- [ ] The tracking pages are current: `docs/porting/CRITICAL-PATH.md` and the generated `docs/porting/TRACKING.md`.
- [ ] The README states the status honestly: what works on which platform, how each was verified (CI, a virtual machine, a simulator or an emulator) and what was not, that the Vello backend is experimental, and whether the release is a preview.
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
- [ ] Set the next version (`scripts/release/set-version.sh 0.1.0-preview.<N+1>` after a preview, the first preview of the next minor version, such as `0.2.0-preview.1`, after a release), so that `main` never carries a version that is on crates.io with other contents.

## 10. A preview that turns out broken

Yank every crate of that version (the owner, with `cargo yank --version <version> <crate>` for each crate of section 1.1) and release the next preview. A yanked version stays downloadable for builds that have it in a lock file. Nothing is ever removed.

## 11. What is not done yet

- Per-crate README files: every package carries the README of the repository.
- The site assembly scripts of the browser platform are not in a package (section 6.1).
- The documentation build of `ferroui-ios` on docs.rs fails (section 7); the crates that were placeholders in the first preview have not been built there yet.

None of these blocks a preview.

## 12. Decisions before the first release

Made by the owner on 2026-10-10, before the first preview was released (on 2026-10-15). They are kept as the record of that release; where a later preview changed the state, the item says so.

1. **Go**: yes. The first publish created 37 crates under the owner's account for good: the 31 crates with code and the six reserved names of section 2.2 (all six are real crates since, section 2.2).
2. **The names `xamlx` and `microcom-codegen`**: kept as they are.
3. **The bare name `ferroui`**: published as an empty placeholder; the facade of section 2.1 replaces it from the second preview on.
4. **The rate limit**: not raised; the first release takes as long as the limit of new crates makes it (section 8.3).
5. **The environment `crates-io`**: as the repository has it; required reviewers were not asked for.
6. **The token**: the repository secret `CARGO_REGISTRY_TOKEN`; trusted publishing can replace it now that the crates exist.
7. **The unfinished platforms**: all are in the first preview (Windows, Linux on X11 with the FreeDesktop services, iOS, Android), with their state in the changelog; Android with the note that its native library needs a nightly toolchain (since 2026-10-15 it builds with stable Rust: `docs/porting/android-platform.md`, section 5.1).
8. **The changelog entry**: `CHANGELOG.md`, the entry of `0.1.0-preview.1`.
