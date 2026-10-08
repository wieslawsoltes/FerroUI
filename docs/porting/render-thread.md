# Render thread: a UI thread and a render thread, on the desktop and in the browser

Status: design and plan, 2026-10-08. The owner made this critical work on 2026-10-08: the port is to have upstream's two-thread model (the UI thread commits batches, a render thread owns the server compositor and draws) on the desktop and in the browser. This page reverses two earlier decisions: "the server compositor stays on the thread of its compositor" (row 12 of `CRITICAL-PATH.md`) and "single-threaded, browser-driven; no WASM threads" (`browser-platform.md`, section 6).

Markers: **[M]** measured or read from the code, **[E]** estimated, **[H]** a hypothesis that a stage must verify before it builds on it.

## 1. Where the port is today

- **[M]** One thread. On macOS the display link of the native backend ticks on its own thread (`PlatformRenderTimer.mm`, `FerroNativeRenderTimer` is `Send + Sync`), and `ThreadProxyRenderTimer` and `ServerCompositorLoopTask` marshal every tick to the thread of the compositor, where `ServerCompositor::render` runs. In the browser the render timer is `requestAnimationFrame` on the only thread.
- **[M]** The compositor is already split as upstream splits it: client objects, a serialised batch (`CompositionBatch`, `BatchStreamData`, `CommittedBatch`), server objects (`rendering/composition/server`, 35 files), readback indices, jobs and post-target jobs. The split is honoured in the code; what is missing is the freedom to run the two halves on different threads.
- **[M]** What stops that is ownership, not structure. The server half uses `Rc` (284 uses in `server/`, 197 in `drawing/`), which is right for objects confined to one thread, and stays. The batch that crosses the boundary carries payloads that are not `Send`.

## 2. What crosses the boundary (compiler audit)

Requiring `Send` of the reference-typed payloads of a batch (`BatchObject::Value(Box<dyn Any + Send>)`) makes the base crate fail on exactly these types **[M]**:

| Payload | Where it comes from |
|---|---|
| `Rc<dyn IBrush>` | immutable brushes of the render data and of visuals |
| `Rc<dyn IPen>` | immutable pens |
| `Rc<dyn IEffect>` | immutable effects |
| `Rc<dyn IRenderDataGeometry>`, `Rc<dyn IGeometryImpl>` | geometry of draw operations and clips |
| `Rc<Rc<dyn IGlyphRunImpl>>` | glyph runs |
| `Rc<Rc<dyn IBitmapImpl>>` | bitmaps |
| `Rc<dyn ICustomDrawOperation>` | custom draw operations |
| `Rc<T>` of `BatchResource<T>` | any other immutable resource sent by value |

Besides the payloads: `BatchObject::Job`, `ObjectJob` and `Create` hold closures made on the UI thread and run on the server (`ServerJob`, `ServerObjectJob`, `ServerObjectFactory`); the plain values of the batch are bytes and already cross freely.

**[M]** Requiring `Send + Sync` of `IGeometryImpl`, `IBitmapImpl` and `IGlyphRunImpl` fails in the base crate at one type so far (`ImmutableGeometryImpl`, which holds an `Rc`); the Skia backend was not reached by that audit and is audited in stage R1.

**[M]** The UI side reaches into the server directly in a bounded number of places: `Compositor` holds `server: Rc<ServerCompositor>` and uses it six times; 31 other expressions outside `server/` name a server object.

## 3. The design

The rule: **the server compositor and every server object are created on the render thread, used only there, and never leave it.** They keep `Rc` and `RefCell`. Only what a batch carries has to be thread-safe.

1. **Render resources are shared, immutable and thread-safe.** The payload contracts of section 2 gain `Send + Sync`, and a batch holds them in `Arc`. This is what upstream relies on implicitly: an immutable brush, a geometry implementation, a bitmap and a glyph run are read by the UI thread and drawn by the render thread at the same time.
   - Immutable brushes, pens, effects and transforms are plain data: their types become `Send + Sync` by holding their parts in `Arc` (gradient stops, dash styles, the bitmap of an image brush).
   - `IGeometryImpl`, `IBitmapImpl` and `IGlyphRunImpl` are implemented by the render backend. Skia's path, image and text blob are thread-safe for reading **[H]**; the wrappers of the Skia backend must hold them without `Rc` or `RefCell` (caches of derived data go behind a `Mutex` or `OnceLock`).
   - The conversion happens where the UI object is serialised (`to_immutable()`, the render data writer), so the mutable media objects of the UI thread (`SolidColorBrush`, `Pen`, `StreamGeometry`) stay `Rc`-based. The blast radius is the immutable and platform types, not the property system.
   - `ICustomDrawOperation` gains `Send + Sync`, as upstream documents that it is called on the render thread.
2. **Jobs and factories are `Send`.** `ServerJob`, `ServerObjectJob` and `ServerObjectFactory` become `Box<dyn FnOnce(..) + Send>`. A factory builds the server object on the render thread from plain data. A job that returns something to the UI thread does it through `ServerJobTask`, whose shared state becomes `Arc<Mutex<..>>` with completion posted to the dispatcher of the compositor (today it is `Rc<RefCell<..>>`).
3. **The compositor holds a handle, not the server.** `Compositor.server: Rc<ServerCompositor>` becomes a handle that can only enqueue (batches, jobs) and read what is published for reading (`ReadbackIndices`, which is already `Arc`). Each of the six direct uses and the 31 outside expressions is turned into a job, a readback, or is shown to run on the render thread already. The server compositor is constructed by the first tick on the render thread.
4. **The render loop runs where its timer ticks.** `RenderLoop` calls its tasks on the timer's thread when the timer runs in the background, as upstream does; `ServerCompositorLoopTask` stops marshalling. The dispatcher-thread mode stays, selected by a compositor option, for the browser without threads and for the tests (upstream has the same switch: `UseUiThreadForSynchronousCompositorCommits` and the `IRenderTimer.RunsInBackground` contract).
5. **Synchronous commits and resizes follow upstream.** A resize and a first show wait for the render thread to draw the committed batch (`CompositionBatch::rendered`, `ServerCompositor::ui_thread_is_inside_render` and the render-interface lock of upstream's `Compositor.Commit` path), so that a window never shows a stale frame.
6. **Platform graphics on the render thread.** The Metal device and command queue of the macOS backend are thread-safe; the Graphite context of the Skia backend is created on, and confined to, the render thread; `IPlatformGraphicsContext::ensure_current` takes the lock upstream takes (`DisposableLock` of `MetalDevice`, which the port waived as "the Metal device takes no lock" and must now port). Presenting a drawable from the render thread is what the native code already does for the display link.
7. **The software framebuffer path** (`DeferredFramebuffer` of the native backend) draws on the render thread and hands the frame to the UI thread, as upstream's does.

What does not change: the property system, layout, input, styling and every control stay on the UI thread with `Rc`. `Dispatcher` is already `Arc` and thread-aware.

## 4. The browser

Upstream's browser backend has a threaded mode (`WasmEnableThreads`): the UI on a worker with a blocking dispatcher, a render worker drawing into an `OffscreenCanvas`, raw input grouped and queued across threads. The port left it out. To enable it, in order of what can block:

1. **Cross-origin isolation.** Threads in the browser need `SharedArrayBuffer`, which needs the page to be served with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`). GitHub Pages cannot set response headers **[H]**; the usual way around it is a service worker that adds the headers to the page's own responses (the site already ships a service worker, `ferroui-sw.ts`). To verify first, with a two-thread "hello" page on the published host.
2. **The Rust toolchain.** A threaded build needs the standard library compiled with atomics (`-C target-feature=+atomics,+bulk-memory`, `-Zbuild-std` on a nightly toolchain) **[H]**; the pinned toolchain of the browser target is stable 1.99.0. To verify: whether `wasm32-unknown-emscripten` with `-pthread` links and runs a spawned thread on the pinned stable toolchain, and if not, which nightly is needed and what it costs (the pin exists for WebAssembly exception handling).
3. **Skia.** The browser build links prebuilt Skia objects (the project's rule: never build Skia from source). Emscripten requires every object of a `-pthread` link to be built with threads **[H]**; the published binaries are built without **[H]**. If both hold, the browser cannot have threads until rust-skia publishes a threaded Emscripten binary or the owner lifts the binary-only rule for this target. To verify: link `themed_view` with `-pthread` against the published binary.
4. **The backend itself.** Port `RenderWorker`, the worker branch of `WebRenderTargetRegistry`, `BrowserWindowingPlatform.EventGrouperDispatchQueue` and the blocking dispatcher (`ManagedDispatcherImpl` on the UI worker), with `OffscreenCanvas` for WebGL2. This is the part that is ordinary porting once 1 to 3 hold.

### Stage B0, measured on 2026-10-08

| Check | Result |
|---|---|
| The pinned stable toolchain (Rust 1.99.0, Emscripten 6.0.10) | **[M]** A program that spawns a thread builds without thread flags and fails at run time (`spawn failed: Not supported`). With `-C target-feature=+atomics,+bulk-memory -C link-arg=-pthread` the link fails: `wasm-ld: --shared-memory is disallowed by panic_unwind ... because it was not compiled with 'atomics' or 'bulk-memory' features`. The standard library that ships with the stable toolchain cannot be used for a threaded build. |
| A rebuilt standard library | **[M]** With a nightly toolchain (the 1.93.0 nightly of 2025-10-31 that is installed, with `rust-src`), `-Zbuild-std=std,panic_unwind`, the flags above, `-C linker=em++` and `-sPTHREAD_POOL_SIZE=2`, the same program builds and runs under Node: the spawned thread runs and is joined. A threaded build therefore needs a nightly toolchain and `-Zbuild-std` until the target ships a threaded standard library; the nightly has to be one with the WebAssembly exception handling that made the project pin 1.99.0 (section 3 of `browser-platform.md`). |
| The prebuilt Skia objects | **[M]** The hypothesis of item 3 is wrong, in the port's favour. The first 40 objects of `libskia.a` of the published Emscripten binary (skia-bindings 0.153.3) all declare `+atomics` and `+bulk-memory` in their `target_features` section, so the linker accepts them in a shared-memory link. Not yet done: linking and running the port's own module with Skia on a thread (stage B1). |
| Cross-origin isolation on the published host | Not measured yet. It needs a page on the host (GitHub Pages) with a service worker that adds the two headers, and a check of `crossOriginIsolated`; it is the first step of B1. |

So the browser is not blocked by Skia. Its cost is the toolchain: a nightly compiler with a rebuilt standard library for the threaded build, next to the stable one for the build without threads, or one nightly for both.

The non-threaded mode must keep working: a host without cross-origin isolation falls back to it, as upstream's does.

Items 1 to 3 are feasibility checks of an hour or two each and are done first (stage B0); their outcome decides whether stages B1 to B3 can start or wait for a dependency. The desktop stages do not depend on them.

## 5. Stages

Each stage is a pull request that builds and passes on its own; the single-threaded mode keeps working until the last desktop stage switches the default.

| Stage | Content | Done when |
|---|---|---|
| R1 | Thread-safe render resources: `Send + Sync` on the payload contracts, `Arc` in the batch and the render data, the immutable media types and the Skia wrappers made thread-safe. Compiler-driven: the audit of section 2 becomes permanent (`BatchObject::Value` requires `Send`). | The workspace builds and passes with the bound in place. |
| R2 | `Send` jobs and factories; `ServerJobTask` over `Arc<Mutex<..>>` with completion on the dispatcher. | `CommittedBatch: Send` is asserted at compile time. |
| R3 | The compositor's handle to the server; every direct access from the UI side turned into a job or a readback; the server compositor constructed on its own thread. | No code outside `server/` names a server object; a test runs the server on a second thread and commits from the first. |
| R4 | The render loop on the timer's thread; synchronous commit, resize and first-show waits; the dispatcher-thread mode kept behind the option. | The compositor suites pass in both modes. |
| R5 | macOS: Graphite/Metal context and the software framebuffer on the render thread; the Metal device lock; render-thread naming and priority as upstream. Default switched to the render thread on the desktop. | `themed_window` and the catalog run with rendering off the UI thread; start-up and scroll numbers of `desktop-performance.md` re-measured; a stress run (resize, theme switch, scrolling) shows no stale or torn frame. |
| B0 | Browser feasibility: the three checks of section 4. | Each is answered with a measurement, recorded here. |
| B1 | The browser toolchain and site for threads (headers or service worker, build flags, the non-threaded fallback). | A two-thread page of the port's own module runs on the published host. |
| B2 | The threaded browser backend: render worker, `OffscreenCanvas`, the event grouper queue, the blocking dispatcher. | `themed_view` and the catalog render from the worker; the browser tests pass in both modes. |
| B3 | Measurements: first frame and scrolling, threaded against not. | The table is in `browser-platform.md`. |

Order: R1 to R5 in sequence; B0 in parallel with R1, since it only measures.

### Progress of R1

R1 is delivered one payload contract at a time; each step builds and passes on its own.

| Step | Contract | State |
|---|---|---|
| R1.1 | `IGeometryImpl`, `IStreamGeometryImpl`, `ITransformedGeometryImpl` | Done: the contract requires `Send + Sync` and is held in `Arc` everywhere. The Skia geometries keep their paths behind a lock and hand out copies (a copy of a path shares its storage); the stroke cache and the path measure are under a lock; the headless stubs and the test geometries follow. |
| R1.2 | `IGlyphRunImpl` | Done: `Send + Sync`, held in `Arc`. The text blob cache of the Skia glyph run is under a lock, and the closures of the two level cache are `Send`. |
| R1.3 | `IBitmapImpl` and the contracts built on it | Done. A bitmap of the UI side is a `SharedBitmapImpl` (`dyn IBitmapImpl + Send + Sync`) held in `Arc`; `IWriteableBitmapImpl` and `IRenderTargetBitmapImpl` require `Send + Sync`; the counted reference (`RefCounted`) counts atomically. A layer of a drawing context stays an `Rc` on the render thread and is drawn by reference; where the original hands a layer to the UI thread (the snapshot of a visual), the layer gives a shared snapshot of its contents instead. In the Skia backend the pixels of a writeable bitmap and the image drawn from them are under one lock, and the render target a render target bitmap draws itself with is bound to the thread that created it (`ThreadBound`). |
| R1.4a | Custom draw operations; the counted holders of render data; the geometry wrapper | Done: `ICustomDrawOperation: Send + Sync` in `Arc`; glyph runs and bitmaps of render data are held as `Arc<Arc<..>>`; a geometry wrapper is never sent by value. |
| R1.4b | Immutable effects | Done: `IImmutableEffect: Send + Sync`, held in `Arc` by render data and by the composition visual. An immutable effect is a value; converting one to immutable gives an equal value in a handle of its own. |
| R1.4b2 | The shared form of immutable brushes and pens | Done for render data: `SharedBrush` and `SharedPen`, one type for all kinds (solid, the three gradients, image), cached by the immutable object; see "Brushes and pens" below. The same forms are what R1.4c sends for the properties of composition objects. |
| R1.4c | The values of `BatchResource<T>`; dash styles; the scene info of a target | Done: a resource value is sent in its wire form (`BatchResourceValue`): the shared form of a brush, the matrix of a transform, the offset and colour of a gradient stop; the server makes a handle of its own from it. A dash style is sent by value. The platform specific scene info is an `Arc<dyn Any + Send + Sync>` from the window implementation to the server target. |
| R1.4d | Animation instances | Done: an animation creates the *factory* of its instance (`AnimationInstanceFactory`), which is what a batch carries and the server calls. Parsed expressions are held in `Arc`; the parameters cross as a snapshot source from which the snapshot is built on the server; a key frame carries its easing in shared form (`IEasing::to_shared`); see "Animation instances" below. |
| R1.5 | `BatchObject::Value` requires `Send`; measurements | Done with R1.4d: `Value(Box<dyn Any + Send>)`, and the workspace builds with the bound. Measured below: no change of start-up; the scroll measurement of the browser is not repeated yet. |

Design of R1.4. With `BatchObject::Value(Box<dyn Any + Send>)` the compiler names what is left: the brush, pen and effect of render data, its geometry wrapper, custom draw operations, the two counted holders (glyph run, bitmap) and the values of `BatchResource<T>`.

- A brush, pen or effect object stays an `Rc` on the UI thread, where mutable and immutable ones share one handle type and render data keeps them for hit testing. What is *sent* is the **shared form** of an immutable one: `to_shared()` returns an `Arc<dyn IBrush + Send + Sync>` (and likewise for pens and effects) with the same values, cached by the object so that sending the same brush again costs a count. The server table holds shared forms beside the server objects it resolves from ids; the drawing context takes `&dyn IBrush` either way.
- The immutable types hold their parts in thread-safe form (gradient stops, dash style, transform, the counted bitmap of an image brush).
- `ICustomDrawOperation` requires `Send + Sync` and is held in `Arc`: it is rendered on the render thread, as upstream documents.
- The geometry wrapper of render data (`IRenderDataGeometry`) is never sent as a value: with a compositor a geometry is sent as the id of its server object.
- The counted holders become `Arc<Arc<..>>`.

#### Brushes and pens (R1.4b2), decided

The accessors of the brush and pen contracts return `Rc` handles (`IBrush::transform`, `IPen::brush`, `IPen::dash_style`, `IGradientBrush::gradient_stops`, `IImageBrush::source`), because mutable brushes are objects of the UI thread. An immutable brush as it is today holds such handles too, so it cannot be shared. Two ways were weighed:

1. **Send a value and rebuild the immutable object on the server.** Smallest change, but the rebuilt object has a new identity on every send, and the stroke cache of a geometry is keyed by the identity of the pen's brush (`pen_helper::get_hash_code`): every stroked geometry would recompute its stroke path on every frame. Rejected.
2. **A shared form with a stable identity** (chosen). The values of an immutable brush are copied into one type made of plain values (`Matrix` for a transform, a list of offset and colour for stops, the counted bitmap for an image), which is `Send + Sync`, implements the same contracts, and builds the `Rc` handles its accessors return on demand. `to_shared()` of the immutable object returns the twin in an `Arc`, created once and cached in the object, so the identity the server sees is stable for the life of the brush. A shared pen holds the shared brush; the handle `IPen::brush` returns wraps it and reports the identity of the shared brush.

Open inside this step: the scene brush contents (`CompositionRenderDataSceneBrushContent`, `ImmediateRenderDataSceneBrushContent`) are immutable brushes that hold render data; how they reach the server is read from the code before they are converted. A brush type of an application that implements only the interface has no shared form until it provides one.

#### Animation instances (R1.4d), decided

An animation instance is created on the UI thread (`ICompositionAnimation::create_instance`) and from then on belongs to the server: it resolves server objects, subscribes to them and keeps its clock state in cells. Its type therefore cannot be `Send`, although nothing of the server is in it when it is sent. What is sent instead is an **instance factory**: a `Send` closure that builds the instance on the server (`Box<dyn FnOnce() -> Rc<dyn IAnimationInstance> + Send>`). The property writers of the composition objects keep and send factories; the server calls the factory where it used to take the instance from the batch.

What a factory captures has to be `Send`:

| Captured | Today | Becomes |
|---|---|---|
| The parsed expression | `Rc<Expression>` | `Arc<Expression>` (a tree of values) |
| The parameters | `Rc<PropertySetSnapshot>`, which holds nested snapshots in `Rc` and, once resolved, weak handles to server objects | a snapshot *source* (variant, nested source, or the id of a server object) from which the snapshot is built on the server |
| Key frames | value, expression and an `Rc<dyn IEasing>` | the easing in its shared form |
| The easing of a key frame | an easing object of the UI thread, evaluated on the render thread by convention | `IEasing::to_shared()`: an `Arc<dyn IEasing + Send + Sync>` with the parameters of the easing at that moment. The stateless easings return themselves; the spline and spring easings copy their parameters. |

Found by R1.1 in the Skia backend: a path can be sent to another thread but not shared by reference, and a path measure can be neither. The geometries therefore never lend a path; the path measure is cached behind a lock in a wrapper that asserts it may move between threads (it owns its contours and has no thread affinity).

Found by R1.3: a Skia bitmap cannot be sent to another thread either; it is held in a wrapper under a lock, like the path measure. `ThreadBound<T>` (`utilities/thread_bound.rs`) is the tool for a part of a shared resource that one thread owns: it panics when reached from another thread and leaks, rather than drops, when the resource dies there.

### R1 measured

Release builds (`cargo build --release`, the thin LTO profile) of `main` before R1 (`8c8c595`) and of the branch with all of R1 (`80e17fb`), on the development Mac (Apple silicon, macOS 26). The two builds of an application were launched alternately, after one launch each to warm the caches. **The machine was busy with other work throughout (load average 50 to 80)**, so the absolute numbers are not comparable with `desktop-performance.md`; the comparison between the two builds is what counts.

| Measure | Before R1 | After R1 |
|---|---|---|
| `themed_window`, process start to `Window opened`, median of 15 (range) | 206 ms (171 to 596) | 207 ms (173 to 374) |
| the same, second series | 225 ms (181 to 319) | 219 ms (186 to 494) |
| `control-catalog-desktop`, process start to `App activated`, median of 9 (range) | 411 ms (371 to 504) | 401 ms (373 to 496) |
| catalog, 150 pages in 20 s (`FERROUI_SMOKE_PAGES=150`), user CPU of three runs | 2.22, 2.46, 2.20 s | 2.32, 2.36, 2.71 s |
| size of the unstripped binary, `themed_window` | 100.00 MB | 100.04 MB |
| size of the unstripped binary, catalog | 147.45 MB | 147.95 MB |

Start-up is unchanged. The CPU of the page run is about 0.1 to 0.2 s higher on average after R1, which is inside the spread of the three runs at this load; it is to be repeated on a quiet machine before it is read as a cost of the atomic counts. The scroll profile of the browser (`scripts/browser/scroll-profile.mjs`) was not repeated.

### R2 done

`ServerJob`, `ServerObjectJob` and `ServerObjectFactory` require `Send`, and `CommittedBatch: Send` is asserted at compile time (`transport/batch.rs`). How each item of the table below was settled:

- The queues of the server compositor hold jobs of the render thread (`RenderThreadJob`), which may capture a resolved server object.
- `ServerJobTask` holds its result under a lock; its continuations are bound to the thread of the task and run there, from the dispatcher when the job completed elsewhere; the error is `ServerJobError`.
- The closures that create a server object are `Send`; a custom visual takes the factory of its handler, and its messages are `Arc<dyn Any + Send + Sync>`.
- The debug events of a target are `Arc` and `Send + Sync`.
- **Left bound to the UI thread, in `ThreadBound`:** the render surfaces of a target, the update closures of a drawing surface, the import and dispose closures of the interop objects. A server on its own thread panics on their first use: this is the list of what R3 to R5 (and B2) have to make usable from the render thread.

### R3 done: the server compositor on its own thread

`Compositor::with_render_thread` creates a compositor whose server compositor is created by the first tick of the render loop, on the thread that ticks, from a `Send` factory, and is kept there in a thread-local registry by the key of the compositor. The compositor itself holds what the two threads share (the queue of committed batches under a lock, the readback indices) and no server object; `Compositor::server()` is the accessor of the dispatcher-thread mode and of the tests that run the server on their own thread. Dropping the compositor tells the render thread to release the server compositor at its next tick.

`render_thread_tests.rs` is the test the stage asks for: objects are created and a batch is committed on one thread, the loop is ticked on a second one, a job runs there and reports the thread it ran on and the number of server objects, and its continuation runs on the first thread from the dispatcher.

What the render-thread mode does **not** have yet, each a panic or an absent answer today and the content of R4 and R5:

- No composition target: the render surfaces are bound to the UI thread (R2), so a window cannot be rendered in this mode.
- `try_get_render_interface_feature` answers `None`: the features are objects of the render thread.
- The platform graphics object is made by a closure on the render thread; no backend supplies one yet.
- Nothing chooses the mode: `Compositor::new` (the one applications get) still creates the dispatcher-thread mode. The option and the default are R4 and R5.
- The synchronous points (resize, first show) and the render timer's thread are R4.

### R4 in progress

What the survey of R4 found: the render loop and its timers are ported and already tick in the background where the platform does (the display link of the native backend, the sleep loop, the thread proxy); the dispatcher-thread mode is what marshals those ticks to the UI thread. R4 is therefore the waits and the choice of the mode, not a new loop.

| Step | Content | State |
|---|---|---|
| R4.1 | The synchronous wait of the media context (`SyncWaitCompositorBatch`): in the render-thread mode a synchronous commit blocks on the `Processed` or `Rendered` completion of the batch (`BatchCompletion::wait`) where the dispatcher-thread mode renders on the spot. The condition is upstream's (`UseUiThreadForSynchronousCommits` false and a background loop) and the mode of the compositor. | Done; `render_thread_tests.rs` commits synchronously against a render thread that ticks. |
| R4.2 | `NonPumpingLockHelper` around the waits, as upstream. | Not applicable: the helper only has an implementation on Windows (it keeps a single-threaded apartment from pumping messages while it waits); without one it does nothing. The other places upstream uses it in `Compositor` are not waits, and the port has their logic (a commit requested while a batch is pending is triggered from its `Processed` completion through the dispatcher). |
| R4.3 | The option that chooses the mode and where `Compositor::new` reads it. | Open; depends on R5 for a window to render. |

### R5, the finding that shapes it: both threads render the server compositor upstream

The native platform creates its compositor as `new Compositor(_platformGraphics, true)`: `UseUiThreadForSynchronousCommits` is **true** on macOS, and the render loop runs on the thread of `ThreadProxyRenderTimer`. The port's platform set-up is the same, line for line. So upstream on macOS:

- the render thread renders frames (`ServerCompositor.Render` from the loop);
- the UI thread **also** calls `ServerCompositor.Render`, at the synchronous points (`MediaContext.SyncWaitCompositorBatch`: the resize a window asks for, the first show, the disposal of a target), because AppKit presents a resized window in step with the UI thread;
- `ServerCompositor.Render` takes `lock (_lock)`, and server objects check that they are only touched under it (`VerifyAccess`: "can be only accessed under compositor lock"). The server compositor is not confined to a thread: it is confined to a **lock**.

The render-thread mode of R3 confines the server compositor to one thread (thread-local, `Rc`), which is not this model. Two ways forward for the desktop:

1. **Upstream's model: confinement to the lock.** The graph of server objects (all `Rc` and cells inside) lives in a cell that is `Send` by an `unsafe impl`, entered only through the lock; whichever thread holds the lock renders. This is sound if no handle into the graph exists outside the lock, which is what R1 to R3 established for everything that crosses (batches, resources, jobs, readback) and what has to be audited for the rest (thread-locals, the render interface context, the dispatcher a frame disables, the diagnostics). It keeps upstream's behaviour at the synchronous points, including the resize of a window.
2. **Strict confinement to the render thread.** `UseUiThreadForSynchronousCommits` false: the UI thread waits for the render thread (R4.1) and never renders. No `unsafe`, but it departs from what upstream does on macOS, where presenting a resize from another thread than the UI thread is what the flag exists to avoid; the result on screen during a live resize has to be seen before it can be chosen.

Recommended: 1, because it is the port of what upstream does on this platform and the audit it needs is the continuation of R1 to R3; 2 stays available per platform (it is upstream's model where `UseUiThreadForSynchronousCommits` is false, and what the browser's worker needs). Not started: it is the owner's call, since 1 puts an `unsafe impl Send` under the whole server side.

Either way R5 also needs, on the native backend: the platform graphics (`MetalPlatformGraphics`) and the render surfaces of a top level usable by the thread that renders (upstream creates the software render target on the UI thread only, and throws `RenderTargetNotReady` elsewhere; the port has the same check), and the lock of the Metal device.

### Scope of R3, surveyed

What the UI side asks the server compositor for directly today (outside `rendering/composition/server/`, tests aside). Each becomes a member of the handle the compositor keeps, a job, or a readback:

| Where | What it reaches | Direction |
|---|---|---|
| `Compositor` itself | owns `Rc<ServerCompositor>`, hands it out with `server()`, and gives it the committed batches | The compositor keeps a handle: the queue of committed batches (thread-safe), the readback, and what the rows below need. The server compositor is created by, and stays on, the thread that renders. |
| `container_visual.rs`, `composition_target.rs`, `visual.rs` | `server().readback()` (read revision, next read, the indices) | The readback is the structure that is shared between the threads by design upstream; it is reached through the handle. To check: that all of it is atomics or under its lock. |
| `Compositor::try_get_render_interface_feature` | the cached features of the render interface, else the render interface itself when it is ready | As upstream: the cache is filled by the render thread and read by the UI thread; the direct path is only legal on the render thread. |
| `composition_interop.rs` | the current context of the render interface, compared by identity and used by imports | Belongs with the interop closures that R2 bound to the UI thread; settled with the GPU contexts in R5. |
| `media_context.rs` | `compositor.server().render()` | This is the dispatcher-thread mode: the UI thread renders. It stays, behind the option, and is the only caller that may hold the server compositor itself. |
| 7 generated property blocks, `server_composition_brush.rs` | `host.server()`: the *id* of the server object, not the server compositor | No change. |

47 files outside `server/` name a server type; most do so to create their server object inside a factory closure, which already runs on the render thread. The ones that hold a server object across calls are found by making `Compositor::server()` private.

### Scope of R2, as the compiler names it

Requiring `Send` of `ServerJob`, `ServerObjectJob` and `ServerObjectFactory` (tried on top of R1) fails at these places in the base crate; each is a decision, not a rename:

| Where | What is captured | Direction |
|---|---|---|
| The job queue of the server compositor | the resolved server object of an object job (`Rc`) | The queue is the server's own: its element type is a job of the render thread, not the `Send` job of a batch. |
| `Compositor::invoke_server_job_async` and `invoke_server_object_job_async` | the task (`Rc<RefCell<..>>`) and an error as `Rc<dyn Error>` | `ServerJobTask` holds its result in `Arc<Mutex<..>>`; the continuations stay on the UI thread and run from the dispatcher; the error is `Send`. The result type `T` is `Send`. |
| `CompositionBrush::create`, `CompositionVisual::create_with` | the closures that make the server object and its content | The bound is passed on to the callers; what they capture is checked next. |
| `CompositionCustomVisual` | the messages for the handler (`Rc<dyn Any>`) | A message crosses threads: `Box<dyn Any + Send>`. |
| `CompositionTarget::new` | the render surfaces (`Rc<dyn Fn() -> Vec<Rc<dyn IPlatformRenderSurface>>>`) | The surfaces are made by the window implementation on the UI thread and used by the render target on the render thread: the closure and the surface contract become thread-safe, which reaches the native backend. This is the largest item of R2. |
| `CompositionTarget::set_debug_events` | `Rc<dyn ICompositionTargetDebugEvents>` | `Arc` and `Send + Sync`; the receiver is called on the render thread. |

## 6. Risks

- **Scope of R1.** The audit names eight payload kinds, but each pulls in what it holds (a gradient brush its stops and transform, an image brush its bitmap, a glyph run its typeface). The stage is driven by the compiler until the bound holds; its size is **[E]** the largest of the desktop stages.
- **Performance.** `Arc` in place of `Rc` on render resources adds atomic counts on paths that the performance work of `docs/porting/performance/` measured. R1 re-runs the scroll and start-up measurements; a regression is weighed against what the render thread gives back (layout no longer waits for drawing).
- **Deadlocks at the synchronous points** (R4): resize and first show wait for the render thread while the render thread may wait for a UI-thread service. Upstream's order of locks is followed exactly, and the stress run of R5 exists for this.
- **Thread-affine platform calls.** AppKit requires some calls on the main thread; the native code already separates them for the display link, and R5 checks each call the render thread makes.
- **The browser may be blocked by a dependency** (section 4, item 3). That is found in B0, before any backend work.

## 7. Tests

- Upstream's compositor and renderer suites, in both modes (R4).
- A cross-thread test harness in the base crate: a server compositor on a spawned thread, commits from the test thread, assertions through readback and jobs (R3).
- Compile-time assertions that `CommittedBatch` and the payload contracts are `Send` (R1, R2), so that a later change cannot quietly put an `Rc` into a batch.
- The stress run of R5 and the browser tests in both modes (B2).
