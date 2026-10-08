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
| R1.4b | Effects, dash styles, and the shared form of immutable brushes and pens | Open. |
| R1.4c | The values of `BatchResource<T>`: transforms, gradient stops, brushes of composition objects | Open. |
| R1.4d | Animation instances (`Rc<dyn IAnimationInstance>` is sent by 30 property writers of the generated composition objects); the untyped value of the composition target | Open. An instance moves to the server and is owned there: it has to be `Send`, not shared. |
| R1.5 | `BatchObject::Value` requires `Send`; measurements | Open. Switching the bound on locally is how the list above was found: after R1.4a the compiler names exactly the kinds of R1.4b to R1.4d. |

Design of R1.4. With `BatchObject::Value(Box<dyn Any + Send>)` the compiler names what is left: the brush, pen and effect of render data, its geometry wrapper, custom draw operations, the two counted holders (glyph run, bitmap) and the values of `BatchResource<T>`.

- A brush, pen or effect object stays an `Rc` on the UI thread, where mutable and immutable ones share one handle type and render data keeps them for hit testing. What is *sent* is the **shared form** of an immutable one: `to_shared()` returns an `Arc<dyn IBrush + Send + Sync>` (and likewise for pens and effects) with the same values, cached by the object so that sending the same brush again costs a count. The server table holds shared forms beside the server objects it resolves from ids; the drawing context takes `&dyn IBrush` either way.
- The immutable types hold their parts in thread-safe form (gradient stops, dash style, transform, the counted bitmap of an image brush).
- `ICustomDrawOperation` requires `Send + Sync` and is held in `Arc`: it is rendered on the render thread, as upstream documents.
- The geometry wrapper of render data (`IRenderDataGeometry`) is never sent as a value: with a compositor a geometry is sent as the id of its server object.
- The counted holders become `Arc<Arc<..>>`.

Found by R1.1 in the Skia backend: a path can be sent to another thread but not shared by reference, and a path measure can be neither. The geometries therefore never lend a path; the path measure is cached behind a lock in a wrapper that asserts it may move between threads (it owns its contours and has no thread affinity).

Found by R1.3: a Skia bitmap cannot be sent to another thread either; it is held in a wrapper under a lock, like the path measure. `ThreadBound<T>` (`utilities/thread_bound.rs`) is the tool for a part of a shared resource that one thread owns: it panics when reached from another thread and leaks, rather than drops, when the resource dies there.

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
