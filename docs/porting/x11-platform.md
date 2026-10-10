# The Linux platform: X11 and the FreeDesktop services

The design of the Linux desktop backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Avalonia.X11` (88 files, 291 types, 4897 members) with `src/Avalonia.FreeDesktop` (18 files, 26 types, 216 members) at the tracked commit (`TRACKING.md`). `Avalonia.FreeDesktop.AtSpi` (27 files, accessibility), `Avalonia.Wayland` (81 files) and `Avalonia.LinuxFramebuffer` (30 files) are later stages, named at the end.

Marks, as in `browser-platform.md`: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`), **[M]** measured here (a build or a test run on this machine), **[R]** recalled and not verified here, **[CI]** to be shown by the CI job on a real X server. **[VM]** measured in a virtual machine on the development machine (Ubuntu 24.04, ARM64, section 13). The development machine is a Mac; the CI job runs on x86-64 and had not run when this was written.

## 1. Crates

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-x11` | `src/FerroUI.X11` | `Avalonia.X11`, and `src/Shared/RawEventGrouping.cs`, which that project compiles in | `AppBuilder::use_x11()` (`FerroX11PlatformExtensions`); `use_platform_detect()` of `ferroui-desktop` on Linux |
| `ferroui-freedesktop` | `src/FerroUI.FreeDesktop` (not created yet) | `Avalonia.FreeDesktop` | used by `ferroui-x11` (and later by the Wayland backend), as upstream's project reference |

`ferroui-x11` depends on `ferroui-base`, `ferroui-controls`, `ferroui-dialogs` (the managed storage provider, the last of the storage providers of a window) and `ferroui-opengl` (`GlVersion` in the options, the GL contracts of stage 2a). Its content is compiled on Unix systems (`cfg(unix)`): the X libraries exist wherever an X server does, and compiling the whole crate on the development machine is what lets its tests run there. The two key tables are plain data and compile everywhere. The crate is empty on Windows.

The example (`examples/x11_window.rs`) needs a renderer, a text shaper and a theme; those are optional dependencies behind the feature `example`, so that the crate and its tests build without the native libraries of Skia and HarfBuzz. That is what makes `cargo check -p ferroui-x11 --target x86_64-unknown-linux-gnu --all-targets` possible from a Mac **[M]**.

## 2. Bindings: Xlib through `x11-dl`

Upstream calls Xlib and its extension libraries through platform invokes (`XLib.cs`, `Glx.cs`, `XIStructs.cs`): `libX11.so.6`, `libXi.so.6`, `libXrandr.so.2`, `libXext.so.6` (XSync and MIT-SHM), `libXcursor.so.1`, `libXfixes.so.3`, and outside the core `libGL.so.1`, `libvulkan.so.1`, `libgtk-3.so.0`, `libgdk-3.so.0`, `libglib-2.0.so.0`, `libgobject-2.0.so.0`, `libSM.so.6`, `libICE.so.6` and `libc` **[V]**. Each library is loaded when one of its functions is first called, and a library that is missing shows as an exception that `X11Info` catches per extension.

The port uses **`x11-dl` 2.21** (MIT, pure Rust, depends on `libc` and `once_cell`) **[V]**:

- It declares the same C functions and structures as upstream's own declarations and opens the libraries with `dlopen` at run time, as the runtime of upstream does. Nothing is linked: its build script only asks `pkg-config` for library directories and goes on without them (`pkg_config::get_variable(..)` failing gives `None`), so the crate checks for a Linux target on a machine without the X libraries **[V][M]**.
- It covers what the backend calls: the core protocol, Xkb, the input method functions and their variadic forms (`XCreateIC`, `XGetIMValues`, `XVaCreateNestedList`), `xinput2`, `xrandr` (with monitors), `xcursor`, `xfixes` (regions and shapes), `sync`, `xshm`, `glx` **[V]**. One structure is missing (`XIMStyles`) and is declared in `xlib.rs`.

The alternatives, and why not:

| Alternative | Why not |
|---|---|
| `x11` (the same declarations, linked) | Needs the X libraries at build time, which a check from another system and a minimal CI image do not have. |
| `x11rb` 0.14 (the protocol in pure Rust, no Xlib) | The behaviour to port is, in several places, the behaviour of Xlib's client side and not of the protocol: the event queue (`XPending`, `XNextEvent`, `XPutBackEvent`), `XFilterEvent` and the input methods (XIM is an Xlib facility), `XLookupString` and `Xutf8LookupString` (compose sequences and the locale), `XkbLookupKeySym` and `XkbTranslateKeySym`, the cursor theme lookup of `libXcursor`, the error handler. Each would have to be re-implemented or replaced by another library with other behaviour (`xkbcommon` for keys). GLX needs a `Display*`, and EGL and Vulkan are created upstream from the Xlib display. A hybrid (Xlib for those, `x11rb` over the same connection through `XGetXCBConnection`) puts two event readers on one socket. |
| Per extension, decided separately | Every extension the backend uses has an Xlib-side library that `x11-dl` declares, and all of them share the display and the event queue; mixing would gain nothing. |

How the port keeps the `unsafe` of a C binding contained: `xlib.rs` is the port of `XLib.cs` and `XLib.Helpers.cs` and the one module that calls the libraries. Every call is a safe function there, with its safety argument; results are copied out of C memory and freed before the function returns (`WindowProperty` for `XGetWindowProperty`, `XIDeviceEventData` for an `XIDeviceEvent`, `MonitorInfo` for `XRRGetMonitors`). A connection is the type `XDisplay`, made only by `x_open_display` and never closed (as upstream), which is what lets calls that take only a connection and identifiers be safe. An event (`XEvent`, a C union of plain data) is read through views (`button_event(&ev)`, `configure_event(&ev)`), sound for any event because every bit pattern is valid for every member. Outside `xlib.rs` there are four places with `unsafe`: the system calls of the event loop (`dispatching/x11_platform_threading.rs`), the error handler Xlib calls (`x_error.rs`), the blit of a framebuffer by address (`x11_framebuffer_surface.rs`), and tests that build C events.

The structures of `X11Structs.cs` and `XIStructs.cs` are therefore not ported: they are those of `x11-dl`. Their enumerations are (`x11_structs.rs`, `x11_enums.rs`, `xi_structs.rs`), converted mechanically from the upstream files: flags as `bitflags`, enumerations with equal members as sets of constants.

## 3. D-Bus: `zbus`

Upstream's FreeDesktop project references `Tmds.DBus.Protocol` and generates its proxies and handlers from the interface descriptions in `DBusXml/` with `Tmds.DBus.Generator` **[V]**. The interfaces: `org.freedesktop.portal.FileChooser`, `.Request` and `.Settings`, `org.kde.StatusNotifierWatcher` and `.StatusNotifierItem`, `com.canonical.dbusmenu` and `com.canonical.AppMenu.Registrar`, `org.freedesktop.DBus`, `org.freedesktop.IBus.Portal` (with `InputContext` and `Service`), and the four Fcitx interfaces **[V]**.

The port will use **`zbus` 5.x** (5.19.0 at the time of writing; MIT; pure Rust; minimum Rust 1.87, the workspace is at 1.89) **[V]**:

- Pure Rust, so the FreeDesktop crate checks for Linux from another system like the X11 crate does, and needs no `libdbus` on the machine that runs the application.
- Proxies and interfaces are written as traits and `impl` blocks with attribute macros (`#[proxy]`, `#[interface]`), the counterpart of upstream's generated code; the descriptions in `DBusXml/` are the specification they are written from, kept under `docs/porting/` data or the crate as reference.
- Runtime: its default feature set (`async-io`, `blocking-api`) runs the socket on an executor of its own (a background thread per connection, `internal_executor`), and every call is a future that can be awaited on any executor **[V]** (features from `cargo info zbus`). The futures are awaited on the dispatcher of the UI thread (`Dispatcher::invoke_async_task_local`): a completion wakes the dispatcher through its signal handle (the wake-up pipe of section 5), so handlers run on the UI thread like upstream's continuations, which resume on the synchronization context. The `tokio` feature stays off: the port has no Tokio runtime.
- Signals (`SettingChanged`, `Response`, `NameOwnerChanged`, the input method signals) are streams; each subscription is a task of the UI dispatcher.

| Alternative | Why not |
|---|---|
| `dbus` 0.9 | Binds `libdbus-1` (C): a build and run-time dependency on the system library, and no check from another system. |
| `rustbus` 0.19 | Pure Rust but synchronous only, with no proxy or interface macros: the message loop and every interface would be written by hand. |
| Port upstream's D-Bus stack | `Tmds.DBus` is a third-party library, not part of the framework. |

Upstream's `DBusCallQueue` (one call in flight, in order) is ported as it is, over `zbus` calls: it orders the calls of an input method context, which `zbus` does not do by itself.

## 4. Skia on Linux

- The Skia backend of the port builds with the features of its target (`src/Skia/FerroUI.Skia/Cargo.toml`): Graphite on Metal for Apple targets, Ganesh on WebGL for the browser, and **no GPU feature for Linux**, so a Linux build has the raster back end only **[V]**. That is what stage 1 renders with: Skia draws into memory and the X11 platform sends the pixels to the server.
- `skia-bindings` 0.153.3 has, for Linux, the features `gl` (with `egl`, `x11`, `wayland` selecting the window system libraries it links: `EGL`, `GL`, `wayland-egl`), `vulkan`, and `graphite` as an engine feature; a Linux build links `freetype2` and `fontconfig` found through `pkg-config`, and `stdc++` **[V]** (`build_support/platform/linux.rs`, `features.rs` of the crate in the cargo registry).
- Which feature sets have a **published binary** for `x86_64-unknown-linux-gnu` cannot be read from the crate: the key of a binary is built from the feature list, but the list of published keys is in the workflow files of the rust-skia repository. **[R]**: binaries are published for Linux without a GPU feature, with `gl`, with `vulkan`, and with combinations including `gl` with `x11`, `egl` and `wayland`; whether a Graphite binary with Vulkan exists is not known. A combination without a binary makes the build fall back to compiling Skia from source (`browser-platform.md`, section 3), which must not happen: the CI job states the feature set it expects and fails otherwise, as the browser job does.
- Consequence for the rendering modes: EGL and GLX (stage 2a) need Ganesh on GL (`skia-safe` with `gl`, plus `egl` and `x11`), which is the GPU of the Skia backend the browser already uses (`gpu/open_gl`, behind the configuration `ferro_skia_ganesh_gl`). Vulkan needs `vulkan` and the Vulkan GPU of the Skia backend, which is not ported (`CRITICAL-PATH.md`, row 16). One Linux binary has to carry every mode the platform may choose at run time, so the feature set for Linux is decided once, in stage 2a, after checking which combination is published.

## 5. The event loop

`X11PlatformThreading` (`dispatching/x11_platform_threading.rs`) is the dispatcher implementation (`IControlledDispatcherImpl`, with pending input), a loop on the UI thread:

1. Raise the timer when the next timer is due.
2. Flush the requests to the server. When no event is pending in Xlib's queue: wait, with `epoll_wait`, on the descriptor of the connection and on the reading end of a pipe, until the next timer (at least one millisecond; without a timer, without limit); then drain the pipe and clear the wake-up flag.
3. Raise `signaled` when another thread signalled (a flag under a lock; `signal` writes one byte to the pipe unless a wake-up is already requested).
4. Dispatch the events of the connection (`X11EventDispatcher`): `XNextEvent`, `XFilterEvent` (input methods), the data of a generic event, the event hook (the drag source of stage 2d), then the X Input manager for generic events of its extension or the handler registered for the window of the event (`FerroX11Platform::set_window`).
5. Dispatch the raw input the handlers queued (`ManualRawEventGrouperDispatchQueue`), checking the signal between events.

The part other threads reach (the flags and the writing end of the pipe) is the `IDispatcherSignal` handle of the dispatcher implementation. The clock is a monotonic one started with the loop.

Upstream runs on Linux only. On other Unix systems (the development machine, the BSDs) the same two descriptors are waited on with `poll`; this is an addition (DEVIATIONS.md).

`GlibDispatcherImpl` (the option `use_g_lib_main_loop`, for applications that use GLib libraries on the main thread) is stage 2e, with the GTK dialogs that need GLib anyway. Until then the option fails with a message.

## 6. Threads and rendering

The platform opens two connections, as upstream: `Display` for the UI thread and `DeferredDisplay` for rendering. `XInitThreads` is called first, so Xlib locks each connection around a call.

The render timer is `SleepLoopRenderTimer` (60 frames a second, then the highest refresh rate of the screens, updated when they change) unless `should_render_on_ui_thread` asks for `UiThreadRenderTimer`. With the sleep loop the render loop runs in the background, so the compositor renders on that thread (`render-thread.md`, "The render thread is the default on the desktop"), and the platform creates the compositor without the option of synchronous commits on the UI thread, as upstream's `new Compositor(graphics)`.

What crosses to the render thread is `Send + Sync` by construction:

- `X11FramebufferSurface` holds the deferred connection, the identifier of the window and its depth. The framebuffer upstream keeps in the surface is kept by the render target (made on the thread that renders), a representation difference recorded in DEVIATIONS.md.
- The scaling and the size of a window, which upstream reads from the window with interlocked operations, are atomics shared between the window and its surfaces (`WindowShared`).

### Rendering modes, in the order they are built

| Mode | Upstream | Stage | Notes |
|---|---|---|---|
| Software, `XPutImage` | `X11FramebufferSurface.cs` | 1, built | A retained framebuffer (BGRA, premultiplied) of the size of the window (`XGetGeometry`), sent with `XPutImage` under `XLockDisplay`, followed by `XSync`. Kept between frames only with `use_retained_framebuffer`. The window has the visual of depth 32 when the screen has one, else depth 24. |
| Software, shared memory | `XShm/*`, `X11DeferredDisplayDispatcher.cs`, `LibC.cs` | 2b | `XShmCreateImage` over a System V segment, `XShmPutImage` with a completion event, at most two frames in flight, images pooled by size. Only with `use_x_shm_framebuffer`, depth 32 and the extension. Until built, the option fails with a message. |
| EGL | `X11EglHelper.cs`, `EglPlatformGraphics` of the OpenGL project (ported) | 2a | The display through `EGL_PLATFORM_X11` when the extension is there; configurations probed by creating a window surface (an nvidia workaround); a child window as the render window. Needs Skia with Ganesh on GL. |
| GLX | `Glx/*` | 2a | `libGL` through `glXGetProcAddress`; a frame buffer configuration with a preference for depth 32; a pixel buffer as the default drawable of a context; the renderer blacklist (`llvmpipe`, `SVGA3D`, overridden by an environment variable). Upstream's default mode list is GLX, then software. |
| Vulkan | `Vulkan/*` | 2a, after the Vulkan GPU of the Skia backend | `VK_KHR_xlib_surface` over the window. |

`FerroX11Platform::initialize_graphics` walks `rendering_mode` as upstream. Until stage 2a the three GPU modes count as modes that failed to initialize, so the default list (GLX, software) gives software; an empty list, or a list without a mode that applies, fails with upstream's messages.

## 7. Windows

`X11Window` (`x11_window.rs`) is a window, a popup (override-redirect, with the managed popup positioner) and a top-level in one class, as upstream. What stage 1 builds:

- **Creation.** The visual and colormap (depth 32 when available), backing store and gravity attributes, the default size (three quarters by seven tenths of the working area of the screen at the origin, at least 300 by 200), `_NET_WM_PID`, `WM_CLIENT_MACHINE`, `WM_PROTOCOLS` (`WM_DELETE_WINDOW`, `_NET_WM_SYNC_REQUEST`, and `WM_TAKE_FOCUS` with the focus proxy), `_NET_WM_WINDOW_TYPE`, `WM_CLASS` (from `RESOURCE_NAME` or the name of the process, and the class of the options), the event mask (everything but the redirect masks, and without the pointer masks when the X Input extension delivers pointer input), the input context.
- **Events.** Map and unmap; expose and visibility (a paint posted at render priority); configure (size and position applied in one deferred step, the position translated to the root; scaling re-evaluated when the position changed); property changes (`_NET_FRAME_EXTENTS`, `_NET_WM_STATE` to the window state and to activation); `WM_DELETE_WINDOW` (the closing callback may refuse); `_NET_WM_SYNC_REQUEST` (the counter of the XSync extension is set after the paint that follows the configure); destroy; core pointer and key events.
- **State and hints.** Window states through `_NET_WM_STATE` (the property while unmapped, the client message always) and `XIconifyWindow`; Motif hints for decorations and functions, recomputed for resizable, minimizable, maximizable and disabled; size hints (minimum, maximum when small enough, position flags once the window was placed, pinned when it cannot be resized); `_NET_WM_STATE_ABOVE` and `_NET_WM_STATE_SKIP_TASKBAR`; `WM_HINTS` input for a disabled window; the transient parent; the title in `_NET_WM_NAME` and `WM_NAME`; the icon in `_NET_WM_ICON`; frame extents for the frame size and the position; moves and resizes started by the window manager (`_NET_WM_MOVERESIZE`), also from drawn decorations when the experimental options enable them; the input shape for popups that are not hit-test visible (XFixes).
- **Activation.** Three modes chosen by `X11Globals` from what the window manager supports: `_NET_WM_STATE_FOCUSED`, the root's `_NET_ACTIVE_WINDOW`, or focus events (`WindowActivationTrackingHelper`, `X11ActiveWindowTracker`), with the speculative activation of the owner when a dialog closes.
- **Transparency.** None, transparent with a compositing manager, blur with KWin (`TransparencyHelper`).
- **Modes.** `DefaultTopLevelWindowMode` and `InputProxyWindowMode` (a focus proxy child window, option `enable_input_focus_proxy`); `XEmbedClientWindowMode` is stage 2f.
- **Features.** The storage provider (the managed dialogs; the portal and GTK come first in stage 2e), the clipboard and its manager, the launcher, the X11 options of a top-level (`_NET_WM_WINDOW_TYPE`, `WM_CLASS`), the screens.

## 8. Screens and scaling

`X11Screens` (`screens/`) is a `ScreensBase` keyed by the name atom of a monitor. With RandR 1.5 the monitors come from `XRRGetMonitors`, on a helper window that selects RandR events; without it there is one screen of the size of the root window. The working area is `_NET_WORKAREA` intersected with the monitor; the physical size comes from the EDID property of an output when the monitor reports none; the refresh rate from the mode of the controller of each output.

Scaling, in upstream's order: the environment (a global factor and per-screen factors; this framework's variables first, Qt's unless told to ignore them), then `Xft.dpi` of the resource database (`XResources`, the `RESOURCE_MANAGER` property of the root, re-read when it changes), then the physical size of the monitor. The variables of the framework are renamed (`FERROUI_GLOBAL_SCALE_FACTOR`, `FERROUI_SCREEN_SCALE_FACTORS`, `FERROUI_USE_PHYSICAL_DPI`, `FERROUI_SCREEN_SCALE_IGNORE_QT`); DEVIATIONS.md lists every renamed name.

Several parties subscribe to the change of the screens upstream (`Changed +=`); the contract of the port has one slot, which the controls layer takes, so `X11Screens` has an event of its own (`changed_event`) beside it.

## 9. Input

- **Pointer, touch, pen, scroll: the X Input extension, version 2** (`xi2_manager.rs`). The master pointer's motion, button, enter and leave events, and touch events unless `enable_multi_touch` is off, are selected per window; device changes on the root. Scroll valuators give smooth wheel deltas (the difference to the previous value over the increment, the first value after a reset or a leave only remembered); buttons 4 to 7 give wheel steps unless the server emulated them from the valuators; pressure and tilt valuators make the device a pen, and a slave device named "eraser" sets the eraser modifier; touch contacts carry pressure and a contact rectangle from the major and minor axes scaled to the screen. The translation is a function over copied event data (`translate_device_event`), which is what the tests drive.
- **Core pointer events** are handled as upstream when the extension is missing.
- **Keyboard** (`x11_window_ime.rs`, `x11_key_transform.rs`, `keysyms.rs`). The physical key from the key code (the scan code table); the key from the key symbol of the current keyboard group (`XkbLookupKeySym`), trying the other groups when the symbol maps to no key (a Latin key for a non-Latin layout), digits always from the physical key, and a QWERTY fallback; without Xkb, `XLookupString`. The key symbol text through `XkbTranslateKeySym`; the text of a key press through the input context (`Xutf8LookupString`), without control characters. Detectable auto-repeat is set once.
- **Input methods** (stage 2c): the queue that lets an input method filter key events (`FilterIme`), the input method of the server (`XimInputMethod`: focus, reset, spot location), and the D-Bus input methods of the FreeDesktop crate (IBus through its portal, Fcitx 4 and 5; chosen from this framework's module variable, `GTK_IM_MODULE`, `QT_IM_MODULE`, then `XMODIFIERS`). Until then keys produce text through the keyboard mapping alone, the platform never asks for the input method of the server, and it logs a warning when one is configured.
- Raw input is queued and grouped (`raw_event_grouping.rs`): consecutive moves of one device with the same modifiers become one event with intermediate points.

## 10. Clipboard and selections

`selections/` is the ICCCM selection protocol, shared by the clipboard (stage 1) and drag and drop (stage 2d):

- **Owning** (`SelectionDataProvider`): `TARGETS` (with `MULTIPLE`), `SAVE_TARGETS` for the clipboard manager at exit, `MULTIPLE` requests as pairs, and data; a value larger than the maximum property size (from the maximum request size of the connection) is sent incrementally (`INCR`: announce the length, then one part per deletion of the property by the requestor, ended by an empty part), with a timeout of five seconds per step.
- **Reading** (`SelectionReadSession`): `XConvertSelection` to a property on a window of its own (`EventStreamWindow`), the `SelectionNotify`, the property, and `INCR` reads until an empty part.
- **Formats** (`DataFormatHelper`): text as `UTF16_STRING`, `UTF8_STRING`, `text/plain`, `text/plain;charset=utf-8`, `STRING` in that order of preference; files as `text/uri-list`; bitmaps as `image/png` (and `image/jpeg` when reading); application formats under a prefix.
- The clipboard and the primary selection are two instances of `X11ClipboardImpl`.

The protocol steps are state machines over a small connection trait (`ISelectionConnection`), which the tests drive with a mock; the asynchronous waits are futures on the UI dispatcher.

Drag and drop (stage 2d) is XDND version 5 (minimum 3): `X11DropTarget` per window (`XdndAware`, enter, position, status, drop, finished) and `X11DragSource` as an event hook of the dispatcher during a drag (target lookup through the window tree and `XdndProxy`, in-process windows without the protocol, a timeout of five seconds, cursors from the cursor theme).

## 11. FreeDesktop services (stage 2e)

- **Storage provider.** A window offers a fallback chain, as upstream: the file chooser portal (`DBusSystemDialog`, `org.freedesktop.portal.FileChooser` on `org.freedesktop.portal.Desktop`, version checked; the parent window as `x11:<hex id>`; the response awaited on the request object, subscribed before the call), then the GTK dialogs (`NativeDialogs/Gtk*`: `libgtk-3`, a thread running the GTK main loop unless the GLib dispatcher is used, the parent set through a foreign GDK window), then the managed dialogs. Stage 1 has the last link.
- **Tray icon** (`DBusTrayIconImpl`): a `StatusNotifierItem` on a connection of its own, registered with `org.kde.StatusNotifierWatcher`, with the menu exported by `DBusMenuExporter`. Upstream's fallback (`XEmbedTrayIconImpl`) only logs. Until built, the platform has no tray icon.
- **Global menu** (`DBusMenuExporter`): `com.canonical.dbusmenu`, registered per window with `com.canonical.AppMenu.Registrar` (option `use_d_bus_menu`).
- **Platform settings** (`DBusPlatformSettings`): colour scheme and accent colour from `org.freedesktop.portal.Settings` (`org.freedesktop.appearance`), with change notifications. Until built, the defaults of the framework.
- **Mounted volumes** (`LinuxMountedVolumeInfoProvider`): `/proc/partitions`, `/proc/mounts` and `/dev/disk/by-label`, polled every second.

Stage 2f, the rest of the X11 project: session management (`X11PlatformLifetimeEvents` over `libSM` and `libICE`: the shutdown request of the session manager), the native control host (`X11NativeControlHost`), XEmbed (`XEmbedPlug`, `XEmbedClientWindowMode`).

## 12. File table

One row per upstream file. "built" files are on the branch; the stage of an open file is the one that builds it.

| Upstream file (`src/Avalonia.X11`) | Lines | Rust file (`src/FerroUI.X11`) | Stage | State | Notes |
|---|---:|---|---|---|---|
| `ActivityTrackingHelper.cs` | 115 | `activity_tracking_helper.rs` | 1 | built |  |
| `ICELib.cs` | 60 | `ice_lib.rs` | 2f | open |  |
| `Keysyms.cs` | 2108 | `keysyms.rs` | 1 | built | a set of constants: the enumeration has members with equal values |
| `LibC.cs` | 34 | `lib_c.rs` | 2b | open | the shared memory calls, with the shared memory framebuffer |
| `SMLib.cs` | 132 | `sm_lib.rs` | 2f | open |  |
| `TransparencyHelper.cs` | 112 | `transparency_helper.rs` | 1 | built |  |
| `X11ActiveWindowTracker.cs` | 55 | `x11_active_window_tracker.rs` | 1 | built |  |
| `X11Atoms.cs` | 275 | `x11_atoms.rs` | 1 | built |  |
| `X11AtSpiAccessibility.cs` | 176 | `x11_at_spi_accessibility.rs` | 3 | open |  |
| `X11CursorFactory.cs` | 182 | `x11_cursor_factory.rs` | 1 | built |  |
| `X11DeferredDisplayDispatcher.cs` | 98 | `x11_deferred_display_dispatcher.rs` | 2b | open |  |
| `X11EglHelper.cs` | 97 | `x11_egl_helper.rs` | 2a | open |  |
| `X11EnumExtensions.cs` | 30 | `x11_enum_extensions.rs` | 1 | built |  |
| `X11Enums.cs` | 121 | `x11_enums.rs` | 1 | built |  |
| `X11Exception.cs` | 12 | `x11_exception.rs` | 1 | built |  |
| `X11FocusProxy.cs` | 90 | `x11_focus_proxy.rs` | 1 | built |  |
| `X11FramebufferSurface.cs` | 73 | `x11_framebuffer_surface.rs` | 1 | built |  |
| `X11Globals.cs` | 261 | `x11_globals.rs` | 1 | built |  |
| `X11IconLoader.cs` | 88 | `x11_icon_loader.rs` | 1 | built |  |
| `X11Info.cs` | 171 | `x11_info.rs` | 1 | built |  |
| `X11KeyTransform.cs` | 428 | `x11_key_transform.rs` | 1 | built |  |
| `X11NativeControlHost.cs` | 202 | `x11_native_control_host.rs` | 2f | open |  |
| `X11Platform.cs` | 612 | `x11_platform.rs` | 1 | built |  |
| `X11PlatformLifetimeEvents.cs` | 274 | `x11_platform_lifetime_events.rs` | 2f | open |  |
| `X11Structs.cs` | 2027 | `x11_structs.rs` | 1 | built | enumerations and Motif hints; structures are those of `x11-dl` |
| `X11Window.cs` | 1818 | `x11_window.rs` | 1 | built |  |
| `X11Window.Ime.cs` | 377 | `x11_window_ime.rs` | 1 | built | keyboard part built; the input method queue is stage 2c |
| `X11Window.Xim.cs` | 126 | `x11_window_xim.rs` | 2c | open |  |
| `X11WindowInfo.cs` | 7 | `x11_window_info.rs` | 1 | built |  |
| `XEmbedPlug.cs` | 84 | `x_embed_plug.rs` | 2f | open |  |
| `XEmbedTrayIconImpl.cs` | 47 | `x_embed_tray_icon_impl.rs` | 2f | open |  |
| `XError.cs` | 31 | `x_error.rs` | 1 | built |  |
| `XI2Manager.cs` | 612 | `xi2_manager.rs` | 1 | built |  |
| `XIStructs.cs` | 337 | `xi_structs.rs` | 1 | built | enumerations; structures are those of `x11-dl` and the copies of `xlib.rs` |
| `XLib.cs` | 866 | `xlib.rs` | 1 | built | the calls into Xlib through `x11-dl`; one safe function per call |
| `XLib.Helpers.cs` | 66 | `xlib.rs` | 1 | built | merged |
| `XResources.cs` | 75 | `x_resources.rs` | 1 | built |  |
| `Dispatching/GLibDispatcherImpl.cs` | 60 | `dispatching/g_lib_dispatcher_impl.rs` | 2e | open |  |
| `Dispatching/GlibDispatcherImplBase.cs` | 289 | `dispatching/glib_dispatcher_impl_base.rs` | 2e | open |  |
| `Dispatching/IX11PlatformDispatcher.cs` | 8 | `dispatching/i_x11_platform_dispatcher.rs` | 1 | built |  |
| `Dispatching/X11EventDispatcher.cs` | 72 | `dispatching/x11_event_dispatcher.rs` | 1 | built |  |
| `Dispatching/X11PlatformThreading.cs` | 206 | `dispatching/x11_platform_threading.rs` | 1 | built |  |
| `Glx/Glx.cs` | 126 | `glx/glx.rs` | 2a | open |  |
| `Glx/GlxConsts.cs` | 108 | `glx/glx_consts.rs` | 2a | open |  |
| `Glx/GlxContext.cs` | 139 | `glx/glx_context.rs` | 2a | open |  |
| `Glx/GlxDisplay.cs` | 195 | `glx/glx_display.rs` | 2a | open |  |
| `Glx/GlxGlPlatformSurface.cs` | 102 | `glx/glx_gl_platform_surface.rs` | 2a | open |  |
| `Glx/GlxPlatformFeature.cs` | 38 | `glx/glx_platform_feature.rs` | 2a | open |  |
| `Interop/Glib.cs` | 192 | `interop/glib.rs` | 2e | open |  |
| `Interop/GtkInteropHelper.cs` | 15 | `interop/gtk_interop_helper.rs` | 2e | open |  |
| `NativeDialogs/Gtk.cs` | 243 | `native_dialogs/gtk.rs` | 2e | open |  |
| `NativeDialogs/GtkNativeFileDialogs.cs` | 279 | `native_dialogs/gtk_native_file_dialogs.rs` | 2e | open |  |
| `Screens/X11Screen.Providers.cs` | 389 | `screens/x11_screen_providers.rs` | 1 | built |  |
| `Screens/X11Screens.cs` | 40 | `screens/x11_screens.rs` | 1 | built |  |
| `Screens/X11Screens.Scaling.cs` | 248 | `screens/x11_screens_scaling.rs` | 1 | built |  |
| `Selections/Clipboard/ClipboardDataReader.cs` | 40 | `selections/clipboard/clipboard_data_reader.rs` | 1 | built |  |
| `Selections/Clipboard/ClipboardDataTransfer.cs` | 30 | `selections/clipboard/clipboard_data_transfer.rs` | 1 | built |  |
| `Selections/Clipboard/ClipboardDataTransferItem.cs` | 20 | `selections/clipboard/clipboard_data_transfer_item.rs` | 1 | built |  |
| `Selections/Clipboard/ClipboardReadSessionFactory.cs` | 20 | `selections/clipboard/clipboard_read_session_factory.rs` | 1 | built |  |
| `Selections/Clipboard/EventStreamWindow.cs` | 103 | `selections/clipboard/event_stream_window.rs` | 1 | built |  |
| `Selections/Clipboard/X11ClipboardImpl.cs` | 146 | `selections/clipboard/x11_clipboard_impl.rs` | 1 | built |  |
| `Selections/DataFormatHelper.cs` | 177 | `selections/data_format_helper.rs` | 1 | built |  |
| `Selections/DragDrop/DragDropDataProvider.cs` | 30 | `selections/drag_drop/drag_drop_data_provider.rs` | 2d | open |  |
| `Selections/DragDrop/DragDropDataReader.cs` | 49 | `selections/drag_drop/drag_drop_data_reader.rs` | 2d | open |  |
| `Selections/DragDrop/DragDropDataTransfer.cs` | 40 | `selections/drag_drop/drag_drop_data_transfer.rs` | 2d | open |  |
| `Selections/DragDrop/DragDropDataTransferItem.cs` | 31 | `selections/drag_drop/drag_drop_data_transfer_item.rs` | 2d | open |  |
| `Selections/DragDrop/DragDropTimeoutManager.cs` | 41 | `selections/drag_drop/drag_drop_timeout_manager.rs` | 2d | open |  |
| `Selections/DragDrop/IXdndWindow.cs` | 13 | `selections/drag_drop/i_xdnd_window.rs` | 2d | open |  |
| `Selections/DragDrop/SynchronousXEventWaiter.cs` | 124 | `selections/drag_drop/synchronous_x_event_waiter.rs` | 2d | open |  |
| `Selections/DragDrop/X11DragSource.cs` | 766 | `selections/drag_drop/x11_drag_source.rs` | 2d | open |  |
| `Selections/DragDrop/X11DropTarget.cs` | 204 | `selections/drag_drop/x11_drop_target.rs` | 2d | open |  |
| `Selections/DragDrop/XdndActionHelper.cs` | 29 | `selections/drag_drop/xdnd_action_helper.rs` | 2d | open |  |
| `Selections/DragDrop/XdndConstants.cs` | 8 | `selections/drag_drop/xdnd_constants.rs` | 2d | open |  |
| `Selections/IXEventWaiter.cs` | 9 | `selections/i_x_event_waiter.rs` | 1 | built |  |
| `Selections/SelectionDataProvider.cs` | 261 | `selections/selection_data_provider.rs` | 1 | built |  |
| `Selections/SelectionDataReader.cs` | 108 | `selections/selection_data_reader.rs` | 1 | built |  |
| `Selections/SelectionHelper.cs` | 8 | `selections/selection_helper.rs` | 1 | built |  |
| `Selections/SelectionReadSession.cs` | 150 | `selections/selection_read_session.rs` | 1 | built |  |
| `Selections/UriListHelper.cs` | 54 | `selections/uri_list_helper.rs` | 1 | built |  |
| `Vulkan/VulkanNativeInterop.cs` | 29 | `vulkan/vulkan_native_interop.rs` | 2a | open |  |
| `Vulkan/VulkanSupport.cs` | 95 | `vulkan/vulkan_support.rs` | 2a | open |  |
| `X11WindowModes/DefaultWindowMode.cs` | 87 | `x11_window_modes/default_window_mode.rs` | 1 | built |  |
| `X11WindowModes/InputProxyWindowMode.cs` | 55 | `x11_window_modes/input_proxy_window_mode.rs` | 1 | built |  |
| `X11WindowModes/WindowMode.cs` | 61 | `x11_window_modes/window_mode.rs` | 1 | built |  |
| `X11WindowModes/XEmbedClientWindowMode.cs` | 208 | `x11_window_modes/x_embed_client_window_mode.rs` | 2f | open |  |
| `XShm/X11ShmFramebufferRenderTarget.cs` | 176 | `x_shm/x11_shm_framebuffer_render_target.rs` | 2b | open |  |
| `XShm/X11ShmFramebufferSurface.cs` | 31 | `x_shm/x11_shm_framebuffer_surface.rs` | 2b | open |  |
| `XShm/X11ShmImage.cs` | 105 | `x_shm/x11_shm_image.rs` | 2b | open |  |

| Upstream file (`src/Avalonia.FreeDesktop`) | Lines | Rust file (`src/FerroUI.FreeDesktop`) | Stage | State | Notes |
|---|---:|---|---|---|---|
| `DBusCallQueue.cs` | 105 | `dbus_call_queue.rs` | 2e | open | one call in flight, in order |
| `DBusHelper.cs` | 57 | `dbus_helper.rs` | 2e | open | the session bus connection |
| `DBusMenuExporter.cs` | 355 | `dbus_menu_exporter.rs` | 2e | open | `com.canonical.dbusmenu`, `com.canonical.AppMenu.Registrar` |
| `DBusPlatformSettings.cs` | 148 | `dbus_platform_settings.rs` | 2e | open | `org.freedesktop.portal.Settings`: colour scheme, accent colour |
| `DBusSystemDialog.cs` | 294 | `dbus_system_dialog.rs` | 2e | open | `org.freedesktop.portal.FileChooser` |
| `DBusTrayIconImpl.cs` | 444 | `dbus_tray_icon_impl.rs` | 2e | open | `org.kde.StatusNotifierItem` |
| `IPortalParentLease.cs` | 29 | `i_portal_parent_lease.rs` | 2e | open |  |
| `IX11InputMethod.cs` | 34 | `ix11_input_method.rs` | 2c | open | with the input methods (stage 2c) |
| `LinuxMountedVolumeInfoListener.cs` | 103 | `linux_mounted_volume_info_listener.rs` | 2e | open |  |
| `LinuxMountedVolumeInfoProvider.cs` | 15 | `linux_mounted_volume_info_provider.rs` | 2e | open |  |
| `NativeMethods.cs` | 40 | `native_methods.rs` | 2e | open | `readlink`: `std::fs::read_link` |
| `DBusIme/DBusTextInputMethodBase.cs` | 358 | `dbus_ime/dbus_text_input_method_base.rs` | 2c | open |  |
| `DBusIme/Fcitx/FcitxEnums.cs` | 67 | `dbus_ime/fcitx/fcitx_enums.rs` | 2c | open |  |
| `DBusIme/Fcitx/FcitxICWrapper.cs` | 60 | `dbus_ime/fcitx/fcitx_ic_wrapper.rs` | 2c | open |  |
| `DBusIme/Fcitx/FcitxX11TextInputMethod.cs` | 200 | `dbus_ime/fcitx/fcitx_x11_text_input_method.rs` | 2c | open |  |
| `DBusIme/IBus/IBusEnums.cs` | 45 | `dbus_ime/ibus/ibus_enums.rs` | 2c | open |  |
| `DBusIme/IBus/IBusX11TextInputMethod.cs` | 185 | `dbus_ime/ibus/ibus_x11_text_input_method.rs` | 2c | open |  |
| `DBusIme/X11DBusImeHelper.cs` | 65 | `dbus_ime/x11_dbus_ime_helper.rs` | 2c | open |  |

`RawEventGrouping.cs` of `src/Shared` is `raw_event_grouping.rs` (built). Two files of the port have no upstream file: `event.rs` (the multicast events of the backend's own classes) and `pixel_buffer.rs` (a framebuffer over pixels the crate owns).

## 13. Verification

Three levels, because an X application cannot run on the development machine:

1. **Compilation for Linux**: `cargo check -p ferroui-x11 --target x86_64-unknown-linux-gnu --all-targets` **[M]**, and the host: the crate compiles whole on macOS.
2. **Tests without a server** (`cargo test -p ferroui-x11`, 163 tests **[M][VM]**), of everything that is logic and not a round trip: the key tables; atoms and property decoding; Motif, size and state hints, window state from `_NET_WM_STATE`, allowed actions, frame extents; the translation of X Input events (built as the C structures the library delivers, copied as the dispatcher copies them); scaling from `Xft.dpi`, the environment and physical sizes, EDID, refresh rates, working areas; the resource database; the selection protocol with a mock connection (targets, multiple, incremental transfers in both directions, timeouts), formats and encodings, uri lists; the wake-up pipe and the timer wait of the event loop; the cursor tables; icon data; the options and the input method decisions.
3. **A real server in CI** (the job `x11` of `.github/workflows/ci.yml`, Ubuntu, Xvfb): the crate is built for Linux, its tests run on Linux, and `examples/x11_window.rs --smoke` runs under `xvfb-run`. The smoke mode asks the server, not the framework: the window is viewable, has the expected size, `WM_NAME` and `_NET_WM_NAME`, `_NET_WM_PID`, `WM_PROTOCOLS`, `WM_CLASS`, `_NET_WM_WINDOW_TYPE`, and pixels read back with `XGetImage` at three points have the fill colour; it then sends itself `WM_DELETE_WINDOW`, which has to close the window and end the application. Every check prints a line and the exit code is the result.

The job runs the smoke mode twice: on a bare Xvfb, and under a window manager (`openbox`) on a screen narrower than the window, where the window manager clamps the window and the framework has to follow (the size check compares the server with the framework, and with the requested size only without a window manager).

### Measured in the virtual machine (2026-10-10) **[VM]**

Ubuntu 24.04.3 on ARM64 (`aarch64-unknown-linux-gnu`, Rust 1.97.1), the sources and the build directory on a shared folder, one package installed for the build (`libfontconfig1-dev`):

| What | Result |
|---|---|
| `cargo test -p ferroui-x11` | 163 passed, as on macOS |
| `cargo build -p ferroui-x11 --features example --example x11_window` | Builds in under four minutes: `skia-bindings` 0.153.3 has a published binary for `aarch64-unknown-linux-gnu` with the default features (no source build), and the bundled HarfBuzz compiles |
| The smoke mode under `xvfb-run` (no window manager), 1280x1024x24 | All checks pass at the first attempt: viewable, 640x400, depth 32, `WM_NAME`, `_NET_WM_NAME`, `_NET_WM_PID`, `WM_PROTOCOLS`, `WM_CLASS`, `_NET_WM_WINDOW_TYPE`, three pixels `0xff336699`, closed by `WM_DELETE_WINDOW`, exit code 0 |
| The same under `openbox` in Xvfb, 1280x1024 and 600x500 | Pass; on the small screen the window is 598x400 on the server and in the framework |
| The same once on the XWayland display of the desktop session (GNOME, mutter, `Xft.dpi` 192) | Scaling 2 from `Xft.dpi`; the window mapped, titled, drawn and closed. Two checks failed that were wrong in the example, not in the platform: the window manager had clamped the 1280 pixel wide window to the 1148 pixel work area, and the example expected the requested size and read a pixel beyond the window. The example was corrected and the corrected checks run under `openbox` (above); the run on the session display was not repeated |
| `cargo test -p ferroui-desktop --lib` | `platform_detect_selects_the_linux_subsystems` passes |
| `hello_window` of `ferroui-desktop` under `xvfb-run` (`use_platform_detect`) | Opens, closes after its timer, exit code 0 |

Nothing had to be changed in the platform for these runs. Only in CI so far: the build for x86-64 (whether `skia-bindings` has a binary for that target with the default features is **[R]**). Verified nowhere yet: input from real devices or synthetic input (`xdotool`), the clipboard between two clients, popups, more than one screen, a compositing manager under plain X11.

## 14. Stages

| Stage | Content | What its CI run proves |
|---|---|---|
| 1 (built) | The crate and bindings; platform initialisation and options; atoms; the event loop; windows (creation, events, states, hints, activation, transparency, popups); screens with RandR and scaling; cursors; the software framebuffer; the render timer; pointer, touch and keyboard input; the clipboard; `use_x11` and `use_platform_detect` on Linux; the example | Section 13, level 3 |
| 2a | GPU rendering: EGL, GLX, then Vulkan; the Skia feature set for Linux | The smoke run per mode on Mesa's software GL (`llvmpipe` needs the blacklist override), pixels read back |
| 2b | The shared memory framebuffer | The smoke run with `use_x_shm_framebuffer` |
| 2c | Input methods: the key event queue, XIM, IBus and Fcitx over D-Bus (starts `ferroui-freedesktop`) | Text committed by an IBus daemon in the job |
| 2d | Drag and drop (XDND source and target) | A drag between two windows of the test, driven with `xdotool` |
| 2e | FreeDesktop services: the portal file chooser, the GTK dialogs and the GLib dispatcher, tray icon, global menu, platform settings, mounted volumes | Services against a session bus in the job (`dbus-run-session`), with test doubles of the portal interfaces |
| 2f | Session management, the native control host, XEmbed | A plug embedded in a socket window of the test |
| 3 | Accessibility: `X11AtSpiAccessibility` and `Avalonia.FreeDesktop.AtSpi` (27 files) | The tree read back over the accessibility bus |
| 4 | Wayland (`Avalonia.Wayland`, 81 files), sharing the FreeDesktop crate | A headless compositor in the job |
| 5 | The framebuffer backend (`Avalonia.LinuxFramebuffer`, 30 files: DRM and fbdev) | To be designed |

## 15. Deviations

Recorded in `DEVIATIONS.md`, section "X11 platform": the renamed environment variables, atom and format names; the event loop on non-Linux Unix systems; where the framebuffer of a surface lives; the change event of the screens; how a window icon is recognised; what is not built yet and how each such place behaves.
