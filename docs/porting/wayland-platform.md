# The Linux platform: Wayland

The design of the Wayland backend of FerroUI, the decisions it rests on, the file table of the port and its stages. Upstream: `src/Avalonia.Wayland` at the tracked commit (`TRACKING.md`): 84 source files (11 341 lines), a README and the project file; the tracker counts the 81 files that declare types, with 123 types and 816 members (`tracking/Avalonia.Wayland.md`), and `tests/Avalonia.Wayland.UnitTests` has 12 tests in 2 files. After stage 1 the tracker found 53 of the 81 files, 73 of the 123 types and 384 of the 816 members (47.1 %); after parts 1 to 3 of stage 2 (popups, the verification of the decorations, fractional scaling) 58 files, 80 types and 418 members (51.2 %); after part 4 (the clipboard and drag and drop) 68 files, 91 types and 509 members (62.4 %); after part 5 (input methods, the parent handle of a portal dialog) 74 files, 97 types and 542 members (66.4 %). After the triage of section 12.2, which waives with their reasons the declaration files the bindings replace and what is written differently, the page counts 74 of 79 files, 97 of 121 types and 542 of 780 members (88.6 %), and the 70 members that remain open are stage 3. After stage 3 (the dmabuf swapchain and a display another library opened) it counts 79 of 79 files, 108 of 121 types (13 waived) and 612 of 780 members (168 waived): nothing of the project is open. The backend shares the FreeDesktop crate with the X11 backend (`x11-platform.md`), and three source files of the X11 project, which upstream compiles into both.

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
| `xdg_wm_base` | 3 to 4 (the port: 2 to 4, sections 7.1 and 11.1) | yes | `xdg_surface`, `xdg_toplevel` (with `configure_bounds`, 4), `xdg_popup` with `reposition` (3), `xdg_positioner` |
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
- **GBM and DRM** (`DrmGbmUnsafeNativeMethods.cs`, stage 3): `libgbm.so.1` and `libdrm.so.2` opened at run time the first time the dmabuf swapchain asks for them (`server/interop/drm_gbm_unsafe_native_methods.rs`), through the loader of the base crate (`ferroui_base::platform::native_library`: `NativeLibrary` and the macro `native_functions!`; until 2026-10-15 it was the X11 crate's, exported for this). The declarations are the Wayland project's own, as upstream has them twice (`DrmGbmUnsafeNativeMethods.cs` here, `DrmBindings.cs` in the framebuffer project): the two projects use different functions of the libraries (buffer objects with modifiers and planes, the device of a device number and PRIME export here; surfaces, modes and page flips there). Section 5.1 has the swapchain and the shared loader.
- Skia: nothing new. The feature set of the Skia backend for Linux is `gl` alone (`x11-platform.md`, section 4); the backend gives Skia the entry points of the context the platform made.

## 3. Crate

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-wayland` | `src/FerroUI.Wayland` | `Avalonia.Wayland` | `AppBuilder::use_wayland()`, `AppBuilder::use_wayland_with_fallback()` (`FerroWaylandPlatformExtensions`) |

It depends on `ferroui-base`, `ferroui-controls`, `ferroui-dialogs`, `ferroui-freedesktop`, `ferroui-opengl`, and on **`ferroui-x11`** for the files upstream compiles into both projects from the X11 one: `RawEventGrouping.cs` (`raw_event_grouping.rs`), `X11IconLoader.cs`, `UriListHelper.cs`, `Interop/Glib.cs` and `Dispatching/GlibDispatcherImplBase.cs`. They are public modules of the X11 crate already, the X11 crate links nothing (its libraries are opened at run time), and an application that uses Wayland with a fallback has it anyway; moving the five files into a third crate would touch the X11 crate for no gain now.

**`use_platform_detect` is unchanged**: upstream's platform detection chooses X11 on Linux, and an application opts into Wayland with `UseWaylandWithFallback()` after it **[V]** (`AvaloniaWaylandPlatformExtensions.cs`; its ControlCatalog does exactly that). `ferroui-desktop` does not depend on the Wayland crate.

The part of the crate that talks to a compositor is compiled for Linux only (`cfg(target_os = "linux")`: upstream runs on Linux only, and a frame needs `memfd_create`). What is plain logic compiles and is tested on every host: the key tables and the key resolution over a keymap trait, the states of a configure, the window geometry and the size limits, the scale of a surface, the logical geometry of an output, the frames of pointer and touch events, the wake-up flag of the render loop, the options.

The example (`examples/wayland_window.rs`) needs a renderer, a text shaper, a theme and the protocols of a test client, behind the feature `example`, like the X11 example. A second feature, `example-check`, is `example` without the renderer, the text shaper and the theme, whose native libraries cannot be built for Linux on another system: with it the example is type-checked from the Mac (it does not run built that way).

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

### 4.4 A display another library opened

`WaylandWorker.Start(options, WlDisplay foreignDisplay, outputsSink)` (stage 3) runs the backend on a display that another toolkit or library opened and owns. Upstream has no caller of it at the tracked commit; it is a member of the worker for an embedder. The port: `WaylandWorker::start_with_foreign_display(options, foreign_display, outputs_sink)` takes the display as a `wayland_client::Connection`; `WaylandConnection::from_foreign_display_ptr` makes one of a `wl_display*` (`Backend::from_foreign_display`: a backend in "guest" mode, which never closes the display), and `WaylandConnection::from_foreign_display` is the constructor upstream has (`_ownsDisplay` false, exposed as `owns_display`). The objects of the backend are on a queue of their own, as always (section 1), so its events and those of the owner do not mix. As upstream: nothing waits for the globals, there are no reconnects, and a lost display (or globals that cannot be bound) ends the process (the fatal path of the worker). The smoke mode checks it (the phase `foreign`, section 11.2).

### 4.5 The render timer

`server/wayland_worker_render_timer.rs` is upstream's `WaylandWorker.RenderTimer.cs`:

- `RenderLoopImpl` (`IRenderLoop`, runs in the background): a list of tasks under a lock; `wakeup` sets "tick pending" and the wake-up descriptor; `do_tick` renders every task once, never re-entered.
- A frame callback (`wl_callback.done` of the callback requested before a buffer is attached) and a sealed configure wake the loop.
- After every commit of the UI thread's compositor (`Compositor::after_commit`): if commands were posted with it, the loop is woken at once; and the starvation timer is started: if no tick happened within a twentieth of a second, the loop is woken from the timer's thread (through the out-of-band queue). This is what keeps animations of the UI thread alive while no surface is visible.

`IRenderLoopTask::render` of the port returns whether the task wants another tick; the tick of a Wayland surface comes from its frame callback, and a task that wants more without one is covered by the starvation timer, as upstream.

## 5. Rendering

`WaylandPlatformOptions::rendering_mode` selects presentation. The default `Auto` retains EGL and shared-memory fallback. `Software` selects shared memory. `Vulkan(options)` requires a device that can present to the connected compositor. Ferro Desktop selects Vulkan for Skia Graphite.

The Vulkan backend uses a temporary Wayland surface to test device presentation support. Each configured window or popup gets a swapchain through FerroUI.Vulkan on the same worker-owned connection. At presentation time the worker acknowledges the configure and sets the logical viewport, scale and frame callback before the driver commits the buffer. Loss of the native surface disposes its render target before destroying the Wayland object. See [Vulkan Wayland surface creation](https://docs.vulkan.org/refpages/latest/refpages/source/VK_KHR_wayland_surface.html).

| Mode | Upstream | Stage | Notes |
|---|---|---|---|
| Software, `wl_shm` | `WaylandFramebuffer.cs` | 1 | A frame is a file of memory (`memfd_create`, sized, mapped) that Skia draws into (BGRA, premultiplied; the format `argb8888`); when the lock is released: unmap, `wl_shm.create_pool`, one `wl_buffer` over the whole pool, the pool destroyed, the per-frame state staged (`on_before_new_buffer_attached`: `ack_configure`, the window geometry, the buffer scale or the viewport destination, the frame callback), `attach`, `damage_buffer`, `commit`. The buffer is destroyed when the compositor releases it. Always in the list of a surface, after the GPU surface. |
| EGL, `wl_egl_window` | `WaylandEglWsiPlatformGraphics.cs`, `WaylandEglWsiSurface.cs`, `WaylandEglNativeMethods.cs` | 1 | The display through `EGL_PLATFORM_WAYLAND_KHR` and the `wl_display*`; contexts of the profiles of the options; a `wl_egl_window` per render target, resized when the scene size changes; swap interval 0 and the per-frame state staged immediately before `eglSwapBuffers`, which commits. No waits (`SkipWaits`). Tried at start; when it cannot be created, software alone. |
| EGL, dmabuf swapchain | `WaylandEglDmaBuf*.cs`, `WaylandDmabufFeedback.cs`, `WaylandEglDisplay.cs`, `DrmGbmUnsafeNativeMethods.cs` | 3 (built) | Only with `use_dmabuf_swapchain` and a compositor that has `zwp_linux_dmabuf_v1` version 4; buffers allocated with GBM on the render node of the compositor's main device (section 5.1). |

### 5.1 The dmabuf swapchain (stage 3)

The backend allocates the buffers and hands them to the compositor, instead of leaving it to the window system integration of the driver. Upstream's order, kept:

1. **Selection** (`WaylandGlobals`): `zwp_linux_dmabuf_v1` is bound at version 4 when announced. With `use_dmabuf_swapchain` and the global the backend is `WaylandEglDmaBufPlatformGraphics::try_create`; when that fails the worker renders in software (upstream gives no fallback to the window system integration there). Without the global the option is ignored and the backend is the `wl_egl_window` one; the port says so in a warning (`OpenGL`: "The dmabuf swapchain was requested, but the compositor has no zwp_linux_dmabuf_v1 ..."), which upstream does not (DEVIATIONS.md).
2. **Feedback** (`wayland_dmabuf_feedback.rs`): `get_default_feedback` and a round trip; the format table (a file of 16-byte entries, mapped privately), the main device, and the tranches (target device, the pairs by index into the table, the scan-out flag) until `done`. The listener is the user data of the feedback object, so its decoding is plain logic and tested on every host. Without `done` after the round trip: a warning, no swapchain.
3. **Device**: `drmGetDeviceFromDevId` of the main device; its render node (`DRM_NODE_RENDER`) is opened, `gbm_create_device` made on it, and an EGL display over the GBM device (`EGL_PLATFORM_GBM_KHR`) with the contexts of `gl_profiles`. `EGL_KHR_image_base` and `EGL_EXT_image_dma_buf_import` are required. The dispose callback of the display destroys the GBM device and closes the node.
4. **Formats**: the pairs of all tranches, kept where EGL lists the modifier for the format (`eglQueryDmaBufModifiersEXT`) or the modifier is the implicit one; without the query only implicit modifiers; nothing left gives `ARGB8888` with the implicit modifier.
5. **Frames** (`wayland_egl_dma_buf_surface.rs`): four slots per render target, made again when the size changes. A slot is a GBM buffer object (`gbm_bo_create_with_modifiers2` with the modifiers of `ARGB8888`, else `gbm_bo_create`), one PRIME descriptor per plane, an `EGLImage` over them, and a `wl_buffer` from `zwp_linux_buffer_params_v1.create`, answered within a round trip. A frame takes the free slot used least recently, draws into the image through a framebuffer object (the image render target of the OpenGL crate), and on finish stages the per-frame state, attaches the buffer, damages it, marks the slot busy and commits. `wl_buffer.release` frees the slot; a slot that is busy when the swapchain is made again is destroyed when it is released.

How the bindings shape it. The reference creates the parameters object on a temporary queue, takes the `wl_buffer` of `created` with a listener after a round trip of that queue, and moves the buffer to the main queue (`SetQueue`). `wayland-client` has no way to move an object to another queue, and a child object gets its object data when the event that creates it is read. So the parameters object is created with object data of its own (`DmaBufCreateParamsListener`, an `ObjectData` of the backend): the backend calls it when it reads `created`, it records the identity of the buffer and returns the object data of the worker's queue for it, with the release listener of the slot as user data. The round trip is one of the connection without a queue (`Connection::roundtrip`), which reads and hands such events over but dispatches no queue: the events of the worker's queue that arrive meanwhile wait for the worker's loop, as upstream's temporary queue keeps them out. The release listener holds the render target and the slot behind `ThreadBound` (user data has to be `Send`; both are objects of the worker thread, where the event is dispatched).

Which render surface draws. A window lists the surface of the window system integration, the dmabuf surface and the framebuffer surface (upstream asks the bound backend for its surface on the thread that renders). Each OpenGL surface answers as one only while its backend is the bound one (`WaylandEglDmaBufPlatformGraphics::is_current`), so the renderer, which takes the first OpenGL surface, takes the one of the bound backend.

**The shared loader (done on 2026-10-15).** Three places of the Linux backends opened a library at run time and resolved a table of functions: the X11 crate (`interop/native_library.rs`: `NativeLibrary` and `native_functions!`, for GLib, GTK, ICE and SM, and for GBM and DRM of the Wayland crate), the framebuffer crate (`native_library.rs`: a macro `native_library!` for libinput, libdrm and libgbm), and the EGL loaders (`load_egl_interface` of the Wayland crate, `x11_egl_helper.rs`, the EGL of the DRM output), which resolve through `eglGetProcAddress` first. The first two are now one loader in the base crate, `ferroui_base::platform::native_library` (`NativeLibrary`, `NativeLibraryError`, and the macro `ferroui_base::native_functions!`), and the X11, Wayland and framebuffer crates use it; the X11 module and the framebuffer macro are gone. Why the base crate: it is the lowest crate all three depend on (the only crate below it is the source scanner of the build scripts, a build dependency), so no crate is added to the published set and its order does not change; the framebuffer crate cannot depend on the X11 crate, and the OpenGL crate, which all three also reach, is about rendering, while GLib, GTK and the session libraries are not; upstream's counterpart is the runtime's own resolution of platform invokes, which every assembly has. The base crate gets `libc` as a dependency on Unix targets other than Emscripten (the module exists there only). It was a refactor without a change of behaviour: the X11 and Wayland libraries are opened as before (`RTLD_LAZY | RTLD_GLOBAL`, `NativeLibrary::open`), the framebuffer's with its flags (`RTLD_NOW | RTLD_LOCAL`, `NativeLibrary::open_with_flags`), and each crate keeps its messages: the error is an enumeration of the library and the entry point that cannot be had, whose text is the X11 crate's, and the framebuffer's `load_library` (`native_library.rs`) writes a missing function as before ("libdrm.so.2 does not export drmModeAddFB2"). The tests of the X11 module moved with it; the framebuffer's helper has two of its own (Linux only). The EGL loaders are not part of it: they resolve through the function of EGL first and are one function each.

Upstream has no option that turns the GPU off. An empty list of profiles (`gl_profiles`) makes the creation of the display fail, which leaves software rendering: that is how the smoke run renders in software, and it is upstream's behaviour, not an addition.

## 6. Scale, screens

- A surface's scale (`WSurface.RecomputeScale`): the preferred fractional scale (`wp_fractional_scale_v1`, in 120ths) when the compositor has fractional scaling and a viewporter; else `preferred_buffer_scale` (`wl_surface` 6); else the largest scale of the outputs the surface is on. With fractional scaling the buffer has the scaled size and the viewport destination is the logical size; without, `set_buffer_scale` gets the scale rounded up. All three sources are built (the fractional one with stage 2, part 3: `fractional_scale_manager` and `viewporter` of the globals, the fractional scale object and the viewport of a surface, created with it and destroyed before it). A preferred scale of zero is ignored. On a compositor that has both protocols every surface takes this path, also on an output of an integer scale: the buffer scale stays 1 and the viewport maps the buffer to the logical size. sway has both, so from part 3 on the runs under sway render through the viewport at every scale, and the integer path (`set_buffer_scale`) is what the runs of stage 1 measured.
- Screens (`screens/`): the worker tracks `wl_output` globals (`WaylandOutputsTracker`): an output is visible after its first `done`; with `xdg_output` its logical position and size (except where the size equals the mode with a scale above one, a compositor bug upstream works around), else the mode divided by the scale. Every change sends a snapshot of all outputs to the UI thread, where `SnapshotScreensImpl` is a `ScreensBase` keyed by the identity of an output. The scaling of a screen is always 1: scaling belongs to surfaces. The screen of a window is the last output its surface entered.

## 7. Windows

`WindowImpl` over `WindowBaseImpl` (UI thread) and `WXdgTopLevel` over `WXdgShellSurface` over `WSurface` (worker):

- **Show** creates the worker's surface and waits for its first configure; a compositor that gives no size gets 640 by 480 limited by the bounds. The serial is acknowledged with the first buffer. Hide and dispose destroy the surface.
- **Configure**: the size and states of `xdg_toplevel.configure` and the bounds are collected until `xdg_surface.configure` seals them as a batch, which goes to the UI thread: window state (fullscreen, maximized, normal), activation, the size (plus the shadow extents of drawn decorations), the size to restore. The UI thread answers with the serial to acknowledge on the next commit.
- **Requests**: title, parent, minimum and maximum size (clamped so that the minimum never exceeds the maximum, which is a protocol error), maximize, fullscreen, minimize, interactive move and resize with the serial of the press (a cookie that is valid once and for the connection it came from).
- **Decorations**: server-side is asked for when the decoration manager exists; the answer, or its absence, decides whether the framework is asked to draw the title bar, the border, the resize grips and the shadow (`requested_drawn_decorations`). Presses on drawn chrome start a move or a resize in the compositor. Once the application asks for less than full decorations the decoration object is destroyed for good (a limit of version 1 of the protocol).
- **Not on Wayland** (upstream's empty members): the position of a top-level (it is the origin of the "screen" space, its buffer-relative logical space; a popup reports its offset in it, section 7.1), activation by the client, topmost, the icon, the task bar.

One addition: `xdg_toplevel.set_app_id`. Upstream never sets it, so a compositor cannot match the window to a desktop entry, and a test cannot find the window by it. `WaylandPlatformOptions::app_id` (default: the file name of the executable) is sent when the top-level is created (DEVIATIONS.md).

### Desktop layers

`layer_shell::LayerWindow` gives a FerroUI window a background, bottom, top or overlay role on a named output. `LayerConfig` controls anchors, logical size, margins, exclusive zones and keyboard policy. The first configure establishes the available size before rendering. A panel can change its height or keyboard policy while visible. Layer changes require protocol version 2 and on-demand keyboard focus requires version 4. A named layer closes when the compositor removes its output. Callers can watch the existing screen-change notification to establish a new role on a surviving output. [Layer-shell protocol](https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-layer-shell-unstable-v1.xml)

The native Wayland example's layer smoke phase samples actual panel pixels and the working area of an independent xdg-shell client, promotes a layer to a keyboard overlay and exercises hide, remap and close. The [Linux desktop session](../../samples/LinuxDesktop/WAYLAND.md) uses these APIs for multi-monitor panels and notifications.

### 7.1 Popups

`PopupImpl` (UI thread) and `WXdgPopup` over `WXdgShellSurface` (worker), stage 2, part 1:

- **A popup is positioned by the compositor.** The framework gives the positioner of the popup (`IPopupImpl::popup_positioner`) its parameters; the popup translates the anchor, the gravity and the constraint adjustment into the values of the protocol (`wayland_conversion_extensions.rs`: the anchor and the gravity are bit fields in the framework and dense enumerations on the wire, the adjustment is translated flag by flag) and takes the size as its client size at once, so that the first frame has the right size before the compositor answers. The worker builds an `xdg_positioner` from them (`compute_positioner_geometry`): the size without the margin of the popup's child (the margin is the shadow of the popup, and becomes its window geometry), the anchor rectangle moved from the buffer of the parent into its window geometry and clamped into it so that it is never empty (an empty or outlying rectangle is a fatal protocol error), the offset when there is one. Halves round to even, as upstream's `Math.Round`.
- **The parent** is the surface the popup was created from: a top-level, or another popup for a nested one. The worker's popup names it by number. `xdg_surface.get_popup` needs a mapped parent, so a popup whose parent has no buffer yet is remembered by the parent and attached when the parent's first buffer is staged (`WaylandWorkerState::on_before_new_buffer_attached`, which is where the port has the loop upstream runs inside the parent object).
- **No grab** (`xdg_popup.grab` is never called), as upstream decides: a popup is closed by the light dismiss of the framework (a press in the parent window), by `popup_done` of the compositor (the popup is disposed), or with its parent. A popup therefore gets pointer and touch input that is over it and no keyboard focus; keys go to the top-level.
- **Configure**: the position and size of `xdg_popup.configure` are collected until `xdg_surface.configure` (`XdgPopupConfigureBatch::geometry`, none for a bare `xdg_surface.configure`); the UI thread acknowledges the serial with the next frame and ignores the suggested size (the size of the framework's positioner is the one that is laid out), as upstream.
- **Position** (upstream 8bbe90d2aa, #22291): a popup reports where it is. From the configured window geometry (relative to the window geometry of the parent), the origin of the parent's window geometry in its buffer (its shadow extents), the parent's own position and the popup's own origin (the margin of its positioner, kept when a positioner has none), the popup derives its buffer-relative position in the space of the top-level, raises `position_changed` when it changes and tells its own popups (`ChildPositionsInvalidated`, a list of handlers on `WindowBaseImpl`), since the compositor sends a child no configure when only its parent moved. A window whose shadow extents change tells its popups too. `point_to_client` and `point_to_screen` offset by the position.
- **Reposition.** With `xdg_wm_base` from version 3 new parameters are sent with `xdg_popup.reposition` and a token, as upstream. **Before version 3 the popup is created again**: its `xdg_popup` and `xdg_surface` are destroyed, a null buffer is committed (an `xdg_surface` cannot be made for a surface that has a buffer), and both objects are created again on the same `wl_surface` with a positioner of the new parameters; the surface then waits for its configure like a new one, and the batch of that configure is marked (`XdgPopupConfigureBatch::recreated`) so that the UI thread asks for a frame, which nothing else would when only the place changed. Parameters equal to the last ones do nothing on this path, and a popup that has an open popup of its own is left where it is with a warning (the topmost popup has to be destroyed first). This is what toolkits did before the request existed (GTK 3 hides and shows a popup to move it); the alternatives were to require version 3, which refuses sway 1.9 and with it the compositor of the tests and of Ubuntu 24.04, or to ignore new parameters, which leaves a combo box or a tool tip at a stale place. The cost is one frame without the popup. It is the path the job under sway 1.9 measures; `xdg_popup.reposition` itself runs in the informational job under sway 1.10.1 (section 11), which showed that the answer to it has to be followed by a frame: the compositor moves the popup when the configure is acknowledged and committed, so a frame is asked for after every configure of a popup.

### 7.2 Decorations as verified

Upstream has no client-side decoration code of its own in this project: it negotiates (`zxdg_toplevel_decoration_v1`, server-side asked for when the manager exists), tells the framework what to draw (`requested_drawn_decorations`: title bar, border, resize grips, shadow, when the compositor does not decorate), carries the shadow extents into the window geometry, and starts an interactive move or resize in the compositor when a press hits drawn chrome (`handle_surface_specific_dispatch`) or the framework asks (`begin_move_drag`, `begin_resize_drag`). All of that was ported with stage 1; part 2 of stage 2 is its verification (the decorations phase of the smoke mode, section 11.2). There is no window menu (`xdg_toplevel.show_window_menu`) at the tracked commit, and none in the port. `WaylandPlatformOptions::force_drawn_decorations` (upstream's test option) leaves the decoration manager unbound.

## 8. Input

- **Pointer** (`WaylandInputDispatcher.PointerHandler`): the events of a `wl_pointer` are collected and handed out at `frame`, in their order; the axis events of a frame are combined into one wheel event at the place of the first (steps from `axis_value120` or `axis_discrete` when present, else the continuous value over 10); the buttons `BTN_LEFT` to `BTN_EXTRA`; the keyboard modifiers of the seat are added. The cursor is set at `enter` and when the focused surface changes its cursor.
- **Touch**: the same framing; a contact remembers the surface it went down on.
- **Keyboard** (`WaylandInputDispatcher.Keyboard.cs`, `XkbKeyTransform.cs`): the keymap of the compositor compiled by `libxkbcommon`; the physical key from the evdev code; the key from the key symbol of the layout, digits always from the physical key, other layouts tried for a symbol that maps to no key, then the QWERTY key; the text of the key (control characters filtered); compose sequences when the focused surface has a text input client. Modifiers from the mask of `wl_keyboard.modifiers`. **Repeat is the client's on Wayland**: the UI thread repeats the last key with a dispatcher timer from the rate and delay of `repeat_info`.
- Raw input is queued and grouped (`raw_event_grouping.rs` of the X11 crate, with the queue that empties itself through the dispatcher).

### 8.1 The clipboard and drag and drop

Stage 2, part 4: `wl_data_device_manager` version 3, one `wl_data_device` per seat. Upstream binds no primary selection, and neither does the port.

**The worker half** (`server/transient/clipboard/`):

- `WaylandDataDevice` belongs to a seat (the seat makes it when the manager exists; the user data of the protocol object is the registry name of the seat). An offer the compositor announces is pending until `selection` or the `enter` of a drag takes it. The selection offer and the offer of a drag each get a `WaylandOfferCookie`, which is what the UI thread holds: it is invalidated when the selection is replaced, when the drag leaves, and when the device goes. A drop moves the offer and its cookie to a list of pending drops, which `finish_dnd` (posted by the cookie after the framework handled the drop) finishes in order: `wl_data_offer.finish`, then destroy.
- `WaylandDataOffer` collects the MIME types and the actions of the source. `receive` makes a pipe (`pipe2` with `O_CLOEXEC`), gives the writing end to `wl_data_offer.receive`, closes it here, and returns the reading end. A `wl_data_offer` is created by an event, so its user data cannot name the seat: the events of an offer look for it among the offers of every device.
- `WaylandDataSource` wraps a `wl_data_source` this client made, with callbacks for `send`, `cancelled`, `dnd_drop_performed`, `dnd_finished` and `action`. The sources of a connection are kept by the globals (`data_sources`, by the identity of the protocol object), which is where an event finds its source; the data device names the source it set as the selection by that identity. A callback receives the state of the worker, where upstream's closures capture the device and the globals.
- The seat records the serial of the last input event that can authorize `set_selection` (keyboard enter and key, pointer button, touch down). `start_drag` consumes the cookie of the press (as an interactive move does) and names the surface that has the pointer as the origin. During a drag this client started, `wl_data_source.action` sets the cursor of the pointer (copy, move, link for "ask", "no" otherwise), which is what Qt does and what KWin and Mutter honour; Weston does not (upstream's note).

**The UI half** (`clipboard/`, `window_impl_base_drag_drop.rs`):

- `WaylandClipboardImpl` (`IClipboardImpl`, `IOwnedClipboardImpl`): reading asks the worker for the cookie and the MIME types of the selection and builds a `WaylandDataTransfer`; setting makes a `WaylandOutgoingTransfer` and has the worker set its source as the selection; clearing sets an empty transfer (a source without a type); the owner question asks whether the device has an active source.
- `WaylandDataTransfer` implements both data transfer contracts over an offer: one item per file (the `text/uri-list` is read when the items are built) and one item for all other formats. **Where the pipes are read**: for the asynchronous contract on a thread of its own, awaited on the UI thread, so that the UI thread stays free (it has to be: when the owner of the selection is this process, its `send` is answered on the UI thread); for the synchronous contract (a drop handler reads during the event) on the calling thread, which is safe because a drag of this process never reads a pipe (below) and the worker never waits for the UI thread.
- `WaylandOutgoingTransfer`: the MIME types of the formats of the transfer (formats of the process are left out), a source with them, and `send` answered by a call posted to the UI thread: the data is taken from the transfer there and written to the pipe on a thread of its own, so that a slow reader does not hold the UI thread. A drag adds a sentinel MIME type (`application/x-ferroui-inproc-dnd-<instance>-<operation>`) and registers the transfer of the application under the operation key: a surface of this process that the drag enters finds the key among the offered types and gets the original transfer, not one that reads pipes its own UI thread would have to write.
- `WaylandMimeMapper`: text is `text/plain;charset=utf-8` and `text/plain`, files `text/uri-list`, a bitmap `image/png`, an application format `application/x-ferroui-<identifier>`, a platform format its own name; an unknown MIME type is a platform format of bytes.
- The sink of a surface (top-level and popup alike) gets `on_drag_enter`, `on_drag_motion`, `on_drag_leave`, `on_drop` and turns them into the raw drag events of the framework. A motion that arrives while the previous one is still queued updates the queued one. After the framework handled an enter or a motion, its answer (the effects) goes back through the cookie: `set_actions` with copy preferred, and `accept` of every offered type, or of none. After a drop the cookie posts the finish.

The result of a drag (`IPlatformDragSource::do_drag_drop_async`) is the action the compositor negotiated last when `dnd_finished` arrives, and no effect when the source is cancelled or the drag could not start. If the compositor was replaced between the press and the request, upstream's task never completes; the port's resolves to no effect (DEVIATIONS.md).

### 8.2 Input methods

Stage 2, part 5: `zwp_text_input_manager_v3` version 1, one `zwp_text_input_v3` per seat.

**The worker half** (`server/transient/wayland_text_input_v3.rs`) is upstream's `WaylandTextInputV3`: the text input object of every seat and, for every surface whose window has asked for a text input method, the state of its client. The rules, each of which a test pins:

- The compositor sends `enter` and `leave` with the keyboard focus (sway only while the seat has an input method). A surface is **enabled** when a seat has entered it and it has a client: `enable`, the content type, the cursor rectangle, the surrounding text with the change cause "other", and one `commit`. Whichever of the two comes second triggers it (the focus, or the client). Without a client the surface is disabled (`disable`, `commit`).
- **The enabling commit carries the state of the client, and this is where the port leaves upstream.** The protocol makes the state of that commit what the client supports for the session, and wlroots ignores a surrounding text, a content type or a cursor rectangle that first comes in a later one. Upstream enables as soon as the client is set, and the state follows in commits of its own: under sway the input method then hears of no text and its popup has no rectangle (measured, section 11). The port holds the enable back until the worker has run the commands of the UI thread (`flush`, after the queue and the tick of every iteration of the worker), so that the client and its state, which the UI thread posts together, are one commit; a kind of state that first arrives later enables again. For the same reason the text input method of a window sends the options of the focused element again after the client: the framework sets them before it, and the worker forgets the state of a surface when its client changes (DEVIATIONS.md).
- Every change of state while enabled (`set_content_type`, `set_cursor_rectangle`, `set_surrounding_text`) is sent with a `commit` of its own, and every `commit` counts: the count is the serial of the protocol.
- The events `preedit_string`, `commit_string` and `delete_surrounding_text` are pending until `done`, which applies them as one batch and resets them (so a `done` without a preedit clears the preedit). A `done` whose serial is not the count of the last commit is dropped with its batch, as the protocol demands; so is one of a seat that is not the last that entered the surface. A batch that changes nothing (no commit, nothing to delete, the preedit that was delivered last) is not delivered.
- The protocol counts in bytes of UTF-8, the framework in UTF-16 code units (`wayland_text_utils.rs`): the cursor of a preedit, the cursor and the anchor of the surrounding text, and the lengths of `delete_surrounding_text`, which are converted against the surrounding text that was sent last and dropped without one. The surrounding text is cut to 4000 bytes around the cursor.
- The cursor rectangle is in logical units of the surface, moved by the shadow extents of drawn decorations, rounded (halves to even), and never empty.
- A client that changes aborts the composition (`disable`, `commit`, the cached state forgotten); a reset is `disable`, `commit`, `enable` with the state, `commit`, and clears the preedit of the client; `leave` clears a delivered preedit.

It is written against three contracts (the requests of the protocol object, the manager, what it reads of a surface) so that all of the above runs without a compositor: 23 tests on every host, beside the 12 of upstream's two test files (`wayland_text_utils.rs`, `text_input_options_converter.rs`), ported with their cases, and one of the two helpers the port adds to the first.

**The UI half** (`window_impl_text_input.rs`) is the `ITextInputMethodImpl` of a top-level, which the framework asks for as a feature of the window (a popup has none: keys go to the top-level). It registers its sink with the surface, tells the worker whether there is a client and what it supports, with an epoch that every update of the worker carries back (an update for a previous client is dropped), pushes the surrounding text and the selection when the client says they changed, and applies an update in the order of the protocol: the preedit is cleared before a delete or a commit; a delete selects the range and sends a Backspace key through the input of the window (what the text box does with a selection); a commit is a raw text input event; then the new preedit, with its cursor. After hide and show the sink is registered again on the new surface.

Compose sequences (`XkbComposeState`) are fed only while the focused surface has a text input client, which the same call sets; they were built with stage 1 and never ran before this part.

### 8.3 The parent of a portal dialog

Stage 2, part 5: `zxdg_exporter_v2` version 1. The first storage provider of a window is the file chooser of the desktop portal (`DBusSystemDialog` of the FreeDesktop crate), which wants the window the dialog belongs to as `wayland:<handle>`. A lease (`IPortalParentLease`) exports the top-level (`zxdg_exporter_v2.export_toplevel`), waits for the handle the compositor answers with, and destroys the export when it is disposed. The factory probes with one lease when it is evaluated: on a compositor without the exporter, or one that never answers, it gives nothing and the managed dialogs take over (a portal dialog without a parent floats unparented, which upstream judges worse than the managed one). The exports of a top-level end with its connection, and the chain is evaluated again for a new surface.

There is no protocol tracer: upstream installs one on its bindings when the log area `WaylandProtocol` is enabled; the bindings of the port have no such hook, and `WAYLAND_DEBUG=1` prints the same from the library (DEVIATIONS.md).

Confirmed again at the tracked commit for this part (`git grep` over the project): no `xdg_activation`, no primary selection (`zwp_primary_selection` or `gtk_primary_selection`), no `wp_cursor_shape`, no pointer gestures, no `wp_presentation`; the only word "Activation" is a comment over the empty `Activate` of a window.

## 9. Cursors

`WaylandCursorManager` loads the theme `default` at size 24 and, for each standard cursor, the first of upstream's names the theme has; each gets a surface with the first image attached. `WaylandBitmapCursor` is a persistent object: its pixels (copied on the UI thread as BGRA) are drawn into a `wl_shm` buffer on every connection. A window's cursor is a proxy the worker resolves at `enter`.

## 10. File table

Stage 1 is what this document's first implementation builds; 2 and 3 are the hand-over (section 12). "Shared" means the port is a module of another crate.

| Upstream file | Port | Stage |
|---|---|---|
| `AvaloniaWaylandException.cs` | `ferro_wayland_exception.rs` | 1 |
| `AvaloniaWaylandPlatformExtensions.cs` | `ferro_wayland_platform_extensions.rs` | 1 |
| `WaylandPlatform.cs` | `wayland_platform.rs` | 1 (the clipboard and the drag source bind in 2) |
| `WaylandPlatformOptions.cs` | `wayland_platform_options.rs` | 1 |
| `WaylandTopLevelFactory.cs` | `wayland_top_level_factory.rs` | 1 |
| `WaylandSurfaceCreateResult.cs` | `wayland_surface_create_result.rs` | 1 |
| `WaylandGlibDispatcher.cs` | `wayland_glib_dispatcher.rs` | 1 |
| `WaylandCursorFactory.cs` | `wayland_cursor_factory.rs` | 1 |
| `WindowImplBase.cs`, `.Pointer.cs`, `.Keyboard.cs` | `window_impl_base.rs`, `window_impl_base_pointer.rs`, `window_impl_base_keyboard.rs` | 1 |
| `WindowImplBase.DragDrop.cs` | `window_impl_base_drag_drop.rs` | 2, part 4 (built) |
| `WindowImpl.cs`, `WindowImpl.Sink.cs` | `window_impl.rs`, `window_impl_sink.rs` | 1; the storage provider chain and the text input method in 2, part 5 (built) |
| `WindowImpl.TextInput.cs`, `TextInputOptionsConverter.cs`, `WaylandTextUtils.cs` | `window_impl_text_input.rs`, `text_input_options_converter.rs`, `wayland_text_utils.rs` | 2, part 5 (built) |
| `PopupImpl.cs`, `PopupImpl.Sink.cs`, `WaylandConversionExtensions.cs` | `popup_impl.rs`, `popup_impl_sink.rs`, `wayland_conversion_extensions.rs` | 2, part 1 (built) |
| `IWaylandXdgTopLevelExport.cs` | `i_wayland_xdg_top_level_export.rs` | 2, part 5 (built) |
| `XkbContext.cs`, `XkbCommonKeymap.cs`, `XkbComposeTable.cs`, `XkbComposeState.cs` | `xkb_context.rs`, `xkb_common_keymap.rs`, `xkb_compose_table.rs`, `xkb_compose_state.rs` | 1 |
| `XkbKeyTransform.cs` | `xkb_key_transform.rs` | 1 |
| `Screens/IWaylandOutputsSink.cs`, `SnapshotScreensImpl.cs`, `WaylandOutputSnapshot.cs` | `screens/i_wayland_outputs_sink.rs`, `snapshot_screens_impl.rs`, `wayland_output_snapshot.rs` | 1 |
| `Clipboard/WaylandClipboardImpl.cs`, `WaylandDataTransfer.cs`, `WaylandDragSource.cs`, `WaylandMimeMapper.cs`, `WaylandOutgoingTransfer.cs` | `clipboard/wayland_clipboard_impl.rs`, `wayland_data_transfer.rs`, `wayland_drag_source.rs`, `wayland_mime_mapper.rs`, `wayland_outgoing_transfer.rs` | 2, part 4 (built) |
| `Server/ServerSignaler.cs`, `WaylandDispatchPriority.cs`, `WaylandMarshallers.cs` | `server/server_signaler.rs`, `wayland_dispatch_priority.rs`, `wayland_marshallers.rs` | 1 |
| `Server/WaylandPlatformGraphics.cs` | `server/wayland_platform_graphics.rs` | 1 |
| `Server/WaylandWorker.cs`, `WaylandWorker.RenderTimer.cs`, `WaylandWorkerClient.cs` | `server/wayland_worker.rs`, `wayland_worker_render_timer.rs`, `wayland_worker_client.rs` | 1; the start on a foreign display in 3 (built) (the protocol tracer of the worker has no counterpart: the bindings have no hook for one; `WAYLAND_DEBUG=1` of the library prints the same) |
| `Server/Interop/WaylandConnection.cs`, `WakeupFd.cs`, `UnsafeNativeMethods.cs` | `server/interop/wayland_connection.rs`, `wakeup_fd.rs`, `unsafe_native_methods.rs` | 1; the constructor of a foreign display in 3 (built) |
| `Server/Interop/WaylandEglNativeMethods.cs` | not ported: the declarations are those of `wayland-egl` | 1 |
| `Server/Interop/XkbCommonNativeMethods.cs` | not ported: the declarations are those of `xkbcommon-dl` | 1 |
| `Server/Interop/Pipe2Stream.cs` | `server/interop/pipe2_stream.rs` | 2, part 4 (built: the pipes of a data transfer); with it `server/cross_thread_result.rs`, the task of `InvokeOobAsync` and of a read on another thread |
| `Server/Interop/DrmGbmUnsafeNativeMethods.cs` | `server/interop/drm_gbm_unsafe_native_methods.rs` | 3 (built) |
| `Server/Persistent/IPersistentObject.cs`, `IWSurface.cs`, `IWSurfaceEventSink.cs`, `IWXdgTopLevel.cs` | `server/persistent/i_persistent_object.rs`, `i_w_surface.rs`, `i_w_surface_event_sink.rs`, `i_w_xdg_top_level.rs` (each with the proxy upstream generates) | 1; `IWXdgPopup` and `IWXdgPopupEventSink` in 2, part 1 (built); the members of drag and drop in part 4, of text input and the export in part 5 (built) |
| `Server/Persistent/WSurface.cs` | `server/persistent/w_surface.rs` (`WSurface`, `WXdgShellSurface`, `WXdgTopLevel`, `WXdgPopup`) | 1; `WXdgPopup` in 2, part 1, the fractional scale and the viewport in 2, part 3 (built) |
| `Server/Persistent/XdgConfigureBatch.cs`, `DecorationMode.cs`, `WaylandInputEventCookie.cs` | `server/persistent/xdg_configure_batch.rs`, `decoration_mode.rs`, `wayland_input_event_cookie.rs` | 1 |
| `Server/Persistent/IWaylandCursor.cs`, `WaylandCursor.cs`, `WaylandBitmapCursor.cs` | `server/persistent/i_wayland_cursor.rs`, `wayland_cursor.rs`, `wayland_bitmap_cursor.rs` | 1 |
| `Server/Persistent/XdgPopupConfigureBatch.cs`, `XdgPopupPositionerParams.cs` | `server/persistent/xdg_popup_configure_batch.rs`, `xdg_popup_positioner_params.rs` | 2, part 1 (built) |
| `Server/Transient/WaylandGlobals.cs` | `server/transient/wayland_globals.rs` | 1; the fractional scale manager and the viewporter in 2, part 3; the data device manager in part 4; text input and the exporter in part 5; `linux-dmabuf` and the selection of the swapchain in 3 (built) |
| `Server/Transient/WaylandOutputsTracker.cs` | `server/transient/wayland_outputs_tracker.rs` | 1 |
| `Server/Transient/WaylandInputDispatcher.cs`, `.Keyboard.cs` | `server/transient/wayland_input_dispatcher.rs`, `wayland_input_dispatcher_keyboard.rs` | 1 |
| `Server/Transient/WaylandCursorManager.cs` | `server/transient/wayland_cursor_manager.rs` | 1 |
| `Server/Transient/Rendering/IWaylandFramebufferSurface.cs`, `WaylandFramebuffer.cs` | `server/transient/rendering/i_wayland_framebuffer_surface.rs`, `wayland_framebuffer.rs` | 1 |
| `Server/Transient/Rendering/WaylandEglWsiPlatformGraphics.cs`, `WaylandEglWsiSurface.cs` | `server/transient/rendering/wayland_egl_wsi_platform_graphics.rs`, `wayland_egl_wsi_surface.rs` | 1 |
| `Server/Transient/Rendering/WaylandEglDisplay.cs`, `WaylandEglDmaBufPlatformGraphics.cs`, `WaylandEglDmaBufSurface.cs`, `WaylandDmabufFeedback.cs` | `server/transient/rendering/wayland_egl_display.rs`, `wayland_egl_dma_buf_platform_graphics.rs`, `wayland_egl_dma_buf_surface.rs`, `wayland_dmabuf_feedback.rs` | 3 (built) |
| `Server/Transient/WaylandTextInputV3.cs`, `IWaylandTextInputV3Events.cs` | `server/transient/wayland_text_input_v3.rs`, `i_wayland_text_input_v3_events.rs` | 2, part 5 (built) |
| `Server/Transient/XdgToplevelExport.cs` | `server/transient/xdg_toplevel_export.rs` | 2, part 5 (built) |
| `Server/Transient/Clipboard/WaylandDataDevice.cs`, `WaylandDataOffer.cs`, `WaylandDataSource.cs`, `WaylandOfferCookie.cs` | `server/transient/clipboard/wayland_data_device.rs`, `wayland_data_offer.rs`, `wayland_data_source.rs`, `wayland_offer_cookie.rs` | 2, part 4 (built) |
| `src/Shared/RawEventGrouping.cs`, `Avalonia.X11/X11IconLoader.cs`, `Selections/UriListHelper.cs`, `Interop/Glib.cs`, `Dispatching/GlibDispatcherImplBase.cs` | shared: modules of `ferroui-x11` | built |
| `README.md` | this document | |

## 11. Verification

Three levels, as for X11, because a Wayland client cannot run on the development machine:

1. **Compilation for Linux** from the Mac: `cargo check -p ferroui-wayland --features example-check --target x86_64-unknown-linux-gnu --all-targets` **[M]** (the crate, its tests and the example).
2. **Tests without a compositor** (`cargo test -p ferroui-wayland`): the logic named in section 3, 32 tests on the Mac **[M]**; on Linux also the wrappers of the C library (the pipe, the wait, a memory file and its mappings), the keymap compiled by `libxkbcommon` (two layouts, modifiers, the fallback to another layout), the mapping of a configure to the state of a window, and the fallback of the application builder.
3. **A compositor**: the CI job `wayland` (Ubuntu, x86-64) and the virtual machine, with the smoke mode of `examples/wayland_window.rs`: every check prints a line beginning `[ ok ]` or `[FAILED]`, and the exit code is the result.

### 11.1 The compositor of the tests: `sway` on the headless backend of wlroots

A Wayland client cannot ask the compositor about its own window, synthesize input, or read the screen through the core protocol. What a headless compositor offers beyond it decides which checks are possible:

| Compositor | Input injection | Reading the composed screen | Window state from outside |
|---|---|---|---|
| `weston --backend=headless` | Only through `weston-test`, a protocol of its test build that the distribution packages do not have **[R]** | The screenshooter, only with `--debug` **[R]** | None |
| `cage` (wlroots, a kiosk) | The virtual pointer and virtual keyboard protocols | `wlr-screencopy` | None; every window is forced full screen, so sizes and states say nothing |
| **`sway`** (wlroots) with `WLR_BACKENDS=headless` | `zwlr_virtual_pointer_v1` and `zwp_virtual_keyboard_v1`, for any client | `zwlr_screencopy_manager_v1`, for any client (what `grim` uses) | Its IPC (`swaymsg -t get_tree`): title, application identifier, geometry, focus, fullscreen; `swaymsg` changes the output (mode, scale) and closes a window |

So the job runs **`sway`**, with `WLR_BACKENDS=headless`, `WLR_LIBINPUT_NO_DEVICES=1` and `WLR_RENDERER=pixman` (no GPU on the runner), on a configuration file of the job: no bar, no borders, so the one tiled window covers the output exactly and a point of the window is the same point of the screen. The example is itself the test client: a second connection of its own binds the virtual pointer, the virtual keyboard (with a keymap it compiles with `libxkbcommon`) and the screen copy manager. That keeps the run to one process and three packages (`sway`, which brings `swaymsg`; the libraries; Mesa for EGL), and makes every check a comparison in one place. `wtype`, `wlrctl` and `grim` do the same from outside and are used by the catalog script only (`grim`).

One thing the choice costs: sway 1.9 (Ubuntu 24.04) has `xdg_wm_base` at version 2, and upstream requires 3, for `xdg_popup.reposition` **[CI]** (the first run of the job failed with "Required wayland global xdg_wm_base (>=3) not found"). The port accepts version 2 (DEVIATIONS.md): nothing of stage 1 uses a request of version 3, and a backend that refuses the compositor of the current Ubuntu LTS and of the tests would be verified nowhere. Stage 2 has to decide what a reposition is on version 2.

Touch cannot be injected: wlroots has no virtual touch protocol. The touch handler is covered by the tests of its framing only.

### 11.2 The phases of the smoke mode

| Phase | What is done | What is asked of whom |
|---|---|---|
| platform | `use_wayland` with the options of the run | The platform: the windowing platform, the screens, the cursor factory and the render loop are the Wayland ones; with `--mode=egl` the worker has platform graphics, with `--mode=software` none; with `--mode=dmabuf` (stage 3) the backend is the dmabuf swapchain where the compositor has `zwp_linux_dmabuf_v1`, and where it has not the run prints "[not measured]" and checks the fallback to a `wl_egl_window` |
| foreign (stage 3) | A second worker is started on the `wl_display` of a connection the example opened with `wayland-client` (handed over as a foreign display) | The second worker: it runs on that very `wl_display`, does not own it, binds its globals (`wl_compositor` version 4 or later) and has its platform graphics ready; the library that opened the display still makes a round trip. The display is left open for the worker, which runs until the process ends |
| screens | Nothing | The screens of the platform against the outputs `swaymsg -t get_outputs` reports: number, name, logical geometry |
| window | A window with a title is shown | The compositor (`get_tree`): a view with that title and the application identifier, focused, of the size of the output; the framework: the client size is that size, the window is active, the render scaling is the scale of the output (`--expect-scale`) |
| frames | Nothing | The compositor (screen copy): the fill colour at three points of the output, and the colour of a marker rectangle at its place, which shows the frame is the right way up and at the right scale |
| input | The virtual pointer moves to two points, presses and releases the left button, turns the wheel both ways; the virtual keyboard presses and releases "a", then holds "b" | The input callback of the window: moves at the expected positions (in logical units), the button events there, wheel deltas of one step with the right sign, the key with its physical key and its symbol, the text "a", and repeated key-down events for "b" at the rate the compositor announced |
| cursor | The window is given the text cursor, then a bitmap cursor | The worker: both resolve to a surface; no protocol error follows (a round trip of the example's own) |
| resize | `swaymsg` gives the output another mode | The compositor: the view has the new size; the framework: `resized` was raised with it; screen copy: the fill colour near the new bottom right corner |
| state | The window asks for fullscreen and back | `window_state_changed` with fullscreen and with normal; the compositor reports the view fullscreen in between |
| popup (stage 2) | A `Popup` of the framework is opened under the marker of the window (its left edge at the marker's); the pointer is moved over it; it is given a horizontal offset; a second popup is opened to the right of the first, with the first as its placement target, and closed; the left button is pressed on the window beside the popup | The worker: one popup registered, with its `xdg_popup`, mapped; the version of `xdg_wm_base`. Screen copy: the colour of the popup at its centre and its corners, the window's beside it and where it was after the move, the nested popup's at its place. The framework: the content of the popup is under the pointer and the marker is not; the popup is not an overlay; after the press it is closed, and the worker has no popup left |
| clipboard (stage 2, part 4) | The application sets a text, then a text with bytes of an application format and a format of the process, then a megabyte of text; the test client (which sees and sets the selection through `zwlr_data_control_manager_v1`, as `wl-copy` and `wl-paste` do) reads each; `wl-paste` reads the text; the test client sets a text, a megabyte of bytes and a list of one file; `wl-copy` sets a text; the application clears the clipboard | The worker: the data device manager and a data device of the seat. The other client: the MIME types of the selection (`text/plain;charset=utf-8`, `text/plain`, the application format; not the format of the process) and the bytes. The framework: `IClipboard` reads what the other client set, the application is the owner after it set something and not after another client did, and the worker has no source left then (`cancelled` was handled). No protocol error on either connection |
| dragdrop (stage 2, part 4) | The virtual pointer presses the marker, which starts a drag of a text, moves by (200, 150) and releases; then the same with a window that answers every drag with no effect | The drop target (the window itself): enters (the marker, then the window around it), motions, one drop at the place of the release with the text of the drag (the original transfer: the drag is of this process), and `do_drag_drop_async` resolves to copy; for the refused drag no drop and no effect. The worker: after either, the sources it had before (the selection), so the source of the drag is gone |
| textinput (stage 2, part 5) | The test client becomes the input method of the seat (`zwp_input_method_manager_v2`, which sway has; the same second connection as the pointer and the keyboard). The content of the window is replaced by a page with a text box, which takes the focus. The input method sends a preedit; commits two characters of three bytes each; sends a commit and a preedit in one batch; deletes four bytes before the cursor; maps an input popup; the virtual keyboard types a key; the page is taken away again | The worker: the text input manager and a text input object of the seat; the surface entered, with a client, enabled. The input method: activated once, with the content type "normal" and an empty surrounding text; after the commit told the text with the cursor after six bytes; deactivated once at the end. The text box: the preedit and not the text; the text and no preedit; both; the text without the two characters the four bytes are. The compositor: the rectangle it tells the popup has the size of the cursor rectangle the worker sent, and the composed output has the colour of the popup directly below that rectangle (where sway places an input popup). The key arrives as text while the text input is enabled (the input method has no grab; the compose state is fed). Both connections alive |
| export (stage 2, part 5) | The window is asked for the lease of a portal parent; the test client imports the window by the handle (`zxdg_importer_v2`), then asks for a handle nobody exported; the lease is disposed | The worker: the exporter, no export before, one with the lease, none after. The lease: a handle `wayland:` and what the compositor gave. The compositor: the import of the handle stays, the unknown one is answered with `destroyed`, and the importer is told `destroyed` when the lease ends |
| limits, title | A new title; a minimum and maximum size | The compositor: the title; no protocol error |
| decorations (stage 2, the last phase) | The window asks for less than full decorations once and for full ones again; `swaymsg` floats it, resizes it and places it; the virtual pointer drags the drawn title bar, then the drawn grip of the bottom right corner | The platform: nothing to draw while the decoration manager answers server-side, then title bar, border, resize grips and shadow. Screen copy: the content moved down by the title bar. The worker: the window geometry of the floating window starts after the shadow and has the size of the view of the compositor. The framework: the points pressed are the title bar and the grip (`hit_test_chrome_element`). The compositor: the view moved, then shrank (`xdg_toplevel.move` and `resize` done by it) |
| close | `swaymsg kill` | The closing callback runs, the window closes, the application ends with exit code 0 |

The job runs the smoke mode in software and through EGL (Mesa's `llvmpipe`, `LIBGL_ALWAYS_SOFTWARE=1`), each on an output of scale 1 and of scale 2, and since stage 2, part 3 of scale 1.5 (a mode of 1200 by 720, which 1.5 divides; without the resize phase, whose mode it does not divide).

### Stage 3: what was checked when it was built (2026-10-15)

| Check | Result |
|---|---|
| `cargo test -p ferroui-wayland` on the Mac | 79 passed: the 76 of before and 3 of the feedback decoding (the pairs of a tranche by index into the format table, a new table replacing the old one, a short device number) **[M]** |
| `cargo check -p ferroui-wayland --features example-check --target x86_64-unknown-linux-gnu --all-targets` | Clean, without a warning of the crate: the swapchain, the foreign display, the example with `--mode=dmabuf` and the phase `foreign`, and the Linux tests (the format codes and the layouts of `gbm_bo_handle` and `drmDevice`, the negotiation of formats, the attributes of a plane of an `EGLImage`) **[M]** |
| `cargo check -p ferroui-x11 --target x86_64-unknown-linux-gnu` | Clean with `native_functions!` exported **[M]** |
| The tracking page | 79 of 79 files, 612 of 780 members (168 waived) **[M]** |
| The CI jobs `wayland` and `wayland-next` | Extended (the dmabuf fallback under the software renderer in both; the render node of the runner, with the module `vgem` where it has none, informational); results with the pull request |
| The virtual machine | `ferroui-vm-wayland/scripts/stage3.sh`, sources at `e7b4a2f4`: every step passed, the swapchain included (the next section) **[VM]** |

What a run on the runner can and cannot show, from the sources rather than measured: wlroots announces `zwp_linux_dmabuf_v1` only when its renderer has a DRM device, which the software renderer (`WLR_RENDERER=pixman`, the one the tests use for want of a GPU) has not, so under it the smoke mode measures the fallback. `vgem` gives a render node without a driver of Mesa, on which the OpenGL ES renderer of wlroots most likely does not start; the step reports what happened.

### Measured in the virtual machine for stage 3 (2026-10-15) **[VM]**

Ubuntu 24.04.3 on ARM64, sway 1.9, GNOME Shell 46.0, `ferroui-vm-wayland/scripts/stage3.sh` (every step bounded with `timeout -k 10`), sources at commit `e7b4a2f4`; "checks of this script: ok 61, failed 0, not measured 10". The machine has a virtio GPU: `/dev/dri/card1` and the render node `/dev/dri/renderD128` (kernel driver `virtio-pci`), on which sway starts with its OpenGL ES renderer (`sway-run.sh --renderer gles2 --render-node /dev/dri/renderD128`) and then announces `zwp_linux_dmabuf_v1`. So the swapchain itself ran here.

| Step | Result |
|---|---|
| Tests of the crate | 115 passed (the 3 new tests of the feedback among them; on Linux also the format codes, the layouts of `gbm_bo_handle` and `drmDevice`, the negotiation of formats and the attributes of a plane) |
| Smoke through `wl_shm` buffers, sway on its software renderer, scale 1 | 103 of 103, with the phase `foreign` |
| Smoke through EGL (`LIBGL_ALWAYS_SOFTWARE=1`), the same compositor | 103 of 103 |
| `--mode=dmabuf` under the software renderer (the fallback) | 103 of 103: the warning "The dmabuf swapchain was requested, but the compositor has no zwp_linux_dmabuf_v1 (version 4 or later): rendering through a wl_egl_window", the "[not measured]" line of the swapchain, and the worker on `wl_egl_window` |
| Smoke through `wl_shm` buffers at scale 1.5 | 99 of 99 |
| **`--mode=dmabuf`, sway on OpenGL ES over the render node, scale 1** | **103 of 103**: "the GPU backend is dmabuf, with 1 format and modifier pairs"; every frame, the popups, the resize, the decorations and the screen copies drawn through the swapchain |
| The same at scale 2 | 103 of 103 |
| The same on 1200 by 720 at scale 1.5 | 99 of 99 |
| `--mode=egl` on the same compositor (the window system integration, for comparison) | 103 of 103 |
| The ControlCatalog through the swapchain (`FERROUI_CATALOG_WAYLAND=dmabuf`, `catalog-pictures.sh ... dmabuf`, `WAYLAND_DEBUG=1`), 1280 by 800 | 7 pictures of the compositor in `ferroui-screenshots/wayland/stage3/` (`dmabuf-01-Home.png` to `dmabuf-07-Slider.png`, the pages the right way up and as under the other modes) and the 7 frames of the framework. The trace: the default feedback asked once; 8 `create_params` and 8 `created` (two swapchains of four: the window was resized once), no `failed`; 502 buffers attached. Each buffer one plane, `ARGB8888` (875713089), a stride of 4400 for a width of 1100, the modifier 0 (linear) |
| The catalog under a headless Mutter, asking for the swapchain | Mutter announces `zwp_linux_dmabuf_v1` version 5 and the backend binds version 4; the default feedback once with its `main_device`; 16 buffers created (four swapchains), none failed; no protocol error. The frame of the framework is in `stage3/mutter/01-home.png` (Mutter has no screen copy for a client) |

What the logs also show: every smoke run has "Unable to initialize Wayland WSI EGL rendering: 'No suitable EGL config was found'", which is the second worker of the phase `foreign`, started with no OpenGL profile (it renders nothing), as the software mode of the main worker has always logged it. The phase passed in all eight smoke runs: the second worker on the `wl_display` of the other connection (the same pointer, not owned), `wl_compositor` version 6, its platform graphics ready, and a round trip of the owning connection afterwards.

Not shown by these runs: a hardware GPU (the virtio GPU of the machine is the paravirtual one), a buffer of more than one plane or with a modifier other than linear, KWin.

### Measured by the CI job (run 38083641943, 2026-10-10) **[CI]**

Ubuntu 24.04 on x86-64 (`ubuntu-latest`), sway 1.9, Mesa's `llvmpipe` for EGL (`LIBGL_ALWAYS_SOFTWARE=1`), commit `c3f096bf`:

| Step | Result |
|---|---|
| Tests of the crate | 45 passed (the 32 of every host and 13 of Linux: the C library wrappers and the wake-up pipe, the keymap of two layouts compiled by `libxkbcommon`, the configure of a window, the key repeat interval, the cursor names, the fallback of the builder) |
| Smoke through `wl_shm` buffers, scale 1 | 39 of 39 |
| Smoke through EGL, scale 1 | 39 of 39: the worker has a display of EGL, and the frames the compositor composed have the colours |
| Smoke through `wl_shm` buffers, scale 2 | 39 of 39: the window renders at 2, the output of 1280 by 720 pixels is 640 by 360 logical units, the marker is at twice its logical place |
| Smoke through EGL, scale 2 | 39 of 39 |
| Without a compositor | The backend fails with "Unable to connect to Wayland display" and what it tried |

What the 39 checks are, from the log of the run: the services of the platform and the render loop of the worker; the screen against `get_outputs` (HEADLESS-1, its logical rectangle); the view of the compositor with the title and the application identifier, focused and active, covering the output, with the client size and the scale; six pixels of the composed output (fill at three points, the marker, beside the marker) and its size; two pointer moves at their logical positions, a button press and release there, a wheel notch each way (deltas of -1 and 1); the key of "a" with its physical key and symbol, the text "a", and 24 key-down events for "b" held for a second and a half (the compositor announces 25 a second after 600 milliseconds; none after the release); a themed cursor with an image, a bitmap cursor with a surface, both released; the output given the mode 1024 by 600, the framework and the view following, the composed output of that size with the fill at its new corner; fullscreen and back, as the platform and the compositor report it; a title changed and restored; size limits whose minimum exceeds the maximum, after which the connection is still there.

The first run of the job (38083227527) had the tests and the build pass and every smoke step fail at start for want of `xdg_wm_base` 3 (section 11.1).

### Measured by the CI job for stage 2, parts 1 to 3 (runs 38092848947 and 38095307673, 2026-10-11) **[CI]**

Ubuntu 24.04 on x86-64, sway 1.9 (`xdg_wm_base` version 2), Mesa's `llvmpipe` for EGL, commit `0816ddd5`; the last run, on commit `78233748`, has five checks more in every smoke run (59 of 59, and 55 of 55 at scale 1.5: the configured rectangle of every popup, the popup that is moved 24 times in quick succession, the popup of the decorated window) and the catalog steps described after the table:

| Step | Result |
|---|---|
| Tests of the crate | 52 passed (34 of every host: with the positioner of a popup, the clamp of its anchor rectangle, the preferred fractional scale and the destination of the viewport; 18 of Linux: with the conversions of the anchor, the gravity and the constraint adjustment) |
| Smoke through `wl_shm` buffers and through EGL, scale 1 | 54 of 54 each: the 39 of stage 1, 8 of the popup phase, 7 of the decorations phase |
| The same on an output of scale 2 | 54 of 54 each |
| The same on an output of 1200 by 720 at scale 1.5 (without the resize phase) | 50 of 50 each: the window renders at 1.5, the output is 800 by 480 logical units, the composed output has 1200 by 720 pixels with the marker and the popups at one and a half times their logical places |

What the popup phase showed, from the log: one popup of the worker with its `xdg_popup`, mapped; in the composed output the colour of the popup at its centre (180, 145) and at both corners, the window's colour to its right and below it, the marker above it; the pointer at that centre is over the content of the popup; with an offset of 60 the popup is at its new place and the window shows where it was ("created again: the shell is older than version 3"); a second popup to the right of the first has the first as its parent on the compositor (two attached, two mapped, one nested) and goes when it is closed; a press on the window at (470, 310) closes the popup and the worker has none left; the connection and the window are still there.

What the decorations phase showed: the compositor has a decoration manager and the framework is asked to draw nothing; after the application asked once for less than full decorations it is asked for the title bar, the border, the resize grips and the shadow; the content is 31 or 32 units further down in the composed output and the top of the window is white; floating, the window geometry is (8, 8, 640, 480) for a view of the compositor of 640 by 480 (the shadow of 8 is outside it); a drag of the drawn title bar by (40, 20) moved the view from (40, 30) to (80, 50); a drag of the drawn top left grip by (60, 40) made the view (140, 90, 580, 440) and the client size of the framework followed.

**The catalog's popups, and a fault they found.** The first pictures of the catalog with a combo box and a menu open (virtual machine) had both popups at the right edge of the output. The popup phase had not seen it, because it only looked at pixels of popups at one place; since then the worker keeps, for every popup, the anchor rectangle the framework gave, the numbers of the positioner and the configure of the compositor (`PopupPlacement`), the smoke mode compares the configured rectangle of every popup with the expected one, and the catalog's screenshot run prints a line per popup it opened that says whether the popup is below its control (`catalog-pictures.sh` fails when it is not). The job builds the catalog and runs both popups at scale 1, 1.5 and 2. Run 38094108634 gave the numbers: the positioner was right for what it was given, and what it was given was an anchor rectangle at or beyond the right edge of the window. Run 38094721072 said why: a page of the catalog slides in from the right, and the screenshot run opened the control while it was still a page width away ("appeared at x = 2490", "stays at x = 830"). A popup takes its place from where its control is when it opens, on every platform; the run now opens a control once it stays put. Since then (run 38095307673) the six popups are configured exactly at their controls: for example an anchor of (830, 151, 250 by 32) and a popup at (830, 183), and at scale 1.5 the menu under "File" at (280, 141).

The same run 38094721072 found a fault of the backend: at scale 1.5 the catalog ended with the protocol error "wrong configure serial". A popup that is laid out twice while it opens is created again on a shell before version 3 (section 7.1), and the answer of the UI thread to the configure of its first `xdg_surface` arrived at the worker after that and was acknowledged on the second. A shell surface now keeps the serials of the configure events of its current `xdg_surface` and drops an answer that names none of them (DEVIATIONS.md); the smoke mode moves a popup 24 times at intervals of 0 to 28 milliseconds and expects it at its last place with the connection alive.

Three earlier runs of the job belong to the record. Run 38091618151 (commit `9330fab4`) was the first with popups: 47 of 47 in all four smoke runs, at the first attempt. Runs 38092018485 and 38092424028 failed in the decorations phase for faults of the phase, not of the backend: the first version sent the press, the moves and the release of a drag before the UI thread had seen the press, so the request for the move reached the compositor after the button was up and was ignored (which is the behaviour the serial exists for); the second assumed the size sway gives a floating window and pressed a grip that lay beyond the output. A tiled window whose decorations appear keeps the size of its content, as the framework decides: its window geometry was 1026 by 632 on a tile of 1024 by 600 (the title bar and the border are added), which sway tolerates.

The log of sway has "A configure is scheduled for an uninitialized xdg_surface" once for every popup: sway 1.9 unconstrains a popup before the popup's first commit. It is noise of the compositor.

### Measured by the CI job for stage 2, part 4 (runs 38098222664 and 38098820720, 2026-10-11) **[CI]**

Ubuntu 24.04 on x86-64, sway 1.9, `wl-clipboard` of the distribution, commit `cf933e21`:

| Step | Result |
|---|---|
| Tests of the crate | 68 passed (40 of every host: with the MIME types of the formats and the result of work of another thread; 28 of Linux: with the pipe of a transfer, the actions of the protocol as effects, the value of the bytes of a format, the list of files, the sentinel type of a drag of the process) |
| Smoke through `wl_shm` buffers and through EGL, scale 1 and scale 2 | 80 of 80 each: the 59 of parts 1 to 3, 14 of the clipboard phase, 7 of the dragdrop phase |
| The same on an output of 1200 by 720 at scale 1.5 (without the resize phase) | 76 of 76 each |

What the clipboard phase showed, from the log: the worker has the data device manager and a data device of the seat; a text the application set ("café 世界" in it) is announced to the other client with the types `text/plain;charset=utf-8` and `text/plain` and read by it byte for byte, and the application is the owner; a transfer with a text, 256 bytes of an application format and a format of the process is offered as the two text types and `application/x-ferroui-smoke-bytes` (the format of the process is not offered) and the bytes arrive; 1 048 576 bytes of text are read by the other client (the writer waits for the reader: a pipe holds 65 536); `wl-paste` prints the text; a text the other client set is read by the application ("grüße" in it), which is then not the owner, and the worker went from one source to none (`cancelled` handled); 1 048 576 bytes of the application format set by the other client are read by the application; a `text/uri-list` of one file gives one storage item of that name; a text `wl-copy` set is read; after the application cleared the clipboard the selection has no type, and the source of the other client was cancelled. Neither connection was ended.

What the dragdrop phase showed: a press on the marker starts a drag, which enters the marker and then the window around it, is moved in 8 steps and dropped at (320, 230), where the drop handler reads the text of the drag; `do_drag_drop_async` resolves to copy, and the worker has the sources it had before (the selection the application owns since the clear), so the source of the drag was destroyed after `dnd_finished`. With a window that answers every drag with no effect there is no drop, the result is no effect (`cancelled`), and the source is gone as well.

The first of the two runs (commit `92aa2d12`) had the clipboard phase pass at the first attempt, 14 of 14 in every smoke run, and four checks of the dragdrop phase fail for faults of the phase, not of the backend: it expected one enter (the framework raises one for the marker and one for the window around it) and no source at all after a drag (the selection of the application is one). Two unit tests asked for what a test has not (a bitmap needs the render interface of an application; a list of files keeps only what exists).

**What part 4 does not measure.** A drag between two clients: both drags of the phase start and end in the application, so the offer side reads no pipe during a drag (the code that would is the code the clipboard phase runs: `WaylandDataTransfer` over an offer cookie) and the source side of a drag answers no `send` (the same handler as the selection's). A test client that is a drop target or a drag source needs a mapped surface of its own and, as a source, its own thread (the drop handler of the application reads synchronously while the test client would have to write). Also not measured: a bitmap through the clipboard, the cursor of a drag (`wl_pointer.set_cursor` during a drag is not visible in a screen copy), a drag started by touch, and `wl_data_device` on Mutter or KWin (their `source_actions` and `action` order, the cursor during a drag).

### Measured by the CI job for stage 2, part 5 (runs 38101694610 to 38104145078, 2026-10-11) **[CI]**

Ubuntu 24.04 on x86-64, sway 1.9, commit `7b26c3fc` (run 38104145078, the job green):

| Step | Result |
|---|---|
| Tests of the crate | 104 passed (76 of every host: with the 23 of the text input facade, the 7 of the conversions between bytes and UTF-16 indices and the 6 of the content type; 28 of Linux) |
| Smoke through `wl_shm` buffers and through EGL, scale 1 and scale 2 | 100 of 100 each: the 80 of parts 1 to 4, 14 of the textinput phase, 6 of the export phase |
| The same on an output of 1200 by 720 at scale 1.5 (without the resize phase) | 96 of 96 each |
| The TextBox page of the ControlCatalog with the test client as the input method of the seat, at scale 1 and 1.5 | The first card of the gallery is pressed and the text box of the sample takes the focus; the input method is activated with the content type (0, 0) and the surrounding text `("", 0, 0)`, commits "typed by an input method: Zażółć gęślą jaźń" and sets the preedit "ni hao"; the catalog says the text box has the text, and the input method was last told the text with the cursor after its 52 bytes. The picture of the compositor (`software-textinput-01-TextBox.png` of the artifact) has the text in the box with the preedit underlined after it, and the readout of the bound property with the text alone |

What the textinput phase showed, from the log: the worker has `zwp_text_input_manager_v3` and a text input object of the one seat; with the focus of the text box the surface is entered by the seat, has a client and is enabled, with the cursor rectangle (165, 145, 1, 14) in the text box at (160, 140); the input method is activated once, with the content type "normal" and an empty surrounding text; the preedit "ni" is the preedit of the text box and its text stays empty; two characters of three bytes each become its text, the preedit goes, and the input method is told the text with the cursor and the anchor after six bytes; a batch with a commit and a preedit gives both; `delete_surrounding_text(4, 0)` removes the two characters the four bytes are ("你好!" becomes "你") and the batch without a preedit clears the preedit; the key of "a" arrives as text while the text input is enabled; without the text box the input method is deactivated once and the worker has no client; both connections are alive.

What the export phase showed: the worker has `zxdg_exporter_v2` and no export; the lease of the window has a handle such as `wayland:343b4830cf37dd67dc2889ef045ce20e` and the worker one export; the test client imports the window by that handle and is not told it is gone; a handle nobody exported is answered with `destroyed`; when the lease is disposed the importer is told `destroyed` and the worker has no export.

**Two faults of upstream's order, found by the first run** (38101694610, commit `b86b490a`: 95 of 101, with the preedit, the commit, the delete, the key and the whole export phase passing at the first attempt). The input method was told no surrounding text and no content type, although the worker had sent both. (1) wlroots takes the state of the commit that enables as what the client supports for the session (`active_features`), as the protocol says, and upstream enables as soon as the client is set, before its state has arrived. (2) The framework sets the options of the focused element before the client, and upstream forgets the state of a surface when its client changes, so the content type was never sent at all. The port holds the enable back until the commands of the UI thread have run and sends the options again after the client (section 8.2; DEVIATIONS.md). From run 38102871806 on the input method hears of both.

**What the phase cannot measure under sway 1.9: the cursor rectangle at the compositor.** The input method protocol shows the rectangle only to a popup of the input method (`zwp_input_popup_surface_v2.text_input_rectangle`), and sway has those popups from 1.10: the binary of 1.9 has no handler for one (run 38103535167 looked: its strings name the input method relay and nothing of a popup; with debug logging the compositor logs the commits of the text input and never the popup). The test client maps the popup all the same; where a compositor tells it a rectangle, the phase compares its size with the cursor rectangle the worker sent and looks for the popup directly below that rectangle in the composed output. Under 1.9 the run prints "[not measured]" for it and checks only that the rectangle the worker sent lies in the text box.

Also not measured by part 5: a real input method (IBus, Fcitx 5) and its candidate window; the text input on Mutter and KWin; the portal file chooser itself with the exported parent (the job has no session bus and no portal: the factory gives nothing and the managed dialogs answer, which is the path the catalog's dialogs take there); a compositor restart while a text box has the focus.

### Measured under sway 1.10.1 by the informational job (runs 38105648789 and 38106254262, 2026-10-11) **[CI]**

The job `wayland-next` runs the same smoke mode in a container of Debian 13, whose sway is 1.10.1 (the compositor run by a user of the container, not by root). It does not fail the workflow: the phases were written against sway 1.9, and a difference is a finding. sway 1.10.1 has a newer `xdg_wm_base`, of which the backend binds version 4 (the smoke mode prints the bound version), so **a popup is moved with `xdg_popup.reposition` here**, and it has the popups of an input method.

| Step, commit `4a6dccba` | Result |
|---|---|
| Smoke through `wl_shm` buffers, scale 1 | 102 of 102: the 100 of sway 1.9 and the two the older compositor cannot answer |
| The same on an output of 1200 by 720 at scale 1.5 (without the resize phase) | 98 of 98 |
| Smoke through EGL (`llvmpipe`), scale 1 | 102 of 102 |

- **The cursor rectangle at the compositor**: the worker sent (172, 145, 1, 14); the compositor tells the popup of the test input method (0, -14, 1, 14), relative to the popup, which is the same rectangle seen from a popup whose top left corner is the bottom left corner of the rectangle; and the composed output has the colour of the popup at (196, 171), directly below the rectangle (at scale 1.5: (173, 145, 1, 14), and the pixel at (295, 256)).
- **A fault of the reposition path, found by the first run** (commit `3fd937ba`, 99 of 102): "popup moved", "popup moved in a hurry" and "nested popup" failed. The worker had the new rectangle of the popup from the compositor, and the composed output had the popup at its old place. A compositor answers `reposition` with a configure and moves the popup when that configure is acknowledged and committed; the backend, like upstream, acknowledges a serial with the next buffer, and nothing asked for a buffer when only the place of the popup changed. The frame the port already asked for after a popup was created again (the path of version 2) is now asked for after every configure of a popup (DEVIATIONS.md). Upstream at the tracked commit has the same order and no such request.

### Measured in the virtual machine for stage 2, part 5 (2026-10-11) **[VM]**

Ubuntu 24.04.3 on ARM64, sway 1.9, GNOME Shell 46.0, `ferroui-vm-wayland/scripts/stage2c.sh`, sources at commit `3fd937ba` (the second run of the script; "checks of this script: ok 130, failed 0, not measured 13"):

| Step | Result |
|---|---|
| Tests of the crate | 104 passed |
| Smoke through `wl_shm` buffers and through EGL (`LIBGL_ALWAYS_SOFTWARE=1`), scale 1 and scale 2 | 100 of 100 each |
| Smoke through `wl_shm` buffers on an output of 1200 by 720 at scale 1.5 | 96 of 96 |
| Smoke through EGL with the driver the machine chooses by itself | 100 of 100 |
| The TextBox page of the ControlCatalog with the test input method: through `wl_shm` buffers, through EGL, and at scale 2 | 3 pictures in `ferroui-screenshots/wayland/stage2c/` (`software-textinput-01-TextBox.png`, `egl-...`, `scale2/software-...`): the sample "First Look" with the committed text (with its diacritics) in the text box, the preedit "ni hao" underlined after it, and the readout of the bound text without the preedit |
| The catalog started without a variable | It runs as a Wayland client (exit 0; under the headless compositor nothing else could) |
| The catalog with `FERROUI_CATALOG_WAYLAND=0` | It asks for an X display, which the headless compositor does not have, and never for the Wayland backend: the switch is honoured |

**Mutter, for the first time** (GNOME Shell 46.0 without a monitor: `gnome-shell --headless --wayland --virtual-monitor 1280x800` on a session bus of its own, `mutter-run.sh` beside the stage script; nothing was installed). Mutter has none of the test protocols of the smoke mode, so the evidence is the trace of the library (`WAYLAND_DEBUG=1`) of the catalog running under it, read by the script:

| Check | Result |
|---|---|
| The shell | `xdg_wm_base` version 6; the backend binds 4 |
| Decorations | No decoration manager: the framework draws them from the first frame (the path sway reaches only when the application asks) |
| Scale | `wp_fractional_scale_v1.preferred_scale(240)`: 2; the top-level is configured with 640 by 368 logical units |
| The popup of a combo box | Created (`get_popup`), configured at (270, 216) with 250 by 136, which is the left edge and the bottom of the anchor rectangle (270, 184, 250, 32) the backend sent; the configure acknowledged, a buffer attached and committed; no `popup_done`; no protocol error. No grab is needed for it to map |
| The popup against its control | At its control: the control is at (270, 183.5) on the surface of the window. The first run compared with the place of the control in the window, which lies a title bar higher where the framework draws the decorations, and called the popup misplaced; the catalog's hook now measures on the surface |
| Text input | A `zwp_text_input_v3` of the seat is made; **not measured beyond that**: the headless shell never gives the window the keyboard focus (no `wl_keyboard.enter`: it starts in its overview), so it sends no `enter` to the text input and the backend, rightly, no `enable` |

Not shown by these runs: a picture of what Mutter composed (it has no screen copy for a client; the pictures of the steps are the frames the framework drew of the window, which never contain a popup), `xdg_popup.reposition` on Mutter (the run opens the popup once), and input of any kind.

### Measured in the virtual machine for stage 2, part 4 (2026-10-11) **[VM]**

Ubuntu 24.04.3 on ARM64, sway 1.9, grim, wl-clipboard 2.2.1, `ferroui-vm-wayland/scripts/stage2b.sh`, sources at commit `cf933e21`:

| Step | Result |
|---|---|
| Tests of the crate | 68 passed |
| Smoke through `wl_shm` buffers and through EGL (`LIBGL_ALWAYS_SOFTWARE=1`), scale 1 and scale 2 | 80 of 80 each |
| Smoke through `wl_shm` buffers on an output of 1200 by 720 at scale 1.5 | 76 of 76 |
| Smoke through EGL with the driver the machine chooses by itself | 80 of 80 |
| The ControlCatalog's Clipboard and Drag+Drop pages, through `wl_shm` buffers and through EGL, output of 1280 by 800 | 4 pictures in `ferroui-screenshots/wayland/stage2b/` (`software-01-Clipboard.png`, `software-02-Drag-Drop.png`, and the two of EGL): the pages as under X11. The pictures show the pages; nobody pressed their buttons (the clipboard and the drags are what the smoke mode measures) |

### Measured in the virtual machine for stage 2, parts 1 to 3 (2026-10-11) **[VM]**

Ubuntu 24.04.3 on ARM64, sway 1.9 and grim, `ferroui-vm-wayland/scripts/stage2a.sh`, sources at commit `17089f29` (the second run of the script; the first, on `0816ddd5`, had the same smoke results with 54 and 50 checks and showed the misplaced popups described above):

| Step | Result |
|---|---|
| Tests of the crate | 52 passed |
| Smoke through `wl_shm` buffers and through EGL (`LIBGL_ALWAYS_SOFTWARE=1`), scale 1 and scale 2 | 59 of 59 each |
| The same on an output of 1200 by 720 at scale 1.5 | 55 of 55 each |
| Smoke through EGL with the driver the machine chooses by itself | 59 of 59 |
| The ControlCatalog, 7 pages, output of 1280 by 800; of 2560 by 1600 at scale 2; of 1920 by 1200 at scale 1.5 | 7 pictures each in `ferroui-screenshots/wayland/stage2/`, `stage2/scale2/`, `stage2/scale15/` |
| The catalog with the first combo box of the ComboBox page open: through `wl_shm` buffers, through EGL, and at scale 2 | "Popup placement: [ok]" each (an anchor of (510, 154, 250 by 32)); `software-popup-01-ComboBox.png`, `egl-popup-01-ComboBox.png`, `scale2/software-popup-01-ComboBox.png`: the list directly under its combo box |
| The catalog with the first menu item of the Menu page open | "Popup placement: [ok]" (an anchor of (280, 116, 47 by 32)); `software-popup-01-Menu.png`: the menu under "File" |
| The catalog floating on an output of 1600 by 1000 after it asked once for less than full decorations | `software-decorated-01-Home.png`, `software-decorated-02-Buttons.png`: the title bar with the title and the caption buttons, the border; the window at the size it asks for |

**A popup opened while its control is moving.** What the picture hook met is reachable by a user: a press on a control of a page that is still sliding in opens its popup at the place the control has at that moment, because the anchor rectangle is the control's bounds transformed to the root at the time of the update (`calculate_anchor_rect`, `transform_to_visual`), and nothing updates it when only a render transform changes. Read against upstream at the tracked commit (`Popup.cs`): the position of an open popup is updated when the window moves (`PositionChanged`), when the placement target raises `LayoutUpdated`, and when a parent popup moves; an animated render transform raises none of them, so upstream leaves the popup where the control was as well, on every platform (on Wayland the compositor then also slides a popup that would leave the output back into it, which is why the pictures had it at the edge). A host test of the controls crate has since confirmed it (`popup_moving_target_tests.rs`: a popup opened in the middle of a slide is given the rectangle of the moving target, for example (299.2, 30) for a target that rests at (20, 30), and is not positioned again when the page comes to rest; the target is hit at its visual position during the transition), the port was compared with upstream file by file and matches it, and the fix is put to the owner in `decisions/popup-opened-while-its-target-moves.md`. Nothing in the controls crate was changed for it.

### Measured in the virtual machine (2026-10-10) **[VM]**

Ubuntu 24.04.3 on ARM64 (`aarch64-unknown-linux-gnu`), sway 1.9 and grim from the distribution, the sources and the build directory on the shared folder (`ferroui-vm-wayland/scripts/stage1b.sh`, commit `3c93449f`):

| Step | Result |
|---|---|
| Tests of the crate | 45 passed |
| Smoke through `wl_shm` buffers, scale 1 and scale 2 | 39 of 39 each |
| Smoke through EGL with `LIBGL_ALWAYS_SOFTWARE=1`, scale 1 and scale 2 | 39 of 39 each |
| Smoke through EGL with the driver the machine chooses by itself | 39 of 39 |
| The ControlCatalog (`FERROUI_CATALOG_WAYLAND`), through `wl_shm` buffers and through EGL, output of 1280 by 800 | 7 pages each (Home, Buttons, TextBlock, ListBox, Image, Calendar, Slider), a picture of the output per page taken by `grim`: `ferroui-screenshots/wayland/software-NN-<page>.png` and `egl-NN-<page>.png`; the window covers the output and the pages look as they do under X11 |
| The same on an output of 2560 by 1600 at scale 2, through `wl_shm` buffers | 7 pictures in `ferroui-screenshots/wayland/scale2/`: the catalog renders at scale 2 (text and icons at twice the resolution) |

Two things the catalog runs showed:

- The first run, with the backend still refusing sway for its `xdg_wm_base` version, ended in the panic of the fallback (the X11 backend finds no display under a headless compositor) with no word about Wayland on the error stream: the warning upstream logs goes to the log sink only. `use_wayland_with_fallback` now also writes the reason to the error stream when the fallback fails (DEVIATIONS.md).
- At scale 2 the window in the pictures is 640 by 400 logical units in the middle of the output, not the whole output. That is the catalog's own smoke helper (`fit_window_to_screen` of the desktop host, which shrinks the window to its screen): it divides the bounds of the screen by the scaling of the window, and a screen of this backend has logical bounds and a scaling of 1 by upstream's design (section 6), so the helper halves them. The window then has a size other than the one the compositor configured for a tiled view, which sway shows centred. Not a fault of the backend's rendering (the smoke mode at scale 2 covers the output); open: either the helper learns that a platform may report logical screens, or the question goes upstream's way when its sample does the same.

Verified nowhere (section 12.3 has the list as it stands after stage 2, with the cheapest way to verify each): a desktop compositor (GNOME's Mutter, KDE's KWin: fractional scaling, their decoration answers, their popups, their data devices; Mutter has no decoration manager, so there the framework draws from the start, which the runs reach only through the application's own request), `xdg_popup.reposition` (version 3 of the shell), `popup_done` (sway did not dismiss a popup by itself in any run), a popup with a grab (there is none), a GPU with a hardware driver (the nvidia workarounds of the EGL surface), input from a real device, touch, a compositor restart with popups open.

## 12. Stages

| Stage | Content | What its run proves |
|---|---|---|
| 1 (built) | The crate and bindings; `use_wayland`, `use_wayland_with_fallback`, the options; the worker, the connection, the globals; outputs as screens; the top-level; the render timer; software and EGL rendering; pointer, keyboard and touch; cursors; the example and the CI job | Section 11 |
| 1b (built) | The ControlCatalog on Wayland when asked (`FERROUI_CATALOG_WAYLAND=1`), with pictures from `grim` (`scripts/wayland/catalog-pictures.sh`) | The script of the virtual machine: a picture per page, 21 pictures |
| 2, part 1 (built) | Popups: `xdg_popup` and the positioner, nested popups, reposition and its equivalent before version 3 of the shell (section 7.1) | The popup phase of the smoke mode; the catalog with a combo box and a menu open, pictured by the compositor |
| 2, part 2 (built: verification) | Decorations: the negotiation, what the framework draws, interactive move and resize (section 7.2; the code is stage 1's) | The decorations phase of the smoke mode; the catalog with drawn decorations, pictured by the compositor |
| 2, part 3 (built) | Fractional scaling and the viewport | The smoke mode on an output of scale 1.5 |
| 2, part 4 (built) | The clipboard and drag and drop (`wl_data_device`; upstream binds no primary selection): section 8.1 | The phases clipboard and dragdrop of the smoke mode: the test client of the example in both directions, `wl-copy` and `wl-paste` |
| 2, part 5 (built) | Input methods (`text-input-v3`): section 8.2; the portal parent through `xdg-foreign` and the storage provider chain: section 8.3; no protocol tracer (the bindings have no hook; upstream binds no `xdg_activation` and no primary selection) | The phases textinput and export of the smoke mode, with the test client as the input method of the seat and as the importer; the TextBox page of the catalog typed into by that input method, pictured by the compositor |
| 3 (built, 2026-10-15) | The dmabuf swapchain (GBM, DRM, feedback: section 5.1) and a display another library opened (section 4.4) | `--mode=dmabuf` of the smoke mode: the fallback under sway on its software renderer (the CI jobs), the swapchain itself under a sway that renders with OpenGL ES on a render node (the virtual machine, and the runner where it has one); the phase `foreign` in every smoke run (section 11) |

**The catalog selects Wayland by itself since stage 2 is complete**, as upstream's does: `ControlCatalog.Desktop` calls `use_platform_detect().use_wayland_with_fallback()` always, so on a session with a compositor it is a Wayland client, and where the compositor cannot be reached or used (no socket, no `libwayland-client`, a required global missing) the backend of the platform detection takes over, with the reason logged. What had kept it opt-in is built: popups (part 1), the clipboard and drag and drop (part 4), input methods (part 5). Nothing else stops it. `FERROUI_CATALOG_WAYLAND` remains as a switch of the port for runs that need one backend: `0` or `x11` for the backend of the platform detection alone (the catalog through XWayland on a Wayland session), `software` for shared memory buffers alone (DEVIATIONS.md). What a user of a Wayland session gives up against the X11 backend through XWayland, all of it upstream's as well: accessibility (below), the position of a window and what else the protocol does not offer, the native control host (the embedding page of the catalog), and the GTK file dialogs (the chain is the portal, then the managed dialogs).

`use_platform_detect` is unchanged and stays so: an application opts in.

### 12.1 What remains before platform detection could choose the backend, and for accessibility

**Platform detection.** Upstream keeps the backend opt-in: `UsePlatformDetect` chooses X11 on Linux, and an application calls `UseWaylandWithFallback()` after it (`AvaloniaWaylandPlatformExtensions.cs`); nothing at the tracked commit looks at `WAYLAND_DISPLAY` by itself. The port follows: `use_platform_detect` is unchanged and `ferroui-desktop` does not depend on the crate. Should upstream, or the owner, make it the default on a Wayland session one day, this is what the backend lacks against the X11 one, in the order an application would notice: accessibility (below); verification on Mutter and KWin, which are the compositors of the sessions the default would apply to (everything measured so far is sway: section 12.3); `xdg_popup.reposition` exercised on a shell of version 3; the dmabuf swapchain is not needed for it (the `wl_egl_window` path is upstream's default as well). Input methods and the portal file dialogs with a parent window exist since part 5. Not available on Wayland by the protocol, on either side: the position of a window, activation by the client (upstream binds no `xdg_activation`), topmost, the window icon, a native control host, a tray icon outside D-Bus.

**Accessibility.** Upstream's Wayland project has no accessibility at the tracked commit: no reference to the AT-SPI project, no automation peer, no `AtSpi` in any of its files (`git grep -i "atspi\|automation\|accessib"` over `src/Avalonia.Wayland` finds nothing; confirmed again when the backend was closed, 2026-10-11), and its project file references the base package, the dialogs and the FreeDesktop project only. So the port wires none either (`WaylandPlatform` does not start the AT-SPI server of `ferroui-freedesktop`, which `X11AtSpiAccessibility` starts for X11), and it is not a gap of the port against upstream: a screen reader user of the catalog on a Wayland session has `FERROUI_CATALOG_WAYLAND=0`. The AT-SPI server of `ferroui-freedesktop` (`at_spi/`) is independent of the window system (D-Bus); what a Wayland counterpart of `X11AtSpiAccessibility` would have to do is start it when the platform starts, add and remove the root of each window as it is shown and closed, and report the active window. Two things have no answer on Wayland and would need a decision: the screen coordinates of an element (`GetExtents` with the screen as the coordinate type: a client does not know where its surface is), and the window the pointer or the screen reader refers to by a native handle. Recorded here; not built.

### 12.2 The tracking page after the triage (2026-10-11)

`tracking/Avalonia.Wayland.md` counted as missing a good deal that is there in another shape. The triage went through every item of the page and recorded, in the tracking data, why the scanner does not find it (`data/path-overrides.toml` and `data/member-waivers.toml`, section `Avalonia.Wayland`), as the X11 project's declaration files were handled:

| What | How it is recorded | Members |
|---|---|---|
| `XkbCommonNativeMethods.cs`, `WaylandEglNativeMethods.cs`: platform invokes whose declarations are those of `xkbcommon-dl` and `wayland-egl` | not applicable (out of the totals) | 36 |
| `UnsafeNativeMethods.cs`: the C library and `libwayland-cursor` with their structures and constants, replaced by `libc` behind safe functions and by `wayland-cursor` | replaced | 42 in 6 types |
| The listener classes of the generated bindings (`XdgSurfaceListener`, `TopLevelListener`, `PopupListener`, `WaylandDataDeviceListener`, `WaylandDataOfferListener`, `WaylandDataSourceListener`): `Dispatch` implementations beside their objects | waived types | 28 in 6 types |
| `WaylandConversionExtensions`: extension methods, functions of the module | waived type | 4 |
| `WindowBaseImpl`: a base class; its members of the window contracts are implemented by the window and by the popup each | waived | 24 |
| `WSurface`: the connection and the globals as arguments, the text input members on the state of the worker, the render surfaces made by the client, the two change notifications as a returned value | waived | 16 |
| The constructors of the exception classes and of five records; the members of the worker that live on the client or on the worker thread; single members with another name (`dispose_export`, `connect_to_fd`, `on_output_done`, `destroy_cursor`, ...) | waived, each with the name of its counterpart | 54 |

After stage 3 (2026-10-15) the page shows nothing open: 79 of 79 files, 108 of 121 types (13 waived), 612 of 780 members (168 waived). One waiver was added for it (`CreateRenderSurface` of the dmabuf backend, as for the other one), and the foreign display matched without one. What the page showed as open before stage 3: the dmabuf swapchain (`WaylandDmabufFeedback.cs`, `WaylandEglDisplay.cs`, `WaylandEglDmaBufPlatformGraphics.cs`, `WaylandEglDmaBufSurface.cs`, `DrmGbmUnsafeNativeMethods.cs`: 67 members), `ZwpLinuxDmabufV1` of the globals, and a display another toolkit owns (`WaylandWorker.Start` and the constructor of `WaylandConnection` with a foreign display): 70 members, all of stage 3. Nothing of stages 1 and 2 is counted as missing. `server/cross_thread_result.rs`, the one file of the crate without an upstream file, has its reason.

### 12.3 What is not verified, and the cheapest way to verify each

Everything measured ran under sway 1.9 (wlroots 0.17) on its headless backend with the software renderer, in the CI job and in the virtual machine. Nothing below was installed or built for this work; each line names what the owner would have to provide.

| Not verified | Why not yet | The cheapest way |
|---|---|---|
| GNOME (Mutter) beyond what section 11 has (start, drawn decorations, scale 2, a popup created, placed and mapped): input of any kind, text input with IBus, the data device, `xdg_popup.reposition`, what the compositor actually shows, the portal dialog with the exported parent | Not a wlroots compositor: none of the three test protocols of the smoke mode (virtual pointer, virtual keyboard, screen copy) and no `swaymsg`; headless, the shell stays in its overview and gives the window no focus | A person in the GNOME session of the virtual machine, where the catalog now starts on Wayland by itself: nothing to install. The trace (`WAYLAND_DEBUG=1`) of such a session answers the protocol questions; `mutter-run.sh` on the share is the headless form for what needs no focus |
| KDE (KWin): its decoration answers (server-side by default), its data device order, text input with Fcitx 5 | Not installed anywhere | One package in the virtual machine, for the owner to decide: `apt-get install kwin-wayland`, then `kwin_wayland --virtual --width 1280 --height 800 --exit-with-session <catalog>` with `WAYLAND_DEBUG=1`, as for Mutter; or a Kubuntu live image in a virtual machine of its own |
| `xdg_popup.reposition` on Mutter and KWin | **Verified under sway 1.10.1** since the informational job exists (section 11: it found and fixed a fault); not under the two desktop compositors | The traces of the sessions above |
| The cursor rectangle with a real input method (where its candidate window is) | **Verified under sway 1.10.1** with the popup of the test input method (section 11) | Fcitx 5 under any compositor and a look |
| The informational job as a required one | It is `continue-on-error`: a failure does not stop a merge | The owner's decision, once it has been green for a while: remove the line |
| `popup_done` | The popups of the backend take no grab, and sway did not dismiss one by itself in any run | The Mutter and KWin traces show whether those compositors send it to a popup without a grab (when its parent loses the focus or is closed); else a test compositor: nothing cheaper is known |
| A real input method (IBus, Fcitx 5) with its key grab and candidate window | The test input method is the example's own second client | `apt-get install fcitx5 fcitx5-chinese-addons` in the virtual machine and the catalog in its session; a person types |
| The portal file chooser with the exported parent | No session bus and no portal under the headless compositor: the factory gives nothing and the managed dialogs answer | The catalog in the GNOME session of the virtual machine (it has `xdg-desktop-portal-gnome`): open a file dialog and see that it is attached to the window; `busctl --user monitor org.freedesktop.portal.Desktop` shows the `wayland:` handle |
| A hardware GPU (the `wl_egl_window` path on Mesa's Intel or AMD drivers, and on NVIDIA, whose workarounds the EGL surface carries) | The runners and the virtual machine render with `llvmpipe` or the paravirtual driver of the machine (measured: "the driver the machine chooses by itself") | `wayland_window --smoke --mode=egl` under `scripts/wayland/sway-run.sh` on any Linux machine with such a GPU (the script forces the software renderer of the compositor only, not of the client); for NVIDIA there is no way without the hardware |
| Touch | wlroots has no protocol that synthesizes it; the handler is covered by the tests of its framing | A touch screen. Without one: a virtual touch device through `uinput` (`apt-get install evemu-tools`, as root) reaches a compositor only on its libinput backend, so in the session of the virtual machine, not under the headless compositor; KWin has a fake input protocol with touch for clients it authorizes |
| A drag between two clients; a bitmap through the clipboard | The test client has no surface to drop on | A second surface in the test client of the example (code, no tooling): the first thing to add to the smoke mode |
| A compositor restart with popups open or a text box focused | Never tried | `swaymsg exit` and a new sway on the same socket name while the example runs: a phase of the smoke mode, no tooling |
| A popup with a grab | The backend takes none, as upstream | Nothing to verify until upstream changes |
| The dmabuf swapchain on a hardware GPU (Intel, AMD; NVIDIA with GBM) and under KWin | **Verified on the virtio GPU of the virtual machine** under sway with OpenGL ES and under a headless Mutter (section 11); the runners have no render node with a driver of Mesa | `wayland_window --smoke --mode=dmabuf` under `scripts/wayland/sway-run.sh --renderer gles2 --render-node /dev/dri/renderD128` on any Linux machine with such a GPU; the catalog with `FERROUI_CATALOG_WAYLAND=dmabuf` in a desktop session |
| The swapchain with explicit synchronization, fewer than four buffers, a format other than `ARGB8888`, a buffer of several planes | Upstream has none of it at the tracked commit (four slots, implicit synchronization, `ARGB8888` only); a multi-plane buffer needs a modifier that gives one, which only a hardware driver does | The hardware run above shows the plane count of the buffers in a trace (`WAYLAND_DEBUG=1`: the `add` requests per `create`) |
| A display another library opened, with that library rendering beside the backend (GTK or SDL on the same `wl_display`) | The phase `foreign` checks a second worker of this backend beside a `wayland-client` connection that does nothing | An embedding example with another toolkit; upstream has no caller of the member either |

## 13. Deviations

Recorded in `DEVIATIONS.md`, section "Wayland platform".
