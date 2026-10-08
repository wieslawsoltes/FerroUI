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
| B1 | The browser toolchain and site for threads (headers or service worker, build flags, the non-threaded fallback). Written; the test page is built and passes, the port's module is not built in this mode yet: see "B1 validated" below. | A two-thread page of the port's own module runs on the published host. |
| B2 | The threaded browser backend: render worker, `OffscreenCanvas`, the event grouper queue, the blocking dispatcher. | Designed in [`browser-render-worker.md`](browser-render-worker.md): what upstream does, the model for the browser (strict confinement to a render worker, the UI thread staying on the browser's main thread), the surface objects, memory growth, the steps B2.1 to B2.8 with their tests, and the risks. Done when its step B2.7 is: `themed_view` and the catalog render from the worker; the browser tests pass in both modes. |
| B3 | Measurements: first frame and scrolling, threaded against not. | The table is in `browser-platform.md`. |

Order: R1 to R5 in sequence; B0 in parallel with R1, since it only measures.

### B1 validated for the test page (2026-10-08)

Built and run on the development Mac with the nightly that the feasibility check used (`FERROUI_BROWSER_NIGHTLY=nightly`, which is 1.93.0-nightly of 2025-10-31 there): `scripts/build-browser.sh thread_spawn --threads` builds, and `scripts/browser/tests/thread_spawn.test.mjs` passes its three checks: a spawned thread sends its value back in a page isolated by the headers of the server; without the headers the service worker isolates the page after one reload; a page that cannot be isolated says so. One fault found and fixed: the service worker rebuilt a response without a body (a 204 or a 304) with a body, which is an error. The build without `--threads` is unchanged (`themed_view` builds and passes its 30 checks).

Validated since, with the pinned nightly (`nightly-2026-07-01`, which reports 1.98.0-nightly): the test page builds and passes; and **the port's own module links and runs in the threaded mode**: `scripts/build-browser.sh themed_view --threads` builds, and the 30 checks of `themed_view.test.mjs` pass against it when the site is served isolated (rendering is still done by the page's thread; the worker is B2). Two things had to be found for the link:

- The nightly of the feasibility check (2025-10-31) unwinds with JavaScript exceptions, so Emscripten links its own `emscripten_longjmp`, which collides with the setjmp bridge of the Skia backend. The pinned nightly unwinds with WebAssembly exceptions, as the stable toolchain does.
- The prebuilt archive of the Skia bindings (`libskia-bindings.a`: the objects `bindings`, `gl`, `gpu`, `ganesh`) is compiled without atomics, unlike `libskia.a` beside it, and the linker refuses it in a module with shared memory. The threaded link passes `--no-check-features`. In those four objects a `thread_local` is a plain global and the guard of a local static is not atomic; they are forwarding functions. The proper fix is to compile them in the build with `-pthread` (the sources are in the `skia-bindings` crate) or to get binaries built so; until then this is a known hazard of the threaded mode.

Not validated: the service worker on the published host.

### B1 as written (2026-10-08)

The toolchain and the site for threads exist as an opt-in that leaves the default build as it was; `browser-platform.md`, section 21, is the description. Nothing of it has been built or run: the branch was written without a build, and the first build decides what of it stands.

- **Build.** `scripts/build-browser.sh <application> --threads` builds with the flags B0 found (a nightly toolchain, `-Zbuild-std=std,panic_unwind`, `+atomics,+bulk-memory`, `-pthread`, `-sPTHREAD_POOL_SIZE` from `FERROUI_BROWSER_THREAD_POOL_SIZE`, default 2) plus two that B0 did not need for a plain program: `-pthread` for the C and C++ code that build scripts compile, and `-sENVIRONMENT=web,worker`. Its target directory (`target/threads`) and site directory (`target/browser-threads/<application>`) are its own.
- **Toolchain.** `scripts/browser/setup.sh --threads` installs the nightly with `rust-src` and the target. The pin is `nightly-2026-07-01`, not the nightly of 2025-10-31 of the B0 measurement: that one precedes the unwinding with WebAssembly exceptions that the port's module needs (the caveat of the B0 table). The date is an assumption until it has been installed.
- **Isolation.** The test server sends the two headers on request (`harness.mjs`, `serve.mjs --isolated`). For a host that cannot, `ferroui-coi-sw.js` (since B2.2 a part of the platform's worker, registered as `ferroui-sw.js?coi=1`) adds them from a service worker, and `ferroui-threads.js` registers it, reloads once and shows a message when the page still cannot be isolated. This is the answer to the last row of the B0 table in code, not yet in measurement: it has not run on GitHub Pages.
- **Test page.** `examples/thread_spawn` of the browser crate with `scripts/browser/tests/thread_spawn.test.mjs`: one spawned thread sends a value back, read from the page; the page is isolated once by headers and once by the service worker. It links nothing of the framework, so it does not yet answer the open item of the B0 table, the port's own module with Skia on a thread: `scripts/build-browser.sh themed_view --threads` is that link, and its run needs the host page to call the check.
- **Open for B1 to be done:** the first build and test run; `themed_view` linked and started threaded; the threaded test page published next to the catalog on GitHub Pages and checked there (`crossOriginIsolated` after the reload); a CI job (the nightly through `setup.sh --threads`, its own build cache, `thread_spawn` built and tested). The two service workers of a site (this one and `ferroui-sw.js`) share a scope and have to become one before the catalog can be published threaded.

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

### R3 done: the server compositor on its own thread (superseded by R5.1 for where the server compositor lives)

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

**Decided by the owner on 2026-10-08: 1, the lock model.** It is the port of what upstream does on this platform, and the audit it needs is the continuation of R1 to R3. 2 stays what a platform gets where `UseUiThreadForSynchronousCommits` is false, and what the browser's worker needs.

Either way R5 also needs, on the native backend: the platform graphics (`MetalPlatformGraphics`) and the render surfaces of a top level usable by the thread that renders (upstream creates the software render target on the UI thread only, and throws `RenderTargetNotReady` elsewhere; the port has the same check), and the lock of the Metal device.

### R5.1 done: the compositor lock

`rendering/composition/server/compositor_lock.rs`: `CompositorLock` (a lock its holder may enter again, as `lock` in C#) and `LockedServerCompositor`, which holds the server compositor and hands it out only inside the lock. It carries the one `unsafe impl Send` and `Sync` of the server side, with the invariant written next to it. The compositor holds it in both modes:

- dispatcher-thread mode: as before, a tick is marshalled to the thread of the compositor; `Compositor::server()` reaches the server compositor without the lock, because no second thread enters;
- render-thread mode (`Compositor::with_render_thread`): the thread that ticks the loop renders under the lock; the thread of the compositor enters with `with_server`, and renders itself at the synchronous points when `use_ui_thread_for_synchronous_commits` is set (`render_on_this_thread`), as upstream on macOS. `server()` panics in this mode.

This replaces the thread-local server compositor of R3 (the server compositor is created with the compositor again). A frame disables the processing of the UI dispatcher only when the UI thread renders it, as upstream. The render interface of the platform is looked up when the context manager is created: the service locator belongs to a thread here, where upstream's is global.

`render_thread_tests.rs`: the thread that ticks renders and a job result comes back; a synchronous commit waits for the render thread; with the flag set the UI thread renders it itself; and 200 rounds in which both threads render in turn, with every job checking that no other job is inside the server.

Open for the rest of R5: the audit of what a frame reaches outside the lock when it runs on the render thread (the service locator and other thread-locals, the UI dispatcher, the objects R2 bound to the UI thread: surfaces, drawing surface updates, interop imports); the native backend's surfaces and Metal context; the option and `Compositor::new` choosing the mode.

### R5.2: what a frame reaches outside the lock

A frame on the render thread has no service locator (it belongs to a thread here, where upstream's is global) and must not touch objects of the UI thread. Audit of `rendering/composition/server/`, `drawing/`, `brushes/` and the drawing path of the Skia backend:

| Found | Settled |
|---|---|
| `PlatformRenderInterfaceContextManager` looks the render interface up when it creates the backend context | Done in R5.1: looked up when the manager is created; `platform_render_interface()` hands it to the rest of the server side. |
| `ServerCompositionTarget::new` looks the render interface up for the dirty rect trackers (a target is created by the thread that applies the batch) | Done: asks the compositor's context manager. |
| The debug overlays: the text renderer is made from the default typeface (font manager, typefaces: objects of the UI thread), and the time graphs look the render interface up | Done: one text renderer per server compositor, created under the lock on the UI thread when debug overlays are switched on (`CompositionTarget::set_debug_overlays`); the graphs take the render interface from the compositor. A frame of the render thread draws no overlay text until the renderer exists. Its glyph runs are shared resources since R1.2. |
| Skia `DrawingContextImpl` reads `SkiaOptions` from the service locator for every context | Done: falls back to the options the backend was initialized with (`SkiaPlatform::options`). |
| Skia paint, rounded rectangle and text blob builder caches are thread-local | No change: per-thread caches, as upstream's are thread-static. |
| `ServerCompositor::render` and the UI dispatcher | Done in R5.1: only a frame of the UI thread disables its processing. |
| The three kinds R2 bound to the UI thread in `ThreadBound`: the render surfaces of a target, the update closures of a drawing surface, the import and dispose closures of the interop objects | **Open.** A frame of the render thread panics on them. The surfaces are next: they are what a window needs. |
| `Compositor::server()` callers outside tests: `composition_interop.rs` (three places) | **Open**, with the interop closures. |

### R5.3 in progress: render surfaces a frame of the render thread may use

State (branch `render-thread-r5-3`): the contract is done (`IPlatformRenderSurface: Send + Sync` in `Arc`; `RenderSurfaces` is `Send + Sync`; `ITopLevelImpl::render_surfaces` gives the function the rendering thread calls). Backends, each written by a sub-agent without a build and validated centrally:

| Backend | State |
|---|---|
| OpenGL / EGL | Done. The surface info contract is `Send + Sync` in `Arc` (it has no implementor yet); the GL view of the surface is a handle of its own over the same info. |
| Headless | Done. `HeadlessWindowSurface` holds the framebuffer format, size, scaling and the last frame under locks; the window reads through it. The last frame is kept as pixel memory, and the bitmap is made on the UI thread when it is asked for. |
| Browser | Done for the contract: both render targets keep what belongs to the page's thread in `ThreadBound`; the top level publishes its live surfaces in a shared cell. Real cross-thread surfaces are stage B2. Compiles for `wasm32-unknown-emscripten`. |
| Skia | Test surfaces converted; no change in the backend. |
| Native (macOS) | Done for the software path. `TopLevelFramebufferSurface` is the surface of a top level; the framebuffer render target sits on its shared state with upstream's one lock around `SetFrame` and the release; a top level publishes its surfaces in a shared cell and empties it when it is disposed. Creating a render target stays a matter of the UI thread, as upstream: the surfaces answer "not ready" elsewhere, and the next frame of the UI thread creates it. The Metal surface satisfies the contract; **Metal frames on the render thread are not sound yet**, see below. |

Found in the native library by this step (`native/FerroUI.Native/inc/comimpl.h`): **the reference counts of the native objects are not atomic** (`ComObject::AddRef` and `Release` are plain increments, and most methods take and return a reference to their object while they run). A native pointer may therefore not be cloned, released or called by two threads at once, whatever the method does inside. What that means per interface, from reading the native sources:

| Native interface | From the render thread |
|---|---|
| Top level: creating a software or a Metal render target | UI thread only (the native code refuses elsewhere); kept. |
| Software render target: `SetFrame`, release | Safe as the port uses it: the native method is guarded and written for both threads, and the port holds the only reference and serialises every use under the surface's lock. This is the one `unsafe impl Send` of the native backend. |
| Metal render target, session, device | **Not safe**: no lock on the native side, the native side holds a second reference that the main thread releases, and the Rust wrappers clone their pointer on every call. |

So before `Compositor::new` may choose the render-thread mode on macOS with Metal (the default rendering mode), the native reference counts have to become atomic (a change in `comimpl.h`, which the port owns), or the Metal objects have to be used under one lock on both sides. `MetalDevice::ensure_current` is where upstream takes the lock of the device; it is still empty here. That is R5.4.

The plan as written before the work:

#### The plan

How upstream does it, read from the native backend: `TopLevelImpl.Surfaces` is called by the thread that renders and returns objects that thread then uses. For the GPU they are purpose-made objects over the native top level (`MetalPlatformSurface`, the GL surface), whose native side is callable from the render thread. For software rendering it is the top level itself, and `CreateFramebufferRenderTarget` throws `RenderTargetNotReady` unless it is called on the UI thread: the software render target of a window is created by a frame the UI thread renders (the first show and every resize are synchronous commits, and the platform sets `UseUiThreadForSynchronousCommits`), and the frames of the render thread reuse it.

The port today: `RenderSurfaces` is `Rc<dyn Fn() -> Vec<Rc<dyn IPlatformRenderSurface>>>`, implemented by the window implementation itself in the native and headless backends (an object of the UI thread with cells), and R2 bound it to the UI thread. 46 uses of the surface handle in 25 files (the contract, the Skia backend's render target creation, the native, headless, browser and EGL backends, tests).

Plan:

1. The contract: `IPlatformRenderSurface: Send + Sync`, held in `Arc`; `RenderSurfaces` is `Arc<dyn Fn() -> Vec<Arc<dyn IPlatformRenderSurface>> + Send + Sync>`. The `ThreadBound` around the surfaces of a target goes away.
2. Each backend hands out surface objects made for it, not its window implementation:
   - native: the Metal surface already is one (it holds the native top level; `ComPtr` has to be allowed to cross threads for the native interfaces that are callable from the render thread, which is a statement about the native library to verify in `native/`), and a software surface object that holds the native top level and keeps upstream's rule (creation of the render target on the UI thread only, "not ready" elsewhere);
   - headless: a surface object with the last frame under a lock (upstream guards it with a lock too);
   - browser, EGL: single-threaded today; their surface objects keep what belongs to their thread in `ThreadBound` until B2.
3. `TopLevel` snapshots nothing: the closure asks the surface objects' owner under a lock of its own, or the window implementation publishes its current surfaces in a shared cell when they change.
4. Then `Compositor::new` can choose the render-thread mode for a background render loop (R4.3), first behind an option, and `themed_window` is the first window to render off the UI thread.

### R5.4: the native library and the first window in the render-thread mode

- The reference counts of the native objects are atomic: a patch of `comimpl.h` (`std::atomic`), kept in `native/FerroUI.Native/patches/` so that `scripts/sync-native.sh` applies it again after a sync. Upstream has the plain increments.
- `DisposableLock` (`utilities/disposable_lock.rs`) is ported, and `MetalDevice::ensure_current` takes it, as upstream; it was empty while the device was only used on the UI thread.
- **`FERROUI_RENDER_THREAD=1`** makes the native platform create its compositor with `Compositor::with_render_thread` (`use_ui_thread_for_synchronous_commits` true, as upstream on this platform). Without it nothing changes. It is a switch for bringing the mode up, not an option of the product yet.
- Found by the first run: the render interface is registered after the platform creates its compositor, so the render thread could not find it. It is now looked up by the UI thread when a composition target is created, and a frame of a thread that cannot find it waits (`is_ready`).

With the switch, on the development Mac: `platform_window` (software: render target created by the UI thread's frame, one paint), `platform_window --metal` (two paints) and `themed_window` (six runs) open, render and close without a panic; the frames of the loop come from the thread of the render timer (`RenderTimerLoop`).

Checked since: a capture of the `themed_window` window is byte for byte the same in both modes, and the ControlCatalog selects every one of its 59 pages in the render-thread mode (`FERROUI_SMOKE_PAGES=150`) without a panic, as in the dispatcher-thread mode.

Measured, one release build run in both modes (the switch is read at start-up), launched alternately, on the development Mac under load from other work (load average 35 to 42, so only the comparison counts):

| Measure | Dispatcher-thread mode | Render-thread mode |
|---|---|---|
| `themed_window`, process start to `Window opened`, median of 15 (range) | 150 ms (147 to 165) | 155 ms (147 to 188) |
| `control-catalog-desktop`, process start to `App activated`, median of 10 (range) | 385 ms (379 to 398) | 379 ms (375 to 389) |
| catalog, 150 pages in 20 s, user + system CPU of three runs | 2.76, 2.95, 3.23 s | 2.73, 2.72, 2.80 s |

Start-up is the same in both modes (the first frame is a frame of the UI thread in either). The total CPU of the page run is the same or slightly lower in the render-thread mode. What the mode is for, the UI thread being free while a frame is drawn, is not what these numbers show: that needs a frame time and input latency measurement during scrolling.

Not done, and needed before the mode can be the default:

- Interaction under load, by hand: live resize, scrolling in the catalog, theme switch, popups, closing a window while it renders.
- The Metal wrappers on the Rust side (`Rc` objects behind the Skia contracts) are used under the compositor lock; audited in R5.5 below, which leaves the handles shared with the service locator open.
- The update closures of a drawing surface and the import closures of the interop objects are still bound to the UI thread (R2).
- The measurements of `desktop-performance.md` in both modes.

### R5.5: the Metal objects under the compositor lock

The audit R5.4 left open, done by reading (no build, no run): every object of the Metal rendering path, who creates it, who uses it, and whether a thread can reach it without holding the compositor lock. Read: `src/FerroUI.Native/metal.rs`, `src/Skia/FerroUI.Skia/gpu/metal/`, `skia_backend_context.rs`, `gpu/graphite/graphite_gr_context.rs`, `rendering/platform_render_interface_context_manager.rs`, `server_composition_target.rs`, `server_compositor.rs`, the native `metal.mm`, and every caller of `IPlatformGraphics`, `gpu_context`, `IMetalDevice`, `try_get_feature` and `try_get_render_interface_feature` in the native backend, the controls, the base library outside the server side and the OpenGL library.

**Verdict.** Every Metal object below the platform graphics is created inside the server graph and never leaves it: all its uses are under the compositor lock, and the UI thread has no handle to it outside the lock. The frame itself is not rendered under the lock of the device, here as upstream: the compositor lock is what serialises the two threads, and the lock of the device is only taken where upstream takes it. What is not confined are the two objects the server graph shares with the service locator, the platform graphics and the render interface: both are `Rc`, and for the render interface the render thread changes the reference count that the UI thread changes too. That one is a real race and is open, as is the thread that releases the graph at shutdown.

| Object | Created by, on which thread | Used by | Under the compositor lock | Verdict |
|---|---|---|---|---|
| `MetalPlatformGraphics` (`Rc<dyn IPlatformGraphics>`, holds the native display) | The platform set-up, UI thread. Three handles: the platform, the service locator, the context manager of the server compositor | The context manager: `uses_shared_context` and `create_context`, by reference, on the thread that creates the backend context. Anything on the UI thread that asks the locator (`examples/platform_window.rs` does, and calls `create_context`) | The server side yes; the UI thread's handles no | Sound as used, open as a type. The server side never clones or releases its handle while it renders, so only the UI thread changes the count; the native `CreateDevice` has no state and the native counts are atomic, so two threads may create a device at once. See open items 2 and 3. |
| `MetalDevice` (`Rc`, `RefCell` around the native device, the lock of the device) | `create_context`, called by `ensure_valid_backend_context`: the thread that renders the first frame, under the lock | The context manager (`gpu_context`, `ensure_current`), `SkiaMetalGpu` (the device and queue handles once, `ensure_current`), `MetalPlatformSurface::create_metal_render_target` (UI thread, in a frame), the lease a custom drawing operation may take during a frame | Yes | Confined. `gpu_context()` is public on the context manager but has no caller outside it; upstream's `GpuContext` is internal and unused outside it too. The only feature the device hands out is itself as `IMetalDevice`, to the Skia backend, under the lock. |
| The lock of the device (`DisposableLock`, `ensure_current`) | With the device | `ServerCompositor::create_render_target`, `create_composition_visual_snapshot`, `ServerCompositionTarget::reset_render_target`, the drawing surface, `SkiaMetalGpu::try_get_gr_context` (offscreen layers, the size limit) | Always taken inside the compositor lock | Same places as upstream; nothing takes it outside the compositor lock, so the order is always compositor lock, then device lock, and it cannot invert. It protects nothing the compositor lock does not protect already until something outside the compositor uses the device (the external objects, not ported). |
| `MetalPlatformSurface` (`Arc`, `Send + Sync`) and its view (`Rc`, made per call) | The top level, UI thread | `is_ready` and `try_get_surface_kind` by the thread that renders; `create_metal_render_target` on the UI thread only; `close` by the top level on the UI thread, outside the lock | The rendering uses yes | Sound. The native top level is bound to the UI thread (`SurfaceTopLevel`): another thread sees none, so the surface answers "not ready" there and never touches the cell `close` empties. The view is created and dropped by the thread that asked. |
| `MetalRenderTarget` (`Rc`, `RefCell` around the native render target) | `create_metal_render_target`, a frame of the UI thread | `begin_rendering` by either thread; `dispose` when the target is reset or disposed (a batch or a job) | Yes | Confined. Native side: the view keeps a second reference and writes the pending size from the UI thread without a lock (`resize:withScale:`), but `BeginDrawing` reads the pending size on the main thread only, and the size a session of the render thread gets was copied by the last frame of the main thread, under the compositor lock. |
| `MetalDrawingSession` (`Rc`, `RefCell` around the native session) | `begin_rendering`, the thread that renders the frame | The same frame: texture, size, scaling, then disposed (which presents) | Yes | Confined to one frame of one thread. |
| `SkiaMetalGpu` (`Rc`), the shared cell of its Graphite context | `create_backend_context`, under the lock, by the thread that creates the backend context | `SkiaContext`, the Skia render targets, offscreen layers and drawing contexts of the frame | Yes | Confined. It has no feature (`try_get_feature` is `None`), so nothing of it is handed to callers. |
| `GraphiteGrContext` (`Rc`, `RefCell` around recorder, context and the uploaded images) | With the GPU | The sessions and layers of a frame; `flush` when a session ends | Yes | Confined. An uploaded image keeps the Skia image of a bitmap of the UI thread alive and reads its reference count; Skia's counts are atomic and an image is immutable. Recorder and context are used by one thread at a time, in turn, which Graphite allows. |
| `SkiaMetalRenderTarget`, `SkiaGpuRenderTarget` (`Rc`) | `SkiaContext::create_render_target`, a frame of the UI thread | `ServerCompositionTarget::render` on either thread | Yes | Confined; held by the server target only. |
| `SkiaMetalRenderSession` and its autorelease pool | `begin_rendering_session`, the thread that renders the frame | The drawing context of the frame; disposed when the frame ends | Yes | Confined to one frame of one thread, which the pool requires (it is pushed and popped by the same thread). |
| `SkiaContext` (the backend context) | `ensure_valid_backend_context`, under the lock | The context manager, the visual cache, the drawing surface | Yes | Confined. Its public features are an empty map on Metal. |
| `PlatformRenderInterfaceContextManager` (`Rc`, cells) | With the server compositor, UI thread | The server side, under the lock; `CompositionTarget::new` and `Compositor::try_get_render_interface_feature` through `with_server` | Yes | Confined, except for what it holds of the locator: see open item 1. |

#### What the UI thread can reach outside the lock, and what upstream does

Upstream none of this needs an argument: the objects are garbage collected, a reference may be copied by any thread, and `MetalPlatformGraphics` and `PlatformRenderInterface` have only read-only fields. The service locator is global, so `IPlatformGraphics` is reachable from every thread there as well; what upstream relies on is that `CreateContext` returns a new device each time (`new MetalDevice(_factory, _display.CreateDevice())`), so a caller outside the compositor never gets the compositor's device, and that the compositor's own device, which a caller can only get at through the external objects feature, is used by that feature in jobs of the server compositor (`CompositionInterop`: `InvokeServerJobAsync`, and `EnsureCurrent` around the release), so under the compositor lock. The port has the same shape; the difference is that a handle is an `Rc`, and an `Rc` may not be cloned or released by two threads.

- **`IPlatformGraphics` in the service locator and in the server graph.** A caller on the UI thread can clone and release the handle and call `create_context`. Neither races today: the context manager uses its handle by reference and never clones it, and the device a caller creates is its own (its own native device object, command queue and lock). It would race the moment the server side cloned the handle under the lock, and it does race if the graph is released by the render thread (open item 2).
- **`IPlatformRenderInterface` in the service locator and in the context manager.** This one races. `platform_render_interface()` returns a clone of the `Rc`, and frames of the render thread call it: `is_ready` on every frame until the backend context exists, `ensure_valid_backend_context`, `ServerCompositionTarget::new` for every target whose batch a frame of the render thread applies (a window, a popup), and the time graphs of the debug overlays. The UI thread clones and releases the same `Rc` outside the lock all the time (`platform::render_interface()`, for every geometry, glyph run and bitmap it creates). Two unsynchronised updates of a plain reference count can lose one; a lost increment ends, after enough of them, in the render interface being freed while it is in use. The object itself is safe to call from both threads (the Skia render interface has no cells).
- **`gpu_context()` of the context manager.** It would hand the compositor's `MetalDevice` to a caller, who could release the native device or call it while a frame draws with it. No caller exists outside the context manager, and the manager is only reachable through `with_server`. `composition_interop.rs` reaches the manager through `Compositor::server()`, which panics in the render-thread mode (already open in R5.2).
- **Features handed to callers (`Compositor::try_get_render_interface_feature`).** The map of public features is read under the lock and an `Rc<dyn Any>` of it is returned to the UI thread, which then holds and releases it outside the lock while a frame may clone the same `Rc` from the cache. On Metal the map is empty today, so nothing is handed out. It becomes a race with the first feature: the external objects of Metal (`SkiaMetalExternalObjectsFeature` upstream, whose imports run as jobs of the server compositor), or the GL texture sharing if the mode ever runs on a GL platform (`open_gl_composition_interop.rs` asks for it).
- **The lease of the platform graphics API (`try_lease_platform_graphics_api`).** A custom drawing operation gets an `Rc` of the device during a frame, under the lock. If it keeps the handle after the frame it holds a handle of the graph outside the lock; upstream the same operation may keep the object, and uses it under `EnsureCurrent`. Nothing in the tree keeps it.

#### Changed by the audit

- `MetalDevice::device`, `command_queue` and the three getters of `MetalDrawingSession` call the native object through the cell they hold it in, without taking and giving back a native reference per call (three pairs per frame). The calls that can take time or change the native object (`BeginDrawing`, creating a render target) keep their own reference for the duration, as before.
- The order of the two locks is written next to `MetalDevice::ensure_current`.

#### Open, most serious first

1. **The reference count of the render interface is changed by both threads** (above). To settle in `platform_render_interface_context_manager.rs` and its callers: hand the render interface out by reference under the lock instead of as a clone, or make `IPlatformRenderInterface` `Send + Sync` and hold it in `Arc` in the locator and in the manager. The second is upstream's shape (one global object any thread may hold) and the larger change.
2. **The thread that releases the server graph.** The render loop holds a reference to the loop task of the compositor for the length of a tick (`items_copy` in `render_loop.rs`), and the task holds the locked server compositor. If the compositor is dropped on the UI thread during a tick, the last reference is released by the render thread, outside the lock, and the whole graph is dropped there: the Metal render targets, the device, the Graphite context, and the two handles shared with the locator, whose counts the UI thread may be changing. In the native backend the compositor lives as long as the process, so this is the shutdown path. To settle in `compositor.rs`: the loop task holds a weak reference, or the compositor empties the graph under the lock on its own thread before it lets go.
3. **`IPlatformGraphics` is an `Rc` shared between the locator and the server graph.** Sound only by a rule nothing enforces (the server side never clones or releases its handle while it renders). The same answer as item 1 applies: `Send + Sync` in `Arc`, which `MetalPlatformGraphics` satisfies once the native display pointer may cross threads (its one method has no state).
4. **Public features leave the lock as `Rc`** (above): empty on Metal today, to be decided before the external objects of Metal are ported. Upstream awaits the answer from the render thread, keeps the feature object on the UI thread and runs its imports as jobs of the server compositor; here the feature would have to be `Send + Sync`, or used only through jobs. **Closed in R5.8**: the caller gets a handle that stays inside the lock (`RenderInterfaceFeature`) and lends the feature there.
5. **The lock of the device is redundant today**, so it has never been contended by two threads. Nothing to do until the external objects exist.
6. **Not covered**: the EGL contexts and the GL Skia GPU (the render-thread mode is switched on by the macOS platform only), and behaviour under load, which needs the runs listed under R5.4.

### R5.5 and R5.6 settled: what the audit found, and the interop objects

The audit above (R5.5) found two faults in the earlier steps of R5, both fixed with it:

- **The handle of the render interface was cloned by the render thread.** It is an `Rc` shared with the service locator of the UI thread. The context manager now lends it (`with_platform_render_interface`), and the UI thread looks it up (`capture_platform_render_interface`, when a composition target is created); the time graphs of the overlays ask the compositor for each use.
- **The server graph could be released by the render thread**, outside the lock, when the compositor was dropped during a tick. `Compositor::drop` now releases the server compositor itself, under the lock, on its thread (`LockedServerCompositor::release`); a tick that still holds the loop task finds nothing to render.

Left as the audit describes them at the time: the handle of the platform graphics is sound only because the manager never clones it, and `try_get_render_interface_feature` hands a feature out of the lock (no backend has one on Metal yet). Both are closed in R5.8 below.

R5.6: the update of a drawing surface and the imports and disposals of the interop objects no longer capture anything of the UI thread. Their server parts are values confined to the compositor lock (`LockBound<T>`, in `compositor_lock.rs`, reached only with a reference to the server compositor it is bound to); the jobs capture the bound value and plain values. One capture was still bound to the UI thread: the image of `import_shared_image`, whose contract passed an `Rc` that the caller keeps; on a render thread that import failed its task. R5.7 below settles it.

R5.7: the image of `import_shared_image` is shared between the two threads, as upstream, where the UI thread creates and disposes the object and the render thread reads it during the import. The contract says so: `ICompositionImportableSharedGpuContextImage: Send + Sync`, passed as `Arc<dyn ...>` by `ICompositionGpuInterop::import_shared_image`, by `IExternalObjectsRenderInterfaceContextFeature::import_shared_image` and by `IOpenGlTextureSharingRenderInterfaceContextFeature::create_shared_texture_for_composition`. The import job captures the `Arc` and runs on whichever thread renders; `GpuImportError` is gone. Nothing a job captures is bound to the UI thread any more.

What an implementation of the image owes the contract (upstream: `GlSkiaSharedTextureForComposition`, in the Skia backend; no backend here creates shared textures yet, so the only implementation is the fake of the tests): the texture id, the internal format and the size are plain values, the id behind an atomic or a lock because the disposal clears it; the GL context the texture was made on is an `Rc` of the UI thread and stays in a `ThreadBound`, so a drop on the render thread leaks the handle instead of touching it. Upstream deletes the texture from either thread with that thread's own context of the share group (`Dispose(IGlContext)` from the imported image, `Dispose()` from the owner), under a lock; the same shape holds here, with the context of the caller reached only on its thread. The check `Context.IsSharedWith` that upstream makes on the render thread against the UI thread's context object needs a value the render thread may read (the share group, not the context).

### R5.8: the two handles the audit left

Written on 2026-10-09 without a build or a test run; the branch is validated by another session.

**Features handed to callers (open item 4 of R5.5, doubt 5 of B2.4).** `Compositor::try_get_render_interface_feature` cloned the `Rc<dyn Any>` of a feature out of the map of the server compositor and returned it: the caller then cloned and dropped its handle outside the lock while a frame of the render thread, inside it, dropped the map with the context it belonged to. Two threads on one plain count.

What each caller did with the feature, and on which thread:

| Caller | What it did | Thread |
|---|---|---|
| `Compositor::try_get_composition_gpu_interop` | Took the external objects feature (`IExternalObjectsRenderInterfaceContextFeature`), cloned the inner `Rc` and handed it to `CompositionInterop::try_new`, which binds it to the lock (R5.6). The outer handle was dropped inside the lock, because the whole call ran in `with`. | The thread of the compositor, inside the lock. Sound already; it went through the public query. |
| `OpenGlCompositionInterop::try_create_compatible_gl_context` (`src/FerroUI.OpenGL`) | Took the texture sharing feature (`IOpenGlTextureSharingRenderInterfaceContextFeature`), cloned the inner `Rc` out, asked it `can_create_shared_context` and `create_shared_context`, and gave the clone to `CompositionGlContext`, which kept it for the life of the context and asked it `create_shared_texture_for_composition` for every texture. | The thread of the compositor, outside the lock: the race. |
| `IRenderer::try_get_render_interface_feature` (`CompositingRenderer`, and nine renderers of tests that answer `None`) | Passed the compositor's answer on. Nothing in the tree calls it. | The thread of the renderer. |
| `render_thread_tests.rs` | Asked whether the feature is of a type, and dropped it. | The test thread, outside the lock. |

Not callers of this query, read to be sure: the lease of the Skia API (`ISkiaApiLeaseFeature`) is a feature of the drawing context, asked for during a frame by a custom drawing operation, inside the lock; the features of a graphics context (`IMetalDevice`, the external objects of a GL context) are asked for by the backend with `IOptionalFeatureProvider::try_get_feature`, inside the lock too; `IGlContextExternalObjectsFeature` in the GL interop is a feature of the context the caller itself created. No backend overrides `public_features` (the default is an empty map), so only the doubles of the tests hand a feature out today.

The fix is the one R5.6 chose for the interop objects. The query returns a `RenderInterfaceFeature` (`rendering/composition/render_interface_feature.rs`): a `LockBound<Rc<dyn Any>>` whose handle is cloned from the map **inside** the lock, bound to the lock there, and dropped inside it by whichever thread lets go of it. The caller reaches the feature in two ways, both lending the feature and never the `Rc`: `with::<dyn IFoo, _>(|feature| ..)`, which enters the lock for the call, and `get::<dyn IFoo>(server)` for a caller that has the server compositor in hand (a job). `is::<T>()` tells what the feature is registered as. The handle keeps the feature alive after its context is replaced, as the object upstream does. It is `Send + Sync` (asserted), by the argument of `LockBound` and no new one.

- `Compositor::try_get_composition_gpu_interop` no longer goes through a handle: `with_render_interface_feature` lends the entry of the map in place, and the interop clones the inner `Rc` inside the lock as before.
- `OpenGlCompositionInterop` keeps the handle, asks the feature inside the lock, and gives the handle to `CompositionGlContext`, which holds a `RenderInterfaceFeature` and asks it for each texture inside the lock. What leaves the lock are the context the feature creates for its caller and the shared texture, which is `Send + Sync` by its contract (R5.7).
- `IRenderer::try_get_render_interface_feature` returns the handle.
- A confined compositor (B2.4) answers from the cache as before, and posts the job that fills it when there is none; the handle it returns is bound to the lock in the same way. Dropped by the thread of the compositor it is leaked, as every `LockBound` of a confined compositor is (doubt 10 of B2.4): the feature of a confined compositor that a caller ever held is not freed with its context.

What changed in behaviour: a call of a feature through a handle waits for the frame the render thread is rendering, as the calls of `CompositionInterop` do since R5.6. Nothing else.

What the types still cannot check, written next to the `unsafe impl` in `compositor_lock.rs`: a caller of `with_server` gets the server compositor itself and must not clone an `Rc` out of it; and a feature that returns an `Rc` to a caller who keeps it (`create_shared_context`) must return one that shares no count with the graph.

Tests, `render_thread_tests.rs`: 200 rounds in the lock model in which the test thread asks for the feature and calls it while the render thread renders, then loses the graphics context so that a frame (of the render thread, and of the test thread in every other round) replaces the backend context and the map; a handle from every third round is kept to the end. A second test sends 400 handles to a third thread that calls the feature and drops them. Both count the features made and dropped (a lost count shows as a feature that is never dropped, a doubled one as a crash) and that no two threads were inside a feature or a frame at once. A third test covers the dispatcher-thread mode, `get` with the server compositor, and a handle that outlives its compositor.

### The render thread is the default on the desktop (owner, 2026-10-08)

`Compositor::new` follows the render loop, as upstream: with a loop that runs in the background the thread that ticks it renders the frames, under the compositor lock, and the thread of the compositor renders at the synchronous points when the platform asks for that. The native platform (macOS) has such a loop (`ThreadProxyRenderTimer` over the display link) and asks for the UI thread at the synchronous points, so its windows are rendered by the `RenderTimerLoop` thread from now on; a sample of the running ControlCatalog shows the frames of the server compositor on that thread only. `FERROUI_RENDER_THREAD=0` keeps the rendering on the UI thread (each tick is marshalled to it): the way back while the mode is young. The experimental `FERROUI_RENDER_THREAD=1` is gone.

Not changed by this: the headless platform (its render timer belongs to the UI thread, as upstream's) and the browser (its timer is the animation frame of the page; rendering from a worker is stage B2, `browser-render-worker.md`).

What was known to be open when the default was switched, by the owner's decision: interaction by hand (live resize, scrolling, popups, closing a window while it renders) was not exercised; the frame time of both modes was not measured yet; the handle of the platform graphics and the features of the render interface are as the audit of R5.5 left them.

### Frame time, both modes

What the render-thread mode is for, the UI thread being free while a frame is drawn, is what the measurements under R5.4 do not show. The frame benchmark of the catalog (`samples/ControlCatalog/tests/frame_benchmark.rs`) measures it: `frame_benchmark_table_view_scrolling_render_thread` builds the table view page in a window over the mock window implementation and a raster surface, scrolls the table by 20 pixels a frame (300 frames after a sweep that is not measured), and does so twice, each time in an application of its own: with the compositor of the test services (dispatcher-thread mode) and with a compositor created by `Compositor::with_render_thread` with `use_ui_thread_for_synchronous_commits` set, as the macOS platform creates it, over a render loop that a thread of the benchmark ticks.

```sh
cargo test -p control-catalog --release --lib frame_benchmark_table_view_scrolling_render_thread -- --ignored --nocapture --test-threads=1
```

Per frame and per mode it records:

- **UI thread**: the time the UI thread is busy from the input (the offset is set) until it has nothing left to do for the frame: layout, the recording of the render data, the commit, the jobs the frame posts back, and in the dispatcher-thread mode the rendering. The time it waits for the render thread without work is not counted.
- **Frame completion**: from the input until the batch that holds its changes has been rendered (the `rendered` completion of the batch, stamped by the thread that renders it).
- **Render thread** (render-thread mode): the time inside the ticks of the frame.

Pacing: as in the other frame benchmarks no frame waits for a display. A frame starts when the one before it is complete, and the render thread ticks when the compositor wakes its loop for a committed batch; only the ticks the server compositor asks for without a commit come at 60 a second. The numbers are the cost of a frame in each mode, without the wait for the next tick of a display, which is the same in both. The first frame of the window is a frame of the UI thread in either mode (the render thread of the benchmark starts after the window is shown), so the render target of the raster surface is created there, as the software render target of a native window is.

The benchmark asserts that every scroll step reaches the surface before the next one is made, in both modes; in the dispatcher-thread mode the surface receives exactly one frame per step, in the render-thread mode at least one.

Numbers, taken on 2026-10-08 on the development Mac, one release build, both modes in the same run. **The machine was heavily loaded by other work (load average 40 to 98)**, so the runs differ from each other and one of the four is an outlier; within a run the two modes are measured seconds apart. Median ms per frame (p95):

| Run | UI thread, dispatcher-thread mode | UI thread, render-thread mode | Frame completion, dispatcher-thread mode | Frame completion, render-thread mode |
|---|---|---|---|---|
| 1 | 1.76 (2.55) | 0.50 (0.84) | 1.69 (2.54) | 1.38 (1.93) |
| 2 (outlier) | 2.71 (5.99) | 1.63 (2.31) | 2.56 (5.78) | 4.29 (5.36) |
| 3 | 1.86 (2.85) | 0.49 (0.82) | 1.75 (2.64) | 1.37 (1.83) |
| 4 | 1.55 (2.58) | 0.36 (0.51) | 1.48 (2.32) | 1.14 (1.40) |

In run 1 the render thread spent a median of 1.37 ms in the ticks of a frame, against 1.48 ms of rendering on the UI thread in the other mode.

Reading: in the render-thread mode the UI thread is busy for about 0.4 to 0.5 ms of a scroll step instead of 1.6 to 1.9 ms (about a quarter), because the frame is drawn elsewhere; and a frame is complete somewhat sooner, because the UI thread's work and the drawing overlap less than they queue. Under load the completion can be later instead (run 2): the frame then waits for the render thread to be scheduled. To be repeated on a quiet machine before the numbers are quoted.

One thing the benchmark showed about the dispatcher-thread mode: a scroll step is not always rendered by the first tick after it. A commit that is requested while the batch before it is pending is made one pass of the dispatcher later, so the benchmark pumps the frame until its batch is rendered (300 steps took 450 rendered frames).

What the numbers cannot show: the surface is a raster framebuffer, so the share of the rendering in a frame is not the one of a window on Metal, and there is no presentation. The same measurement in a real window is the entry of `desktop-performance.md` that R5.4 lists as not done.

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
