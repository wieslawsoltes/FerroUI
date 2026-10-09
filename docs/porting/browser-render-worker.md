# Browser render worker: the design of stage B2

Status: design, 2026-10-08. Nothing on this page has been built or run; it was written by reading the two source trees. It is the content of stage B2 of `render-thread.md` (section 5), and it builds on stage B1 (`browser-platform.md`, section 21: the opt-in threaded build and the cross-origin isolation of the site).

Upstream reference: the checkout at `16572aeff1`, `src/Browser/Avalonia.Browser`, `src/Avalonia.Base` and `src/Shared`. Paths under "upstream" below are relative to `src/` of that checkout.

Markers:

- **[U]** read in the upstream source; the file is named.
- **[M]** read in the code of this repository; the file is named.
- **[D]** behaviour that Emscripten, wasm-bindgen or the browsers document, written from knowledge of that documentation and not checked in this repository. Every **[D]** that the design rests on is checked by a step of section 6.
- **[I]** inferred from what was read; the source does not state it.
- **[H]** a hypothesis that a step must verify before the next one builds on it.

## Summary

- Upstream's threaded mode has three kinds of threads: the browser's main thread (the DOM and the script modules), a managed UI thread with a blocking dispatcher loop, and a render worker that owns the canvas through `OffscreenCanvas`. What carries DOM calls from the UI thread to the main thread, and DOM events back, is the .NET runtime, not code of the framework.
- This port has no such carrier: a wasm-bindgen handle and every function of the script module belong to the thread they were made on. **The design therefore keeps the UI thread on the browser's main thread and moves only rendering to a worker.** This is a deviation from upstream and from the one-line description of B2 ("the event grouper queue, the blocking dispatcher"): neither is needed while the UI thread is the main thread. Moving the UI to a worker is left as a later, optional stage (section 6, "Not in B2").
- **Decided by the owner on 2026-10-08: the UI thread stays on the browser's main thread and rendering moves to a worker**, as this page recommends. Running the UI on a worker, as upstream does, is not part of B2.
- The compositor model is **strict confinement to the render thread** (`use_ui_thread_for_synchronous_commits` false, the wait of R4.1), not the lock model of the desktop: a WebGL context and a transferred canvas exist in one worker only.
- The first step is a test page that transfers a canvas to a thread of the port's own module and clears it with WebGL through the port's GL interface, before the compositor is involved.

## 1. What upstream does

### Threads

| Thread | What runs on it | Source |
|---|---|---|
| The browser's main thread | The DOM, the script module `avalonia.js` (input subscriptions, `ResizeObserver`, canvas creation), and the start of the application: `StartBrowserAppAsync` awaits `PreSetupBrowser` there and then, in the threaded mode, starts the UI thread and awaits a completion source that the UI thread sets once set-up is done | **[U]** `Browser/Avalonia.Browser/BrowserAppBuilder.cs` (`StartBrowserAppAsync`, `PreSetupBrowser`) |
| The managed UI thread | A `new Thread(...)` whose body calls `builder.SetupWithLifetime(lifetime)` and then `builder.Instance.Run(CancellationToken.None)`. The dispatcher of that thread is `ManagedDispatcherImpl`, whose `RunLoop` blocks on an `AutoResetEvent` (`_wakeup.WaitOne()`) when it has nothing to do | **[U]** `BrowserAppBuilder.cs`; `Browser/Avalonia.Browser/WindowingPlatform.cs` (`Register`: `Dispatcher.InitializeUIThreadDispatcher(new ManagedDispatcherImpl(new ManualRawEventGrouperDispatchQueueDispatcherInputProvider(EventGrouperDispatchQueue)))`); `Avalonia.Base/Platform/ManagedDispatcherImpl.cs` |
| The render worker | Started by `RenderWorker.InitializeAsync` through the runtime's `JSWebWorker.RunAsync` (reached with an unsafe accessor: "not part of ref assemblies and is not a stable API"). Its body imports the main script module into the worker (`AvaloniaModule.ImportMainToWorkerContext`), installs the message handler of the render targets (`WebRenderTargetRegistry.initializeWorker`), records `pthread_self()` in `RenderWorker.WorkerThreadId`, starts the render timer on itself (`BrowserSharedRenderLoop.RenderTimer.StartOnThisThread()`) and then awaits a completion source that never completes, so that the worker stays alive ("Never surrender") | **[U]** `Browser/Avalonia.Browser/Rendering/RenderWorker.cs`; `Browser/Avalonia.Browser/Interop/AvaloniaModule.cs` (`ImportMainToWorkerContext`: "a web worker ... needs its own copy of every module it calls into") |

Whether threads are on is decided at run time: `BrowserWindowingPlatform.IsThreadingEnabled` reads the non-public `Thread.IsThreadStartSupported`, or starts a thread and catches the exception **[U]** (`WindowingPlatform.cs`, `DetectThreadSupport`). The build switch is the SDK's `WasmEnableThreads`; the only thing the package adds for it is `PThread` in the exported runtime methods of Emscripten **[U]** (`Browser/Avalonia.Browser/build/Avalonia.Browser.targets`). `BrowserPlatformOptions.PreferManagedThreadDispatcher` is obsolete and has no effect: "The managed thread dispatcher is always used when WasmEnableThreads is enabled" **[U]** (`BrowserAppBuilder.cs`).

**Where the managed main thread runs, and why the UI thread may block.** `Atomics.wait` is not allowed on the main thread of a page. Upstream's answer in its own code is that the dispatcher loop does not run there: it runs on the `new Thread` above, which is a pthread and therefore a web worker **[I]** (the framework only says `new Thread`; that a managed thread is a worker is the runtime's doing). What the upstream source does **not** show: on which thread the runtime runs the managed `Main` (the browser's main thread or a worker of the runtime), how a `[JSImport]` call made on the UI thread reaches the DOM of the main thread, and on which thread a `[JSExport]` called by the page's script executes. All three belong to the .NET runtime. The only traces in the framework are two comments: "Capture initial GlobalThis, so we can use it as a contextual bridge between threads" **[U]** (`WindowingPlatform.cs`) and "in MT mode callbacks may arrive on a thread without one [a dispatcher]" **[U]** (`Browser/Avalonia.Browser/Interop/JsCallbackHelper.cs`).

### The canvas

The canvas is created on the main thread as an ordinary element and its control is **transferred**:

1. The top level creates its surface with `CanvasHelper.CreateRenderTargetSurface(container, modes, topLevelId, RenderWorker.WorkerThreadId)` **[U]** (`Rendering/RenderTargetBrowserSurface.cs`, `Create`; `Interop/CanvasHelper.cs`). The thread id is 0 without threads.
2. `CanvasSurface.create` makes the canvas and attaches it; its constructor calls `WebRenderTargetRegistry.create(threadId, canvas, modes)` and observes the size of the element **[U]** (`webapp/modules/avalonia/rendering/canvasSurface.ts`).
3. With a thread id, the registry finds the `Worker` of that pthread in Emscripten's `PThread.pthreads` table (`module.PThread`, hence the exported runtime method), calls `canvas.transferControlToOffscreen()` and posts `{ avaloniaCmd: "registerCanvas", canvas: offscreen, modes, id }` to the worker with the canvas in the transfer list. It keeps the element and the worker under the id and creates **no** render target on the main thread **[U]** (`webapp/modules/avalonia/rendering/webRenderTargetRegistry.ts`, `create`).
4. In the worker, `initializeWorker` has replaced `self.onmessage` with a handler that takes `registerCanvas` (it creates the render target from the `OffscreenCanvas` with the same list of modes and stores it under the id), takes `unregisterCanvas` (it deletes the entry) and passes every other message to the handler that was there before, which is Emscripten's **[U]** (same file).
5. The managed side asks for the target by id (`WebRenderTargetRegistry.getRenderTarget`, called from `BrowserRenderTarget.GetRenderTarget`) lazily, from `BrowserPlatformGraphics.Target` **[U]** (`Rendering/WebRenderTarget.cs`, `Rendering/RenderTargetBrowserSurface.cs`). The lookup is answered by the registry of whichever thread's script the call runs in: the worker's registry has the target, the main thread's has none **[I]**.

Nothing in the upstream tree sends `unregisterCanvas`, and `CanvasSurface.destroy` is empty **[U]** (a search of `src/Browser` finds the string only in the handler).

### Rendering modes on the threaded path

All three. The worker creates its target with the same function as the main thread, `createRenderTarget(canvas, modes)`, which tries the modes in order (`WebGL2`, `WebGL1`, `Software2D`) and falls back to software **[U]** (`webRenderTargetRegistry.ts`). `WebGlRenderTarget` and `SoftwareRenderTarget` take `HTMLCanvasElement | OffscreenCanvas` **[U]**. The software target has one branch for the threaded mode: "Need to make a copy if using MT, ImageData can't consume shared arrays", taken when the canvas is an `OffscreenCanvas` **[U]** (`webapp/modules/avalonia/rendering/softwareRenderTarget.ts`). `WebGlContext` records the thread that created it and throws "Call from invalid thread" from `EnsureCurrent` and `MakeCurrent` on any other **[U]** (`Rendering/BrowserWebGlRenderTarget.cs`).

### Frame pacing

`requestAnimationFrame` on the thread the render timer was started on. `TimerHelper.runAnimationFrames` calls `self.requestAnimationFrame` in a perpetual loop and calls the export `TimerHelper.JsExportOnAnimationFrame` **[U]** (`webapp/modules/avalonia/timer.ts`, `Interop/TimerHelper.cs`). Without threads, `BrowserRenderTimer` starts that loop on the thread that first sets `Tick`; with threads it does not, and only `RenderWorker` starts it, on the worker **[U]** (`Rendering/BrowserRenderTimer.cs`, the `Tick` setter and `StartOnThisThread`). So in the threaded mode the ticks, and with them `ServerCompositor.Render`, come from `requestAnimationFrame` of the render worker.

One thing in the source does not fit the rest and is recorded as read: the shared timer is created as `new BrowserRenderTimer(false)`, so `RunsInBackground` is **false** in both modes **[U]** (`Rendering/BrowserSharedRenderLoop.cs`), and the compositor of a surface is created with the default `useUiThreadForSynchronousCommits: false` **[U]** (`RenderTargetBrowserSurface.cs`, `CreateCompositor`; `Avalonia.Base/Rendering/Composition/Compositor.cs`). `MediaContext.SyncWaitCompositorBatch` waits for the render thread only when `UseUiThreadForSynchronousCommits` is false **and** `Loop.RunsInBackground` is true, and otherwise calls `compositor.Server.Render` on the calling thread **[U]** (`Avalonia.Base/Media/MediaContext.Compositor.cs`); `Compositor.GetRenderInterfacePublicFeatures` likewise calls the render-thread function directly when the loop does not run in the background **[U]** (`Compositor.cs`). Read literally, a synchronous commit of the UI thread in the threaded mode renders on the UI thread, where the WebGL context refuses the call and where the registry has no render target. The source does not show how this is reconciled, and no sample in the tree runs the threaded mode: `samples/ControlCatalog.Browser/ControlCatalog.Browser.csproj` sets `WasmEnableThreads` to `false` **[U]**. This design does not copy the flag (section 3).

### Input, and why a grouper queue exists

- The script subscribes to DOM events on the main thread and calls the exports `InputHelper.OnPointerMove`, `OnKeyDown` and the rest **[U]** (`webapp/modules/avalonia/input.ts`). Each export returns a `Task` and runs its handler through `RedirectInputAsync`, which calls the top level's input handler **on the thread the export is called on**, after installing the dispatcher's synchronisation context if that thread has a dispatcher **[U]** (`Interop/InputHelper.cs`, `Interop/JsCallbackHelper.cs`). Input is not posted to the UI dispatcher by the framework; compare `CanvasHelper.OnSizeChanged`, which in the threaded mode does `Dispatcher.UIThread.InvokeAsync(...)` **[U]** (`Interop/CanvasHelper.cs`).
- `BrowserInputHandler.ScheduleInput` hands each raw event to a `RawEventGrouper` when `BrowserWindowingPlatform.EventGrouperDispatchQueue` exists (threaded mode only), and otherwise dispatches it at once **[U]** (`BrowserInputHandler.cs`).
- The grouper merges consecutive `Move` and `TouchUpdate` events of one device with the same modifiers into one event with intermediate points, and puts events into a queue **[U]** (`Shared/RawEventGrouping.cs`: "maintains an input queue for backends that handle input asynchronously. While doing that it groups Move and TouchUpdate events so we could provide GetIntermediatePoints API"). In the browser backend the queue is `ManualRawEventGrouperDispatchQueue`, and the dispatcher loop drains it one event at a time, after signals and timers and before background work (`ManagedDispatcherImpl.RunLoop`, through `IManagedDispatcherInputProvider`) **[U]**.
- So the queue exists because, with a blocking UI loop, input can no longer be handled inside the DOM event: it has to be parked until the loop takes it, and pointer moves that pile up meanwhile have to be merged. With the grouper, the browser's own coalesced events are not used: "Rely on native GetCoalescedEvents only when managed event grouping is not available" **[U]** (`BrowserInputHandler.cs`).
- Two things the source does not show. `ManualRawEventGrouperDispatchQueue.Add` takes no lock and wakes nobody (a plain `Queue`), and `RunLoop` only looks at the queue when something else wakes it **[U]**; which thread calls `Add` is the runtime's matter (above). And the exports return `args.Handled` straight after `ScheduleInput`, so with the queue the answer is given before the event has been processed **[I]**; the script calls `preventDefault` from the `.then` of the key handlers and unconditionally for pointer events **[U]** (`input.ts`).

### What was looked for and not found

- No use of `Atomics` and no blocking primitive in the upstream script modules. `SharedArrayBuffer` appears once, in `webapp/modules/avalonia/stream.ts` (`isSharedArrayBuffer`: a write falls back to a copy when the heap is shared).
- No script of the framework that runs the UI on a worker: the UI thread is a managed `Thread`.
- No code that carries a DOM call from the UI thread to the main thread, or an input event from the main thread to the UI thread.
- No sender of `unregisterCanvas`; no release of the worker's render target.
- No test and no sample of the threaded mode.

## 2. What Emscripten pthreads give this port, and what they do not

### What a thread is

| Fact | Consequence for the port |
|---|---|
| A pthread is a web worker that loads the script of the module again and instantiates the module over the **same** `WebAssembly.Memory` **[D]**. Stage B1 saw it work for a module with two exports and no imports **[M]** (`render-thread.md`, "B1 validated") | Rust statics, the heap, atomics, `Arc`, `Mutex` and channels are shared. `thread_local!` is per thread |
| Each worker has its own JavaScript global and its own instances of every script module **[D]** | The worker has its own copy of `ferroui.js` (its own `WebRenderTargetRegistry`, `FerroExports`), its own Emscripten `Module` and `GL` object, and no `document` or `window` |
| A wasm-bindgen `JsValue` is an index into a table that lives in the script of one thread **[D]** | A `JsObject` (`interop/mod.rs`: `pub type JsObject = wasm_bindgen::JsValue` **[M]**) made on one thread means nothing, or something else, on another. `JsValue` is not `Send`; `ThreadBound` is what lets the render targets hold one today **[M]** (`rendering/browser_web_gl_render_target.rs`, `rendering/browser_software_render_target.rs`) |
| A wasm-bindgen import is a function of the script module as loaded in the calling thread's global **[D]** | `get_render_target(id)` called on the worker asks the worker's registry. `dom_helper`, `input_helper`, `storage_helper` and the rest only work on the main thread: their script needs the DOM |
| A wasm-bindgen export is a function of the module instance of the thread whose script calls it **[D]** | `TimerHelper_JsExportOnAnimationFrame` called by the worker's `ferroui.js` runs on the worker. `timer_helper.rs` keeps its subscribers in a `thread_local!` **[M]**, so a timer started on the worker ticks on the worker without change |
| A thread whose start function returns exits, and its worker goes back to the pool **[D]** | A render worker that lives on events (`requestAnimationFrame`, messages) must return to its event loop without ending the thread: `emscripten_exit_with_live_runtime` or `emscripten_runtime_keepalive_push` **[D]**. How that combines with a thread started by `std::thread` is **[H]**, step 1 |
| A worker only starts running when the thread that created it returns to the browser, unless it comes from the pool **[D]** | `-sPTHREAD_POOL_SIZE` (already set, default 2 **[M]** `scripts/build-browser.sh`): one render worker and one spare |
| WebGL contexts and Emscripten's `GL` table are per worker **[D]**; a canvas whose control was transferred can only be drawn to in the worker that received it, and its element can no longer be given a width or height on the main thread **[D]** | The render target is usable from one thread. This is what rules out the lock model (section 3) |

### Blocking on the browser's main thread

`Atomics.wait` throws on the main thread of a page. Emscripten emulates a blocking wait there by spinning: a futex wait on the main thread is a busy loop that also runs the calls other threads proxy to it **[D]**. Rust's `Mutex`, `Condvar` and channel waits end in that futex on this target **[I]**. So a wait on the main thread is possible but (a) freezes the page for its duration, and (b) never ends if the thread it waits for needs something that only happens when the main thread returns to the browser: the start of a thread outside the pool, the delivery of a message the main thread has posted to itself, and possibly the worker's own animation frames (**[H]**, step 3). The rule B1 wrote for the test page stays the rule: the main thread does not wait for anything of unbounded length.

### The Emscripten features in question

| Feature | What it is | Used today | In B2 |
|---|---|---|---|
| `-pthread`, `+atomics,+bulk-memory`, `-Zbuild-std`, `-sPTHREAD_POOL_SIZE`, `-sENVIRONMENT=web,worker` | The threaded build | Yes, with `--threads` **[M]** (`scripts/build-browser.sh`) | Unchanged |
| `-sEXPORTED_RUNTIME_METHODS=GL,HEAPU8` | What the script side takes from the module | Yes **[M]** (`.cargo/config.toml`) | Add `PThread` (the registry finds the worker of a pthread through it, as upstream) and `wasmMemory` (section 5), for the threaded build |
| `-sALLOW_MEMORY_GROWTH=1` | Growable memory | Yes, in both builds **[M]** (`.cargo/config.toml`) | Section 5 |
| `-sWASM_BINDGEN`, `-sMODULARIZE`, `-sEXPORT_ES6`, `-sINVOKE_RUN=0` | The glue merged into the module script; a factory; nothing runs at instantiation | Yes **[M]** | Unchanged. The start of the application stays an export the host page calls |
| `-sPROXY_TO_PTHREAD` | Runs `main` on a pthread and leaves the main thread to serve proxied calls **[D]** | No | **Not used.** The module has no `main` that runs (`INVOKE_RUN=0`, exports called by the page), and the option only helps if the UI moves to a worker. See "Not in B2" |
| `-sOFFSCREENCANVAS_SUPPORT` | Lets Emscripten's own HTML5 API (`emscripten_webgl_create_context`, `emscripten_pthread_attr_settransferredcanvases`) hand a canvas to a pthread at its creation and create a context on it there **[D]** | No | **Not needed [H]**: the port's script transfers the canvas itself (as upstream) and registers the context with `GL.registerContext` in the worker, the way `webGlRenderTarget.ts` does on the main thread today **[M]**. It is the fallback if `GL.registerContext` does not work in a worker (step 1) |
| `-sOFFSCREEN_FRAMEBUFFER` | The other answer to "GL from a pthread": the context stays on the main thread and every GL call is proxied to it **[D]** | No | **Not used.** It keeps rasterisation on the main thread, which is what B2 removes |
| `emscripten_set_main_loop`, `emscripten_request_animation_frame_loop` on a worker | Emscripten's frame loops; on a pthread they use the worker's `requestAnimationFrame` where the browser has one **[D]** | No. The port's loop is `TimerHelper.runAnimationFrames`, which already calls `self.requestAnimationFrame` **[M]** (`webapp/modules/ferroui/timer.ts`) | The port's loop, on the worker. No Emscripten main loop |
| Proxying to the main thread (`emscripten_proxy_async`, `emscripten_async_run_in_main_runtime_thread`) | Runs a C function on another thread's event loop **[D]** | No | Used in two places: the worker wakes the UI dispatcher (section 3), and the UI thread asks the worker for a frame out of turn (section 3). Both carry no JavaScript object, only a function and an integer |

## 3. The mapping onto the port's compositor

### Why the UI thread stays on the browser's main thread

Upstream's UI thread is a worker. For that, every call the UI makes into the page has to cross to the main thread, and every event has to cross back. Upstream gets both from its runtime (section 1). The port would have to build them: `interop/` is eleven files of wasm-bindgen imports that take and return `JsObject` handles **[M]**, and rule 4 of `browser-platform.md`, section 5, makes calls from the page synchronous so that `preventDefault` can be decided inside the event, which a second thread cannot do. Emscripten's `PROXY_TO_PTHREAD` proxies C library calls, not wasm-bindgen imports.

Keeping the UI on the main thread costs one thing: layout and input handling still share a thread with the page. It gains the purpose of the render thread on the desktop: while a frame is rasterised, the UI thread is free. Whether the remaining UI work on the main thread is a problem is what B3 measures; if it is, "Not in B2" describes the further stage.

Consequences, each a deviation from upstream to record in `DEVIATIONS.md` when implemented:

| Upstream (threaded) | This design |
|---|---|
| UI on a managed thread with `ManagedDispatcherImpl` | UI on the main thread with `BrowserSingleThreadedDispatcherImpl`, plus a wake-up that works from another thread (below) |
| `ManualRawEventGrouperDispatchQueue` and `RawEventGrouper` | Not used: input is dispatched inside the DOM event, as today **[M]** (`browser_input_handler.rs`: "the original groups raw events only when it runs with a managed dispatcher on another thread") |
| `BrowserRenderTimer(false)` in both modes | `runs_in_background` is true when the timer is started on the worker |

### Lock model or strict confinement

The desktop uses the lock model (R5, decided 2026-10-08): the UI thread also renders at the synchronous points, under the compositor lock. That needs a render target that either thread can draw to. In the browser neither kind qualifies: a WebGL context and a 2D context of a transferred canvas exist only in the worker **[D]**, and upstream's `WebGlContext` and the port's refuse any other thread **[U]** **[M]** (`verify_access` in `browser_web_gl_render_target.rs`).

So the browser takes the other model `render-thread.md` names for it: **strict confinement.** The compositor is created with `Compositor::with_render_thread` and `use_ui_thread_for_synchronous_commits` **false**; the render loop's timer reports `runs_in_background()` true; `MediaContext::sync_wait_compositor_batch` then takes its first branch and waits for the `Processed` or `Rendered` completion of the batch instead of rendering **[M]** (`src/FerroUI.Base/media/media_context.rs`, the condition of R4.1). The compositor lock still exists in this mode; what changes is that the UI thread never renders under it.

### What blocks, where, and whether it may

| Wait | Thread | Today | In B2 |
|---|---|---|---|
| `BatchCompletion::wait` at a synchronous commit (first show, the resize a top level reports, the disposal of a composition target) **[M]** (`transport/batch.rs`, a `Condvar`) | UI = browser main | Not taken: the UI thread renders | Taken. It is a spin on the main thread (section 2). It is bounded only if the worker renders without waiting for its next animation frame, so the UI thread **asks for a frame out of turn** before it waits: a function proxied to the worker that ticks the render loop once. With that, the page is held for the time of one frame, which is what the same commit costs today, when the UI thread renders the frame itself. Legal **[D]**; to be shown by step 3 before anything depends on it |
| `CompositorLock::enter` from the UI thread **[M]** (`server/compositor_lock.rs`, a `Mutex` and `Condvar`): `CompositionTarget::new`, `CompositionTarget::set_debug_overlays`, `Compositor::try_get_render_interface_feature`, `try_get_composition_gpu_interop`, the three places of `composition_interop.rs`, `Compositor::drop` | UI = browser main | No contention: one thread | Each can meet a frame in progress and spin for the rest of it. Bounded by one frame; all are rare (start-up, disposal, a feature query). Each must be shown not to touch the render target while it holds the lock (next table) |
| The worker waiting for the compositor lock | Render worker | n/a | An ordinary blocking wait (`Atomics.wait` is allowed in a worker **[D]**) |
| The worker waiting for the main thread | Render worker | n/a | **Must not exist.** A frame may not make a call that Emscripten proxies synchronously to the main thread while the main thread may be waiting for that frame. Emscripten's main-thread wait serves proxied calls **[D]**, so the known cases do not deadlock, but each is a hidden round trip per frame; step 6 looks for them (a counter of proxied calls during a frame) |

If step 3 shows that the wait cannot be made safe (for example, a worker cannot be made to render out of turn, or the spin is long), the fallback is a third behaviour of the synchronous points for this platform: commit and do not wait. The browser has no window that the system presents in step with the UI thread, which is what the wait exists for on the desktop; the cost would be one frame of a stretched canvas during a resize. It is a change in `MediaContext` and is not designed further until the measurement asks for it.

### What the base library still has to change for strict confinement

Read from the code; each is a place where the UI thread would reach the render target or the graphics context in the render-thread mode.

| Where | Problem in the browser | Direction |
|---|---|---|
| `Compositor::try_get_render_interface_feature` **[M]** (`compositor.rs`): under the lock, when nothing is cached and the render interface is ready, it calls `rt_get_render_interface_features` on the calling thread | That creates the backend context (the Skia GL context) on the UI thread | With `use_ui_thread_for_synchronous_commits` false: answer from the cache, else `None`, and let a job of the render thread fill the cache. This is upstream's shape for a background loop (`InvokeServerJobAsync(Server.RT_GetRenderInterfaceFeatures)`) **[U]** (`Compositor.cs`, `GetRenderInterfacePublicFeatures`) |
| `Compositor::drop` releases the server graph on the thread of the compositor, under the lock **[M]** (R5.5: so that the render thread never drops it) | The graph holds the Skia GL objects; dropping them on the main thread makes GL calls where no context is current | In the strict mode the release is a last job of the render thread; the UI thread drops only the handle. `ThreadBound` leaks rather than drops on the wrong thread, which is the safety net, not the design |
| `ServerCompositor::new(gpu, ...)` takes `Option<Rc<dyn IPlatformGraphics>>` made on the UI thread **[M]** (`compositor.rs`, `with_render_thread`) | `BrowserPlatformGraphics` has cells the UI thread writes (`canvas_size`) and a cell the render thread would fill (`target`) **[M]** (`render_target_browser_surface.rs`) | Section 4: the platform graphics of the threaded mode holds only shared, thread-safe state and resolves the target per thread |
| `IPlatformGraphicsReadyStateFeature::is_ready` and `uses_contexts` are asked by both threads | Their answer today comes from the script registry of the calling thread | Section 4: answered from shared atomics the worker publishes |
| `CompositionTarget::new` and `set_debug_overlays` enter the lock on the UI thread **[M]** | They capture the render interface and build the diagnostic text renderer: no GL | No change; covered by the audit test of step 4 |
| Offscreen rendering asked for by the UI thread: `create_composition_visual_snapshot`, drawing surfaces, the GPU interop | Each needs the GL context | They already run as jobs or batches of the server (R5.6) **[M]**; step 4 verifies it with a surface that panics off its thread |

### The dispatcher wake-up from the worker

The completions of batches and of server jobs are delivered to the UI thread through the dispatcher (R2). `BrowserSingleThreadedDispatcherImpl::signal_handle` returns a handle that forwards to the instance in a `thread_local!` **[M]** (`browser_single_threaded_dispatcher_impl.rs`): called on the worker it finds no instance and the signal is lost. In the threaded build the handle becomes a real cross-thread wake-up: an atomic "signalled" flag, and a function proxied to the main thread (`emscripten_async_run_in_main_runtime_thread` or the proxying queue **[D]**) that calls what `BrowserSingleThreadedDispatcherImpl_OnSignaled` calls today. The script-side scheduling (`postTask`, `MessageChannel`) stays as it is for signals raised on the main thread.

### The wasm-bindgen glue in a worker (the first doubt of B1)

The doubt has two parts, and step 1 settles both with one page:

1. **Does the script of the module load in a worker when its glue imports `./ferroui.js`?** The import is evaluated in every worker of the pool. `ferroui.js` must therefore load without a DOM. Read from the sources, it should **[H]**: the only top-level statements that look at the page are guarded (`caretHelper.ts`: `typeof window !== "undefined"`), `SingleThreadedDispatcherHelper` only touches `scheduler` and `MessageChannel`, which workers have, and the storage bundle is imported on first use **[M]**. Settled by: the pool of the test page of step 1 starts without an error in a module that links the whole browser crate.
2. **Does a wasm-bindgen import called on a worker reach the worker's copy of the script, and does the worker's script reach the worker's module?** The first is how module imports work **[D]**. The second needs work: the script side finds the module through `FerroExports.attach(runtime)`, which the host page calls on the main thread **[M]** (`ferroExports.ts`); in a worker nobody calls it, so `FerroExports.runtime?.GL` and `HEAPU8` are undefined there. To settle: a small script added to the module for the threaded build (`--pre-js`) that, when it runs in a pthread, imports `./ferroui.js` and attaches the worker's own `Module` before the thread's start function runs **[H]**.

If either fails and cannot be repaired in the glue, the fallback is the one `browser-platform.md`, section 5, already names: the handful of functions the worker needs (get the target, set its size, make the context current, put pixels, start the frame loop) move to an Emscripten `--js-library`, called through `extern "C"` with integers only. The worker then calls no wasm-bindgen import at all.

### The two service workers

A scope has one service worker, and both the platform's (`ferroui-sw.js`, for the streamed saves of the file picker polyfill) and the isolation worker (`ferroui-coi-sw.js`) want the directory of the site **[M]** (`browser-platform.md`, section 21). Settled by making them one:

- The `fetch` handler of `scripts/browser/threads/ferroui-coi-sw.js` moves into `webapp/modules/ferroui-sw.ts`, after the handler of the polyfill: a request the polyfill does not answer is fetched and handed on with the three headers. The header part is switched on by a query parameter of the worker's own address (`ferroui-sw.js?coi=1`), so a site without threads keeps the behaviour it has.
- `ferroui-threads.js` registers `./ferroui-sw.js?coi=1` instead of the separate file; `register_ferro_service_worker` in a threaded site registers the same address, so both registrations name one script and the second is a no-op **[D]**. `ferroui-coi-sw.js` is deleted.
- Test: `thread_spawn.test.mjs` unchanged in what it asserts; `storage_view.test.mjs` with `RegisterServiceWorker=true` on a threaded site without headers asserts `crossOriginIsolated` and a streamed save in the same page.

## 4. The surface objects

### What exists today

`BrowserWebGlRenderTarget` holds `ThreadBound<WebGlRenderTargetState>`: the `JsObject` of the script's render target, the size getter (`Rc<dyn Fn() -> (PixelSize, f64)>`), the `GlInfo` and the `Rc<WebGlContext>`. `BrowserSoftwareRenderTarget` holds the `JsObject` and the size getter the same way. Both are created by `get_render_target(id, size_getter)` on the UI thread, from `BrowserPlatformGraphics::target()`, and the top level publishes them in a shared cell **[M]** (`rendering/*.rs`, `browser_top_level_impl.rs`, `publish_render_surfaces`). A frame on another thread panics at `ThreadBound::get`.

### What replaces it

Three objects, by who owns them:

| Object | Thread | Holds | Replaces |
|---|---|---|---|
| `BrowserSurfaceShared` (new, `Arc`, `Send + Sync`) | Both | The target id; the canvas size in device pixels and the scaling; the kind of the target once the worker knows it (none yet, WebGL, software); whether the target exists; whether the top level is disposed. All atomics, the size and scaling under one small lock so that a frame never sees a new size with an old scaling | The `canvas_size` cell of `BrowserPlatformGraphics` and the size getters |
| The render surface handed to the compositor (`BrowserRenderSurface`, `Arc`, `Send + Sync`) | Created on the UI thread, used by the render thread | Only the `Arc<BrowserSurfaceShared>`. `is_ready` reads the atomics. `try_get_surface_kind` and `as_framebuffer_surface` look the thread's target up (next row) and answer "not ready" on a thread that has none | `BrowserWebGlRenderTarget` and `BrowserSoftwareRenderTarget` as surfaces, and their `ThreadBound` |
| The thread's render targets (a `thread_local!` table by target id) | The thread that draws: the worker in the threaded mode, the main thread otherwise | What the `ThreadBound` state holds today: the `JsObject`, the `GlInfo`, the `WebGlContext`, the retained framebuffer. Filled lazily by `get_render_target(id)`, which asks the registry of the calling thread's script, as upstream's `BrowserPlatformGraphics.Target` does **[U]** | The state structs |

The same three objects serve the build without threads: the main thread's table has the target, and nothing waits. `ThreadBound` disappears from the browser render targets; `WebGlContext::verify_access` stays.

`BrowserPlatformGraphics` keeps its role and loses its cells: `is_ready` is "the target exists and the size is not empty" from the shared state (as upstream's `IsReady` **[U]**); `uses_contexts` is the kind; `get_shared_context` resolves the thread's target and is only called by the thread that renders.

### Who creates, who transfers

1. The UI thread calls `CanvasSurface.create(container, modes, topLevelId, threadId)`, with the render worker's pthread id in the threaded mode (`interop/canvas_helper.rs` gains the argument, as upstream's `CreateRenderTargetSurface`).
2. The script on the main thread creates and attaches the canvas, starts observing its size, and in `WebRenderTargetRegistry.create` either creates the target (no thread) or transfers control and posts `registerCanvas` to the worker of the pthread (the worker branch of upstream's registry, ported as it is).
3. The worker's handler (`WebRenderTargetRegistry.initializeWorker`, installed by the render worker when it starts) creates the target from the `OffscreenCanvas` and then calls a new export, `CanvasHelper_OnRenderTargetRegistered(id, kind)`, which publishes "exists" and the kind in the shared state and asks the render loop for a frame. Upstream has no such call: its managed side polls the registry from `IsReady` on each frame. The port cannot poll from the UI thread (the registry of the main thread never has the target), so the worker reports.
4. Disposal: the UI thread marks the shared state disposed, disposes the composition target (a synchronous commit: the worker releases the Skia surface), and the script posts `unregisterCanvas`, upon which the worker drops the script target and the thread's table entry. Upstream never sends the message (section 1); the port does, or the WebGL context of every closed view stays alive in the worker.

The render worker must be running, with its handler installed, before the first `registerCanvas` is posted: a message that arrives earlier goes to Emscripten's handler, which does not know it **[D]**. Upstream awaits `RenderWorker.InitializeAsync()` in `PreSetupBrowser` **[U]**. The port's start function cannot wait, so the script's registry holds `registerCanvas` messages back until the worker has reported its pthread id (one more export, `RenderWorker_OnStarted`, called on the main thread through the dispatcher wake-up), and the surface is simply not ready until then.

### What the UI thread still needs from the canvas

| Value | Source | How it crosses |
|---|---|---|
| Size in device pixels and scaling, and their changes (window resize, zoom, a move to a screen with another device pixel ratio) | `ResizeObserver` on the canvas **element**, on the main thread, unchanged: the element stays in the document after its control is transferred **[D]**, and upstream observes it the same way **[U]** (`canvasSurface.ts`, `resizeHandler.ts`) | `CanvasHelper_OnSizeChanged` on the UI thread, as today, writes the shared state; the top level raises `resized` and `scaling_changed`. The worker reads the shared state at the start of each frame and sets the size of the `OffscreenCanvas` (`WebRenderTarget.setSize` in the worker's script), which is the only place it can be set |
| Visibility of the document | `visibilitychange`, on the main thread, unchanged (`DomHelper_DocumentVisibilityChanged`) | Not sent to the worker: the UI thread stops committing, and the browser stops the worker's animation frames for a hidden page **[H]** (step 6 checks that a hidden page does not render) |
| Whether the target exists, and its kind | The worker | Atomics in the shared state (step 3 of the list above) |
| Loss of the WebGL context | `webglcontextlost` on the `OffscreenCanvas`, in the worker | Not handled today in either tree (`is_lost` is "TODO: Implement" **[U]** **[M]**). Out of scope; an atomic in the shared state is where it would go |
| The picture itself (a screenshot, a snapshot of a visual) | The compositor | A job of the render thread, as on the desktop |
| The canvas element as the control handle, the cursor, pointer capture, the input element | The container element, not the canvas | Unchanged, main thread |

## 5. Memory growth

With threads the memory is a `SharedArrayBuffer`. When any thread grows it, the buffer object other threads hold is not detached: it stays valid and keeps its **old length** **[D]**. A view made from it still reads and writes the old range correctly and fails, or silently truncates, beyond it. Emscripten's own script guards its accesses in a build with threads and growth, and warns that this is slower; code outside the module that reads `Module.HEAPU8` gets whatever view that thread's module made last **[D]**. Two further facts: several browser interfaces refuse a view over shared memory (`ImageData`, `TextDecoder.decode`, `Blob`, stream writes) **[D]** **[U]** (the copy in upstream's `softwareRenderTarget.ts` and the test in `stream.ts`), and each worker has its own `Module.HEAPU8`.

### Every place that holds or makes a view

| Place | What it does | With shared, growing memory |
|---|---|---|
| `webapp/modules/ferroui/ferroExports.ts`, `FerroRuntime.HEAPU8` | The property of the attached module, read at each use; not cached by the port **[M]** | The property itself may be stale in a thread that did not grow the memory. In a worker nothing is attached at all (section 3) |
| `webapp/modules/ferroui/rendering/softwareRenderTarget.ts`, `putPixelData` | `new Uint8Array(heap8.buffer, pointer, length)` per call, then `unpremultiply` into `this.straight`, an array over an ordinary `ArrayBuffer` kept between frames, then `ImageData` over `straight` **[M]** | A stale buffer makes the constructor throw a `RangeError` for a framebuffer allocated after the growth. `straight` is never shared, so `ImageData` accepts it: upstream's extra copy is not needed here |
| `webapp/modules/ferroui/stream.ts`, `write` | `heap().slice(pointer, pointer + count)`: a copy out of the module memory **[M]** | **A stale view makes `slice` return fewer bytes without an error**: a short write to the user's file. The copy itself is an ordinary array, which the stream accepts |
| `webapp/modules/ferroui/stream.ts`, `toMemoryView` | `heap().set(buffer, pointer)` **[M]** | A stale view throws a `RangeError` |
| `webapp/modules/ferroui/input.ts`, `addBytesToWriteableClipboardItem` | Receives a `Uint8Array` that the wasm-bindgen glue made over the module memory for a `&[u8]` argument, and copies it at once (`value.slice`) **[M]** | Depends on the glue (next row). The copy is ordinary and `Blob` accepts it |
| The wasm-bindgen glue merged into `<application>.js` (generated) | Keeps cached views of the memory for every string, slice and vector that crosses: `&str` and `String` everywhere, `&[i32]` (`create_render_target_surface`), `&[u8]` (clipboard), `Vec<u8>` and number arrays returned by the script (`input_helper.rs`) **[D]** | Whether the cache is refreshed by comparing the buffer (right for shared memory) or by testing for a detached buffer (which never happens with shared memory), and whether strings are decoded through a copy, is not known for the glue Emscripten merges **[H]**. To be read in the generated script of step 1 |
| Rust: `rendering/browser_software_render_target.rs` (`put_pixel_data(address, size, ...)`), `interop/stream_helper.rs` (`write_raw(pointer, len)`, `array_buffer_to_memory_view(buffer, pointer)`) | Pass an address and a length; hold no view **[M]** | Nothing to change |
| `webapp/tests/software-blit.test.mjs` | Attaches a fake runtime `{ HEAPU8 }` **[M]** | Follows the accessor below |
| `scripts/browser/tests/storage_view.test.mjs` | Reads `storageView.HEAPU8.buffer.byteLength` before and after a large write to assert that the memory grew **[M]** | Reads the page's view, which may be stale after a growth by another thread; it should read the memory object |
| `scripts/browser/first-frame.mjs` | Grows the exported memory of an instrumented module to read a profile **[M]** | A measurement tool for the build without threads; untouched |

No file of `interop/` holds a typed array over the module memory, and `ferroui-sw.ts` does not touch the memory **[M]**.

### What to do

Both, in this order:

1. **Re-acquire, in the port's own script, in both builds.** One accessor, `FerroExports.heapU8()`, replaces the three reads of `runtime.HEAPU8`: it takes the module's memory object (`wasmMemory`, added to the exported runtime methods) and returns a view over `memory.buffer`, made anew when the buffer object differs from the one the cached view is over. Reading `memory.buffer` is what returns the current buffer after a growth by any thread **[D]**. It is right and cheap in the build without threads too. `storage_view.test.mjs` asserts growth through the same memory object.
2. **Decide growth in the threaded build by reading the glue.** If the generated glue refreshes its views by buffer identity and decodes strings through a copy, growth stays on. If not, the threaded build is linked with a fixed memory (`-sALLOW_MEMORY_GROWTH=0` and an `-sINITIAL_MEMORY` large enough for the catalog, to be taken from a measurement of its peak, not guessed) until the glue is right, because a stale view inside the glue cannot be repaired from the port's script. A fixed memory also removes the slower guarded accesses of Emscripten's script. The build without threads keeps growth either way.

## 6. Steps

Each step is one pull request that builds and passes in both modes. "Threaded" means `scripts/build-browser.sh <application> --threads` and a page opened with `{ isolated: true }` of `scripts/browser/harness.mjs`. No step changes the build without threads except where it says so.

| Step | What it changes | How it is tested | Must be true before the next |
|---|---|---|---|
| B2.1 | **One frame from a worker, without the compositor.** A new example `src/Browser/FerroUI.Browser/examples/render_worker_clear` (page and `main.rs`), in the manner of `thread_spawn`. The page isolates itself (`ferroui-threads.js`), creates the module, attaches it and calls an export. The export starts a thread; the thread installs the worker handler of the registry (`WebRenderTargetRegistry.initializeWorker`, ported with the worker branch of `create`), publishes its pthread id and stays alive. The page then creates a canvas surface with that id: the registry transfers the canvas; the worker creates the script's WebGL target and reports it; the thread wraps it (`get_render_target`, `WebGlContext::new`, the `GlInterface` of `ferroui-opengl`), makes the context current, binds the framebuffer and clears it to one colour. State is reported as in `thread_spawn` (a line of `name=value` pairs). Build: `PThread` added to the exported runtime methods of the threaded build; the worker-side attach of section 3 | `scripts/browser/tests/render_worker_clear.test.mjs`: the state line says the clear ran on another thread; a capture of the page (`harness.mjs`, as `themed_view.test.mjs` reads pixels) shows the colour in the canvas; no error in the page; once with the headers of the server and once through the service worker. `thread_spawn.test.mjs` still passes | Written down in this page: whether wasm-bindgen imports work in a worker or the `--js-library` fallback was needed; how the worker got its `GL`; how the thread was kept alive; the shape of `PThread.pthreads` in Emscripten 6.0.10; whether `OFFSCREENCANVAS_SUPPORT` was needed; what the generated glue does with its memory views (section 5). The module of the example links the whole browser crate, so this is also the first threaded link and start of the port's own module with Skia, which B1 left open |
| B2.2 | **Memory views and one service worker** (script only). `FerroExports.heapU8()` and its three callers; `wasmMemory` exported; the decision on growth of section 5 applied to the threaded flags; the `fetch` handler of the isolation worker merged into `ferroui-sw.ts`, `ferroui-threads.js` registering it, `ferroui-coi-sw.js` removed | `webapp/tests/software-blit.test.mjs` (accessor with a replaced buffer); `storage_view.test.mjs` in both builds, including its growth check and, threaded without headers, a streamed save in an isolated page; `thread_spawn.test.mjs` and `render_worker_clear.test.mjs` against the merged worker | `storage_view` passes threaded. No reader of `runtime.HEAPU8` is left in `webapp/` |
| B2.3 | **Pacing, software frames, resize, and the wait, still without the compositor.** `render_worker_clear` grows options: `?RenderingMode=Software2D` (a `RetainedFramebuffer` filled and put through `putPixelData` in the worker); a frame loop on the worker (`BrowserRenderTimer::start_on_this_thread` there, a frame counter in an atomic, the colour changing per frame); the canvas size crossing through `BrowserSurfaceShared`; the dispatcher wake-up from the worker (the counter is reported to the main thread through it, not by polling); a frame out of turn on request; and an experiment switch that makes the main thread wait on a `Condvar` for the worker's next frame | The test asserts: frames keep counting while the page's script keeps the main thread busy for 200 ms (the point of the stage); after a resize of the window the capture has the new size and no stretched content; the software mode shows the colour; the wake-up arrives; and for the wait: its duration with a frame out of turn, and whether it ends at all without one (that is, whether a worker's animation frames run while the main thread spins) | The wait of section 3 is shown to be bounded, and its duration recorded here; or the "commit and do not wait" fallback is chosen instead. The surface objects of section 4 exist and are exercised outside the compositor |
| B2.4 | **The base library, strict mode** (no browser code). The four changes of "What the base library still has to change": the feature query through the cache or a job; the release of the server graph on the render thread when `use_ui_thread_for_synchronous_commits` is false; the request for a frame out of turn before a synchronous wait (a method of the render loop that a timer may implement); the render timer contract unchanged | `rendering/composition/render_thread_tests.rs` on the desktop: a compositor with the flag false, a render thread, and a render surface and graphics context that panic on any thread but the one that first used them. The test shows, resizes, queries a feature, takes a snapshot, disposes the target and drops the compositor, and the surface is never touched or dropped by the test thread. The existing suites pass in the three modes | The test passes; the lock-model tests of R5 are unchanged |
| B2.5 | **The browser backend on the new objects, still on one thread.** `BrowserSurfaceShared`, the per-thread table of targets, `BrowserPlatformGraphics` without cells, the render surface without `ThreadBound`; `BrowserRenderTimer::set_tick` no longer starts the loop when a render worker exists (upstream's condition **[U]**); `rendering/render_worker.rs` (start, pthread id, keep-alive, the two exports of section 4) compiled but not started | Unit tests of the browser crate on the host; `themed_view.test.mjs`, `storage_view.test.mjs`, `control_catalog.test.mjs` without threads, unchanged in number and result | The build without threads behaves as before, on the objects the threaded mode will use |
| B2.6 | **The compositor on the worker.** In a threaded module the platform starts the render worker, creates the shared timer with `runs_in_background` true and the compositor with `Compositor::with_render_thread(..., false, ...)`; the dispatcher's signal handle is the cross-thread one. A query parameter of the examples (`?RenderThread=false`) keeps the module on one thread for comparison. The host pages of `themed_view` and `storage_view` call `ensureCrossOriginIsolated` | `themed_view.test.mjs` run against the threaded site (site directory as its argument, isolated), all its checks, plus: an export reports that frames are rendered on another thread; `?RenderingMode=Software2D` and `WebGL1`; a resize; a hidden page renders no frame; the count of calls proxied to the main thread during a frame is zero or explained. The same file against the site without threads | `themed_view` renders from the worker in the three modes and passes the same checks as on one thread |
| B2.7 | **The catalog and the rest.** `control-catalog-browser` threaded; disposal of a view (`unregisterCanvas`); the pages that render offscreen (snapshots, drawing surfaces); popups; text input (the caret and the IME element are main-thread objects positioned from UI state: no change expected) | `control_catalog.test.mjs` and `storage_view.test.mjs` in both modes; `scripts/browser/capture.mjs` of a set of catalog pages compared between the modes (the desktop check of R5.4 found the two modes byte for byte equal; here equal within the tolerance the tests already use) | The "Done when" of B2: `themed_view` and the catalog render from the worker; the browser tests pass in both modes |
| B2.8 | **The site and CI.** A published site carries both modules; the host page imports the threaded one when the page is, or can be made, isolated and the browser can transfer a canvas and has animation frames in workers, and the other one otherwise (this is the fallback of `render-thread.md`, section 4: a module built with threads cannot start at all without shared memory). A CI job builds and tests the threaded examples | The test of each example in both modes in CI; on GitHub Pages by hand: `crossOriginIsolated` after the reload, frames from the worker | B3 can measure |

### Not in B2

**The UI on a worker** (upstream's shape). It would need: the application started on a pthread (`PROXY_TO_PTHREAD` or a thread started by the start export); every import of `interop/` proxied to the main thread with handles that are valid there; every export called by the page forwarded to the UI thread; `ManagedDispatcherImpl` (already ported for the desktop **[M]**, `src/FerroUI.Base/platform/managed_dispatcher_impl.rs`) as the dispatcher; `RawEventGrouper` and `ManualRawEventGrouperDispatchQueue` (not ported **[M]**) with a lock and a wake-up that upstream's do not have; and a decision on `preventDefault`, which could no longer depend on the answer of the UI. It is worth designing only if B3 shows that UI work on the main thread, not rasterisation, is what holds frames back.

## 7. Risks and open questions

Most serious first. Each names the measurement or experiment that answers it.

| # | Risk or question | Why it matters | Answered by |
|---|---|---|---|
| 1 | **wasm-bindgen imports in a pthread worker.** That the glue Emscripten merges sets up its imports and its object table in each worker, and that `ferroui.js` loads there, is unverified (B1's first doubt). | Everything the worker does in script (the target, the size, the pixels, the frame loop) goes through it | B2.1. Fallback: the worker's five functions in a `--js-library` |
| 2 | **Waiting on the main thread.** The synchronous points and the compositor lock make the main thread spin. It is unknown how long, and whether a worker's animation frames run at all while the main thread does not return to the browser. | A wait that never ends freezes the page for good; a long one is worse than today's single thread | B2.3: the duration of the wait with and without a frame out of turn. Fallback: commit and do not wait |
| 3 | **The UI thread reaching the render target.** Paths in the base library that create, use or drop graphics objects on the thread of the compositor: the feature query, the release in `Compositor::drop`, anything the audit of section 3 missed. On the desktop the lock made these sound; in a worker they are GL calls without a context. | Silent corruption or a lost context rather than a panic, if the drop is not caught | B2.4: the test surface that panics off its thread; in the browser `WebGlContext::verify_access` panics and the test pages fail on any page error |
| 4 | **Stale memory views.** Section 5: the port's own readers are easy; the generated glue is unknown, and a stale view in `StreamHelper.write` truncates a file without an error. | Data loss in storage; wrong strings anywhere | B2.1 (read the glue), B2.2 (`storage_view` growth check threaded). Fallback: fixed memory in the threaded build |
| 5 | **Keeping the render thread alive on events.** A Rust thread that must return to the worker's event loop without ending; `emscripten_exit_with_live_runtime` leaves by unwinding through the frames above it, which were compiled with WebAssembly exception handling. | Without it there is no worker that can receive a canvas or an animation frame | B2.1. Fallback: the thread is created by a few lines of C (`pthread_create` with a start function that calls into Rust and then keeps the runtime alive), compiled by the build script that already compiles the setjmp bridge |
| 6 | **Emscripten internals.** `PThread.pthreads` and the worker object in it are not a documented interface (upstream copes with two shapes **[U]**); replacing `self.onmessage` in a pthread relies on Emscripten installing its handler that way. | A change of Emscripten breaks the transfer | B2.1 records what 6.0.10 has. Fallback: `OFFSCREENCANVAS_SUPPORT` and a canvas transferred when the thread is created, for the first view |
| 7 | **Skia and GL in a worker.** That Skia's native GL interface and Emscripten's GL emulation work from a worker's context registered by `GL.registerContext`, and that nothing in the Skia backend of the port assumes the thread that initialised it (thread-local caches are per thread by design, R5.2). | The WebGL modes | B2.1 for raw GL; B2.6 for Skia (`themed_view` WebGL2 and WebGL1) |
| 8 | **Browsers.** Animation frames in workers, `transferControlToOffscreen` and WebGL on an `OffscreenCanvas` are not in every browser the build without threads supports; the tests run in headless Chrome only (`browser-platform.md`, section 16, B9b). | The threaded module must not be chosen where it cannot render | B2.8: the host page tests for the three features before it picks the module; a manual run in Firefox and Safari |
| 9 | **Round trips hidden in a frame.** A call in the frame path that Emscripten proxies to the main thread synchronously (a clock, a log line, a file read of a font). | A frame that waits for the main thread defeats the stage and can meet the wait of risk 2 | B2.6: count proxied calls per frame |
| 10 | **Start-up and size.** Every worker of the pool instantiates the module before the application starts; the threaded module is larger (a rebuilt standard library, atomics). | First frame, the measure the browser work has optimised | B3: `scripts/browser/first-frame.mjs` and `module-sizes.mjs` on both sites |
| 11 | **Latency.** A commit of the UI thread is drawn at the worker's next animation frame, not in the same task. | One frame more between input and picture in the worst case | B3: `scripts/browser/frame-times.mjs` and `scroll-profile.mjs` in both modes |
| 12 | **Upstream's `RunsInBackground` false.** Section 1: read literally, upstream's synchronous commits render on the UI thread in the threaded mode. Either the reading misses something the runtime does, or the mode does not work at the commit read. | The port deviates here knowingly; if upstream turns out to rely on it for a reason, the reason applies to the port too | Outside this repository: build upstream's `ControlCatalog.Browser` with `WasmEnableThreads` true and resize the window. Not a blocker |
| 13 | **The toolchain items B1 left open**: the nightly pin, `-Zbuild-std` with the `browser` profile, the order of the two `-sENVIRONMENT` settings. | They stop B2.1 before it starts | The first threaded build of an example that links the framework, which is B2.1 |

## B2.4: the base library, strict mode

Status: written on 2026-10-08 without a build or a test run; the branch is validated by another session. Nothing in the browser crate changed. Markers as above; everything here is **[M]** unless it says otherwise.

### What "confined" is

A compositor is **confined to its render thread** when it was created in the render-thread mode (`Compositor::with_render_thread`, or `Compositor::new` over a loop that runs in the background) with `use_ui_thread_for_synchronous_commits` false **and** its loop reports `runs_in_background()` true. This is upstream's condition for waiting instead of rendering **[U]** (`MediaContext.SyncWaitCompositorBatch`, `Compositor.GetRenderInterfacePublicFeatures`). It is decided once, when the compositor is created (`Compositor::is_confined_to_render_thread`), because the release of the server graph depends on it and has to agree with everything before it.

Not confined, and unchanged: the dispatcher-thread mode; the lock model (flag true); and a render-thread compositor with the flag false over a loop that does **not** run in the background, which is what `composition_drawing_surface_tests.rs` and the first test of `render_thread_tests.rs` use (they tick the loop from a new thread per frame and build the interop on the test thread).

One desktop case becomes confined that was not written down before: `OffscreenTopLevelImpl` and the headless platform create their compositor with the flag false. With the headless timer that is the dispatcher-thread mode as before. An offscreen top level created in an application of the native platform shares the background loop, so it is confined: it already waited at the synchronous points (R4.1); from now on it also takes the three paths below.

### The four changes

| # | Change | Where |
|---|---|---|
| 1 | **The feature query.** `Compositor::try_get_render_interface_feature` of a confined compositor enters the lock, reads `at_try_get_cached_render_interface_features` and answers from it. Without a cache it answers `None` and posts one job (`rt_get_render_interface_features`, upstream's `InvokeServerJobAsync(Server.RT_GetRenderInterfaceFeatures)`); a flag keeps a second job from being posted while the first is on its way. A cache without the feature answers `None` and posts nothing. The calling thread never reaches `render_interface().value()`. | `compositor.rs` |
| 2 | **The release of the server graph.** `Compositor::drop` of a confined compositor (a) asks the loop task for the release (an atomic flag, set before the lock is entered), (b) enters the lock and drops the handles the graph shares with its own thread (`PlatformRenderInterfaceContextManager::release_platform_handles`), (c) wakes the loop and leaves the task in it. The next tick of the render thread reads the flag inside the lock, renders nothing, calls `LockedServerCompositor::release` (the graph, every render target and the graphics context are dropped there) and posts the removal of the task to the dispatcher of the compositor, because `IRenderLoop::remove` belongs to that thread. | `compositor.rs`, `server/compositor_lock.rs`, `platform_render_interface_context_manager.rs` |
| 3 | **A frame out of turn.** `IRenderLoop::request_frame_out_of_turn` and `IRenderTimer::request_tick_out_of_turn`, both with a default that does nothing. `DefaultRenderLoop` forwards the first to its timer. `MediaContext::sync_wait_compositor_batch` calls the loop's before `BatchCompletion::wait` (the commit has already woken the loop). A timer that implements it ticks once on its own thread and never on the caller's. | `i_render_loop.rs`, `i_render_timer.rs`, `render_loop.rs`, `media/media_context.rs` |
| 4 | **The render timer contract** is otherwise unchanged; no implementer had to change. | |

What had to change besides the four, found while reading for them:

- **The graph is bound to one thread, not only to the lock.** `LockedServerCompositor::confine_to_current_thread` is called by every frame of a confined compositor; the first decides, and a frame from another thread panics. Two `Drop` impls read it: the lock object leaks an unreleased graph when its last handle is dropped by another thread, and a `LockBound` leaks its value there. This is the safety net; the design is change 2.
- **The handles the graph shares with the UI thread.** R5.5 released the graph on the UI thread because the graph holds `Rc` handles whose counts the UI thread changes (the platform graphics, the render interface). Releasing the graph on the render thread would turn that around, so `Compositor::drop` takes those handles out first, on its own thread. For that the context manager holds the platform graphics and the ready state feature in cells. The clock was a third shared `Rc` (one handle cloned into the server compositor): each side now makes its own handle over the shared function. Since R5.8 of `render-thread.md` the platform graphics and the ready state feature are shared by contract (`Arc`, `Send + Sync`), so only the handle of the render interface still has to be taken back; `release_platform_handles` takes the three as before.
- **`render_on_this_thread` panics** for a confined compositor. Its one caller, the media context, does not reach it there.
- **`CompositionInterop`** took `render_interface().value()`, which creates the context, on the calling thread. `try_new` takes the context that exists for a confined compositor and answers `None` without one; `try_get_composition_gpu_interop` uses it.

### The audit: every path on which the thread of the compositor enters the server side

| Path | Confined compositor | Verdict |
|---|---|---|
| `CompositionTarget::new` (`with_server`) | Looks the render interface up in the locator of the calling thread and stores the handle. No graphics object. | No change |
| `CompositionTarget::set_debug_overlays` (`with_server`) | Creates the text renderer of the overlays: glyph runs made through the render interface, which are shared resources (R1.2). It is dropped with the graph, on the render thread; it holds nothing of the UI thread. | No change |
| The first show, a resize, `paint` (`MediaContext::immediate_render_requested`) | Commit, request for a frame out of turn, wait for `Rendered`. | Change 3 |
| The disposal of a target (`sync_dispose_composition_target`) | The out-of-band batch, the request, a wait for `Processed`. The server target disposes its layer and render target while the batch is applied, and leaves the table, and is dropped, after the batch: all on the render thread. The surfaces closure is moved into the server target by the batch, so the thread of the compositor holds none of it afterwards. | Change 3 |
| `Compositor::try_get_render_interface_feature`, `IRenderer::try_get_render_interface_feature` | Change 1. | Changed |
| `Compositor::try_get_composition_gpu_interop`, `CompositionInterop::new` | The feature from the cache; the context that exists. | Changed |
| `CompositionInterop`: the supported handle types, the capabilities, `wrap_image_handle_on_any_thread`, `is_lost`, the device ids | Read on the calling thread, inside the lock. Upstream reads the same members on its UI thread, with a render thread and the same flag **[U]** (`CompositionInterop.cs`). | No change; doubt 4 |
| The import, use and disposal of an imported image or semaphore; the update of a drawing surface | Jobs (R5.6). The server parts are created inside the lock by the calling thread (handles are cloned, nothing is called) and dropped by the render thread, or leaked. | `LockBound::drop` |
| `Compositor::create_composition_visual_snapshot` | A post-target job; the bitmap that comes back is a shared snapshot (R1.3). | No change |
| The readback (`hit testing`, `try_get_valid_readback`) | Atomics and a lock of its own. | No change |
| `Compositor::render_on_this_thread` | Panics. | Changed |
| `Compositor::with_server` (public) | Still hands the server compositor to the caller under the lock. A caller could render with it; nothing in the tree does. | Open: doubt 6 |
| `Compositor::drop` | Change 2. | Changed |
| A batch that is never applied (committed after the last frame) | Dropped with the queue by whoever drops it last. Its content is `Send` by construction (R2). A creation it carries may hold the surfaces closure of a target that was never shown; the surface is then dropped by that thread, having never been used. | No change |

### When the loop has stopped ticking

| Change | The loop does not tick again |
|---|---|
| 1, the feature query | The job stays in a batch that is never applied; the flag stays set, so no further job is posted. Every call answers `None` (or from a cache that an earlier frame filled). Nothing blocks. |
| 2, the release | The task stays in the loop with the graph. The compositor's own handles are gone, and so are the three shared handles, so the graph only holds what belongs to the render thread. When the loop is dropped, the task drops the last handle to the lock object: if a thread ever rendered and it is not the one dropping, the graph is **leaked** (`Drop for LockedServerCompositor`), with its render targets and graphics context; if no thread ever rendered, the graph holds no graphics object and is dropped in place. The alternative, releasing when the task is removed, was not taken: `remove` runs on the thread of the compositor, which is exactly the thread that may not drop the graph. For the browser this is the end of the page or of the worker, where a leak costs nothing; a view that is closed while the worker lives is released by the worker's next tick. |
| 2, the removal of the task | If the dispatcher of the compositor no longer runs, the task stays in the loop, empty, and answers "no next tick". |
| 3, the frame out of turn | The request is lost and `BatchCompletion::wait` does not return. This is unchanged from R4.1 and is risk 2 of section 7: B2.3 has to show that the worker's timer can honour the request, or the "commit and do not wait" fallback of section 3 replaces the wait. The base library does not time the wait out. |

### The test

`render_thread_tests.rs`, last part. The doubles (a surface, the platform graphics, a graphics context, a backend context that hands out one public feature, a render target over the mock one) share a log: the first thread that uses one of them owns them all, and a use or a drop by any other thread is recorded, not panicked on, because a panic on the render thread would end the thread and leave the test thread in its wait. `MockPlatformRenderInterface::set_backend_context_factory` lets a test supply the backend context.

- `a_confined_compositor_keeps_what_renders_on_the_render_thread`: a feature query before any frame (absent, and nothing was touched); a show; a resize (two requests for a frame out of turn so far); the feature query again (present), and one for a feature the context does not have; a snapshot; the disposal of the target (the render target is disposed and dropped); the drop of the compositor, after which the task leaves the loop and the backend context and the graphics context have been dropped. No violation, and the owner is not the test thread.
- `a_confined_compositor_whose_loop_has_stopped_is_leaked_and_not_dropped_by_another_thread`: the render thread ends first; dropping the compositor and then the loop drops nothing of the render thread.
- `render_loop.rs`: the default loop passes the request to its timer and does not render.

The four earlier tests of the file assert what they asserted. One of them (`a_synchronous_commit_waits_for_the_render_thread`) creates a confined compositor and joins its render thread before the compositor is dropped: its graph is now leaked at the end of the test, where it used to be dropped by the test thread.

### What is doubted

1. **None of this was compiled.** The likeliest faults are in the test doubles (the exact members of the platform contracts) and in the closure the mock render interface takes.
2. **Whether the server compositor is freed by the release.** The first new test asserts that the backend context and the graphics context are dropped once the task has left the loop. That holds if nothing keeps the server compositor alive after `release` (the server objects hold it weakly, as read). If a cycle does, the assertion fails, and the fault is older than this step: the lock model would then leak the same graph on the UI thread.
3. **The graph is dropped, not disposed.** The last job drops the server compositor, as the release of the lock model does; it does not dispose the targets or make the graphics context current first. A backend whose objects need their context current when they are dropped (several WebGL contexts in one worker, one per canvas) needs more than this: either its objects make their context current in their own drop, or the last job disposes the active targets under `ensure_current` before it drops the graph. To be decided with the browser's objects in B2.5 and B2.6.
4. **What the interop reads on the calling thread.** `is_lost` of the context and the lists of the external objects feature are called on the thread of the compositor, inside the lock, as upstream calls them on its UI thread. For a backend whose context refuses every call off its thread (the port's `WebGlContext::verify_access` **[M]**) these would have to become jobs or cached values. The browser has no external objects feature, so nothing reaches them there.
5. **A feature leaves the lock as an `Rc`.** Open item 4 of R5.5 is unchanged: `try_get_render_interface_feature` clones the feature inside the lock and the caller drops it outside, while the render thread drops the cache on its side when the context goes. A feature that a confined platform publishes has to be safe for that (what the browser's Skia context publishes is to be read in B2.5; on Metal the map is empty, R5.5). **Closed (R5.8 of `render-thread.md`)**: the query hands out a `RenderInterfaceFeature`, a handle bound to the compositor lock that lends the feature inside it. The confined compositor still answers from the cache and posts the job; the handle of the cached feature follows the rule of every `LockBound` of a confined compositor, so the thread of the compositor leaks it instead of dropping it (doubt 10).
6. **`with_server` is public.** The type cannot keep a caller from rendering, or from creating the backend context, inside it. The assertion is in `render_on_this_thread` only.
7. **"Absent" for "not known yet".** A caller of the feature query cannot tell the two apart and is not told when to ask again. `OpenGlCompositionInterop` (`src/FerroUI.OpenGL`) asks when it is set up; with a confined compositor it would have to ask again after the first frame. A completion on the dispatcher would settle it, and needs the feature map to cross threads, which is item 5.
8. **One render thread for the life of the compositor.** A timer that ticks from more than one thread (a pool) makes the second frame panic. `ThreadProxyRenderTimer`, which the native platform uses, ticks from its one thread **[M]**; the other background timers of the tree were not read for this; the browser's will tick from the worker.
9. **Where the loop runs is read once.** A timer that only starts to report `runs_in_background()` true after the compositor was created (the browser's, if the compositor is created before the render worker has started) gives a compositor that is not confined: its synchronous commits still wait (the media context asks the loop each time), but the feature query and the release take the paths of the lock model, on the UI thread. B2.6 has to create the timer in its final state before the compositor, or pass the mode explicitly.
10. **The leak of a `LockBound`.** The interop object of a confined compositor leaks the counts of the context and of the feature when its owner drops it, and an imported image whose owner outlives its disposal job leaks its shell. Since R5.8 the same holds for the handle of a feature (`RenderInterfaceFeature`): each one the thread of a confined compositor drops leaks a count of the feature, which is then not freed with its context. Correct would be a queue of values for the render thread to drop at its next entry into the lock; it needs an `unsafe impl Send` for values that are not `Send`, and was left out while no confined platform has the feature.

## B2.1: one frame from a worker (written and validated, 2026-10-08)

Status: written, **not built and not run**. The branch was written without cargo, without the browser build and without a browser; the session that validates it builds it first. What could be checked without a build was: the script modules pass the type check and the linter of `webapp/` (run with the tools already installed in the main checkout; the only errors are the missing package of the storage bundle, which is not installed in the worktree), the new scripts parse, the shell script parses, and the Rust files parse (the formatter reads them). Nothing below marked **[R]** has been seen to run: it was read in the sources of the tools that are installed for the build, Emscripten 6.0.10 (`.tools/emsdk/upstream/emscripten/src`) and the wasm-bindgen tool 0.2.129 (`wasm-bindgen-cli-support` in the cargo registry), and in the script of a module built without threads (`target/browser/themed_view/themed_view.js`).

### What was written

| Piece | Where | What it is |
|---|---|---|
| The worker branch of the registry | `webapp/modules/ferroui/rendering/webRenderTargetRegistry.ts`, `canvasSurface.ts` | `WebRenderTargetRegistry.create(pthreadId, canvas, modes)` and `initializeWorker()` ported from upstream's file as they are, and the `threadId` argument of `CanvasSurface`. Two differences: the module is taken from `FerroExports.runtime` (upstream: `self.Module ?? getDotnetRuntime(0).Module`), and the message key is `ferrouiCmd` |
| The report of the worker | The same handler; `ferroExports.ts` (`CanvasHelper.OnRenderTargetRegistered`); `interop/canvas_helper.rs` | After the worker created the target it calls the export `CanvasHelper_OnRenderTargetRegistered(id, kind)` of its own module (section 4, step 3). `kind` is an integer (2 WebGL, 1 software), not the name, so that the report passes no string into the module. In Rust the export reaches the subscribers of the calling thread (`add_render_target_registered`, a `thread_local!` list like the one of the animation frames), with a unit test. Recorded in `DEVIATIONS.md` |
| The Rust side of the transfer | `interop/canvas_helper.rs`, `rendering/web_render_target.rs`, `rendering/mod.rs`, `rendering/render_target_browser_surface.rs` | `create_render_target_surface` takes `thread_id` (the one caller of the crate passes 0); `initialize_worker()` is the import of `WebRenderTargetRegistry.initializeWorker`; `get_render_target` and `initialize_worker` are exported by `rendering` |
| The attach in a worker | `scripts/browser/threads/ferroui-worker-import.js` (`--extern-pre-js`), `scripts/browser/threads/ferroui-worker-attach.js` (`--post-js`) | Below |
| The build | `scripts/build-browser.sh`, the `--threads` flags | `-sEXPORTED_RUNTIME_METHODS=GL,HEAPU8,PThread` and the two scripts above, appended to the flags of the target. They apply to every threaded build, `thread_spawn` and `themed_view` included |
| The example | `src/Browser/FerroUI.Browser/examples/render_worker_clear` (`main.rs`, `wwwroot/index.html`, `wwwroot/main.js`), registered in the manifest of the crate | Exports `renderWorkerClearStart(width, height)`, `renderWorkerClearCreateSurface(container)`, `renderWorkerClearState()`. The thread subscribes to the report, calls `initialize_worker`, keeps itself alive, publishes `pthread_self()` in an atomic and returns. On the report it calls `get_render_target`, takes the `IGlContext` of the target and its `IGlPlatformSurface`, and draws one frame as a backend does: `create_gl_render_target`, `begin_draw` (which sets the size of the canvas, makes the context current and binds the framebuffer), `clear_color`, `clear`, `get_error`, `flush`, and the end of the session. The colour is 32, 96, 192. The state line ends as `state=done thread=<id> other_thread=true target=1 kind=webgl gl=3 gl_error=0 size=200x120 color=32,96,192` |
| The test | `scripts/browser/tests/render_worker_clear.test.mjs` | Two checks, with the headers of the server and through the service worker: the state line; the canvas cannot give the page a context (its control is gone); a capture has the colour at the centre and at the four corners of the canvas and the page colour beside it; no error; the same service worker and navigation counts as `thread_spawn.test.mjs` |

### The attach in a worker: what was done instead of a `--pre-js` with an import

Section 3 proposed a `--pre-js` that imports `./ferroui.js` in a pthread and attaches the module. Read in the tools, that shape has a hole, and a second thing turned out to be missing:

- **[R]** The script of the module is evaluated in the worker at once (`modularize.js`: `isPthread && Module()`), and its message handler takes the `load` command, instantiates the module and answers in one synchronous run (`runtime_pthread.js`); run dependencies are not looked at in a pthread (`postamble.js`, `run`). An `import()` in a pre-js is asynchronous, so nothing would make the thread's start wait for it short of holding the worker's messages back by hand.
- **[R]** The wasm-bindgen glue already puts a **static** import of `./ferroui.js` at the top of the script, outside the factory function (the tool writes it to a file that Emscripten adds with `--extern-pre-js`; `link.py`, `run_wasm_bindgen`). The port adds one more static import the same way (`ferroui-worker-import.js`: `FerroExports` under the name `__ferroui_FerroExports`). It is resolved before the script runs, so the code inside the factory can call `__ferroui_FerroExports.attach(Module)` synchronously. That code is a `--post-js` rather than a `--pre-js` because of the next point: it needs `PThread`, which exists only after the library code.
- **[R] The glue is not started in a pthread.** The start function of the glue (`__wbindgen_start`, called by `initBindgen`) is registered with `addOnInit`, and `initRuntime` returns before the initialisers in a pthread (`preamble.js`). That function is what fills the fixed entries of the table of script objects (`__wbindgen_init_externref_table`: undefined, null, true, false, at the indices 128 to 131 that the crate treats as constants), and every worker has a WebAssembly instance, and so a table, of its own. Without it the allocator of the crate, which is thread-local with atomics (`externref.rs`), would hand the indices of the constants to the first four objects of the worker. `ferroui-worker-attach.js` therefore wraps `PThread.threadInitTLS`, which the worker calls for each thread before its start function (`runtime_pthread.js`, the `run` command), and calls `___wbindgen_start()` once per worker after it. A module without script objects has no such function and the call is skipped.

So the worker-side attach is two files, both synchronous. If either of the two names they rely on (`PThread.threadInitTLS`, `___wbindgen_start`) is not what the build produces, the link still succeeds and the failure shows in the page: an error in the worker (the first), or wrong objects in the worker (the second).

### Left out, on purpose

- The registry does not hold `registerCanvas` back until the worker has reported its id (section 4, the last paragraph): the page of the example creates the canvas only after the thread published its id, which is after it installed the handler. The hold belongs to B2.5, with `RenderWorker_OnStarted`.
- `unregisterCanvas` is handled by the worker, as upstream, and still sent by nobody (B2.7).
- No `BrowserSurfaceShared`: the size of the canvas is given by the page when it starts the thread and read from two atomics; the `ResizeObserver` reports of the canvas go to `CanvasHelper_OnSizeChanged` with top level 0 and are dropped (B2.3).
- `pthread_self` and the keep-alive are declared in the example, not in the crate: `rendering/render_worker.rs` is B2.5.
- The host page calls the Rust export to create the surface; nothing of the platform start-up (`BrowserAppBuilder`, the dispatcher, the timer) runs in the example.
- `browser-platform.md`, section 21, does not describe the two new scripts yet, and no CI job builds the example.

### The questions of the row, as settled at validation

| Question | What the sources say | Status |
|---|---|---|
| Do wasm-bindgen imports work in a worker, or was the `--js-library` fallback needed? | **[R]** The static import of `./ferroui.js` is part of the script every worker loads, and each import function of the glue calls the binding of that worker's copy. The glue has to be started per worker (above); with that, nothing was found that ties an import to the main thread. Whether `ferroui.js` loads in a worker at all (section 3, the first part of the doubt) is now also asked of `thread_spawn`: its script gains the import | Settled: it works as read (validation of 2026-10-08, below) |
| How did the worker get its `GL`? | **[R]** Each worker's script has its own `GL` object; it is exported on the worker's `Module` like on the page's, and `webGlRenderTarget.ts` reads it through the attached module. `GL.registerContext` in a build with threads allocates the handle with `_malloc` and stores `_pthread_self()` in it (`libwebgl.js`), which works in a worker that hosts a thread. GL calls are proxied to the main thread only with `OFFSCREEN_FRAMEBUFFER` (`system/lib/gl/webgl1.c`, `webgl_internal.h`), which the build does not set | Settled: it works as read (validation of 2026-10-08, below) |
| How was the thread kept alive? | `emscripten_runtime_keepalive_push()` before the closure of `std::thread` returns. **[R]** `invokeEntryPoint` does not call `__emscripten_thread_exit` when the counter is above zero (`libpthread.js`), and the counter counts in a build with threads (`libcore.js`, `$keepRuntimeAlive`). No unwinding through Rust frames, unlike `emscripten_exit_with_live_runtime` (risk 5). Unknown: what the standard library does after the closure returns and the thread does not exit (the result packet is dropped; thread-local destructors should not run before the exit), and whether `std::thread::current()` still answers on later calls into the worker (`WebGlContext::verify_access` compares its id) | Settled: it works as read (validation of 2026-10-08, below) |
| The shape of `PThread.pthreads` in Emscripten 6.0.10 | **[R]** A map from the pthread pointer to the `Worker` itself (`PThread.pthreads[threadParams.pthread_ptr] = worker`, `libpthread.js`): the first of the two shapes the registry copes with. The example passes the id as a signed 32-bit integer, which equals the key while pointers are below 2 GB | Settled: it works as read (validation of 2026-10-08, below) |
| Was `OFFSCREENCANVAS_SUPPORT` needed? | Not set. **[R]** The canvas is transferred by the port's script and the context is registered with `GL.registerContext` in the worker; nothing on that path asks for the setting | Settled: it works as read (validation of 2026-10-08, below) |
| What does the generated glue do with its memory views (section 5)? | **[R]** In the Emscripten mode the glue keeps no views of its own: it reads `HEAPU8` and a `HEAP_DATA_VIEW` that it refreshes by wrapping `updateMemoryViews`; with a shared memory it decodes strings through `slice`, a copy (`js/mod.rs` of the tool). Whether Emscripten keeps those two current in a thread that did not grow the memory is not read yet; the script of the threaded example is where to read it | Settled: it works as read (validation of 2026-10-08, below) |
| The first threaded link and start of the port's own module with Skia on a thread | The example links the whole browser crate and creates the `GlInterface` on the thread; Skia itself is not called (B2.6) | Settled: it works as read (validation of 2026-10-08, below) |

### Validation (2026-10-08)

Built with `scripts/build-browser.sh render_worker_clear --threads` and run with `node scripts/browser/tests/render_worker_clear.test.mjs` in headless Chrome: both checks pass on the first build, with no change to what was written.

- The state line is `state=done thread=<id> other_thread=true target=1 kind=webgl gl=<2 or 3> gl_error=0 size=200x120 color=32,96,192`: the canvas was transferred, the worker created the WebGL target, and the thread of the module made the context current and cleared it.
- The capture of the page shows the colour inside the canvas and the page beside it; the canvas on the page refuses a 2D context (it was transferred).
- Once with the headers of the server (no service worker registered, one navigation) and once through the isolation service worker (two navigations); no error in the page either way.
- So the wasm-bindgen glue does work in a pthread worker with the static import and the start of the glue per worker, and the `--js-library` fallback is not needed. A string passed through the glue on the worker (`renderTargetType`) works. `OFFSCREENCANVAS_SUPPORT` is not needed. The thread stays alive with the keep-alive counter and answers the registry's message afterwards.
- The flags of the threaded build changed for every example, so the others were rebuilt: `thread_spawn.test.mjs` passes its 3 checks, `themed_view` built with threads passes its 30 checks served isolated, and `themed_view` without threads passes its 30 checks.
- Not exercised by this step, still open for B2.6: Skia called on the thread.

### Doubts, most likely to bite first

1. **The link flags.** `--extern-pre-js=<file>` and `--post-js=<file>` passed as `-Clink-arg` through `--config`, with absolute paths inside a TOML string; and the second `-sEXPORTED_RUNTIME_METHODS` replacing the first rather than being refused. **[R]** `cmdline.py` takes both `--x=<file>` forms; the replacement is the same mechanism as the two `-sENVIRONMENT` settings of B1.
2. **`thread_spawn` and `themed_view` threaded** now carry the import and the post-js. If `ferroui.js` does not load in a worker, `thread_spawn.test.mjs` fails where it passed before.
3. **Names inside the generated script**: `___wbindgen_start` (seen under this name in the script built without threads), `PThread.threadInitTLS`, `ENVIRONMENT_IS_PTHREAD`, and that the optimiser of Emscripten leaves the free name `__ferroui_FerroExports` alone, as it leaves the `__wbg_` names of the glue.
4. **Calling `__wbindgen_start` in a worker.** It is what the table needs; whether the start function of this module contains anything else that must not run a second time over the shared memory was not read (the tool leaves the threading set-up to Emscripten in this mode: "Emscripten owns multithreading bootstrap logic").
5. **Rust that was not compiled.** The example and the changes of the crate were checked name by name against the sources but not by the compiler. The places most likely to need a touch: the method calls on trait objects without the traits imported (`try_get_feature` through `IPlatformGraphicsContext`, `begin_draw`, `dispose`), the `thread_local!` with `const` initialisers over `CanvasSurface`, and warnings for unused items in the build for the host.
6. **A string on the worker.** `get_render_target` reads `renderTargetType` and so passes a string from the worker's script into the module, through the glue's encoder and the shared memory. It works on the page's thread in a threaded module (`themed_view`, B1); the worker has the same code and a different instance.
7. **WebGL on a transferred canvas in headless Chrome with SwiftShader**, with `failIfMajorPerformanceCaveat`. If the context is refused the worker falls back to the software target and the example reports `state=failed error=the worker created a software render target, not a WebGL one`.
8. **The capture.** That `Page.captureScreenshot` shows the frame of a canvas presented from a worker, and how soon; the test asks again for up to 15 seconds.
9. **Presenting without a frame callback.** The clear runs inside a message handler of the worker and the context has `preserveDrawingBuffer: false`: the browser should present when the handler returns. If the capture stays empty although the state line is right, the frame has to be drawn from `requestAnimationFrame` of the worker, which is B2.3.
10. **The size of the canvas** is set by `begin_draw` on the `OffscreenCanvas` (`WebRenderTarget.setSize` in the worker) from the device pixels the page computed; at a device scale factor other than 1 the test's 200 x 120 does not hold (the test runs at 1).

## B2.2: memory views and one service worker (written, 2026-10-08)

Status: written, **the modules not built**. No cargo and no browser build was run. What was run, with the tools installed in the main checkout: the type check, the linter and the bundle of `webapp/` (clean); `webapp/tests/software-blit.test.mjs` in headless Chrome (passes); and `thread_spawn.test.mjs` against a copy of the `thread_spawn` site of the B2.1 validation in which `ferroui-sw.js` and `ferroui-threads.js` were replaced by the new ones and `ferroui-coi-sw.js` removed (its 3 checks pass: the merged worker isolates the page). The module of that copy was linked with the old flags, so nothing below about the link, about `wasmMemory` on a real module, or about `storage_view` has been seen to run. **[R]** marks what was read in Emscripten 6.0.10 (`.tools/emsdk/upstream/emscripten`) and **[G]** what was read in the script of the threaded `themed_view` of the B2.1 validation (`target/browser-threads/themed_view/themed_view.js`).

### What was written

| Piece | Where | What it is |
|---|---|---|
| The accessor | `webapp/modules/ferroui/ferroExports.ts` | `FerroExports.heapU8()`: reads `buffer` of the memory object of the attached module (`runtime.wasmMemory`) and returns a `Uint8Array` over it, kept until the buffer is another object. It throws when no module is attached or the memory is not exported. `FerroRuntime` no longer declares `HEAPU8` |
| Its three callers | `rendering/softwareRenderTarget.ts` (`putPixelData`), `stream.ts` (`write`, `toMemoryView`) | They ask for the view at each call. `stream.ts` loses its own `heap()`. No reader of `runtime.HEAPU8` is left under `webapp/` |
| The export | `.cargo/config.toml`, `scripts/build-browser.sh` | `wasmMemory` added to `-sEXPORTED_RUNTIME_METHODS` in both builds (`GL,HEAPU8,wasmMemory`, and with `PThread` in the threaded one). `HEAPU8` stays exported: nothing of the port reads it any more, and nothing was removed that a host page might read |
| The memory in a worker | `scripts/browser/threads/ferroui-worker-attach.js` | In a thread `Module.wasmMemory` becomes a property that reads the variable of the script (below) |
| Growth | `scripts/build-browser.sh`, the `--threads` flags | `-sALLOW_MEMORY_GROWTH=0` and `-sINITIAL_MEMORY=<n>MB`, `n` from `FERROUI_BROWSER_THREAD_MEMORY_MB` (default 512, checked to be a number between 16 and 2048). The build without threads is unchanged |
| One service worker | `webapp/modules/ferroui-sw.ts` | The handler of the isolation worker follows the handler of the polyfill in one `fetch` listener, behind `new URL(self.location.href).searchParams.get("coi") === "1"`. Kept from the old file: the request of the browser's own tools that is left alone, the opaque response passed on as it is, and the null body for the statuses 101, 204, 205 and 304. One addition: with the parameter the response of a download gets the three headers too (below) |
| Who registers it | `scripts/browser/threads/ferroui-threads.js`, `interop/ferro_module.rs` | The default of `ensureCrossOriginIsolated` is `./ferroui-sw.js?coi=1`. `resolve_service_worker_path` answers with the same address when the module is built with threads (`cfg!(target_feature = "atomics")`, what `thread_spawn` already reports) and with `./ferroui-sw.js` otherwise |
| The file removed | `scripts/browser/threads/ferroui-coi-sw.js` | Deleted; the build copies one file of that directory to a threaded site. `thread_spawn.test.mjs` and `render_worker_clear.test.mjs` assert `ferroui-sw.js?coi=1` as the script of the controller, and nothing else in them changed |
| A page that serves both builds | `scripts/browser/threads/ferroui-worker-import.js`, `examples/storage_view/wwwroot/main.js` | The script of a threaded module exports `ferrouiThreads = true` (one more top-level statement of the `--extern-pre-js`). The host page of `storage_view` reads it from the namespace of the module script and only then imports `ferroui-threads.js` and calls the check. A site without threads makes no extra request. This is the part of B2.6 that the test of this step needs; `themed_view` is left to B2.6 |
| Tests | `webapp/tests/software-blit.test.mjs`, `scripts/browser/tests/storage_view.test.mjs` | Below |
| Records | `DEVIATIONS.md` (two rows), `NOTICE.md` of the crate, `browser-platform.md` section 21 | The second part of the worker and its address; the accessor |

### `wasmMemory` on the module

- **[R]** `wasmMemory` is a symbol of the script library (`$wasmMemory`, `lib/libcore.js`), so it is a legal name in `EXPORTED_RUNTIME_METHODS` (`modules.mjs`, `exportRuntimeSymbols`).
- **[R]** Without threads it is an alias of the `memory` export of the module, and the property is assigned with the other exports when the module is instantiated (`tools/emscripten.py`: the assignment of the export continues with `= wasmMemory = Module['wasmMemory']`). The assertion of a debug build that `Module['wasmMemory']` is not given by the page runs before that.
- **[R]** With threads the memory is imported (`IMPORTED_MEMORY`), created by `initMemory()` before the list of runtime exports, and `Module['wasmMemory'] = wasmMemory` is one of those assignments. **In a thread `initMemory` returns at once** and the memory arrives later, in the message that loads the module (`runtime_pthread.js`: `wasmMemory = msgData.wasmMemory`), so the property would stay `undefined` in every worker. `ferroui-worker-attach.js` therefore redefines it in a thread as a property with a getter over the variable. `Module.HEAPU8`, by contrast, is assigned by `updateMemoryViews` in every thread, which is why B2.1 worked without this.

### The decision on growth: fixed

Read in the generated script **[G]**, against the two conditions of section 5:

- Every access of the glue to `HEAPU8`, `HEAP32`, `HEAPU32` is rewritten by Emscripten to `(growMemViews(), HEAPU8)`, and `growMemViews` compares `wasmMemory.buffer` with the buffer of the views: refreshed by identity. Strings are decoded through `slice`, a copy. So far the conditions hold.
- **`HEAP_DATA_VIEW` is not guarded.** The glue replaces it only inside `updateMemoryViews` (which it wraps), that is, when this thread grows the memory or happens to make a guarded access after another thread did. It writes through it directly in 43 places of that script: the pointer and length of every returned string, array or `Option` (`setInt32`, `setFloat64` into the return area), and the elements of an array of script objects (`passArrayJsValueToWasm0`, right after the allocation that may lie in new memory); it reads through it in `getArrayJsValueFromWasm0`. Most return-area writes follow a guarded access in the same function and are safe by accident; those of an `Option` that is `None`, of a number, and the two array functions are not. After a growth by another thread such an access throws a `RangeError` on a `DataView` that ends where the memory ended before.
- On the page's thread the return area is on the main stack, low in the memory, so only the array functions are exposed there (`getAllScreens`, the accept types of the file pickers). On a worker the stack of the thread is itself allocated, and all of them are.

That is the "if not" of section 5: a stale view inside the glue, which the port's script cannot renew. So the threaded build is linked with a memory that does not grow. With growth off Emscripten emits no `growMemViews` at all and creates the views once per thread **[R]** (`runtime_common.js`), so the guarded accesses and their cost go too, and an allocation that does not fit aborts with Emscripten's message that names `INITIAL_MEMORY`.

**The size is not measured.** Section 5 asks for the peak of the catalog and a margin; this branch could not build or run anything, so 512 MB is a provisional default behind a variable, and the first thing validation should replace: read `wasmMemory.buffer.byteLength` of the build without threads after a tour of the catalog (a growing memory records its own high-water mark), add the stacks of the pool, and set the default. A shared memory reserves address space, not pages, on desktop systems; phones are known to refuse large reservations, which B2.8 has to keep in mind when it chooses the module for a device.

The other way out, not taken: `-sGROWABLE_ARRAYBUFFERS=2` **[R]** (`settings.js`, `runtime_common.js`) makes the views length-tracking over a growable shared buffer, which no thread ever has to renew. Whether the glue's `new DataView(wasmMemory.buffer)` is then over that same buffer, and which browsers have the interface (the settings file says it was not usable in Firefox before 154), was not looked into. It is the candidate if a fixed size turns out to be too rigid.

`heapU8()` is correct either way: with a fixed memory the buffer never changes and the accessor returns one view for ever; in the build without threads it renews the view after each growth.

### One worker, two registrations

The page of a threaded site registers `./ferroui-sw.js?coi=1` when the host does not send the headers; the application registers the same address when it sets `register_ferro_service_worker`. Three cases:

| Site | Who registers | Result |
|---|---|---|
| Without threads | The application, `./ferroui-sw.js` | As before: the worker answers downloads only |
| Threaded, host sends the headers | The application only, `./ferroui-sw.js?coi=1` | The worker also adds headers the server already sent. Harmless, and the address is the same on both kinds of host, so that moving a site between them never swaps the worker |
| Threaded, static host | The page, then the application, both `./ferroui-sw.js?coi=1` | The second registration finds the registration it asks for **[D]** |

The download of the polyfill is a navigation of a hidden frame to an address the worker answers with the stream. In an isolated page the document of a frame must itself carry `Cross-Origin-Embedder-Policy`, and whether the browser checks that before it turns the response into a download is not something to rest on, so with `?coi=1` the stream response gets the three headers as well. Without the parameter the response is the original's.

### Tests

- `webapp/tests/software-blit.test.mjs`: the page stands in for the module with an object that has a `buffer`. Three frames: from the first buffer; after the buffer was replaced by a larger one and the old one detached (`ArrayBuffer.prototype.transfer`), with the frame beyond the old end; and from a `SharedArrayBuffer` that replaced a smaller shared one which stays alive. It asserts the pixels of each, that the accessor returns the same view while the buffer stays and a new one over the new buffer, and that the old content is there. The test server now sends the isolation headers, which the shared buffer needs. Run: passes.
- `scripts/browser/tests/storage_view.test.mjs`: a site is taken as threaded when it has `ferroui-threads.js`, and is then served isolated. The large write reads the size from `storageView.wasmMemory`, asserts that it is a `WebAssembly.Memory` with a buffer of the expected kind, and asserts growth without threads and an unchanged size with them. The service worker check expects the address of the mode and one registration. One more check, threaded only, serves the site **without** the headers with `?PreferPolyfill=true&RegisterServiceWorker=true`: two loads, `crossOriginIsolated`, the controller is `ferroui-sw.js?coi=1`, the streamed save arrives, and after one more load the page is still isolated with one registration.
- `thread_spawn.test.mjs`, `render_worker_clear.test.mjs`: the name of the controlling script.

### Not verified, most likely to fail first

1. **`storage_view` threaded has never been linked or started.** Everything the test asks of it beyond the service worker (the top level, the storage provider, the dispatcher in a module with atomics) is first exercised here; `themed_view` threaded is the nearest thing that ran.
2. **The size of the fixed memory** (above). Too small shows as an abort naming `INITIAL_MEMORY`; whether a 512 MB shared memory is granted everywhere the tests run was not tried.
3. **The download frame in an isolated page.** A navigation of a hidden frame that ends as a download, under `require-corp`, answered by the service worker. The headers are added to that response on the assumption that this is what the browser wants; if the save does not arrive in the threaded checks, this is the place.
4. **`Module.wasmMemory` in a worker**: the `defineProperty` over the property the export list assigned (an ordinary assignment in the script read, so configurable). Not exercised by any test of this step: nothing reads the memory through the port's script on a worker until the software frames of B2.3, which then depend on it.
5. **The export `ferrouiThreads`**: a second top-level statement in the file given as `--extern-pre-js`, which Emscripten writes in front of the script unchanged **[R]** (`link.py`, after the optimiser). If the export is lost, `storage_view` threaded starts without the check: it still passes served with headers and fails the check without them.
6. **Both registrations being one** **[D]**: that `register` with the address and scope of the active worker neither installs a second worker nor makes the page lose its controller. The last assertions of the new check are there for this.
7. **`-sALLOW_MEMORY_GROWTH=0` after the `=1` of the file**, replaced as the other repeated settings are; the warning Emscripten gives for threads with growth should be gone from the link.
8. **A debug build** (`--debug --threads`): Emscripten's assertion that `updateMemoryViews` runs once per thread with a fixed memory, with the glue's wrapper around it. Read as satisfied; not run.

### Validation (2026-10-08)

Nothing written for this step had to change. Built and run in headless Chrome:

- With threads: `storage_view` links and starts in a module with atomics for the first time and passes its 19 checks, among them the streamed save through the one service worker and, without the headers, one service worker that both isolates the page and streams the save; `thread_spawn` passes its 3 checks and `render_worker_clear` its 2 against the merged worker.
- Without threads: `storage_view` passes its 18 checks and `themed_view` its 30.
- The type check, the linter and the pixel test of `webapp/` pass, the pixel test with a grown and with a shared memory; the host tests of the browser crate pass (171).
- The intermittent failure of the check "the service worker is registered at the root of the site and streams the polyfill's download" of `storage_view.test.mjs` (the streamed file is never downloaded; no error in the page) is found and fixed: next heading.
- The fixed memory of the threaded build (512 MB by default) was granted in every run. The measured peak of the catalog, which section 5 asks for, is still to be taken (B2.7).

### The streamed save that never arrived: cause and fix (2026-10-08)

**It is not the isolation, and not the threads.** The first reading (the build without threads passed eight runs of eight, so the cause is on the path with the headers) was wrong: eight runs were too few on an idle machine. The failure is a race in the save picker polyfill, in both builds, and how often it shows depends on how busy the machine is.

**Cause.** `createWritable` of the polyfill's download handle (`native-file-system-adapter`, `src/adapters/downloader.js`) posts the stream to the service worker (`sw.active.postMessage({ url, headers, readablePort })`) and, in the same turn, appends the hidden frame whose address the worker is to answer. The message and the navigation of the frame reach the worker by different routes through the browser, and nothing orders them. When the `fetch` event of the navigation is delivered before the `message` event, the worker's map has no entry for the address: the worker of a threaded site fetches it from the server and hands the answer on, the worker of a site without threads leaves it to the browser, and either way the answer is the server's 404 for a file the site does not have. The stream is registered a moment later and nobody asks for it. The scenario still reports `saved=report.txt`, because the writer of the page only needs the worker's first request for a chunk, which the stream makes when it is created.

**Evidence** (headless Chrome; the check alone, in a loop; the worker instrumented to keep a list of its events with a random identity made when its script starts, read back by a message after the run):

- Every failing run has one line in the page log that passing runs do not: `Failed to load resource: the server responded with a status of 404 (Not Found)` for `/report.txt`.
- In every failing run the worker records the `fetch` of `/report.txt` (`mode=navigate`, `destination=iframe`, no entry in the map, the map empty) and then the `message` with that address, 0.0 to 0.5 ms later; in passing runs the order is the other way round. For example: `fetch .../report.txt navigate iframe hit=false` at `...78026.9`, `message .../report.txt` at `...78027.1`.
- The identity of the worker is the same in the two lines and in the line of its start, seconds earlier: the worker was not restarted between the message and the request and did not lose its map. Its registration is `activated` and the page is controlled before the save starts, in failing runs as in passing ones.
- Without threads the same order and the same 404 occur. Counts of the unchanged scripts: with threads 5 failures in 20 runs and 1 in 15 on a machine that was busy with other work, 4 in 12 with ten processes spinning; without threads 9 in 25 with ten processes spinning. With the DevTools protocol attached to the worker (which delays its events) 20 of 20 pass, which is why the first attempts to watch the failure did not see it.

**Fix**, in the page, since that is where the order can be known:

- `webapp/modules/storage/downloadFileHandle.ts` is the port's own form of the polyfill's download handle (the same protocol and the same fallback to a blob link without a worker or in Safari), with one difference: the frame is created and navigated when the worker's first request for a chunk arrives on the stream's port, not right after the stream was posted. The worker makes that request only after it has made the stream for the address, so the navigation cannot be ahead of the message any more. No waiting and no second try is involved; the writer of the page was already held back by the same request.
- `StorageProvider.saveFileDialog` (`storage/storageProvider.ts`) makes the polyfill's `FileSystemFileHandle` over that handle when there is no native save picker or the polyfill is preferred, and calls the polyfill's `showSaveFilePicker` otherwise, which then is the native picker. Whether a native picker exists is looked up when the bundle is loaded, as the polyfill does.
- `ferroui-sw.ts` records the address before it makes the stream (it did so after). The order was already safe, since the stream asks from a microtask; now it does not rest on that.

**After the fix** (a copy of each site with the rebuilt `storage.js` and `ferroui-sw.js`; the same loop, all with ten processes spinning, the condition under which a third of the runs failed before): with threads and the headers 40 passes of 40; with threads and without the headers (the worker isolates the page) 20 of 20; without threads 30 of 30. The instrumented worker now records the `message` first and the `fetch` of the frame 3.6 to 7.5 ms after it, with the entry found (16 runs of 16). The whole of `storage_view.test.mjs` passes against both copies (19 checks with threads, 18 without); the type check and the linter of `webapp/` are clean. The modules were not rebuilt: only the two script bundles changed.

The fix departs from upstream, whose service worker and polyfill have the same race: a row in `DEVIATIONS.md` (browser backend) and one in `browser-platform.md`, section 14.

## B2.3: pacing, software frames, resize, and the wait (written and validated, 2026-10-08)

Status: written on 2026-10-08, **not built and not run**. The branch was written without cargo, without the browser build and without a browser; the session that validates it builds it first. What could be checked without a build: the Rust files parse (the formatter reads them, and at a width of 120 it changes nothing in them), the script modules pass the type check of `webapp/` with the tools of the main checkout (the only errors are the missing package of the storage bundle, as in B2.1), and the page script and the test parse. Nothing below marked **[R]** has been seen to run: it was read in the sources of Emscripten 6.0.10 (`.tools/emsdk/upstream/emscripten`).

### What was written

| Piece | Where | What it is |
|---|---|---|
| Calls between threads | `interop/thread_proxy.rs` (new) | `current_thread()` (`pthread_self`, 0 without threads) and, inside the crate, `run_on_thread(thread, work)`: a boxed `FnOnce() + Send` queued for another thread with `emscripten_proxy_async` on a queue of the platform's own (`em_proxying_queue_create`, made on first use, never destroyed). Guarded by `all(target_os = "emscripten", target_feature = "atomics")`; any other build has one thread, id 0, and queues nothing. The two `unsafe` blocks are the calls and the box that crosses as a pointer |
| The shared surface object | `rendering/browser_surface_shared.rs` (new), exported as `rendering::BrowserSurfaceShared` | Section 4's object: the target id, the size in device pixels and the scaling, the kind of the target, a disposed flag; `is_ready`, `uses_contexts`, and `size_getter()`, the function the existing render targets ask at the start of a frame. **All atomics and no lock**, which is a change against section 4 ("the size and scaling under one small lock"): the two are read as one value through a version counter that is odd during a write, so that the thread of the page never meets a lock here. Host tests include a writer and a reader on two threads |
| Size changes without a top-level | `interop/canvas_helper.rs` | `add_size_changed` / `remove_size_changed`: `CanvasHelper_OnSizeChanged` also reaches subscribers of its thread, with the id the canvas was created with. The example has no top-level; in B2.5 the top-level writes the shared object itself and this may go again |
| The render timer | `rendering/browser_render_timer.rs`, `interop/timer_helper.rs`, `webapp/modules/ferroui/timer.ts` | `set_tick` starts the loop only for a timer that does **not** run in the background; a background timer is started by the thread that renders (`start_on_this_thread`), which records that thread. `request_tick_out_of_turn` (the contract of B2.4) queues one tick for that thread through `run_on_thread`; requests before the tick has started are one; the timestamp is `TimerHelper.now()` (new in the script: `performance.now()`, the clock of the animation frames of that worker). It does nothing when the caller is the thread that ticks, before the start, without a callback, and without threads. The shared timer of the platform (`BrowserSharedRenderLoop`, `new(false)`) behaves as before |
| The wake-up of the dispatcher | `browser_single_threaded_dispatcher_impl.rs` | The signal handle, on a thread without a dispatcher: an atomic flag (wake-ups before the thread has taken one are one) and a call queued for the thread of the dispatcher, recorded when the dispatcher is created. The call clears the flag and signals the dispatcher **on its own thread**, which posts its task as always. A change against section 3, which had the proxied function do what `OnSignaled` does: see "The two queues" below |
| The example | `examples/render_worker_clear` | `renderWorkerClearStart(width, height, scaling, mode, animated, reporter)`, `renderWorkerClearCreateSurface(container)`, `renderWorkerClearState()`, and new `renderWorkerClearFrames()` and `renderWorkerClearWaitForFrame(outOfTurn, timeoutMs)`. Query parameters of the page: `RenderingMode=Software2D\|WebGL1\|WebGL2`, `Frames=true`, `Wait=OutOfTurn\|NextFrame`. The container of the canvas follows the window (62.5vw by 60vh: 200 x 120 in the test's window of 320 x 200) |
| The test | `scripts/browser/tests/render_worker_clear.test.mjs` | Five checks (below) |
| `DEVIATIONS.md` | Browser backend | Three rows: the timer, the wake-up, the shared object |

How the example uses the pieces:

- **Software frames.** With `RenderingMode=Software2D` the worker creates the software target; the thread takes the target's `IFramebufferRenderTarget` once and, per frame, locks it (the target sets the size of the canvas and keeps one `RetainedFramebuffer`, a new one when the size changed), fills the pixels through `with_data` and disposes the lock, which is `putPixelData` in the worker's script.
- **The frame loop.** With `Frames=true` the thread sets the tick of a background `BrowserRenderTimer` and starts it on itself. Each tick draws a frame and counts it in an atomic. The colour is 32, 96, and 64 + (frame mod 96) with a loop, and 32, 96, 192 without one, so the two checks of B2.1 see what they saw.
- **A square in every frame**, 30 device pixels wide, 20 from the left and the top edge, in 224, 160, 32. It is what tells a frame of the right size from a stretched one: a canvas that the browser scales has the square at another place. In software it is written with the pixels. With WebGL it is copied from a small framebuffer with `blit_framebuffer`, because the GL interface of the port has no scissor; a WebGL 1 context cannot do that, its frames have no square (`marker=false` in the state line) and the test then only checks the colour.
- **The size.** The page gives the first size; the observer of the canvas **element** (unchanged script) reports every later one to `CanvasHelper_OnSizeChanged`, the subscriber writes it into the `BrowserSurfaceShared`, and the render target reads it through `size_getter()` at the start of the next frame and sets the size of the `OffscreenCanvas`.
- **The wake-up.** The page's thread creates a `BrowserSingleThreadedDispatcherImpl`, listens to its `signaled` event and hands its signal handle to the other thread, which signals after each frame. The handler calls `reporter.frames(count, wakeUps)` of the page, which writes the element `frames`. The page polls only until the first frame, as in B2.1.
- **The wait.** `renderWorkerClearWaitForFrame` takes a `Mutex`, optionally calls `request_tick_out_of_turn` on the timer, and waits on a `Condvar` with a timeout for the frame count to change; the other thread counts and notifies after each frame. On the thread of the page that wait is the busy loop of the runtime (next section).

### The two queues, and what the main thread does while it waits

- **[R]** A futex wait on the main browser thread is `futex_wait_main_browser_thread` (`system/lib/pthread/emscripten_futex_wait.c`): a loop that checks the timeout, calls `_emscripten_yield` and checks the value again. `_emscripten_yield` on the main thread runs `emscripten_main_thread_process_queued_calls`, which executes the **system** proxying queue (`library_pthread.c`). The header says of that queue that its work "may be processed at any time inside system functions" and must be "similar to a native signal handler" (`emscripten/proxying.h`).
- **[R]** Work on any **other** queue reaches a thread through its mailbox: `emscripten_proxy_async` enqueues and notifies (`em_task_queue_send`, `emscripten_thread_mailbox_send`), with `Atomics.notify` when the target waits with `Atomics.waitAsync` and with a `checkMailbox` message otherwise (`libpthread.js`). Either way it runs from the event loop of the target, never inside its spin.
- So the platform has **its own queue**, for both directions. A wake-up of the dispatcher that arrives while the main thread waits for a frame is delivered after the wait; it cannot run in the middle of the wait. And the function it runs only signals the dispatcher of its thread, which posts the usual task: the dispatcher's work stays a task of its own, with the priority it has today. The cost is one more hop than section 3 planned (queue, then `postTask`).
- The frame out of turn goes the other way on the same queue. The worker is at its event loop between frames, so the request runs at once; whether the worker's event loop turns while the main thread spins is the same question as for its animation frames, with one difference: a notified `Atomics.waitAsync` and a message both need only the worker, not the page.
- **[I]** Rust's `Mutex` and `Condvar` end in `emscripten_futex_wait` on this target, either directly (the futex implementation of the standard library) or through the C library. The second path calls `emscripten_check_blocking_allowed` (`pthread_cond_timedwait.c`), which in a build with the assertions of Emscripten warns once on the console, as an error, that the main thread blocks. The test records that line and does not fail on it.

### The checks of the test

1. and 2. The two checks of B2.1 (headers; service worker), with the state line of B2.3 (`... size=200x120 color=32,96,192 frames=1 marker=true`), the square, and, in the first, one wake-up that told the page of one frame.
3. `?RenderingMode=Software2D`: `kind=software gl=0`, the colour from edge to edge and the square in the capture.
4. `?Frames=true` (WebGL) and 5. `?Frames=true&RenderingMode=Software2D`, each: the first frame; the colour in the capture changes; **at least 3 frames are counted while a script loop keeps the main thread busy for 200 ms** (the counter is read from the atomic before and after, inside the same script call); the wake-up counter of the page grows by 5 and never exceeds the frames; the waits (five with a frame out of turn, three without, timeout 2000 ms each): asserted is that each call returns, within its timeout plus a second, and that frames go on afterwards; the window is resized to 480 x 300 and to 240 x 150, and each time the state reports `size=300x180` / `size=150x90`, the capture has the size of the window, the colour reaches the far corner of the canvas and the square is where it belongs.

The measurements are printed as `measured: ...` lines before the verdict of the check.

### Settled at validation (2026-10-08, headless Chrome)

| Question of the row | Where the answer appears | Status |
|---|---|---|
| Do a worker's animation frames run while the main thread is busy in script? | Check 4 and 5, `frames drawn while the thread of the page was busy for 200 ms` | Yes: 12 frames (WebGL) and 8 (software) drawn during the 200 ms |
| The duration of the wait with a frame out of turn | `wait for a frame with a frame out of turn: ended=... duration_ms=...` (five samples per mode) | 0.2 to 0.8 ms, once 8.4 ms (the first sample, WebGL); software 0.3 to 0.6 ms |
| Does the wait end at all without a frame out of turn (do a worker's animation frames run while the main thread spins in the runtime)? | `wait for a frame without a frame out of turn (timeout 2000 ms): ended=...` (three samples per mode) | Yes, every sample: 2.7, 13.9 and 14.4 ms (WebGL), 2.7, 14.2 and 14.8 ms (software): up to one frame interval |
| Is the wait of section 3 bounded, or is "commit and do not wait" chosen? | The two lines above: bounded if the first ends in about a frame's work; the fallback if it times out | Bounded. The wait stays; the "commit and do not wait" fallback is not needed. With a frame out of turn it costs well under a millisecond |
| Does the wake-up arrive, and how many per frame? | `wake-ups of the page: N for M frames` | It arrives: 23 wake-ups for 34 frames (WebGL), 27 for 34 (software): signals raised while one is pending are merged |
| After a resize, the new size and no stretched content | The resize part of checks 4 and 5 | Passes in both modes |
| The software mode shows the colour | Check 3 and 5 | Passes |
| Which render target headless Chrome gives (WebGL 2 or 1), and so whether the square was checked for WebGL | `render target: webgl, OpenGL ES N, the frames have / do not have the square` | WebGL 2 (OpenGL ES 3); the square was checked |

### What could not be verified without a build

- Everything in Rust beyond parsing: names, signatures and trait bounds were checked by reading, not by the compiler.
- That `em_proxying_queue_create` and `emscripten_proxy_async` link from Rust with the declarations written here (`pthread_t` as `usize`, the C `bool` as `bool`), and that a queue made by one thread serves all.
- Every behaviour in a browser: animation frames in the worker of a thread, the delivery of queued calls, `putImageData` on an `OffscreenCanvas` from the worker, the blit to the default framebuffer, the observer of an element whose canvas was transferred, and the spin of the main thread.

### Left out, on purpose

- The registry of shared surfaces by target id, and the worker looking its surface up there: the example has one surface in a static, created before the canvas so that the worker's report cannot arrive first. B2.5.
- `rendering/render_worker.rs`: the thread is still started and kept alive by the example (B2.5). `current_thread` moved into the crate because the timer and the dispatcher need it.
- The render targets still hold their state in `ThreadBound` and take a size function; `BrowserSurfaceShared` feeds that function. The render surface and the platform graphics over the shared object are B2.5.
- No wait in the base library changed, and nothing times a wait out there.
- `browser-platform.md` does not describe the new options of the example.

### Doubts, most likely to bite first

1. **Rust that was not compiled.** Likeliest: the example (closures coerced to `Rc<dyn Fn(())>` and to the tick type, the `Reporter` import without a module, method calls on trait objects), `OnceLock` statics over `Arc<BrowserRenderTimer>` and `Arc<dyn IDispatcherSignal>` (both have to be `Send + Sync`), unused-item warnings in the host build of the example, and the fn-pointer fields of the timer taking wasm-bindgen imports.
2. **The `Reporter` import.** A type and a method imported by the example without `raw_module`; the crate only has imports with one. If the glue of the Emscripten mode refuses it, the report has to become an export the page calls from a callback of its own, or a function of `ferroui.js`.
3. **Animation frames in a pthread worker.** `self.requestAnimationFrame` in the worker of a thread that returned to its event loop with the keep-alive counter; and whether Chrome paces them for a worker whose `OffscreenCanvas` came from a placeholder canvas. If there are none, the state stays at one frame and checks 4 and 5 fail at `frames > 10`.
4. **The main thread's spin and the worker.** If the worker's frames need the main thread (the wait without a frame out of turn times out), that is a result. If the frame **out of turn** also times out, the request did not reach the worker or the worker could not present: then section 3's fallback applies.
5. **The blit.** `glBlitFramebuffer` from a renderbuffer of `GL_RGBA8` to the default framebuffer of a WebGL 2 context with `antialias: false`; and that binding `GL_READ_FRAMEBUFFER` after the session bound the framebuffer of the canvas leaves the draw binding alone. A failure shows as `gl_error` in the state line and a missing square.
6. **`set_size` on every frame.** Both targets set the width and the height of the canvas at the start of each frame, changed or not (as on the main thread today). For a 2D canvas that resets the context each time. It was not changed here; the capture between two frames could in principle catch a cleared canvas.
7. **The software path and memory growth.** `putPixelData` in the worker reads `HEAPU8` of the worker's module; a growth by the main thread after the worker made its view is section 5's problem and B2.2's change (`FerroExports.heapU8()`), which this branch does not have.
8. **The id of a thread that ended.** `run_on_thread` hands the runtime a thread descriptor; for a thread that has exited that is freed memory. The crate only passes the thread of the dispatcher and a render thread that is kept alive, and the function is not public; a render thread that can end needs the timer to forget it first.
9. **A frame out of turn that is never run** (the worker gone) leaves the timer's request flag set, and later requests do nothing. Harmless while a render thread never ends.
10. **Wake-ups against frames.** The check `wakeUps <= frames` assumes nothing else signals the dispatcher of the example. Nothing does today.
11. **The first size.** The page computes it from `clientWidth` and the device pixel ratio; the observer then reports device pixels from `devicePixelContentBoxSize`. At a scale factor other than 1 the two can differ by a pixel, and the first frame is drawn at the first (the test runs at 1).
12. **The service worker check** still expects `ferroui-coi-sw.js`; B2.2 merges the two workers and changes that line of this file too.

### Validation

```
scripts/browser/setup.sh --threads && source .tools/env.sh
cargo test -p ferroui-browser --lib
scripts/build-browser.sh render_worker_clear --threads
node scripts/browser/tests/render_worker_clear.test.mjs
scripts/build-browser.sh thread_spawn --threads && node scripts/browser/tests/thread_spawn.test.mjs
scripts/build-browser.sh themed_view && node scripts/browser/tests/themed_view.test.mjs
scripts/build-browser.sh themed_view --threads
```

and `themed_view.test.mjs` against `target/browser-threads/themed_view`, served isolated as at the validation of B2.1 (the test file does not ask for isolation itself). The shared timer and the dispatcher of `themed_view` take the changed code paths in both builds: the timer is created with `false` and starts on `set_tick` as before, and the signal handle is only ever called on the thread of the dispatcher there.

The page by hand, served isolated: `?Frames=true`, `?Frames=true&RenderingMode=Software2D`, `?Frames=true&Wait=OutOfTurn`, `?Frames=true&Wait=NextFrame` (the element `wait` shows the outcome after 30 frames).

### Result of the validation (2026-10-08)

The step built and passed its five checks without a change to what was written. What it settles for the stages after it:

- Risk 2 of section 7 (waiting on the main thread) is closed in favour of the design: a worker's animation frames keep running both while the page's script is busy and while the main thread spins in the runtime, so a synchronous wait of the UI thread ends, and with a frame asked for out of turn it ends in under a millisecond.
- Risk 5 (keeping the render thread alive on events) is closed: the thread answers queued calls and animation frames after its start function has returned.
- The port's own proxying queue reaches the worker while the main thread waits, and the dispatcher is woken from the worker.
- Open as before: Skia on the thread (B2.6), and browsers other than headless Chrome.

## B2.5: the browser backend on the new objects, still on one thread (written and validated, 2026-10-08)

Status: written on 2026-10-08, **not built and not run**. No cargo, no browser build and no browser. What could be checked without a build: the Rust files parse (the formatter reads them), the script modules pass the type check of `webapp/` (the only errors are the missing package of the storage bundle, as before) and the linter for `modules/ferroui/rendering`, and the page script and the test parse. Everything here is **[M]** unless marked.

### What was written

| Piece | Where | What it is |
|---|---|---|
| The per-thread table of render targets | `rendering/web_render_target.rs` | A `thread_local!` map from target id to `BrowserRenderTarget`. `get_render_target(id)` answers from the table of the calling thread and, on a miss, asks the registry of that thread's script, wraps what it finds and keeps it; a thread whose script does not have the target gets `None` each time. `remove_render_target(id)` takes an entry out (for B2.7; nothing calls it yet). `BrowserRenderTarget` is an enum of the two kinds (`Rc<BrowserWebGlRenderTarget>`, `Rc<BrowserSoftwareRenderTarget>`) with `platform_graphics_context()` and `kind()` |
| The render targets as objects of a thread | `rendering/browser_web_gl_render_target.rs`, `rendering/browser_software_render_target.rs` | Both lose their `ThreadBound`, their size function and the surface traits. They are `Rc` objects holding the script object (and, for WebGL, the `GlInfo` and the `WebGlContext`), and each creates its render target over an `Arc<BrowserSurfaceShared>` (`create_gl_render_target(shared)`, `create_framebuffer_render_target(shared)`); a frame reads the size there and sets the size of the canvas, as before. `WebGlContext` and `verify_access` are unchanged |
| The render surface | `rendering/browser_render_surface.rs` (new), exported as `rendering::BrowserRenderSurface` | Section 4's object: an `Arc`, `Send + Sync`, holding only the `Arc<BrowserSurfaceShared>`. `is_ready` is "a thread published the target and the view is not disposed", from the atomics. `as_framebuffer_surface` and `try_get_surface_kind` answer by the kind of the render target **of the calling thread** (none while no target is published, and none on a thread that does not have it); the two `create_*_render_target` resolve that target and panic on a thread without it |
| The registry of canvases by target id | `rendering/browser_surface_shared.rs` | `register(id)`, `unregister()`, `find(id)`, `report_target(id, kind)`: one map behind a `Mutex`, holding the canvases weakly. A report for an id that no canvas is registered with yet is kept and applied by `register`. `size_getter` and the `CanvasSize` type are gone (nothing asks a function for the size any more) |
| The surface of a view | `rendering/render_target_browser_surface.rs` | `RenderTargetBrowserSurface` holds the `BrowserSurface`, the shared object and the `BrowserRenderSurface`. `new` registers the shared object under the target id and, when the canvas was created for no thread, publishes the kind of the target at once (this thread owns it). `on_size_changed` writes the shared object. `get_render_surfaces` is the one surface, from the start. `create` passes `RenderWorker::canvas_thread_id()` to the script (0 while nothing starts the worker). `dispose` unregisters |
| The platform graphics | the same file | `BrowserPlatformGraphics { shared }`: no cell, no weak handle of itself. `is_ready` is `BrowserSurfaceShared::is_ready`, `uses_contexts` the published kind, `get_shared_context` the context of the render target of the calling thread. The ready state feature it hands out is a second object over the same shared state |
| The render timer | `rendering/browser_render_timer.rs` | `set_tick` starts the loop only if the timer does not run in the background **and** no render worker exists (`RenderWorker::exists`, through the table of calls the tests replace). Without a worker nothing changed |
| The render worker | `rendering/render_worker.rs` (new), exported as `rendering::RenderWorker` | Below |
| `registerCanvas` held back | `webapp/modules/ferroui/rendering/webRenderTargetRegistry.ts`, `rendering/web_render_target.rs` (`PENDING_RENDER_THREAD`, `worker_started`) | Below |
| The example | `examples/render_worker_clear` (`main.rs`, `wwwroot/main.js`) | Uses `RenderWorker::start` in place of its own thread, `pthread_self` and keep-alive; the shared surface is created by the page's thread, registered under the target id and found there by the other thread; the frames are drawn through `BrowserRenderSurface` (`try_get_gl_surface`, `as_framebuffer_surface`), that is, through the objects the compositor will be handed. New: `?Early=true` and the export `renderWorkerClearHeldBack()` |
| The test of the example | `scripts/browser/tests/render_worker_clear.test.mjs` | One more check (six): with `?Early=true` the first frame arrives and is in the canvas; whether the script really held the canvas back is printed as a measurement |
| Host tests | the `tests` modules of the files above | 27 new: the per-thread table (wrapped once, per thread, a miss asks again, removal); the registry by id (found while alive, a report from another thread, a report before the registration, unregistering); the render surface (not ready without a target, the kind seen by the thread that has the target and not by one that has none, a thread without the target cannot create a render target); the platform graphics (ready from what another thread published, software uses no context, the panics of upstream); the worker's state (no thread, pending, running; the announcement once and on the page's thread; a failed start); the timer with a worker. A software render target can be made on the host (its script object is `null` and is never called); a WebGL one cannot, so no host test has a WebGL target in its table |
| Records | `DEVIATIONS.md` (the first three rows of the browser backend rewritten, two rows extended, three added), `browser-platform.md` (the file table) | |

### The render worker

`RenderWorker::start(timer, on_thread)` is called by the thread that creates the canvases and returns at once. The thread it starts:

1. subscribes `RenderWorker::on_render_target_registered` to the reports of its registry (`add_render_target_registered`), which is `BrowserSurfaceShared::report_target`: the kind is published in the canvas registered under the id, or kept for it;
2. calls `initialize_worker()`;
3. keeps itself alive (`emscripten_runtime_keepalive_push`, the one `unsafe` of the file);
4. runs `on_thread` (a caller's own subscriptions; the example sets the tick of its timer there);
5. starts the timer it was given on itself (`start_on_this_thread`);
6. stores its id (`RenderWorker::thread_id()`, `pthread_self` through `thread_proxy::current_thread`);
7. queues a call for the thread that started it (`thread_proxy::run_on_thread`), which tells the registry of that thread's script that the worker takes canvases (`WebRenderTargetRegistry.workerStarted(id)`).

`RenderWorker::exists()` is true from `start` on (it is what the timer asks); `canvas_thread_id()` is 0 without a worker, `PENDING_RENDER_THREAD` (-1) between `start` and step 6, and the id afterwards; `post(work)` queues a call for the thread. The functions exist in every build; without threads `start` answers with an error (`Unsupported`) and the rest answer "no worker". The thread-only code (the spawn, the subscription, the keep-alive) is behind `all(target_os = "emscripten", target_feature = "atomics")`.

**What B2.6 has to do to switch it on**, and nothing else in these files: call `RenderWorker::start(Some(BrowserSharedRenderLoop::render_timer()), None)` in the platform set-up, before the first top-level and before the render loop is asked for; create that timer with `is_background` true when the worker was started (doubt 9 of B2.4: before the compositor is created); and create the compositor of `RenderTargetBrowserSurface::new` with `Compositor::with_render_thread(.., false, ..)` when `thread_id != 0`. The surface, the graphics, the size and the hold-back are already what that mode needs.

Section 4 asked for two exports. `CanvasHelper_OnRenderTargetRegistered` exists since B2.1; the render worker is now its subscriber. `RenderWorker_OnStarted` did not become an export: nothing in script would call it. The worker's thread tells the page's thread through the proxying queue of B2.3, and what runs there calls a function of the script (an import). The size change needs no export of the worker either: `CanvasHelper_OnSizeChanged` arrives on the page's thread as before, `RenderTargetBrowserSurface::on_size_changed` writes the shared object, and the worker reads it at the start of the next frame.

The render loop is not asked for a frame when a target is reported (section 4, step 3, said it would be): `BrowserRenderTimer` ticks with every animation frame of the worker for as long as it lives, and a composition target without a render target asks its surfaces again on each tick **[M]** (`server_composition_target.rs`, `server_compositor.rs`: a frame of a compositor whose graphics is not ready asks for the next tick).

### `registerCanvas` held back

`WebRenderTargetRegistry.create(pthreadId, ..)` has a third case. With `pthreadId === -1` and no worker announced yet, it transfers the control of the canvas at once (the canvas element has to be in its final state when `create` returns: the surface of the view reads its size and the observer is attached) and puts `{ id, canvas, modes }` on a list. `WebRenderTargetRegistry.workerStarted(pthreadId)` records the id and posts every held message to the worker of that thread. After that, -1 and the real id both post at once. A real id for a thread that was never announced posts at once, as since B2.1.

Why the Rust side cannot simply pass the id: it is only known inside the thread (`pthread_self`), and the thread has not run when the first view is created in the same task as `start`. Why the announcement is safe against the order of things: the worker stores its id after it installed its handler, so a canvas that is created with the real id is never early; a canvas that read 0 for the id was created before the announcement ran on the same thread, so it is on the list when the announcement comes. If the queued call cannot be delivered (the queue could not be created), `canvas_thread_id()` makes the announcement itself the next time a canvas is created, which is on the right thread; canvases held back until then wait for that.

Between the transfer and the report the surface is "simply not ready" (section 4): the shared object has no kind, so `BrowserPlatformGraphics::is_ready` and `BrowserRenderSurface::is_ready` are false on both threads.

### What stays exactly as it was without threads, and the three places where the path differs

`RenderWorker::canvas_thread_id()` is 0, so the script creates the target on the page's thread as before, and the table of that thread is the only one. What differs in the order of calls, none of it visible:

- The render target is wrapped (and its `WebGlContext` made, which makes the context current once and restores the previous one) in `RenderTargetBrowserSurface::new`, where it used to be wrapped at the first question about the surfaces or the readiness, a little later in the same task.
- `get_render_surfaces` has the surface from the start. Before, it was empty until the target existed, which without threads was never observable (the target exists when the canvas does).
- `IPlatformRenderSurface::is_ready` of the surface is "published and not disposed"; the old targets answered the default `true`. Published is always true here.

The disposed flag of the shared object is **not** set when a view is disposed. Setting it would make the graphics of that view's compositor "not ready", and a compositor that is not ready skips its jobs and its targets (`render_core`), which is not what happens today. The disposal order of section 4 (mark, dispose the composition target, `unregisterCanvas`, drop the table entry) is B2.7; the pieces for it exist (`dispose`, `unregister`, `remove_render_target`, `RenderWorker::post`).

### What could not be verified without a build

- Everything in Rust beyond parsing.
- That the build without threads passes `themed_view.test.mjs` (30 checks), `storage_view.test.mjs` (18) and `control_catalog.test.mjs` unchanged: all three render through the rewritten surface, graphics and targets.
- The hold-back in a browser: `transferControlToOffscreen` followed by a `postMessage` of the `OffscreenCanvas` in a later task; and that the observer of the canvas element reports sizes in between.
- That a queued call reaches the main thread of the page when it was queued by a thread whose start function has not returned yet (the announcement is queued from inside the start function; B2.3 only queued from an event of the worker).

### Doubts, most likely to bite first

1. **Rust that was not compiled.** Likeliest: the two `impl` blocks of sub-traits on `BrowserRenderSurface` (`IFramebufferPlatformSurface`, `IGlPlatformSurface`) and the coercions to `Rc<dyn IGlPlatformSurface>` / `&dyn IFramebufferPlatformSurface`; the `thread_local!` holding a function pointer; `Option<Box<dyn FnOnce() + Send>>` arguments built in place; the example (imports, the `match` with a guard on `STARTED`); unused-import or dead-code warnings in one of the three builds (host, module without threads, module with threads), since `render_worker.rs` compiles different halves in each.
2. **The same example on new objects.** `render_worker_clear` now draws through `BrowserRenderSurface` and finds its shared object by id. Its five old checks are the first evidence for the per-thread table and the registry on a real second thread. The first frame is drawn when both the report and the registration have happened; the page's thread asks the worker to look again after it registered (`RenderWorker::post`), because the report can win the race. If the first frame never comes in a check that passed before, this pairing is the place.
3. **`?Early=true` may not exercise the hold-back.** The page creates the canvas in the call that started the thread; the thread could in principle report itself first (it runs in a worker of the pool at once). The test prints which it was and asserts only the frame.
4. **The announcement from inside the start function** (above). If it is lost, a canvas that was held back is posted only when the next canvas is created, and `?Early=true` never draws.
5. **The timer condition is read when the tick is set.** A render loop whose tick was set before `RenderWorker::start` has already started the loop on the page's thread; the worker then starts it nowhere (the timer is started once). B2.6 has to start the worker first.
6. **A started worker with a compositor of the page's thread.** From `start` on, every new top-level transfers its canvas to the worker, while `RenderTargetBrowserSurface::new` still creates the compositor for the page's thread: such a view never becomes ready (the page's thread has no target) and shows nothing. Nothing in the platform starts the worker, so only a host application that calls `RenderWorker::start` itself and then creates a view can meet this before B2.6.
7. **A lock on the page's thread.** The registry by id is behind a `Mutex`; the page's thread takes it when a view is created or closed, the worker when it reports a target. A collision is a spin of the page's thread for a map look-up. Section 4 asked for no lock in the shared object, and the per-frame state has none.
8. **Kept reports are never dropped.** A target reported under an id that is never registered (a canvas created by code that does not use the registry) stays in the map of kept reports: an integer per such canvas. Ids are not reused by the script.
9. **Doubt 3 of B2.4 is still open**: what is dropped when the server graph is released holds GL objects of Skia, and with several canvases one worker has several WebGL contexts. Nothing of the browser crate makes GL calls when it is dropped (`BrowserWebGlRenderTarget`, `WebGlContext` and the `GlInterface` hold handles and function pointers), so the question is Skia's context alone, for B2.6.
10. **Doubt 5 of B2.4 was not read**: what the Skia context of the browser publishes as features of the render interface, and whether such a feature may leave the lock as an `Rc`. Nothing here depends on it; B2.6 does.
11. **The tracking data** (`docs/porting/data/path-overrides.toml`) still maps upstream's `RenderWorker.cs` as not applicable, and the generated tracking page says so. The scanner was not run here; the entry and the waivers of the rendering files (`BrowserRenderTarget` is an enum now, the surface traits moved to `BrowserRenderSurface`) are for the session that runs it.

### Validation

```
scripts/browser/setup.sh --threads && source .tools/env.sh
cargo test -p ferroui-browser --lib
cargo build -p ferroui-browser --examples
(cd src/Browser/FerroUI.Browser/webapp && npm run typecheck && npm run lint && npm run test:pixels)
scripts/build-browser.sh themed_view && node scripts/browser/tests/themed_view.test.mjs
scripts/build-browser.sh storage_view && node scripts/browser/tests/storage_view.test.mjs
scripts/build-browser.sh control-catalog-browser && node scripts/browser/tests/control_catalog.test.mjs
scripts/build-browser.sh render_worker_clear --threads && node scripts/browser/tests/render_worker_clear.test.mjs
scripts/build-browser.sh thread_spawn --threads && node scripts/browser/tests/thread_spawn.test.mjs
scripts/build-browser.sh themed_view --threads
scripts/build-browser.sh storage_view --threads && node scripts/browser/tests/storage_view.test.mjs target/browser-threads/storage_view
```

Expected: 27 host tests of the browser crate more than before (the crate has 208 `#[test]` functions now); the three tests of the build without threads unchanged in number and result; `render_worker_clear.test.mjs` with six checks; `themed_view.test.mjs` against `target/browser-threads/themed_view` served isolated, 30 checks, as at the validation of B2.1 (a module with threads in which no worker is started renders on the page's thread, through the same objects).

### Result of the validation of B2.5 (2026-10-08)

The step built and passed without a change to what was written.

- The host tests of the browser crate pass (208, 27 of them new).
- Without threads, on the new objects: `themed_view` 30 checks, `storage_view` 18 checks, the pixel test of `webapp/`; the catalog is run by CI.
- With threads: `render_worker_clear` passes its six checks on the objects of the crate (`RenderWorker`, the per-thread table, the shared surface), among them the new one, where the script did keep the canvas back until the thread had reported itself; `thread_spawn` 3 checks, `storage_view` 19, `themed_view` served isolated 30.
- So B2.6 can switch the render worker on: start it before the render loop sets its tick, make the timer a background one, and create the compositor with `with_render_thread` and the flag false.

## B2.6: the compositor on the worker (written and validated, 2026-10-09)

Status: written on 2026-10-09, **not built and not run**. No cargo, no browser build and no browser. What could be checked without a build: the Rust files parse (the formatter reads them), the script modules pass the type check of `webapp/` with the tools of the main checkout (the only errors are the missing package of the storage bundle, as before), and the scripts and the test parse (`node --check`). **[R]** marks what was read in Emscripten 6.0.10 (`.tools/emsdk/upstream/emscripten`) and **[G]** what was read in the script of the threaded `themed_view` of the B2.5 validation (`target/browser-threads/themed_view/themed_view.js`); everything else is **[M]** unless marked.

### What was written

| Piece | Where | What it is |
|---|---|---|
| The switch | `browser_app_builder.rs`, `BrowserPlatformOptions::render_thread` | `true` by default: a module built with threads renders on a render thread. `false` keeps such a module on the thread of the page. No effect without threads |
| The start of the render thread | `rendering/browser_shared_render_loop.rs`, `BrowserSharedRenderLoop::start_render_thread` and `renders_on_render_thread`; called by `BrowserWindowingPlatform::register` (`windowing_platform.rs`) | In a module built with threads (`cfg!(all(target_os = "emscripten", target_feature = "atomics"))`) and while the render timer of the page does not exist yet: creates the timer with `is_background` true, hands it to `RenderWorker::start`, and makes it the timer of the page when the thread could be created. Otherwise it does nothing and the timer is created on first use as before (`new(false)`). `register` calls it after the dispatcher is installed (the render thread wakes it) and before anything asks for the render loop |
| The compositor of a view | `rendering/render_target_browser_surface.rs` | A canvas is created for the render thread only when the render loop of the page ticks there (`renders_on_render_thread`); then `thread_id` is not 0 and the compositor is `Compositor::with_render_thread(loop, gpu, false, ..)`. Over a background loop that is a compositor **confined to its render thread** ("B2.4"). With `thread_id` 0 the code is the one of B2.5, character for character |
| A tick asked for before the timer is started | `rendering/browser_render_timer.rs` | `request_tick_out_of_turn` on a background timer that no thread has started yet sets a flag, and `start_on_this_thread` ticks once when it finds it. Below, "The first show" |
| Where the frames are drawn | `rendering/render_statistics.rs` (new), exported as `rendering::RenderStatistics` | Counters of the module, all atomics: the frames that reached a canvas, the thread, kind, OpenGL ES version and size of the last one; the ticks of the frame loop of a render thread and the calls those ticks had the runtime carry to the main thread. A frame is counted where it ends: when the session of a WebGL render target is disposed, and after `putPixelData` of a software one |
| The count of proxied calls | `scripts/browser/threads/ferroui-worker-attach.js` (part 3), `webapp/modules/ferroui/ferroExports.ts` (`proxiedCalls`, `lastProxiedFunction`), `interop/thread_proxy.rs` (`proxied_calls`, `last_proxied_function`), `browser_render_timer.rs` | In the worker of a thread the script wraps `proxyToMainThread` and counts its calls on the worker's `Module`. A background timer reads the count before and after each tick and reports the difference. Below, "What a frame asks of the main thread" |
| The examples | `examples/themed_view/main.rs`, `examples/storage_view/main.rs` | `?RenderThread=false` sets `render_thread` to false. `themed_view` exports `themedViewRendering()`: `frames`, `frame_thread`, `page_thread`, `other_thread`, `render_thread`, `on_render_thread`, `kind`, `gl`, `size`, `ticks`, `proxied`, `last_proxied` as a line of `name=value` pairs |
| The host pages | `examples/themed_view/wwwroot/main.js` | Calls `ensureCrossOriginIsolated` when the script of the module says it was built with threads, as the page of `storage_view` does since B2.2 (checked: unchanged). A site without threads makes no extra request |
| The test | `scripts/browser/tests/themed_view.test.mjs` | Below |
| Host tests | `render_statistics.rs` (4), `browser_render_timer.rs` (3 new, 2 changed), `browser_shared_render_loop.rs` (1), `browser_app_builder.rs` (1) | The counters; what a background tick reports; the request before the start; no render thread without threads |
| Records | `DEVIATIONS.md` (browser backend: one row changed, five added), `browser-platform.md` (the file table) | |

**The base library was not changed.** Everything the browser needs was there since B2.4 and B2.5: the confined mode, the frame out of turn, the feature query through the cache, the release of the graph by the render thread.

### The order at start-up

1. `BrowserWindowingPlatform::register`: the dispatcher of the page is created; then `start_render_thread` creates the background timer and starts the thread. `RenderWorker::exists()` is true from here on.
2. The thread (a worker of the pool, so it runs at once and in parallel **[R]**) subscribes to the reports of its registry, installs the handler, keeps itself alive, starts the timer on itself (its animation frames begin) and publishes its id. A call queued for the thread of the page announces it to the script there.
3. The first view: `RenderWorker::canvas_thread_id()` makes the announcement itself if the thread has reported by then, and answers with the id; otherwise with `PENDING_RENDER_THREAD`. The script transfers the canvas and posts it (at once, or when the announcement comes: B2.5). The compositor is created confined: `RenderLoop::from_timer` over the background timer, the flag false.
4. The worker creates the script's render target and reports it (`CanvasHelper_OnRenderTargetRegistered`); the shared state of the canvas has a kind, so the graphics is ready on both threads.
5. The next tick of the render thread creates the backend context: `BrowserPlatformGraphics::get_shared_context` wraps the target of that thread (`get_render_target`, the `WebGlContext`, the `GlInterface`), and Skia creates its GL interface and its Ganesh context there; then the render target of the composition target, then the first frame.

### The first show, and why nothing waits for ever

The first show, a resize and the disposal of a target are synchronous commits: the thread of the page commits, asks the loop for a frame out of turn and waits for the batch (`MediaContext::sync_wait_compositor_batch`). Three things make that wait end, read in the code:

- **The wait ends with a tick, not with a picture.** `ServerCompositor::render` notifies `Rendered` for the batches it applied at the end of every tick, also of one that found the graphics not ready and drew nothing (`render_core` returns early; `NotifyRendered` runs on the way out). So the first show does not wait for the canvas to reach the worker, which in the `PENDING_RENDER_THREAD` case could not happen while the thread of the page waits (the announcement is delivered to its event loop).
- **The tick is asked for.** The commit has woken the loop (the tick callback is set), then `request_tick_out_of_turn` queues one tick for the render thread, which runs it from its event loop whether or not the page is visible.
- **A request that comes before the thread has started the timer is kept** (new). Without it the request did nothing, and the wait depended on the first animation frame of the worker, which a page that is loaded hidden does not have. The flag is taken exactly once by whichever thread gets there: the one that starts the timer, or the one that asked.

The cost is what B2.3 measured: under a millisecond with a frame out of turn, on the main thread, as a spin.

A frame can be drawn between the moment the thread of the page writes a new size of the canvas (`BrowserSurfaceShared`, in `on_size_changed`) and the commit of the layout for that size. Such a frame has the new canvas size and the old scene. The synchronous commit of the resize follows in the same task of the page.

### The audit: what a frame of the render thread reaches

Read from `ServerCompositor::render` down: `render_core`, `ServerCompositionTarget::render`, `PlatformRenderInterfaceContextManager`, the Skia backend (`PlatformRenderInterface::create_backend_context`, `SkiaContext`, `GlSkiaGpu`, `GlRenderTarget`, `GaneshGrContext`, `FramebufferRenderTarget`, `DrawingContextImpl` and the caches), `ferroui-opengl` as far as the browser uses it, and the browser crate. The desktop audits (R5.2, R5.5) covered the server side and the drawing path of Skia for the lock model; this one is for the confined mode, the Ganesh GL path and the browser's objects. "Found" lists what assumes a thread; "Verdict" what was done.

| Found | Who creates it, who uses it | Verdict |
|---|---|---|
| `BrowserPlatformGraphics` (`Rc<dyn IPlatformGraphics>`) | The thread of the page, with the compositor. The render thread calls `uses_shared_context` and `get_shared_context` by reference; the context manager never clones the handle (R5.5), and `Compositor::drop` takes it back on its own thread before the graph is released ("B2.4") | Correct as it is: it holds only the `Arc<BrowserSurfaceShared>`, and `get_shared_context` resolves the target of the calling thread |
| The ready state feature (`Rc<dyn IPlatformGraphicsReadyStateFeature>`) | Asked for once by the context manager when the compositor is created (thread of the page); `is_ready` and `uses_contexts` by reference on the render thread | Correct: a second object over the same atomics. `uses_contexts` panics before the target exists; its one caller (`ensure_valid_backend_context`) runs after `is_ready` |
| The render interface (`Rc<dyn IPlatformRenderInterface>`, the Skia `PlatformRenderInterface`) | Looked up by the thread of the page (when the compositor and when a composition target is created); lent to the render thread for `create_backend_context` | Correct: plain fields, and lent, not cloned (R5.5) |
| `get_render_target`, the table of render targets, `BrowserWebGlRenderTarget`, `BrowserSoftwareRenderTarget`, `WebGlContext`, `GlInterface`, `GlSurface`, `GlSession`, the framebuffer render target and its `RetainedFramebuffer` | The render thread, on its first frame and per frame; `Rc` and cells | Correct by B2.5: objects of the thread that draws, in a table of that thread. `WebGlContext::verify_access` still panics on any other thread |
| `BrowserRenderSurface` and the list the top-level publishes (`Arc<Mutex<Vec<Arc<..>>>>`) | Created and written by the thread of the page, read by the render thread when it creates a render target | Correct by B2.5 |
| `GlSkiaGpu`, the Skia `Interface` (`Interface::new_native`), `GaneshGrContext` (`RefCell<DirectContext>`), `GlRenderTarget`, `SkiaContext`, `SkiaGpuRenderTarget`, the sessions and surfaces of a frame | The render thread, inside the graph; `GlSkiaGpu` has no feature (`try_get_feature` is `None`) and `SkiaContext` hands out the features of its GPU, so nothing of it reaches the thread of the page | Correct: confined. The public feature map of the render interface is empty in the browser, which settles doubt 5 of "B2.4" and doubt 10 of "B2.5" for this platform |
| The thread-local caches of the Skia backend: `SkPaintCache`, `SkRoundRectCache`, `SkTextBlobBuilderCache`, the acrylic noise image | Per thread, by design (R5.2) | No change: the render thread has its own; the noise image is decoded once more there, on first use |
| `SkiaOptions` read from the service locator by every `DrawingContextImpl` | The render thread has no services | Correct since R5.2: falls back to `SkiaPlatform::options`, a static behind a lock |
| The `longjmp` bridge of the decoders (`emscripten/emscripten_sjlj.cpp`) | A decode on either thread | Correct: it has no state of its own, and `setThrew` of the runtime is per thread. Bitmaps are decoded when they are loaded, on the thread of the page (`immutable_bitmap.rs`); the only decode a frame makes is the noise image |
| Glyph runs, typefaces, geometries, bitmaps, brushes, pens, effects that a batch carries | Created by the thread of the page (shaping and measuring stay there); drawn by the render thread | Correct since R1: `Send + Sync` in `Arc`, caches behind locks. What both threads then share inside Skia (a typeface, the glyph strikes, the FreeType face behind them, the resource cache) Skia guards itself **[D]**. See doubt 2 for the one part of the binary that was not built for threads |
| `Dispatcher::ui_thread()` in `ServerCompositor::render` | Every frame | Correct: on the render thread it is the dispatcher of the page, and `check_access` is false there, so the frame does not touch its processing (R5.1) |
| The completions of batches and jobs | Posted to the dispatcher by the render thread | Correct since B2.3: the signal handle carries the wake-up across (`interop/thread_proxy.rs`); checked that `Dispatcher` takes its handle from `signal_handle()` |
| The debug overlays (the text renderer, the time graphs) | The text renderer on the thread of the page, under the lock, when overlays are switched on | No change (R5.2); a frame draws no overlay text until it exists |
| `Logger` | A frame that logs | Correct: the sink is a static behind a lock; the sink of a thread is per thread and the render thread has none |
| `DefaultRenderLoop::timer_tick` catches a panic of a frame and logs it | The render thread | No change, but see doubt 8: a frame that panics is not an error of the page |
| `BrowserSurface` (the cells for the client size and scaling), `BrowserTopLevelImpl`, the input handler, the insets, the storage | The thread of the page only | Not reached by a frame: the render thread reads the size from `BrowserSurfaceShared` |
| What the thread of the page did with the render target | `RenderTargetBrowserSurface::new` wraps the target and publishes its kind when the canvas was created for no thread | With a render thread it does neither; the worker reports the kind. Nothing else on the thread of the page names a render target |

Nothing had to change in the Skia backend. Not read: the custom draw operations and the custom visual handlers of `src/FerroUI.Controls` (they run on the thread that renders, as upstream documents; the controls are outside this step), and anything a frame reaches only on pages `themed_view` does not have (drawing surfaces, visual snapshots, the GPU interop: "B2.7").

### What a frame calls in script, and what the runtime would carry to the main thread

Imports of the wasm-bindgen glue, each a function of the worker's own copy of `ferroui.js` (settled in "B2.1"):

| When | Calls |
|---|---|
| The start of the thread | `WebRenderTargetRegistry.initializeWorker`, `TimerHelper.runAnimationFrames` |
| Each tick | `FerroExports.proxiedCalls` twice; `TimerHelper.now` for a tick out of turn |
| The first frame of a canvas | `WebRenderTargetRegistry.getRenderTarget`, the getters of the target (`renderTargetType`, a string; `contextHandle`, `fboId`, `stencil`, `sample`, `depth`, `attrs.majorVersion`), `WebGlRenderTarget.getCurrentContext` and `makeContextCurrent` |
| A WebGL frame | `WebRenderTarget.setSize`, `getCurrentContext`, `makeContextCurrent` (twice when the context was not current), and the GL entry points, which are functions of the worker's own `GL` object and are not proxied without `OFFSCREEN_FRAMEBUFFER` **[R]** ("B2.1") |
| A software frame | `WebRenderTarget.setSize`, `SoftwareRenderTarget.staticPutPixelData` (which reads the memory through `FerroExports.heapU8()` of the worker) |
| The end of a batch | Nothing in script: the wake-up of the dispatcher is `emscripten_proxy_async` on the platform's own queue. It is queued, nobody waits for it, and wake-ups raised before the page has taken one are one |

What Emscripten carries from a thread to the main thread, synchronously, is the list of its script functions marked for it **[G]**: the file system calls (`__syscall_openat`, `__syscall_fstat64`, `__syscall_stat64`, `__syscall_lstat64`, `__syscall_newfstatat`, `__syscall_getdents64`, `__syscall_getcwd`, `__syscall_fcntl64`, `__syscall_ioctl`, `fd_read`, `fd_pread`, `fd_seek`, `fd_close`), **`fd_write`**, `_mmap_js` and `_munmap_js` (a file mapping; an anonymous one stays in C), `environ_get` and `environ_sizes_get`, and `proc_exit`. A frame reaches none of them by reading: no file is opened (fonts and bitmaps are in memory before a frame sees them), the clock is `performance.now()` of the worker, the memory is fixed ("B2.2"). The one that can appear is `fd_write`: a line on standard output or standard error. That is the panic message of a frame that panics, a log sink that prints, and Skia's own diagnostics (a shader that does not compile). So the expected count for a frame is **zero**, and a count above zero is a line somebody printed.

How it is counted: every such function calls `proxyToMainThread` **[R]** (`libpthread.js`), which is a `var` of the script **[G]**. `ferroui-worker-attach.js` replaces it, in a worker only, by a function that counts and calls the original, and keeps the index of the last function (an index into `proxiedFunctionTable` of the generated script, which is how a count above zero is explained: look the index up there). The counters are properties of the worker's `Module`; `FerroExports.proxiedCalls()` answers -1 where nothing counts (the main thread, a build without threads). `BrowserRenderTimer` reads the count around each tick of a background timer and adds the difference to `RenderStatistics`; a tick of the timer of the page is not measured and makes no extra call, so the build without threads does what it did.

Not counted by this: calls proxied from C without the script (`emscripten_proxy_sync` and its relatives). The port makes none, and no path a frame takes was found that does.

### The test

`themed_view.test.mjs` takes the site directory as its argument, as before; a site is threaded when it has `ferroui-threads.js` (the way `storage_view.test.mjs` decides since B2.2), and is then opened isolated. Against a site without threads it registers the same 30 checks with the same assertions.

Against a threaded site:

- **The 30 checks, on the render thread.** One assertion differs there: "input works with the software render target too" asked the canvas of the page for its 2D context, which a canvas whose control was transferred refuses; it asserts `kind=software` from `themedViewRendering` instead. Before the checks of a threaded site start, the page is waited for until `frames` is above zero (the splash is closed by the thread of the page, the first frame comes from the other one).
- **The same 30 checks with `?RenderThread=false`** ("(one thread)" in their names).
- **The frames are drawn by a render thread, and a frame asks nothing of the main thread**: `on_render_thread`, `other_thread`, `frame_thread` equal to `render_thread` and not to `page_thread`; WebGL 2 and the size of the view; the canvas of the page gives out no context; a capture shows a view; pointer input draws further frames and a click is counted; `ticks` above zero, `proxied` counted and **0**; no error in the page; and the capture equals the capture of the same page on one thread in the same state (sampled every 4 pixels, at most 0.5 % of the samples may differ by more than 8 per channel; the count is printed).
- **`?RenderThread=false`**: no render thread (`render_thread=0`, `on_render_thread=false`), frames drawn by the thread of the page, no tick of a render thread, and the page kept its canvas.
- **`?RenderingMode=Software2D`, `WebGL1`, `WebGL2`**, each: drawn by the render thread, the kind and the OpenGL ES version of the mode, a click toggles the check box, no error, and the capture equals the one of one thread.
- **A resize** (`WebGL2` and `Software2D`), to 600 x 400 and to 380 x 560: the last frame has the new size, the capture has it, and the capture equals the one of one thread resized the same way (which is how a stretched canvas would show).
- **A hidden page**: a change of the view draws frames while the page is visible; while it is hidden (the window is minimised, or, if that does not hide it, a tab is opened in front) a second change draws none in 2.5 s; shown again, frames follow.

68 checks against a threaded site.

### What could not be verified without a build

- Everything in Rust beyond parsing, in the three builds (host, module without threads, module with threads).
- That the build without threads passes `themed_view.test.mjs` (30), `storage_view.test.mjs` (18) and `control_catalog.test.mjs` unchanged. What changed on its path: `BrowserPlatformOptions` has one more field; `register` calls a function that returns at once; the glue has two more imports (never called); `GlSession::dispose` and the blit count a frame in six atomics.
- Every behaviour of the threaded page: Skia on the render thread; the first show against a compositor whose canvas has not arrived; the three modes from the worker; the resize; the hidden page; the counter of proxied calls.
- That `storage_view` and the catalog, which now render from the worker by default when built with threads, still pass their tests (`storage_view.test.mjs target/browser-threads/storage_view`, 19 checks; the catalog threaded is "B2.7").

### Doubts, most likely to bite first

1. **Rust that was not compiled.** Likeliest: the two imports with `js_namespace = FerroExports` in `interop/thread_proxy.rs` (the first imports of the crate from that class; if the glue refuses a static method of an exported class there, they move to `TimerHelper`); `Option<Rc<BrowserPlatformOptions>>::as_ref().is_none_or(..)` in `register`; the constants of `canvas_helper` as patterns of a `match` in the example; `std::thread::scope` over a local in the tests of `render_statistics.rs`; the test closures of the timer; an unused import in one of the three builds.
2. **Skia called by two threads at once, for the first time.** The thread of the page builds paths, measures and shapes text and decodes bitmaps while the render thread rasterises. `libskia.a` is built for threads; **`libskia-bindings.a` is not** (`render-thread.md`, "B1 validated": linked with `--no-check-features`; "a `thread_local` is a plain global and the guard of a local static is not atomic" in its four objects). Until now only one thread called Skia, so this was dormant. A function-local static of the bindings that both threads initialise at the same moment, or a `thread_local` of theirs that both use, is a race no test of this step looks for. If the threaded page shows rare corruption or a crash inside Skia, this is the first place; the fix is the one B1 names (compile those four objects with `-pthread`).
3. **Ganesh on a WebGL context of a worker.** `Interface::new_native()` and `direct_contexts::make_gl` on the render thread; B2.1 to B2.5 drew with raw GL only. With `WebGL1` in particular: no test ran Skia on WebGL 1 before, on any thread, so a failure of that check may not be about threads (the `(one thread)` capture of the same mode tells).
4. **The hidden page in headless Chrome.** Whether minimising the window or a tab in front makes `document.visibilityState` "hidden" there was not tried; if neither does, the check fails with a message that says so, and needs another way to hide the page. Whether a worker's animation frames stop for a hidden page is the hypothesis of section 4 that the check exists for.
5. **The captures compared between the two modes.** Same module, same backend, same state: they should be equal. What could differ: a hover or focus transition still running when the capture is taken (the comparison waits until no frame was drawn for 400 ms), and the WebGL of a worker against the WebGL of the page in SwiftShader.
6. **The wrapper of `proxyToMainThread`.** It relies on the name and on `var` in the generated script **[G]**; with `const` or another name the assignment throws while the script loads in a worker, and no thread starts (the page stays on its splash and every threaded test fails at once). The `typeof` guard covers a missing name, not a `const`.
7. **The first frames.** That the canvas arrives, the target is reported and the first frame is drawn without the thread of the page having to do anything but return to its event loop; and that hit testing and input, which the 30 checks exercise, behave the same when the readback comes from another thread (the desktop has this since R5; the browser not).
8. **A panic in a frame is swallowed.** `DefaultRenderLoop::timer_tick` catches it and logs through `Logger`; the page sees no exception. On one thread that was so too, but there a panic in a frame usually showed elsewhere. The checks would see it as frames that stop (`framesAfter` times out) or as a `proxied` count above zero (the panic message is a `fd_write`).
9. **The console of the worker is not in the log of the test.** `page.errors` is the page target; `console.error` in the worker (a render target that could not be created) is not there. An error thrown in the worker reaches the page through the runtime; a logged one does not.
10. **A debug build.** With the assertions of Emscripten the first wait of the main thread logs "Blocking on the main thread is very dangerous" as an error ("B2.3"), and two of the 30 checks assert that the page logged no error. The release build, which the tests use, has no such line.
11. **Several canvases.** Each view has a compositor, a WebGL context and a Ganesh context of its own in the one worker. Frames make their context current; the release of a graph (a closed view) drops Ganesh's objects without making its context current (doubt 3 of "B2.4"), which with a second context current would delete objects by id in the wrong one. `themed_view` has one canvas and never closes it. "B2.7".
12. **The catalog and `storage_view` threaded** render from the worker from now on, without having been run so. The catalog can opt out only through the new field of the options (its `parse_args` does not read `RenderThread`; the sample is outside this step).
13. **The statistics are always on.** A background tick makes two calls into script for the count, and every frame writes six atomics. Negligible next to a frame, but it is not behind a switch.
14. **`RenderStatistics` is read field by field**: a reader can see the thread of one frame with the count of the next. The checks only read it when no frame was drawn for a while, or wait for a field to reach a value.
15. **A render thread somebody else started** (`RenderWorker::start` called by an application before the platform is registered): `start_render_thread` answers `false`, the page renders on its own thread and no canvas of a view goes to that thread. Before this step such a page showed nothing (doubt 6 of "B2.5").

### Validation

```
scripts/browser/setup.sh --threads && source .tools/env.sh
cargo test -p ferroui-browser --lib
cargo build -p ferroui-browser --examples
(cd src/Browser/FerroUI.Browser/webapp && npm run typecheck && npm run lint && npm run test:pixels)
scripts/build-browser.sh themed_view && node scripts/browser/tests/themed_view.test.mjs
scripts/build-browser.sh storage_view && node scripts/browser/tests/storage_view.test.mjs
scripts/build-browser.sh control-catalog-browser && node scripts/browser/tests/control_catalog.test.mjs
scripts/build-browser.sh themed_view --threads && node scripts/browser/tests/themed_view.test.mjs target/browser-threads/themed_view
scripts/build-browser.sh storage_view --threads && node scripts/browser/tests/storage_view.test.mjs target/browser-threads/storage_view
scripts/build-browser.sh render_worker_clear --threads && node scripts/browser/tests/render_worker_clear.test.mjs
scripts/build-browser.sh thread_spawn --threads && node scripts/browser/tests/thread_spawn.test.mjs
```

Expected: 9 host tests of the browser crate more than before (217); the three tests of the build without threads unchanged in number and result (30, 18, the catalog); `themed_view.test.mjs` against the threaded site with 68 checks, printing as `measured:` the pixels that differ between the two modes, the ticks, frames and proxied calls, and how the page was hidden; `storage_view` threaded with its 19 checks, now drawn by the render thread; `render_worker_clear` (6) and `thread_spawn` (3) unchanged (the first uses `RenderWorker` and a background timer of its own, and now also reports its ticks to `RenderStatistics`).

By hand, served isolated (`node scripts/browser/serve.mjs target/browser-threads/themed_view --isolated`): the page, `?RenderThread=false`, `?RenderingMode=Software2D`, `?RenderingMode=WebGL1`; `themedView.themedViewRendering()` in the console.

### Result of the validation of B2.6 (2026-10-09)

The step built on the first try; nothing in the platform had to change. Two checks were corrected, both about when a test may act, and both are things an application of the render-thread mode has to know:

- **A hidden page.** The check expected no frame of a hidden page. The render thread drew one frame in 2.5 seconds for a change the page made while hidden: its own animation frames had stopped, but the tick the page asks for out of turn when it commits is made, as it must be (a commit the page waits for is answered whether the page is visible or not). The check now allows the few frames that were asked for, instead of none.
- **Input before the first frame.** In `storage_view` a file dropped on the view right after the view existed was not delivered on the render thread, and was on one thread. A view is hit from what its last frame drew. On one thread the first frame is drawn inside the start of the application; with a render thread it is drawn a few milliseconds after the view exists, and input that arrives in between hits nothing. The check waits until the drop point is hit (`storageViewIsHit`, a new export of the example). The first frame came at the same time in both modes (about 2.6 s after navigation in a debug run here; the start of the module takes most of it).

Measured in headless Chrome, `themed_view` built with threads (68 checks):

| What | Result |
|---|---|
| The 30 checks of the example on the render thread, and again with `?RenderThread=false` | pass |
| Frames drawn by another thread than the page's | yes, in WebGL2, WebGL1 and Software2D |
| Calls proxied to the main thread during a frame | 0 |
| Sampled pixels that differ between the render thread and one thread | 0 of 14950 in the default mode, in Software2D, in WebGL1 and in WebGL2; 0 of 15000 and 0 of 13300 after the two resizes, in WebGL2 and in Software2D |
| A hidden page | 1 frame in 2.5 s (the one asked for out of turn), 11 ticks; drawn again when shown |

Other examples with this step: `storage_view` with threads, now rendering from the worker, 19 checks (six runs; one run failed in the removal of the browser's profile directory by the harness, not in a check); `render_worker_clear` 6; `thread_spawn` 3; without threads `themed_view` 30 and `storage_view` 18; the host tests of the browser crate 217.

Still open: the Skia bindings shim is linked without the atomics feature while two threads now call Skia (the fix is written on the branch `skia-shim-threads` and waits for validation); several canvases in one worker (B2.7); the catalog on the worker (B2.7); browsers other than headless Chrome.

## B2.7: the catalog and the rest (written, 2026-10-09)

Status: written on 2026-10-09, **not built and not run**. No cargo, no browser build and no browser. What could be checked without a build: the Rust files parse (the formatter reads them), the script modules pass the type check of `webapp/` with the tools of the main checkout (the only errors are the missing package of the storage bundle, as before), and the scripts and tests parse (`node --check`). The linter of `webapp/` was not run (its configuration does not resolve from a worktree). Everything here is **[M]** unless marked; **[R]** is read in Emscripten 6.0.10.

### What was written

| Piece | Where | What it is |
|---|---|---|
| The catalog host, threaded | `samples/ControlCatalog.Browser/wwwroot/main.js`, `program.rs` | The page calls `ensureCrossOriginIsolated` when the script of the module says it was built with threads, as the pages of `themed_view` and `storage_view` do. `parse_args` reads `RenderThread` (`bool.TryParse`, as `PreferFileDialogPolyfill`): `?RenderThread=false` keeps a module built with threads on the thread of the page. New exports, none a port: `catalogRendering` (the line of `themedViewRendering`), `catalogMemory`, `catalogPanicInFrame`; the page offers `controlCatalogPanics()` and samples the memory with `?MemoryReport=true` |
| The disposal of a view | `rendering/render_target_browser_surface.rs` (`dispose`, `Drop`, `release_canvas`, `release_canvas_objects`), `rendering/web_render_target.rs` (`unregister_canvas`), `rendering/browser_render_surface.rs`, `webRenderTargetRegistry.ts` (`unregister`, `releaseTarget`), `webRenderTarget.ts` and `webGlRenderTarget.ts` (`release`); after the first run also the base library: `compositor.rs` (`locked_server`, the release of a confined graph, the batches of a dropped compositor), `server_compositor.rs` (`release_gpu_resources`, `stop_rendering`, `BatchQueue::complete_all`), `server_composition_target.rs` (`release_render_target`) | Below, "The disposal of a view" and "What failed in the first run of the second view" |
| A way to close a view | `ferro_view.rs`, `FerroView::dispose` | Disposes the top-level of the view. Not from upstream, whose view cannot be closed; without it nothing outside the crate reaches the disposal |
| A panic of the render thread | `rendering/render_worker.rs` (`report_panics`, `on_panic`, `panic_message`), `interop/thread_proxy.rs` (`report_render_thread_panic`), `ferroExports.ts` (`reportRenderThreadPanic`, `renderThreadPanics`) | Below, "A panic on the render thread" |
| The statistics | `rendering/render_statistics.rs` | Two more counters: `canvases_released`, `render_thread_panics` |
| A snapshot that crosses threads is always in memory | `src/Skia/FerroUI.Skia/surface_render_target.rs`, `create_shared_snapshot`, `read_raster_snapshot` | Below, the table of offscreen paths, first row |
| The example | `examples/themed_view/main.rs` | `themedViewSecondView("open" \| "close")`: a second view in an element the page adds, and its disposal. `themedViewRendering` reports `released` and `panics` |
| The tests | `scripts/browser/tests/control_catalog.test.mjs`, `themed_view.test.mjs`; `scripts/browser/catalog-pages.mjs` (new), `harness.mjs` | Below, "The tests" |
| The comparison of the two modes | `scripts/browser/capture-catalog.mjs` (new) | Below, "The comparison" |
| Host tests | `render_target_browser_surface.rs` (2), `render_worker.rs` (1), `samples/ControlCatalog.Browser/program.rs` (2) | A released canvas is disposed, leaves the table of its thread, is of no kind any more and is unregistered with the script of the thread that created it, and only there; the message of a panic; the option and the probes of the host |
| Records | `DEVIATIONS.md` (browser backend: six rows; Skia backend: one; the sample: one) | |

As first written, the base library was not changed and the disposal was a job of the compositor (`Compositor::post_server_job`). The first run showed that a job cannot carry it; the base library has four small additions since, none of them a change of a contract of `src/FerroUI.Base/rendering` (below, "What failed in the first run of the second view").

### The disposal of a view

Upstream's order **[U]** (`EmbeddableControlRoot.Dispose`, `BrowserTopLevelImpl.Dispose`, `RenderTargetBrowserSurface.Dispose`, `BrowserSurface.Dispose`): the top-level implementation is disposed first; its surface posts a job to the compositor (`InvokeServerJobAsync`, which takes the server compositor out of the render loop: "CompositionTarget should be gone at this point too") and then destroys the surface of the script side (`CanvasHelper.Destroy`, which does nothing in script); then the root closes, which stops its renderer. Nothing sends `unregisterCanvas`, nothing releases Skia's context, and the canvas with its WebGL context lives as long as the garbage collector lets it.

The first version of the port kept the order and gave the job the work. It failed in the first run (below, "What failed"), and the job is gone. What `RenderTargetBrowserSurface::dispose` does now:

1. **The thread of the page**: the canvas leaves the registry by target id (`BrowserSurfaceShared::unregister`, as before). The thread that renders is given the release **directly, not as a job of a batch**: when this thread renders the view itself, it does the work on the spot, under the compositor lock (`Compositor::with_server`); when the compositor is confined to the render thread, the work is queued for that thread (`RenderWorker::post`) with a handle to the server compositor behind its lock (`Compositor::locked_server`, new). Then the surface of the script side is destroyed (`CanvasSurface.destroy`, a no-op as upstream's), the top-level publishes an empty list of surfaces and the root closes, as before.
2. **The thread that renders, outside a frame, under the compositor lock** (`release_canvas`): `ServerCompositor::release_gpu_resources` (new) makes the graphics context of the canvas current, releases the layer and the render target of every active target and the backend context (Skia's `SkiaContext::dispose`, `GlSkiaGpu::dispose`, `release_resources_and_abandon` of the Ganesh context, whose `DirectContext` is dropped there), and restores the context that was current before. It asks for nothing that would have to be created, so it does not depend on the graphics being ready. Then `ServerCompositor::stop_rendering` (new): the compositor still applies the batches that arrive, so that the thread of the page is answered when it waits for the disposal of the composition target, and it renders nothing and asks for no tick any more. This is the port's form of upstream's `Loop.Remove(Server)`.
3. **The same thread, next** (`release_canvas_objects`, unchanged): the shared state is marked disposed; the render target leaves the table of the thread (`remove_render_target`); `canvases_released` is counted; and the script of the thread that created the canvas is told (`unregister_canvas`): directly when that is this thread, through its event loop otherwise (`thread_proxy::run_on_thread`).
4. **The script of the thread that created the canvas** (`WebRenderTargetRegistry.unregister`) and 5. **the worker** (`unregisterCanvas`, `WebGlRenderTarget.release`, `GL.deleteContext` **[R]**): unchanged.

The work of the render thread and the release of the graph (the compositor is dropped, and the next tick of its loop task drops the graph) can come in either order, and both are sound, because **the release of a confined graph now does step 2's first half itself** (`RenderThreadLoopTask::render`: `release_gpu_resources` before `release`). If the work comes first, the graph that is dropped later has no backend context and makes no call of the graphics interface. If the graph is released first, it has released what it drew with in its own context; the work then finds no graph (`try_with` answers `None`) and does step 3 alone.

Why the disposed flag is still set by the thread that renders and not by the thread of the page: the composition target of the view is disposed after the top-level, in a batch of its own (`sync_dispose_composition_target`), and its server side releases its render target under `ensure_current`, which panics for graphics that is not ready. If that batch reaches the render thread before the queued work, the target releases its render target in the ordinary way; if the work comes first, the target finds nothing left to release (`reset_render_target` returns before it asks for the context).

**A view that is dropped without `dispose`** takes the same path: `RenderTargetBrowserSurface` has a `Drop` that calls `dispose` when nobody did. The surface is dropped with the top-level implementation, on the thread of the page, while it still holds its compositor.

The same code runs without a render thread: the release runs on the thread of the page, inside `dispose`, and tells its script directly.

What it does not cover:

- **A canvas whose release cannot be queued for a render thread that runs** (the queue of calls could not be created): the canvas is only marked as disposed; the worker keeps the canvas and its context, unused. A view that is closed before the render thread has reported itself is let go of on the thread of the page (the script still holds its canvas back and forgets it).
- **A panic in the release of a confined graph** (`release_gpu_resources` in the tick that releases): the graph is then not released, and the tick is tried again. The only way found to get there is a graphics context that the script has deleted while the backend context lives, which no path does.
- **The canvas element** stays in the document, as upstream's does: a view that is opened again in the same element adds a second canvas to it.

### What failed in the first run of the second view, and why (2026-10-09)

Reproduced against the threaded `themed_view` site with a probe that opens and closes the second view and samples `themedViewRendering()`; the errors of the worker were read through the DevTools protocol (auto-attach to the worker of the render thread). Four combinations, all failing: the render thread and `?RenderThread=false`, each with WebGL2 and with `?RenderingMode=Software2D`.

| Observed | Where |
|---|---|
| `released` stays 0 after `themedViewSecondView("close")`, for as long as the page is watched | all four |
| `TypeError: Cannot read properties of undefined (reading 'finish')` in `_emscripten_glFinish`, once, right after the close. On the render thread it is thrown inside the animation frame of the worker (`TimerHelper_JsExportOnAnimationFrame` is at the bottom of the stack); with `?RenderThread=false` it is an uncaught exception of the page. The ten innermost functions of the module are the same in both | WebGL2, both threads |
| After the close the ticks of the render loop stop (`ticks` 24, then 26, then 26 for ever) and no frame is drawn again | the render thread |
| After the close the first view still counts clicks (`clicks` goes on in `themedViewState`) and **never draws again** (`frames` stays), with no error anywhere | Software2D, both threads; a click before the close draws two frames each time |
| The canvas of the closed view stays in the document; a second open adds a third canvas | all four |

The causes, in the order in which they act:

1. **The compositor of a closed view is dropped before its last batch is applied, and the release was a job of that batch.** `post_server_job` asks for a commit; the media context holds the compositor in its list of requested commits (`requested_commits`, an `Rc`), commits it in its next pass, and drops its list. The example lets go of the view right after `dispose`, so that list was the last holder: the compositor is dropped in the same call that committed the batch. `Compositor::drop` then releases the graph: at once in the mode without a render thread, and in the confined mode by asking the loop task for the release, which the next tick does **instead of rendering**. The batch with the job is never applied in either mode. Evidence: `released` stays 0 in all four combinations, and the exception below is thrown by the tick (or by the pass of the media context), not by the call that closed the view. This is doubt 3 of the list below, with another reason than the two it named.
2. **The graph is dropped with Skia's context alive and no graphics context current.** A frame makes its context current and restores the one before (`WebGlContext::ensure_current`, `RestoreContext`); before the first frame none was current, so between two frames none is, and `GLctx` of the script of the module is undefined. The destructor of a Ganesh `DirectContext` that was not abandoned finishes its outstanding work first, which is `glFinish`. Evidence: the function that fails, in both thread modes with the same inner stack, and its absence with `Software2D`. With two WebGL views that both drew, the context that happens to be current would be the other view's, and the objects of the closed view would be deleted there: the corruption doubt 3 of "B2.4" describes. The exception is thrown by script through the frames of the module (it is not a panic, so nothing catches it): on the render thread it leaves the animation-frame callback before the callback asks for the next frame, which is why the loop of the page stops for every view.
3. **The media context waits for ever for the batch of the dropped compositor.** It commits no compositor while a batch it committed is not processed (`commit_compositors_with_throttling` returns early while `pending_composition_batches` is not empty), and the entry of a batch is only removed when the batch is processed. A batch whose compositor is dropped before a frame applied it is never processed. Evidence: with `Software2D`, where nothing throws, the first view takes input and never draws again, on both threads. This one is independent of the browser: any compositor that is dropped with a committed batch in its queue stops every other compositor of the process.

Not causes, checked: the two views draw correctly side by side before the close (the first failure doubt 2 expected did not happen); `GL.deleteContext` was never reached; nothing was observed of a compositor ticking for ever, because nothing got that far. Doubt 5 (a closed view whose compositor lives on asks for a tick for ever) is real by reading (`render_core` returns `true` for graphics that is not ready) and is what `stop_rendering` answers.

The fix, by cause:

| Cause | Change | Where |
|---|---|---|
| 1 | The release is not a job. The thread that renders is handed it directly and enters the graph under the lock; it runs whether or not the compositor commits again and whether or not its graphics are ready | `render_target_browser_surface.rs` (`dispose`, `release_canvas`), `compositor.rs` (`Compositor::locked_server`) |
| 2 | What a compositor drew with is released with its own context current: by the release of the view, and by the release of a confined graph before the graph is dropped. A view that is dropped without `dispose` disposes its surface | `server_compositor.rs` (`release_gpu_resources`), `server_composition_target.rs` (`release_render_target`, split out of `reset_render_target`), `compositor.rs` (`RenderThreadLoopTask::render`), `render_target_browser_surface.rs` (`Drop`) |
| 3 | A compositor that is dropped completes the batches that are still in its queue, unapplied (`BatchQueue::complete_all`, called at the end of `Compositor::drop` in every mode) | `server_compositor.rs`, `compositor.rs` |
| doubt 5 | The compositor of a closed view stops rendering and asks for no tick | `server_compositor.rs` (`stop_rendering`, `render_core`), called by `release_canvas` |

`MediaContext` was not changed (another piece of work has it). A second guard there would be cheap and is left as a note: `commit_compositors_with_throttling` could drop the entries of `pending_composition_batches` whose compositor is gone before it decides to wait.

**None of this was built or run.** What the next run of `themed_view.test.mjs` against the threaded site should show: 72 of 72; in the two checks of the second view on each thread, `released` goes from 0 to 1 after the first close and to 2 after the second, `panics` stays 0, the page logs no error (no `finish`), and the first view counts its click and draws a frame after each close. Without threads: 32 of 32. The host tests of the base library that drop a confined compositor (`render_thread_tests.rs`) now see the graphics context made current and the backend context disposed by the render thread before both are dropped; they assert neither.

### Offscreen rendering asked for by the thread of the page

Read against the confined mode ("B2.4"): the thread of the page has the render interface of Skia in its locator and no graphics context; `WebGlContext::verify_access` refuses every thread but the one that wrapped it.

| Path | Used by the catalog | What runs where | Verdict |
|---|---|---|---|
| `Compositor::create_composition_visual_snapshot` | `Pages/open_gl_page.rs` (the Snapshot button, which is only shown under a `Window`: hidden in a browser) | A post-target job (`compositor.rs`). On the render thread: `ensure_current`, an offscreen layer on the Ganesh context of that thread, the visual drawn into it, `create_shared_snapshot`, the layer disposed, the context restored (`server_compositor.rs`). The bitmap that crosses is a raster copy (`make_raster_image`). **Was not sound in one case**: when that copy failed, `create_shared_snapshot` handed out the image of the GPU surface itself, in an `Arc` that then crossed to the thread of the page | **Fixed**: a shared snapshot of a surface with a GPU context is always in memory: the copy of the context, else the pixels read from the surface, else (a lost context) transparent pixels of its size. A surface without a GPU context is unchanged |
| `RenderTargetBitmap` (`with_dpi`, `render`) | No page names it; `src/FerroUI.Controls/animation/connected_animation.rs` does | `create_render_target_bitmap` makes a `WriteableBitmapImpl` and a framebuffer render target without a graphics context; `render` goes through `ImmediateRenderer`, which has no compositor and takes no lock | Sound: the thread of the page, in memory |
| Custom draw operations (`context.custom`) | `Pages/TabbedPage/FluidNavBar/fluid_nav_bar.rs` | The trait is `Send + Sync + 'static`; `render` runs in the replay of a frame, on the thread that renders. The Skia lease it can ask for there hands out the canvas (and the GPU context) of that thread. The operation of the catalog captures plain values and an `Arc<Mutex<..>>` | Sound |
| Custom visual handlers (`create_custom_visual`) | `Pages/composition_page.rs` | The factory is `Send` and, by its documentation, runs on the render thread (where it runs was not traced); messages are `Arc<dyn Any + Send + Sync>`, sent as one job per composition update; `on_render` and `on_animation_frame_update` are driven by the server | Sound |
| Drawing surfaces, the GPU interop (`try_get_composition_gpu_interop`, `create_drawing_surface`) | `Pages/OpenGl/open_gl_interop_page.rs`, and `OpenGlControlBase` under `Pages/open_gl_page.rs` | The Skia backend context publishes no feature and `GlSkiaGpu::try_get_feature` is `None`, so the interop is absent before anything is created: `try_create_compatible_gl_context` answers `None`, the interop page shows "Compositor OpenGL interop is not available on this platform", and `OpenGlControlBase::initialize` logs and gives up. No surface, no GL call, no wait. `BrowserPlatformGraphics::create_context` (which panics) is not on the path | Not reachable in the browser, in either mode |
| `try_get_render_interface_feature` | Not called by the catalog | Confined: from the cache, or `None` and one job ("B2.4", change 1). The map is empty in the browser | Sound |
| `WriteableBitmap` | None | In memory | Sound |

The information text of the OpenGL page is set from `on_open_gl_render`, which never runs in a browser: the page shows its knobs and an empty text there, in both modes.

### Popups and text input

**Popups are overlays of the view.** `BrowserTopLevelImpl::create_popup` answers `None` (as upstream **[U]**), and the one caller of the platform's `create_popup`, `OverlayPopupHost::create_popup_host`, then makes an `OverlayPopupHost` on the overlay layer of the same top-level. `Popup` goes through it, and tooltips, context menus and flyouts go through `Popup`. `BrowserTopLevelImpl::new` has one call site (`ferro_view.rs`): no popup gets a canvas. A `Window` cannot be created in the browser at all (`BrowserWindowingPlatform::create_window` panics); the buttons of the catalog that open one (`Pages/dialogs_page.rs`) do in both modes what they did.

**The caret and the IME element wait for nothing.** The rectangle comes from `InputMethodManager::update_cursor_rect`: `transform_to_visual` (matrices of the visual tree) and the caret bounds of the text presenter (its text layout), both on the thread of the page; `BrowserTextInputMethod::set_cursor_rect` hands it to `InputHelper.setBounds`, which places the input element in the DOM (`caretHelper.ts`). It is triggered by a `TransformTrackingHelper` that posts to the dispatcher at the priority after render: a job of the thread of the page, not a wait for a frame. Nothing on the path reads the readback of the compositor or enters `MediaContext::sync_wait_compositor_batch`. No change.

### A panic on the render thread

What happens to it: `DefaultRenderLoop::timer_tick` catches the panic of a frame, logs it through the logger and returns; the next tick runs. That is upstream's behaviour **[U]** (`RenderLoop.TimerTick`: `try`, `catch (Exception)`, a log line "Exception in render loop", and the loop goes on), and it is kept: the loop **continues**. A compositor does not know that a frame of it failed; the guards of `ServerCompositor::render` still tell the batches of the tick "rendered", so a thread of the page that waits for the frame is released.

What was missing is that the page hears of it: the render thread has no log sink, and a line on the console of a worker is not on the console of the page. `RenderWorker::start` now wraps the panic hook of the process once (the hook that was there runs first). When the thread that panics is the render thread, the hook counts the panic (`RenderStatistics::render_thread_panics`) and queues a call for the thread of the page (`thread_proxy::run_on_thread`, nobody waits for it), which calls `FerroExports.reportRenderThreadPanic(message)`: `console.error("FerroUI: the render thread panicked: <message> (<file>:<line>)")`, and the message is kept in `FerroExports.renderThreadPanics`. `page.errors` of the harness holds every `console.error` of the page, so a test that asserts "no error was logged" now fails on a frame that panicked on the render thread.

The hook for a test: `catalogPanicInFrame()` posts a job that panics to the compositor of the view. `control_catalog.test.mjs` calls it on a threaded site and asserts the error line, the kept message, `panics=1`, and that the view goes on taking input and drawing.

### The tests

`control_catalog.test.mjs` takes the site directory as its argument; a site with `ferroui-threads.js` is opened isolated. What changed for every site:

- **Readiness.** A page is waited for until a frame was drawn and something of its view is hit (`waitUntilReady` of `catalog-pages.mjs`: `frames` of `catalogRendering`, `hit` of the elements of `catalogState`). `catalogState` already told: its `hit` is the hit test of the view, which answers from the last frame. Before, the first check that reads `hit` without waiting ("the navigation drawer opens from its toggle button") could run before the first frame of a render thread. Every other place that sends input goes through `page.find` with a filter on `hit`, which waits. `storage_view.test.mjs` has one place that sends input to the view (the drop), which waits since the validation of "B2.6"; nothing changed there.
- **The kind of the canvas** is asked of the thread that draws when the canvas was transferred (`catalogRendering`: `kind`, `gl`), and the size of the canvas is the size of the last frame there; otherwise as before.
- **One more check for every site: the memory after a tour of pages** (below).

Against a threaded site, three more: the frames are drawn by the render thread (the ids, WebGL 2, the canvas of the page hands out no context, a navigation is drawn by that thread; the calls of its ticks to the main thread are printed, not asserted: the catalog was never measured); `?RenderThread=false` (no render thread, frames by the thread of the page, the page kept its canvas, navigation works); and the panic. 13 checks against a site without threads, 16 against a threaded one.

`themed_view.test.mjs`: **a second view is drawn and closed while the first one goes on drawing**, in the default mode and in `Software2D`. The page adds an element, `themedViewSecondView("open")` puts a view of one colour in it, the colour is on the screen, the first view takes a click; the second view is closed, `released` goes up by one, the first view takes input and draws, and its picture equals the one before the close in the same state; a view opened and closed again is released again; no panic of the render thread, no error. It runs like the other checks: once without threads (32 checks), and on the render thread and on one thread against a threaded site (72).

The helpers that compare two captures (`differing`, `colours`) moved from `themed_view.test.mjs` to `harness.mjs`.

### The comparison

`node scripts/browser/capture-catalog.mjs [<threaded site>] [--mode WebGL2] [--out <directory>]` visits a tour of thirteen pages (`TOUR` of `catalog-pages.mjs`) in one page drawn by the render thread and in one with `?RenderThread=false`, both with `?PrefetchAssets=false`, at 1024 x 1100, and prints per page how many sampled pixels differ: every fourth pixel, 8 per channel, a page passes with at most 0.5 % different (the tolerance of the tests of "B2.6"). The tour: Platform Information, Data Validation, OpenGL (its knobs and information text), Image and Container Queries (pictures), Composition (a custom visual handler; it animates, so it is visited and not compared), Border, TabControl, ListBox, TextBox (text input; nothing has the focus, so no caret blinks), TextBlock, Slider, Buttons. A page is captured when no frame was drawn for 700 ms; one that keeps drawing is reported as not compared. The corner where the catalog draws its frame counter (520 x 28 pixels at the top left) is left out: it shows another number in every run. The script fails when a page differs by more, when a run was not drawn by the thread it was meant for, or when a page logged an error.

### Memory

`catalogMemory()` answers `top=<bytes>;peak=<bytes>;size=<bytes>`: `top` is the end of the dynamic memory of the module (`sbrk(0)`), below which everything the module uses lies (its data, the stack of the page, and what the allocator took from the system: the stacks of the threads are allocations); `peak` is the largest `top` a call has seen, and with `?MemoryReport=true` the page calls it ten times a second; `size` is the memory of the module (`emscripten_get_heap_size`): the fixed size of a threaded build, or what the memory has grown to without threads. The last check of `control_catalog.test.mjs` opens the catalog with the prefetch on, visits the tour, waits for the prefetch to end and prints the end of the dynamic memory after each page and, as `measured:`, start, end, peak and size. That peak, of a threaded site in WebGL2, is the number section 5 asks for. **The default of 512 MB was not changed.**

### What could not be verified without a build

- Everything in Rust beyond parsing, in the three builds (host, module without threads, module with threads).
- The catalog on the render thread: it has never been run so. Its checks, the tour and the comparison are untried against any site.
- The disposal in a browser: the second view itself (two views in one page have not been run in this port), the order of the job and the batch, `GL.deleteContext` after Skia let go, the message to the worker.
- That the panic of a job reaches the hook on the render thread and the page.
- The numbers of the memory check.

### Doubts, most likely to bite first

1. **Rust that was not compiled.** Likeliest: `js_unregister as fn(i32)` (a wasm-bindgen import as a function pointer in a `thread_local!`); the closure handed to `std::panic::set_hook` (inference of `&PanicHookInfo`); `ElementComposition::get_element_visual(&top_level)` with a `Ref<TopLevel>`; `self.top_level.dispose()` on the view (the method the class macro generates); `Surface::read_pixels` and `images::raster_from_data` in `read_raster_snapshot` (written after the Graphite readback); the `extern "C"` block inside `module_memory`; an unused import in one of the three builds.
2. **Two views in one page.** `themedViewSecondView` is the first second `FerroView` of the port: global handlers that assume one view, focus, the splash look-up, two compositors over one render loop, and with threads two canvases in the worker. A failure of the new check before its `close` is this, not the disposal.
3. **The job never runs.** It needs a commit of a compositor whose top-level has just left the media context (`request_commit_async`; read: the media context commits whatever is in its list of requested commits), and graphics that is ready. If `released` does not go up, look here first. **It did not run** (the batch was committed and never applied, because the compositor was dropped); the release is no longer a job.
4. **`GL.deleteContext`.** Still untried: the first run never got to it. Read in `libwebgl.js` **[R]**: it calls `JSEvents.removeAllHandlersOnTarget` when `JSEvents` is linked, and with threads `_free` on the handle. Called after Skia abandoned its context and with the Rust `WebGlContext` still alive in the context manager until the compositor is dropped; nothing was found that makes that context current again (a compositor that is not ready renders nothing, and its release drops without GL calls). If something does, `makeContextCurrent` answers false and the frame panics, which the new report would show.
5. **A closed view whose compositor lives on.** Its graphics is "not ready" for good: every tick of the loop asks for the next one for it (`render_core` returns early with `true`), until the compositor is dropped; a job posted to it afterwards (a snapshot) never runs and its task never completes. Since the fix the compositor of a closed view is stopped (`stop_rendering`) and asks for no tick; a job posted to it afterwards is dropped with its batch, and its task still never completes.
6. **The catalog under cross-origin isolation.** The iframe of the Native Embed page loads another site, which a page with `require-corp` blocks; the check filters error lines that name that site, and the blocked load may be logged in other words. The preload check counts requests by initiator, with a pool of workers in the page for the first time.
7. **The tour.** It finds entries in the drawer by their text and scrolls the drawer when an entry is not in view; the layout of the drawer at 1024 x 1100 was not seen. The frame counter is assumed to stay inside 520 x 28 pixels. A page of the tour that animates without being marked is reported as not compared, not as a failure.
8. **The peak of the memory.** `sbrk(0)` is the high-water mark only while the allocator never gives memory back at the top; Emscripten's default allocator was not read for that, which is why the page samples. Allocations of the render thread count (one memory); what the browser itself holds for the WebGL contexts and the canvases does not, and is not part of the fixed size either.
9. **A panic while the thread of the page waits.** The guards release a wait for "rendered". A wait for "processed" (the disposal of a composition target) whose batch panics while it is applied is released by the next tick, which needs an animation frame of the worker or another request. And the hook that ran before prints the panic to standard error, which the runtime carries to the main thread synchronously: the page may log the panic twice, and `proxied` counts that line.
10. **The hook is global and stays.** It is installed when the render thread was created (never on the host, where none can be) and runs for every panic of the module, also for those that are caught; for a thread other than the render thread it only compares two numbers. A panic of the render thread outside a frame (in the handler of the registry) is reported too, and then unwinds into the worker as before.
11. **A view dropped without `dispose`**, and **a view closed before its canvas arrived**: above. Both are handled since the fix, by reading only: no check drops a view without closing it.
14. **The fix of the disposal was written without a build**, like the rest. Likeliest to fail: the closure handed to `RenderWorker::post` (it captures an `Arc<LockedServerCompositor>`, which is `Send` by an `unsafe impl`); `server.try_with(|server| release_canvas(server, ..))` (a `&Rc<ServerCompositor>` where a `&ServerCompositor` is asked for); the `Drop` of `RenderTargetBrowserSurface`; an import that is unused now.
15. **The work of the render thread runs outside a frame.** It enters the graph from the event loop of the worker, under the compositor lock, where so far only frames did. `release_gpu_resources` touches the targets and the context manager and nothing that asserts a frame (`verify_access`); that was read, not run.
16. **A second view in the element of a closed one.** The canvas of the closed view stays in its element, with its context deleted; the check opens the second view again in the same element and expects its colour on the screen. Which of the two canvases is on top was not seen: the first run never drew after the first close.
17. **Batches completed without being applied.** A waiter for "rendered" of a batch of a dropped compositor is told so though nothing was rendered. Nothing waits there today (the compositor is gone and the waits are made by its thread); a continuation of `processed` runs on the thread that drops the compositor.
12. **The fallback of the snapshot** was never taken in a test: a context that cannot copy its surface is a lost one. The Metal path of the desktop takes the same code; there it used to hand out the GPU image, which is the unsound thing on any render thread.
13. **The tracking data** (`docs/porting/data/path-overrides.toml`) was not touched; `FerroView::dispose` and the new script functions have no upstream counterpart to map.

## Skia shim: the bindings of Skia compiled for threads (written and built, 2026-10-09)

Since B1 the threaded link passed `-Wl,--no-check-features`, because the linker refused the bindings shim of Skia in a module with shared memory. From B2.6 on two threads of the module call Skia (the UI thread measures and shapes text and decodes images, the render worker rasterises), so what the flag hid is a data race and not a formality. This section is the investigation, the fix as written, and how to check it. Nothing here was built: the rules of the session allowed reading existing files only.

### How the two archives are produced

Read in `skia-bindings` 0.153.3 (the version of `Cargo.lock`), in the cargo registry:

- **Nothing is compiled for this target.** `build.rs` (`main`) has three ways. With `SKIA_SOURCE_DIR` and `SKIA_LIBRARY_SEARCH_PATH` it takes the Skia libraries from a directory, compiles the shim and generates `bindings.rs` itself (it needs the definitions in `SKIA_BUILD_DEFINES`, and libclang for the binding generator). With `SKIA_SOURCE_DIR` alone it builds Skia from that source. Otherwise, with the feature `binary-cache` (a default), `build_support/binary_cache/download.rs` (`try_prepare_download`) downloads `skia-binaries-<key>.tar.gz` from `SKIA_BINARIES_URL` (default: the releases of `rust-skia/skia-binaries`, tag = the version of the crate) and unpacks it into `OUT_DIR/skia`; only if `FORCE_SKIA_BUILD` is set or the download fails does it build Skia from source (`gn`, `ninja`, the whole of Skia, with the source downloaded into the directory of the crate). The key is the first twenty digits of the revision the crate was packaged from, the target and the features: here `b7f043e0b1e2a850e702-wasm32-unknown-emscripten-ganesh-gl-jpegd-jpege-pdf`.
- **The archive holds both libraries and the bindings**: `libskia.a` (1020 objects, 13.4 MB), `libskia-bindings.a` (4 objects, 156 kB), `bindings.rs`, `key.txt`, `tag.txt` and Skia's licence. The log of the build script in the target directory of the threaded build (`target/threads/wasm32-unknown-emscripten/browser/build/skia-bindings-*/output`) shows the download and no compile.
- **`libskia.a` is Skia**, built by Skia's own build (`gn` and `ninja`). Its configuration for WebAssembly passes `-pthread` to every compile: `gn/skia/BUILD.gn` of the Skia source, `config("wasm") { common = [ "-pthread" ] ... cflags = common }`.
- **`libskia-bindings.a` is the shim**: the crate's `src/bindings.cpp`, `gl.cpp`, `gpu.cpp` and `ganesh.cpp` for this feature set (`build_support/skia_bindgen.rs`, `Configuration::new`), compiled by the `cc` crate (`generate_bindings`: `cc_build.compile("skia-bindings")`) on the machine that published the binaries. The `cc` crate reads `CXXFLAGS_wasm32_unknown_emscripten`, which is why `build-browser.sh --threads` exports `-pthread` in it, but that only acts where the compile runs, and it does not run here. The members of the archive have the names the `cc` crate gives (`0602fb52cb66f316-bindings.o`).
- `CC`, `CXX`, `EMCC_CFLAGS` and the `CFLAGS` variables therefore change nothing in the Skia part of a build from the published binaries. `EMCC_CFLAGS` still reaches every `emcc` the other build scripts start (HarfBuzz, the setjmp bridge).

### The evidence

```
node scripts/browser/wasm-features.mjs --summary <OUT_DIR>/skia/libskia.a <OUT_DIR>/skia/libskia-bindings.a
```

run on the published files in the target directory of the threaded build (`wasm-features.mjs` reads the `target_features` section of every object; the SDK has no `wasm-objdump`):

| Archive | Objects | Target features |
|---|---|---|
| `libskia.a` | 1020 of 1020 | `atomics bulk-memory mutable-globals sign-ext` |
| `libskia-bindings.a` | `bindings`, `gl`, `ganesh` | `mutable-globals sign-ext`, and `-shared-mem` (disallowed) |
| `libskia-bindings.a` | `gpu` | `mutable-globals sign-ext` |
| `libembedded_harfbuzz.a`, `libferroui_emscripten_sjlj.a` of the threaded build | all | `atomics bulk-memory` and others |

So the comment of B1 was right about `libskia.a` (and it holds for all its objects, not the 40 sampled in B0). `-shared-mem` is what the compiler records when it compiles thread-local storage without atomics: it turns the variables into plain globals and forbids the object in a shared memory, which is the error the linker gave. Three of the four objects therefore do contain thread-local variables that are one global for both threads today, besides the reference counts of the inline `sk_sp` code and the guards of local statics. `llvm-nm` of the SDK on the published shim shows what it was compiled with otherwise: 774 functions defined, 765 of them the `C_` functions of the bindings; no position-independent code; and, by which of the conditional functions are present or referenced, the definitions `SK_CODEC_DECODES_JPEG`, `SK_CODEC_ENCODES_JPEG`, `SK_SUPPORT_PDF`, `SK_ASSUME_WEBGL`, `SK_FONTMGR_FREETYPE_DIRECTORY_AVAILABLE` and `SK_FONTMGR_FREETYPE_EMPTY_AVAILABLE`.

### The fix

The published archive is kept, with the shim compiled again with `-pthread`: `scripts/browser/skia-threads-shim.sh`, run by `scripts/browser/setup.sh --threads`. Its header lists the steps; in short it downloads the crate file (checked against the checksum of `Cargo.lock`), the published binaries for the key, and the Skia source of the tag the crate names; gets the preprocessor definitions of the Skia build the way the crate does in a build from source (`gn gen` with the crate's arguments for the feature set, then the `defines` lines of `obj/skia.ninja` and `obj/gpu.ninja`); compiles the four sources with the flags of the `cc` crate and of the build script plus `-pthread`; checks the result; and writes `.tools/skia-threads/<id>/skia-binaries-<key>.tar.gz`, where the id names the version of the crate, the features, the version of Emscripten and the format of the script. `scripts/build-browser.sh --threads` exports `SKIA_BINARIES_URL=file://.../skia-binaries-{key}.tar.gz`, which the crate supports for binaries from elsewhere, refuses to build when the directory of the id is missing, and no longer passes `--no-check-features`. `libskia.a` and `bindings.rs` are the published files, so the Rust side of the bindings is what it was. The build without `--threads` reads none of this.

Routes not taken, and why:

- **The crate's own split** (`SKIA_SOURCE_DIR` with `SKIA_LIBRARY_SEARCH_PATH`) would compile the shim inside the build, which is the neatest, but it also generates `bindings.rs` on the machine: that needs libclang on the host (the SDK ships none) parsing the C++ library of the SDK, and replaces a file the published build generated with one from another compiler. It needs the same Skia source and the same definitions as the route taken.
- **A build of Skia from source** (`FORCE_SKIA_BUILD`) compiles 1020 objects for each cargo output directory, clones Skia's third-party sources and writes into the cargo registry; section 13 of `browser-platform.md` notes that it fails today.
- **A patched copy of the crate** (`[patch.crates-io]`) is not needed: the crate already has the variable.

The way back, for finding out whether a fault comes from the shim built here: `FERROUI_BROWSER_SKIA_PUBLISHED_SHIM=1 scripts/build-browser.sh <application> --threads` links the published shim with `--no-check-features`, as before. It is unsafe with two threads in Skia.

### Cost

Once per tools directory and per version of the crate or of Emscripten, in `setup.sh --threads`: about 200 MB of downloads and unpacked source (the Skia source is most of it; the work directory is deleted at the end), a `gn gen` (seconds), and four compiles, of which `bindings.cpp` (4200 lines against most of Skia's public headers) is the long one: expected well under two minutes in all on the development Mac, not measured. The kept result is the archive (about 4.5 MB) and two text files. A build pays nothing: the crate unpacks a local file where it downloaded one. The first threaded build after the change runs the build script of `skia-bindings` again (it watches `SKIA_BINARIES_URL`) and links again.

### Validation

```
scripts/browser/setup.sh --threads && source .tools/env.sh
cargo test -p ferroui-browser --lib
cargo test -p control-catalog-browser
cargo test -p ferroui-skia
cargo build -p ferroui-browser --examples
(cd src/Browser/FerroUI.Browser/webapp && npm run typecheck && npm run lint && npm run test:pixels)
scripts/build-browser.sh themed_view && node scripts/browser/tests/themed_view.test.mjs
scripts/build-browser.sh storage_view && node scripts/browser/tests/storage_view.test.mjs
scripts/build-browser.sh control-catalog-browser && node scripts/browser/tests/control_catalog.test.mjs
scripts/build-browser.sh themed_view --threads && node scripts/browser/tests/themed_view.test.mjs target/browser-threads/themed_view
scripts/build-browser.sh storage_view --threads && node scripts/browser/tests/storage_view.test.mjs target/browser-threads/storage_view
scripts/build-browser.sh control-catalog-browser --threads && node scripts/browser/tests/control_catalog.test.mjs target/browser-threads/control-catalog-browser
node scripts/browser/capture-catalog.mjs target/browser-threads/control-catalog-browser --out target/browser-threads/captures
node scripts/browser/capture-catalog.mjs target/browser-threads/control-catalog-browser --mode Software2D
scripts/build-browser.sh render_worker_clear --threads && node scripts/browser/tests/render_worker_clear.test.mjs
scripts/build-browser.sh thread_spawn --threads && node scripts/browser/tests/thread_spawn.test.mjs
```

Expected: 3 host tests of the browser crate more than before (220) and 2 more of the catalog host; `themed_view` 32 checks without threads and 72 against the threaded site; `storage_view` 18 and 19, unchanged; the catalog 13 checks without threads and 16 against the threaded site, with the memory printed as `measured:`; `capture-catalog.mjs` with twelve pages compared and one visited. To record here afterwards: the peak of the memory of the threaded catalog (and the fixed size chosen from it), the differing pixels per page, and the calls the ticks of the render thread had the main thread serve in the catalog.

By hand, served isolated (`node scripts/browser/serve.mjs target/browser-threads/control-catalog-browser --isolated`): the catalog, `?RenderThread=false`, `controlCatalog.catalogRendering()` and `controlCatalog.catalogMemory()` in the console.

### The interactions that failed on the render thread: cause and fix (2026-10-09)

Against the threaded site, 13 of the 16 checks of `control_catalog.test.mjs` passed. "The navigation drawer opens from its toggle button" failed every time; "the native controls of the Native Embed page" and the tour of the memory check timed out, most of the time, waiting for a page of the section "Window & Platform". All three pass with `?RenderThread=false`. Investigated against the built site without a rebuild: scratch scripts over `harness.mjs`, a second DevTools connection attached to the workers of the page, and `Atomics.waitAsync`, `requestAnimationFrame` and the script of the dispatcher wrapped from the page.

**One cause for the three: the check clicked where the state said an element was, before the view had drawn that state.** `catalogState` reads the elements as the thread of the page has laid them out. Input hits what the last frame drew: the hit test of a top-level answers from the readback the thread that renders writes with each frame (`CompositionTarget::try_hit_test_first`, `CompositionVisual::try_get_valid_readback`), which is upstream's design **[U]** (`CompositionTarget.TryHitTest`, `Readback.NextRead`). Between a change of the application and the frame with it the two differ. On one thread that window is one animation frame and the thread of the page is busy for most of it. On the render thread the thread of the page answers a test at once, and the frame comes when the browser gives the worker an animation frame, which headless Chrome with the software rasteriser does not do for hundreds of milliseconds after a frame that was expensive to present.

Nothing in the platform is wrong, and nothing in it was changed. The application does what a user would see: the view shows the old frame for that time, and a click lands on what is shown.

What was measured (headless Chrome, `--use-angle=swiftshader`, the site of this branch):

| Question | What was done | Result |
|---|---|---|
| Does the click make the view animate without end? | The frame count sampled without any input, 600 pixels wide | No. The banner of the Home page animates (the bounds of its four logos change from sample to sample): 434 frames in 8 s on the render thread, 485 on one thread, with or without a click |
| Does the press reach the toggle button? | Press and release at the centre of `PART_BackButton` (31, 24) while `frames=1`; the focus read afterwards | No: the focus stays `null`, the drawer stays closed. The DOM events arrive (`pointermove`, `pointerdown`, the capture, `pointerup`, `click`), but the platform does not give the container the focus, which it does 4 ms after the `pointerdown` of a click that works |
| Is input delivered and hit tested at all at that moment? | The wheel at (300, 500) while `frames=1` | Yes: the content scrolls (the first entry of the page moves from y 316 to 253) |
| What does the last frame show? | A click at (291, 24) while `frames=1`: where the toggle button is while the drawer is open beside the content (260 + 12 + 19) | The focus goes to the `Button`. Two seconds later the same click leaves the focus `null`. So the first frame shows the drawer open in the inline layout; the main view closes it for a narrow view in its `Loaded` handler (`MainView::update_adaptive_layout`), which runs after the first frame, and the frame with the closed drawer had not been drawn. (The press takes the focus; the release raises no `Click`, because in the layout of now the pointer is not over the button.) |
| Is it a matter of time? | The same click 2 s after the start; and move, press and release 600 ms apart | The drawer opens |
| Does a wake-up of the dispatcher get lost or arrive late? | `Atomics.waitAsync` (the mailbox of the main thread), `SingleThreadedDispatcherHelper.signal` and `OnSignaled` logged with times, 6 s | No: 887 signals, 887 `OnSignaled`; a notification of the worker is followed by the signal and by `OnSignaled` within 0.2 ms. During a gap the main thread has nothing to do: it waits for the worker |
| Is the render thread busy, or not ticking? | In the worker: `requestAnimationFrame` wrapped (start and duration of every callback) and a 4 ms interval that records when it was late by more than 30 ms | Neither. Example, 1440 x 900, times relative to the moment the page was ready: animation frames at -32.1 and -15.5 ms; the click on the section "Window & Platform" at 4.8 to 32.7 ms; the thread of the page works at 85.0 ms for 13.8 ms (it commits, the render loop is woken); **the next animation frame of the worker at 392.0 ms**; the interval never late in between; from 400 ms on one animation frame every 16.6 ms. At 600 pixels after the first frames: gaps of 178 and 126 ms, and up to 632 ms; a `requestAnimationFrame` loop of the page has the same gaps at the same times |
| Do the animations of the thread of the page advance? | `navigating` of the navigation page during the transition to the section page | Yes: the transition ends, after about 550 ms, while one or two frames were drawn. The clock is pulsed by `MediaContext::render_core` on the dispatcher of the page, as upstream's **[U]** (`MediaContext.RenderCore`, `_animationsTimer`); it does not depend on the render timer. Only the frames are missing: `frames` is 3 when the transition has ended on the render thread, 18 and 19 on one thread |
| The Native Embed check | Its steps at 1440 x 900: the section, then a click on "Native Embed" at (159, 646) as soon as the state shows the entry | Render thread: not reached, 3 of 3; with 1.5 s before the click: reached, 2 of 2. One thread: reached, 2 of 2. In the frame that is on the screen at the click the section is not expanded yet: there is no entry at that point |
| Is it the rasteriser? | The same two scenarios with `--use-angle=metal` (the GPU of the machine) | "Native Embed" is reached 3 of 3, with `frames` at 17, 15 and 30 at the click; the drawer opens when clicked at `frames=3`; no gap above 120 ms in 6 s |

The hypotheses the investigation started from: the animation clock of the thread of the page advances; no completion of a batch is lost and the throttling of commits does what upstream's does; the light dismiss of the drawer is not involved (the press never reached the button). The hit test from the last frame is the one that holds, and upstream has the same exposure: `BrowserRenderTimer` is `requestAnimationFrame` and nothing else **[U]**, `Button.OnPointerReleased` hit tests through the renderer **[U]**, and the readback is written by the render thread.

Why the software rasteriser withholds the animation frames of a worker was not established. What is known of Chrome, not verified here: a worker's animation frame is skipped while one of its canvases has frames the display has not acknowledged, and the first frames of a page are the ones for which the rasteriser compiles the shaders of Skia (the first two animation frame callbacks of the worker last 150 and 205 ms; later ones 1 ms). With a GPU the gaps are gone.

**The fix is in the tests**, with two exports of the host that make it exact:

| Piece | Where | What it is |
|---|---|---|
| `catalogRequestFrame()` | `samples/ControlCatalog.Browser/program.rs`, `catalog_request_frame` | Asks the compositor of the view to commit (`Compositor::request_commit_async`) and keeps the batch. `false` before the view has a compositor. Not a port |
| `catalogFrameDrawn()` | the same file, `catalog_frame_drawn` | Whether that batch has been rendered (`CompositionBatch::rendered`): the thread that renders has applied it, drawn the targets and written the readback (`ServerCompositor::render_core`, then `notify_batches_rendered`). `false` when no frame was asked for |
| `drawn(page)`, `page.drawn()` | `scripts/browser/catalog-pages.mjs` | Asks for a frame and polls every 20 ms until it is drawn |
| `page.element(description, match)`, `page.find(text, filter)` | the same file | Wait for the element in the state, then for a frame asked for after that, read the state again and return the element only if it is still at the same bounds; otherwise start over. `page.find` had returned the element of the first state that showed it |
| `waitUntilReady` | the same file | Ends with `drawn`: the first frame is not the last of the start (the adaptive layout of the main view follows it) |
| `drawerEntry` (the tour of the memory check and of `capture-catalog.mjs`) | the same file | Through `page.element` |
| The checks | `scripts/browser/tests/control_catalog.test.mjs` | The toggle button, the text box of the "First Look" sample and the check box of the Native Embed page are taken after the view has drawn them; every other click already went through `page.find` |

A frame asked for after a state was read contains every change the state shows: the commit is made by the next render pass of the media context, after its layout pass. That the element is at the same bounds afterwards rules out a change in between.

Validated without a rebuild, with the two exports emulated in a scratch copy of `catalog-pages.mjs` ("two more frames were drawn, or 1.5 s have passed"): the two failing checks passed in 4 runs of 4, and the whole test against the threaded site passed 15 of 16; the sixteenth, the brand assets, looks for the repository beside the test file and cannot pass from a copy. With the real exports the wait is exact and shorter.

To confirm after a rebuild: `node scripts/browser/tests/control_catalog.test.mjs target/browser-threads/control-catalog-browser` passes 16 of 16, repeatedly, and 13 of 13 against the site without threads; `controlCatalog.catalogRequestFrame()` answers `true` in the console and `controlCatalog.catalogFrameDrawn()` turns `true` within an animation frame on a machine with a GPU. In a trace like the one above the numbers of the application do **not** change, because nothing in it was changed: the worker still gets no animation frame for some hundred milliseconds after the first frames under the software rasteriser, and `frames` stands still meanwhile. What changes is that the check sends its click after `frames` has moved on.

What is doubted:

1. **The two exports were not compiled.** `catalog_request_frame` takes the compositor the way `catalog_panic_in_frame` does; `CompositionBatch` is imported from `ferroui_base::rendering::composition::transport`; the kept batch is an `Arc` in a `thread_local!`. The host test asserts both answer `false` outside a web page.
2. **A commit with nothing to commit.** `request_commit_async` on an idle view makes the media context commit an empty batch and the render loop tick once; that the batch is then told "rendered" was read (`NotifyRendered` runs at the end of every tick), not run. If it is not, `drawn` times out with the line of `catalogRendering` in its message.
3. **`hit` of `catalogState` is generous**: it is true when the hit element is an ancestor of the element asked about, which is how the toggle button counted as hit while the frame on the screen had the drawer over that point. With the wait the frame is the right one, so the answer means what it says; it was not made stricter, because a text block that is not hit-test visible is legitimately hit through its ancestor.
4. **The other scripts** that drive the catalog with their own waits (`frame-times.mjs`, `scroll-profile.mjs`) were not changed; they read a state and send input without this wait.
5. **The lag itself.** With the software rasteriser the view of a page that renders from a worker is hundreds of milliseconds behind the application after an expensive frame, where the same page on one thread keeps its animation frames. A tick out of turn when no animation frame has come for some time would shorten it (the timer has the mechanism since "B2.3"); it would also draw for a hidden page, which upstream's pacing and this one's do not, and it was not written. To be measured in a browser with a GPU and on a slow machine before anything is decided.

### Result of the validation of B2.7 (2026-10-09)

The step built on the first try. Three things were wrong at validation, and each is recorded where it was fixed:

- **The catalog did not start with threads.** The workers of the module load its script by the name the linker gave it, which for an application with a hyphen in its name is not the name of the script Cargo writes (`control_catalog_browser.js` against `control-catalog-browser.js`). Every worker failed to load, silently, and the module never finished starting. The build with threads now writes the script under the worker's name too (one line that imports the script).
- **Closing a second view** (the section "What failed in the first run of the second view, and why"): the release job never ran, the graph was dropped with no context current, and the media context waited for a batch of a compositor that was gone. Fixed in the base library and the browser backend.
- **Clicks sent before the frame they aim at was drawn** (the section on the drawer): not a defect of the platform. The checks now wait for a drawn frame.

Measured in headless Chrome:

| What | Result |
|---|---|
| `control_catalog.test.mjs` against the catalog built with threads | 16 of 16: the 13 checks of the catalog on the render thread, frames drawn by the render thread, `?RenderThread=false`, a panic of a frame reported to the page |
| Calls a tick of the render thread had the main thread serve | 0 |
| `themed_view.test.mjs` against the site built with threads | 72 of 72, among them a second view opened and closed while the first goes on drawing, in WebGL2 and Software2D, on the render thread and on one thread |
| `capture-catalog.mjs`: twelve pages of the catalog captured on the render thread and on one thread | 0 of 69490 sampled pixels differ on every page (Platform Information, Data Validation, OpenGL, Image, Container Queries, Border, TabControl, ListBox, TextBox, TextBlock, Slider, Buttons); the Composition page keeps drawing and is not compared |
| Memory of the module after the tour of 13 pages | 59.3 MB at the start, 127.4 MB at the end and at the peak, of 512 MB fixed |

The peak of this tour is a quarter of the fixed memory. Section 5 asks for the peak of the catalog to choose the size: 512 MB stays the default until every page was visited in one session (the tour visits 13); 256 MB would hold this tour twice over.

Open after this step: the Skia bindings shim without the atomics feature (its fix waits for the owner's approval of its downloads); the delay of a worker's animation frames under the software rasteriser of headless Chrome (up to about 600 ms after an expensive frame; not seen with a GPU rasteriser), which B3 measures on real hardware; browsers other than Chrome.

## B2.8: the site and CI (written, 2026-10-09)

Status: written on 2026-10-09, **no module built**. No cargo, no browser build and no `setup.sh`. What was run, with what was already on the machine: the scripts parse (`node --check`, `bash -n`), the workflow files parse as YAML, and, in headless Chrome, **the composition and the loader against modules built earlier**: `combine-site.mjs` composed a site with both modules from the `themed_view` sites that the validations of the earlier steps left in the main checkout (`target/browser` and `target/browser-threads`, copied, with the host page and the loader of this branch put in), and `site_loader.test.mjs` passes its 10 checks against it, three runs. So the layout, the one-line re-export, the choice, the service worker route, the fallbacks and the request assertions have been seen to work with a real module of each kind. The catalog as a site with both modules, `--both` of the build script, and everything in the workflows have not. No Rust source changed. Everything here is **[M]** unless marked; **[G]** is read in the script of a module built with threads.

### What was written

| Piece | Where | What it is |
|---|---|---|
| The loader | `scripts/browser/threads/ferroui-loader.js` | `loadModule({ script, element })`: imports the script of the module for a host page, and on a site with both modules decides which one first. No imports, nothing of the platform. Below, "The decision" |
| The composition | `scripts/browser/combine-site.mjs` | Makes one site of a site built without threads and one built with them. Below, "The site with both modules" |
| The build | `scripts/build-browser.sh` | `--both`: the build without the option, the build with `--threads`, then the composition, into `target/browser-both/<application>` (or `--out`); `--debug` applies to both builds. Every site gets `ferroui-loader.js` next to its host page. Nothing of the threaded flags changed |
| The host pages | `samples/ControlCatalog.Browser/wwwroot/main.js` and `index.html`; `examples/themed_view/wwwroot/main.js`, `examples/storage_view/wwwroot/main.js` | They import the script of the module through the loader, not by a static import. The page of the catalog preloads the loader with its other scripts. `thread_spawn` and `render_worker_clear` exist with threads only and keep their pages |
| The test | `scripts/browser/tests/site_loader.test.mjs` | Below, "The test" |
| The harness | `scripts/browser/harness.mjs` | `open(.., { network: true })` keeps the addresses the page requests, over every load, in `page.requests`. The removal of the browser's profile directory is tried again when the browser is still writing to it (the failure "Directory not empty" that the validation of "B2.6" met once: here 4 checks of 20 failed in it before the change, on a machine busy with a build, and none of 20 after) |
| CI | `.github/workflows/ci.yml`, `.github/actions/browser-toolchain/action.yml` | Below, "CI" |
| Pages | `.github/workflows/pages.yml` | Builds `--both`, tests the three sites, publishes the composed one; the size budget is for each module |

### The site with both modules

```
index.html main.js app.css ...          the host page: the files of the site without threads
ferroui.js storage.js ferroui-sw.js     the script modules of the platform and the service worker, once (with their maps)
ferroui-loader.js                       the loader
assets/...                              the files the build scripts of the application wrote, once
<application>.js  <name>.wasm           the module without threads, where a site with one module has it
threads/<application>.js                the module with threads, under the same two names
threads/<name>.wasm
threads/ferroui.js                      one line: export * from "../ferroui.js";
```

**Why a directory and not other file names.** The script of a module names its WebAssembly file and, with threads, itself (each web worker of a thread is `new Worker(new URL("<application>.js", import.meta.url), ..)`) relative to its own address **[G]**. Other file names would mean rewriting generated script; a directory needs no change to it. What a directory does break is the one import the script makes: `./ferroui.js`, written by the wasm-bindgen glue and by `ferroui-worker-import.js`, would be a second copy of the script module of the platform, while the host page attaches the module to its own copy (`FerroExports.attach`). `threads/ferroui.js` re-exports the one of the site, so the page, the module and the module script of each worker resolve to one address and, per thread, one instance. `ferroui.js` has no default export, and it finds `storage.js` relative to its own address, which has not moved. The composition refuses a module script that imports anything else relative to itself.

**The service worker** stays at the root, so its scope covers `threads/`; the application in a threaded module registers `./ferroui-sw.js?coi=1` relative to the document, the address the loader registers.

**Shared files are verified, not assumed.** Every file both sites have, other than the script of the module and the WebAssembly file, is compared byte for byte; a file only one of them has is an error, except `ferroui-threads.js` of the site with threads, which is left out (the loader isolates the page itself and never shows the message of that check). When something differs the composition fails with the list and writes nothing. This is what "not duplicated where they are identical" rests on: the bundles are built twice by the two builds, with the same esbuild from the same lock file, and the asset files and their list are written twice by the build script of the catalog. Tried: two `themed_view` sites built from different commits were refused for `ferroui.js` and its map.

**The host page** of the composed site differs from the one of the sources in two places: the elements that preload the script of the module and the WebAssembly file are removed, and `<meta name="ferroui-modules" content='{"threads":"./threads/","wasm":"<name>.wasm"}'>` is added before `</head>`. The element is how the loader knows, without a request, that the site has both.

**Size.** The composed site is the site without threads, plus the two files of the module with threads, plus about 200 bytes. Composed from the `themed_view` sites at hand: shared files 0.33 MB (ten files, maps included), the module without threads 30.59 MB (`themed_view.js` 0.18 MB, `themed_view.wasm` 30.41 MB), the module with threads 30.42 MB (0.20 MB and 30.22 MB): 61.35 MB, raw. For the catalog the same sum applies: its site without threads (the module, 39.11 MB raw in the last build in the main checkout, and about 23 MB of asset files) plus one more module of about the same size. The threaded catalog module has not been built on this branch, so its number is for the validation to fill in (`combine-site.mjs` prints the three parts, `module-sizes.mjs` the table). A visitor downloads one of the two modules, never both; the size budget of the Pages workflow is therefore applied to each module, not to their sum.

### The preloads, and the preload test

Preloading a module that is not used downloads it for nothing, so on a site with both the page preloads neither, and the loader adds the preload of the WebAssembly file of the module it chose (`<link rel="preload" as="fetch" type="application/wasm" crossorigin>`, the attributes of the element of the catalog page) right before it imports the script of that module: the two downloads run side by side, and the script's own fetch of the WebAssembly file is answered by the preload.

- **A site with one module is unchanged in what it downloads and when**, but for one small file: its page keeps its preload elements, and the loader finds no `ferroui-modules` element, makes no decision, asks no worker and imports the script the page named, which the page preloaded. The first frame of the build without threads therefore waits for nothing new. `main.js` no longer has the script of the module among its static imports, so it runs when its small imports are there, and the dynamic import that follows finds the module script already fetched. Not measured: B3.
- **The existing preload check stays what it was** for the sites it runs against (`control_catalog.test.mjs` against `target/browser/..` and `target/browser-threads/..`): the WebAssembly file, the list and the start-up files are each downloaded once, by the elements of the page.
- **For the site with both**, `site_loader.test.mjs` asserts the same thing of the chosen module in every check: its WebAssembly file is in the resource timing of the page exactly once, with the initiator `link`, and no file of the other module was requested in any load of the page.
- **What the site with both pays**: the download of the module starts when `main.js` runs, not with the page, and on the first visit of a session after the answer of the probe worker. Section 19 of `browser-platform.md` measured 180 ms for the preloads at 50 Mbit/s; some of that is given back on the published site. If B3 finds it matters, the decision can move into a script in the head of the page; the session cache already makes later loads decide without a worker.

### The decision

`choose` of the loader, for a site with both modules. First match wins; nothing of either module has been requested when it runs.

| # | Condition | Module | Service worker registered | Reload |
|---|---|---|---|---|
| 1 | `?Threads=false` | without threads | no | no |
| 2 | The module with threads could not be created earlier in this session | without threads | no | no |
| 3 | No `WebAssembly`, no `Worker`, no `Atomics`, no `HTMLCanvasElement.prototype.transferControlToOffscreen`, or no shared memory | without threads | no | no |
| 4 | Not isolated, and the page could not be isolated earlier in this session | without threads | no | no |
| 5 | The probe worker says a worker has no `requestAnimationFrame` or no `OffscreenCanvas`, cannot be a script module, fails, or does not answer in 2 s (the answer is kept for the session) | without threads | no | no |
| 6 | `crossOriginIsolated` | **with threads** | no | no |
| 7 | Not a secure context, or no `navigator.serviceWorker` | without threads | no | no |
| 8 | The page was reloaded for the service worker and is still not isolated | without threads; recorded for the session (row 4 from then on) | it is there | no |
| 9 | The service worker cannot be registered, or is not active within 10 s | without threads; recorded | maybe | no |
| 10 | Session storage cannot count the reload | without threads | yes | no |
| 11 | Otherwise | none in this load: after the reload, row 6 (or row 8) | yes, `./ferroui-sw.js?coi=1` | **once** |

`?Threads=true` skips rows 2 to 5 and enters at row 6: it forces the module with threads past what the browser was asked, but not past isolation, without which the module cannot be created at all.

- **Why this order.** Everything that can say "no" without side effects comes before the isolation, so a browser that cannot run the module with threads never gets the service worker with the headers and is never reloaded.
- **Shared memory before isolation.** `SharedArrayBuffer` is not a global in a page that is not isolated yet, which is the page that asks. The loader accepts the global, or else a `WebAssembly.Memory` made with `shared: true` whose buffer is a shared one **[D]**, which a browser with shared memory makes in any page.
- **The probe** is a worker made from a blob of one line, created as a script module because a thread of the module is one (a browser without such workers never reads the `type` option, which is how that is seen). It answers by `typeof`: a hidden page has no animation frames to wait for. One worker per session; the answer, a timeout included, is in session storage (`ferroui-loader-worker-frames`).
- **No reload loop.** The reload happens only after the flag `ferroui-loader-reloaded` was written to session storage, and a load that finds the flag and is not isolated removes it, records `ferroui-loader-not-isolated` and loads the module without threads; from then on nothing is tried in that session. This is the guard of `ferroui-threads.js` with the message replaced by the other module, and with keys of its own, so that the two cannot read each other's flag. A load that is isolated clears both.
- **Silent.** No path of a site with both shows the message of `ferroui-threads.js`. The loader logs nothing on a fallback; what it decided and why is `globalThis.ferrouiModule` (`threads`, `reason`, `site`).
- **A last net.** When the module with threads was chosen and its factory rejects (the fixed memory of "B2.2", 512 MB by default, is refused, which phones are known to do), the loader warns on the console, records it for the session (row 2) and creates the module without threads in the same page. This is the only case in which one page load requests both, and it has not been exercised: nothing here can make the factory fail.
- **`?RenderThread=false`** is not the loader's: the application reads it and keeps a module built with threads on the thread of the page. `?Threads=false` loads the other module.

**A site with one module**: built without threads, the script is imported and that is all; built with threads alone, the loader does what the pages did since "B2.2" (`ensureCrossOriginIsolated` of `ferroui-threads.js`, with the message, and `null` to the page when it cannot be isolated), because there is nothing to fall back to. `thread_spawn.test.mjs` and the tests of the threaded sites assert that behaviour and are unchanged.

### The test

`node scripts/browser/tests/site_loader.test.mjs [<site with both modules>]` (default `target/browser-both/control-catalog-browser`; it also knows a `themed_view` site). Every check opens the site with the requests of the page recorded and asserts: which module `ferrouiModule` says was chosen; the script and the WebAssembly file of that module were requested and no file of the other one, in any load; the WebAssembly file was downloaded once, by the preload; where the frames are drawn (`catalogRendering` or `themedViewRendering`: by a render thread other than the thread of the page, or by the thread of the page with no render thread); no message element; no error.

1. The layout of the site (no browser).
2. Served with the headers: the module with threads, one load, no service worker.
3. Served without: one reload, `ferroui-sw.js?coi=1` controls the page, the module with threads; a second visit is isolated at once (one more load, not two) and has one registration.
4. `?Threads=false` with the headers: the module without threads.
5. `?Threads=false` without: no service worker, no reload.
6. `transferControlToOffscreen` removed before the page runs: the module without threads, no service worker, no reload, and the reason says so.
7. The probe worker cannot be created (served with the headers): the module without threads, the answer `0` kept.
8. `?Threads=true` with the same failing probe: the module with threads, and the probe did not run.
9. `navigator.serviceWorker` removed, no headers: the module without threads, no message.
10. `crossOriginIsolated` made to answer false whatever the headers: two loads and not more, the module without threads, the failure recorded and the reload flag gone; a further load decides from the record.

### CI

Two jobs more, beside the existing one, and nothing compiled twice:

| Job | What it does | New |
|---|---|---|
| `browser` (its steps unchanged) | Builds and tests the three sites without threads | Uploads `target/browser/themed_view` and `target/browser/control-catalog-browser` as an artifact kept for a day |
| `browser-threads` | `setup.sh --threads` through the composite action (`threads: true`); builds with `--threads` and tests `thread_spawn`, `render_worker_clear`, `themed_view`, `storage_view` and the catalog, each test against its site in `target/browser-threads`; uploads the two sites the next job needs | The whole job |
| `browser-site` (needs both) | Node only. Downloads the four sites, composes `themed_view` and the catalog (which is also the check that the two jobs produced identical shared files), runs `site_loader.test.mjs` against both | The whole job |

**Why a job beside and not steps after.** The build with threads compiles everything again: another toolchain, a standard library built from source, its own target directory. As steps of the `browser` job it would about double that job, which a pull request waits for; in a job of its own it runs at the same time. What that costs in machine time is one more installation of the toolchain from caches, node, and the artifacts (four sites, about 130 MB before compression): a few minutes. The composition job compiles nothing. Doing all of it in one job would save those minutes and add the whole threaded build to the wait. No duration of the existing `browser` job is written down in the documents (section 18 of `browser-platform.md` has 11 to 18 minutes for one full build of `themed_view` and 15 for the catalog in a 4-core container, without a cache); the expectation for `browser-threads` is the duration of `browser` plus the standard library and two small examples, and for `browser-site` about five minutes, most of it the catalog starting a dozen times in headless Chrome.

**Caches.** The composite action reads the nightly pin from `setup.sh` like the other pins. With `threads: true` it restores `~/.rustup/toolchains/<pin>-*` and `~/.rustup/update-hashes/<pin>-*` under a key made of the pin, calls `setup.sh --threads` as the one entry (whatever else that script installs for the mode is not cached and not known to the workflow), and gives the Rust build cache a key of its own with the pin in it, since `target/threads` holds a standard library built by that nightly.

**Docs-only changes** still start nothing: the `paths-ignore` of the workflow covers every job.

**Required checks.** The names of the two new checks are "Check (browser with threads, wasm32-unknown-emscripten)" and "Check (browser, the site with both modules)". `CONTINUATION.md` lists three checks as the condition for a merge; whether the two new ones join them is the owner's.

### Pages: what the workflow does, and the list for a check by hand

`pages.yml` installs the toolchain with `threads: true`, runs `scripts/build-browser.sh control-catalog-browser --both`, tests the module without threads and the module with threads with `control_catalog.test.mjs` against their sites and the composed site with `site_loader.test.mjs`, and uploads `target/browser-both/control-catalog-browser`. The size budget (`PUBLISHED_MODULE_GZIP_BUDGET_MB`, 14 MB with gzip) is checked for each of the two modules. The build job runs both builds one after the other; its time limit went from 120 to 240 minutes.

CI serves the sites from `127.0.0.1`; what the real host does can only be seen there. After the first deployment, in a browser with the developer tools open (Chrome, then Firefox and Safari), at the address of the site:

1. **First visit** (a private window, or after "Clear site data"): the page loads, reloads itself once, and shows the catalog. In the console `crossOriginIsolated` is `true`, `ferrouiModule` is `{ threads: true, site: "both", reason: .. }`, and the Network panel of the second load has `threads/control_catalog_browser.wasm` and no `control_catalog_browser.wasm` of the root.
2. **Frames come from the worker**: `controlCatalog.catalogRendering()` has `on_render_thread=true`, `other_thread=true`, `frames` rising while the pointer moves over the view, `panics=0`; the developer tools list the `em-pthread` workers.
3. **A second visit** (a reload, and a new tab in the same window): the page does not reload itself and is isolated at the first response; Application, Service workers shows one registration, `ferroui-sw.js?coi=1`, activated.
4. **`?Threads=false`**: the catalog works, `ferrouiModule.threads` is `false`, `catalogRendering()` has `on_render_thread=false`, and the Network panel has the module of the root and nothing of `threads/`.
5. **`?RenderThread=false`** (without `Threads`): the module with threads, `on_render_thread=false`.
6. **The storage features of the service worker still work**: with `?PreferFileDialogPolyfill=true`, a file saved from the catalog arrives as a download, and one can be opened; without the parameter the native pickers open and save. The catalog does not register the service worker itself; on a visit that took the module with threads the worker is there because the loader registered it, and it is the same worker, with the part that streams the saves.
7. **The fallbacks**: a private window of a browser that has no service workers there shows the catalog with `ferrouiModule.threads` false and no message; a forced reload (which bypasses the worker) gives a page that reloads once more by itself and is isolated again.
8. **The headers**: `curl -I` of the site shows no `Cross-Origin-Opener-Policy`; the responses in the Network panel of an isolated visit show the three headers and the service worker as their source.
9. **Phones**: the catalog starts on a phone. If the fixed memory of the module with threads is refused there, the console has the warning of the loader and the catalog still starts (the last net above); this is the case nothing could exercise.

### What could not be verified without a build

- `scripts/build-browser.sh <application> --both` itself: the script calling itself twice and the composition after it.
- The catalog as a site with both modules: its page with the two preload elements removed (tried on stand-in files only), the list and the start-up files fetched before the choice, and, on the service worker route, their requests cut short by the reload.
- That the two builds write identical shared files for the catalog (its asset files and their list are written by a build script, once per build and by two compilers), in one job and across two.
- The host pages against their own tests with a current module. What ran: `themed_view.test.mjs` against a copy of the site without threads of the main checkout with the new page and the loader passes 30 of its 32 checks, and the two that fail call `themedViewSecondView`, an export of "B2.7" that this older module does not have; against a copy of the site with threads, served isolated, 68 of 72 pass, and the four that fail are the checks of the second view of "B2.7" (`released` never reaches 1; one run logs `Cannot read properties of undefined (reading 'finish')` from `_emscripten_glFinish`), in a module of that step that was not validated yet when it was copied: they are about the disposal of a view, not about how the module is loaded, and are for the validation of "B2.7" to look at. The pages of `storage_view` and of the catalog did not run.
- Everything in the workflows: the input of the composite action, the cache of the nightly, the artifacts and their paths, the two new jobs, the Pages build.
- The size of the threaded catalog module against the budget.
- Any browser but headless Chrome; the real host.

### Doubts, most likely to bite first

1. **The size budget of Pages.** It is now applied to the module with threads too. If that module is over 14 MB with gzip the deployment stops, where before nothing measured it. The `themed_view` modules of the two kinds differ by under 1 % raw; the catalog's was never measured.
2. **Identical shared files across two jobs.** The composition in CI compares files built on two runners by two toolchains. If the list of the asset files or a bundle is not reproducible, `browser-site` fails naming the file, and the answer is either to make it reproducible or to build both in one job (`--both`, as the Pages workflow does).
3. **Requests cut short by the reload.** The catalog page starts fetching its asset list and start-up files before it asks the loader. A reload aborts them; whether Chrome logs an aborted fetch as an error of the page, which check 3 of the loader test would then report for the catalog, was not seen (`themed_view` fetches nothing before the choice). If it does, the page should ask the loader first.
4. **The cache of the nightly.** Restoring a directory of `~/.rustup/toolchains` and its update hash was not tried; if rustup does not accept the restored toolchain it installs it again (slower, not wrong).
5. **The Rust build cache with a nested target directory.** `target/threads` is a cargo target directory inside the one the cache action looks after; that it is kept and cleaned as one is from memory of that action, not seen.
6. **The start of the download on the site with both** (above, "The preloads"): later than on a site with one module, by the scripts of the page and, once a session, the probe. On the service worker route the first visit pays the page and its scripts twice, and the start-up files of the catalog up to twice.
7. **`WebAssembly.Memory` with `shared: true` in a page that is not isolated** **[D]**: Chrome makes it (the checks ran there); Firefox and Safari were not tried. If one of them refuses, row 3 says no and that browser gets the module without threads although it could run the other: the safe direction.
8. **The probe in other browsers.** A worker from a blob under a content security policy that forbids `blob:` fails, which reads as "no". The getter on the `type` option as the test for workers that are script modules is the usual one **[D]**. Two seconds may be short on a loaded phone; the answer is then "no" for the session.
9. **Two addresses of one service worker.** The module without threads registers `./ferroui-sw.js`, the one with threads and the loader `./ferroui-sw.js?coi=1`. An application that sets `register_ferro_service_worker` and is opened once with each module (`?Threads=false`, or a fallback, in a browser that had the other) replaces the registration each time, and the next visit with threads reloads once again. The catalog does not register the worker itself. Where the page is controlled by the worker with the headers and runs the module without threads (rows 2 and 8), `require-corp` applies to that page all the same: content of another origin without CORS is refused (doubt 6 of "B2.7").
10. **An embedding that cannot be isolated** (a frame in a page that is not): the page registers the worker, reloads once, finds itself not isolated, and falls back; that is one reload per session for nothing, since the record is the session's. A longer-lived record would spare it and would also keep a browser from ever trying again.
11. **The last net** (a module with threads that cannot be created): written, never taken. That the factory rejects rather than failing later, that the workers of the pool it leaves behind do no harm, and that the second module can be created in the same page are read, not seen.
12. **`ferroui-loader.js` lives in `scripts/browser/threads/`** although every site gets it, the sites of `thread_spawn` and `render_worker_clear` included, which do not use it.
13. **`module-sizes.mjs` on the composed site** sums the two modules in its `wasm_*` outputs; the workflows take those outputs from the two sites with one module each.

### Validation

```
scripts/browser/setup.sh --threads && source .tools/env.sh
node --check scripts/browser/combine-site.mjs && node --check scripts/browser/tests/site_loader.test.mjs && bash -n scripts/build-browser.sh
scripts/build-browser.sh themed_view --both
node scripts/browser/tests/themed_view.test.mjs
node scripts/browser/tests/themed_view.test.mjs target/browser-threads/themed_view
node scripts/browser/tests/site_loader.test.mjs target/browser-both/themed_view
scripts/build-browser.sh storage_view && node scripts/browser/tests/storage_view.test.mjs
scripts/build-browser.sh storage_view --threads && node scripts/browser/tests/storage_view.test.mjs target/browser-threads/storage_view
scripts/build-browser.sh thread_spawn --threads && node scripts/browser/tests/thread_spawn.test.mjs
scripts/build-browser.sh render_worker_clear --threads && node scripts/browser/tests/render_worker_clear.test.mjs
scripts/build-browser.sh control-catalog-browser --both
node scripts/browser/tests/control_catalog.test.mjs
node scripts/browser/tests/control_catalog.test.mjs target/browser-threads/control-catalog-browser
node scripts/browser/tests/site_loader.test.mjs
node scripts/browser/module-sizes.mjs target/browser-threads/control-catalog-browser
node scripts/browser/module-sizes.mjs target/browser-both/control-catalog-browser
```

Expected: the tests of the sites with one module unchanged in number and result from "B2.7" (`themed_view` 32 and 72, `storage_view` 18 and 19, the catalog 13 and 16, `thread_spawn` 3, `render_worker_clear` 6); `site_loader.test.mjs` 10 checks against each composed site; the composition printing the three parts of each site. To record here afterwards: the sizes of the catalog site with both modules, the gzip size of the threaded catalog module against the budget, and the first run of the three jobs with their durations.

By hand, the composed catalog as a static host serves it and as a host with the headers: `node scripts/browser/serve.mjs target/browser-both/control-catalog-browser` and the same with `--isolated`; `ferrouiModule` and `controlCatalog.catalogRendering()` in the console; `?Threads=false`; then the list above on the published site.

### Result of the validation of B2.8 (2026-10-09)

Both combined sites were built with `scripts/build-browser.sh <application> --both` and tested in headless Chrome.

- `site_loader.test.mjs`: 10 of 10 on the combined `themed_view` site (twice) and on the combined catalog site. (A first run of the `themed_view` site while the machine ran a benchmark timed out in the two checks of the service worker route; not seen again.)
- The single-module sites with the new host pages: the catalog without threads 13 of 13, `themed_view` without threads 32 of 32.
- The combiner had to learn two things the catalog has and `themed_view` does not: the script the workers of the module load under another name (B2.7) belongs to the module with threads and moves with it, and the script of the module imports scripts of the application's host besides the platform's (`embed.js`, `page-assets.js`), each of which is re-exported from the directory of the module with threads so that the page and the module share one instance.
- Sizes, raw: `themed_view` 0.33 MB shared, 30.63 MB without threads, 30.42 MB with, 61.39 MB in all; the catalog 24.39 MB shared (assets), 39.57 MB without threads, 39.35 MB with, 103.30 MB in all. A visitor downloads the shared files and one module.

Not verified here: the workflows (the new jobs run for the first time in the pull request of this step) and the published site on GitHub Pages (the list for a check by hand is above).

### The reload that brought back the first document (found and fixed, 2026-10-09)

The two checks of `site_loader.test.mjs` that go through the reload of the service worker route failed in CI and under load here with "timed out waiting for `themedView` and a frame". It read as an application that starts and never draws. It was not: the application drew, and then the page was replaced by a document that has no application.

**Cause.** The first document of a visit without the headers registers the service worker and calls `location.reload()`; its promise never resolves, by design, because the document is going away. It does not go away. The page that replaces it carries `Cross-Origin-Opener-Policy: same-origin` and the first one does not, so the browser moves the tab to another group of browsing contexts, and a document left behind by such a move is one the back/forward cache may keep: Chrome freezes it and keeps it (`pagehide` with `persisted` true). A later reload of the page can then be answered with that kept document instead of a load: Chrome reports the navigation as `historyDifferentDocument` and the commit as `BackForwardCacheRestore`, with the loader id of the first document. What comes back is the page as the loader left it: the splash, no canvas, no module, not isolated, and a loader that waits for ever on a reload that already happened. Both checks reload the page for their second half ("a second visit does not reload", "nothing is tried again in the session"), and that reload is where they failed; the frames they waited for were asked of a document that never had a module. The warnings in the log of the failures (from `threads/themed_view.js` and from `themed_view.js`) were those of the first half, which had passed. Whether a reload is answered that way depends on timing inside the browser, which is why the load of the machine decides; when it is not, the same reload is an ordinary one (`navigationType` `reload`) and the kept document is never seen. What exactly makes Chrome choose was not found; the entry of the session history has a new id after such a restore (13 before, 16 after), which an ordinary reload does not change.

**Evidence** (Chrome 154 headless, the combined `themed_view` site of the validation, a script that does what the two checks do and reads the page when the wait ends; 14 busy loops beside it on a machine with 11 cores):

- Failures, all in the second half, after `Page.reload`: of the check of the service worker route 2 of 6, 3 of 8, 4 of 6 and 6 of 6 runs; of the check of the page that is never isolated 0 of 6 and 2 of 6. Without the busy loops (other work kept the load average between 14 and 22): 1 of 12. CPU throttling of the page through the DevTools protocol (6 and 20 times, 18 runs) never failed: the slowness that matters is not that of the page.
- The state of a failed page: `crossOriginIsolated` false although the page is controlled by `ferroui-sw.js?coi=1`; `ferrouiModule` and the global of the application undefined; no canvas, the splash still in the host element; `ferroui-loader-reloaded` set in the session storage; `performance.timeOrigin` that of the first document and `performance.now()` counting from the start of the run; the navigation entry of type `navigate`, not `reload`.
- Its own record of events: `pageshow`, then at the loader's reload `pagehide persisted=true`, `visibilitychange` to hidden and `freeze`; then, at the moment of the test's reload seconds later, `resume`, `visibilitychange` to visible and `pageshow persisted=true`.
- The requests of the page: none for the third navigation (no `index.html`, no script), where a passing run has the whole list again.
- The page was visible and its animation frames ran throughout (hundreds to thousands counted by an init script), the host element was 800 by 600, the service worker active and answering: the other candidates (frames that do not fire after the reload, a view without a size, a module started twice, a late answer of the worker) are ruled out by the same reading.
- Tried against the same load, 8 runs each, with the page reloading itself when shown from the cache: the loader as it was fails as above; with that alone every run passed and every run had four loads (the restore happened 8 of 8 times and was repaired); with a held lock, with an `unload` listener, or with `location.replace` in place of the reload, every run passed with three loads (the document was not kept).

**Fix**, in `ferroui-loader.js` and the same in `ferroui-threads.js`, which reloads the same way for a site with the module with threads alone (`reload()` in both):

1. Before the reload the document takes a lock of the Web Locks API and never releases it. A document that holds a lock is not kept by the back/forward cache, so there is nothing to restore. The lock is shared, so two pages of the site that reload at the same time do not wait for each other; it ends with the document.
2. A listener of `pageshow` reloads the page when the document is shown again from the cache (`persisted`). This is for a browser that keeps the document in spite of the lock, or has no locks: the reload goes through the service worker this time. It costs one more load in that case and cannot loop: the reloads are still counted in the session storage by the documents that are loaded, and this one is not loaded again.

`location.replace` was not chosen although it also worked: with a fragment in the address it is a navigation inside the document and reloads nothing. The `unload` listener is being removed from browsers. The service worker and the platform are unchanged; no Rust source changed. The test is unchanged in what it asserts: the application was right and so was the check, three loads included.

**After the fix** (the scripts swapped into copies of the built sites, the same 14 busy loops): the two checks 24 of 24 (12 each, three loads every time), against 8 failures in 12 for the unchanged site in the minutes after; `site_loader.test.mjs` 10 of 10 on the combined `themed_view` site, quiet and under that load, and 10 of 10 on the combined catalog; `thread_spawn.test.mjs` 3 of 3 and `render_worker_clear.test.mjs` 6 of 6 with the new `ferroui-threads.js`.

**Open.** (1) This was seen through `Page.reload` of the DevTools protocol in headless Chrome. Whether the reload button of a window does the same was not tried; the fix does not depend on the answer. (2) That a held lock keeps a document out of the cache is today's behaviour of Chrome, not a promise; the listener is what holds if it changes, at the price of the fourth load, which the two checks would then report (`loads`). Firefox and Safari were not run. (3) Why the catalog site did not fail in CI: not examined. Nothing in the cause is particular to `themed_view`; the unchanged catalog site was not run under load here. (4) One run at a load average above 70 ended without a frame in the first half, in the module with threads, within a wait of 15 s that the script used then (canvas at its default size, no render target yet): taken for slowness, not seen in the 24 runs with a wait of 30 s, and not examined further.

## B3: the measurements (2026-10-09)

Stage B3 of `render-thread.md`: rendering from the worker against rendering on one thread, measured. The tables, the method, the machine and its load are in `browser-platform.md`, section 22; this section is what they mean. Everything was measured in headless Chrome 154 on one machine (Apple M3 Pro), with the software rasteriser the tests use and with the GPU of the machine (`--use-angle=metal`), five alternating runs each, against the sites this branch builds: the module without threads, the module with threads on its render thread, the same module with `?RenderThread=false`, and the site with both modules. No source of the platform changed; the measurement scripts were extended to handle a site built with threads (`first-frame.mjs`, `scroll-profile.mjs`, `frame-times.mjs`, the harness) and two were added (`catalog-memory.mjs`, and `measure-render-thread.mjs`, which runs the series and prints the tables).

### What the render thread buys

| What | Without the render thread | With it | |
|---|---:|---:|---|
| Thread of the page per wheel event, scrolling the TableView page (processor time) | 10.3 ms (GPU), 9.3 ms (software rasteriser) | 7.1 ms, 7.0 ms | 31 % and 25 % less; the worker spends what left |
| Thread of the page in tasks while the Home page animates in Software2D | 82 % of the time, a task of 13.6 ms every frame | 2.5 %, no task over 8 ms | at the same 60 frames a second |
| The same in WebGL2 with the GPU | 10.0 % | 3.8 % | a frame costs under 1 ms here |
| Long tasks of the thread of the page before the first frame of the catalog | 0.62 s (GPU), 3.1 s (software rasteriser) | 0.48 s, 0.52 s | the longest 2.5 s against 0.4 s under the software rasteriser |
| Animation frames lost by the view during page transitions (gaps over 33 ms in 10 s, GPU) | 4 | 0 | the transition still blocks the page |

1. **The thread of the page loses the rendering, which is a quarter to a third of its work per input event in the scrolling scenario, and nearly all of it where Skia rasterises on the processor.** That is the purpose of the stage, and the size of it is what `performance/README.md` predicted from the profile (compositor render and Skia, 25 %).
2. **The view keeps drawing while the page is busy.** During a page transition the thread of the page runs tasks of 55 to 74 ms; a view the page draws misses frames then, the render thread does not (with the GPU).
3. **Where the first frame makes the thread that renders wait, it is the worker that waits.** Under the software rasteriser the first frame holds its thread for about 2 s; with the render thread the page can run script during them.

### What it costs

1. **First frame: 20 to 25 ms with a GPU** (the catalog 708 ms against 684 ms, `themed_view` 332 against 311 ms), of which 10 to 22 ms are the threaded build itself (the pool of two workers), the rest the hop to the worker. Under the software rasteriser the first frame takes 2.5 to 3.3 s either way and the difference is inside the spread. Risk 10 of section 7 is answered: the pool costs 10 to 22 ms.
2. **The site with both modules: 11 ms more** when the host sends the headers (the loader requests the module; the page cannot preload it), **and 37 to 49 ms more for the service worker route with its reload**, on a local server: 756 ms against 684 ms for the catalog from the first navigation to the first frame, on the first visit of a session. A network makes the reload dearer; that was not measured.
3. **Size: nothing.** The module with threads is 0.4 % smaller with gzip (11.67 against 11.72 MB for the catalog); its script is 4.8 kB larger with gzip. The published site carries one more module (11.7 MB with gzip) that a visitor does not download.
4. **Memory: 24.5 MB at the start** (59.5 against 35.0 MB for the same module on one thread), **2 MB at the end of the tour** (128 against 126 MB), and the fixed size: 512 MB reserved where the module without threads had grown to 131 MB.
5. **Total processor time: none measurable.** Scrolling uses 618 ms on two threads against 619 ms on one with the GPU.
6. **Latency: none measurable.** From a wheel event to the drawn frame the median is two frame intervals (33.5 ms) and the longest three (50 ms) in every configuration with both rasterisers. Risk 11 of section 7 does not show. The render thread draws fewer frames for the same input (92 against 120 for 60 wheel events): commits between two of its animation frames become one frame.
7. **The requirement itself**: a cross-origin isolated page, with what section 21 of `browser-platform.md` lists (content of other origins refused, one reload on the first visit of a static host, no isolation without service workers).

### Where it is worse, and why

- **The splash screen closes long before the first frame under the software rasteriser**: at 0.84 s, with the first frame at 2.9 s, in the catalog; on one thread it closes 0.46 s before the first frame. The splash is closed by the first animation frame of the top-level on the thread of the page (`FerroView::new`, as upstream), which with a render thread is not the frame that draws. For those two seconds the page shows an empty canvas, and input hits nothing (a view is hit from what its last frame drew: "B2.6"). With the GPU the gap is 0.1 s. The catalog's host also starts prefetching the files of its pages at that moment (`page-assets.js` waits for the splash to close), so on the render thread the prefetch runs beside the start.
- **Animation frames withheld by the software rasteriser** (the lag "B2.7" found). In the first 6 s of the catalog the worker waits 1.3 s in all for animation frames it asked for, 0.48 s at the longest. **But so does a page that draws on one thread**: 0.8 to 1.1 s in all, 0.40 to 0.57 s at the longest, with overlapping ranges. The delay belongs to the rasteriser, not to the worker; "B2.7" met it on the worker because there the thread of the page is free to act on a state the view has not drawn, where on one thread the thread that would act is the one that waits. With the GPU the worker waits 14 ms in all, less than a page on one thread (about 100 ms). After the start, while the catalog draws steadily or runs transitions, no gap over 250 ms was seen in either mode with either rasteriser.
- **A page transition is not shorter.** The tasks of 55 to 74 ms are UI work.

### Recommendations

1. **The loader keeps choosing the module with threads wherever it can run** (the table of "B2.8" as it is). The numbers give no case in which the render thread is slower in use, the module is not larger, and the first frame pays 21 to 81 ms with a GPU, depending on the route. `?Threads=false` stays the way out. Two reservations, neither measured: phones (the fixed memory), and a slow network on the service worker route.
2. **Do not add a tick out of turn for an overdue animation frame.** It was proposed for the lag of "B2.7". The measurement shows that lag on one thread as well, where there is no tick to ask for, and shows it gone with a GPU; and it would draw for hidden pages. Whether a frame drawn out of turn would be presented any sooner was not tried. The tests already wait for a drawn frame (`catalogRequestFrame`), which is the right remedy for a test.
3. **Close the splash screen when the first frame has been drawn, in a module that renders on a render thread**, not at the first animation frame of the thread of the page: the completion of the first batch (`CompositionBatch::rendered`, what `catalogFrameDrawn` reads) is the signal. This is a deviation from upstream that the render thread of this port needs. The catalog's prefetch then starts after the first frame again without a change of its own. This is the one defect the measurements found.
4. **The fixed memory: 256 MB is supported by what was measured, which is not yet enough to change the default.** The peak of the tour of thirteen pages is 128 MB, so 256 MB is twice it and 512 MB four times. The tour is not the whole catalog, and "B2.7" asked for a session that visits every page before the default moves; that session was not run here either. Do that, then set `FERROUI_BROWSER_THREAD_MEMORY_MB` to twice its peak, rounded up. A smaller reservation matters where a browser refuses a large one, which only a phone can show. **Done since ("B3 follow-ups"): the whole catalog needs 289 MB for one pass and 470 MB for two, so 256 MB is not supported, and the default stays 512 MB.**
5. **Measure first frames with a GPU as well.** Under the software rasteriser about 2 s of the 3 s to the first frame of the catalog are a wait of the thread that renders that a GPU does not have (0.7 s in all with Metal). First-frame figures taken with the software rasteriser, those of sections 19 and 20 of `browser-platform.md` and any taken in CI, contain that wait and hide differences of tens of milliseconds. `first-frame.mjs --angle metal` (or `FERROUI_BROWSER_ANGLE`) measures without it on a machine that has a GPU.
6. **The UI on a worker (section 6, "Not in B2") is not indicated by these numbers.** It "is worth designing only if B3 shows that UI work on the main thread, not rasterisation, is what holds frames back": frames are not held back (the render thread keeps its animation frames through a transition), and what remains on the thread of the page is 7 ms per wheel event and tasks of up to 74 ms when a page is built. Those are costs of layout and of building pages, for which `performance/README.md` has the designs; moving them to a worker would not make them smaller.

### What could not be measured, and why

- **A network and the published host**: everything was served from the machine itself. The reload of the service worker route and the later start of the download on the site with both modules are the two costs a network enlarges.
- **Other browsers, other machines, phones**: one machine and headless Chrome were at hand. Whether a phone accepts the fixed memory, and what the pool costs on a slow processor, are open.
- **When a frame reaches the screen**: "drawn" is when the thread that renders finished the frame. The presentation of a worker's canvas and of the page's was not timed.
- **A scene that is expensive to draw in WebGL**: nothing in the catalog that animates by itself costs more than 1 ms a frame on this machine. The Software2D mode, where a frame costs 13 ms, stands in.
- **Memory outside the module** (compiled code, the workers, the WebGL contexts): the exports report the memory of the module only.
- **A quiet machine**: it was shared with builds and system work. The load average was 5 to 14 during the series (it is recorded per run), the runner waited while it was 12 or more, and one series (the Home page in WebGL2 under the software rasteriser) ran while it rose to 125 and is marked; a repeat of it was started and given up when the load stayed high. Processor times are trusted; of wall-clock times only differences larger than the spread of the runs are.
- **Why the software rasteriser makes the thread that renders wait**, in the first frame and for the animation frames after it: only the wait was observed, from the traces, not its cause in the GPU process.

ID="$(scripts/browser/skia-threads-shim.sh --id)"; cat .tools/skia-threads/$ID/complete
wc -l .tools/skia-threads/$ID/skia-defines.txt .tools/skia-threads/$ID/functions.txt
scripts/build-browser.sh themed_view --threads
node scripts/browser/wasm-features.mjs --require atomics,bulk-memory \
  target/threads/wasm32-unknown-emscripten/browser/build/*/out/*.a \
  target/threads/wasm32-unknown-emscripten/browser/build/skia-bindings-*/out/skia/*.a \
  target/browser-threads/themed_view/themed_view.wasm
scripts/build-browser.sh themed_view && node scripts/browser/tests/themed_view.test.mjs
```

Expected:

- `setup.sh --threads` ends its new part with `774 functions, the same as the published shim`, then `libskia.a: 1020 of 1020 objects: atomics bulk-memory mutable-globals sign-ext` and one line for the four objects of the new shim with `atomics bulk-memory` among their features (the other features depend on the compiler of the SDK) and no `disallows`, then `written: ...`. A second run prints `up to date`. `functions.txt` has 774 lines; `skia-defines.txt` has the definitions, among them the six named above, `SK_GANESH`, `SK_GL` and `NDEBUG`.
- The threaded build prints, after the link, `== target features (threads)` with the module (`themed_view.wasm: no target_features section [shared memory]`: Emscripten strips the section from an optimised module; with `--debug` the features may be listed) and a line or two per archive of C and C++ objects, all with `atomics bulk-memory`. The build fails there if an object lacks them.
- The command by hand lists every object of the archives of the threaded target directory (Skia's 1020, the four of the shim, HarfBuzz, the setjmp bridge), each with `atomics bulk-memory`, and the module, and exits with 0; add `--summary` for a line per archive. A target directory built before this change has a second `skia-bindings-*` directory with the published shim, whose four objects are then listed on the standard error as lacking the features: the directory of the new build is the one whose `output` file names the `file://` address.
- The build script of the crate logs `FROM: file://.../skia-binaries-b7f043e0b1e2a850e702-wasm32-unknown-emscripten-ganesh-gl-jpegd-jpege-pdf.tar.gz` and `DOWNLOAD AND INSTALL SUCCEEDED` in `target/threads/.../build/skia-bindings-*/output`. `STARTING A FULL BUILD` there means the archive was not found under the key the crate asked for.
- The threaded tests as before (`thread_spawn`, `render_worker_clear`, `storage_view` and `themed_view` served isolated), and the build without threads unchanged: it never reads `.tools/skia-threads`.

### What is doubted, most likely first

1. **`gn gen` on a Skia source without its third-party checkouts.** The crate runs `tools/git-sync-deps` before `gn`; the script does not (it compiles nothing of Skia). Read: the build files import nothing from `third_party/externals` except `third_party/libjxl`, which this feature set should not reach, and `gn` does not look for source files when it generates. Not run. If it fails, the definitions have to come from elsewhere: a full `git-sync-deps` first, or the list written by hand from `gn/skia.gni` and `BUILD.gn`.
2. **The arguments given to `gn`** are copied by hand from `build_support/skia/config.rs` and `platform/emscripten.rs` for this feature set. A wrong one changes the definitions. The check of the 774 functions catches the definitions that decide what the shim contains; a definition that only changes a layout or an inline function in a header would not be caught. The compiler named to `gn` (`emcc`, `em++`) is a guess at what the published build used; it should not change the definitions.
3. **The flags of the compile** are those the `cc` crate 1.6.0 and the build script give for this target, read from their sources, with `-O3` for the release profile the published binaries were presumably built with; `-nobuiltininc` with the include directories of the sysroot is what the crate passes and was not tried with this SDK.
4. **The link without `--no-check-features`.** B1 recorded the shim as the only thing the linker refused. If another object is refused, the message names it.
5. **The check after the link** fails a threaded build for any C or C++ object without atomics. HarfBuzz and the setjmp bridge have them in the existing threaded build; an object without a `target_features` section is reported and passes.
6. **A change of the Skia features of the workspace** makes the crate ask for a key that is not in the directory, and it then starts a build of Skia from source instead of failing. CI checks the features of the target (`ci.yml`, "Skia features of the target"); the pin is `SKIA_FEATURES_KEY` and `SHIM_SOURCES` in the script.
7. **Downloads**: the crate file from `static.crates.io`, the tag archive from `codeload.github.com` (its top directory is assumed to be `skia-<tag>`, as the crate assumes), the `gn` binary from `chrome-infra-packages.appspot.com` through Skia's `bin/fetch-gn`. The options of `tar` are those both the `tar` of macOS and GNU `tar` take.
8. **Skia's own thread safety** is not changed by this: objects Skia documents as not shareable between threads stay so. The shim compiled for threads makes the reference counts, the statics and the thread-local storage of its inline code behave as those of Skia's own objects already do.

### Result of the validation of the Skia shim (2026-10-09)

`scripts/browser/setup.sh --threads` built the shim on the first try, in 26 seconds including the downloads: the definitions came from `gn gen` without Skia's third-party checkouts (28 definitions), the four sources compiled with `-pthread`, the new shim defines the same 774 functions as the published one, and the checks pass (`libskia.a`: 1020 of 1020 objects with atomics and bulk memory; `libskia-bindings.a`: 4 of 4).

`scripts/build-browser.sh themed_view --threads` then links without `--no-check-features`. The check after the link lists every C and C++ archive of the build with atomics and bulk memory (the two of Skia, HarfBuzz, the setjmp bridge) and the module with a shared memory. `themed_view.test.mjs` against that site passes its 72 checks, rendering from the worker with the shim compiled for threads.

## B3 follow-ups (written, 2026-10-09)

Status: written on 2026-10-09 on top of the measurements of "B3", **no module built**: no cargo and no browser build. The Rust below was checked against the source by reading; the scripts parse and were run against the sites the validation of "B3" left in the main checkout, which still have the old behaviour.

### The splash closes on the first drawn frame

**What closed it.** `FerroView::with_host` asked the top-level for an animation frame (`top_level.request_animation_frame`) and closed the splash in its callback, as upstream's `AvaloniaView` does. That callback is run by the media context on the thread of the user interface, when it pulses the animation clock at the start of the first frame it prepares (`MediaContext::render_core`): before the layout and the commit of that frame, and before anything is drawn. Upstream's view is drawn by the thread that runs that callback, soon after it. On one thread this port does the same, and the splash was still early: measured against the site without threads in the main checkout, the class was set 0.45 s before the first frame was finished under the software rasteriser (`themed_view`: splash closed at 4420 ms, first draw call at 4617 ms, frame reported at 4867 ms, on a loaded machine), in an earlier task than the one that drew, with no frame drawn. So "on one thread the splash closes on the first frame" was not true either; the page could present the closed splash before the frame, for a shorter time. With a render thread the callback runs on the thread of the page, the frame is drawn by the worker when the browser gives it an animation frame, and the page presents the closed splash over an empty canvas for as long as that takes (2.1 s in the catalog under the software rasteriser, 0.1 s with the GPU).

**What closes it now.** The first frame that reached the canvas of the view, in both modes:

| Piece | Where | What |
|---|---|---|
| The count and the notification | `rendering/browser_surface_shared.rs` | `BrowserSurfaceShared` counts the frames of its canvas (`frames`, an atomic). `on_first_frame(handler)` is called by the thread that created the canvas: the handler stays in a table of that thread, by the id of the render target (it holds objects of the page and cannot cross); the thread is recorded. `frame_presented()` is called by the thread that drew: at the first frame it runs the handler directly when it is the thread that asked, and otherwise queues a call for that thread through the queue of the platform (`thread_proxy::run_on_thread`), which carries the id only. `forget_first_frame()` drops a handler that still waits |
| Where a frame is counted | `rendering/browser_web_gl_render_target.rs`, `rendering/browser_software_render_target.rs` | Where `RenderStatistics::frame_presented` already is: when the session of the WebGL render target ends, and after `putPixelData` |
| The view | `ferro_view.rs` | Gives the handler that closes the splash to the surface of its top-level, in place of the animation frame |
| The disposal | `rendering/render_target_browser_surface.rs` | `dispose` forgets the handler of a view closed before its first frame |

Nothing is polled and nobody waits. The order of the two threads: the thread of the page records itself and stores the handler before it reads the count; the thread that draws counts before it reads the thread. Whichever comes first, the handler runs once. A handler given after the first frame runs at once.

**What this does not do.** A view that never draws (a hidden page, a host element without a size) keeps its splash; before, the splash closed at the first animation frame, which a hidden page does not have either. When the call cannot be queued (the queue of the platform could not be created), the splash stays: there is no second route from the worker to the page, and the same failure keeps the dispatcher from being woken.

**The prefetch of the catalog** (`page-assets.js`, `prefetchAfterFirstFrame`) waits for the class of the splash and needed no change: it starts at the first frame again in both modes.

**Deviation**: recorded in `DEVIATIONS.md` (Browser backend).

**What the validating session should see after a rebuild** (`scripts/build-browser.sh themed_view --both`, `scripts/build-browser.sh control-catalog-browser --both`):

1. `cargo test -p ferroui-browser` (host): three new tests in `rendering::browser_surface_shared` (`the_first_frame_runs_the_handler_once`, `a_handler_given_after_the_first_frame_runs_at_once`, `a_forgotten_handler_does_not_run`).
2. `node scripts/browser/tests/themed_view.test.mjs` (32) and the same against `target/browser-threads/themed_view` (72), `node scripts/browser/tests/control_catalog.test.mjs` (13) and against `target/browser-threads/control-catalog-browser` (16): the same numbers as before. Every start of a page now asserts that the module reported at least one frame in the task that closed the splash (`SPLASH_PROBE` and `splashFrames` of `harness.mjs`); the failure reads "the splash was closed before the first frame of the view was drawn". **Against a module built before this change every check fails with that message** (the sites in the main checkout do: `{"frames":0,...}`), which is how to tell that the module under test is the new one.
3. `node scripts/browser/first-frame.mjs target/browser-threads/control-catalog-browser --isolated --phases --runs 3` prints one new line per load before the phases:

   ```
   load 1: splash closed 2950 ms, first frame 2948 ms: the splash closed 2.0 ms after the first frame; 1 frame drawn when it closed
   ```

   (the times are an example: those of "B3" under the software rasteriser). "first frame" is the frame the module reports; the distance is the hop from the worker to the event loop of the page: 0 to a few milliseconds, more while the page is in a long task, never "BEFORE", and at least 1 frame drawn. The same command against `target/browser/control-catalog-browser` (no `--isolated`), and against the threaded site with `--query "?RenderThread=false"`, prints a distance of 0.0 ms: on one thread the class is set inside the frame and the script sees both in the same task. Before the change the line reads, for the render thread, `the splash closed 2107.0 ms BEFORE the first frame; 0 frames drawn when it closed (CLOSED ON AN EMPTY CANVAS)`, and on one thread about `450 ms BEFORE` with the same remark. Among the phases, "splash closed" is now listed, at or just after "first frame reported by the module".
4. `measure-render-thread.mjs --only first-frame` has one more row in the table of the phases, "frames drawn when the splash closed": 1 or more in every column.

### The fixed memory, from a session over the whole catalog

"B2.7" and "B3" measured a tour of thirteen pages (peak 128 MB) and asked for a session that visits every page before the default of `FERROUI_BROWSER_THREAD_MEMORY_MB` moves. That session was run here, against the sites the validation of "B3" built (no rebuild; the splash change above is not in them and does not matter for memory).

**The tour.** `node scripts/browser/catalog-memory.mjs <site> --all [--isolated] [--query "&RenderThread=false"]`. `catalogPages()` of `catalog-pages.mjs` reads the pages from the list the application is built from (`samples/ControlCatalog/ViewModels/main_window_view_model_page_list.rs`: 74 pages in 11 sections; the host exports no list, and the list of asset files of the site has the pages but not their sections). `visitAll` opens each from the drawer, at 1024 x 1100 in WebGL2 with the prefetch on, scrolls its content down and back with the wheel (four events of 480 pixels; thirty on ListBox and TableView, twelve on TreeView, RefreshContainer, ScrollViewer, WrapPanel, TextBlock and Buttons, so that the rows of the virtualized lists are realized and recycled), and opens the demos that need no file and stay in the page (`PAGE_ACTIONS`): a showcase or demo page of CarouselPage (Sanctuary), DrawerPage (FerroFlix), TabbedPage (Fluid Nav Bar), NavigationPage, ContentPage (its performance monitor), CommandBar, PipsPager (Care Companion) and TextBox (fonts and complex scripts), the tabs of TabControl, Composition and Image, two expanders, two transitions of TransitioningContentControl, two notifications. Every demo named was shown in every run. The memory is the end of the dynamic memory of the module (`catalogMemory`, sampled ten times a second by the page), in MB of 1,048,576 bytes.

**Pages not visited** (2 of 74):

| Page | Why |
|---|---|
| OpenGL Lease | Not in the drawer: the application leaves it out (`excluded.txt` of the sample; it waits for Ganesh on OpenGL in the desktop build) |
| ComboBox | **The page ends the application in a browser.** Opening it panics (`Could not create glyphTypeface. Font family: Cascadia Mono (key: fonts:SystemFonts)`, `src/FerroUI.Base/media/typeface.rs:95`); the navigation never finishes and the view draws no further frame. Seen in the first whole tour, with the module without threads; the tour skips the page since. A browser has no system fonts, and the page creates a typeface of a family the font manager names. This is a defect of the catalog in the browser, found by the tour and not fixed here (the catalog is outside this task) |

Visited, with what the tour could not do there: Dialogs (its pickers and windows need a file or a window; the page is opened and scrolled), Clipboard and Drag+Drop (opened and scrolled; nothing copied or dragged), OpenGL and OpenGL Interop (the knobs and the information text: a browser has no GL control), Native Embed (opened; its popups not shown), Flyouts, Menu, ContextMenu and ContextFlyout (opened and scrolled; no flyout is opened, because the flyout of the Flyouts page stays open over the view after Escape and blocks the drawer), ColorPicker and the date pickers (their drop-downs are not opened).

**Result**, two runs of each configuration (the three configurations of a round at the same time; load average of the last minute 25 to 54 in the first round, 11 to 28 in the second: the machine had been restarted and ran other builds; the numbers are sizes, not times, and the two runs agree within 1.4 MB):

| Configuration | At the start | At the end | Peak | Size of the memory |
|---|---:|---:|---:|---:|
| without threads | 35.1, 35.1 | 286.0, 287.3 | 286.0, 287.3 | 326.6, grown to |
| one thread (`?RenderThread=false`) | 35.0, 35.1 | 285.6, 286.9 | 285.6, 286.9 | 512, fixed |
| render thread | 59.8, 59.8 | 288.7, 288.6 | 288.7, 288.6 | 512, fixed |

72 pages visited in every run, no error on the console of the page. The memory rises with nearly every page and never falls (the end of the dynamic memory cannot fall; but a page whose memory was free again would be paid for by the next one, and it is not). The largest steps: DrawerPage with its showcase 30 MB, Calendar 29 MB, the first page visited 24 MB without the render thread (the files of the prefetch arrive then; with the render thread the start already has them), Container Queries 19 MB, TextBox 14 MB, TableView 8 to 9 MB, Flex Panel 8 MB. The render thread costs 2 to 3 MB at the end, as in the tour of thirteen.

**A page that is visited again costs again.** `--passes 2` goes through the catalog twice in one session (render thread): 288.8 MB after the first pass, **470.4 MB after the second** (71 of the 72 pages; one did not open in time). The second pass costs 182 MB, four fifths of the first. A third would not fit in 512 MB. So the peak of "a session that visits every page" is not a property of the catalog that a fixed size can be read from: the memory in use grows with what the user does, at 2.5 to 3 MB a page visit, and does not come back. Whether the pages are kept alive after the navigation leaves them (the catalog creates a page on every visit: `PageItem::create_page`), whether what they created is (bitmaps, text layouts, compiled styles, composition objects), or whether it is the allocator not reusing what was freed, was not looked for: it needs a build with the allocation counters (`perf-counters`) and is the next thing to do here. It is not a property of the threaded build either: the module without threads grows the same way (286 MB after one pass) and is only saved by a memory that can grow.

**Recommendation.**

1. **The default stays 512 MB; it was not changed.** The rule was the peak of the whole session with a factor of two under the size. The peak of one pass is 289 MB: 512 MB is 1.77 times that, and the 256 MB that "B3" held possible would have ended the application before the end of the first pass (at TextBox, with 246 MB in use before it and 14 MB for it). Nothing smaller than 512 MB is supported by this measurement.
2. **512 MB is enough for one visit to every page of the catalog and not for a session of any length.** By the rule (twice the peak, rounded up to a power of two) the size for one pass would be 1024 MB, and that only moves the end to the fourth or fifth pass. Raising the default was not done either: it does not remove the limit, a reservation of a gigabyte is the one most likely to be refused on a phone ("B3", what could not be measured), and the published site falls back to the module without threads only when the module with threads cannot be created, not when it runs out later. The remedy is to find why a page visit keeps its memory (above); with pages that give their memory back, the peak is the base plus the largest pages, and the default can then be set from a `--passes 3` run that no longer grows.
3. **What happens when the size is exceeded.** The memory of a module built with threads does not grow (`-sALLOW_MEMORY_GROWTH=0`, "B2.2"): the allocation that does not fit fails, and the module aborts with a message on the console of the page that names `INITIAL_MEMORY` (`build-browser.sh` passes the size as `-sINITIAL_MEMORY=<n>MB`); the page stays as its last frame left it and answers nothing. It was not provoked here: the two passes ended 42 MB under the limit.
4. **How an application raises it.** `FERROUI_BROWSER_THREAD_MEMORY_MB=1024 scripts/build-browser.sh <application> --threads` (or `--both`): a number of megabytes between 16 and 2048, fixed when the module is linked. There is no setting at run time. An application sizes it with `catalogMemory`'s counterpart of its own (the end of the dynamic memory, `sbrk(0)`, sampled over its longest session), and takes twice that.
5. **Until the growth is understood, a long session of the published catalog is safer on the module without threads** (`?Threads=false`), whose memory grows. The loader's choice is not changed by this: a visitor who opens a dozen pages is far from the limit, and the measured gains of the render thread are for that visitor.

**What was added to the scripts.** `catalog-pages.mjs`: `catalogPages`, `PAGE_ACTIONS`, `visitAll` (a tour that goes on when a page does not open and ends when three in a row do not). `catalog-memory.mjs`: `--all`, `--passes <n>`, `--verbose`. The tour of thirteen and its users (`control_catalog.test.mjs`, `capture-catalog.mjs`) are unchanged.

