# The Linux platform: Wayland

The design of the Wayland backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Avalonia.Wayland` at the tracked commit (`TRACKING.md`): 84 source files (11 341 lines), a README and the project file. The backend shares the FreeDesktop crate with the X11 backend (`x11-platform.md`), and three source files of the X11 project, which upstream compiles into both.

Marks, as in `x11-platform.md`: **[V]** verified from sources (the upstream files, the sources of a crate in the cargo registry, `cargo info`), **[M]** measured here (a build or a test run on the development machine, a Mac), **[R]** recalled and not verified here, **[CI]** shown by the CI job on a compositor, **[VM]** measured in the virtual machine (Ubuntu 24.04, ARM64).

## 1. What upstream is

The README of the project is the specification of its architecture **[V]**:

- **A compositor may restart.** The backend keeps its own state of every surface in a form that can be sent again to a new connection (`Server/Persistent`), and treats everything bound to a connection as transient (`Server/Transient`). The worker reconnects (one probe a second) unless `EnableReconnects` is off or the connection came as a descriptor.
- **The compositor says when to render.** There is no render timer: a frame callback of a surface wakes the render loop. A fallback timer of 20 ticks a second covers what the callbacks miss (a commit of the UI thread while no surface has a callback pending).
- **One worker thread owns the connection and renders.** "Wayland worker thread == render thread". The UI thread sends commands as closures: with the next composition batch (so that they arrive with the frame they belong to) or out of band; the worker sends events to the UI thread as posted calls. No variable is shared.
- **A queue of its own.** All objects are on one event queue of the display that is not the default one, so the backend could live beside another toolkit on a foreign display.

Protocols it binds **[V]** (`Server/Transient/WaylandGlobals.cs`), with the version range it accepts:

| Global | Versions | Required | Used for |
|---|---|---|---|
| `wl_compositor` | 4 to 6 | yes | surfaces, regions (the input region of a surface that is not hit-test visible), `preferred_buffer_scale` (6) |
| `wl_shm` | 1 | yes | software frames, bitmap cursors, the cursor theme |
| `xdg_wm_base` | 3 to 4 | yes | `xdg_surface`, `xdg_toplevel` (with `configure_bounds`, 4), `xdg_popup` with `reposition` (3), `xdg_positioner` |
| `wl_seat` | 5 to 9 | no | pointer (frames, `axis_value120` from 8), keyboard, touch |
| `wl_output` | 2 to 4 | no (a top-level needs one) | screens; name and description (4) |
| `zxdg_output_manager_v1` | 3 | no | logical position and size of an output |
| `zxdg_decoration_manager_v1` | 1 | no | server-side decorations; without it, or once the application asks for less than full decorations, the framework draws them |
| `wp_fractional_scale_manager_v1`, `wp_viewporter` | 1 | no | fractional scaling (both, or neither) |
| `wl_data_device_manager` | 3 | no | clipboard and drag and drop |
| `zwp_text_input_manager_v3` | 1 | no | input methods |
| `zxdg_exporter_v2` | 1 | no | the parent handle of a portal dialog (`wayland:<handle>`) |
| `zwp_linux_dmabuf_v1` | 4 | no | the dmabuf swapchain (only with `UseDmabufSwapchain`) |

Not bound at the tracked commit, although a Wayland toolkit often has them **[V]** (`grep` over the project): the primary selection, pointer gestures, `wp_cursor_shape`, `xdg_activation`, `wp_presentation`, and `xdg_toplevel.set_app_id` is never called. The port follows, with one addition (the application identifier, section 7).

Upstream generates its protocol bindings from the XML descriptions (the NWayland package: one class per interface with a listener given at creation), calls `libwayland-client.so.0` through them, and declares `libwayland-cursor.so.0`, `libwayland-egl.so.1`, `libxkbcommon.so.0`, `libgbm` and `libdrm`, and the C library as platform invokes **[V]** (`Server/Interop/*`).

## 2. Bindings

### 2.1 The protocol: the `wayland-client` family over `libwayland-client`, opened at run time

| Crate | Version (pinned exactly) | Licence | What it is | Depends on (non-optional, beyond the family) |
|---|---|---|---|---|
| `wayland-client` | 0.31.15 | MIT | Proxies, event queues, the `Dispatch` trait; the core protocol generated from `wayland.xml` | `bitflags`, `rustix`, `wayland-scanner` (a procedural macro, build time) |
| `wayland-backend` | 0.3.17 | MIT | The wire: a pure Rust implementation, or `libwayland-client` with the feature `client_system` | `downcast-rs`, `rustix`, `smallvec`; `scoped-tls` and `wayland-sys` with `client_system` |
| `wayland-sys` | 0.31.11 | MIT | The C declarations of `libwayland-client`, `libwayland-egl`; with `dlopen`, opened at run time through `dlib` | `dlib`, `log`, `once_cell`; `pkg-config` at build time, which links nothing with `dlopen` |
| `wayland-protocols` | 0.32.13 | MIT | Generated bindings of the protocol extensions: `xdg-shell`, `xdg-decoration`, `xdg-output`, `viewporter`, `fractional-scale`, `text-input-v3`, `xdg-foreign-v2`, `linux-dmabuf` (features `client`, `staging`, `unstable`) | `bitflags` |
| `wayland-cursor` | 0.31.14 | MIT | Cursor themes: the XCursor files read in Rust (`xcursor` 0.3, MIT), the images in `wl_shm` buffers | `rustix`, `xcursor` |
| `wayland-egl` | 0.32.11 | MIT | `wl_egl_window_create`, `_resize`, `_destroy` of `libwayland-egl` | |
| `wayland-protocols-wlr`, `wayland-protocols-misc` | 0.3.12 | MIT | Only for the example (feature `example`): the virtual pointer and screen copy protocols of wlroots, the virtual keyboard protocol | |

All verified with `cargo info` on 2026-10-10 and from the sources in the cargo registry **[V]**; minimum Rust 1.86 (`wayland-protocols`), the workspace is at 1.89.

**The backend is `libwayland-client`, not the pure Rust one, and EGL decides it.** `eglGetPlatformDisplay(EGL_PLATFORM_WAYLAND_KHR, ...)` takes a `wl_display*` and `wl_egl_window_create` a `wl_surface*` of the C library: the driver sends its own requests (buffers, `wl_surface.commit` inside `eglSwapBuffers`) over that connection. The pure Rust backend has no such pointers: `Backend::display_ptr` and `ObjectId::as_ptr` exist only with `client_system` **[V]** (`wayland-backend/src/sys/mod.rs`, `wayland-egl/src/lib.rs`). A second connection for EGL is no way out: a `wl_surface` belongs to one connection.

**Nothing is linked.** With the feature `dlopen`, `wayland-sys` opens `libwayland-client.so.0` (and `libwayland-egl.so.1`) with `dlopen` the first time a function is called; its build script asks `pkg-config` for nothing in that case **[V]** (`wayland-sys/build.rs`: "Do not link to anything"). A machine without the library gets `NoWaylandLib` from the connect, which is a failed probe: `use_wayland_with_fallback` then falls back. So the crate checks for a Linux target from the Mac **[M]** (`cargo check -p ferroui-wayland --target x86_64-unknown-linux-gnu`), and an application that carries the Wayland backend still starts on a system that has only X11.

The alternatives, and why not:

| Alternative | Why not |
|---|---|
| The pure Rust backend of `wayland-backend` | No `wl_display*` for EGL (above). It would leave software rendering only. |
| Upstream's approach: a generator of the port's own, from the XML files | NWayland is a third-party package, not part of the framework; `wayland-scanner` is the same generator for Rust, maintained with the protocol. The XML files would have to be vendored. |
| `wayland-sys` alone (the C API by hand) | Every request and event marshalled by hand, with `unsafe` at each: what the generated proxies exist to avoid. |
| `smithay-client-toolkit` | A toolkit over the same crates with its own state objects for seats, outputs, shells and data devices: the behaviour to port is upstream's own handling of exactly those, including what is kept across a reconnect. |
| `libwayland-cursor` (upstream) instead of `wayland-cursor` | One more library to open; the crate reads the same theme files and gives buffers of the connection. The theme is asked for by upstream's name and size (`default`, 24) rather than through the crate's reading of `XCURSOR_THEME` (DEVIATIONS.md). |

How the model of the bindings differs from upstream's, and what the port does about it. NWayland gives every object a listener object at creation. `wayland-client` dispatches every event to one state value through `Dispatch<Interface, UserData>` implementations, with a user data value per object. The state value of the port is the worker's state (`WaylandWorkerState`); the user data of an object is the key of the object it belongs to (the number of a surface, the registry name of a seat or an output), and the `Dispatch` implementation is the port of the listener class: it finds the object by the key and calls the method the listener called. Like upstream's listeners given at creation, user data is given at creation, so no event is lost between creation and attachment.

### 2.2 The keyboard: `xkbcommon-dl`

**`xkbcommon-dl` 0.4.2** (MIT; `bitflags`, `dlib`, `log`, `once_cell`, `xkeysym`) **[V]**: the C declarations of `libxkbcommon.so.0`, opened at run time, with the compose functions; it is what `winit` uses. The 20 functions and the constants of upstream's `XkbCommonNativeMethods.cs` are a subset of it, so that file is not ported: its declarations are those of the crate, as the structures of `X11Structs.cs` are those of `x11-dl`. The four small classes over it (`XkbContext`, `XkbCommonKeymap`, `XkbComposeTable`, `XkbComposeState`) are ported and are where the `unsafe` of the library is: each owns one pointer and frees it once.

| Alternative | Why not |
|---|---|
| `xkbcommon` 0.9 | Links `libxkbcommon` at build time: no check from another system, and a link-time dependency of every application that carries the backend. |
| A function table of the port's own (`native_functions!` of the X11 crate) | Possible, and what upstream's file is; the crate is the same table, already written and used widely. |

A system without `libxkbcommon` has no keymap: keys are then translated from their evdev codes alone, which is upstream's fallback for a keymap that cannot be compiled.

### 2.3 The rest

- **EGL**: `libEGL.so.1` opened with `dlopen`, entry points through `eglGetProcAddress` first (extension entry points such as `eglGetPlatformDisplayEXT` are not exported symbols) and `dlsym` after it; the `EglDisplay`, `EglContext` and `EglPlatformSurfaceRenderTargetBase` of the OpenGL crate, as the X11 backend.
- **The C library** (`libc`, already a dependency of the X11 crate): `pipe2`, `poll`, `read`, `write`, `close` for the wake-up descriptor and the wait; `memfd_create`, `ftruncate`, `mmap`, `munmap` for a frame and for a keymap. One module (`server/interop/unsafe_native_methods.rs`) wraps them in safe functions over owned descriptors and mappings.
- **GBM and DRM** (`DrmGbmUnsafeNativeMethods.cs`) belong to the dmabuf swapchain, stage 3.
- Skia: nothing new. The feature set of the Skia backend for Linux is `gl` alone (`x11-platform.md`, section 4); the backend gives Skia the entry points of the context the platform made.

## 3. Crate

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-wayland` | `src/FerroUI.Wayland` | `Avalonia.Wayland` | `AppBuilder::use_wayland()`, `AppBuilder::use_wayland_with_fallback()` (`FerroWaylandPlatformExtensions`) |

It depends on `ferroui-base`, `ferroui-controls`, `ferroui-dialogs`, `ferroui-freedesktop`, `ferroui-opengl`, and on **`ferroui-x11`** for the files upstream compiles into both projects from the X11 one: `RawEventGrouping.cs` (`raw_event_grouping.rs`), `X11IconLoader.cs`, `UriListHelper.cs`, `Interop/Glib.cs` and `Dispatching/GlibDispatcherImplBase.cs`. They are public modules of the X11 crate already, the X11 crate links nothing (its libraries are opened at run time), and an application that uses Wayland with a fallback has it anyway; moving the five files into a third crate would touch the X11 crate for no gain now.

**`use_platform_detect` is unchanged**: upstream's platform detection chooses X11 on Linux, and an application opts into Wayland with `UseWaylandWithFallback()` after it **[V]** (`AvaloniaWaylandPlatformExtensions.cs`; its ControlCatalog does exactly that). `ferroui-desktop` does not depend on the Wayland crate.

The part of the crate that talks to a compositor is compiled for Linux only (`cfg(target_os = "linux")`: upstream runs on Linux only, and a frame needs `memfd_create`). What is plain logic compiles and is tested on every host: the key tables and the key resolution over a keymap trait, the states of a configure, the window geometry and the size limits, the scale of a surface, the logical geometry of an output, the frames of pointer and touch events, the wake-up flag of the render loop, the options.

The example (`examples/wayland_window.rs`) needs a renderer, a text shaper, a theme and the protocols of a test client, behind the feature `example`, like the X11 example.

## 4. Threads and the event loop

### 4.1 The worker

`WaylandWorker` (`server/wayland_worker.rs`) is upstream's: a thread named for the backend that owns the connection, dispatches its events, runs the commands of the UI thread and ticks the render loop. One iteration (`RunConnection`):

1. `dispatch_queue_or_wakeup` (`server/interop/wayland_connection.rs`): `prepare_read` on the queue (if events are already queued, dispatch them and return); flush the requests (`EAGAIN` is not an error; `EPIPE` and `ECONNRESET` are a lost connection; `EPROTO` a protocol error, which is fatal); `poll` on the descriptor of the connection and on the reading end of the wake-up pipe, without a timeout, again after `EINTR`; if only the pipe woke, cancel the read and return "wake-up"; else read the events and dispatch the queue.
2. Clear the wake-up flag if the pipe woke.
3. Run the queued out-of-band commands.
4. Tick the render loop if a wake-up of it is pending: every registered task renders (the compositor of the framework: commands that came with the batch, then the composition targets). A surface with a frame callback pending is "not ready", so its target is skipped until the callback arrives, which wakes the loop.

`wayland-client` has the same calls as the C API upstream uses: `EventQueue::prepare_read` (a guard whose drop is `wl_display_cancel_read`), `ReadEventsGuard::read`, `EventQueue::dispatch_pending`, `Connection::flush`, `EventQueue::roundtrip` **[V]**. The events of a queue the default queue never sees, because every object is created with the handle of the worker's queue.

Unlike the X11 event loop (`x11-platform.md`, section 5), the UI dispatcher has nothing to do with the connection. It is the `ManagedDispatcherImpl` of the base library (a condition variable and a clock; what upstream uses), or the dispatcher over the main loop of GLib with the option `use_g_lib_main_loop` (`GlibDispatcherImplBase` of the X11 crate with no source of its own: upstream's `WaylandGlibDispatcher` is an empty subclass). The hazard recorded for X11 (events queued in the library while the socket is empty) exists on the worker and is what `prepare_read` returning "nothing to read, dispatch" handles.

### 4.2 What crosses between the threads

"Values and handles cross, objects stay" (`render-thread.md`):

- **UI thread to worker**: a command is a closure that is `Send` and receives the worker's state (`WaylandWorkerThread`): `post_oob` puts it in a queue under a lock and sets the wake-up descriptor; `post_with_commit` gives it to the compositor as a server job (`Compositor::post_server_job`), so it runs on the worker when the batch it was posted with is rendered. The generated proxies of upstream (`WSurfaceProxy`, `WXdgTopLevelProxy`, `WaylandCursorProxy`: its C# source generator `[GenerateCrossThreadProxy]`) are written by hand: a proxy is the number of the worker's object and the marshaller, and each method posts a closure that looks the object up by its number and calls the method. An object that is gone makes the call a no-op, which is what a call on a disconnected object is upstream.
- **Worker to UI thread**: an event sink proxy (`WXdgTopLevelEventSinkProxy`, `WaylandOutputsSinkProxy`) holds the sink of the UI thread behind `ThreadBound` and the dispatcher, and each method posts a job (`Dispatcher::post`, default priority) with the arguments as values; the job reaches the sink on the UI thread. Raw input events are built there, from the values, because an input event holds its device and its root.
- **Blocking waits of the UI thread**, both upstream's: `TryInitialize` waits for the worker to have bound its globals (or to have failed), and the first show of a window waits for the first configure. Each is a channel the worker sends one value on.
- **Shared by contract**: the platform graphics (`WaylandPlatformGraphics`: `IPlatformGraphics`, with the ready state as atomics; the display of EGL stays on the worker), the render surfaces of a window (the number of the worker's surface and nothing else; `IPlatformRenderSurface` is `Send + Sync` in the port, and its render target is an object of the thread that renders, which is the worker), the render loop (`IRenderLoop`).

The worker's state lives in a value of the worker thread (`thread_local`), because the render targets are called by the compositor during a tick, without a reference to the worker: a render target reaches its surface through that value by number. The state is not borrowed while the render loop ticks.

### 4.3 The compositor of the framework

Upstream constructs `new Compositor(_renderLoop, PlatformGraphics)` in the worker's constructor. The port has that constructor as `Compositor::with_render_thread(render_loop, gpu, false, ...)`: a loop that runs in the background and no synchronous commits on the UI thread make the server compositor an object of the thread that ticks, which is the worker (`render-thread.md`, R5). The compositor itself is an object of the UI thread (`Rc`), so the worker holds what it needs of it as handles: the render loop tasks, and the locked server compositor for the two calls of a reconnect (`reset_all_gpu_resources`, `invalidate_all_composition_targets`).

### 4.4 The render timer

`server/wayland_worker_render_timer.rs` is upstream's `WaylandWorker.RenderTimer.cs`:

- `RenderLoopImpl` (`IRenderLoop`, runs in the background): a list of tasks under a lock; `wakeup` sets "tick pending" and the wake-up descriptor; `do_tick` renders every task once, never re-entered.
- A frame callback (`wl_callback.done` of the callback requested before a buffer is attached) and a sealed configure wake the loop.
- After every commit of the UI thread's compositor (`Compositor::after_commit`): if commands were posted with it, the loop is woken at once; and the starvation timer is started: if no tick happened within a twentieth of a second, the loop is woken from the timer's thread (through the out-of-band queue). This is what keeps animations of the UI thread alive while no surface is visible.

`IRenderLoopTask::render` of the port returns whether the task wants another tick; the tick of a Wayland surface comes from its frame callback, and a task that wants more without one is covered by the starvation timer, as upstream.

## 5. Rendering

| Mode | Upstream | Stage | Notes |
|---|---|---|---|
| Software, `wl_shm` | `WaylandFramebuffer.cs` | 1 | A frame is a file of memory (`memfd_create`, sized, mapped) that Skia draws into (BGRA, premultiplied; the format `argb8888`); when the lock is released: unmap, `wl_shm.create_pool`, one `wl_buffer` over the whole pool, the pool destroyed, the per-frame state staged (`on_before_new_buffer_attached`: `ack_configure`, the window geometry, the buffer scale or the viewport destination, the frame callback), `attach`, `damage_buffer`, `commit`. The buffer is destroyed when the compositor releases it. Always in the list of a surface, after the GPU surface. |
| EGL, `wl_egl_window` | `WaylandEglWsiPlatformGraphics.cs`, `WaylandEglWsiSurface.cs`, `WaylandEglNativeMethods.cs` | 1 | The display through `EGL_PLATFORM_WAYLAND_KHR` and the `wl_display*`; contexts of the profiles of the options; a `wl_egl_window` per render target, resized when the scene size changes; swap interval 0 and the per-frame state staged immediately before `eglSwapBuffers`, which commits. No waits (`SkipWaits`). Tried at start; when it cannot be created, software alone. |
| EGL, dmabuf swapchain | `WaylandEglDmaBuf*.cs`, `WaylandDmabufFeedback.cs`, `WaylandEglDisplay.cs`, `DrmGbmUnsafeNativeMethods.cs` | 3 | Only with `use_dmabuf_swapchain`; buffers allocated with GBM. |

Upstream has no option that turns the GPU off. An empty list of profiles (`gl_profiles`) makes the creation of the display fail, which leaves software rendering: that is how the smoke run renders in software, and it is upstream's behaviour, not an addition.

## 6. Scale, screens

- A surface's scale (`WSurface.RecomputeScale`): the preferred fractional scale (`wp_fractional_scale_v1`, in 120ths) when the compositor has fractional scaling and a viewporter; else `preferred_buffer_scale` (`wl_surface` 6); else the largest scale of the outputs the surface is on. With fractional scaling the buffer has the scaled size and the viewport destination is the logical size; without, `set_buffer_scale` gets the scale rounded up. The fractional path belongs to stage 2 with its two protocols; stage 1 has the other two sources.
- Screens (`screens/`): the worker tracks `wl_output` globals (`WaylandOutputsTracker`): an output is visible after its first `done`; with `xdg_output` its logical position and size (except where the size equals the mode with a scale above one, a compositor bug upstream works around), else the mode divided by the scale. Every change sends a snapshot of all outputs to the UI thread, where `SnapshotScreensImpl` is a `ScreensBase` keyed by the identity of an output. The scaling of a screen is always 1: scaling belongs to surfaces. The screen of a window is the last output its surface entered.

## 7. Windows

`WindowImpl` over `WindowBaseImpl` (UI thread) and `WXdgTopLevel` over `WXdgShellSurface` over `WSurface` (worker):

- **Show** creates the worker's surface and waits for its first configure; a compositor that gives no size gets 640 by 480 limited by the bounds. The serial is acknowledged with the first buffer. Hide and dispose destroy the surface.
- **Configure**: the size and states of `xdg_toplevel.configure` and the bounds are collected until `xdg_surface.configure` seals them as a batch, which goes to the UI thread: window state (fullscreen, maximized, normal), activation, the size (plus the shadow extents of drawn decorations), the size to restore. The UI thread answers with the serial to acknowledge on the next commit.
- **Requests**: title, parent, minimum and maximum size (clamped so that the minimum never exceeds the maximum, which is a protocol error), maximize, fullscreen, minimize, interactive move and resize with the serial of the press (a cookie that is valid once and for the connection it came from).
- **Decorations**: server-side is asked for when the decoration manager exists; the answer, or its absence, decides whether the framework is asked to draw the title bar, the border, the resize grips and the shadow (`requested_drawn_decorations`). Presses on drawn chrome start a move or a resize in the compositor. Once the application asks for less than full decorations the decoration object is destroyed for good (a limit of version 1 of the protocol).
- **Not on Wayland** (upstream's empty members): position, activation by the client, topmost, the icon, the task bar.

One addition: `xdg_toplevel.set_app_id`. Upstream never sets it, so a compositor cannot match the window to a desktop entry, and a test cannot find the window by it. `WaylandPlatformOptions::app_id` (default: the file name of the executable) is sent when the top-level is created (DEVIATIONS.md).

## 8. Input

- **Pointer** (`WaylandInputDispatcher.PointerHandler`): the events of a `wl_pointer` are collected and handed out at `frame`, in their order; the axis events of a frame are combined into one wheel event at the place of the first (steps from `axis_value120` or `axis_discrete` when present, else the continuous value over 10); the buttons `BTN_LEFT` to `BTN_EXTRA`; the keyboard modifiers of the seat are added. The cursor is set at `enter` and when the focused surface changes its cursor.
- **Touch**: the same framing; a contact remembers the surface it went down on.
- **Keyboard** (`WaylandInputDispatcher.Keyboard.cs`, `XkbKeyTransform.cs`): the keymap of the compositor compiled by `libxkbcommon`; the physical key from the evdev code; the key from the key symbol of the layout, digits always from the physical key, other layouts tried for a symbol that maps to no key, then the QWERTY key; the text of the key (control characters filtered); compose sequences when the focused surface has a text input client. Modifiers from the mask of `wl_keyboard.modifiers`. **Repeat is the client's on Wayland**: the UI thread repeats the last key with a dispatcher timer from the rate and delay of `repeat_info`.
- Raw input is queued and grouped (`raw_event_grouping.rs` of the X11 crate, with the queue that empties itself through the dispatcher).

## 9. Cursors

`WaylandCursorManager` loads the theme `default` at size 24 and, for each standard cursor, the first of upstream's names the theme has; each gets a surface with the first image attached. `WaylandBitmapCursor` is a persistent object: its pixels (copied on the UI thread as BGRA) are drawn into a `wl_shm` buffer on every connection. A window's cursor is a proxy the worker resolves at `enter`.

## 10. File table

Stage 1 is what this document's first implementation builds; 2 and 3 are the hand-over (section 12). "Shared" means the port is a module of another crate.

| Upstream file | Port | Stage |
|---|---|---|
| `AvaloniaWaylandException.cs` | `wayland_exception.rs` | 1 |
| `AvaloniaWaylandPlatformExtensions.cs` | `wayland_platform_extensions.rs` | 1 |
| `WaylandPlatform.cs` | `wayland_platform.rs` | 1 (the clipboard and the drag source bind in 2) |
| `WaylandPlatformOptions.cs` | `wayland_platform_options.rs` | 1 |
| `WaylandTopLevelFactory.cs` | `wayland_top_level_factory.rs` | 1 |
| `WaylandSurfaceCreateResult.cs` | `wayland_surface_create_result.rs` | 1 |
| `WaylandGlibDispatcher.cs` | `wayland_glib_dispatcher.rs` | 1 |
| `WaylandCursorFactory.cs` | `wayland_cursor_factory.rs` | 1 |
| `WindowImplBase.cs`, `.Pointer.cs`, `.Keyboard.cs` | `window_impl_base.rs`, `window_impl_base_pointer.rs`, `window_impl_base_keyboard.rs` | 1 |
| `WindowImplBase.DragDrop.cs` | `window_impl_base_drag_drop.rs` | 2 |
| `WindowImpl.cs`, `WindowImpl.Sink.cs` | `window_impl.rs`, `window_impl_sink.rs` | 1 (the storage provider chain and the text input method in 2) |
| `WindowImpl.TextInput.cs`, `TextInputOptionsConverter.cs`, `WaylandTextUtils.cs` | `window_impl_text_input.rs`, `text_input_options_converter.rs`, `wayland_text_utils.rs` | 2 |
| `PopupImpl.cs`, `PopupImpl.Sink.cs`, `WaylandConversionExtensions.cs` | `popup_impl.rs`, `popup_impl_sink.rs`, `wayland_conversion_extensions.rs` | 2 |
| `IWaylandXdgTopLevelExport.cs` | `i_wayland_xdg_top_level_export.rs` | 2 |
| `XkbContext.cs`, `XkbCommonKeymap.cs`, `XkbComposeTable.cs`, `XkbComposeState.cs` | `xkb_context.rs`, `xkb_common_keymap.rs`, `xkb_compose_table.rs`, `xkb_compose_state.rs` | 1 |
| `XkbKeyTransform.cs` | `xkb_key_transform.rs` | 1 |
| `Screens/IWaylandOutputsSink.cs`, `SnapshotScreensImpl.cs`, `WaylandOutputSnapshot.cs` | `screens/i_wayland_outputs_sink.rs`, `snapshot_screens_impl.rs`, `wayland_output_snapshot.rs` | 1 |
| `Clipboard/WaylandClipboardImpl.cs`, `WaylandDataTransfer.cs`, `WaylandDragSource.cs`, `WaylandMimeMapper.cs`, `WaylandOutgoingTransfer.cs` | `clipboard/*` | 2 |
| `Server/ServerSignaler.cs`, `WaylandDispatchPriority.cs`, `WaylandMarshallers.cs` | `server/server_signaler.rs`, `wayland_dispatch_priority.rs`, `wayland_marshallers.rs` | 1 |
| `Server/WaylandPlatformGraphics.cs` | `server/wayland_platform_graphics.rs` | 1 |
| `Server/WaylandWorker.cs`, `WaylandWorker.RenderTimer.cs`, `WaylandWorkerClient.cs` | `server/wayland_worker.rs`, `wayland_worker_render_timer.rs`, `wayland_worker_client.rs` | 1 (the protocol tracer of the worker in 2: the bindings have no hook for one; `WAYLAND_DEBUG=1` of the library prints the same) |
| `Server/Interop/WaylandConnection.cs`, `WakeupFd.cs`, `UnsafeNativeMethods.cs` | `server/interop/wayland_connection.rs`, `wakeup_fd.rs`, `unsafe_native_methods.rs` | 1 |
| `Server/Interop/WaylandEglNativeMethods.cs` | not ported: the declarations are those of `wayland-egl` | 1 |
| `Server/Interop/XkbCommonNativeMethods.cs` | not ported: the declarations are those of `xkbcommon-dl` | 1 |
| `Server/Interop/Pipe2Stream.cs` | `server/interop/pipe2_stream.rs` | 2 (the pipes of a data transfer) |
| `Server/Interop/DrmGbmUnsafeNativeMethods.cs` | `server/interop/drm_gbm.rs` | 3 |
| `Server/Persistent/IPersistentObject.cs`, `IWSurface.cs`, `IWSurfaceEventSink.cs`, `IWXdgTopLevel.cs` | `server/persistent/i_persistent_object.rs`, `i_w_surface.rs`, `i_w_surface_event_sink.rs`, `i_w_xdg_top_level.rs` (each with the proxy upstream generates) | 1 (the members of text input, drag and drop, popups and the export in 2) |
| `Server/Persistent/WSurface.cs` | `server/persistent/w_surface.rs` (`WSurface`, `WXdgShellSurface`, `WXdgTopLevel`) | 1 (`WXdgPopup`, the fractional scale and the viewport in 2) |
| `Server/Persistent/XdgConfigureBatch.cs`, `DecorationMode.cs`, `WaylandInputEventCookie.cs` | `server/persistent/xdg_configure_batch.rs`, `decoration_mode.rs`, `wayland_input_event_cookie.rs` | 1 |
| `Server/Persistent/IWaylandCursor.cs`, `WaylandCursor.cs`, `WaylandBitmapCursor.cs` | `server/persistent/i_wayland_cursor.rs`, `wayland_cursor.rs`, `wayland_bitmap_cursor.rs` | 1 |
| `Server/Persistent/XdgPopupConfigureBatch.cs`, `XdgPopupPositionerParams.cs` | `server/persistent/xdg_popup_*.rs` | 2 |
| `Server/Transient/WaylandGlobals.cs` | `server/transient/wayland_globals.rs` | 1 (the globals of stage 2 bound in 2) |
| `Server/Transient/WaylandOutputsTracker.cs` | `server/transient/wayland_outputs_tracker.rs` | 1 |
| `Server/Transient/WaylandInputDispatcher.cs`, `.Keyboard.cs` | `server/transient/wayland_input_dispatcher.rs`, `wayland_input_dispatcher_keyboard.rs` | 1 |
| `Server/Transient/WaylandCursorManager.cs` | `server/transient/wayland_cursor_manager.rs` | 1 |
| `Server/Transient/Rendering/IWaylandFramebufferSurface.cs`, `WaylandFramebuffer.cs` | `server/transient/rendering/i_wayland_framebuffer_surface.rs`, `wayland_framebuffer.rs` | 1 |
| `Server/Transient/Rendering/WaylandEglWsiPlatformGraphics.cs`, `WaylandEglWsiSurface.cs` | `server/transient/rendering/wayland_egl_wsi_platform_graphics.rs`, `wayland_egl_wsi_surface.rs` | 1 |
| `Server/Transient/Rendering/WaylandEglDisplay.cs`, `WaylandEglDmaBufPlatformGraphics.cs`, `WaylandEglDmaBufSurface.cs`, `WaylandDmabufFeedback.cs` | `server/transient/rendering/*` | 3 |
| `Server/Transient/WaylandTextInputV3.cs`, `IWaylandTextInputV3Events.cs` | `server/transient/wayland_text_input_v3.rs`, `i_wayland_text_input_v3_events.rs` | 2 |
| `Server/Transient/XdgToplevelExport.cs` | `server/transient/xdg_toplevel_export.rs` | 2 |
| `Server/Transient/Clipboard/WaylandDataDevice.cs`, `WaylandDataOffer.cs`, `WaylandDataSource.cs`, `WaylandOfferCookie.cs` | `server/transient/clipboard/*` | 2 |
| `src/Shared/RawEventGrouping.cs`, `Avalonia.X11/X11IconLoader.cs`, `Selections/UriListHelper.cs`, `Interop/Glib.cs`, `Dispatching/GlibDispatcherImplBase.cs` | shared: modules of `ferroui-x11` | built |
| `README.md` | this document | |

## 11. Verification

Three levels, as for X11, because a Wayland client cannot run on the development machine:

1. **Compilation for Linux** from the Mac: `cargo check -p ferroui-wayland --target x86_64-unknown-linux-gnu --all-targets` **[M]**.
2. **Tests without a compositor** (`cargo test -p ferroui-wayland`, on every host): the logic named in section 3.
3. **A compositor**: the CI job `wayland` (Ubuntu, x86-64) and the virtual machine, with the smoke mode of `examples/wayland_window.rs`: every check prints a line beginning `[ ok ]` or `[FAILED]`, and the exit code is the result.

### 11.1 The compositor of the tests: `sway` on the headless backend of wlroots

A Wayland client cannot ask the compositor about its own window, synthesize input, or read the screen through the core protocol. What a headless compositor offers beyond it decides which checks are possible:

| Compositor | Input injection | Reading the composed screen | Window state from outside |
|---|---|---|---|
| `weston --backend=headless` | Only through `weston-test`, a protocol of its test build that the distribution packages do not have **[R]** | The screenshooter, only with `--debug` **[R]** | None |
| `cage` (wlroots, a kiosk) | The virtual pointer and virtual keyboard protocols | `wlr-screencopy` | None; every window is forced full screen, so sizes and states say nothing |
| **`sway`** (wlroots) with `WLR_BACKENDS=headless` | `zwlr_virtual_pointer_v1` and `zwp_virtual_keyboard_v1`, for any client | `zwlr_screencopy_manager_v1`, for any client (what `grim` uses) | Its IPC (`swaymsg -t get_tree`): title, application identifier, geometry, focus, fullscreen; `swaymsg` changes the output (mode, scale) and closes a window |

So the job runs **`sway`**, with `WLR_BACKENDS=headless`, `WLR_LIBINPUT_NO_DEVICES=1` and `WLR_RENDERER=pixman` (no GPU on the runner), on a configuration file of the job: no bar, no borders, so the one tiled window covers the output exactly and a point of the window is the same point of the screen. The example is itself the test client: a second connection of its own binds the virtual pointer, the virtual keyboard (with a keymap it compiles with `libxkbcommon`) and the screen copy manager. That keeps the run to one process and three packages (`sway`, which brings `swaymsg`; the libraries; Mesa for EGL), and makes every check a comparison in one place. `wtype`, `wlrctl` and `grim` do the same from outside and are used by the catalog script only (`grim`).

Touch cannot be injected: wlroots has no virtual touch protocol. The touch handler is covered by the tests of its framing only.

### 11.2 The phases of the smoke mode

| Phase | What is done | What is asked of whom |
|---|---|---|
| platform | `use_wayland` with the options of the run | The platform: the windowing platform, the screens, the cursor factory and the render loop are the Wayland ones; with `--mode=egl` the worker has platform graphics, with `--mode=software` none |
| screens | Nothing | The screens of the platform against the outputs `swaymsg -t get_outputs` reports: number, name, logical geometry |
| window | A window with a title is shown | The compositor (`get_tree`): a view with that title and the application identifier, focused, of the size of the output; the framework: the client size is that size, the window is active, the render scaling is the scale of the output (`--expect-scale`) |
| frames | Nothing | The compositor (screen copy): the fill colour at three points of the output, and the colour of a marker rectangle at its place, which shows the frame is the right way up and at the right scale |
| input | The virtual pointer moves to two points, presses and releases the left button, turns the wheel both ways; the virtual keyboard presses and releases "a", then holds "b" | The input callback of the window: moves at the expected positions (in logical units), the button events there, wheel deltas of one step with the right sign, the key with its physical key and its symbol, the text "a", and repeated key-down events for "b" at the rate the compositor announced |
| cursor | The window is given the text cursor, then a bitmap cursor | The worker: both resolve to a surface; no protocol error follows (a round trip of the example's own) |
| resize | `swaymsg` gives the output another mode | The compositor: the view has the new size; the framework: `resized` was raised with it; screen copy: the fill colour near the new bottom right corner |
| state | The window asks for fullscreen and back | `window_state_changed` with fullscreen and with normal; the compositor reports the view fullscreen in between |
| limits, title | A new title; a minimum and maximum size | The compositor: the title; no protocol error |
| close | `swaymsg kill` | The closing callback runs, the window closes, the application ends with exit code 0 |

The job runs the smoke mode in software and through EGL (Mesa's `llvmpipe`, `LIBGL_ALWAYS_SOFTWARE=1`), each on an output of scale 1 and of scale 2.

Verified nowhere: a desktop compositor (GNOME's Mutter, KDE's KWin: fractional scaling, their decoration answers, their popups), a GPU with a hardware driver (the nvidia workarounds of the EGL surface), input from a real device, touch, a compositor restart.

## 12. Stages

| Stage | Content | What its run proves |
|---|---|---|
| 1 | The crate and bindings; `use_wayland`, `use_wayland_with_fallback`, the options; the worker, the connection, the globals; outputs as screens; the top-level; the render timer; software and EGL rendering; pointer, keyboard and touch; cursors; the example and the CI job | Section 11 |
| 1b | The ControlCatalog on Wayland when asked (`FERROUI_CATALOG_WAYLAND=1`), with pictures from `grim` | The script of the virtual machine: a picture per page |
| 2 | Popups (`xdg_popup` and the positioner), fractional scaling and the viewport, the clipboard and drag and drop (`wl_data_device`), text input (`text-input-v3`), the portal parent through `xdg-foreign`, the storage provider chain; the protocol tracer | The smoke mode grows a phase for each; the clipboard against `wl-copy` and `wl-paste` |
| 3 | The dmabuf swapchain (GBM, DRM, feedback) | A compositor with a render node: the virtual machine |

The catalog is opt-in in stage 1b where upstream's calls `UseWaylandWithFallback()` always: until stage 2 a popup of the framework has no Wayland counterpart and `create_popup` fails with a message that names the stage, so the catalog must not move to Wayland by itself on a Wayland session (DEVIATIONS.md).

## 13. Deviations

Recorded in `DEVIATIONS.md`, section "Wayland platform".
