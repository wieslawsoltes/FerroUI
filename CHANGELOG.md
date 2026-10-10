# Changelog

All notable changes to FerroUI are recorded in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). All crates of the workspace are released together at one version; a version with a pre-release identifier (`-preview.N`) is a preview, which may change any part of the public API. The release process is described in [docs/release.md](docs/release.md).

## [Unreleased]

## [0.1.0-preview.1] - Unreleased

The first preview. It is being prepared and has not been published; the date is set when it is.

The preview makes the framework available as crates so that it can be tried outside the repository. It is not a stable release: the public API is not settled, several subsystems are incomplete, and the per-subsystem state is tracked in [docs/porting/CRITICAL-PATH.md](docs/porting/CRITICAL-PATH.md).

### Added

- **Object model and base library** (`ferroui-base`): styled and direct properties with value priorities, inheritance, coercion and change notification; logical and visual trees; routed events; layout; input, focus and gestures; styling, control themes, resources and theme variants; data binding; animation and transitions; the media object model, fonts and text layout; the compositor with its render loop and dirty-region tracking.
- **Controls** (`ferroui-controls`, `ferroui-controls-color-picker`, `ferroui-dialogs`): the application model, top levels and windows, most of the control set including items controls, text input, menus, pickers, calendar, pages and automation peers; the colour picker library; the managed file chooser and the about dialog.
- **Themes** (`ferroui-themes-fluent`, `ferroui-themes-simple`, `ferroui-fonts-inter`): the Fluent and Simple themes for every ported control, in light and dark variants, and the embedded Inter font collection.
- **Markup** (`ferroui-markup`, `ferroui-markup-xaml`, `ferroui-markup-xaml-loader`, `xamlx`, `ferroui-build`, `ferroui-build-scan`): binding paths, the XAML run-time library and markup extensions, the run-time XAML loader, and the compilation of markup documents to Rust source by the build script of a crate.
- **Rendering and text** (`ferroui-skia`, `ferroui-harfbuzz`, `ferroui-opengl`, `ferroui-metal`, `ferroui-vello`): the Skia render backend (Graphite on Metal; Ganesh on OpenGL, on ANGLE and on WebGL; raster), HarfBuzz text shaping, the OpenGL and Metal platform contracts, and the Vello render backend.
- **Platforms** (`ferroui-desktop`, `ferroui-native`, `ferroui-microcom`, `ferroui-win32`, `ferroui-x11`, `ferroui-freedesktop`, `ferroui-ios`, `ferroui-android`, `ferroui-browser`, `ferroui-headless`):
  - macOS: windows, input, clipboard, drag and drop, menus, tray icon, Metal and software surfaces.
  - Windows: windows, input including pointer and touch, clipboard and drag and drop over OLE, icons, the task bar, the tray icon, file dialogs, mounted volumes, platform settings, the native control host, the extended client area; rendering in software, through ANGLE and through DirectComposition.
  - Linux on X11: windows, input, input methods (XIM, IBus, Fcitx), clipboard, drag and drop, screens, the desktop portals (settings, file chooser), GTK file dialogs, menus and the tray icon over D-Bus, session management, the native control host and XEmbed, accessibility over AT-SPI; rendering in software, with the shared memory extension, through GLX and through EGL.
  - iOS: a view over Metal, touch, keys and text input, platform settings, clipboard, storage, the native control host, accessibility.
  - Android: a view over a surface with EGL or software rendering, touch, keys and the input method, lifecycle, clipboard, storage, the native control host, accessibility.
  - The browser platform on `wasm32-unknown-emscripten`, and the headless platform for tests.
- **Tools** (`ferroui-designer-support`, `ferroui-remote-protocol`, `microcom-codegen`): the previewer and its remote protocol, and the code generator of the interop runtime.
- **Reserved names** (`ferroui`, `ferroui-wayland`, `ferroui-vulkan`, `ferroui-linuxframebuffer`, `ferroui-winui`, `ferroui-headless-vnc`): empty placeholder crates for parts that do not exist yet. They hold no code; a later preview replaces each with the real crate.

### Known limitations

- **Platforms.** macOS and the browser are the most complete. Windows, Linux (X11), iOS and Android are new in this preview and less tested: they were verified on the runners of the continuous integration, in virtual machines, in the iOS simulator and on the Android emulator, not on a range of real machines, devices and desktops.
- **Windows.** No input method (IME) support yet: ordinary typing works, composition for languages that need it does not. No UI Automation (accessibility), no WGL, and the blur, acrylic and mica transparency levels are reported as unsupported.
- **Linux.** X11 only: Wayland, the Linux framebuffer and Vulkan are not ported. Accessibility over AT-SPI was verified against the registry of at-spi2-core, not with a screen reader.
- **iOS.** Verified in the simulator only; no signed device build is described. Accessibility was not verified with VoiceOver.
- **Android.** Verified on the emulator (API 36) only. An application for Android cannot be built with stable Rust for now: the native library needs a nightly toolchain with a rebuilt standard library, because the standard library of the stable Android targets takes one thread key per thread-local value and the system has 128 (`docs/porting/android-platform.md`, section 5.1). Accessibility was not verified with TalkBack.
- **API stability.** None. Names, signatures and crate boundaries may change between previews, and the crates of one preview work only with one another (they are pinned at the exact version).
- **Vello backend.** Experimental. Skia is the default backend and the reference; the Vello backend is measured against it and does not yet match it everywhere (see `docs/porting/vello-backend.md`).
- **Compositor.** Composition animations and custom visuals are incomplete.
- **macOS platform.** Storage dialogs and parts of accessibility are incomplete.
- **Markup.** The run-time loader passes the ported test suites with a small number of documented gaps; the compiler of markup to Rust source is used by the themes and the samples.
- **Build requirements.** The Skia backend downloads a prebuilt Skia at build time, or builds it from source when no prebuilt binary matches the target and features, which needs network access and can take a long time. HarfBuzz is built from source and needs a C++ compiler. The macOS platform compiles an Objective-C++ library and needs the Xcode command line tools. The Windows platform needs the MSVC build tools, and `libclang` for its default feature `angle`. The Linux platform opens its system libraries (Xlib, GTK, GLib, the session management libraries) at run time and links none of them. The browser platform needs Rust 1.93 or later, Emscripten and the matching `wasm-bindgen` command-line tool (see `docs/porting/browser-platform.md`).
- **Documentation.** The API documentation is the documentation comments carried over by the port; there are no guides beyond the README and the documents under `docs/`. Not every crate is expected to build on docs.rs (see `docs/release.md`).
- **Tests of the published crates.** The packages hold the sources of the tests but not the test fonts and images of the Skia backend; the test suites are run from the repository.

[Unreleased]: https://github.com/wieslawsoltes/FerroUI/commits/main
[0.1.0-preview.1]: https://github.com/wieslawsoltes/FerroUI/commits/main
