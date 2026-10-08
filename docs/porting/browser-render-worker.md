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
- **The handles the graph shares with the UI thread.** R5.5 released the graph on the UI thread because the graph holds `Rc` handles whose counts the UI thread changes (the platform graphics, the render interface). Releasing the graph on the render thread would turn that around, so `Compositor::drop` takes those handles out first, on its own thread. For that the context manager holds the platform graphics and the ready state feature in cells. The clock was a third shared `Rc` (one handle cloned into the server compositor): each side now makes its own handle over the shared function.
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
5. **A feature leaves the lock as an `Rc`.** Open item 4 of R5.5 is unchanged: `try_get_render_interface_feature` clones the feature inside the lock and the caller drops it outside, while the render thread drops the cache on its side when the context goes. A feature that a confined platform publishes has to be safe for that (what the browser's Skia context publishes is to be read in B2.5; on Metal the map is empty, R5.5).
6. **`with_server` is public.** The type cannot keep a caller from rendering, or from creating the backend context, inside it. The assertion is in `render_on_this_thread` only.
7. **"Absent" for "not known yet".** A caller of the feature query cannot tell the two apart and is not told when to ask again. `OpenGlCompositionInterop` (`src/FerroUI.OpenGL`) asks when it is set up; with a confined compositor it would have to ask again after the first frame. A completion on the dispatcher would settle it, and needs the feature map to cross threads, which is item 5.
8. **One render thread for the life of the compositor.** A timer that ticks from more than one thread (a pool) makes the second frame panic. `ThreadProxyRenderTimer`, which the native platform uses, ticks from its one thread **[M]**; the other background timers of the tree were not read for this; the browser's will tick from the worker.
9. **Where the loop runs is read once.** A timer that only starts to report `runs_in_background()` true after the compositor was created (the browser's, if the compositor is created before the render worker has started) gives a compositor that is not confined: its synchronous commits still wait (the media context asks the loop each time), but the feature query and the release take the paths of the lock model, on the UI thread. B2.6 has to create the timer in its final state before the compositor, or pass the mode explicitly.
10. **The leak of a `LockBound`.** The interop object of a confined compositor leaks the counts of the context and of the feature when its owner drops it, and an imported image whose owner outlives its disposal job leaks its shell. Correct would be a queue of values for the render thread to drop at its next entry into the lock; it needs an `unsafe impl Send` for values that are not `Send`, and was left out while no confined platform has the feature.

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
