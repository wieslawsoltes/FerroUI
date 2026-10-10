# The Linux platform: X11 and the FreeDesktop services

The design of the Linux desktop backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Avalonia.X11` (88 files, 291 types, 4897 members) with `src/Avalonia.FreeDesktop` (18 files, 26 types, 216 members) at the tracked commit (`TRACKING.md`). `Avalonia.FreeDesktop.AtSpi` (27 files, accessibility), `Avalonia.Wayland` (81 files) and `Avalonia.LinuxFramebuffer` (30 files) are later stages, named at the end.

Marks, as in `browser-platform.md`: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`), **[M]** measured here (a build or a test run on this machine), **[R]** recalled and not verified here, **[CI]** to be shown by the CI job on a real X server. **[VM]** measured in a virtual machine on the development machine (Ubuntu 24.04, ARM64, section 13). The development machine is a Mac; the CI job runs on x86-64 and had not run when this was written.

## 1. Crates

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-x11` | `src/FerroUI.X11` | `Avalonia.X11`, and `src/Shared/RawEventGrouping.cs`, which that project compiles in | `AppBuilder::use_x11()` (`FerroX11PlatformExtensions`); `use_platform_detect()` of `ferroui-desktop` on Linux |
| `ferroui-freedesktop` | `src/FerroUI.FreeDesktop` (since stage 2c) | `Avalonia.FreeDesktop` | used by `ferroui-x11` (and later by the Wayland backend), as upstream's project reference |

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

How the port keeps the `unsafe` of a C binding contained: `xlib.rs` is the port of `XLib.cs` and `XLib.Helpers.cs` and the one module that calls the libraries. Every call is a safe function there, with its safety argument; results are copied out of C memory and freed before the function returns (`WindowProperty` for `XGetWindowProperty`, `XIDeviceEventData` for an `XIDeviceEvent`, `MonitorInfo` for `XRRGetMonitors`). A connection is the type `XDisplay`, made only by `x_open_display` and never closed (as upstream), which is what lets calls that take only a connection and identifiers be safe. An event (`XEvent`, a C union of plain data) is read through views (`button_event(&ev)`, `configure_event(&ev)`), sound for any event because every bit pattern is valid for every member. `x_shm/x11_shm_image.rs` owns the two allocations of a shared memory image, `lib_c.rs` has the four shared memory calls of the C library; `glx/glx.rs` is the same for `libGL` (the port of `Glx.cs`), and `x11_egl_helper.rs` loads `libEGL` with `dlopen` and resolves its entry points with `dlsym`. Outside these there are four places with `unsafe`: the system calls of the event loop (`dispatching/x11_platform_threading.rs`), the error handler Xlib calls (`x_error.rs`), the blit of a framebuffer by address (`x11_framebuffer_surface.rs`), and tests that build C events.

The structures of `X11Structs.cs` and `XIStructs.cs` are therefore not ported: they are those of `x11-dl`. Their enumerations are (`x11_structs.rs`, `x11_enums.rs`, `xi_structs.rs`), converted mechanically from the upstream files: flags as `bitflags`, enumerations with equal members as sets of constants.

## 3. D-Bus: `zbus`

Upstream's FreeDesktop project references `Tmds.DBus.Protocol` and generates its proxies and handlers from the interface descriptions in `DBusXml/` with `Tmds.DBus.Generator` **[V]**. The interfaces: `org.freedesktop.portal.FileChooser`, `.Request` and `.Settings`, `org.kde.StatusNotifierWatcher` and `.StatusNotifierItem`, `com.canonical.dbusmenu` and `com.canonical.AppMenu.Registrar`, `org.freedesktop.DBus`, `org.freedesktop.IBus.Portal` (with `InputContext` and `Service`), and the four Fcitx interfaces **[V]**.

The port uses **`zbus` 5.19.0**, pinned exactly (MIT; pure Rust; minimum Rust 1.87, the workspace is at 1.89; `cargo info zbus`, 2026-10-10) **[V]**:

- Pure Rust, so the FreeDesktop crate checks for Linux from another system like the X11 crate does **[M]**, and needs no `libdbus` on the machine that runs the application.
- Proxies and interfaces are written as traits and `impl` blocks with attribute macros (`#[proxy]`, `#[interface]`), the counterpart of upstream's generated code; the descriptions in upstream's `DBusXml/` are the specification they are written from. A proxy has the members the port calls and the signals it listens to, not the whole interface.
- **Asynchronous calls on the UI dispatcher, a blocking connect.** With its default features (`async-io`, `blocking-api`) a connection reads its socket on a thread of its own (the internal executor of the library), and every call is a future that any executor can await **[V]**. The port awaits them as tasks of the dispatcher of the UI thread (`Dispatcher::invoke_async_task_local`): a reply or a signal that arrives on the thread of the library wakes the task by posting a job to the dispatcher, so every handler runs on the UI thread, like upstream's continuations, which resume on the synchronization context. Measured: the tests of the crate make calls and receive signals this way, against a service on another connection, and the order in which a signal and the reply that follows it are handled is the order they were sent in (the test of the text IBus commits while it is being reset depends on it) **[M]**. Only the connection itself is made blocking (`DBusHelper::try_create_new_connection`, the blocking builder of the library), as upstream connects synchronously. The `tokio` feature stays off: the port has no Tokio runtime.
- Signals (`NameOwnerChanged`, the input method signals; later `SettingChanged`, `Response`) are streams; a subscription is a task of the UI dispatcher that ends when its handle is disposed (`signal_watch.rs`, the counterpart of the `IDisposable` upstream's `Watch...Async` returns).
- Watching the owner of a name (upstream: `WatchNameOwnerAsync` of its library) is the signal `NameOwnerChanged` of the bus for that name, subscribed before `GetNameOwner` is asked, so that no change is lost.

How it is tested without a bus: the feature `p2p` of the library (in the tests only) connects two ends of a socket pair. The second end serves doubles of the IBus and Fcitx services and of the bus itself (`GetNameOwner`, `NameOwnerChanged`), and gives its signals the sender a bus would give them, because the proxies of the library filter signals by the unique name of the owner. The same tests run on a real bus with `FERROUI_FREEDESKTOP_TEST_BUS=session` under `dbus-run-session`, where the doubles take and release the well-known names.

| Alternative | Why not |
|---|---|
| `dbus` 0.9 | Binds `libdbus-1` (C): a build and run-time dependency on the system library, and no check from another system. |
| `rustbus` 0.19 | Pure Rust but synchronous only, with no proxy or interface macros: the message loop and every interface would be written by hand. |
| Port upstream's D-Bus stack | `Tmds.DBus` is a third-party library, not part of the framework. |

Upstream's `DBusCallQueue` (one call in flight, in order) is ported as it is, over `zbus` calls: it orders the calls of an input method context, which `zbus` does not do by itself. Its `async void` loop is a task of the UI dispatcher, which starts with the next job of the dispatcher and not inside the call that queued (DEVIATIONS.md).

## 4. Skia on Linux

- The Skia backend of the port builds with the features of its target (`src/Skia/FerroUI.Skia/Cargo.toml`): Graphite on Metal for Apple targets, Ganesh on WebGL for the browser, Ganesh on OpenGL ES for Windows, and since stage 2a **Ganesh on OpenGL for Linux** (the feature `gl` of `skia-safe`, and the configuration `ferro_skia_ganesh_gl` from `build.rs`, as for Windows). Raster is always present: it is what the software mode renders with (Skia draws into memory and the X11 platform sends the pixels to the server).
- `skia-bindings` 0.153.3 has, for Linux, the features `gl` (with `egl`, `x11`, `wayland` selecting the window system libraries it links: `EGL`, `GL`, `wayland-egl`), `vulkan`, and `graphite` as an engine feature; a Linux build links `freetype2` and `fontconfig` found through `pkg-config`, and `stdc++` **[V]** (`build_support/platform/linux.rs`, `features.rs` of the crate in the cargo registry).
- **The feature set for Linux is `gl` alone** (the bindings then have `ganesh` and `gl`; the key of the binary is `ganesh-gl-jpegd-jpege-pdf`). The features `x11`, `egl` and `wayland` of the bindings only add the window system libraries (`GL`, `EGL`, `wayland-egl`) with which Skia would create a native interface of its own; the backend never asks Skia for one: it gives Skia the entry points of the context the platform created (`glXGetProcAddress`, `eglGetProcAddress`), as on Windows and in the browser. So one binary serves GLX and EGL, and nothing of the window system is linked: the X11 crate opens `libGL` and `libEGL` at run time.
- **Published binaries.** The key of a binary is built from the feature list, and the list of published keys is in the workflow files of the rust-skia repository, so it is asked, not read: the job `x11` of CI asks the release of the bindings for both keys (raster and `ganesh-gl`) for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` and fails without the `ganesh-gl` one **[CI]**. For `aarch64-unknown-linux-gnu` the binary with `gl` was downloaded in the virtual machine (the build of the example with the new feature took 1 min 41 s; a source build of Skia takes about an hour) **[VM]**. A combination without a binary would make the build fall back to compiling Skia from source (`browser-platform.md`, section 3).
- Vulkan needs `vulkan` and the Vulkan GPU of the Skia backend, which is not ported (`CRITICAL-PATH.md`, row 16); whether a binary with `gl` and `vulkan` together is published for Linux has to be asked the same way when that is built.

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
| Software, shared memory | `XShm/*`, `X11DeferredDisplayDispatcher.cs`, `LibC.cs` | 2b, built | `XShmCreateImage` over a System V segment (`shmget`, `shmat`), `XShmPutImage` with a completion event, at most two frames in flight (a third `lock` waits on the events of the render connection), images pooled by size and disposed in the order of the extension's specification (detach, destroy the image, detach and remove the segment). Only with `use_x_shm_framebuffer`, depth 32 and the extension; the surface comes before the `XPutImage` one in the list of the window. The dispatcher of the completion events is an object of the thread that renders (`X11DeferredDisplayDispatcher::for_current_thread`); upstream creates it on the platform and uses it from the render thread. The completion event is declared in `xlib.rs`: the declaration of the bindings has two members of other sizes, which moves the segment identifier. |
| EGL | `X11EglHelper.cs`, `EglPlatformGraphics` of the OpenGL project (ported) | 2a, built | `libEGL.so.1` loaded by name; the display through `EGL_PLATFORM_X11` when the client extensions have it; configurations probed by creating a window surface on a throwaway window, those with a visual of depth 32 first (an nvidia workaround); the window takes the visual of the configuration; a child window as the render window; the `EglGlPlatformSurface` of the OpenGL crate. |
| GLX | `Glx/*` | 2a, built | `libGL` opened at run time, `glXCreateContextAttribsARB` through `glXGetProcAddress`; a frame buffer configuration with a preference for a visual of depth 32; a pixel buffer as the default drawable of a context; the profiles of the options tried in order; the renderer blacklist (`llvmpipe`, `SVGA3D`; `FERROUI_GLX_IGNORE_RENDERER_BLACKLIST=1` overrides it); the render window resized from the compositor at the beginning of a frame. Upstream's default mode list is GLX, then software. |
| Vulkan | `Vulkan/*` | 2a, open: after the Vulkan GPU of the Skia backend | `VK_KHR_xlib_surface` over the window. Until then the mode is passed over like one that failed to initialize. |

`FerroX11Platform::initialize_graphics` walks `rendering_mode` as upstream: software ends the walk without platform graphics; GLX and EGL are tried, and the first that initializes gives the graphics; a mode that does not is logged (area `OpenGL`) and passed over; an empty list, or a list in which nothing applies, fails with upstream's messages. The walk is a function over a factory, tested on every host.

**Threads and the GPU modes.** Upstream shares its GLX display and its EGL display between the UI thread, which probes them and reads the visual for its windows, and the render thread, which creates the contexts. In the port the platform graphics are the object the two threads share (`IPlatformGraphics: Send + Sync`), and what belongs to one thread stays there:

- `GlxDisplay` holds the connection, the frame buffer configuration, the visual, the extension list and the version that worked (behind a lock): values and identifiers of the GLX library, which locks the connection. The probe context (`DeferredContext`) is kept for the thread that created the display. A `GlxContext` is an object of the thread that created it, so the monitor upstream guards it with is not needed.
- The EGL display of the OpenGL crate is an object of one thread. `X11EglPlatformGraphics` (in `x11_egl_helper.rs`) keeps the display it was probed with for the UI thread and gives another thread a display object of its own over the same connection: EGL has one display per native connection, so both are the same display of the library with the same configuration. The visual of the configuration is read once and kept as a value.
- The window asks the platform which of its graphics it registered (`glx_graphics()`, `egl_graphics()`), where upstream casts the registered service.

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
- **Input methods** (stage 2c, built). The platform decides at start (`FerroX11Platform::initialize`, when `enable_ime` and the environment allow input methods): an input method over D-Bus when the environment names one this port has (`X11DBusImeHelper`: this framework's module variable, `GTK_IM_MODULE`, `QT_IM_MODULE`, then `@im=` of `XMODIFIERS`; `ibus`, `fcitx`, `fcitx5`) and the session bus can be reached; otherwise the input method of the server when `XMODIFIERS` is set and no other module is asked for (`should_use_xim`), which makes `X11Info` open it with the modifiers of the environment. A window then has an input method (`initialize_ime`), offered as its `ITextInputMethodImpl` feature, which is what the framework gives the focused text control as its client.
  - **Over D-Bus** (`ferroui-freedesktop`, `dbus_ime/`): `DBusTextInputMethodBase` watches the names of the service, connects to the first that is there (again after the service is gone and back), and reports focus (the window is active and a client is set), capabilities and the cursor rectangle in screen pixels through the call queue, each only when it changed. `IBusX11TextInputMethod` talks to the portal of IBus (`org.freedesktop.portal.IBus`), `FcitxX11TextInputMethod` to Fcitx 4 (`org.fcitx.Fcitx`) or 5 (`org.freedesktop.portal.Fcitx`) through `FcitxICWrapper`. While such an input method is enabled, a key event of the window goes to its queue first (`filter_ime`): the events are offered to the input method one at a time, in order, and those it does not consume go on to the application (the release of a modifier key always does). Committed text arrives as text input; a forwarded key as a key event without a physical key.
  - **The input method of the server** (`x11_window_xim.rs`, `XimInputMethod`): the input context of the window gets and loses the focus with the window and the client, is reset, and is told the spot of the pre-edit (the bottom left of the cursor rectangle, in a job at background priority). The text comes through the input context when a key is looked up (`Xutf8LookupString`), after `XFilterEvent` gave the event to the input method; this class filters nothing itself.
- Raw input is queued and grouped (`raw_event_grouping.rs`): consecutive moves of one device with the same modifiers become one event with intermediate points.

## 10. Clipboard and selections

`selections/` is the ICCCM selection protocol, shared by the clipboard (stage 1) and drag and drop (stage 2d):

- **Owning** (`SelectionDataProvider`): `TARGETS` (with `MULTIPLE`), `SAVE_TARGETS` for the clipboard manager at exit, `MULTIPLE` requests as pairs, and data; a value larger than the maximum property size (from the maximum request size of the connection) is sent incrementally (`INCR`: announce the length, then one part per deletion of the property by the requestor, ended by an empty part), with a timeout of five seconds per step.
- **Reading** (`SelectionReadSession`): `XConvertSelection` to a property on a window of its own (`EventStreamWindow`), the `SelectionNotify`, the property, and `INCR` reads until an empty part.
- **Formats** (`DataFormatHelper`): text as `UTF16_STRING`, `UTF8_STRING`, `text/plain`, `text/plain;charset=utf-8`, `STRING` in that order of preference; files as `text/uri-list`; bitmaps as `image/png` (and `image/jpeg` when reading); application formats under a prefix.
- The clipboard and the primary selection are two instances of `X11ClipboardImpl`.

The protocol steps are state machines over a small connection trait (`ISelectionConnection`), which the tests drive with a mock; the asynchronous waits are futures on the UI dispatcher.

Drag and drop (stage 2d, built; `selections/drag_drop/`) is XDND version 5 (minimum 3):

- **Target** (`X11DropTarget`, one per window, when the drag and drop device of the framework is registered). The window announces `XdndAware`. `XdndEnter` gives the source, its version (a source outside 3 to 5 is ignored) and its formats (the three of the message, or `XdndTypeList` of the source window when the message says there are more); `XdndPosition` raises a drag event at the point of the window (the first one enters, the following ones move over) and answers with `XdndStatus` and the action of the effects the handlers chose; `XdndLeave` raises the leave; `XdndDrop` raises the drop where the pointer last was and answers with `XdndFinished` and the action of the drop. The data is read when a handler asks for it, as the selection `XdndSelection`, into a property of the window itself, through `SynchronousXEventWaiter`: a wait inside the call that takes events off the queue of the connection until the answer comes (at most five seconds) and puts the others back in their order.
- **Source** (`X11DragSource`, registered as the drag source of the platform). A drag has a handler that is the event hook of the dispatcher while it lasts. It assumes the implicit pointer grab of the press that started the drag: pointer events of any window are the drag's (core events, and the events of the X Input extension), and the cursor of the grab shows the current effect (the drag cursors of the cursor theme). The target under the pointer is the first window from the root down that is a top-level of the application or has `XdndAware` (itself or the window its `XdndProxy` names, when that names itself). A window of the application gets its drag events directly, without the protocol. Another client gets `XdndEnter` and `XdndPosition` with the action of the effects that are allowed with the modifier keys that are down (control: copy; shift: move; alt: link); positions are sent one at a time, the last one kept until `XdndStatus` answers; a release sends `XdndDrop` (after the pending answer), or `XdndLeave` when the target refused; `XdndFinished` ends the drag with the action of the target (version 5), limited to the allowed effects. A target that does not answer within five seconds ends the drag without an effect (`DragDropTimeoutManager`; sending data to the target restarts the time). The data is offered as the selection `XdndSelection` by `DragDropDataProvider`, the provider of the clipboard for another selection.

The handler of a drag has the protocol and reaches the connection and the platform through a host (`IDragSourceHost`), and the drop target through a connection trait (`IXdndTargetConnection`), like the selection transfers: the tests drive both with doubles (a tree of windows with properties; a recording connection, a window at a known place and a drag and drop device that answers with chosen effects).

## 11. FreeDesktop services (stage 2e)

- **Storage provider.** A window offers a fallback chain, as upstream: the file chooser portal (`DBusSystemDialog`, `org.freedesktop.portal.FileChooser` on `org.freedesktop.portal.Desktop`, version checked; the parent window as `x11:<hex id>`; the response awaited on the request object, subscribed before the call), then the GTK dialogs (`NativeDialogs/Gtk*`: `libgtk-3`, a thread running the GTK main loop unless the GLib dispatcher is used, the parent set through a foreign GDK window), then the managed dialogs. Stage 1 has the last link.
- **Tray icon** (`DBusTrayIconImpl`): a `StatusNotifierItem` on a connection of its own, registered with `org.kde.StatusNotifierWatcher`, with the menu exported by `DBusMenuExporter`. Upstream's fallback (`XEmbedTrayIconImpl`) only logs. Until built, the platform has no tray icon.
- **Global menu** (`DBusMenuExporter`): `com.canonical.dbusmenu`, registered per window with `com.canonical.AppMenu.Registrar` (option `use_d_bus_menu`).
- **Platform settings** (`DBusPlatformSettings`, built): colour scheme and accent colour from `org.freedesktop.portal.Settings` (`org.freedesktop.appearance`; `ReadOne` from version 2 of the portal, the deprecated `Read` before), with change notifications (`SettingChanged`). Without a session bus or a portal, the defaults of the framework. Upstream reads nothing else at the tracked commit: no contrast preference. The X11 platform registers it as the platform settings.
- **Mounted volumes** (`LinuxMountedVolumeInfoProvider`, built): `/proc/partitions`, `/proc/mounts` and `/dev/disk/by-label`, polled every second; the X11 platform registers it for the managed file dialogs.

Stage 2f, the rest of the X11 project: session management (`X11PlatformLifetimeEvents` over `libSM` and `libICE`: the shutdown request of the session manager), the native control host (`X11NativeControlHost`), XEmbed (`XEmbedPlug`, `XEmbedClientWindowMode`).

## 12. File table

One row per upstream file. "built" files are on the branch; the stage of an open file is the one that builds it.

| Upstream file (`src/Avalonia.X11`) | Lines | Rust file (`src/FerroUI.X11`) | Stage | State | Notes |
|---|---:|---|---|---|---|
| `ActivityTrackingHelper.cs` | 115 | `activity_tracking_helper.rs` | 1 | built |  |
| `ICELib.cs` | 60 | `ice_lib.rs` | 2f | open |  |
| `Keysyms.cs` | 2108 | `keysyms.rs` | 1 | built | a set of constants: the enumeration has members with equal values |
| `LibC.cs` | 34 | `lib_c.rs` | 2b | built | the shared memory calls, with the shared memory framebuffer |
| `SMLib.cs` | 132 | `sm_lib.rs` | 2f | open |  |
| `TransparencyHelper.cs` | 112 | `transparency_helper.rs` | 1 | built |  |
| `X11ActiveWindowTracker.cs` | 55 | `x11_active_window_tracker.rs` | 1 | built |  |
| `X11Atoms.cs` | 275 | `x11_atoms.rs` | 1 | built |  |
| `X11AtSpiAccessibility.cs` | 176 | `x11_at_spi_accessibility.rs` | 3 | open |  |
| `X11CursorFactory.cs` | 182 | `x11_cursor_factory.rs` | 1 | built |  |
| `X11DeferredDisplayDispatcher.cs` | 98 | `x11_deferred_display_dispatcher.rs` | 2b | built |  |
| `X11EglHelper.cs` | 97 | `x11_egl_helper.rs` | 2a | built | with the display factory of `InitializeGraphics`, the loader of `libEGL` and the platform graphics of EGL (`X11EglPlatformGraphics`) |
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
| `X11Window.Ime.cs` | 377 | `x11_window_ime.rs` | 1, 2c | built | the keyboard part in stage 1, the input method of the window and its key queue in stage 2c |
| `X11Window.Xim.cs` | 126 | `x11_window_xim.rs` | 2c | built |  |
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
| `Glx/Glx.cs` | 126 | `glx/glx.rs` | 2a | built |  |
| `Glx/GlxConsts.cs` | 108 | `glx/glx_consts.rs` | 2a | built |  |
| `Glx/GlxContext.cs` | 139 | `glx/glx_context.rs` | 2a | built |  |
| `Glx/GlxDisplay.cs` | 195 | `glx/glx_display.rs` | 2a | built |  |
| `Glx/GlxGlPlatformSurface.cs` | 102 | `glx/glx_gl_platform_surface.rs` | 2a | built |  |
| `Glx/GlxPlatformFeature.cs` | 38 | `glx/glx_platform_feature.rs` | 2a | built |  |
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
| `Selections/DragDrop/DragDropDataProvider.cs` | 30 | `selections/drag_drop/drag_drop_data_provider.rs` | 2d | built |  |
| `Selections/DragDrop/DragDropDataReader.cs` | 49 | `selections/drag_drop/drag_drop_data_reader.rs` | 2d | built |  |
| `Selections/DragDrop/DragDropDataTransfer.cs` | 40 | `selections/drag_drop/drag_drop_data_transfer.rs` | 2d | built | holds the data transfer of the base library |
| `Selections/DragDrop/DragDropDataTransferItem.cs` | 31 | `selections/drag_drop/drag_drop_data_transfer_item.rs` | 2d | built |  |
| `Selections/DragDrop/DragDropTimeoutManager.cs` | 41 | `selections/drag_drop/drag_drop_timeout_manager.rs` | 2d | built | a timer of the UI dispatcher |
| `Selections/DragDrop/IXdndWindow.cs` | 13 | `selections/drag_drop/i_xdnd_window.rs` | 2d | built |  |
| `Selections/DragDrop/SynchronousXEventWaiter.cs` | 124 | `selections/drag_drop/synchronous_x_event_waiter.rs` | 2d | built |  |
| `Selections/DragDrop/X11DragSource.cs` | 766 | `selections/drag_drop/x11_drag_source.rs` | 2d | built | the handler over a host (`IDragSourceHost`); tests in `x11_drag_source/tests.rs` |
| `Selections/DragDrop/X11DropTarget.cs` | 204 | `selections/drag_drop/x11_drop_target.rs` | 2d | built | over a connection trait; tests in `x11_drop_target/tests.rs` |
| `Selections/DragDrop/XdndActionHelper.cs` | 29 | `selections/drag_drop/xdnd_action_helper.rs` | 2d | built |  |
| `Selections/DragDrop/XdndConstants.cs` | 8 | `selections/drag_drop/xdnd_constants.rs` | 2d | built |  |
| `Selections/IXEventWaiter.cs` | 9 | `selections/i_x_event_waiter.rs` | 1 | built |  |
| `Selections/SelectionDataProvider.cs` | 261 | `selections/selection_data_provider.rs` | 1 | built |  |
| `Selections/SelectionDataReader.cs` | 108 | `selections/selection_data_reader.rs` | 1 | built |  |
| `Selections/SelectionHelper.cs` | 8 | `selections/selection_helper.rs` | 1 | built |  |
| `Selections/SelectionReadSession.cs` | 150 | `selections/selection_read_session.rs` | 1 | built |  |
| `Selections/UriListHelper.cs` | 54 | `selections/uri_list_helper.rs` | 1 | built |  |
| `Vulkan/VulkanNativeInterop.cs` | 29 | `vulkan/vulkan_native_interop.rs` | 2a | open | waits for the Vulkan project of the port and the Vulkan GPU of the Skia backend |
| `Vulkan/VulkanSupport.cs` | 95 | `vulkan/vulkan_support.rs` | 2a | open | as above |
| `X11WindowModes/DefaultWindowMode.cs` | 87 | `x11_window_modes/default_window_mode.rs` | 1 | built |  |
| `X11WindowModes/InputProxyWindowMode.cs` | 55 | `x11_window_modes/input_proxy_window_mode.rs` | 1 | built |  |
| `X11WindowModes/WindowMode.cs` | 61 | `x11_window_modes/window_mode.rs` | 1 | built |  |
| `X11WindowModes/XEmbedClientWindowMode.cs` | 208 | `x11_window_modes/x_embed_client_window_mode.rs` | 2f | open |  |
| `XShm/X11ShmFramebufferRenderTarget.cs` | 176 | `x_shm/x11_shm_framebuffer_render_target.rs` | 2b | built |  |
| `XShm/X11ShmFramebufferSurface.cs` | 31 | `x_shm/x11_shm_framebuffer_surface.rs` | 2b | built |  |
| `XShm/X11ShmImage.cs` | 105 | `x_shm/x11_shm_image.rs` | 2b | built |  |

| Upstream file (`src/Avalonia.FreeDesktop`) | Lines | Rust file (`src/FerroUI.FreeDesktop`) | Stage | State | Notes |
|---|---:|---|---|---|---|
| `DBusCallQueue.cs` | 105 | `dbus_call_queue.rs` | 2c | built | one call in flight, in order |
| `DBusHelper.cs` | 57 | `dbus_helper.rs` | 2c | built | the session bus connection |
| `DBusMenuExporter.cs` | 355 | `dbus_menu_exporter.rs` | 2e | open | `com.canonical.dbusmenu`, `com.canonical.AppMenu.Registrar` |
| `DBusPlatformSettings.cs` | 148 | `dbus_platform_settings.rs` | 2e | built | `org.freedesktop.portal.Settings`: colour scheme, accent colour; holds the default settings |
| `DBusSystemDialog.cs` | 294 | `dbus_system_dialog.rs` | 2e | open | `org.freedesktop.portal.FileChooser` |
| `DBusTrayIconImpl.cs` | 444 | `dbus_tray_icon_impl.rs` | 2e | open | `org.kde.StatusNotifierItem` |
| `IPortalParentLease.cs` | 29 | `i_portal_parent_lease.rs` | 2e | built |  |
| `IX11InputMethod.cs` | 34 | `ix11_input_method.rs` | 2c | built |  |
| `LinuxMountedVolumeInfoListener.cs` | 103 | `linux_mounted_volume_info_listener.rs` | 2e | built | the parsing as functions over text |
| `LinuxMountedVolumeInfoProvider.cs` | 15 | `linux_mounted_volume_info_provider.rs` | 2e | built |  |
| `NativeMethods.cs` | 40 | `native_methods.rs` | 2e | built | `readlink`: `std::fs::read_link` |
| `DBusIme/DBusTextInputMethodBase.cs` | 358 | `dbus_ime/dbus_text_input_method_base.rs` | 2c | built | the base is the object and holds the part that differs (`DBusTextInputMethodCore`) |
| `DBusIme/Fcitx/FcitxEnums.cs` | 67 | `dbus_ime/fcitx/fcitx_enums.rs` | 2c | built |  |
| `DBusIme/Fcitx/FcitxICWrapper.cs` | 60 | `dbus_ime/fcitx/fcitx_ic_wrapper.rs` | 2c | built | an enumeration of the two proxies |
| `DBusIme/Fcitx/FcitxX11TextInputMethod.cs` | 200 | `dbus_ime/fcitx/fcitx_x11_text_input_method.rs` | 2c | built |  |
| `DBusIme/IBus/IBusEnums.cs` | 45 | `dbus_ime/ibus/ibus_enums.rs` | 2c | built |  |
| `DBusIme/IBus/IBusX11TextInputMethod.cs` | 185 | `dbus_ime/ibus/ibus_x11_text_input_method.rs` | 2c | built |  |
| `DBusIme/X11DBusImeHelper.cs` | 65 | `dbus_ime/x11_dbus_ime_helper.rs` | 2c | built |  |

`RawEventGrouping.cs` of `src/Shared` is `raw_event_grouping.rs` (built). Files of the port without an upstream file: in the X11 crate `pixel_buffer.rs` (a framebuffer over pixels the crate owns); in the FreeDesktop crate `event.rs` (the multicast events of the backends' own classes; it moved there from the X11 crate, which re-exports it), `signal_watch.rs` (a subscription to a signal as a disposable handle, and a cancellation flag: facilities of upstream's D-Bus library and runtime) and the proxy traits `dbus_ime/ibus/dbus.rs` and `dbus_ime/fcitx/dbus.rs` (the code upstream generates from `DBusXml/`).

## 13. Verification

Three levels, because an X application cannot run on the development machine:

1. **Compilation for Linux**: `cargo check -p ferroui-x11 --target x86_64-unknown-linux-gnu --all-targets` **[M]**, and the host: the crate compiles whole on macOS.
2. **Tests without a server** (`cargo test -p ferroui-x11`, 218 tests at the end of stage 2d, of which 34 are of drag and drop, and `cargo test -p ferroui-freedesktop`, 41 tests, of which 9 run the input methods against doubles of the IBus and Fcitx services and 4 the platform settings against a double of the settings portal **[M]**; 184 at the end of stage 2c; 173 at the end of stage 2b; 163 at the end of stage 1 **[VM]**), of everything that is logic and not a round trip: the key tables; atoms and property decoding; Motif, size and state hints, window state from `_NET_WM_STATE`, allowed actions, frame extents; the translation of X Input events (built as the C structures the library delivers, copied as the dispatcher copies them); scaling from `Xft.dpi`, the environment and physical sizes, EDID, refresh rates, working areas; the resource database; the selection protocol with a mock connection (targets, multiple, incremental transfers in both directions, timeouts), formats and encodings, uri lists; the wake-up pipe and the timer wait of the event loop; the cursor tables; icon data; the options and the input method decisions; the walk of the rendering modes with a mock of the graphics; the choice of the frame buffer configuration, the context attributes of a version and the renderer blacklist of GLX; the probe order of the EGL configurations and the test for the X11 platform extension.
3. **A real server** (the job `x11` of `.github/workflows/ci.yml`, Ubuntu, Xvfb; and the virtual machine): the crate is built for Linux, its tests run on Linux, and `examples/x11_window.rs --smoke` runs under `xvfb-run`. The smoke mode asks the server, not the framework, wherever it can. Every check prints a line and the exit code is the result. Its phases:

| Phase | What is done | What is asked of whom |
|---|---|---|
| window | The window is shown | The server: viewable, the expected size, `WM_NAME` and `_NET_WM_NAME`, `_NET_WM_PID`, `WM_PROTOCOLS`, `WM_CLASS`, `_NET_WM_WINDOW_TYPE`, and pixels read back with `XGetImage` at three points have the fill colour |
| gpu (`--mode=glx`, `--mode=egl`) | Nothing more: the window is rendered through the mode | The platform: it registered the graphics of the mode and did not fall back. The server: the render window is a viewable child of the window with its size. OpenGL: a context of the platform graphics clears an offscreen framebuffer and `glReadPixels` returns the colour; with GLX a second context is made current on the render window and `glReadPixels` finds the frame in one of its buffers (on Mesa's software rasterizer the back buffer, which keeps the frame after the swap; the front buffer reads as zero). With EGL the frame is checked through the server alone: a window has one EGL surface, and it belongs to the compositor. `--expect-fallback` is the opposite check: the mode is refused and software rendering takes over |
| input | The server synthesizes input through the XTEST extension: two pointer moves, the left button down and up, the wheel up and down (buttons 4 and 5), the key that produces "a" down and up | The input callback of the window (`ITopLevelImpl::input`, with a recorder in front of the framework's handler): a move at the expected position, the button events there, wheel deltas of one step, the key with its symbol, and the text "a" |
| popup | A `Popup` with light dismiss is opened over the window at an offset; XTEST presses inside it and then on the window beside it | The server: one more viewable override-redirect child of the root with the process identifier of the application, at the origin of the window plus the offset, of the expected size, with the popup's colour at its centre. The framework: the popup stays open after the press inside and is closed (the closed event raised, the window gone from the server) after the press outside. The platform takes no pointer grab for a popup, as upstream: the framework dismisses it when the press arrives at its parent |
| shm (`--shm`) | The frames of software rendering go through the shared memory extension | The server has MIT-SHM; the first surface of the window is the shared memory one, so the pixels of every other phase arrived through it |
| resize | The window is given another size (520 by 320) | The server: the window has that size, and the fill colour at its centre and near its new bottom right corner, which takes frames of the new size (a new framebuffer, new shared memory images, a render window resized by the compositor) |
| screens | Nothing, or two monitors made by the caller with `xrandr --setmonitor` | The screens of the platform lie inside the root window, do not overlap and cover it; `--expect-screens=N` states their number |
| clipboard | Text is set and another client (`xclip`) reads it; `xclip` owns text and the framework reads it; each once with a short text with non-ASCII characters and once with three mebibytes | The bytes the other client printed, and the text the framework read. Three mebibytes are more than the largest property the platform writes (one mebibyte) and than the part size of `xclip`, so each side transfers in parts (`INCR`) |
| ime (`--ime=ibus`) | The example is also a service on the session bus that answers as the portal of IBus does, consumes the key "a" and commits another text for it. A text box gets the focus; XTEST presses "a", then "b" | The platform registered the factory of the input method; the window has a text input method; the service was asked for an input context and for its focus; the input callback of the window gets the committed text and no key event for "a", then the key "b" with its text, and the text box has both; the service was offered the four key events in order, each with the key code of the server and the release bit; the cursor location it was told lies inside the window as the server has it |
| ime (`--ime=xim`) | `XMODIFIERS=@im=local`: the input method Xlib has built in. A text box gets the focus; XTEST presses "a", then the compose key, an apostrophe and "e" (the caller gives a key the compose symbol with `xmodmap`) | The platform opened the input method of the server and registered no factory; the window has a text input method; "a" arrives as text through the input context; the compose sequence arrives as the one character it composes, which only an input method produces |
| dnd (`--dnd`) | A square in the window starts a drag of a text when it is pressed. XTEST presses it, moves in steps and releases: first over the window itself, then over the window of a second process (the example started with `--dnd-target=X,Y`: a window that accepts text and prints what is dropped) | The window announces `XdndAware` 5. Inside the window: the content gets the drop with the text and the drag ends with the copy effect; afterwards the event hook is removed and the window does not own `XdndSelection`. To the other process: its window is viewable, announces the protocol and is not a window of this process; it prints the dropped text and the effect; the drag of this process ends with the copy effect, the action the target finished with; the window of this process got no drop |
| close | A `WM_DELETE_WINDOW` message is sent to the window | The window closes and the application ends with exit code 0 |

The job runs the smoke mode on a bare Xvfb; rendered through GLX and through EGL (Mesa's `llvmpipe`, with `FERROUI_GLX_IGNORE_RENDERER_BLACKLIST=1` for GLX); through the shared memory framebuffer; with GLX asked for and refused (the fallback); under a window manager (`openbox`) on a screen narrower than the window, where the window manager clamps the window and the framework has to follow (the size check compares the server with the framework, and with the requested size only without a window manager); and with two monitors side by side (`xrandr --setmonitor`, the server started with `-noreset`: an Xvfb resets when its last client disconnects, and the monitors `xrandr` made would be gone before the application starts).

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

Nothing had to be changed in the platform for these runs.

### Measured in the virtual machine for stage 2 (2026-10-10) **[VM]**

The same machine (Mesa 25.2.8, `llvmpipe`; `xclip` 0.13 and `mesa-utils` installed for these runs), the smoke mode of section 13 under `xvfb-run`:

| Run | Result |
|---|---|
| Bare Xvfb, software | 32 of 32 checks: window, input (XTEST through the X Input extension: move, button, wheel, key code 38 with the symbol and the text "a"), popup (at (50, 40) of the root for a window at (10, 10), 120 by 80, its colour, kept by a press inside, dismissed by a press outside), one screen covering the root, the clipboard in both directions, small and in parts |
| Under `openbox`, 600 by 500, software | 32 of 32; the window at (1, 22) of the root, the popup relative to it |
| Two monitors (`xrandr --setmonitor`, `-noreset`), software, without the clipboard | 27 of 27; the screens `FERRO-A` (0, 0, 640 by 1024) and `FERRO-B` (640, 0, 640 by 1024) |
| `--mode=glx`, `FERROUI_GLX_IGNORE_RENDERER_BLACKLIST=1` | 37 of 37: the platform graphics of GLX; a render window, child of the window; an OpenGL 4.0 core context (Mesa reports 4.5, `llvmpipe (LLVM 20.1.2, 128 bits)`, 8 stencil bits); the offscreen draw read back as (51, 102, 153, 255); the frame read back with `glReadPixels` from the back buffer of the render window (the front buffer reads (0, 0, 0, 0)); the pixels of the window and of the popup on the server (`XGetImage`) as in software |
| `--mode=egl` | 36 of 36: the platform graphics of EGL, the render window, the same context and offscreen read back, the pixels of the window and of the popup on the server |

With the resize phase and the shared memory framebuffer of stage 2b (commit `987ae01b`), every variant again, in the virtual machine and in the CI job (run 38046204940, x86-64): software 33 of 33, `--shm` 35 of 35 (the server has MIT-SHM, the first of the three surfaces of the window is the shared memory one, and the window shows the fill colour up to its corner after the resize to 520 by 320, which takes images of the new size), GLX 38 of 38, EGL 37 of 37, GLX refused 14 of 14, under `openbox` 33 of 33, two monitors 28 of 28 **[VM]**; the job passed with the same steps **[CI]**. Stage 2b needed no change after its first run.

The ControlCatalog (`control-catalog-desktop`, `use_platform_detect`) was run in the virtual machine under Xvfb at 1280 by 800 with `FERROUI_SMOKE_PAGES`: all 75 pages are selected and drawn, once in software (GLX refuses `llvmpipe` and the default mode list goes on to software) and once through GLX (with the override), and the window closes on its timer. Pictures of the virtual display were taken per page (`import -window root`); the host and the platform needed no change **[VM]**.

What these runs found, and what was changed:

| Finding | Change |
|---|---|
| The text `xclip` owns was not read when it was larger than `xclip`'s part size: "the clipboard has no text". `xclip` announces an incremental transfer with an empty property of the type `INCR`; the reader returned nothing for an answer without items before looking at its type, as upstream does | The type is looked at first (`SelectionReadSession::send_data_request`; DEVIATIONS.md; a test with that announcement) |
| Two monitors made with `xrandr --setmonitor` were gone when the application started | Not the platform: an Xvfb resets when its last client disconnects. The server of that run is started with `-noreset` |
| With GLX on Mesa's software rasterizer, a second context current on the render window reads zero from the front buffer | Not the platform (the window on the server has the frame): the check reads both buffers and expects the frame in one |

Nothing else had to be changed: GLX and EGL rendered the first time they ran, on the render thread, popups included.

### Measured by the CI job (run 38045428868, 2026-10-10) **[CI]**

Ubuntu 24.04 on x86-64 (`ubuntu-latest`), Mesa 25.2.8 (`llvmpipe (LLVM 20.1.2, 256 bits)`), commit `0be4a818`:

| Step | Result |
|---|---|
| Skia features of the target | `ganesh` and `gl`, nothing else |
| Skia binaries published for Linux (`skia-bindings` 0.153.3, commit `b7f043e0b1e2a850e702`) | published for `x86_64-unknown-linux-gnu` and for `aarch64-unknown-linux-gnu`, each with the keys `jpegd-jpege-pdf` (raster) and `ganesh-gl-jpegd-jpege-pdf` |
| Tests of the crate | 173 passed |
| Smoke, bare Xvfb, software | 32 of 32 |
| Smoke through GLX | 37 of 37: an OpenGL 4.0 core context on Mesa 4.5, the offscreen read back, the frame in the back buffer |
| Smoke through EGL | 36 of 36 |
| GLX refused (`llvmpipe` on the blacklist), software takes over | 13 of 13 |
| Smoke under `openbox`, 600 by 500 | 32 of 32 |
| Smoke with two monitors | 27 of 27: `FERRO-A` and `FERRO-B`, 640 by 1024 each |

The first run of the extended job (29e3c0ac) failed in its three smoke steps and was superseded before its log could be read; the virtual machine had shown the same two causes by then (the announcement of `xclip`, and the monitors lost to the reset of the server), and the run above has both fixes.

Verified nowhere yet: a GPU with a hardware driver (the visual preference and the configuration probe exist for nvidia; the blacklist for `llvmpipe` and `SVGA3D` is only exercised as a refusal), input from a real device, a compositing manager under plain X11, more than one X screen (the platform uses the default screen, as upstream).

## 14. Stages

| Stage | Content | What its CI run proves |
|---|---|---|
| 1 (built; its unverified items closed in stage 2) | The crate and bindings; platform initialisation and options; atoms; the event loop; windows (creation, events, states, hints, activation, transparency, popups); screens with RandR and scaling; cursors; the software framebuffer; the render timer; pointer, touch and keyboard input; the clipboard; `use_x11` and `use_platform_detect` on Linux; the example | Section 13, level 3 |
| 2a (built, but Vulkan) | GPU rendering: GLX and EGL with Skia's Ganesh on OpenGL; the Skia feature set for Linux. Open: Vulkan, which waits for the Vulkan project of the port and the Vulkan GPU of the Skia backend | The Skia binaries for Linux asked of the release; the smoke run per mode on Mesa's software GL (`llvmpipe` needs the blacklist override), with `glReadPixels` and `XGetImage`; GLX refused and software taking over |
| 2b (built) | The shared memory framebuffer | The smoke run with `--shm` (`use_x_shm_framebuffer`): the extension, the surface, the pixels of every phase and of a resized window through it |
| 2c (built) | Input methods: the key event queue, XIM, IBus and Fcitx over D-Bus (starts `ferroui-freedesktop`) | The input methods against doubles of the services, without a bus and on a private session bus (`dbus-run-session`); the smoke run with `--ime=ibus` (the example is the service) and with `--ime=xim` (the input method Xlib has built in, a compose sequence). A real `ibus-daemon` is not used: which engine it starts, and when, is not deterministic; Fcitx is covered by the doubles only |
| 2d (built) | Drag and drop (XDND source and target) | The smoke run with `--dnd`: a drag inside the window, and a drag to a second process of the example, driven with XTEST. Both sides of the protocol are this port's: another toolkit as source or target was not run |
| 2e (in part: platform settings, mounted volumes, the portal parent lease) | FreeDesktop services: the portal file chooser, the GTK dialogs and the GLib dispatcher, tray icon, global menu, platform settings, mounted volumes | Services against a session bus in the job (`dbus-run-session`), with test doubles of the portal interfaces |
| 2f | Session management, the native control host, XEmbed | A plug embedded in a socket window of the test |
| 3 | Accessibility: `X11AtSpiAccessibility` and `Avalonia.FreeDesktop.AtSpi` (27 files) | The tree read back over the accessibility bus |
| 4 | Wayland (`Avalonia.Wayland`, 81 files), sharing the FreeDesktop crate | A headless compositor in the job |
| 5 | The framebuffer backend (`Avalonia.LinuxFramebuffer`, 30 files: DRM and fbdev) | To be designed |

## 15. Deviations

Recorded in `DEVIATIONS.md`, section "X11 platform": the renamed environment variables, atom and format names; the event loop on non-Linux Unix systems; where the framebuffer of a surface lives; the change event of the screens; how a window icon is recognised; what is not built yet and how each such place behaves.
